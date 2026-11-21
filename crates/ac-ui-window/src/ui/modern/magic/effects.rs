//! What the Effects window lists: every effect on the player, helpful or
//! harmful by its spell, with what it changes, who cast it (or which worn
//! item it comes from), its time left of the whole, and a weaker copy
//! waiting underneath to take over when it ends.
//!
//! The window is retail's positive, negative and vitae panels in one; the
//! HUD's counts read the same enchantments ([`super::super::hud::effects`]).

use ac_core::spells::{
    ActiveEnchantmentRecord, BUCKET_VITAE, EnchantmentTypeFlags, STAT_KEY_MAX_HEALTH,
    STAT_KEY_MAX_MANA, STAT_KEY_MAX_STAMINA,
};
use ac_core::spells::{SpellMetadata, Spellbook};
use ac_ui::panels::indicators::format_seconds_left;

/// What an effect changes, for the window's filter chips.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EffectKind {
    /// An attribute.
    Attributes,
    /// A skill (or every attack or defense skill).
    Skills,
    /// A vital's maximum or how fast it comes back.
    Vitals,
    /// Damage taken, natural armor.
    Protections,
    /// Anything else (shown under All only).
    Other,
}

impl EffectKind {
    /// The filter chips after All, in order.
    pub const FILTERS: [Self; 4] = [
        Self::Attributes,
        Self::Skills,
        Self::Vitals,
        Self::Protections,
    ];

    /// The chip's words.
    #[must_use]
    pub fn words(self) -> &'static str {
        match self {
            Self::Attributes => "Attributes",
            Self::Skills => "Skills",
            Self::Vitals => "Vitals",
            Self::Protections => "Protections",
            Self::Other => "Other",
        }
    }
}

/// One stat of the player's an effect changes, as the window asks its value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Stat {
    /// An attribute by id (1 Strength to 6 Self).
    Attribute(u32),
    /// A vital's maximum by its stat key (1 health, 3 stamina, 5 mana).
    Vital(u32),
    /// A skill by id.
    Skill(u32),
}

/// Where an effect comes from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Source {
    /// The player cast it.
    You,
    /// Someone else did (their name, or none when the client cannot say).
    Caster(Option<String>),
    /// An item the player wears (its id and name); it lasts while it is
    /// worn.
    Gear { item: u32, name: Option<String> },
}

impl Source {
    /// The row's few words: "cast by you", "from Mira", the item's name.
    #[must_use]
    pub fn row_words(&self) -> String {
        match self {
            Self::You => "cast by you".to_owned(),
            Self::Caster(Some(name)) => format!("from {name}"),
            Self::Caster(None) => "from someone".to_owned(),
            Self::Gear { name, .. } => name.clone().unwrap_or_else(|| "your gear".to_owned()),
        }
    }

    /// The details' value beside "Cast by" or "From".
    #[must_use]
    pub fn detail_words(&self) -> String {
        match self {
            Self::You => "You".to_owned(),
            Self::Caster(Some(name)) => name.clone(),
            Self::Caster(None) => "Someone".to_owned(),
            Self::Gear { name, .. } => name.clone().unwrap_or_else(|| "Your gear".to_owned()),
        }
    }
}

/// One effect on the player.
#[derive(Debug, Clone, PartialEq)]
pub struct Effect {
    /// Its enchantment (spell and layer), which picks it in the window.
    pub identity: u32,
    /// Its spell.
    pub spell_id: u32,
    /// The spell's name.
    pub name: String,
    /// "Creature Enchantment · Level VIII", "Cantrip · from your gear".
    pub school: String,
    /// The game's description of the spell.
    pub description: String,
    /// What it changes, for the chips.
    pub kind: EffectKind,
    /// The stat it changes in words ("Focus", "Fire damage taken"); empty
    /// when the client cannot say.
    pub stat: String,
    /// By how much ("+45", "–35", "lower", "faster"; the en dash stands for
    /// minus, which the baked fonts lack).
    pub delta: String,
    /// The stat's value without and with it, when the client knows them.
    pub from_to: Option<(i32, i32)>,
    /// Who cast it, or the item it comes from.
    pub source: Source,
    /// Seconds left; none for one with no end.
    pub left: Option<f64>,
    /// Its whole duration, seconds; none for one with no end.
    pub total: Option<f64>,
    /// A weaker copy waiting underneath, in words.
    pub under: Option<String>,
    /// It is harmful.
    pub harmful: bool,
}

