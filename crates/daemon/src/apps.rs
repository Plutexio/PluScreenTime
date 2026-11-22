//! Telling which app a window belongs to: its desktop entry (name, icon,
//! categories), a Steam game, or failing those its window class, made
//! presentable.

use crate::desktop::DesktopIndex;
use crate::icons::IconLookup;
use crate::steam::Steam;

/// The focused window, as the desktop reports it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Window {
    /// Wayland app ID or X11 class (`resourceClass` on KWin).
    pub class: String,
    pub title: String,
    /// The desktop file the compositor matched it to, when it says (KWin).
    pub desktop_id: Option<String>,
}

/// What a window was recognised as, before it has a row of its own.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resolved {
    pub key: String,
    pub name: String,
    pub icon: Option<String>,
    pub art: Option<String>,
    pub category: String,
}

/// The key for time spent with no window focused (the wallpaper, a panel).
pub const DESKTOP_KEY: &str = "desktop";

/// Windows that are the desktop itself rather than an app.
const DESKTOP_CLASSES: &[&str] = &["", "plasmashell", "org.kde.plasmashell", "hyprpaper", "swww"];

/// Apps whose desktop files say too little (or the wrong thing) for their
/// category, by lowercased key.
const KNOWN: &[(&str, &str)] = &[
    ("discord", "social"),
    ("com.discordapp.discord", "social"),
    ("vesktop", "social"),
    ("dev.vencord.vesktop", "social"),
    ("signal", "social"),
    ("org.signal.signal", "social"),
    ("org.telegram.desktop", "social"),
    ("telegram-desktop", "social"),
    ("slack", "social"),
    ("element", "social"),
    ("spotify", "entertainment"),
    ("com.spotify.client", "entertainment"),
    ("mpv", "entertainment"),
    ("vlc", "entertainment"),
    ("steam", "games"),
    ("com.valvesoftware.steam", "games"),
    ("heroic", "games"),
    ("lutris", "games"),
    ("net.lutris.lutris", "games"),
    ("obsidian", "productivity"),
    ("md.obsidian.obsidian", "productivity"),
    ("kitty", "development"),
    ("alacritty", "development"),
    ("foot", "development"),
    ("com.mitchellh.ghostty", "development"),
    ("org.wezfurlong.wezterm", "development"),
    ("code", "development"),
    ("code-oss", "development"),
    ("cursor", "development"),
    ("zed", "development"),
    ("dev.zed.zed", "development"),
    ("jetbrains-idea", "development"),
    ("jetbrains-clion", "development"),
    ("jetbrains-rustrover", "development"),
    ("blender", "creativity"),
    ("gimp", "creativity"),
    ("org.inkscape.inkscape", "creativity"),
    ("org.kde.krita", "creativity"),
    ("com.obsproject.studio", "creativity"),
    ("org.kde.dolphin", "utilities"),
    ("org.gnome.nautilus", "utilities"),
    ("thunar", "utilities"),
    ("pavucontrol", "utilities"),
    (DESKTOP_KEY, "other"),
];

/// Freedesktop categories, most telling first: a video editor is AudioVideo
/// too, but it's creative work.
const BY_DESKTOP_CATEGORY: &[(&str, &str)] = &[
    ("Game", "games"),
    ("Development", "development"),
    ("IDE", "development"),
    ("TerminalEmulator", "development"),
    ("AudioVideoEditing", "creativity"),
    ("Graphics", "creativity"),
    ("2DGraphics", "creativity"),
    ("3DGraphics", "creativity"),
    ("RasterGraphics", "creativity"),
    ("VectorGraphics", "creativity"),
    ("Photography", "creativity"),
    ("Recorder", "creativity"),
    ("InstantMessaging", "social"),
    ("Chat", "social"),
    ("IRCClient", "social"),
    ("VideoConference", "social"),
    ("WebBrowser", "web"),
    ("Email", "productivity"),
    ("Office", "productivity"),
    ("Calendar", "productivity"),
    ("ContactManagement", "productivity"),
    ("Finance", "productivity"),
    ("TextEditor", "productivity"),
    ("WordProcessor", "productivity"),
    ("Spreadsheet", "productivity"),
    ("Presentation", "productivity"),
    ("Education", "productivity"),
    ("Science", "productivity"),
    ("Player", "entertainment"),
    ("Music", "entertainment"),
    ("Video", "entertainment"),
    ("TV", "entertainment"),
    ("AudioVideo", "entertainment"),
    ("Audio", "entertainment"),
    ("FileManager", "utilities"),
    ("Settings", "utilities"),
    ("System", "utilities"),
    ("Monitor", "utilities"),
    ("Utility", "utilities"),
];

