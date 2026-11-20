//! What an item sheet says: a head, a few facts, and tabs of sections of
//! rows, paragraphs and spells. The item's tabs (Overview, Spells, Needs,
//! Details) are retail's item report sorted by part, each line kept as
//! retail words it ("Value: 1,240" as a row, a sentence as a paragraph),
//! so an item reads the same in a pack, on the ground or in retail.
//! Inspect's other kinds (creatures, players, portals, spells) build the
//! same shape.

use ac_ui::draw::{Color, TextureRef};
use ac_ui::panels::appraisal::{ItemReportParts, ReportFragment, ReportPart, ReportStyle};

/// How a row's value is marked.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Mark {
    /// Plain.
    #[default]
    None,
    /// Raised by a spell (green, an arrow up).
    Up,
    /// Lowered by a spell (red, an arrow down).
    Down,
    /// A need met (green, a tick).
    Met,
    /// A need not met (red, a cross).
    Unmet,
    /// Gold (a set worn, a component running low).
    Gold,
}

/// One entry in a section.
#[derive(Debug, Clone, PartialEq)]
pub enum Entry {
    /// A name and its value.
    Row {
        /// The name.
        key: String,
        /// The value.
        value: String,
        /// Its mark.
        mark: Mark,
    },
    /// A name, a value and a small bar (a vital).
    Bar {
        /// The name.
        key: String,
        /// The value.
        value: String,
        /// How full, 0..1.
        share: f32,
        /// The bar's colour.
        color: Color,
    },
    /// A paragraph.
    Para {
        /// Its words.
        text: String,
        /// Its mark (for its colour).
        mark: Mark,
    },
    /// A spell and what it does.
    Spell {
        /// Its name.
        name: String,
        /// What it does.
        text: String,
        /// A cantrip or enchantment (gold).
        gold: bool,
    },
    /// A plugin's mark on the item, as a chip in its colour.
    Verdict {
        /// Its words.
        words: String,
        /// Its colour.
        color: Color,
    },
    /// An inscription.
    Inscription {
        /// Its words.
        text: String,
        /// The player may change it.
        editable: bool,
        /// The note under it.
        note: String,
    },
}

/// A titled run of entries.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Section {
    /// The small capitals over it (empty: no title).
    pub title: String,
    /// The count beside the title.
    pub count: String,
    /// Its entries.
    pub entries: Vec<Entry>,
}

/// One tab.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Tab {
    /// Its name.
    pub label: String,
    /// Its sections.
    pub sections: Vec<Section>,
}

/// The head.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Head {
    /// The icon (things; none for creatures and players).
    pub icon: Option<TextureRef>,
    /// The name.
    pub name: String,
    /// The name's colour (none: the text colour).
    pub name_color: Option<Color>,
    /// The line under the name.
    pub sub: String,
    /// The level, top right (creatures and players).
    pub level: Option<String>,
    /// Health, 0..1 (creatures and players).
    pub health: Option<f32>,
    /// The assessment failed.
    pub failed: bool,
}

/// A whole sheet.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Sheet {
    /// What it is about (an object id; 0 for a spell).
    pub subject: u32,
    /// The head.
    pub head: Head,
    /// Up to three facts under the head (name, value).
    pub facts: Vec<(String, String)>,
    /// The tabs (one: no tab row).
    pub tabs: Vec<Tab>,
}

impl Sheet {
    /// Any value is marked raised or lowered (the legend shows).
    #[must_use]
    pub fn has_spell_marks(&self) -> bool {
        self.tabs.iter().flat_map(|t| &t.sections).any(|s| {
            s.entries.iter().any(|e| {
                matches!(
                    e,
                    Entry::Row {
                        mark: Mark::Up | Mark::Down,
                        ..
                    }
                )
            })
        })
    }
}

/// The item tabs, in order, and the parts each holds.
const TABS: [(&str, &[ReportPart]); 4] = {
    use ReportPart as P;
    [
        (
            "Overview",
            &[
                P::ValueAndBurden,
                P::WeaponAndArmor,
                P::DefenseModifiers,
                P::ArmorModifiers,
                P::SetAndRatings,
                P::ShortMagic,
                P::SpecialProperties,
                P::Usage,
                P::BoostAndHealing,
                P::CapacityAndLock,
                P::ManaStone,
                P::RemainingUses,
                P::SaleAndRare,
            ],
        ),
        ("Spells", &[P::CasterData, P::MagicInfo]),
        (
            "Needs",
            &[
                P::LevelLimits,
                P::WieldRequirements,
                P::UsageLimits,
                P::ActivationRequirements,
                P::ItemLevel,
            ],
        ),
        ("Details", &[P::Tinkering, P::Craftsman, P::Description]),
    ]
};