impl Effect {
    /// The share of its time still left, 0..=1 (1 with no end).
    #[must_use]
    pub fn share_left(&self) -> f32 {
        match (self.left, self.total) {
            (Some(left), Some(total)) if total > 0.0 => (left / total).clamp(0.0, 1.0) as f32,
            (Some(_), _) => 0.0,
            _ => 1.0,
        }
    }

    /// What it changes in the row's few words ("Focus +45").
    #[must_use]
    pub fn change_words(&self) -> String {
        match (self.stat.is_empty(), self.delta.is_empty()) {
            (false, false) => format!("{} {}", self.stat, self.delta),
            (false, true) => self.stat.clone(),
            (true, false) => self.delta.clone(),
            (true, true) => String::new(),
        }
    }

    /// What it changes in the details' words ("Focus 210 to 255").
    #[must_use]
    pub fn stat_words(&self) -> String {
        match self.from_to {
            Some((from, to)) => format!("{} {from} to {to}", self.stat),
            None => self.stat.clone(),
        }
    }

    /// Whether it matches what was typed in the search: its name or what it
    /// changes, ignoring case.
    #[must_use]
    pub fn matches(&self, search: &str) -> bool {
        let wanted = search.trim().to_lowercase();
        wanted.is_empty()
            || self.name.to_lowercase().contains(&wanted)
            || self.stat.to_lowercase().contains(&wanted)
    }
}

/// Every effect on the player at one moment.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct EffectsList {
    /// The helpful ones in effect, those with an end soonest first, then
    /// those without by name.
    pub helpful: Vec<Effect>,
    /// The harmful ones, in the same order.
    pub harmful: Vec<Effect>,
    /// What vitae leaves (0.94 for 6% lower); none without vitae.
    pub vitae: Option<f32>,
}

/// What the list is read from.
pub struct ReadFrom<'a> {
    /// The player's spellbook (enchantments and spell table).
    pub book: &'a Spellbook,
    /// The enchantments' clock now.
    pub now: f64,
    /// The player's id.
    pub player: u32,
    /// An object's name, if the client knows it.
    pub name_of: &'a dyn Fn(u32) -> Option<String>,
    /// A stat's value now, with every effect on it.
    pub value_of: &'a dyn Fn(Stat) -> Option<i32>,
}

impl std::fmt::Debug for ReadFrom<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ReadFrom")
            .field("now", &self.now)
            .field("player", &self.player)
            .finish_non_exhaustive()
    }
}

/// The level's numeral ("VIII"), for levels 1 to 8.
#[must_use]
pub fn roman(level: i32) -> Option<&'static str> {
    const NUMERALS: [&str; 8] = ["I", "II", "III", "IV", "V", "VI", "VII", "VIII"];
    usize::try_from(level - 1)
        .ok()
        .and_then(|i| NUMERALS.get(i).copied())
}

/// The spell's school and level in words ("Life Magic · Level VI").
#[must_use]
pub fn school_words(spell: &SpellMetadata) -> String {
    match roman(spell.generation) {
        Some(level) => format!("{} \u{b7} Level {level}", spell.school),
        None => spell.school.clone(),
    }
}

/// The float property keys of the damage resistances, and their words.
const RESISTANCES: [(u32, &str); 8] = [
    (64, "Slashing"),
    (65, "Piercing"),
    (66, "Bludgeoning"),
    (67, "Fire"),
    (68, "Cold"),
    (69, "Acid"),
    (70, "Lightning"),
    (166, "Nether"),
];

/// The float property keys of the vitals' rates.
const RATES: [(u32, &str); 3] = [(3, "Health"), (4, "Stamina"), (5, "Mana")];

/// A signed whole amount as the window writes it ("+45", "–35").
fn signed(amount: f32) -> String {
    let whole = amount.round() as i64;
    if whole < 0 {
        format!("\u{2013}{}", whole.unsigned_abs())
    } else {
        format!("+{whole}")
    }
}

