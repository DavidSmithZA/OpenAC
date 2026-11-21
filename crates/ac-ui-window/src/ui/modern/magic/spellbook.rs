//! What the Spellbook lists: the learned spells by school, through the
//! school and level filters the server keeps for the character (the same
//! bits retail's spellbook sets), a "highest level only" switch, a search
//! and an order; and the Components page's rows: what the player carries
//! of each component against the amount it is restocked to.

use ac_ui::modern::tone;
use std::collections::{BTreeMap, HashMap};

use ac_core::spells::MagicSchool;
use ac_ui::draw::{Color, TextureRef};
use ac_ui::modern::theme::hex;
use ac_ui::panels::magic::{school_filter_bit, spell_visible};

/// The schools in the order the book shows them, each with its name and
/// colour.
pub const SCHOOLS: [(MagicSchool, &str, Color); 5] = [
    (MagicSchool::WAR_MAGIC, "War Magic", hex(0xFF_9A6B, 1.0)),
    (MagicSchool::VOID_MAGIC, "Void Magic", hex(0xC9_A2FF, 1.0)),
    (MagicSchool::LIFE_MAGIC, "Life Magic", hex(0x7F_D08A, 1.0)),
    (
        MagicSchool::CREATURE_ENCHANTMENT,
        "Creature Enchantment",
        hex(0x7F_B2FF, 1.0),
    ),
    (
        MagicSchool::ITEM_ENCHANTMENT,
        "Item Enchantment",
        hex(0xE0_B068, 1.0),
    ),
];

/// The filter bits of every school.
pub const ALL_SCHOOLS: u32 = 0x0001 | 0x0002 | 0x0004 | 0x0008 | 0x2000;

/// The filter bit of level `level` (1 to 8).
#[must_use]
pub fn level_bit(level: i32) -> u32 {
    if (1..=8).contains(&level) {
        0x10 << (level - 1)
    } else {
        0
    }
}

/// The filter bits of every level.
pub const ALL_LEVELS: u32 = 0x0FF0;

/// A school's colour.
#[must_use]
pub fn school_color(school: MagicSchool) -> Color {
    SCHOOLS
        .iter()
        .find(|(s, _, _)| *s == school)
        .map_or(tone::soft(1.0), |(_, _, c)| *c)
}

/// A school's name.
#[must_use]
pub fn school_name(school: MagicSchool) -> &'static str {
    SCHOOLS
        .iter()
        .find(|(s, _, _)| *s == school)
        .map_or("Other", |(_, n, _)| n)
}

/// One learned spell.
#[derive(Debug, Clone, PartialEq)]
pub struct SpellRow {
    /// Its id.
    pub id: u32,
    /// Its name.
    pub name: String,
    /// Its school.
    pub school: MagicSchool,
    /// Its level (1 to 8; 0 when the client cannot say).
    pub level: i32,
    /// Its mana.
    pub mana: i32,
    /// Its family (spells of one family are levels of one spell).
    pub family: u32,
    /// The game's display order.
    pub sort_key: i32,
    /// Its icon.
    pub icon: Option<TextureRef>,
    /// The first of the player's bars it is on, by name.
    pub bar: Option<String>,
}

impl SpellRow {
    /// The row's second line ("Level VIII · 80 mana").
    #[must_use]
    pub fn sub(&self) -> String {
        let level = super::effects::roman(self.level)
            .map(|r| format!("Level {r} \u{b7} "))
            .unwrap_or_default();
        format!("{level}{} mana", self.mana.max(0))
    }
}

/// How the list is ordered within a school.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SpellSort {
    /// The game's order.
    #[default]
    School,
    /// By name.
    Name,
    /// Highest level first, then the game's order.
    Level,
}

impl SpellSort {
    /// The button's words.
    #[must_use]
    pub fn words(self) -> &'static str {
        match self {
            Self::School => "School",
            Self::Name => "Name",
            Self::Level => "Level",
        }
    }

    /// The next order.
    #[must_use]
    pub fn next(self) -> Self {
        match self {
            Self::School => Self::Name,
            Self::Name => Self::Level,
            Self::Level => Self::School,
        }
    }
}

/// What picks the spells shown.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SpellFilter {
    /// The server's filter bits (schools and levels).
    pub bits: u32,
    /// Only the highest level of each spell known.
    pub highest_only: bool,
    /// What is typed in the search.
    pub search: String,
    /// The order.
    pub sort: SpellSort,
}