/// The small capitals over a part's section.
fn part_title(part: ReportPart, weapon: bool) -> &'static str {
    use ReportPart as P;
    match part {
        P::ValueAndBurden => "VALUE",
        P::Tinkering => "TINKERING",
        P::SetAndRatings => "SET AND RATINGS",
        P::WeaponAndArmor if weapon => "WEAPON",
        P::WeaponAndArmor | P::ArmorModifiers => "PROTECTION",
        P::DefenseModifiers => "DEFENSE",
        P::ShortMagic => "MAGIC",
        P::SpecialProperties => "PROPERTIES",
        P::Usage => "USE",
        P::LevelLimits => "LEVEL",
        P::WieldRequirements => "TO WIELD",
        P::UsageLimits => "TO USE",
        P::ItemLevel => "ITEM LEVEL",
        P::ActivationRequirements => "TO ACTIVATE",
        P::CasterData => "CASTING",
        P::BoostAndHealing => "EFFECT",
        P::CapacityAndLock => "CAPACITY",
        P::ManaStone => "MANA STONE",
        P::RemainingUses => "USES",
        P::Craftsman => "MADE BY",
        P::SaleAndRare => "NOTES",
        P::MagicInfo => "SPELLS",
        P::Description => "DESCRIPTION",
    }
}

fn mark_of(style: ReportStyle) -> Mark {
    match style {
        ReportStyle::Normal => Mark::None,
        ReportStyle::Beneficial => Mark::Up,
        ReportStyle::Detrimental => Mark::Down,
    }
}

/// A name and its value from "Name: value", when the line reads as one
/// (a short name with no sentence in it).
fn key_value(line: &str) -> Option<(String, String)> {
    let (key, value) = line.split_once(": ")?;
    let key = key.trim();
    let value = value.trim().trim_end_matches('.');
    (!key.is_empty() && key.len() <= 28 && !key.contains('.') && !value.is_empty())
        .then(|| (key.to_owned(), value.to_owned()))
}

/// Sections from one part's fragments: a line "Name: value" is a row, a
/// spell line ("~ Name: what it does") a spell, a line ending in a colon
/// opens a section of its own, anything else is a paragraph.
fn sections_of(title: &str, fragments: &[ReportFragment]) -> Vec<Section> {
    let mut out = vec![Section {
        title: title.to_owned(),
        ..Section::default()
    }];
    for f in fragments {
        let mark = mark_of(f.style);
        for line in f.text.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let section = out.last_mut().expect("one section at least");
            if let Some(spell) = line.strip_prefix("~ ") {
                let (name, text) = spell.split_once(':').unwrap_or((spell, ""));
                section.entries.push(Entry::Spell {
                    name: name.trim().to_owned(),
                    text: text.trim().to_owned(),
                    gold: false,
                });
            } else if let Some(heading) = line.strip_suffix(':')
                && !heading.contains(':')
                && heading.len() <= 28
            {
                let heading = heading.trim();
                let heading = if heading == "Spell Descriptions" {
                    // What came before it (spellcraft, mana) is casting.
                    if !section.entries.is_empty() {
                        section.title = "CASTING".to_owned();
                    }
                    title.to_owned()
                } else {
                    heading.to_uppercase()
                };
                if section.entries.is_empty() {
                    section.title = heading;
                } else {
                    out.push(Section {
                        title: heading,
                        ..Section::default()
                    });
                }
            } else if let Some((key, value)) = key_value(line) {
                section.entries.push(Entry::Row { key, value, mark });
            } else {
                section.entries.push(Entry::Para {
                    text: line.to_owned(),
                    mark,
                });
            }
        }
    }
    out.retain(|s| !s.entries.is_empty());
    for s in &mut out {
        let spells = s
            .entries
            .iter()
            .filter(|e| matches!(e, Entry::Spell { .. }))
            .count();
        if spells > 1 {
            s.count = spells.to_string();
        }
        if s.title == "ENCHANTMENTS" {
            for e in &mut s.entries {
                if let Entry::Spell { gold, .. } = e {
                    *gold = true;
                }
            }
        }
    }
    out
}