/// What `record` changes: its kind, the stat (when it is one of the
/// player's numbers), the stat in words and the change in words.
fn change_of(record: &ActiveEnchantmentRecord) -> (EffectKind, Option<Stat>, String, String) {
    let bits = record.stat_mod_type.unwrap_or(0);
    let has = |f: EnchantmentTypeFlags| bits & f.bits() != 0;
    let key = record.stat_mod_key.unwrap_or(0);
    let value = record.stat_mod_value.unwrap_or(0.0);
    let multiplies = has(EnchantmentTypeFlags::MULTIPLICATIVE);
    let amount = || {
        if multiplies {
            let percent = ((value - 1.0) * 100.0).round();
            if percent < 0.0 {
                format!("\u{2013}{}%", percent.abs())
            } else {
                format!("+{percent}%")
            }
        } else {
            signed(value)
        }
    };
    let several = has(EnchantmentTypeFlags::MULTIPLE_STAT);
    if has(EnchantmentTypeFlags::ATTACK_SKILLS) {
        return (
            EffectKind::Skills,
            None,
            "Attack skills".to_owned(),
            amount(),
        );
    }
    if has(EnchantmentTypeFlags::DEFENSE_SKILLS) {
        return (
            EffectKind::Skills,
            None,
            "Defense skills".to_owned(),
            amount(),
        );
    }
    if has(EnchantmentTypeFlags::ATTRIBUTE) {
        let name = ac_core::player::attribute_name(key);
        return if several || name.is_empty() {
            (
                EffectKind::Attributes,
                None,
                "All attributes".to_owned(),
                amount(),
            )
        } else {
            (
                EffectKind::Attributes,
                Some(Stat::Attribute(key)),
                name.to_owned(),
                amount(),
            )
        };
    }
    if has(EnchantmentTypeFlags::SECOND_ATT) {
        let name = match key {
            STAT_KEY_MAX_HEALTH => "Maximum Health",
            STAT_KEY_MAX_STAMINA => "Maximum Stamina",
            STAT_KEY_MAX_MANA => "Maximum Mana",
            _ => "Vitals",
        };
        let stat = (!several && name != "Vitals").then_some(Stat::Vital(key));
        return (EffectKind::Vitals, stat, name.to_owned(), amount());
    }
    if has(EnchantmentTypeFlags::SKILL) {
        let name = ac_core::player::skill_names::try_get_name(key as i32);
        return match name {
            Some(name) if !several => (
                EffectKind::Skills,
                Some(Stat::Skill(key)),
                name.to_owned(),
                amount(),
            ),
            _ => (EffectKind::Skills, None, "Skills".to_owned(), amount()),
        };
    }
    if has(EnchantmentTypeFlags::BODY_ARMOR_VALUE) {
        return (
            EffectKind::Protections,
            None,
            "Natural armor".to_owned(),
            amount(),
        );
    }
    if has(EnchantmentTypeFlags::FLOAT) {
        if let Some((_, element)) = RESISTANCES.iter().find(|(k, _)| *k == key) {
            // A resistance multiplies the damage taken: under 1 is less.
            let way = if value < 1.0 { "lower" } else { "higher" };
            return (
                EffectKind::Protections,
                None,
                format!("{element} damage taken"),
                way.to_owned(),
            );
        }
        if let Some((_, vital)) = RATES.iter().find(|(k, _)| *k == key) {
            let way = if value >= 1.0 { "faster" } else { "slower" };
            return (
                EffectKind::Vitals,
                None,
                format!("{vital} regained"),
                way.to_owned(),
            );
        }
    }
    (EffectKind::Other, None, String::new(), String::new())
}

/// The stat's value without the record and with it, from its value now.
fn from_to(record: &ActiveEnchantmentRecord, now: i32) -> Option<(i32, i32)> {
    let bits = record.stat_mod_type.unwrap_or(0);
    let value = record.stat_mod_value?;
    if bits & EnchantmentTypeFlags::ADDITIVE.bits() != 0 {
        let without = now - value.round() as i32;
        return Some((without, now));
    }
    if bits & EnchantmentTypeFlags::MULTIPLICATIVE.bits() != 0 && value.abs() > f32::EPSILON {
        let without = (now as f32 / value).round() as i32;
        return Some((without, now));
    }
    None
}

