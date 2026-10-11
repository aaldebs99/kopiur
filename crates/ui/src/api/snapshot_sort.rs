//! `GET /api/v1/snapshots?sort=`: the order the list is paged in.
//!
//! The snapshots list is the one table the server pages, so the server sorts
//! it: a table that re-ordered one page on the client would claim "largest
//! first" about fifty rows. Keys are the SPA table's own column ids, so the
//! column a person clicked is the key the request carries; `-` in front sorts
//! descending. Empty values sort last in either direction, and ties keep the
//! default newest-first order.

use std::cmp::Ordering;

use chrono::{DateTime, Utc};
use kopiur_ui_model::views::{OriginView, SnapshotPhaseView, SnapshotRow};

use crate::api::problem::{ApiError, problem};

/// A column the snapshots list can be ordered by.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnapshotSortKey {
    /// The snapshot's name, then its namespace.
    Name,
    /// Worst first: failed, then in flight, then unknown, then done.
    Phase,
    /// How it came to exist, by its word.
    Origin,
    /// The policy that produced it.
    Policy,
    /// The repository it lives in, as the row displays it.
    Repository,
    /// `status.stats.sizeBytes`.
    Size,
    /// Files considered.
    Files,
    /// When the run started.
    Started,
    /// How long the run took.
    Took,
}

/// A key and a direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SnapshotSort {
    /// What to order by.
    pub key: SnapshotSortKey,
    /// Largest, latest or last-in-the-alphabet first.
    pub descending: bool,
}

/// **Pure.** Parse `?sort=`: a column id, `-` in front for descending. Blank
/// is no sort; anything else unknown is a 400 naming the keys there are.
pub fn parse_sort(value: Option<&str>) -> Result<Option<SnapshotSort>, ApiError> {
    let Some(raw) = value.map(str::trim).filter(|v| !v.is_empty()) else {
        return Ok(None);
    };
    let (descending, id) = raw.strip_prefix('-').map_or((false, raw), |id| (true, id));
    let key = match id {
        "snapshot" => SnapshotSortKey::Name,
        "phase" => SnapshotSortKey::Phase,
        "origin" => SnapshotSortKey::Origin,
        "policy" => SnapshotSortKey::Policy,
        "repository" => SnapshotSortKey::Repository,
        "size" => SnapshotSortKey::Size,
        "files" => SnapshotSortKey::Files,
        "started" => SnapshotSortKey::Started,
        "took" => SnapshotSortKey::Took,
        _other => return Err(bad_sort(raw)),
    };
    Ok(Some(SnapshotSort { key, descending }))
}

fn bad_sort(value: &str) -> ApiError {
    problem(
        400,
        "invalid-filter",
        format!("`{value}` is not a column the snapshots list sorts by."),
        "It names no sortable column.",
        "use one of snapshot, phase, origin, policy, repository, size, files, started or \
         took, with `-` in front for descending",
    )
}

/// One row's value under a key: text or a number, `None` when the row has
/// none. Text compares naturally ([`natural_cmp`]); numbers as numbers.
#[derive(Debug, PartialEq)]
enum Value {
    Text(String),
    Number(i64),
}

impl Value {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self, other) {
            (Self::Text(a), Self::Text(b)) => natural_cmp(a, b),
            (Self::Number(a), Self::Number(b)) => a.cmp(b),
            // One key yields one variant; a mix cannot arise, so order it stably.
            (Self::Number(_), Self::Text(_)) => Ordering::Less,
            (Self::Text(_), Self::Number(_)) => Ordering::Greater,
        }
    }
}

/// **Pure.** The order the SPA's other tables use for text (a numeric,
/// case-insensitive collator): runs of digits compare as numbers, so
/// `photos-2` comes before `photos-10`, and case only breaks ties.
fn natural_cmp(a: &str, b: &str) -> Ordering {
    natural_cmp_folded(a, b).then_with(|| a.cmp(b))
}

fn natural_cmp_folded(a: &str, b: &str) -> Ordering {
    let (mut x, mut y) = (a.chars().peekable(), b.chars().peekable());
    loop {
        match (x.peek().copied(), y.peek().copied()) {
            (None, None) => return Ordering::Equal,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(c), Some(d)) if c.is_ascii_digit() && d.is_ascii_digit() => {
                let order = digit_run(&mut x).cmp(&digit_run(&mut y));
                if order != Ordering::Equal {
                    return order;
                }
            }
            (Some(c), Some(d)) => {
                let order = c.to_lowercase().cmp(d.to_lowercase());
                if order != Ordering::Equal {
                    return order;
                }
                x.next();
                y.next();
            }
        }
    }
}

