//! The `Config` QML singleton: preferences and themes on disk.
//!
//! Structured data crosses into QML as JSON strings; parsing and validation
//! stay on the Rust side so QML only ever sees well-formed values.

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
    }

    #[auto_cxx_name]
    extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qml_singleton]
        type Config = super::ConfigRust;

        /// `{ "theme": id, "theme_mode", "span", … }` (see `Prefs`)
        #[qinvokable]
        fn preferences(&self) -> QString;

        #[qinvokable]
        fn save_preferences(self: Pin<&mut Self>, json: &QString);

        /// Summaries of every theme, with swatches for light or dark, for the picker.
        #[qinvokable]
        fn themes(&self, dark: bool) -> QString;

        /// The scheme of theme `id` (see `theme::Scheme::to_json`). A theme that
        /// cannot be loaded falls back to the app's accent and reports why
        /// through `errorOccurred`. Starts watching what the scheme came from.
        #[qinvokable]
        fn scheme(self: Pin<&mut Self>, id: &QString, dark: bool) -> QString;

        /// `file://` URL of a bundled font, or empty when it isn't installed.
        #[qinvokable]
        fn font_file(&self, name: &QString) -> QString;

        /// `file://` URL of the user themes folder, created if missing.
        #[qinvokable]
        fn themes_folder(self: Pin<&mut Self>) -> QString;

        #[qsignal]
        fn error_occurred(self: Pin<&mut Self>, message: &QString);

        /// A file or folder the current scheme came from changed on disk.
        #[qsignal]
        fn scheme_files_changed(self: Pin<&mut Self>);
    }

    impl cxx_qt::Threading for Config {}
}

use std::fs;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

use anyhow::{Context, Result};
use cxx_qt::{CxxQtType, Threading};
use cxx_qt_lib::QString;

use crate::paths;
use crate::prefs::Prefs;
use crate::theme;

const WATCH_INTERVAL: Duration = Duration::from_millis(700);

type Watched = Arc<Mutex<Vec<(PathBuf, Option<SystemTime>)>>>;

#[derive(Default)]
pub struct ConfigRust {
    watched: Watched,
    watcher_started: bool,
}

fn modified(path: &PathBuf) -> Option<SystemTime> {
    fs::metadata(path).and_then(|m| m.modified()).ok()
}

impl qobject::Config {
    pub fn preferences(&self) -> QString {
        let prefs = paths::prefs_file()
            .ok()
            .and_then(|path| fs::read_to_string(path).ok())
            .and_then(|text| toml::from_str::<Prefs>(&text).ok())
            .unwrap_or_default();
        QString::from(&serde_json::to_string(&prefs).expect("prefs serialize"))
    }

    pub fn save_preferences(self: Pin<&mut Self>, json: &QString) {
        let result = (|| -> Result<()> {
            let prefs: Prefs =
                serde_json::from_str(&json.to_string()).context("malformed preferences")?;
            paths::write_atomic(&paths::prefs_file()?, &toml::to_string_pretty(&prefs)?)
        })();
        self.report(result);
    }

    pub fn themes(&self, dark: bool) -> QString {
        let dir = paths::themes_dir().ok();
        let list = theme::list(dark, dir.as_deref());
        QString::from(&serde_json::to_string(&list).expect("theme list serializes"))
    }

    pub fn scheme(mut self: Pin<&mut Self>, id: &QString, dark: bool) -> QString {
        let dir = paths::themes_dir().ok();
        let (resolved, error) = theme::resolve(&id.to_string(), dark, dir.as_deref());
        *self.rust().watched.lock().unwrap() = resolved
            .watch
            .into_iter()
            .map(|path| {
                let seen = modified(&path);
                (path, seen)
            })
            .collect();
        self.as_mut().start_watcher();
        if let Some(e) = error {
            self.as_mut().report(Err(e));
        }
        QString::from(&resolved.scheme.to_json().to_string())
    }

    pub fn font_file(&self, name: &QString) -> QString {
        let name = name.to_string();
        paths::font_dirs()
            .into_iter()
            .map(|dir| dir.join(&name))
            .find(|path| path.is_file())
            .map(|path| QString::from(&format!("file://{}", path.display())))
            .unwrap_or_default()
    }

    pub fn themes_folder(self: Pin<&mut Self>) -> QString {
        let result = paths::themes_dir().and_then(|dir| {
            fs::create_dir_all(&dir).with_context(|| format!("creating {}", dir.display()))?;
            Ok(dir)
        });
        match result {
            Ok(dir) => QString::from(&format!("file://{}", dir.display())),
            Err(e) => {
                self.report(Err(e));
                QString::default()
            }
        }
    }

    fn report(self: Pin<&mut Self>, result: Result<()>) {
        if let Err(e) = result {
            self.error_occurred(&QString::from(&format!("{e:#}")));
        }
    }

    /// Polls what the current scheme came from; cheap enough and needs no
    /// inotify plumbing. A folder's time changes as files come and go in it.
    fn start_watcher(mut self: Pin<&mut Self>) {
        if self.rust().watcher_started {
            return;
        }
        self.as_mut().rust_mut().watcher_started = true;
        let watched = self.rust().watched.clone();
        let qt_thread = self.qt_thread();
        std::thread::spawn(move || {
            loop {
                std::thread::sleep(WATCH_INTERVAL);
                let changed =
                    watched
                        .lock()
                        .unwrap()
                        .iter_mut()
                        .fold(false, |changed, (path, seen)| {
                            let now = modified(path);
                            let differs = now != *seen;
                            *seen = now;
                            changed | differs
                        });
                if changed
                    && qt_thread
                        .queue(|config| config.scheme_files_changed())
                        .is_err()
                {
                    break;
                }
            }
        });
    }
}