impl EffectsList {
    /// The effects in `src.book`: those in effect, helpful or harmful by
    /// their spell (spells the client cannot name are left out, as the
    /// retail panels leave them out).
    #[must_use]
    pub fn read(src: &ReadFrom<'_>) -> Self {
        let book = src.book;
        let in_effect = book.enchantments_in_effect_snapshot();
        let held: Vec<u32> = in_effect
            .iter()
            .map(ActiveEnchantmentRecord::identity)
            .collect();
        let waiting: Vec<&ActiveEnchantmentRecord> = book
            .active_enchantments()
            .iter()
            .filter(|r| r.bucket != BUCKET_VITAE && !held.contains(&r.identity()))
            .collect();
        let left_of = |r: &ActiveEnchantmentRecord| {
            (r.duration >= 0.0).then(|| (r.start_time + r.duration - src.now).max(0.0))
        };
        let source_of = |r: &ActiveEnchantmentRecord| {
            if r.caster_guid == src.player && r.duration >= 0.0 {
                Source::You
            } else if r.duration < 0.0 {
                Source::Gear {
                    item: r.caster_guid,
                    name: (src.name_of)(r.caster_guid),
                }
            } else {
                Source::Caster((src.name_of)(r.caster_guid))
            }
        };
        let mut helpful = Vec::new();
        let mut harmful = Vec::new();
        for record in &in_effect {
            let Some(spell) = book.try_get_metadata(record.spell_id) else {
                continue;
            };
            let (kind, stat, stat_words, delta) = change_of(record);
            let source = source_of(record);
            let gear = matches!(source, Source::Gear { .. });
            let under = (record.spell_category != 0)
                .then(|| {
                    waiting
                        .iter()
                        .filter(|w| w.spell_category == record.spell_category)
                        .max_by(|a, b| {
                            a.power_level.cmp(&b.power_level).then(
                                left_of(a)
                                    .unwrap_or(f64::MAX)
                                    .total_cmp(&left_of(b).unwrap_or(f64::MAX)),
                            )
                        })
                })
                .flatten()
                .and_then(|w| {
                    let name = book.try_get_metadata(w.spell_id)?.name.clone();
                    let (_, _, _, amount) = change_of(w);
                    let who = match source_of(w) {
                        Source::You => "cast by you".to_owned(),
                        other => other.row_words(),
                    };
                    let time = match left_of(w) {
                        Some(t) => format!("{} left", format_seconds_left(t)),
                        None => "while you wear it".to_owned(),
                    };
                    let detail = if amount.is_empty() {
                        time
                    } else {
                        format!("{amount}, {time}")
                    };
                    Some(format!(
                        "Underneath: {name}, {who} ({detail}). It takes over if this one ends first."
                    ))
                });
            let effect = Effect {
                identity: record.identity(),
                spell_id: record.spell_id,
                name: spell.name.clone(),
                school: if gear {
                    format!("{} \u{b7} from your gear", spell.school)
                } else {
                    school_words(spell)
                },
                description: spell.description.clone(),
                kind,
                stat: stat_words,
                delta,
                from_to: stat
                    .and_then(|s| (src.value_of)(s))
                    .and_then(|now| from_to(record, now)),
                source,
                left: left_of(record),
                total: (record.duration >= 0.0).then_some(record.duration),
                under,
                harmful: !spell.is_beneficial(),
            };
            if effect.harmful {
                harmful.push(effect);
            } else {
                helpful.push(effect);
            }
        }
        for list in [&mut helpful, &mut harmful] {
            sort(list, Sort::TimeLeft);
        }
        // Read as the Character window reads it (the last vitae record).
        let vitae = Some(ac_ui::panels::indicators::current_vitae(Some(book))).filter(|v| *v < 1.0);
        Self {
            helpful,
            harmful,
            vitae,
        }
    }

    /// How many of the helpful ones change things of `kind` (all of them
    /// with none).
    #[must_use]
    pub fn helpful_of(&self, kind: Option<EffectKind>) -> usize {
        self.helpful
            .iter()
            .filter(|e| kind.is_none_or(|k| e.kind == k))
            .count()
    }

    /// The effect `identity`, helpful or harmful.
    #[must_use]
    pub fn find(&self, identity: u32) -> Option<&Effect> {
        self.helpful
            .iter()
            .chain(&self.harmful)
            .find(|e| e.identity == identity)
    }
}

/// How the window orders a list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Sort {
    /// Soonest to end first; those with no end last, by name.
    #[default]
    TimeLeft,
    /// By name.
    Name,
    /// By what it changes, then by name.
    Change,
}

impl Sort {
    /// Every order, as the sort button steps through them.
    pub const ALL: [Self; 3] = [Self::TimeLeft, Self::Name, Self::Change];

