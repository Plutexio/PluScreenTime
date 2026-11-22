//! Whether anybody is there, from the compositor (ext-idle-notify-v1, which
//! Hyprland and KWin both offer).
//!
//! Version 1's notification honours idle inhibitors, so a video or a game
//! that keeps the screen awake keeps the time counting too. It runs on a
//! thread of its own with a plain Wayland connection, found the way the
//! session's apps find it, or by looking in the runtime directory when the
//! service was started without `WAYLAND_DISPLAY`.

use std::os::fd::AsRawFd;
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

use anyhow::{Context, Result};
use tokio::sync::mpsc;
use tracing::debug;
use wayland_client::globals::{GlobalListContents, registry_queue_init};
use wayland_client::protocol::{wl_registry, wl_seat};
use wayland_client::{Connection, Dispatch, QueueHandle};
use wayland_protocols::ext::idle_notify::v1::client::{ext_idle_notification_v1, ext_idle_notifier_v1};

use crate::tracker::{Event, now_ms};

const RETRY: Duration = Duration::from_secs(3);

pub fn spawn(events: mpsc::Sender<Event>, timeout_secs: Arc<AtomicU32>) {
    std::thread::Builder::new()
        .name("idle".to_owned())
        .spawn(move || {
            loop {
                if let Err(e) = watch(&events, &timeout_secs) {
                    debug!("idle: {e:#}");
                }
                if events.blocking_send(Event::IdleDetection(false)).is_err() {
                    return;
                }
                std::thread::sleep(RETRY);
            }
        })
        .expect("starting the idle thread");
}

struct State {
    events: mpsc::Sender<Event>,
}

impl Dispatch<wl_registry::WlRegistry, GlobalListContents> for State {
    fn event(
        _: &mut Self,
        _: &wl_registry::WlRegistry,
        _: wl_registry::Event,
        _: &GlobalListContents,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<wl_seat::WlSeat, ()> for State {
    fn event(_: &mut Self, _: &wl_seat::WlSeat, _: wl_seat::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {}
}

impl Dispatch<ext_idle_notifier_v1::ExtIdleNotifierV1, ()> for State {
    fn event(
        _: &mut Self,
        _: &ext_idle_notifier_v1::ExtIdleNotifierV1,
        _: ext_idle_notifier_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<ext_idle_notification_v1::ExtIdleNotificationV1, ()> for State {
    fn event(
        state: &mut Self,
        _: &ext_idle_notification_v1::ExtIdleNotificationV1,
        event: ext_idle_notification_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        let idle = match event {
            ext_idle_notification_v1::Event::Idled => true,
            ext_idle_notification_v1::Event::Resumed => false,
            _ => return,
        };
        let _ = state.events.blocking_send(Event::Idle { idle, at: now_ms() });
    }
}

/// The compositor's socket: `$WAYLAND_DISPLAY`, else the first `wayland-*`
/// in the runtime directory that answers.
fn connect() -> Result<Connection> {
    let runtime = pluscreentime_protocol::runtime_dir().context("no XDG_RUNTIME_DIR")?;
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Some(name) = std::env::var_os("WAYLAND_DISPLAY") {
        candidates.push(runtime.join(name));
    }
    let mut found: Vec<PathBuf> = std::fs::read_dir(&runtime)?
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with("wayland-") && !n.ends_with(".lock"))
        })
        .collect();
    found.sort();
    candidates.extend(found);
    for path in candidates {
        if let Ok(stream) = UnixStream::connect(&path) {
            debug!("idle: connected to {}", path.display());
            return Ok(Connection::from_socket(stream)?);
        }
    }
    anyhow::bail!("no Wayland compositor is running")
}

fn watch(events: &mpsc::Sender<Event>, timeout_secs: &AtomicU32) -> Result<()> {
    let connection = connect()?;
    let (globals, mut queue) = registry_queue_init::<State>(&connection)?;
    let qh = queue.handle();
    let seat: wl_seat::WlSeat = globals.bind(&qh, 1..=1, ()).context("no seat")?;
    let notifier: ext_idle_notifier_v1::ExtIdleNotifierV1 = globals
        .bind(&qh, 1..=2, ())
        .context("the compositor has no ext-idle-notify")?;
    let mut state = State { events: events.clone() };

    let mut timeout = timeout_secs.load(Ordering::Relaxed);
    let mut notification = notifier.get_idle_notification(timeout * 1000, &seat, &qh, ());
    let _ = events.blocking_send(Event::IdleDetection(true));

    loop {
        queue.dispatch_pending(&mut state)?;
        queue.flush()?;
        if let Some(guard) = queue.prepare_read() {
            let mut poll = libc::pollfd {
                fd: guard.connection_fd().as_raw_fd(),
                events: libc::POLLIN,
                revents: 0,
            };
            // SAFETY: one valid pollfd, for as long as the call lasts.
            let ready = unsafe { libc::poll(&mut poll, 1, 1000) };
            if ready > 0 {
                guard.read()?;
            }
        }
        queue.dispatch_pending(&mut state)?;

        // A new timeout takes a new notification. Whoever changed it is there.
        let wanted = timeout_secs.load(Ordering::Relaxed);
        if wanted != timeout {
            notification.destroy();
            timeout = wanted;
            notification = notifier.get_idle_notification(timeout * 1000, &seat, &qh, ());
            let _ = events.blocking_send(Event::Idle { idle: false, at: now_ms() });
        }
        if events.is_closed() {
            return Ok(());
        }
    }
}