/// The item tabs from its report: each part's sections in its tab, the
/// sections that follow under one title merged, and empty tabs left out.
/// `inscription` goes at the head of Details.
#[must_use]
pub fn item_tabs(parts: &ItemReportParts, weapon: bool, inscription: Option<Entry>) -> Vec<Tab> {
    let mut tabs = Vec::new();
    for (label, which) in TABS {
        let mut sections: Vec<Section> = Vec::new();
        if label == "Details"
            && let Some(e) = &inscription
        {
            sections.push(Section {
                title: "INSCRIPTION".to_owned(),
                count: String::new(),
                entries: vec![e.clone()],
            });
        }
        for &part in which {
            let fragments = parts.fragments(part);
            if fragments.is_empty() {
                continue;
            }
            for s in sections_of(part_title(part, weapon), fragments) {
                match sections.last_mut() {
                    Some(last) if last.title == s.title => last.entries.extend(s.entries),
                    _ => sections.push(s),
                }
            }
        }
        if !sections.is_empty() {
            tabs.push(Tab {
                label: label.to_owned(),
                sections,
            });
        }
    }
    tabs
}

#[cfg(test)]
mod tests {
    use super::*;
    use ac_ui::panels::appraisal::Separator;

    fn frag(text: &str, style: ReportStyle) -> ReportFragment {
        ReportFragment {
            text: text.into(),
            separator: Separator::Line,
            style,
        }
    }

    #[test]
    fn retail_lines_become_rows_paragraphs_and_spells() {
        let s = sections_of(
            "SPELLS",
            &[
                frag("Spellcraft: 280.", ReportStyle::Normal),
                frag("Mana: 2410 / 3000.", ReportStyle::Normal),
                frag(
                    "Spell Descriptions:\n~ Defender VI: Raises defense.\n~ Hermetic Link VI: Mana.",
                    ReportStyle::Normal,
                ),
                frag("Enchantments:\n~ Major Strength: +15.", ReportStyle::Normal),
                frag(
                    "Bonded to you. It cannot be given away.",
                    ReportStyle::Normal,
                ),
                frag("Attack Bonus: +12%", ReportStyle::Beneficial),
            ],
        );
        assert_eq!(s.len(), 3);
        assert_eq!(s[0].title, "CASTING");
        assert_eq!(s[1].title, "SPELLS");
        assert_eq!(s[1].count, "2");
        assert_eq!(
            s[0].entries[0],
            Entry::Row {
                key: "Spellcraft".into(),
                value: "280".into(),
                mark: Mark::None
            }
        );
        assert!(matches!(&s[1].entries[0], Entry::Spell { name, .. } if name == "Defender VI"));
        assert_eq!(s[2].title, "ENCHANTMENTS");
        assert!(matches!(&s[2].entries[0], Entry::Spell { gold: true, .. }));
        assert!(matches!(&s[2].entries[1], Entry::Para { .. }));
        assert_eq!(
            s[2].entries[2],
            Entry::Row {
                key: "Attack Bonus".into(),
                value: "+12%".into(),
                mark: Mark::Up
            }
        );
    }

    #[test]
    fn empty_tabs_are_left_out() {
        let parts = ItemReportParts {
            report: ac_ui::panels::appraisal::ItemAppraisalReport {
                fragments: vec![
                    frag("Value: 350", ReportStyle::Normal),
                    frag("Burden: 140", ReportStyle::Normal),
                    frag("A healing draught.", ReportStyle::Normal),
                ],
            },
            parts: vec![
                (ReportPart::ValueAndBurden, 0..2),
                (ReportPart::Description, 2..3),
            ],
        };
        let tabs = item_tabs(&parts, false, None);
        let labels: Vec<_> = tabs.iter().map(|t| t.label.as_str()).collect();
        assert_eq!(labels, ["Overview", "Details"]);
        assert_eq!(tabs[0].sections[0].title, "VALUE");
        assert_eq!(tabs[1].sections[0].title, "DESCRIPTION");
    }
}
