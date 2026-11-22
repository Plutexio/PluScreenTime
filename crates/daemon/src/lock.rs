//! Whether the screen is locked: a screen locker of this user is running.
//!
//! Lockers announce themselves nowhere common (hyprlock and swaylock lock
//! through the compositor, which tells no other client), but each is a
//! process of its own while the screen is locked, KDE's greeter included.

use std::os::unix::fs::MetadataExt;
use std::time::Duration;

use tokio::sync::mpsc;

use crate::tracker::Event;

const POLL: Duration = Duration::from_secs(2);

/// As `/proc/<pid>/comm` has them: cut to 15 bytes.
const LOCKERS: &[&str] = &[
    "hyprlock",
    "swaylock",
    "gtklock",
    "waylock",
    "kscreenlocker_g",
    "i3lock",
    "physlock",
];

pub async fn run(events: mpsc::Sender<Event>) {
    // SAFETY: getuid cannot fail.
    let uid = unsafe { libc::getuid() };
    let mut locked = false;
    loop {
        let now = locker_running(uid);
        if now != locked {
            locked = now;
            if events.send(Event::Locked(locked)).await.is_err() {
                return;
            }
        }
        tokio::time::sleep(POLL).await;
    }
}

fn locker_running(uid: u32) -> bool {
    let Ok(entries) = std::fs::read_dir("/proc") else {
        return false;
    };
    entries.flatten().any(|entry| {
        let name = entry.file_name();
        let Some(pid) = name.to_str().filter(|n| n.bytes().all(|b| b.is_ascii_digit())) else {
            return false;
        };
        entry.metadata().is_ok_and(|m| m.uid() == uid)
            && std::fs::read_to_string(format!("/proc/{pid}/comm"))
                .is_ok_and(|comm| LOCKERS.contains(&comm.trim_end()))
    })
}
