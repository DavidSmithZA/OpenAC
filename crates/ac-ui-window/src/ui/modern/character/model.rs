//! What the Character window shows, from the sheet retail's window is
//! built from ([`CharacterSheet`]): the rows (six attributes, three
//! vitals, the skills in their four groups), what each costs to raise by
//! one, ten or as much as the unassigned experience buys, how a value
//! adds up, and the raises the window sends.

use ac_ui::panels::PanelOutput;
use ac_ui::panels::character::{
    ATTRIBUTE_ROWS, CharacterSheet, CharacterSkill, RaiseAmount, RaiseRequest, RaiseTarget,
    SkillAdvancement, VITAL_ROWS, attribute_description, experience_raise, ordered_skills,
    raise_price, skill_buff_delta, train_request, vital_buff_delta, vital_description,
};

/// One row of the sheet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Row {
    /// An attribute, by its row (0 Strength .. 5 Self, retail's order).
    Attribute(usize),
    /// A vital, by its row (0 health, 1 stamina, 2 mana).
    Vital(usize),
    /// A skill, by id.
    Skill(u32),
}

/// How far to raise.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Amount {
    /// One point.
    One,
    /// Ten points.
    Ten,
    /// As many as the unassigned experience buys.
    Max,
}

/// The skill groups the list shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkillGroup {
    /// Specialized.
    Specialized,
    /// Trained.
    Trained,
    /// Untrained, usable.
    Untrained,
    /// Untrained, not usable until trained.
    Unusable,
}

impl SkillGroup {
    /// Every group, in order.
    pub const ALL: [Self; 4] = [
        Self::Specialized,
        Self::Trained,
        Self::Untrained,
        Self::Unusable,
    ];

    /// The heading.
    #[must_use]
    pub fn title(self) -> &'static str {
        match self {
            Self::Specialized => "SPECIALIZED",
            Self::Trained => "TRAINED",
            Self::Untrained => "UNTRAINED \u{b7} TRAIN WITH CREDITS",
            Self::Unusable => "UNUSABLE \u{b7} NEEDS TRAINING FIRST",
        }
    }

    /// The chip shown beside a chosen skill's name.
    #[must_use]
    pub fn chip(self) -> &'static str {
        match self {
            Self::Specialized => "SPECIALIZED",
            Self::Trained => "TRAINED",
            Self::Untrained => "UNTRAINED",
            Self::Unusable => "UNUSABLE",
        }
    }

    /// The group `skill` is in.
    #[must_use]
    pub fn of(skill: &CharacterSkill) -> Self {
        match skill.advancement {
            SkillAdvancement::Specialized => Self::Specialized,
            SkillAdvancement::Trained => Self::Trained,
            _ if skill.usable_untrained => Self::Untrained,
            _ => Self::Unusable,
        }
    }
}

/// The skill groups with their skills (by name), those matching `search`.
#[must_use]
pub fn skill_groups<'a>(
    sheet: &'a CharacterSheet,
    search: &str,
) -> Vec<(SkillGroup, Vec<&'a CharacterSkill>)> {
    let wanted = search.trim().to_lowercase();
    SkillGroup::ALL
        .into_iter()
        .map(|g| {
            let (adv, usable) = match g {
                SkillGroup::Specialized => (SkillAdvancement::Specialized, None),
                SkillGroup::Trained => (SkillAdvancement::Trained, None),
                SkillGroup::Untrained => (SkillAdvancement::Untrained, Some(true)),
                SkillGroup::Unusable => (SkillAdvancement::Untrained, Some(false)),
            };
            let skills = ordered_skills(sheet, adv, usable)
                .into_iter()
                .filter(|s| wanted.is_empty() || s.name.to_lowercase().contains(&wanted))
                .collect::<Vec<_>>();
            (g, skills)
        })
        .filter(|(_, s)| !s.is_empty())
        .collect()
}

