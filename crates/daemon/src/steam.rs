//! Steam games: their names and artwork, for windows of class `steam_app_<id>`.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Default, PartialEq)]
pub struct Art {
    pub icon: Option<PathBuf>,
    pub header: Option<PathBuf>,
    pub logo: Option<PathBuf>,
}

#[derive(Default)]
pub struct Steam {
    /// `None` until first used; `Some(None)` when Steam isn't installed.
    root: Option<Option<PathBuf>>,
    names: HashMap<u32, Option<String>>,
}

impl Steam {
    fn root(&mut self) -> Option<&Path> {
        self.root
            .get_or_insert_with(|| {
                let home = PathBuf::from(std::env::var_os("HOME")?);
                [
                    ".local/share/Steam",
                    ".steam/steam",
                    ".var/app/com.valvesoftware.Steam/.local/share/Steam",
                ]
                .iter()
                .map(|p| home.join(p))
                .find(|p| p.join("steamapps").is_dir())
            })
            .as_deref()
    }

    pub fn game_name(&mut self, app_id: u32) -> Option<String> {
        if let Some(name) = self.names.get(&app_id) {
            return name.clone();
        }
        let name = self.root().and_then(|root| {
            let libraries = fs::read_to_string(root.join("steamapps/libraryfolders.vdf"))
                .map(|vdf| library_paths(&vdf))
                .unwrap_or_default();
            std::iter::once(root.to_path_buf())
                .chain(libraries)
                .find_map(|lib| {
                    let manifest = lib.join(format!("steamapps/appmanifest_{app_id}.acf"));
                    manifest_name(&fs::read_to_string(manifest).ok()?)
                })
        });
        self.names.insert(app_id, name.clone());
        name
    }

    pub fn art(&mut self, app_id: u32) -> Art {
        let Some(cache) = self.root().map(|r| r.join("appcache/librarycache")) else {
            return Art::default();
        };
        // Current clients: librarycache/<id>/<hash>/library_header.jpg etc., with
        // the small icon directly in librarycache/<id>/.
        let dir = cache.join(app_id.to_string());
        let mut art = Art::default();
        if let Ok(entries) = fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    for file in fs::read_dir(&path).into_iter().flatten().flatten() {
                        match file.file_name().to_str() {
                            Some("library_header.jpg") => art.header = Some(file.path()),
                            Some("logo.png") => art.logo = Some(file.path()),
                            _ => {}
                        }
                    }
                } else if path.extension().is_some_and(|e| e == "jpg") {
                    art.icon = Some(path);
                }
            }
        }
        // Older clients kept everything flat.
        let flat =
            |suffix: &str| Some(cache.join(format!("{app_id}_{suffix}"))).filter(|p| p.is_file());
        art.icon = art.icon.or_else(|| flat("icon.jpg"));
        art.header = art.header.or_else(|| flat("header.jpg"));
        art.logo = art.logo.or_else(|| flat("logo.png"));
        art
    }
}

/// `"path"  "/mnt/games/SteamLibrary"` lines of libraryfolders.vdf.
fn library_paths(vdf: &str) -> Vec<PathBuf> {
    vdf.lines()
        .filter_map(|line| quoted_value(line, "path"))
        .map(PathBuf::from)
        .collect()
}

fn manifest_name(acf: &str) -> Option<String> {
    acf.lines().find_map(|line| quoted_value(line, "name"))
}

/// `"key"  "value"` with VDF's backslash escapes.
fn quoted_value(line: &str, key: &str) -> Option<String> {
    let rest = line.trim().strip_prefix(&format!("\"{key}\""))?;
    let value = rest.trim().strip_prefix('"')?.strip_suffix('"')?;
    Some(value.replace("\\\\", "\\").replace("\\\"", "\""))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_vdf_values() {
        let vdf = "\"libraryfolders\"\n{\n\t\"0\"\n\t{\n\t\t\"path\"\t\t\"/home/u/.local/share/Steam\"\n\
                   \t}\n\t\"1\"\n\t{\n\t\t\"path\"\t\t\"/mnt/diskGames/SteamLibrary\"\n\t}\n}\n";
        assert_eq!(
            library_paths(vdf),
            [
                PathBuf::from("/home/u/.local/share/Steam"),
                PathBuf::from("/mnt/diskGames/SteamLibrary")
            ]
        );
        let acf = "\"AppState\"\n{\n\t\"appid\"\t\t\"1062090\"\n\t\"name\"\t\t\"Timberborn\"\n}\n";
        assert_eq!(manifest_name(acf).as_deref(), Some("Timberborn"));
        assert_eq!(
            quoted_value(r#"  "name"  "Say \"Hi\" C:\\Games"  "#, "name").as_deref(),
            Some(r#"Say "Hi" C:\Games"#)
        );
    }
}
