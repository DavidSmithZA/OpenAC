//! What the Journal shows: the quests (the server's contract tracker
//! read against the game's contract table) grouped by where they stand,
//! with the picked one's detail; and the notes (the journal kept on this
//! computer), its pages and the page being read.

use ac_core::journal::JournalPage;
use ac_core::quests::{ContractCatalog, ContractEntry, build_progress_text};
use ac_core::text::format_duration;
use ac_runtime::communication::{JournalView, TrackedContract};

/// The Journal's pages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum JournalTab {
    /// The quests.
    #[default]
    Quests,
    /// The notes.
    Notes,
}

/// Where a quest stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuestState {
    /// Under way.
    Active,
    /// Done, and can be done again once its time is up.
    Waiting,
    /// Can be taken up.
    Available,
    /// Done for good.
    Done,
}

impl QuestState {
    /// Every state, in the list's order.
    pub const ALL: [Self; 4] = [Self::Active, Self::Waiting, Self::Available, Self::Done];

    /// The group's heading.
    #[must_use]
    pub const fn heading(self) -> &'static str {
        match self {
            Self::Active => "IN PROGRESS",
            Self::Waiting => "WAITING TO REPEAT",
            Self::Available => "CAN BE STARTED",
            Self::Done => "DONE",
        }
    }

    /// The status pill's words.
    #[must_use]
    pub const fn words(self) -> &'static str {
        match self {
            Self::Active => "In progress",
            Self::Waiting => "Waiting to repeat",
            Self::Available => "Can be started",
            Self::Done => "Done",
        }
    }
}

/// A quest's row in the list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuestRow {
    /// The contract.
    pub id: u32,
    /// Its name.
    pub name: String,
    /// Where it stands, in a few words.
    pub sub: String,
    /// It runs against the clock.
    pub timed: bool,
    /// It shows on screen.
    pub shown: bool,
}

/// One of the people or places a quest names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuestPlace {
    /// "Given by", "Quest area", "Turn in to".
    pub role: &'static str,
    /// Who (or what).
    pub who: String,
    /// The coordinates, or "Indoors".
    pub place: String,
    /// The cell, when it has coordinates (the Map button).
    pub cell: Option<u32>,
}

/// The picked quest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuestDetail {
    /// The contract.
    pub id: u32,
    /// Its name.
    pub name: String,
    /// Where it stands.
    pub state: QuestState,
    /// What it asks.
    pub description: String,
    /// The server's progress line (stage counts shown in the quest's own
    /// words), when it has one.
    pub progress: Option<String>,
    /// The time line: label, time, and whether it runs out (amber).
    pub time: Option<(&'static str, String, bool)>,
    /// Who gives it, the quest area, who takes it back.
    pub places: Vec<QuestPlace>,
    /// "Abandon" or "Remove from list".
    pub abandon: &'static str,
    /// The quiet line beside it.
    pub foot: &'static str,
    /// Who gives it (the question names them).
    pub giver: String,
    /// It shows on screen.
    pub shown: bool,
}

/// The quests page.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct QuestsScreen {
    /// The groups that have quests, in order, with their rows.
    pub groups: Vec<(QuestState, Vec<QuestRow>)>,
    /// The picked quest.
    pub detail: Option<QuestDetail>,
    /// How many quests in all (before the search).
    pub count: usize,
}

/// A quest on screen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShownQuest {
    /// The contract.
    pub id: u32,
    /// Its name.
    pub name: String,
    /// Its line (progress or time left).
    pub line: String,
    /// The line is a time running out.
    pub timed: bool,
}

/// A page's row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageRow {
    /// Its number (from 1).
    pub number: usize,
    /// Its title (or "Untitled page").
    pub title: String,
    /// Its label.
    pub label: String,
    /// Its countdown's time left, when one runs.
    pub left: Option<String>,
}

/// The notes page.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct NotesScreen {
    /// The pages matching the search.
    pub rows: Vec<PageRow>,
    /// The page read (from 1; 0: none).
    pub current: usize,
    /// How many pages.
    pub count: usize,
    /// The page read.
    pub page: JournalPage,
    /// Its recorded place, worded.
    pub place: Option<String>,
    /// Its countdown's time left, when it runs.
    pub left: Option<String>,
    /// Another page's countdown runs (one at a time).
    pub other_running: bool,
}

/// What the quests are worked out from.
#[derive(Debug, Clone, Copy)]
pub struct QuestsInput<'a> {
    /// The tracked contracts.
    pub contracts: &'a [TrackedContract],
    /// The game's contract table.
    pub catalog: &'a ContractCatalog,
    /// The quest picked (0: the first).
    pub picked: u32,
    /// The search.
    pub search: &'a str,
    /// The quests shown on screen.
    pub shown: &'a [u32],
    /// Now, Unix milliseconds.
    pub now_unix_ms: i64,
}