/// A row's name.
#[must_use]
pub fn name(sheet: &CharacterSheet, row: Row) -> String {
    match row {
        Row::Attribute(i) => ATTRIBUTE_ROWS.get(i).map_or("", |r| r.0).to_owned(),
        Row::Vital(i) => VITAL_ROWS.get(i).map_or("", |r| r.0).to_owned(),
        Row::Skill(id) => sheet.skill(id).map(|s| s.name.clone()).unwrap_or_default(),
    }
}

/// A row's value now (a vital's maximum).
#[must_use]
pub fn value(sheet: &CharacterSheet, row: Row) -> i32 {
    match row {
        Row::Attribute(i) => sheet.attributes.get(i).copied().unwrap_or(0),
        Row::Vital(i) => sheet.vital_max.get(i).copied().unwrap_or(0),
        Row::Skill(id) => sheet.skill(id).map_or(0, |s| s.current_level),
    }
}

/// A row's value without spells (and without vitae).
#[must_use]
pub fn base(sheet: &CharacterSheet, row: Row) -> i32 {
    match row {
        Row::Attribute(i) => sheet.attribute_base.get(i).copied().unwrap_or(0),
        Row::Vital(i) => sheet.vital_base_max.get(i).copied().unwrap_or(0),
        Row::Skill(id) => sheet.skill(id).map_or(0, |s| s.base_level),
    }
}

/// What spells add to a row (vitae left out).
#[must_use]
pub fn spells(sheet: &CharacterSheet, row: Row) -> i32 {
    match row {
        Row::Attribute(i) => {
            sheet.attributes.get(i).copied().unwrap_or(0)
                - sheet.attribute_base.get(i).copied().unwrap_or(0)
        }
        Row::Vital(i) => vital_buff_delta(sheet, i),
        Row::Skill(id) => sheet.skill(id).map_or(0, skill_buff_delta),
    }
}

/// What vitae takes off a row (zero or negative).
#[must_use]
pub fn vitae(sheet: &CharacterSheet, row: Row) -> i32 {
    match row {
        Row::Attribute(_) => 0,
        Row::Vital(i) => sheet.vital_vitae.get(i).copied().unwrap_or(0),
        Row::Skill(id) => sheet.skill(id).map_or(0, |s| s.vitae_modifier),
    }
}

impl Row {
    /// The row as the sheet's raises know it.
    #[must_use]
    pub fn target(self) -> RaiseTarget {
        match self {
            Self::Attribute(i) => RaiseTarget::Row(i),
            Self::Vital(i) => RaiseTarget::Row(6 + i),
            Self::Skill(id) => RaiseTarget::Skill(id),
        }
    }
}

impl From<Amount> for RaiseAmount {
    fn from(a: Amount) -> Self {
        match a {
            Amount::One => Self::One,
            Amount::Ten => Self::Ten,
            Amount::Max => Self::Max,
        }
    }
}

/// It is raised with experience (not an untrained skill).
#[must_use]
pub fn raised_with_experience(sheet: &CharacterSheet, row: Row) -> bool {
    match row {
        Row::Skill(id) => sheet.skill(id).is_some_and(|s| s.advancement.is_trained()),
        _ => true,
    }
}

/// What raising `row` by `amount` costs, and by how many points (0 when
/// it cannot be raised so: at the top, or not enough for Max).
#[must_use]
pub fn cost(sheet: &CharacterSheet, row: Row, amount: Amount) -> (u32, u64) {
    raise_price(sheet, row.target(), amount.into())
}

/// Whether `row` can be raised by `amount` now.
#[must_use]
pub fn affordable(sheet: &CharacterSheet, row: Row, amount: Amount) -> bool {
    experience_raise(sheet, row.target(), amount.into()).is_some()
}

/// The raise to send for `row` by `amount`, when it can be afforded.
#[must_use]
pub fn raise(sheet: &CharacterSheet, row: Row, amount: Amount) -> Option<PanelOutput> {
    experience_raise(sheet, row.target(), amount.into()).map(RaiseRequest::output)
}