    /// The button's words.
    #[must_use]
    pub fn words(self) -> &'static str {
        match self {
            Self::TimeLeft => "Time left",
            Self::Name => "Name",
            Self::Change => "What it changes",
        }
    }

    /// The next order.
    #[must_use]
    pub fn next(self) -> Self {
        let at = Self::ALL.iter().position(|s| *s == self).unwrap_or(0);
        Self::ALL[(at + 1) % Self::ALL.len()]
    }
}

/// How `a` and `b` compare in the order `by`.
#[must_use]
pub fn compare(a: &Effect, b: &Effect, by: Sort) -> std::cmp::Ordering {
    let name = |e: &Effect| e.name.to_lowercase();
    match by {
        Sort::TimeLeft => match (a.left, b.left) {
            (Some(x), Some(y)) => x.total_cmp(&y),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => name(a).cmp(&name(b)),
        },
        Sort::Name => name(a).cmp(&name(b)),
        Sort::Change => a
            .stat
            .to_lowercase()
            .cmp(&b.stat.to_lowercase())
            .then_with(|| name(a).cmp(&name(b))),
    }
}

/// Orders `list` by `by`.
pub fn sort(list: &mut [Effect], by: Sort) {
    list.sort_by(|a, b| compare(a, b, by));
}

/// A group of the window's list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Group {
    /// Helpful, ending within the warning time.
    EndingSoon,
    /// Helpful, with an end.
    Timed,
    /// Helpful, from worn gear (no end).
    Gear,
    /// Harmful.
    EndingFirst,
}

impl Group {
    /// The heading, in capitals.
    #[must_use]
    pub fn title(self) -> &'static str {
        match self {
            Self::EndingSoon => "ENDING SOON",
            Self::Timed => "TIMED",
            Self::Gear => "FROM YOUR GEAR",
            Self::EndingFirst => "ENDING FIRST",
        }
    }

    /// The group a helpful effect is in, `soon` seconds being the warning
    /// time (0: never).
    #[must_use]
    pub fn of_helpful(e: &Effect, soon: f64) -> Self {
        match e.left {
            None => Self::Gear,
            Some(t) if t < soon => Self::EndingSoon,
            Some(_) => Self::Timed,
        }
    }
}