/// The spells shown, by school in the book's order; a search looks past
/// the filters (it finds any learned spell by name).
#[must_use]
pub fn shown<'a>(rows: &'a [SpellRow], f: &SpellFilter) -> Vec<(MagicSchool, Vec<&'a SpellRow>)> {
    let wanted = f.search.trim().to_lowercase();
    let mut highest: HashMap<(MagicSchool, u32), i32> = HashMap::new();
    if f.highest_only {
        for r in rows.iter().filter(|r| r.family != 0) {
            let e = highest.entry((r.school, r.family)).or_insert(r.level);
            *e = (*e).max(r.level);
        }
    }
    let pass = |r: &&SpellRow| {
        if !wanted.is_empty() {
            return r.name.to_lowercase().contains(&wanted);
        }
        if !spell_visible(r.school, r.level, f.bits) {
            return false;
        }
        !f.highest_only
            || r.family == 0
            || highest
                .get(&(r.school, r.family))
                .is_none_or(|top| r.level >= *top)
    };
    SCHOOLS
        .iter()
        .filter_map(|(school, _, _)| {
            let mut list: Vec<&SpellRow> = rows
                .iter()
                .filter(|r| r.school == *school)
                .filter(pass)
                .collect();
            match f.sort {
                SpellSort::School => list.sort_by_key(|r| (r.sort_key, r.id)),
                SpellSort::Name => list.sort_by_cached_key(|r| r.name.to_lowercase()),
                SpellSort::Level => {
                    list.sort_by_key(|r| (std::cmp::Reverse(r.level), r.sort_key, r.id));
                }
            }
            (!list.is_empty()).then_some((*school, list))
        })
        .collect()
}

/// How many spells the player knows of each school, in the book's order.
#[must_use]
pub fn school_counts(rows: &[SpellRow]) -> [usize; 5] {
    let mut out = [0; 5];
    for r in rows {
        if let Some(i) = SCHOOLS.iter().position(|(s, _, _)| *s == r.school) {
            out[i] += 1;
        }
    }
    out
}

/// The filter bits with only `school` among the schools (none: every
/// school), the levels kept.
#[must_use]
pub fn with_school(bits: u32, school: Option<MagicSchool>) -> u32 {
    let schools = school.map_or(ALL_SCHOOLS, school_filter_bit);
    (bits & !ALL_SCHOOLS) | schools
}

/// The school chosen when exactly one is in the bits; none when all are
/// (or another mix).
#[must_use]
pub fn chosen_school(bits: u32) -> Option<MagicSchool> {
    let schools = bits & ALL_SCHOOLS;
    SCHOOLS
        .iter()
        .find(|(s, _, _)| school_filter_bit(*s) == schools)
        .map(|(s, _, _)| *s)
}

// ── Components ──────────────────────────────────────────────────────────

/// The component categories in order, with their words.
pub const CATEGORIES: [&str; 7] = [
    "Scarabs",
    "Herbs",
    "Powdered gems",
    "Alchemicals",
    "Talismans",
    "Tapers",
    "Peas",
];

/// A category's words for one component ("Scarab").
#[must_use]
pub fn category_one(category: u32) -> &'static str {
    match category {
        0 => "Scarab",
        1 => "Herb",
        2 => "Powdered gem",
        3 => "Alchemical",
        4 => "Talisman",
        5 => "Taper",
        6 => "Pea",
        _ => "Other",
    }
}

/// One component the player carries or restocks.
#[derive(Debug, Clone, PartialEq)]
pub struct ComponentRow {
    /// The component (its weenie class).
    pub component: u32,
    /// The first pack of it carried (0: none).
    pub object: u32,
    /// Its name.
    pub name: String,
    /// Its icon.
    pub icon: Option<TextureRef>,
    /// Its category (0 to 6; 8 unknown).
    pub category: u32,
    /// How many are carried.
    pub carried: i32,
    /// The amount it is restocked to (0: none).
    pub restock: u32,
}

impl ComponentRow {
    /// It is under its restock amount.
    #[must_use]
    pub fn short(&self) -> bool {
        self.restock > 0 && i64::from(self.carried) < i64::from(self.restock)
    }

    /// The share of the restock amount carried, 0..=1 (1 with none set).
    #[must_use]
    pub fn stock(&self) -> f32 {
        if self.restock == 0 {
            return 1.0;
        }
        (self.carried.max(0) as f32 / self.restock as f32).clamp(0.0, 1.0)
    }
}

/// What a component is, as the dats say.
#[derive(Debug, Clone, PartialEq)]
pub struct ComponentKind {
    /// Its name.
    pub name: String,
    /// Its category.
    pub category: u32,
    /// Its icon.
    pub icon: Option<TextureRef>,
}

/// The rows: every component carried (`carried`, their restock amounts
/// not yet filled in) and every one with a restock amount, by category
/// then name.
#[must_use]
pub fn component_rows(
    carried: Vec<ComponentRow>,
    restock: &BTreeMap<u32, u32>,
    kinds: &HashMap<u32, ComponentKind>,
) -> Vec<ComponentRow> {
    let mut rows: Vec<ComponentRow> = carried
        .into_iter()
        .map(|r| ComponentRow {
            restock: restock.get(&r.component).copied().unwrap_or(0),
            ..r
        })
        .collect();
    for (&component, &amount) in restock {
        if amount == 0 || rows.iter().any(|r| r.component == component) {
            continue;
        }
        let Some(kind) = kinds.get(&component) else {
            continue;
        };
        rows.push(ComponentRow {
            component,
            object: 0,
            name: kind.name.clone(),
            icon: kind.icon,
            category: kind.category,
            carried: 0,
            restock: amount,
        });
    }
    rows.sort_by(|a, b| {
        a.category
            .cmp(&b.category)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    rows
}

/// The rows of `category` (none: all) matching the search.
#[must_use]
pub fn components_shown<'a>(
    rows: &'a [ComponentRow],
    category: Option<u32>,
    search: &str,
) -> Vec<&'a ComponentRow> {
    let wanted = search.trim().to_lowercase();
    rows.iter()
        .filter(|r| category.is_none_or(|c| r.category == c))
        .filter(|r| wanted.is_empty() || r.name.to_lowercase().contains(&wanted))
        .collect()
}