/// Consume a run of ASCII digits, as a key that orders by numeric value:
/// leading zeros dropped, then shorter is smaller, then digit by digit.
fn digit_run(chars: &mut std::iter::Peekable<std::str::Chars<'_>>) -> (usize, String) {
    let mut digits = String::new();
    while let Some(c) = chars.next_if(char::is_ascii_digit) {
        digits.push(c);
    }
    let significant = digits.trim_start_matches('0').to_string();
    (significant.len(), significant)
}

/// **Pure.** Order `rows` by `sort`, stably: rows that tie (or a `None` sort)
/// keep the order they arrived in, which is newest run first.
pub fn sort_rows(rows: &mut [SnapshotRow], sort: Option<SnapshotSort>) {
    let Some(sort) = sort else { return };
    rows.sort_by(|a, b| {
        match (value(a, sort.key), value(b, sort.key)) {
            (None, None) => Ordering::Equal,
            // Empty last, whichever way the column runs.
            (None, Some(_)) => Ordering::Greater,
            (Some(_), None) => Ordering::Less,
            (Some(x), Some(y)) => {
                let order = x.cmp(&y);
                if sort.descending {
                    order.reverse()
                } else {
                    order
                }
            }
        }
    });
}

fn value(row: &SnapshotRow, key: SnapshotSortKey) -> Option<Value> {
    match key {
        SnapshotSortKey::Name => Some(Value::Text(format!("{}\u{0}{}", row.name, row.namespace))),
        SnapshotSortKey::Phase => Some(Value::Number(phase_rank(row.phase.as_ref()))),
        SnapshotSortKey::Origin => row
            .origin
            .as_ref()
            .map(|o| Value::Text(origin_word(o).into())),
        SnapshotSortKey::Policy => row.policy.clone().map(Value::Text),
        SnapshotSortKey::Repository => row.repository.clone().map(Value::Text),
        SnapshotSortKey::Size => row.size_bytes.map(Value::Number),
        SnapshotSortKey::Files => row.files_total.map(Value::Number),
        SnapshotSortKey::Started => instant(row.start_time.as_deref()).map(Value::Number),
        SnapshotSortKey::Took => took(row).map(Value::Number),
    }
}

/// Worst first, in the lamp order the SPA counts by (`HEALTH_ORDER`:
/// failed, degraded, pending, unknown, suspended, healthy) — the same lamp
/// `snapshotPhaseLamp` gives each phase.
fn phase_rank(phase: Option<&SnapshotPhaseView>) -> i64 {
    match phase {
        Some(SnapshotPhaseView::Failed) => 0,
        Some(
            SnapshotPhaseView::Pending | SnapshotPhaseView::Running | SnapshotPhaseView::Deleting,
        ) => 2,
        None | Some(SnapshotPhaseView::Unknown { .. }) => 3,
        Some(
            SnapshotPhaseView::Succeeded
            | SnapshotPhaseView::Discovered
            | SnapshotPhaseView::Unchanged,
        ) => 5,
    }
}

/// The word the table shows (`originLabel`).
fn origin_word(origin: &OriginView) -> &'static str {
    match origin {
        OriginView::Scheduled => "Scheduled",
        OriginView::Manual => "Manual",
        OriginView::Discovered => "Discovered",
        OriginView::Adopted => "Adopted",
        OriginView::Replicated => "Replicated",
    }
}

fn instant(at: Option<&str>) -> Option<i64> {
    at.and_then(|t| DateTime::parse_from_rfc3339(t).ok())
        .map(|t| t.with_timezone(&Utc).timestamp())
}

