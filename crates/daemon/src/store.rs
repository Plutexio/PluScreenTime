//! The history, in SQLite.
//!
//! A *session* is a stretch at the screen: it begins with the first input
//! after being away (idle, locked, asleep, logged out) and ends when that
//! begins again. A *segment* is part of a session spent in one window under
//! one title. Both are written as they open and their ends moved forward as
//! they go on, so a crash loses at most the time since the last flush.

use std::path::Path;

use anyhow::{Context, Result};
use pluscreentime_protocol::AppInfo;
use rusqlite::{Connection, OptionalExtension, params};

use crate::apps::Resolved;

const SCHEMA_VERSION: i32 = 1;

const SCHEMA: &str = "
CREATE TABLE apps (
    id INTEGER PRIMARY KEY,
    key TEXT NOT NULL UNIQUE,
    name TEXT NOT NULL,
    icon TEXT,
    art TEXT,
    category TEXT NOT NULL,
    -- Picked by the user; wins over `category`.
    user_category TEXT,
    first_seen INTEGER NOT NULL
);
CREATE TABLE titles (
    id INTEGER PRIMARY KEY,
    app INTEGER NOT NULL REFERENCES apps(id),
    text TEXT NOT NULL,
    UNIQUE (app, text)
);
CREATE TABLE sessions (
    id INTEGER PRIMARY KEY,
    start INTEGER NOT NULL,
    end INTEGER NOT NULL
);
CREATE TABLE segments (
    id INTEGER PRIMARY KEY,
    session INTEGER NOT NULL REFERENCES sessions(id),
    app INTEGER NOT NULL REFERENCES apps(id),
    title INTEGER REFERENCES titles(id),
    start INTEGER NOT NULL,
    end INTEGER NOT NULL
);
CREATE INDEX segments_by_end ON segments(end);
CREATE INDEX segments_by_app ON segments(app, end);
CREATE INDEX sessions_by_end ON sessions(end);
";

/// A segment as the queries need it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Segment {
    pub session: i64,
    pub app: i64,
    pub title: Option<i64>,
    pub start: i64,
    pub end: i64,
}

pub struct Store {
    db: Connection,
}

