//! The detailed map's row in Options › Graphics: what is on this computer
//! and under way, in words, with the buttons that fit (Download, Update,
//! Cancel, Remove) and how far a download has come.

use ac_world_map::{Busy, MapsStatus, Tier, size_words};

/// A button of the row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MapVerb {
    /// Download the tier, or bring it up to date.
    Download(Tier),
    /// Stop the download.
    Cancel,
    /// Delete the tier.
    Remove(Tier),
}

/// The row as it reads now.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct MapRow {
    /// Its name.
    pub label: String,
    /// What is on this computer and under way.
    pub desc: String,
    /// Its buttons, left to right, with their words.
    pub buttons: Vec<(MapVerb, &'static str)>,
    /// How far a download has come, 0 to 1.
    pub progress: Option<f32>,
}

/// The row for `status`, `chosen` being the tier the Map detail row picks
/// (none: the game's map).
#[must_use]
pub fn map_row(status: &MapsStatus, chosen: Option<Tier>) -> MapRow {
    let mut row = MapRow {
        label: "Detailed map".to_owned(),
        ..MapRow::default()
    };
    match &status.busy {
        Some(Busy::Downloading { tier, done, total }) => {
            let share = if *total > 0 {
                (*done as f64 / *total as f64).clamp(0.0, 1.0)
            } else {
                0.0
            };
            row.desc = format!(
                "Downloading {}: {} of {} ({:.0}%). Play on: the map shows it once it is all here; a stopped download carries on from where it was.",
                tier.words(),
                size_words(*done),
                size_words(*total),
                share * 100.0
            );
            row.buttons.push((MapVerb::Cancel, "Cancel"));
            row.progress = Some(share as f32);
            return row;
        }
        Some(Busy::Removing(tier)) => {
            row.desc = format!("Deleting {}.", tier.words());
            return row;
        }
        Some(Busy::Asking) | None => {}
    }
    let installed: Vec<Tier> = Tier::ALL
        .into_iter()
        .filter(|t| status.tier(*t).installed.is_some())
        .collect();
    match chosen {
        None => {
            row.desc = if installed.is_empty() {
                "No detailed map is on this computer. Choose a tier above to download one."
                    .to_owned()
            } else {
                format!(
                    "On this computer: {}. Choose one above to show it, or to delete it.",
                    list(&installed, status)
                )
            };
        }
        Some(t) => {
            let state = status.tier(t);
            if state.installed.is_some() {
                row.desc = format!(
                    "{}: on this computer ({}).",
                    t.words(),
                    size_words(state.bytes())
                );
                if state.update_out() {
                    row.desc.push_str(" A newer one is out.");
                    row.buttons.push((MapVerb::Download(t), "Update"));
                }
                row.buttons.push((MapVerb::Remove(t), "Remove"));
            } else {
                let size = if state.offered.is_some() {
                    size_words(state.bytes())
                } else {
                    format!("about {}", size_words(state.bytes()))
                };
                let until = match status.drawn(t) {
                    Some(d) => format!("the {} map", d.words()),
                    None => "the game's map".to_owned(),
                };
                row.desc = format!(
                    "{}: {size} to download. Until it is here, the map shows {until}.",
                    t.words()
                );
                row.buttons.push((MapVerb::Download(t), "Download"));
            }
            let others: Vec<Tier> = installed.into_iter().filter(|o| *o != t).collect();
            if !others.is_empty() {
                row.desc
                    .push_str(&format!(" Also here: {}.", list(&others, status)));
            }
        }
    }
    if let Some(f) = &status.failed {
        row.desc.push(' ');
        row.desc.push_str(f);
    }
    row
}

fn list(tiers: &[Tier], status: &MapsStatus) -> String {
    tiers
        .iter()
        .map(|t| format!("{} ({})", t.words(), size_words(status.tier(*t).bytes())))
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn with(installed: &[Tier]) -> MapsStatus {
        let mut s = MapsStatus::empty();
        for t in installed {
            s.tiers[*t as usize].installed = Some(1);
        }
        s
    }

    #[test]
    fn a_tier_not_here_offers_its_download_and_says_what_shows_meanwhile() {
        let r = map_row(&with(&[]), Some(Tier::Extreme));
        assert_eq!(r.label, "Detailed map");
        assert_eq!(
            r.buttons,
            vec![(MapVerb::Download(Tier::Extreme), "Download")]
        );
        assert!(
            r.desc.starts_with("Extreme: about 885 MB to download"),
            "{}",
            r.desc
        );
        assert!(r.desc.contains("the game's map"));
        let r = map_row(&with(&[Tier::Standard]), Some(Tier::Hd));
        assert!(r.desc.contains("shows the Standard map"), "{}", r.desc);
        assert!(r.desc.contains("Also here: Standard"));
    }

    #[test]
    fn a_tier_here_can_be_removed_or_brought_up_to_date() {
        let mut s = with(&[Tier::Hd]);
        let r = map_row(&s, Some(Tier::Hd));
        assert_eq!(r.buttons, vec![(MapVerb::Remove(Tier::Hd), "Remove")]);
        s.tiers[Tier::Hd as usize].offered = Some((2, 40_000_000));
        let r = map_row(&s, Some(Tier::Hd));
        assert_eq!(
            r.buttons,
            vec![
                (MapVerb::Download(Tier::Hd), "Update"),
                (MapVerb::Remove(Tier::Hd), "Remove")
            ]
        );
        assert!(r.desc.contains("40.0 MB"));
    }

    #[test]
    fn a_download_shows_how_far_it_has_come_and_can_be_stopped() {
        let mut s = with(&[]);
        s.busy = Some(Busy::Downloading {
            tier: Tier::Ultra,
            done: 25_000_000,
            total: 100_000_000,
        });
        let r = map_row(&s, None);
        assert_eq!(r.buttons, vec![(MapVerb::Cancel, "Cancel")]);
        assert_eq!(r.progress, Some(0.25));
        assert!(
            r.desc
                .starts_with("Downloading Ultra: 25.0 MB of 100 MB (25%)"),
            "{}",
            r.desc
        );
    }

    #[test]
    fn the_games_map_lists_what_is_here_and_a_failure_is_told() {
        let r = map_row(&with(&[]), None);
        assert!(r.buttons.is_empty());
        assert!(r.desc.starts_with("No detailed map"));
        let mut s = with(&[Tier::Standard]);
        s.failed = Some("The HD map did not download: no network.".to_owned());
        let r = map_row(&s, None);
        assert!(r.desc.contains("On this computer: Standard"));
        assert!(r.desc.ends_with("no network."));
    }
}
