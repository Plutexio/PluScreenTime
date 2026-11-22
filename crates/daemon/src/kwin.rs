//! The focused window on KDE Plasma (KWin 6).
//!
//! KWin tells no outsider which window has the focus, but it runs scripts.
//! The daemon offers `/Focus` on the session bus and loads a small script
//! into KWin that calls it whenever the active window or its caption
//! changes. When KWin restarts, the script is loaded again.

use std::time::Duration;

use anyhow::{Context, Result, bail};
use tokio::sync::mpsc;
use tracing::{debug, info, warn};
use zbus::Connection;
use zbus::names::BusName;

use crate::apps::Window;
use crate::tracker::Event;

const NAME: &str = "kwin";
const BUS_NAME: &str = "io.github.plutexio.PluScreenTime";
const PLUGIN: &str = "pluscreentime";
const POLL: Duration = Duration::from_secs(5);

const SCRIPT: &str = r#"// Loaded by pluscreentimed: tells it which window has the focus.
let current = null;
function report() {
    const w = current;
    callDBus("io.github.plutexio.PluScreenTime", "/Focus", "io.github.plutexio.PluScreenTime.Focus",
             "Activated",
             w ? String(w.resourceClass || "") : "",
             w ? String(w.caption || "") : "",
             w ? String(w.desktopFileName || "") : "");
}
function follow(w) {
    if (current)
        current.captionChanged.disconnect(report);
    current = w;
    if (current)
        current.captionChanged.connect(report);
    report();
}
workspace.windowActivated.connect(follow);
follow(workspace.activeWindow);
"#;

struct Focus {
    events: mpsc::Sender<Event>,
}

#[zbus::interface(name = "io.github.plutexio.PluScreenTime.Focus")]
impl Focus {
    async fn activated(&self, class: String, title: String, desktop_file: String) {
        let window = (!class.is_empty() || !title.is_empty()).then(|| Window {
            class,
            title,
            desktop_id: Some(desktop_file).filter(|d| !d.is_empty()),
        });
        let _ = self.events.send(Event::Focus { desktop: NAME, window }).await;
    }
}

pub async fn run(events: mpsc::Sender<Event>) {
    let connection = match Connection::session().await {
        Ok(connection) => connection,
        Err(e) => {
            debug!("no session bus, so no KDE Plasma: {e}");
            return;
        }
    };
    if let Err(e) = serve(&connection, events.clone()).await {
        warn!("can't offer {BUS_NAME} on the session bus, so KDE Plasma isn't followed: {e:#}");
        return;
    }

    let dbus = match zbus::fdo::DBusProxy::new(&connection).await {
        Ok(proxy) => proxy,
        Err(e) => {
            warn!("session bus: {e}");
            return;
        }
    };
    let kwin: BusName = "org.kde.KWin".try_into().expect("a valid bus name");
    // KWin's unique name on the bus, while it runs and the script is in.
    let mut following: Option<String> = None;
    loop {
        let owner = dbus.get_name_owner(kwin.clone()).await.ok().map(|o| o.to_string());
        if owner != following {
            if following.is_some() {
                let _ = events.send(Event::Focus { desktop: NAME, window: None }).await;
                let _ = events.send(Event::Desktop { name: NAME, present: false }).await;
                following = None;
            }
            if owner.is_some() {
                match load_script(&connection).await {
                    Ok(()) => {
                        info!("following KWin through its script");
                        let _ = events.send(Event::Desktop { name: NAME, present: true }).await;
                        following = owner;
                    }
                    Err(e) => debug!("loading the KWin script: {e:#}"),
                }
            }
        }
        tokio::time::sleep(POLL).await;
    }
}

async fn serve(connection: &Connection, events: mpsc::Sender<Event>) -> Result<()> {
    connection
        .object_server()
        .at("/Focus", Focus { events })
        .await
        .context("offering /Focus")?;
    connection
        .request_name(BUS_NAME)
        .await
        .context("is another pluscreentimed running?")?;
    Ok(())
}

async fn load_script(connection: &Connection) -> Result<()> {
    let dir = pluscreentime_protocol::runtime_dir().context("no runtime directory")?;
    let path = dir.join("pluscreentime-kwin.js");
    std::fs::write(&path, SCRIPT).with_context(|| format!("writing {}", path.display()))?;

    let scripting = zbus::Proxy::new(connection, "org.kde.KWin", "/Scripting", "org.kde.kwin.Scripting").await?;
    // An older copy (from a daemon that went before) would report twice.
    let _: Result<bool, _> = scripting.call("unloadScript", &(PLUGIN,)).await;
    let id: i32 = scripting
        .call("loadScript", &(path.display().to_string(), PLUGIN))
        .await
        .context("loadScript")?;
    if id < 0 {
        bail!("KWin refused the script");
    }
    let script = zbus::Proxy::new(
        connection,
        "org.kde.KWin",
        format!("/Scripting/Script{id}"),
        "org.kde.kwin.Script",
    )
    .await?;
    let ran: Result<(), _> = script.call("run", &()).await;
    if ran.is_err() {
        // KWin starts whatever is loaded but not yet running.
        let _: () = scripting.call("start", &()).await.context("starting the script")?;
    }
    Ok(())
}
