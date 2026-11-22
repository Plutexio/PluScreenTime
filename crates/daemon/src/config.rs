//! The daemon's settings, `~/.config/pluscreentime/daemon.toml`, as the app
//! sets them.

use std::path::Path;

use anyhow::{Context, Result};
use pluscreentime_protocol::Settings;
use tracing::warn;

pub fn load(path: &Path) -> Settings {
    match std::fs::read_to_string(path) {
        Ok(text) => match toml::from_str::<Settings>(&text) {
            Ok(settings) => settings.validated(),
            Err(e) => {
                warn!("ignoring {}: {e}", path.display());
                Settings::default()
            }
        },
        Err(_) => Settings::default(),
    }
}

pub fn save(path: &Path, settings: &Settings) -> Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;
    }
    let text = format!(
        "# PluScreenTime's daemon settings, as set in the app.\n{}",
        toml::to_string_pretty(settings)?
    );
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, text).with_context(|| format!("writing {}", tmp.display()))?;
    std::fs::rename(&tmp, path).with_context(|| format!("replacing {}", path.display()))
}