/// The Components page's summary ("3 below restock · 1 none left").
#[must_use]
pub fn components_summary(rows: &[ComponentRow]) -> String {
    let short = rows.iter().filter(|r| r.short()).count();
    let none = rows.iter().filter(|r| r.carried <= 0).count();
    match (short, none) {
        (0, 0) => format!("{} carried, all stocked", rows.len()),
        (s, 0) => format!("{s} below restock"),
        (s, n) => format!("{s} below restock \u{b7} {n} none left"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(id: u32, name: &str, school: MagicSchool, level: i32, family: u32) -> SpellRow {
        SpellRow {
            id,
            name: name.to_owned(),
            school,
            level,
            mana: 10 * level,
            family,
            sort_key: id as i32,
            icon: None,
            bar: None,
        }
    }

    fn book() -> Vec<SpellRow> {
        vec![
            row(1, "Flame Bolt I", MagicSchool::WAR_MAGIC, 1, 7),
            row(2, "Flame Bolt VI", MagicSchool::WAR_MAGIC, 6, 7),
            row(3, "Heal Self VI", MagicSchool::LIFE_MAGIC, 6, 9),
            row(4, "Focus Self II", MagicSchool::CREATURE_ENCHANTMENT, 2, 11),
        ]
    }

    #[test]
    fn the_filters_pick_schools_and_levels() {
        let rows = book();
        let all = SpellFilter {
            bits: ALL_SCHOOLS | ALL_LEVELS,
            ..SpellFilter::default()
        };
        let shown_all = shown(&rows, &all);
        assert_eq!(shown_all.len(), 3);
        assert_eq!(shown_all[0].0, MagicSchool::WAR_MAGIC);
        let war_six = SpellFilter {
            bits: with_school(level_bit(6), Some(MagicSchool::WAR_MAGIC)),
            ..SpellFilter::default()
        };
        let s = shown(&rows, &war_six);
        assert_eq!(s.len(), 1);
        assert_eq!(s[0].1.len(), 1);
        assert_eq!(s[0].1[0].id, 2);
        assert_eq!(chosen_school(war_six.bits), Some(MagicSchool::WAR_MAGIC));
        assert_eq!(chosen_school(all.bits), None);
    }

    #[test]
    fn highest_only_hides_lower_levels_of_one_spell() {
        let rows = book();
        let f = SpellFilter {
            bits: ALL_SCHOOLS | ALL_LEVELS,
            highest_only: true,
            ..SpellFilter::default()
        };
        let ids: Vec<u32> = shown(&rows, &f)
            .into_iter()
            .flat_map(|(_, r)| r.into_iter().map(|r| r.id))
            .collect();
        assert_eq!(ids, vec![2, 3, 4]);
    }

    #[test]
    fn a_search_looks_past_the_filters() {
        let rows = book();
        let f = SpellFilter {
            bits: with_school(level_bit(1), Some(MagicSchool::WAR_MAGIC)),
            search: "heal".to_owned(),
            ..SpellFilter::default()
        };
        let s = shown(&rows, &f);
        assert_eq!(s.len(), 1);
        assert_eq!(s[0].1[0].id, 3);
    }

    #[test]
    fn rows_say_level_and_mana() {
        assert_eq!(book()[1].sub(), "Level VI \u{b7} 60 mana");
    }

    #[test]
    fn components_carried_and_restocked_make_one_list() {
        let carried = vec![ComponentRow {
            component: 10,
            object: 100,
            name: "Lead Scarab".to_owned(),
            icon: None,
            category: 0,
            carried: 5,
            restock: 0,
        }];
        let restock = BTreeMap::from([(10, 20), (11, 30), (12, 0)]);
        let kinds = HashMap::from([(
            11,
            ComponentKind {
                name: "Hyssop".to_owned(),
                category: 1,
                icon: None,
            },
        )]);
        let rows = component_rows(carried, &restock, &kinds);
        assert_eq!(rows.len(), 2);
        assert!(rows[0].short());
        assert_eq!(rows[1].name, "Hyssop");
        assert_eq!(rows[1].carried, 0);
        assert_eq!(
            components_summary(&rows),
            "2 below restock \u{b7} 1 none left"
        );
        assert_eq!(components_shown(&rows, Some(1), "").len(), 1);
        assert_eq!(components_shown(&rows, None, "lead").len(), 1);
    }
}