/// The credits training `skill` takes, when the player has them.
#[must_use]
pub fn train(sheet: &CharacterSheet, skill: u32) -> Option<PanelOutput> {
    train_request(sheet, skill).map(RaiseRequest::output)
}

/// How a breakdown line is coloured.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    /// Plainly.
    Plain,
    /// Spells that help.
    Good,
    /// What takes away.
    Bad,
    /// A quiet note.
    Quiet,
}

/// One line of how a value adds up: what, by how much, its tone.
pub type Line = (String, String, Tone);

/// A signed amount ("+35", "–21").
#[must_use]
pub fn signed(v: i32) -> String {
    if v < 0 {
        format!("\u{2013}{}", v.unsigned_abs())
    } else {
        format!("+{v}")
    }
}

/// How `row`'s value adds up; `spell_lines` names the spells on it and
/// what each adds (an empty list: the spells' sum shows as one line).
#[must_use]
pub fn breakdown(
    sheet: &CharacterSheet,
    row: Row,
    spell_lines: &[(String, i32)],
    vitae_share: Option<f32>,
) -> Vec<Line> {
    let mut out = Vec::new();
    let spelled = spells(sheet, row);
    match row {
        Row::Attribute(i) => {
            let innate = sheet.attribute_innate.get(i).copied().unwrap_or(0);
            let base = base(sheet, row);
            out.push(("Starting value".to_owned(), innate.to_string(), Tone::Plain));
            out.push((
                "Raised with experience".to_owned(),
                signed(base - innate),
                Tone::Plain,
            ));
        }
        Row::Vital(_) => {
            out.push((
                "From attributes and raises".to_owned(),
                base(sheet, row).to_string(),
                Tone::Plain,
            ));
        }
        Row::Skill(id) => {
            let Some(s) = sheet.skill(id) else {
                return out;
            };
            let words = match s.advancement {
                SkillAdvancement::Specialized => "From attributes, raises and specializing",
                SkillAdvancement::Trained => "From attributes and raises",
                _ => "From attributes",
            };
            out.push((words.to_owned(), s.base_level.to_string(), Tone::Plain));
        }
    }
    if spell_lines.is_empty() {
        if spelled != 0 {
            let tone = if spelled > 0 { Tone::Good } else { Tone::Bad };
            out.push(("Spells".to_owned(), signed(spelled), tone));
        }
    } else {
        for (name, by) in spell_lines {
            let tone = if *by >= 0 { Tone::Good } else { Tone::Bad };
            out.push((format!("Spells: {name}"), signed(*by), tone));
        }
    }
    if let Some(kept) = vitae_share.filter(|v| *v < 1.0) {
        let percent = crate::ui::modern::hud::effects::vitae_penalty_percent(kept);
        let lost = vitae(sheet, row);
        if matches!(row, Row::Attribute(_)) {
            out.push((
                format!("Vitae {percent}%"),
                "does not reduce attributes".to_owned(),
                Tone::Quiet,
            ));
        } else if lost < 0 {
            out.push((format!("Vitae {percent}%"), signed(lost), Tone::Bad));
        }
    }
    out
}

/// The game's words on what `row` is (a skill's formula and description).
#[must_use]
pub fn about(sheet: &CharacterSheet, row: Row) -> String {
    match row {
        Row::Attribute(i) => ATTRIBUTE_ROWS
            .get(i)
            .and_then(|r| attribute_description(r.2))
            .unwrap_or_default()
            .to_owned(),
        Row::Vital(i) => VITAL_ROWS
            .get(i)
            .and_then(|r| vital_description(r.2))
            .unwrap_or_default()
            .to_owned(),
        Row::Skill(id) => sheet
            .skill(id)
            .and_then(|s| s.tooltip.clone())
            .unwrap_or_default(),
    }
}