impl Store {
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;
        }
        let db = Connection::open(path).with_context(|| format!("opening {}", path.display()))?;
        Self::init(db)
    }

    #[cfg(test)]
    pub fn in_memory() -> Self {
        Self::init(Connection::open_in_memory().unwrap()).unwrap()
    }

    fn init(db: Connection) -> Result<Self> {
        db.pragma_update(None, "journal_mode", "WAL")?;
        db.pragma_update(None, "synchronous", "NORMAL")?;
        db.pragma_update(None, "foreign_keys", true)?;
        let version: i32 = db.pragma_query_value(None, "user_version", |row| row.get(0))?;
        match version {
            0 => {
                db.execute_batch(SCHEMA).context("creating the database")?;
                db.pragma_update(None, "user_version", SCHEMA_VERSION)?;
            }
            SCHEMA_VERSION => {}
            newer => anyhow::bail!(
                "the history was written by a newer PluScreenTime (schema {newer}); update this one"
            ),
        }
        Ok(Self { db })
    }

    /// Finds or adds the app, refreshing what was found about it (a new
    /// icon, a renamed desktop entry). The user's category stays.
    pub fn upsert_app(&self, app: &Resolved, now: i64) -> Result<AppInfo> {
        self.db
            .query_row(
                "INSERT INTO apps (key, name, icon, art, category, first_seen) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                 ON CONFLICT (key) DO UPDATE SET
                     name = excluded.name, icon = excluded.icon, art = excluded.art, category = excluded.category
                 RETURNING id, key, name, icon, art, category, user_category",
                params![app.key, app.name, app.icon, app.art, app.category, now],
                app_from_row,
            )
            .context("saving an app")
    }

    pub fn app(&self, id: i64) -> Result<Option<AppInfo>> {
        Ok(self
            .db
            .query_row(
                "SELECT id, key, name, icon, art, category, user_category FROM apps WHERE id = ?1",
                [id],
                app_from_row,
            )
            .optional()?)
    }

    pub fn apps(&self) -> Result<Vec<AppInfo>> {
        let mut statement = self
            .db
            .prepare_cached("SELECT id, key, name, icon, art, category, user_category FROM apps")?;
        let rows = statement.query_map([], app_from_row)?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    pub fn first_seen(&self, app: i64) -> Result<i64> {
        Ok(self
            .db
            .query_row("SELECT first_seen FROM apps WHERE id = ?1", [app], |row| row.get(0))?)
    }

    pub fn set_category(&self, app: i64, category: Option<&str>) -> Result<Option<AppInfo>> {
        self.db
            .execute("UPDATE apps SET user_category = ?2 WHERE id = ?1", params![app, category])?;
        self.app(app)
    }

    pub fn title_id(&self, app: i64, text: &str) -> Result<i64> {
        Ok(self.db.query_row(
            "INSERT INTO titles (app, text) VALUES (?1, ?2)
             ON CONFLICT (app, text) DO UPDATE SET text = excluded.text
             RETURNING id",
            params![app, text],
            |row| row.get(0),
        )?)
    }

    pub fn titles(&self, ids: &[i64]) -> Result<Vec<(i64, String)>> {
        let mut statement = self.db.prepare_cached("SELECT text FROM titles WHERE id = ?1")?;
        ids.iter()
            .map(|&id| Ok((id, statement.query_row([id], |row| row.get(0))?)))
            .collect()
    }

    pub fn open_session(&self, start: i64) -> Result<i64> {
        self.db
            .execute("INSERT INTO sessions (start, end) VALUES (?1, ?1)", [start])?;
        Ok(self.db.last_insert_rowid())
    }

    pub fn set_session_end(&self, id: i64, end: i64) -> Result<()> {
        self.db
            .execute("UPDATE sessions SET end = MAX(start, ?2) WHERE id = ?1", [id, end])?;
        Ok(())
    }

    pub fn open_segment(&self, session: i64, app: i64, title: Option<i64>, start: i64) -> Result<i64> {
        self.db.execute(
            "INSERT INTO segments (session, app, title, start, end) VALUES (?1, ?2, ?3, ?4, ?4)",
            params![session, app, title, start],
        )?;
        Ok(self.db.last_insert_rowid())
    }

    pub fn set_segment_end(&self, id: i64, end: i64) -> Result<()> {
        self.db
            .execute("UPDATE segments SET end = MAX(start, ?2) WHERE id = ?1", [id, end])?;
        Ok(())
    }

    pub fn set_segment_title(&self, id: i64, title: Option<i64>) -> Result<()> {
        self.db
            .execute("UPDATE segments SET title = ?2 WHERE id = ?1", params![id, title])?;
        Ok(())
    }

    /// Nobody was there after `cut`: whatever runs past it ends there, and
    /// what began after it goes. Empty sessions go with their segments.
    pub fn trim_after(&mut self, cut: i64) -> Result<()> {
        let tx = self.db.transaction()?;
        tx.execute("UPDATE segments SET end = MAX(start, ?1) WHERE end > ?1", [cut])?;
        tx.execute("DELETE FROM segments WHERE end <= start AND start >= ?1", [cut])?;
        tx.execute("UPDATE sessions SET end = MAX(start, ?1) WHERE end > ?1", [cut])?;
        tx.execute(
            "DELETE FROM sessions WHERE end <= start AND start >= ?1
             AND NOT EXISTS (SELECT 1 FROM segments WHERE segments.session = sessions.id)",
            [cut],
        )?;
        tx.commit()?;
        Ok(())
    }

    /// Segments that overlap `[from, to)`, uncut, in the order they began.
    pub fn segments(&self, from: i64, to: i64, app: Option<i64>) -> Result<Vec<Segment>> {
        let mut statement = self.db.prepare_cached(
            "SELECT session, app, title, start, end FROM segments
             WHERE end > ?1 AND start < ?2 AND end > start AND (?3 IS NULL OR app = ?3)
             ORDER BY start",
        )?;
        let rows = statement.query_map(params![from, to, app], |row| {
            Ok(Segment {
                session: row.get(0)?,
                app: row.get(1)?,
                title: row.get(2)?,
                start: row.get(3)?,
                end: row.get(4)?,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    /// Sessions that overlap `[from, to)`, uncut: `(start, end)`.
    pub fn sessions(&self, from: i64, to: i64) -> Result<Vec<(i64, i64)>> {
        let mut statement = self.db.prepare_cached(
            "SELECT start, end FROM sessions WHERE end > ?1 AND start < ?2 AND end > start ORDER BY start",
        )?;
        let rows = statement.query_map([from, to], |row| Ok((row.get(0)?, row.get(1)?)))?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    /// Every app's time over all history, and when it was last used.
    pub fn totals(&self) -> Result<Vec<(i64, i64, i64)>> {
        let mut statement = self.db.prepare_cached(
            "SELECT app, SUM(end - start), MAX(end) FROM segments GROUP BY app",
        )?;
        let rows = statement.query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }
}

fn app_from_row(row: &rusqlite::Row) -> rusqlite::Result<AppInfo> {
    let found: String = row.get(5)?;
    let chosen: Option<String> = row.get(6)?;
    Ok(AppInfo {
        id: row.get(0)?,
        key: row.get(1)?,
        name: row.get(2)?,
        icon: row.get(3)?,
        art: row.get(4)?,
        custom_category: chosen.is_some(),
        category: chosen.unwrap_or(found),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn resolved(key: &str) -> Resolved {
        Resolved {
            key: key.to_owned(),
            name: key.to_uppercase(),
            icon: None,
            art: None,
            category: "web".to_owned(),
        }
    }

    #[test]
    fn apps_keep_the_users_category() {
        let store = Store::in_memory();
        let app = store.upsert_app(&resolved("zen"), 1).unwrap();
        assert_eq!(app.category, "web");
        store.set_category(app.id, Some("productivity")).unwrap();
        let again = store.upsert_app(&resolved("zen"), 2).unwrap();
        assert_eq!(again.id, app.id);
        assert_eq!(again.category, "productivity");
        assert!(again.custom_category);
        assert_eq!(store.first_seen(app.id).unwrap(), 1);
        let reset = store.set_category(app.id, None).unwrap().unwrap();
        assert_eq!(reset.category, "web");
    }

    #[test]
    fn trimming_cuts_back_to_the_last_input() {
        let mut store = Store::in_memory();
        let app = store.upsert_app(&resolved("zen"), 0).unwrap().id;
        let session = store.open_session(100).unwrap();
        let first = store.open_segment(session, app, None, 100).unwrap();
        store.set_segment_end(first, 200).unwrap();
        let second = store.open_segment(session, app, None, 200).unwrap();
        store.set_segment_end(second, 400).unwrap();
        store.set_session_end(session, 400).unwrap();
        // A session opened after the cut (no input since) goes entirely.
        let later = store.open_session(350).unwrap();
        let ghost = store.open_segment(later, app, None, 350).unwrap();
        store.set_segment_end(ghost, 400).unwrap();
        store.set_session_end(later, 400).unwrap();

        store.trim_after(150).unwrap();
        let segments = store.segments(0, 1000, None).unwrap();
        assert_eq!(segments.len(), 1);
        assert_eq!((segments[0].start, segments[0].end), (100, 150));
        assert_eq!(store.sessions(0, 1000).unwrap(), [(100, 150)]);
    }

    #[test]
    fn titles_are_shared() {
        let store = Store::in_memory();
        let app = store.upsert_app(&resolved("zen"), 0).unwrap().id;
        let a = store.title_id(app, "Docs").unwrap();
        assert_eq!(store.title_id(app, "Docs").unwrap(), a);
        assert_ne!(store.title_id(app, "Mail").unwrap(), a);
        assert_eq!(store.titles(&[a]).unwrap(), [(a, "Docs".to_owned())]);
    }
}
