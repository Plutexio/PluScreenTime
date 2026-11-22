//! What the time is spent on, as it happens: the focused window, whether
//! anybody is there, and the session and segment being written.
//!
//! The time counts while a desktop the daemon follows is running, the screen
//! is unlocked and there was input within the idle timeout (or something
//! keeps the screen awake). When the compositor says the user went idle, the
//! last `timeout` was nobody, so the history is cut back to the last input.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use anyhow::{Context, Result};
use pluscreentime_protocol::{
    AppInfo, Current, Request, Settings, State, Status, VERSION, error_line, is_category, ok_line,
};
use tracing::{debug, info, warn};

use crate::apps::{AppResolver, Window};
use crate::queries;
use crate::store::Store;

/// Ticks this far apart mean the machine slept (or the daemon stalled):
/// nobody was counted in between.
const GAP_MS: i64 = 15_000;

/// How often the open segment's end is written.
const FLUSH_MS: i64 = 30_000;

/// A title that changes again this soon replaces the last one instead of
/// starting a segment (apps often set a title twice while opening).
const SETTLE_MS: i64 = 1_000;

#[derive(Debug)]
pub enum Event {
    /// A desktop the daemon follows came or went.
    Desktop { name: &'static str, present: bool },
    /// That desktop's focused window; `None` while nothing has the focus.
    Focus { desktop: &'static str, window: Option<Window> },
    /// The compositor's idle notification; `at` is when it came.
    Idle { idle: bool, at: i64 },
    /// Whether the compositor can tell the daemon about idleness at all.
    IdleDetection(bool),
    Locked(bool),
}

pub fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as i64)
}

struct OpenSession {
    id: i64,
    start: i64,
}

struct OpenSegment {
    id: i64,
    app: AppInfo,
    title: String,
    title_id: Option<i64>,
    start: i64,
    /// When this app (rather than this title) got the focus.
    app_since: i64,
}

pub struct Tracker {
    store: Store,
    resolver: AppResolver,
    /// Apps met since the daemon started, by window class and desktop ID, so
    /// each is looked up (and its row refreshed) once a run.
    known: HashMap<(String, Option<String>), AppInfo>,
    settings: Settings,
    settings_path: Option<PathBuf>,
    idle_timeout: Arc<AtomicU32>,

    /// Each desktop's focused window.
    windows: HashMap<&'static str, Option<Window>>,
    desktop: Option<&'static str>,
    idle: bool,
    idle_detection: bool,
    locked: bool,

    session: Option<OpenSession>,
    segment: Option<OpenSegment>,
    last_tick: i64,
    last_flush: i64,
}

impl Tracker {
    pub fn new(
        store: Store,
        settings: Settings,
        settings_path: Option<PathBuf>,
        idle_timeout: Arc<AtomicU32>,
        now: i64,
    ) -> Self {
        idle_timeout.store(settings.idle_timeout_secs, Ordering::Relaxed);
        Self {
            store,
            resolver: AppResolver::default(),
            known: HashMap::new(),
            settings,
            settings_path,
            idle_timeout,
            windows: HashMap::new(),
            desktop: None,
            idle: false,
            idle_detection: false,
            locked: false,
            session: None,
            segment: None,
            last_tick: now,
            last_flush: now,
        }
    }

    fn counting(&self) -> bool {
        self.desktop.is_some() && !self.idle && !self.locked
    }

    fn window(&self) -> Option<&Window> {
        self.windows.get(self.desktop?)?.as_ref()
    }

    fn state(&self) -> State {
        if self.desktop.is_none() {
            State::NoDesktop
        } else if self.locked {
            State::Locked
        } else if self.idle {
            State::Idle
        } else {
            State::Active
        }
    }

