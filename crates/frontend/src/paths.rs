use std::fs::{self, OpenOptions};
use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

fn config_dir() -> Result<PathBuf> {
    pluscreentime_protocol::config_dir().context("cannot determine the config directory")
}

pub fn themes_dir() -> Result<PathBuf> {
    Ok(config_dir()?.join("themes"))
}

pub fn prefs_file() -> Result<PathBuf> {
    Ok(config_dir()?.join("app.toml"))
}

/// Where Google Sans Flex and Material Symbols may be installed beside the app
/// (`install.sh` puts them in the prefix's share folder), when the system has
/// none of its own. PluSystemMonitor and Swiftify ship the same two faces.
pub fn font_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    let names = ["pluscreentime", "plusysmon", "swiftify"];
    if let Some(prefix) = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent()?.parent().map(Path::to_path_buf))
    {
        dirs.extend(names.iter().map(|n| prefix.join("share").join(n).join("fonts")));
    }
    let data_home = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/share")));
    if let Some(data_home) = data_home {
        dirs.extend(names.iter().map(|n| data_home.join(n).join("fonts")));
    }
    for prefix in ["/usr/local/share", "/usr/share"] {
        dirs.extend(names.iter().map(|n| Path::new(prefix).join(n).join("fonts")));
    }
    dirs
}

/// Writes via a temporary file and rename, so a crash never leaves a half-written
/// file behind.
pub fn write_atomic(path: &Path, contents: &str) -> Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;
    }
    let tmp = path.with_extension("tmp");
    OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(&tmp)
        .and_then(|mut f| f.write_all(contents.as_bytes()))
        .with_context(|| format!("writing {}", tmp.display()))?;
    fs::rename(&tmp, path).with_context(|| format!("replacing {}", path.display()))
}
