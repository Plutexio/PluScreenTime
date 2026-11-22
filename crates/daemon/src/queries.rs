//! Turning segments and sessions into what the app shows: totals cut into
//! the buckets the app asked for, by app and category, and the stretches at
//! the screen.

use std::collections::HashMap;

use anyhow::{Result, bail};
use pluscreentime_protocol::{
    AppDetail, AppInfo, AppList, AppTotal, AppUsage, Bucket, CategoryUsage, Summary, TitleUsage,
};

use crate::store::{Segment, Store};

/// At most a year of days, or a few years of weeks.
const MAX_BUCKETS: usize = 1000;

pub fn check_edges(edges: &[i64]) -> Result<()> {
    if edges.len() < 2 {
        bail!("a range needs at least two edges");
    }
    if edges.len() > MAX_BUCKETS + 1 {
        bail!("at most {MAX_BUCKETS} buckets");
    }
    if edges.windows(2).any(|pair| pair[0] >= pair[1]) {
        bail!("edges must ascend");
    }
    Ok(())
}

/// Calls `add(bucket, milliseconds)` for each bucket `[start, end)` reaches.
fn spread(edges: &[i64], start: i64, end: i64, mut add: impl FnMut(usize, i64)) {
    let (from, to) = (edges[0], edges[edges.len() - 1]);
    let (start, end) = (start.max(from), end.min(to));
    if start >= end {
        return;
    }
    // The bucket holding `start`: the last edge at or before it.
    let mut bucket = edges.partition_point(|&edge| edge <= start) - 1;
    while bucket + 1 < edges.len() && edges[bucket] < end {
        let overlap = end.min(edges[bucket + 1]) - start.max(edges[bucket]);
        if overlap > 0 {
            add(bucket, overlap);
        }
        bucket += 1;
    }
}

fn clipped(edges: &[i64], start: i64, end: i64) -> i64 {
    (end.min(edges[edges.len() - 1]) - start.max(edges[0])).max(0)
}

pub fn summary(store: &Store, edges: Vec<i64>) -> Result<Summary> {
    check_edges(&edges)?;
    let (from, to) = (edges[0], edges[edges.len() - 1]);
    let apps: HashMap<i64, AppInfo> = store.apps()?.into_iter().map(|a| (a.id, a)).collect();
    let segments = store.segments(from, to, None)?;
    let sessions = store.sessions(from, to)?;
    Ok(summarize(edges, &apps, &segments, &sessions))
}

fn summarize(
    edges: Vec<i64>,
    apps: &HashMap<i64, AppInfo>,
    segments: &[Segment],
    sessions: &[(i64, i64)],
) -> Summary {
    let (from, to) = (edges[0], edges[edges.len() - 1]);
    let mut buckets = vec![Bucket::default(); edges.len() - 1];
    let mut by_app: HashMap<i64, i64> = HashMap::new();
    let mut by_category: HashMap<String, i64> = HashMap::new();
    let mut total = 0;
    for segment in segments {
        let category = apps
            .get(&segment.app)
            .map_or("other", |app| app.category.as_str());
        spread(&edges, segment.start, segment.end, |i, ms| {
            buckets[i].total += ms;
            *buckets[i].categories.entry(category.to_owned()).or_default() += ms;
        });
        let ms = clipped(&edges, segment.start, segment.end);
        total += ms;
        *by_app.entry(segment.app).or_default() += ms;
        *by_category.entry(category.to_owned()).or_default() += ms;
    }

    let mut app_usage: Vec<AppUsage> = by_app
        .into_iter()
        .filter(|&(_, ms)| ms > 0)
        .filter_map(|(id, total)| Some(AppUsage { app: apps.get(&id)?.clone(), total }))
        .collect();
    app_usage.sort_by(|a, b| b.total.cmp(&a.total).then_with(|| a.app.name.cmp(&b.app.name)));
    let mut categories: Vec<CategoryUsage> = by_category
        .into_iter()
        .filter(|&(_, ms)| ms > 0)
        .map(|(id, total)| CategoryUsage { id, total })
        .collect();
    categories.sort_by(|a, b| b.total.cmp(&a.total).then_with(|| a.id.cmp(&b.id)));

    let mut pickups = 0;
    let mut longest_session = 0;
    let mut cut_sessions = Vec::with_capacity(sessions.len());
    for &(start, end) in sessions {
        if start >= from && start < to {
            pickups += 1;
            let bucket = edges.partition_point(|&edge| edge <= start) - 1;
            buckets[bucket].pickups += 1;
        }
        longest_session = longest_session.max(clipped(&edges, start, end));
        cut_sessions.push([start.max(from), end.min(to)]);
    }

    Summary {
        total,
        buckets,
        apps: app_usage,
        categories,
        pickups,
        longest_session,
        first_activity: cut_sessions.first().map(|s| s[0]),
        last_activity: cut_sessions.last().map(|s| s[1]),
        sessions: cut_sessions,
        edges,
    }
}