    pub fn handle(&mut self, event: Event, now: i64) {
        let was = self.counting();
        // Where the time stops counting, if it does.
        let mut stop_at = now;
        match event {
            Event::Desktop { name, present } => {
                info!("{name} {}", if present { "is running" } else { "went away" });
                if present {
                    self.desktop = Some(name);
                } else if self.desktop == Some(name) {
                    self.desktop = None;
                    self.windows.remove(name);
                }
            }
            Event::Focus { desktop, window } => {
                debug!("focus on {desktop}: {window:?}");
                self.windows.insert(desktop, window);
            }
            Event::Idle { idle, at } => {
                if idle {
                    let timeout = i64::from(self.settings.idle_timeout_secs) * 1000;
                    stop_at = at - timeout;
                }
                self.idle = idle;
            }
            Event::IdleDetection(available) => {
                if available != self.idle_detection {
                    info!("idle detection {}", if available { "works" } else { "is unavailable" });
                }
                self.idle_detection = available;
                if !available {
                    self.idle = false;
                }
            }
            Event::Locked(locked) => {
                info!("screen {}", if locked { "locked" } else { "unlocked" });
                self.locked = locked;
            }
        }
        let result = match (was, self.counting()) {
            (true, false) => self.stop(stop_at.min(now), now),
            (false, true) => self.start(now),
            (true, true) => self.follow(now),
            (false, false) => Ok(()),
        };
        if let Err(e) = result {
            warn!("writing the history failed: {e:#}");
        }
    }

    pub fn tick(&mut self, now: i64) {
        let result = (|| -> Result<()> {
            if (now - self.last_tick > GAP_MS || now < self.last_tick) && self.session.is_some() {
                info!("no time passed for {} s; the machine slept", (now - self.last_tick) / 1000);
                self.stop(self.last_tick, self.last_tick)?;
                if self.counting() {
                    self.start(now)?;
                }
            }
            self.last_tick = now;
            if now - self.last_flush >= FLUSH_MS {
                self.flush(now)?;
            }
            Ok(())
        })();
        if let Err(e) = result {
            warn!("writing the history failed: {e:#}");
        }
    }

    pub fn shutdown(&mut self, now: i64) {
        if let Err(e) = self.stop(now, now) {
            warn!("writing the history failed: {e:#}");
        }
    }

    fn flush(&mut self, now: i64) -> Result<()> {
        self.last_flush = now;
        if let Some(segment) = &self.segment {
            self.store.set_segment_end(segment.id, now)?;
        }
        if let Some(session) = &self.session {
            self.store.set_session_end(session.id, now)?;
        }
        Ok(())
    }

    /// Somebody's at the screen again: a pickup.
    fn start(&mut self, now: i64) -> Result<()> {
        self.session = Some(OpenSession {
            id: self.store.open_session(now)?,
            start: now,
        });
        self.open_segment(now, now)
    }

    /// Nobody's been there since `at`.
    fn stop(&mut self, at: i64, now: i64) -> Result<()> {
        if let Some(segment) = self.segment.take() {
            self.store.set_segment_end(segment.id, at.max(segment.start))?;
        }
        if let Some(session) = self.session.take() {
            self.store.set_session_end(session.id, at.max(session.start))?;
        }
        if at < now {
            self.store.trim_after(at)?;
        }
        self.last_flush = now;
        Ok(())
    }

    /// Keeps the segment in step with the focused window.
    fn follow(&mut self, now: i64) -> Result<()> {
        let Some(window) = self.window().cloned() else {
            return self.switch(now);
        };
        let app = self.app_for(&window, now)?;
        let title = if self.settings.record_titles { window.title } else { String::new() };
        let Some(segment) = &mut self.segment else {
            return self.switch(now);
        };
        if segment.app.id != app.id {
            return self.switch(now);
        }
        if segment.title == title {
            return Ok(());
        }
        if now - segment.start < SETTLE_MS {
            let title_id = title_id(&self.store, app.id, &title)?;
            self.store.set_segment_title(segment.id, title_id)?;
            segment.title = title;
            segment.title_id = title_id;
            return Ok(());
        }
        let app_since = segment.app_since;
        self.close_segment(now)?;
        self.open_segment(now, app_since)
    }

    fn switch(&mut self, now: i64) -> Result<()> {
        self.close_segment(now)?;
        self.open_segment(now, now)
    }

    fn close_segment(&mut self, now: i64) -> Result<()> {
        if let Some(segment) = self.segment.take() {
            self.store.set_segment_end(segment.id, now)?;
        }
        Ok(())
    }