/// A compact cost ("118k", "4.2M"; "top" for 0).
#[must_use]
pub fn short_cost(xp: u64) -> String {
    if xp == 0 {
        return "top".to_owned();
    }
    let v = xp as f64;
    if v >= 1e9 {
        format!("{:.1}B", v / 1e9)
    } else if v >= 1e6 {
        format!("{:.1}M", v / 1e6)
    } else if v >= 1e3 {
        format!("{:.0}k", v / 1e3)
    } else {
        xp.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn skill(id: u32, adv: SkillAdvancement, usable: bool) -> CharacterSkill {
        CharacterSkill {
            id,
            name: format!("Skill {id}"),
            advancement: adv,
            base_level: 100,
            current_level: 120,
            usable_untrained: usable,
            trained_cost: 6,
            raise_cost: 500,
            raise10_cost: 6_000,
            max_raise: (4, 2_400),
            ..CharacterSkill::default()
        }
    }

    fn sheet() -> CharacterSheet {
        CharacterSheet {
            attributes: [135, 100, 100, 100, 100, 100],
            attribute_base: [100, 100, 100, 100, 100, 100],
            attribute_innate: [60, 60, 60, 60, 60, 60],
            attribute_raise_costs: [1_000; 9],
            attribute_raise10_costs: [12_000; 9],
            attribute_max_raises: [(5, 5_500); 9],
            unassigned_xp: 10_000,
            skill_credits: 5,
            skills: vec![
                skill(6, SkillAdvancement::Specialized, true),
                skill(23, SkillAdvancement::Untrained, true),
                skill(41, SkillAdvancement::Untrained, false),
            ],
            ..CharacterSheet::default()
        }
    }

    #[test]
    fn skills_fall_in_their_groups() {
        let s = sheet();
        let groups = skill_groups(&s, "");
        let kinds: Vec<_> = groups.iter().map(|(g, v)| (*g, v.len())).collect();
        assert_eq!(
            kinds,
            vec![
                (SkillGroup::Specialized, 1),
                (SkillGroup::Untrained, 1),
                (SkillGroup::Unusable, 1)
            ]
        );
        assert_eq!(skill_groups(&s, "23").len(), 1);
    }

    #[test]
    fn raises_go_out_only_when_affordable() {
        let s = sheet();
        assert!(matches!(
            raise(&s, Row::Attribute(0), Amount::One),
            Some(PanelOutput::RaiseAttribute {
                attribute: 1,
                xp: 1_000
            })
        ));
        assert!(raise(&s, Row::Attribute(0), Amount::Ten).is_none());
        assert!(matches!(
            raise(&s, Row::Vital(2), Amount::Max),
            Some(PanelOutput::RaiseVital {
                vital: 5,
                xp: 5_500
            })
        ));
        assert!(matches!(
            raise(&s, Row::Skill(6), Amount::Max),
            Some(PanelOutput::RaiseSkill {
                skill: 6,
                xp: 2_400
            })
        ));
        assert!(raise(&s, Row::Skill(23), Amount::One).is_none());
    }

    #[test]
    fn training_needs_the_credits() {
        let mut s = sheet();
        assert!(train(&s, 23).is_none());
        s.skill_credits = 6;
        assert!(matches!(
            train(&s, 23),
            Some(PanelOutput::TrainSkill {
                skill: 23,
                credits: 6
            })
        ));
        assert!(train(&s, 6).is_none());
    }

    #[test]
    fn an_attribute_adds_up_from_where_it_started() {
        let s = sheet();
        let lines = breakdown(&s, Row::Attribute(0), &[], Some(0.95));
        assert_eq!(
            lines[0],
            ("Starting value".to_owned(), "60".to_owned(), Tone::Plain)
        );
        assert_eq!(lines[1].1, "+40");
        assert_eq!(
            lines[2],
            ("Spells".to_owned(), "+35".to_owned(), Tone::Good)
        );
        assert_eq!(lines[3].2, Tone::Quiet);
    }

    #[test]
    fn costs_read_short() {
        assert_eq!(short_cost(118_000), "118k");
        assert_eq!(short_cost(4_210_880), "4.2M");
        assert_eq!(short_cost(999), "999");
        assert_eq!(short_cost(0), "top");
    }
}