pub fn app_detail(store: &Store, app: i64, edges: Vec<i64>) -> Result<AppDetail> {
    check_edges(&edges)?;
    let Some(info) = store.app(app)? else {
        bail!("no such app");
    };
    let (from, to) = (edges[0], edges[edges.len() - 1]);
    // Every app's segments, to tell switching to it from carrying on in it.
    let segments = store.segments(from, to, None)?;
    let (mut detail, titles) = detail(edges, info, &segments);
    let ids: Vec<i64> = titles.iter().map(|&(id, _)| id).collect();
    let texts: HashMap<i64, String> = store.titles(&ids)?.into_iter().collect();
    detail.titles = titles
        .into_iter()
        .filter_map(|(id, total)| Some(TitleUsage { title: texts.get(&id)?.clone(), total }))
        .collect();
    detail.first_seen = store.first_seen(app)?;
    detail.last_used = store
        .totals()?
        .into_iter()
        .find(|&(id, _, _)| id == app)
        .map(|(_, _, last)| last);
    Ok(detail)
}

/// The detail of one app, with its titles as `(title ID, ms)`, most used first.
fn detail(edges: Vec<i64>, app: AppInfo, segments: &[Segment]) -> (AppDetail, Vec<(i64, i64)>) {
    let mut buckets = vec![0; edges.len() - 1];
    let mut total = 0;
    let mut titles: HashMap<i64, i64> = HashMap::new();
    let mut opens = 0;
    let mut longest = 0;
    let mut run = 0;
    let mut previous: Option<&Segment> = None;
    for segment in segments {
        let continues = previous.is_some_and(|p| {
            p.app == segment.app && p.session == segment.session && p.end >= segment.start
        });
        previous = Some(segment);
        if segment.app != app.id {
            continue;
        }
        let ms = clipped(&edges, segment.start, segment.end);
        if !continues {
            run = 0;
            if segment.start >= edges[0] {
                opens += 1;
            }
        }
        run += ms;
        longest = longest.max(run);
        total += ms;
        spread(&edges, segment.start, segment.end, |i, ms| buckets[i] += ms);
        if let Some(title) = segment.title {
            *titles.entry(title).or_default() += ms;
        }
    }
    let mut titles: Vec<(i64, i64)> = titles.into_iter().filter(|&(_, ms)| ms > 0).collect();
    titles.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    titles.truncate(AppDetail::MAX_TITLES);
    let detail = AppDetail {
        app,
        total,
        buckets,
        titles: Vec::new(),
        opens,
        longest,
        first_seen: 0,
        last_used: None,
    };
    (detail, titles)
}