    fn open_segment(&mut self, now: i64, app_since: i64) -> Result<()> {
        let Some(session) = &self.session else {
            return Ok(());
        };
        let session = session.id;
        // With nothing focused, the time is the desktop's.
        let window = self.window().cloned().unwrap_or(Window {
            class: String::new(),
            title: String::new(),
            desktop_id: None,
        });
        let app = self.app_for(&window, now)?;
        let title = if self.settings.record_titles { window.title } else { String::new() };
        let title_id = title_id(&self.store, app.id, &title)?;
        let id = self.store.open_segment(session, app.id, title_id, now)?;
        self.segment = Some(OpenSegment {
            id,
            app,
            title,
            title_id,
            start: now,
            app_since,
        });
        Ok(())
    }

    fn app_for(&mut self, window: &Window, now: i64) -> Result<AppInfo> {
        let key = (window.class.clone(), window.desktop_id.clone());
        if let Some(app) = self.known.get(&key) {
            return Ok(app.clone());
        }
        let resolved = self.resolver.resolve(window);
        let app = self.store.upsert_app(&resolved, now)?;
        debug!("{:?} is {} ({})", window.class, app.name, app.key);
        self.known.insert(key, app.clone());
        Ok(app)
    }

    /// Answers one request with a line of JSON.
    pub fn answer(&mut self, request: Request, now: i64) -> String {
        // The open segment counts up to now.
        if let Err(e) = self.flush(now) {
            warn!("writing the history failed: {e:#}");
        }
        let result: Result<String> = match request {
            Request::Status => Ok(ok_line(&self.status())),
            Request::Summary { edges } => queries::summary(&self.store, edges).map(|s| ok_line(&s)),
            Request::AppDetail { app, edges } => {
                queries::app_detail(&self.store, app, edges).map(|d| ok_line(&d))
            }
            Request::Apps => queries::app_list(&self.store).map(|l| ok_line(&l)),
            Request::SetCategory { app, category } => self.set_category(app, category),
            Request::SetSettings { settings } => self.set_settings(settings, now),
        };
        result.unwrap_or_else(|e| error_line(&format!("{e:#}")))
    }

    fn status(&self) -> Status {
        Status {
            version: VERSION,
            state: self.state(),
            desktop: self.desktop.map(str::to_owned),
            idle_detection: self.idle_detection,
            current: self.segment.as_ref().map(|s| Current {
                app: s.app.clone(),
                title: s.title.clone(),
                since: s.app_since,
            }),
            session_start: self.session.as_ref().map(|s| s.start),
            settings: self.settings.clone(),
        }
    }

    fn set_category(&mut self, app: i64, category: Option<String>) -> Result<String> {
        if let Some(category) = &category
            && !is_category(category)
        {
            anyhow::bail!("no such category: {category}");
        }
        let info = self
            .store
            .set_category(app, category.as_deref())?
            .context("no such app")?;
        // What's cached and open carries the new category too.
        for known in self.known.values_mut().filter(|a| a.id == app) {
            *known = info.clone();
        }
        if let Some(segment) = self.segment.as_mut().filter(|s| s.app.id == app) {
            segment.app = info.clone();
        }
        Ok(ok_line(&info))
    }

    fn set_settings(&mut self, settings: Settings, now: i64) -> Result<String> {
        let settings = settings.validated();
        if let Some(path) = &self.settings_path {
            crate::config::save(path, &settings)?;
        }
        let titles_changed = settings.record_titles != self.settings.record_titles;
        self.settings = settings;
        self.idle_timeout
            .store(self.settings.idle_timeout_secs, Ordering::Relaxed);
        if titles_changed && self.counting() {
            self.switch(now)?;
        }
        Ok(ok_line(&self.settings))
    }
}