/// Seconds from start to end, as the table's Took column measures it; none
/// for a run that has not ended or whose end precedes its start.
fn took(row: &SnapshotRow) -> Option<i64> {
    let start = instant(row.start_time.as_deref())?;
    let end = instant(row.end_time.as_deref())?;
    (end >= start).then_some(end - start)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(name: &str) -> SnapshotRow {
        serde_json::from_value(serde_json::json!({
            "namespace": "media",
            "name": name,
            "pinned": false,
        }))
        .expect("row fixture")
    }

    fn names(rows: &[SnapshotRow]) -> Vec<&str> {
        rows.iter().map(|r| r.name.as_str()).collect()
    }

    fn sort(raw: &str) -> Option<SnapshotSort> {
        parse_sort(Some(raw)).expect("a known key")
    }

    #[test]
    fn a_key_is_the_tables_column_id_with_a_minus_for_descending() {
        assert_eq!(
            parse_sort(Some("-size")).unwrap(),
            Some(SnapshotSort {
                key: SnapshotSortKey::Size,
                descending: true
            })
        );
        assert_eq!(
            parse_sort(Some("snapshot")).unwrap(),
            Some(SnapshotSort {
                key: SnapshotSortKey::Name,
                descending: false
            })
        );
        assert_eq!(parse_sort(Some("  ")).unwrap(), None);
        assert_eq!(parse_sort(None).unwrap(), None);
        assert!(parse_sort(Some("identity")).is_err());
        assert!(parse_sort(Some("--size")).is_err());
    }

    #[test]
    fn numbers_sort_as_numbers_and_empty_values_go_last_either_way() {
        let mut rows = vec![row("none"), row("big"), row("small")];
        rows[1].size_bytes = Some(10_000);
        rows[2].size_bytes = Some(9);
        sort_rows(&mut rows, sort("-size"));
        assert_eq!(names(&rows), ["big", "small", "none"]);
        sort_rows(&mut rows, sort("size"));
        assert_eq!(names(&rows), ["small", "big", "none"]);
    }

    #[test]
    fn no_sort_and_ties_keep_the_order_the_rows_came_in() {
        let mut rows = vec![row("c"), row("a"), row("b")];
        sort_rows(&mut rows, None);
        assert_eq!(names(&rows), ["c", "a", "b"]);
        sort_rows(&mut rows, sort("-size"));
        assert_eq!(names(&rows), ["c", "a", "b"], "all empty: nothing moves");
    }

    #[test]
    fn phase_sorts_worst_first() {
        let mut rows = vec![row("ok"), row("odd"), row("run"), row("bad")];
        rows[0].phase = Some(SnapshotPhaseView::Succeeded);
        rows[1].phase = Some(SnapshotPhaseView::Unknown {
            raw: "Weird".into(),
        });
        rows[2].phase = Some(SnapshotPhaseView::Running);
        rows[3].phase = Some(SnapshotPhaseView::Failed);
        sort_rows(&mut rows, sort("phase"));
        assert_eq!(names(&rows), ["bad", "run", "odd", "ok"]);
    }

    #[test]
    fn times_and_durations_read_the_rows_instants() {
        let mut rows = vec![row("old"), row("new"), row("open")];
        rows[0].start_time = Some("2026-09-08T01:00:00Z".into());
        rows[0].end_time = Some("2026-09-08T01:30:00Z".into());
        rows[1].start_time = Some("2026-09-08T02:00:00Z".into());
        rows[1].end_time = Some("2026-09-08T02:01:00Z".into());
        rows[2].start_time = Some("2026-09-08T03:00:00Z".into());
        sort_rows(&mut rows, sort("-started"));
        assert_eq!(names(&rows), ["open", "new", "old"]);
        sort_rows(&mut rows, sort("-took"));
        assert_eq!(
            names(&rows),
            ["old", "new", "open"],
            "a run still going has no duration"
        );
    }

    #[test]
    fn text_sorts_naturally_and_ignores_case_as_the_other_tables_do() {
        let mut rows = vec![row("photos-10"), row("Photos-3"), row("photos-2")];
        sort_rows(&mut rows, sort("snapshot"));
        assert_eq!(names(&rows), ["photos-2", "Photos-3", "photos-10"]);
        sort_rows(&mut rows, sort("-snapshot"));
        assert_eq!(names(&rows), ["photos-10", "Photos-3", "photos-2"]);
    }

    #[test]
    fn names_then_namespaces_and_words_sort_as_text() {
        let mut rows = vec![row("b"), row("a"), row("c")];
        rows[0].origin = Some(OriginView::Scheduled);
        rows[1].origin = Some(OriginView::Manual);
        sort_rows(&mut rows, sort("-snapshot"));
        assert_eq!(names(&rows), ["c", "b", "a"]);
        sort_rows(&mut rows, sort("origin"));
        assert_eq!(
            names(&rows),
            ["a", "b", "c"],
            "Manual, Scheduled, then the empty one"
        );
    }
}