pub fn app_list(store: &Store) -> Result<AppList> {
    let totals: HashMap<i64, (i64, i64)> = store
        .totals()?
        .into_iter()
        .map(|(app, total, last)| (app, (total, last)))
        .collect();
    let mut apps: Vec<AppTotal> = store
        .apps()?
        .into_iter()
        .map(|app| {
            let found = totals.get(&app.id).copied();
            AppTotal {
                total: found.map_or(0, |(total, _)| total),
                last_used: found.map(|(_, last)| last),
                app,
            }
        })
        .collect();
    apps.sort_by(|a, b| b.total.cmp(&a.total).then_with(|| a.app.name.cmp(&b.app.name)));
    Ok(AppList { apps })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app(id: i64, category: &str) -> AppInfo {
        AppInfo {
            id,
            key: format!("app{id}"),
            name: format!("App {id}"),
            icon: None,
            art: None,
            category: category.to_owned(),
            custom_category: false,
        }
    }

    fn segment(session: i64, app: i64, title: Option<i64>, start: i64, end: i64) -> Segment {
        Segment { session, app, title, start, end }
    }

    #[test]
    fn spreads_over_buckets() {
        let edges = [0, 10, 20, 30];
        let mut got = vec![0; 3];
        spread(&edges, 5, 25, |i, ms| got[i] += ms);
        assert_eq!(got, [5, 10, 5]);
        let mut got = vec![0; 3];
        spread(&edges, -100, 100, |i, ms| got[i] += ms);
        assert_eq!(got, [10, 10, 10]);
        let mut got = vec![0; 3];
        spread(&edges, 30, 40, |i, ms| got[i] += ms);
        assert_eq!(got, [0, 0, 0]);
        let mut got = vec![0; 3];
        spread(&edges, 10, 20, |i, ms| got[i] += ms);
        assert_eq!(got, [0, 10, 0]);
    }

    #[test]
    fn rejects_bad_edges() {
        assert!(check_edges(&[1]).is_err());
        assert!(check_edges(&[1, 1]).is_err());
        assert!(check_edges(&[2, 1]).is_err());
        assert!(check_edges(&[1, 2, 3]).is_ok());
    }

    #[test]
    fn summarizes_by_app_category_and_session() {
        let apps: HashMap<i64, AppInfo> = [(1, app(1, "web")), (2, app(2, "games"))].into();
        // A session from before the range carries into it; it isn't a pickup.
        let segments = [
            segment(1, 1, None, -50, 30),
            segment(1, 2, None, 30, 60),
            segment(2, 1, None, 80, 120),
        ];
        let sessions = [(-50, 60), (80, 120)];
        let s = summarize(vec![0, 50, 100], &apps, &segments, &sessions);
        assert_eq!(s.total, 30 + 30 + 20);
        assert_eq!(s.buckets[0].total, 50);
        assert_eq!(s.buckets[1].total, 30);
        assert_eq!(s.buckets[0].categories["web"], 30);
        assert_eq!(s.buckets[0].categories["games"], 20);
        assert_eq!(s.apps[0].app.id, 1);
        assert_eq!(s.apps[0].total, 50);
        assert_eq!(s.categories[0].id, "web");
        assert_eq!(s.pickups, 1);
        assert_eq!(s.buckets[1].pickups, 1);
        assert_eq!(s.longest_session, 60);
        assert_eq!(s.sessions, [[0, 60], [80, 100]]);
        assert_eq!(s.first_activity, Some(0));
        assert_eq!(s.last_activity, Some(100));
    }

    #[test]
    fn counts_switches_and_runs() {
        // App 1, a new title in it (same run), app 2, back to app 1, then a
        // new session in app 1.
        let segments = [
            segment(1, 1, Some(10), 0, 10),
            segment(1, 1, Some(11), 10, 30),
            segment(1, 2, None, 30, 40),
            segment(1, 1, Some(10), 40, 45),
            segment(2, 1, Some(10), 100, 105),
        ];
        let (d, titles) = detail(vec![0, 50, 200], app(1, "web"), &segments);
        assert_eq!(d.total, 40);
        assert_eq!(d.buckets, [35, 5]);
        assert_eq!(d.opens, 3);
        assert_eq!(d.longest, 30);
        // Equal times go by title ID.
        assert_eq!(titles, [(10, 20), (11, 20)]);
    }
}