fn title_id(store: &Store, app: i64, title: &str) -> Result<Option<i64>> {
    if title.is_empty() {
        return Ok(None);
    }
    Ok(Some(store.title_id(app, title)?))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tracker() -> Tracker {
        Tracker::new(
            Store::in_memory(),
            Settings::default(),
            None,
            Arc::new(AtomicU32::new(0)),
            0,
        )
    }

    fn window(class: &str, title: &str) -> Option<Window> {
        Some(Window {
            class: class.to_owned(),
            title: title.to_owned(),
            desktop_id: None,
        })
    }

    fn summary(t: &mut Tracker, now: i64) -> pluscreentime_protocol::Summary {
        let line = t.answer(Request::Summary { edges: vec![0, 10_000_000] }, now);
        let value: serde_json::Value = serde_json::from_str(&line).unwrap();
        serde_json::from_value(value["ok"].clone()).unwrap()
    }

    #[test]
    fn counts_the_focused_app_until_idle() {
        let mut t = tracker();
        t.handle(Event::Focus { desktop: "hyprland", window: window("kitty-test", "zsh") }, 0);
        t.handle(Event::Desktop { name: "hyprland", present: true }, 1_000);
        t.handle(Event::Focus { desktop: "hyprland", window: window("zen-test", "Docs") }, 61_000);
        // Idle comes 5 min after the last input, at 400 s: input stopped at 100 s.
        t.handle(Event::Idle { idle: true, at: 400_000 }, 400_000);
        let s = summary(&mut t, 500_000);
        assert_eq!(s.total, 99_000);
        assert_eq!(s.pickups, 1);
        let by_name: HashMap<String, i64> =
            s.apps.iter().map(|a| (a.app.key.clone(), a.total)).collect();
        assert_eq!(by_name["kitty-test"], 60_000);
        assert_eq!(by_name["zen-test"], 39_000);

        // Back again: a second pickup.
        t.handle(Event::Idle { idle: false, at: 600_000 }, 600_000);
        t.handle(Event::Locked(true), 610_000);
        let s = summary(&mut t, 700_000);
        assert_eq!(s.total, 109_000);
        assert_eq!(s.pickups, 2);
    }

    #[test]
    fn idle_cuts_back_across_segments() {
        let mut t = tracker();
        t.handle(Event::Desktop { name: "hyprland", present: true }, 0);
        t.handle(Event::Focus { desktop: "hyprland", window: window("a", "") }, 0);
        // A window that grabs the focus while nobody's there.
        t.handle(Event::Focus { desktop: "hyprland", window: window("b", "") }, 350_000);
        t.handle(Event::Idle { idle: true, at: 400_000 }, 400_000);
        let s = summary(&mut t, 500_000);
        assert_eq!(s.total, 100_000);
        assert_eq!(s.apps.len(), 1);
        assert_eq!(s.apps[0].app.key, "a");
    }

    #[test]
    fn sleep_ends_the_session() {
        let mut t = tracker();
        t.handle(Event::Desktop { name: "hyprland", present: true }, 0);
        t.tick(1_000);
        t.tick(2_000);
        // An hour asleep.
        t.tick(3_602_000);
        let s = summary(&mut t, 3_612_000);
        assert_eq!(s.total, 2_000 + 10_000);
        assert_eq!(s.pickups, 2);
    }

    #[test]
    fn titles_settle_and_split() {
        let mut t = tracker();
        t.handle(Event::Desktop { name: "hyprland", present: true }, 0);
        t.handle(Event::Focus { desktop: "hyprland", window: window("zen-test", "New Tab") }, 0);
        t.handle(Event::Focus { desktop: "hyprland", window: window("zen-test", "Docs") }, 200);
        t.handle(Event::Focus { desktop: "hyprland", window: window("zen-test", "Mail") }, 60_000);
        let app = t.segment.as_ref().unwrap().app.id;
        let line = t.answer(Request::AppDetail { app, edges: vec![0, 100_000] }, 90_000);
        let value: serde_json::Value = serde_json::from_str(&line).unwrap();
        let titles = &value["ok"]["titles"];
        assert_eq!(titles[0]["title"], "Docs");
        assert_eq!(titles[0]["total"], 60_000);
        assert_eq!(titles[1]["title"], "Mail");
        assert_eq!(titles.as_array().unwrap().len(), 2);
        assert_eq!(value["ok"]["opens"], 1);
    }

    #[test]
    fn nothing_counts_without_a_desktop() {
        let mut t = tracker();
        t.handle(Event::Focus { desktop: "kwin", window: window("a", "") }, 0);
        let s = summary(&mut t, 100_000);
        assert_eq!(s.total, 0);
    }
}