/// The helpful (or harmful) effects that pass the filter and the search,
/// in groups, each in the order asked for.
#[must_use]
pub fn grouped<'a>(
    list: &'a EffectsList,
    harmful: bool,
    filter: Option<EffectKind>,
    search: &str,
    by: Sort,
    soon: f64,
) -> Vec<(Group, Vec<&'a Effect>)> {
    let pick = |e: &&Effect| (harmful || filter.is_none_or(|k| e.kind == k)) && e.matches(search);
    let order = |mut v: Vec<&'a Effect>| {
        v.sort_by(|a, b| compare(a, b, by));
        v
    };
    if harmful {
        let rows: Vec<&Effect> = list.harmful.iter().filter(pick).collect();
        return if rows.is_empty() {
            Vec::new()
        } else {
            vec![(Group::EndingFirst, order(rows))]
        };
    }
    [Group::EndingSoon, Group::Timed, Group::Gear]
        .into_iter()
        .map(|g| {
            let rows: Vec<&Effect> = list
                .helpful
                .iter()
                .filter(pick)
                .filter(|e| Group::of_helpful(e, soon) == g)
                .collect();
            (g, order(rows))
        })
        .filter(|(_, rows)| !rows.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(spell: u32, layer: u32, caster: u32, duration: f64) -> ActiveEnchantmentRecord {
        ActiveEnchantmentRecord {
            spell_category: spell,
            power_level: 1,
            ..ActiveEnchantmentRecord::new(spell, layer, duration, caster)
        }
    }

    fn attribute(key: u32, amount: f32) -> ActiveEnchantmentRecord {
        ActiveEnchantmentRecord {
            stat_mod_type: Some(
                EnchantmentTypeFlags::ATTRIBUTE.bits() | EnchantmentTypeFlags::ADDITIVE.bits(),
            ),
            stat_mod_key: Some(key),
            stat_mod_value: Some(amount),
            ..record(1, 1, 1, 60.0)
        }
    }

    #[test]
    fn an_additive_attribute_says_what_it_was_and_is() {
        let r = attribute(5, 45.0);
        let (kind, stat, words, delta) = change_of(&r);
        assert_eq!(kind, EffectKind::Attributes);
        assert_eq!(stat, Some(Stat::Attribute(5)));
        assert_eq!(words, "Focus");
        assert_eq!(delta, "+45");
        assert_eq!(from_to(&r, 255), Some((210, 255)));
    }

    #[test]
    fn a_harmful_amount_is_written_with_a_minus() {
        let r = attribute(1, -35.0);
        assert_eq!(change_of(&r).3, "\u{2013}35");
        assert_eq!(from_to(&r, 255), Some((290, 255)));
    }

    #[test]
    fn a_resistance_is_a_protection_saying_which_way() {
        let r = ActiveEnchantmentRecord {
            stat_mod_type: Some(
                EnchantmentTypeFlags::FLOAT.bits() | EnchantmentTypeFlags::MULTIPLICATIVE.bits(),
            ),
            stat_mod_key: Some(67),
            stat_mod_value: Some(0.5),
            ..record(2, 1, 1, 60.0)
        };
        let (kind, stat, words, delta) = change_of(&r);
        assert_eq!(kind, EffectKind::Protections);
        assert_eq!(stat, None);
        assert_eq!(words, "Fire damage taken");
        assert_eq!(delta, "lower");
    }

    #[test]
    fn a_rate_is_a_vital() {
        let r = ActiveEnchantmentRecord {
            stat_mod_type: Some(
                EnchantmentTypeFlags::FLOAT.bits() | EnchantmentTypeFlags::MULTIPLICATIVE.bits(),
            ),
            stat_mod_key: Some(3),
            stat_mod_value: Some(1.5),
            ..record(2, 1, 1, 60.0)
        };
        let (kind, _, words, delta) = change_of(&r);
        assert_eq!(kind, EffectKind::Vitals);
        assert_eq!(
            (words.as_str(), delta.as_str()),
            ("Health regained", "faster")
        );
    }

    #[test]
    fn levels_are_numerals_one_to_eight() {
        assert_eq!(roman(1), Some("I"));
        assert_eq!(roman(8), Some("VIII"));
        assert_eq!(roman(0), None);
        assert_eq!(roman(9), None);
    }

    fn effect(name: &str, left: Option<f64>, kind: EffectKind) -> Effect {
        Effect {
            identity: name.len() as u32 * 7 + left.map_or(0, |t| t as u32),
            spell_id: 1,
            name: name.to_owned(),
            school: String::new(),
            description: String::new(),
            kind,
            stat: kind.words().to_owned(),
            delta: String::new(),
            from_to: None,
            source: Source::You,
            left,
            total: left.map(|_| 5400.0),
            under: None,
            harmful: false,
        }
    }

    #[test]
    fn helpful_effects_fall_in_three_groups() {
        let list = EffectsList {
            helpful: vec![
                effect("Focus", Some(200.0), EffectKind::Attributes),
                effect("Armor", Some(2400.0), EffectKind::Protections),
                effect("Epic Strength", None, EffectKind::Attributes),
            ],
            ..EffectsList::default()
        };
        let groups = grouped(&list, false, None, "", Sort::TimeLeft, 300.0);
        let titles: Vec<_> = groups.iter().map(|(g, r)| (*g, r.len())).collect();
        assert_eq!(
            titles,
            vec![(Group::EndingSoon, 1), (Group::Timed, 1), (Group::Gear, 1)]
        );
        let attrs = grouped(
            &list,
            false,
            Some(EffectKind::Attributes),
            "",
            Sort::TimeLeft,
            300.0,
        );
        assert_eq!(attrs.len(), 2);
        let found = grouped(&list, false, None, "arm", Sort::TimeLeft, 300.0);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].1[0].name, "Armor");
    }

    #[test]
    fn the_share_left_is_of_the_whole() {
        let e = effect("Focus", Some(2700.0), EffectKind::Attributes);
        assert!((e.share_left() - 0.5).abs() < 1e-6);
        assert!((effect("Gear", None, EffectKind::Other).share_left() - 1.0).abs() < 1e-6);
    }

    #[test]
    fn sorting_steps_round() {
        assert_eq!(Sort::TimeLeft.next(), Sort::Name);
        assert_eq!(Sort::Change.next(), Sort::TimeLeft);
    }
}
