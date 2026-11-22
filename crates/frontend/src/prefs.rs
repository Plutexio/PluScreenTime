//! The app's preferences (`app.toml`). Unknown keys are ignored and missing
//! ones defaulted, so the file stays compatible across versions.

use serde::{Deserialize, Serialize};

use crate::theme::DEFAULT_SOURCE;

#[derive(Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Prefs {
    /// A theme ID: "system" for the desktop's Matugen scheme, a built-in, or a
    /// file in the themes folder.
    pub theme: String,
    /// auto (as the desktop prefers), light or dark.
    pub theme_mode: String,
    /// Stills what moves on its own, over and over.
    pub reduce_motion: bool,
    /// No animations at all, not even fades.
    pub performance_mode: bool,
    /// No decorative shapes; the ones that mean something stay.
    pub no_shapes: bool,
    pub sidebar_width: f64,
    /// The page shown last (0 overview, 1 apps), restored on start.
    pub page: u8,
    /// The overview's span: day or week.
    pub span: String,
    /// How the Apps page sorts: time or name.
    pub apps_sort: String,
}

impl Default for Prefs {
    fn default() -> Self {
        Self {
            theme: DEFAULT_SOURCE.to_owned(),
            theme_mode: "auto".to_owned(),
            reduce_motion: false,
            performance_mode: false,
            no_shapes: false,
            sidebar_width: 232.0,
            page: 0,
            span: "day".to_owned(),
            apps_sort: "time".to_owned(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn older_files_get_defaults() {
        let read: Prefs = toml::from_str("theme = \"nord\"\nunknown = 1\n").unwrap();
        assert_eq!(read.theme, "nord");
        assert_eq!(read.span, "day");
    }
}
