//! What the daemon and the app say to each other.
//!
//! The daemon listens on a Unix socket in the user's runtime directory
//! ([`socket_path`]), reachable only by its owner. Each request is one line of
//! JSON ([`Request`]), and each answer one line back: `{"ok": …}` with the
//! reply types below, or `{"error": "…"}`. A connection may carry any number of
//! requests, one after another.
//!
//! Every time is in milliseconds since the Unix epoch. Ranges are given as
//! `edges`: n + 1 ascending times that cut the range into n buckets (the hours
//! of a day, the days of a week). The app works them out in local time, so the
//! daemon never needs to know about time zones or daylight saving.

use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Bumped with every change to the messages.
pub const VERSION: u32 = 1;

/// The categories an app can be in, in the order their colours are given out
/// and they stack in charts. "other" is last and neutral.
pub const CATEGORIES: [&str; 9] = [
    "web",
    "creativity",
    "productivity",
    "utilities",
    "social",
    "games",
    "development",
    "entertainment",
    "other",
];

pub fn is_category(id: &str) -> bool {
    CATEGORIES.contains(&id)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Request {
    /// → [`Status`]
    Status,
    /// → [`Summary`]
    Summary { edges: Vec<i64> },
    /// → [`AppDetail`]
    AppDetail { app: i64, edges: Vec<i64> },
    /// Every app ever seen, with its time over all history. → [`AppList`]
    Apps,
    /// Puts an app in a category of the user's choosing, or back in the one
    /// found for it (`None`). → [`AppInfo`]
    SetCategory { app: i64, category: Option<String> },
    /// → [`Settings`], as kept
    SetSettings { settings: Settings },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// No input for this long and the time stops counting, from the last input
    /// on. What keeps the screen awake (a video, a game) keeps it counting.
    pub idle_timeout_secs: u32,
    /// Keep the window titles, to show what an app was used for.
    pub record_titles: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            idle_timeout_secs: 300,
            record_titles: true,
        }
    }
}

impl Settings {
    pub const MIN_IDLE_SECS: u32 = 30;
    pub const MAX_IDLE_SECS: u32 = 3600;

    pub fn validated(mut self) -> Self {
        self.idle_timeout_secs = self
            .idle_timeout_secs
            .clamp(Self::MIN_IDLE_SECS, Self::MAX_IDLE_SECS);
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum State {
    /// Someone is at the screen; the time counts.
    Active,
    /// No input for the idle timeout.
    Idle,
    /// The screen is locked.
    Locked,
    /// No desktop the daemon can follow (Hyprland or KDE Plasma) is running.
    NoDesktop,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Status {
    pub version: u32,
    pub state: State,
    /// "hyprland" or "kwin": where the focused window comes from.
    pub desktop: Option<String>,
    /// The compositor tells the daemon when nobody's there. Without it the
    /// time counts until the screen locks.
    pub idle_detection: bool,
    pub current: Option<Current>,
    /// When the current stretch at the screen began.
    pub session_start: Option<i64>,
    pub settings: Settings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Current {
    pub app: AppInfo,
    pub title: String,
    pub since: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppInfo {
    pub id: i64,
    /// What the app was recognised by: its desktop file ID or window class.
    pub key: String,
    pub name: String,
    /// Path of its icon, when one was found.
    pub icon: Option<String>,
    /// Path of a wide banner (Steam games).
    pub art: Option<String>,
    pub category: String,
    /// The category was picked by the user rather than found.
    pub custom_category: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Summary {
    pub edges: Vec<i64>,
    pub total: i64,
    pub buckets: Vec<Bucket>,
    /// Most used first.
    pub apps: Vec<AppUsage>,
    /// Most used first; only categories with time.
    pub categories: Vec<CategoryUsage>,
    /// Stretches at the screen that began in the range.
    pub pickups: u32,
    /// The longest stretch at the screen, as far as it lies in the range.
    pub longest_session: i64,
    pub first_activity: Option<i64>,
    pub last_activity: Option<i64>,
    /// Every stretch at the screen, cut to the range: `[start, end]`.
    pub sessions: Vec<[i64; 2]>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Bucket {
    pub total: i64,
    pub categories: BTreeMap<String, i64>,
    pub pickups: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppUsage {
    pub app: AppInfo,
    pub total: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryUsage {
    pub id: String,
    pub total: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppDetail {
    pub app: AppInfo,
    pub total: i64,
    pub buckets: Vec<i64>,
    /// Most used first, at most [`AppDetail::MAX_TITLES`].
    pub titles: Vec<TitleUsage>,
    /// How often it was switched to.
    pub opens: u32,
    /// The longest it was used without switching away.
    pub longest: i64,
    pub first_seen: i64,
    pub last_used: Option<i64>,
}

impl AppDetail {
    pub const MAX_TITLES: usize = 100;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TitleUsage {
    pub title: String,
    pub total: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppList {
    pub apps: Vec<AppTotal>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppTotal {
    pub app: AppInfo,
    pub total: i64,
    pub last_used: Option<i64>,
}

/// One answer line.
pub fn ok_line<T: Serialize>(value: &T) -> String {
    serde_json::json!({ "ok": value }).to_string()
}

pub fn error_line(message: &str) -> String {
    serde_json::json!({ "error": message }).to_string()
}

fn home() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}

fn xdg(var: &str, fallback: &str) -> Option<PathBuf> {
    std::env::var_os(var)
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .or_else(|| Some(home()?.join(fallback)))
}

/// `~/.config/pluscreentime`: the daemon's settings and the app's preferences.
pub fn config_dir() -> Option<PathBuf> {
    Some(xdg("XDG_CONFIG_HOME", ".config")?.join("pluscreentime"))
}

/// `~/.local/share/pluscreentime`: the history.
pub fn data_dir() -> Option<PathBuf> {
    Some(xdg("XDG_DATA_HOME", ".local/share")?.join("pluscreentime"))
}

/// The user's runtime directory, `/run/user/<uid>`.
pub fn runtime_dir() -> Option<PathBuf> {
    std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
}

pub fn socket_path() -> Option<PathBuf> {
    Some(runtime_dir()?.join("pluscreentime.sock"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn requests_are_tagged() {
        let line = serde_json::to_string(&Request::Summary { edges: vec![0, 10] }).unwrap();
        assert_eq!(line, r#"{"type":"summary","edges":[0,10]}"#);
        let back: Request = serde_json::from_str(r#"{"type":"status"}"#).unwrap();
        assert!(matches!(back, Request::Status));
    }

    #[test]
    fn settings_are_kept_in_bounds() {
        let settings = Settings { idle_timeout_secs: 1, record_titles: false }.validated();
        assert_eq!(settings.idle_timeout_secs, Settings::MIN_IDLE_SECS);
        let read: Settings = serde_json::from_str("{}").unwrap();
        assert_eq!(read, Settings::default());
    }
}