/// `seconds` in its two largest units ("1h 42m", "3d 4h", "12m 05s").
#[must_use]
pub fn short(seconds: f64) -> String {
    // Whole seconds, never negative.
    #[allow(clippy::cast_possible_truncation)]
    let total = (seconds as i64).max(0);
    let (d, h, m, s) = (
        total / 86_400,
        total / 3600 % 24,
        total / 60 % 60,
        total % 60,
    );
    if d > 0 {
        format!("{d}d {h}h")
    } else if h > 0 {
        format!("{h}h {m:02}m")
    } else if m > 0 {
        format!("{m}m {s:02}s")
    } else {
        format!("{s}s")
    }
}

#[allow(clippy::cast_precision_loss)]
fn seconds(ms: i64) -> f64 {
    ms as f64 / 1000.0
}

/// Where `c` stands, its progress line and its time line.
fn standing(
    c: &TrackedContract,
    entry: &ContractEntry,
    now_unix_ms: i64,
) -> (QuestState, Option<String>, Option<(&'static str, f64)>) {
    let t = &c.tracker;
    let elapsed = seconds(now_unix_ms) - seconds(c.received_unix_ms);
    match t.stage {
        1 => (QuestState::Available, None, None),
        3 => {
            let left = t.time_when_repeats - elapsed;
            if t.time_when_repeats > 0.0 && left > 0.0 {
                (
                    QuestState::Waiting,
                    None,
                    Some(("Can be done again in", left)),
                )
            } else if entry.questflag_repeat_time.is_empty() {
                (QuestState::Done, None, None)
            } else {
                (QuestState::Available, None, None)
            }
        }
        _ => {
            let progress = (t.stage >= 4 && !entry.description_progress.is_empty()).then(|| {
                build_progress_text(
                    t.stage,
                    t.time_when_repeats,
                    seconds(c.received_unix_ms),
                    entry,
                    seconds(now_unix_ms),
                )
            });
            let time = (t.time_when_done > 0.0)
                .then(|| ("Time left to finish", (t.time_when_done - elapsed).max(0.0)));
            (QuestState::Active, progress, time)
        }
    }
}

fn sub(state: QuestState, progress: Option<&str>, time: Option<(&str, f64)>) -> String {
    match (state, progress, time) {
        (QuestState::Active, _, Some((_, left))) => format!("{} left to finish", short(left)),
        (QuestState::Active, Some(p), None) => p.to_owned(),
        (QuestState::Active, None, None) => "In progress".to_owned(),
        (QuestState::Waiting, _, Some((_, left))) => {
            format!("Done · again in {}", short(left))
        }
        (QuestState::Waiting, _, None) | (QuestState::Done, _, _) => "Done".to_owned(),
        (QuestState::Available, _, _) => "Can be started".to_owned(),
    }
}

fn place(role: &'static str, who: &str, cell: u32) -> Option<QuestPlace> {
    if who.is_empty() && cell == 0 {
        return None;
    }
    let coordinates = ac_ui::radar::RadarCoordinates::from_cell(cell);
    let words = match coordinates {
        Some(c) => format!("{}, {}", c.y_text(), c.x_text()),
        None if cell != 0 => "Indoors".to_owned(),
        None => String::new(),
    };
    let mapped = coordinates.is_some();
    Some(QuestPlace {
        role,
        who: if who.is_empty() {
            "The quest area".to_owned()
        } else {
            who.to_owned()
        },
        place: words,
        cell: mapped.then_some(cell),
    })
}

/// The quests page for `input`.
#[must_use]
pub fn quests(input: &QuestsInput<'_>) -> QuestsScreen {
    let search = input.search.trim().to_lowercase();
    let mut contracts: Vec<&TrackedContract> = input.contracts.iter().collect();
    contracts.sort_by_key(|c| c.tracker.contract_id);
    let mut groups: Vec<(QuestState, Vec<QuestRow>)> =
        QuestState::ALL.iter().map(|s| (*s, Vec::new())).collect();
    let mut first = None;
    let mut detail = None;
    for c in &contracts {
        let id = c.tracker.contract_id;
        let entry = input.catalog.lookup(id);
        let name = if entry.contract_name.is_empty() {
            format!("Quest {id}")
        } else {
            entry.contract_name.clone()
        };
        if !search.is_empty() && !name.to_lowercase().contains(&search) {
            continue;
        }
        let (state, progress, time) = standing(c, entry, input.now_unix_ms);
        let shown = input.shown.contains(&id);
        let row = QuestRow {
            id,
            name: name.clone(),
            sub: sub(state, progress.as_deref(), time),
            timed: state == QuestState::Active && time.is_some(),
            shown,
        };
        if let Some((_, rows)) = groups.iter_mut().find(|(s, _)| *s == state) {
            rows.push(row);
        }
        if first.is_none() {
            first = Some(id);
        }
        if id == input.picked || detail.is_none() && input.picked == 0 {
            let places = [
                place(
                    "Given by",
                    &entry.name_npc_start,
                    entry.location_npc_start_cell,
                ),
                place("Quest area", "", entry.location_quest_area_cell),
                place(
                    "Turn in to",
                    &entry.name_npc_end,
                    entry.location_npc_end_cell,
                ),
            ]
            .into_iter()
            .flatten()
            .collect();
            detail = Some(QuestDetail {
                id,
                name,
                state,
                description: entry.description.clone(),
                progress,
                time: time.map(|(label, left)| {
                    (label, format_duration(left), state == QuestState::Active)
                }),
                places,
                abandon: if state == QuestState::Done {
                    "Remove from list"
                } else {
                    "Abandon"
                },
                foot: match state {
                    QuestState::Waiting => {
                        "Removing it stops the countdown here; the server still knows when it repeats."
                    }
                    QuestState::Done => "",
                    _ => "The quest leaves the list once the server confirms.",
                },
                giver: entry.name_npc_start.clone(),
                shown,
            });
        }
    }
    groups.retain(|(_, rows)| !rows.is_empty());
    // The first in list order, when the pick is gone.
    if detail.is_none() && first.is_some() {
        let first_listed = groups
            .first()
            .and_then(|(_, rows)| rows.first())
            .map(|r| r.id);
        if let Some(id) = first_listed {
            return quests(&QuestsInput {
                picked: id,
                ..*input
            });
        }
    }
    QuestsScreen {
        groups,
        detail,
        count: contracts.len(),
    }
}

/// The quests on screen, in the order they were shown, with their lines.
#[must_use]
pub fn shown_quests(input: &QuestsInput<'_>) -> Vec<ShownQuest> {
    input
        .shown
        .iter()
        .filter_map(|id| {
            let c = input
                .contracts
                .iter()
                .find(|c| c.tracker.contract_id == *id)?;
            let entry = input.catalog.lookup(*id);
            let (state, progress, time) = standing(c, entry, input.now_unix_ms);
            let (line, timed) = match (state, progress, time) {
                (QuestState::Active, _, Some((_, left))) => {
                    (format!("{} left to finish", short(left)), true)
                }
                (QuestState::Active, Some(p), None) => (p, false),
                (state, progress, time) => (sub(state, progress.as_deref(), time), false),
            };
            Some(ShownQuest {
                id: *id,
                name: if entry.contract_name.is_empty() {
                    format!("Quest {id}")
                } else {
                    entry.contract_name.clone()
                },
                line,
                timed,
            })
        })
        .collect()
}

/// The time left on page `page`'s countdown, when it runs.
fn left_on(view: &JournalView, page: &JournalPage, now_unix_ms: i64) -> Option<f64> {
    if !page.is_timer_running() {
        return None;
    }
    let left = match view.timer_started_unix_ms {
        None => page.running_timer_seconds,
        Some(started) => (view.timer_seconds_at_start - seconds(now_unix_ms - started)).max(0.0),
    };
    (left > 0.0).then_some(left)
}

/// The notes page for `view`, the pages matching `search`.
#[must_use]
pub fn notes(view: &JournalView, search: &str, now_unix_ms: i64) -> NotesScreen {
    let search = search.trim().to_lowercase();
    let rows = view
        .pages
        .iter()
        .enumerate()
        .filter(|(_, p)| {
            search.is_empty()
                || p.title.to_lowercase().contains(&search)
                || p.label.to_lowercase().contains(&search)
                || p.notes.to_lowercase().contains(&search)
        })
        .map(|(i, p)| PageRow {
            number: i + 1,
            title: if p.title.trim().is_empty() {
                "Untitled page".to_owned()
            } else {
                p.title.clone()
            },
            label: p.label.clone(),
            left: left_on(view, p, now_unix_ms).map(short),
        })
        .collect();
    let page = view.current();
    let left = left_on(view, &page, now_unix_ms);
    let other_running = view
        .pages
        .iter()
        .enumerate()
        .any(|(i, p)| i + 1 != view.current_page && left_on(view, p, now_unix_ms).is_some());
    NotesScreen {
        rows,
        current: view.current_page,
        count: view.pages.len(),
        place: page
            .has_location
            .then(|| ac_ui::panels::journal_notes::location_text(page.location_x, page.location_y)),
        left: left.map(format_duration),
        other_running,
        page,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ac_proto::contract::ContractTracker;
    use std::collections::HashMap;

    const NOW: i64 = 1_000_000_000;

    fn tracked(id: u32, stage: u32, done: f64, repeats: f64) -> TrackedContract {
        TrackedContract {
            tracker: ContractTracker {
                version: 1,
                contract_id: id,
                stage,
                time_when_done: done,
                time_when_repeats: repeats,
            },
            received_unix_ms: NOW,
        }
    }

    fn catalog() -> ContractCatalog {
        let entry = |id, name: &str, progress: &str, repeat: &str| {
            (
                id,
                ContractEntry {
                    contract_id: id,
                    contract_name: name.to_owned(),
                    description_progress: progress.to_owned(),
                    questflag_repeat_time: repeat.to_owned(),
                    name_npc_start: "Captain Aldric".to_owned(),
                    location_npc_start_cell: 0xA9B4_0031,
                    location_quest_area_cell: 0x0123_0100,
                    ..ContractEntry::default()
                },
            )
        };
        ContractCatalog::new(HashMap::from([
            entry(1, "Drudge Raiders", "You have slain %d Drudge Raiders.", ""),
            entry(2, "Hearts of the Golems", "", "flag"),
            entry(3, "The Ramp Watch", "", ""),
            entry(4, "Tusker Tusks", "", ""),
            entry(5, "The Lost Shipment", "", ""),
        ]))
    }

    #[test]
    fn quests_are_grouped_by_where_they_stand() {
        let contracts = [
            tracked(1, 12, 0.0, 0.0),
            tracked(2, 3, 0.0, 3600.0),
            tracked(3, 1, 0.0, 0.0),
            tracked(4, 3, 0.0, 0.0),
            tracked(5, 2, 6120.0, 0.0),
        ];
        let catalog = catalog();
        let s = quests(&QuestsInput {
            contracts: &contracts,
            catalog: &catalog,
            picked: 0,
            search: "",
            shown: &[5],
            now_unix_ms: NOW,
        });
        let groups: Vec<(QuestState, Vec<u32>)> = s
            .groups
            .iter()
            .map(|(st, rows)| (*st, rows.iter().map(|r| r.id).collect()))
            .collect();
        assert_eq!(
            groups,
            [
                (QuestState::Active, vec![1, 5]),
                (QuestState::Waiting, vec![2]),
                (QuestState::Available, vec![3]),
                (QuestState::Done, vec![4]),
            ]
        );
        let active = &s.groups[0].1;
        assert_eq!(active[0].sub, "You have slain 8 Drudge Raiders.");
        assert_eq!(active[1].sub, "1h 42m left to finish");
        assert!(active[1].timed && active[1].shown);
        let d = s.detail.expect("the first is picked");
        assert_eq!(d.id, 1);
        assert_eq!(d.places[0].role, "Given by");
        assert!(d.places[0].cell.is_some(), "outdoors has coordinates");
        assert_eq!(d.places[1].place, "Indoors");
        assert!(d.places[1].cell.is_none());
        assert_eq!(d.abandon, "Abandon");
    }

    #[test]
    fn a_search_narrows_the_list_and_a_done_quest_is_removed_not_abandoned() {
        let contracts = [tracked(1, 12, 0.0, 0.0), tracked(4, 3, 0.0, 0.0)];
        let catalog = catalog();
        let s = quests(&QuestsInput {
            contracts: &contracts,
            catalog: &catalog,
            picked: 1,
            search: "tusk",
            shown: &[],
            now_unix_ms: NOW,
        });
        assert_eq!(s.count, 2);
        assert_eq!(s.groups.len(), 1);
        let d = s.detail.expect("the first listed stands in for the pick");
        assert_eq!(d.id, 4);
        assert_eq!(d.abandon, "Remove from list");
    }

    #[test]
    fn notes_list_the_pages_and_one_countdown() {
        let page = |title: &str, label: &str, running: f64| JournalPage {
            title: title.to_owned(),
            label: label.to_owned(),
            running_timer_seconds: running,
            ..JournalPage::default()
        };
        let view = JournalView {
            pages: vec![page("Golem hearts", "farm", 0.0), page("", "", 7200.0)],
            current_page: 1,
            revision: 3,
            timer_started_unix_ms: Some(NOW - 600_000),
            timer_seconds_at_start: 7200.0,
        };
        let n = notes(&view, "", NOW);
        assert_eq!(n.rows.len(), 2);
        assert_eq!(n.rows[1].title, "Untitled page");
        assert_eq!(n.rows[1].left.as_deref(), Some("1h 50m"));
        assert_eq!(short(65.0), "1m 05s");
        assert_eq!(short(90_000.0), "1d 1h");
        assert!(n.other_running);
        assert!(n.left.is_none());
        assert_eq!(notes(&view, "golem", NOW).rows.len(), 1);
    }
}