#[derive(Default)]
pub struct AppResolver {
    desktop: DesktopIndex,
    icons: IconLookup,
    steam: Steam,
}

impl AppResolver {
    pub fn resolve(&mut self, window: &Window) -> Resolved {
        let class = window.class.trim();
        if DESKTOP_CLASSES.contains(&class.to_lowercase().as_str()) {
            return Resolved {
                key: DESKTOP_KEY.to_owned(),
                name: "Desktop".to_owned(),
                icon: None,
                art: None,
                category: "other".to_owned(),
            };
        }

        if let Some(app_id) = class
            .strip_prefix("steam_app_")
            .and_then(|id| id.parse::<u32>().ok())
            .filter(|&id| id != 0)
        {
            let art = self.steam.art(app_id);
            let icon = art.icon.or_else(|| self.icons.find(&format!("steam_icon_{app_id}")));
            return Resolved {
                key: format!("steam:{app_id}"),
                name: self
                    .steam
                    .game_name(app_id)
                    .unwrap_or_else(|| format!("Steam game {app_id}")),
                icon: icon.map(|p| p.display().to_string()),
                art: art.header.map(|p| p.display().to_string()),
                category: "games".to_owned(),
            };
        }

        let entry = window
            .desktop_id
            .as_deref()
            .filter(|id| !id.is_empty())
            .and_then(|id| self.desktop.lookup(id))
            .or_else(|| self.desktop.lookup(class))
            // Reverse-DNS classes of apps whose desktop file is named otherwise.
            .or_else(|| self.desktop.lookup(class.rsplit('.').next()?));
        match entry {
            Some(entry) => {
                let icon = entry
                    .icon
                    .as_deref()
                    .and_then(|name| self.icons.find(name))
                    .or_else(|| self.icons.find(class));
                let key = entry.id.to_lowercase();
                Resolved {
                    category: category_for(&key, class, &entry.categories),
                    key,
                    name: entry.name,
                    icon: icon.map(|p| p.display().to_string()),
                    art: None,
                }
            }
            None => {
                let key = class.to_lowercase();
                Resolved {
                    category: category_for(&key, class, &[]),
                    name: pretty_name(class),
                    icon: self.icons.find(class).map(|p| p.display().to_string()),
                    art: None,
                    key,
                }
            }
        }
    }
}

fn category_for(key: &str, class: &str, categories: &[String]) -> String {
    let known = |k: &str| KNOWN.iter().find(|(id, _)| *id == k).map(|(_, c)| *c);
    if let Some(category) = known(key).or_else(|| known(&class.to_lowercase())) {
        return category.to_owned();
    }
    BY_DESKTOP_CATEGORY
        .iter()
        .find(|(name, _)| categories.iter().any(|c| c == name))
        .map_or("other", |(_, category)| category)
        .to_owned()
}

/// `org.example.cool-app` → "Cool app".
fn pretty_name(class: &str) -> String {
    let last = class.rsplit('.').next().unwrap_or(class);
    let spaced = last.replace(['-', '_'], " ");
    let mut chars = spaced.trim().chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => class.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strings(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn picks_the_most_telling_category() {
        assert_eq!(category_for("org.kde.kdenlive", "", &strings(&["AudioVideo", "AudioVideoEditing"])), "creativity");
        assert_eq!(category_for("firefox", "", &strings(&["Network", "WebBrowser"])), "web");
        assert_eq!(category_for("x", "", &strings(&["Utility"])), "utilities");
        assert_eq!(category_for("x", "", &[]), "other");
        // What's known by name wins over what the file says.
        assert_eq!(category_for("vesktop", "", &strings(&["Network"])), "social");
        assert_eq!(category_for("org.foo", "kitty", &strings(&["System"])), "development");
    }

    #[test]
    fn makes_classes_presentable() {
        assert_eq!(pretty_name("org.example.cool-app"), "Cool app");
        assert_eq!(pretty_name("zen"), "Zen");
        assert_eq!(pretty_name(""), "");
    }

    #[test]
    fn the_desktop_is_not_an_app() {
        let mut resolver = AppResolver::default();
        let window = Window { class: String::new(), title: String::new(), desktop_id: None };
        assert_eq!(resolver.resolve(&window).key, DESKTOP_KEY);
    }
}
