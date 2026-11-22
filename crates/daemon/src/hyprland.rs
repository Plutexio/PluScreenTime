//! The focused window on Hyprland, from its event socket.
//!
//! `$XDG_RUNTIME_DIR/hypr/<instance>/.socket2.sock` streams lines like
//! `activewindow>>CLASS,TITLE`, `activewindowv2>>ADDRESS` and
//! `windowtitlev2>>ADDRESS,TITLE`; `.socket.sock` answers `j/activewindow`
//! for the window focused when the daemon arrives. A user service may start
//! before Hyprland and outlive it, so the instance is looked for again and
//! again rather than taken from the environment once.

use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{Context, Result};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;
use tokio::sync::mpsc;
use tracing::debug;

use crate::apps::Window;
use crate::tracker::Event;

const NAME: &str = "hyprland";
const RETRY: Duration = Duration::from_secs(3);

pub async fn run(events: mpsc::Sender<Event>) {
    loop {
        if let Some((dir, stream)) = find_instance().await {
            if let Err(e) = follow(&dir, stream, &events).await {
                debug!("hyprland: {e:#}");
            }
            let _ = events.send(Event::Focus { desktop: NAME, window: None }).await;
            let _ = events.send(Event::Desktop { name: NAME, present: false }).await;
        }
        tokio::time::sleep(RETRY).await;
    }
}

/// The running instance: the one the environment names, else whichever
/// answers, newest first (old instances leave their folders behind).
async fn find_instance() -> Option<(PathBuf, UnixStream)> {
    let root = pluscreentime_protocol::runtime_dir()?.join("hypr");
    let mut candidates: Vec<(std::time::SystemTime, PathBuf)> = std::fs::read_dir(&root)
        .ok()?
        .flatten()
        .filter_map(|entry| {
            let modified = entry.metadata().and_then(|m| m.modified()).ok()?;
            Some((modified, entry.path()))
        })
        .collect();
    candidates.sort_by_key(|c| std::cmp::Reverse(c.0));
    let named = std::env::var_os("HYPRLAND_INSTANCE_SIGNATURE").map(|sig| root.join(sig));
    for dir in named.into_iter().chain(candidates.into_iter().map(|(_, dir)| dir)) {
        if let Ok(stream) = UnixStream::connect(dir.join(".socket2.sock")).await {
            return Some((dir, stream));
        }
    }
    None
}

async fn follow(dir: &Path, events_socket: UnixStream, events: &mpsc::Sender<Event>) -> Result<()> {
    let mut focused = active_window(dir).await.unwrap_or_default();
    // The window first, so the time doesn't start on the desktop.
    let _ = events
        .send(Event::Focus { desktop: NAME, window: focused.window() })
        .await;
    let _ = events.send(Event::Desktop { name: NAME, present: true }).await;

    let mut lines = BufReader::new(events_socket).lines();
    while let Some(line) = lines.next_line().await.context("reading events")? {
        let Some((name, data)) = line.split_once(">>") else {
            continue;
        };
        let changed = match name {
            "activewindow" => {
                let (class, title) = data.split_once(',').unwrap_or((data, ""));
                focused.class = class.to_owned();
                focused.title = title.to_owned();
                // Its address follows in activewindowv2.
                focused.address.clear();
                true
            }
            "activewindowv2" => {
                focused.address = address(data);
                false
            }
            "windowtitlev2" => {
                let (window, title) = data.split_once(',').unwrap_or((data, ""));
                let ours = !focused.address.is_empty() && address(window) == focused.address;
                if ours && focused.title != title {
                    focused.title = title.to_owned();
                }
                ours
            }
            _ => false,
        };
        if changed {
            let _ = events
                .send(Event::Focus { desktop: NAME, window: focused.window() })
                .await;
        }
    }
    Ok(())
}

#[derive(Debug, Default)]
struct Focused {
    class: String,
    title: String,
    address: String,
}

impl Focused {
    fn window(&self) -> Option<Window> {
        (!self.class.is_empty() || !self.title.is_empty()).then(|| Window {
            class: self.class.clone(),
            title: self.title.clone(),
            desktop_id: None,
        })
    }
}

/// `0x55d0f6c1e3a0` and `55d0f6c1e3a0` are the same window.
fn address(text: &str) -> String {
    text.trim().trim_start_matches("0x").to_lowercase()
}

async fn active_window(dir: &Path) -> Result<Focused> {
    let mut socket = UnixStream::connect(dir.join(".socket.sock")).await?;
    socket.write_all(b"j/activewindow").await?;
    let mut reply = String::new();
    tokio::time::timeout(Duration::from_secs(2), socket.read_to_string(&mut reply))
        .await
        .context("Hyprland didn't answer")??;
    let json: serde_json::Value = serde_json::from_str(&reply).context("reading activewindow")?;
    let text = |key: &str| json.get(key).and_then(|v| v.as_str()).unwrap_or_default().to_owned();
    Ok(Focused {
        class: text("class"),
        title: text("title"),
        address: address(&text("address")),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn addresses_compare_with_or_without_prefix() {
        assert_eq!(address("0x557558FC4B60"), address("557558fc4b60"));
    }

    #[test]
    fn an_empty_focus_is_no_window() {
        assert_eq!(Focused::default().window(), None);
    }
}
