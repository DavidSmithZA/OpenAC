//! What the Studio shows, worked out from the session's creation state
//! and the game's creation tables and words: the step rail, each step's
//! choices, the "So far" card and the checks before finishing.

use ac_core::chargen::{
    ChargenGenderOptions, ChargenHeritageGroup as H, ChargenOptions,
    ChargenSkillAdvancementClass as Class, ChargenSkillAdvancementSet, chargen_skill_name,
    chargen_skill_score,
};
use ac_runtime::session::creation::{ChargenAttribute, CreationSnapshot, TEMPLATE_UNSET, UNSET};
use ac_ui::draw::Color;
use ac_ui::pregame::chargen::appearance::{
    Part, clothes_hidden, color_count, color_slot, shade_of, shade_slot, style_count, style_slot,
};
use ac_ui::pregame::chargen::skills::{Bucket, bucket, costs, formula_text};

/// The steps, in order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Step {
    /// Heritage and gender.
    #[default]
    Heritage,
    /// Profession and attributes.
    Profession,
    /// Skills.
    Skills,
    /// The look.
    Appearance,
    /// The starting town.
    Town,
    /// The name.
    Name,
}

impl Step {
    /// Every step.
    pub const ALL: [Self; 6] = [
        Self::Heritage,
        Self::Profession,
        Self::Skills,
        Self::Appearance,
        Self::Town,
        Self::Name,
    ];

    /// Its word on the rail.
    #[must_use]
    pub fn word(self) -> &'static str {
        match self {
            Self::Heritage => "Heritage",
            Self::Profession => "Profession",
            Self::Skills => "Skills",
            Self::Appearance => "Appearance",
            Self::Town => "Town",
            Self::Name => "Name",
        }
    }

    /// Olthoi skip it (the server sets their profession, skills and town).
    #[must_use]
    pub fn olthoi_skip(self) -> bool {
        matches!(self, Self::Profession | Self::Skills | Self::Town)
    }
}

/// The Olthoi heritages.
#[must_use]
pub fn is_olthoi(heritage: u32) -> bool {
    matches!(H(heritage), H::OLTHOI | H::OLTHOI_ACID)
}

/// The steps a heritage goes through.
#[must_use]
pub fn steps_of(heritage: u32) -> Vec<Step> {
    let olthoi = is_olthoi(heritage);
    Step::ALL
        .into_iter()
        .filter(|s| !(olthoi && s.olthoi_skip()))
        .collect()
}

/// The appearance's two groups.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Group {
    /// Hair, eyes, nose, mouth, skin.
    #[default]
    Face,
    /// Headgear, shirt, trousers, footwear.
    Clothes,
}

/// A heritage's tile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeritageTile {
    /// Its id.
    pub id: u32,
    /// Its name.
    pub name: String,
    /// Its masteries ("Dagger, Bow"), or "Always PK".
    pub sub: String,
    /// Always a player killer.
    pub pk: bool,
}

/// An attribute's row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttributeRow {
    /// Which.
    pub attribute: ChargenAttribute,
    /// Its name.
    pub name: &'static str,
    /// Its value (10 to 100).
    pub value: i32,
    /// Locked against balancing.
    pub locked: bool,
}

/// A skill's row, or a group's heading.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SkillRow {
    /// A group: its heading and the count beside it.
    Head(&'static str, String),
    /// A skill.
    Skill(SkillLine),
}

/// One skill.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillLine {
    /// Its id.
    pub id: u32,
    /// Its name.
    pub name: String,
    /// Its starting score (none when it cannot be used untrained).
    pub score: Option<u32>,
    /// Its group.
    pub bucket: Bucket,
    /// What raising it costs and whether it can be paid.
    pub raise: Option<(i32, bool)>,
    /// What lowering it gives back.
    pub lower: Option<i32>,
}

/// The picked skill's details.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SkillInfo {
    /// Its name.
    pub name: String,
    /// "Specialized", "Trained", "Untrained", "Cannot use untrained".
    pub state: &'static str,
    /// Its score.
    pub score: String,
    /// The game's description.
    pub text: String,
    /// "(Focus + Self) / 4".
    pub formula: String,
    /// "Train 6, specialize 6 more".
    pub cost: String,
}

/// A part's tab on the appearance step.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartTab {
    /// The part.
    pub part: Part,
    /// Its word.
    pub label: String,
    /// "Style 4", "None", "Shade 40%".
    pub sub: String,
}

/// The picked part's choices.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PartChoices {
    /// The style picked (from 1; 0: none) and how many there are.
    pub style: Option<(usize, usize)>,
    /// The part may have no style at all (headgear).
    pub may_be_none: bool,
    /// How many colours, and the one picked.
    pub colors: Option<(usize, Option<usize>)>,
    /// The shade, 0 to 1.
    pub shade: Option<f32>,
    /// A note under the choices.
    pub note: Option<&'static str>,
}

/// A starting town's card.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TownCard {
    /// Its index.
    pub index: i32,
    /// Its name.
    pub name: String,
    /// Whose town it is ("Aluvian town").
    pub home: &'static str,
    /// The first lines of the game's words about it.
    pub text: String,
}

/// A check before finishing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Check {
    /// It is fine.
    pub ok: bool,
    /// Its words.
    pub text: String,
    /// The step that fixes it.
    pub fix: Option<Step>,
}

/// A line of the "So far" card.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fact {
    /// Its label.
    pub label: &'static str,
    /// Its value.
    pub value: String,
    /// The step it jumps to (none: not for this heritage).
    pub go: Option<Step>,
}

/// The Studio's whole picture.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct StudioScreen {
    /// The step shown.
    pub step: Step,
    /// The heritage's steps.
    pub steps: Vec<Step>,
    /// Olthoi.
    pub olthoi: bool,
    /// "Leafcull · slot 6 of 11".
    pub subtitle: String,
    /// The heritages.
    pub heritages: Vec<HeritageTile>,
    /// The heritage picked.
    pub heritage: u32,
    /// The genders: key and name.
    pub genders: Vec<(u32, String)>,
    /// The gender picked.
    pub gender: u32,
    /// "Female Aluvian".
    pub who: String,
    /// The heritage's words: its masteries and what it starts with.
    pub heritage_text: Vec<String>,
    /// What every heritage is trained in from the start.
    pub starting_skills: String,
    /// OpenAC's note on how the heritage looks.
    pub heritage_note: Option<&'static str>,
    /// The professions: index and name.
    pub templates: Vec<(u32, String)>,
    /// The profession picked.
    pub template: u32,
    /// The profession's words.
    pub template_text: String,
    /// The attributes.
    pub attributes: Vec<AttributeRow>,
    /// Points not spent, and the heritage's total.
    pub points: (i32, u32),
    /// Health, stamina, mana.
    pub vitals: [i32; 3],
    /// The skills, grouped.
    pub skills: Vec<SkillRow>,
    /// Credits not spent, and the heritage's total.
    pub credits: (i32, u32),
    /// Credits spent on specialized skills.
    pub specialized: i32,
    /// The picked skill's details.
    pub skill_info: SkillInfo,
    /// The appearance group shown.
    pub group: Group,
    /// The heritage wears clothes.
    pub has_clothes: bool,
    /// The group's parts.
    pub parts: Vec<PartTab>,
    /// The part picked.
    pub part: Option<Part>,
    /// Its choices.
    pub choices: PartChoices,
    /// The starting towns.
    pub towns: Vec<TownCard>,
    /// The town picked.
    pub town: i32,
    /// The name.
    pub name: String,
    /// Name examples from the game's words for the heritage.
    pub suggestions: Vec<String>,
    /// The checks before finishing.
    pub checks: Vec<Check>,
    /// The "So far" card's lines.
    pub facts: Vec<Fact>,
    /// The specialized and trained skills: name, specialized, score.
    pub chosen_skills: Vec<(String, bool, u32)>,
}

/// The game's words the Studio reads, by key.
pub type Words<'a> = &'a dyn Fn(&str) -> Option<String>;

/// The six attributes in the order shown.
pub const ATTRIBUTES: [(ChargenAttribute, &str); 6] = [
    (ChargenAttribute::Strength, "Strength"),
    (ChargenAttribute::Endurance, "Endurance"),
    (ChargenAttribute::Coordination, "Coordination"),
    (ChargenAttribute::Quickness, "Quickness"),
    (ChargenAttribute::Focus, "Focus"),
    (ChargenAttribute::Self_, "Self"),
];

/// The most credits a character may spend on specialized skills.
pub const SPECIALIZED_LIMIT: i32 = 70;

/// The longest name.
pub const NAME_LIMIT: usize = 32;

/// How each heritage looks, in OpenAC's words (the game says nothing of
/// it at creation).
fn heritage_note(heritage: u32) -> Option<&'static str> {
    Some(match H(heritage) {
        H::SHADOWBOUND => "A crown of smoke; they float, with smoke below the waist.",
        H::PENUMBRAEN => "A crown of smoke, which a barber can turn off.",
        H::GEARKNIGHT => {
            "No face or clothes: the look you pick is the whole body. Armour does not show."
        }
        H::UNDEAD => "A flaming skull, which a barber can turn off.",
        H::EMPYREAN => "They float and their eyes glow.",
        H::TUMEROK => "Armour on the lower legs and feet does not show.",
        H::LUGIAN => "Armour on the feet and many helmets do not show.",
        H::OLTHOI | H::OLTHOI_ACID => "Gains no experience. Always a player killer, on any server.",
        _ => return None,
    })
}

/// A heritage's name as players know it: the creation table's own,
/// except the four it names by an inner word ("Gear", "OlthoiAcid").
#[must_use]
pub fn heritage_name(id: u32, table: &str) -> String {
    match H(id) {
        H::GEARKNIGHT => "Gear Knight",
        H::TUMEROK => "Aun Tumerok",
        H::OLTHOI => "Olthoi Soldier",
        H::OLTHOI_ACID => "Olthoi Spitter",
        _ => table,
    }
    .to_owned()
}

/// The masteries a heritage's bonus words name ("You gain mastery in
/// Dagger and Bow which ..." gives "Dagger, Bow").
#[must_use]
pub fn masteries(bonus: &str) -> Option<String> {
    let start = bonus.find("mastery in ")? + "mastery in ".len();
    let rest = &bonus[start..];
    let end = rest.find(" which").unwrap_or(rest.len());
    let list = rest[..end]
        .replace(" weaponry", "")
        .replace(" spells", "")
        .replace(" Weapons", "")
        .replace(" weapons", "");
    Some(
        list.split(" and ")
            .map(str::trim)
            .filter(|w| !w.is_empty())
            .collect::<Vec<_>>()
            .join(", "),
    )
}

/// The names a heritage's naming words give as examples ("Examples:
/// Lissith, Ulewinn, Sothlal.").
#[must_use]
pub fn name_examples(words: &str) -> Vec<String> {
    let mut names = Vec::new();
    for part in words.split("Examples:").skip(1) {
        let end = part
            .find(".\n")
            .or_else(|| part.find('\n'))
            .unwrap_or(part.len());
        for name in part[..end].split(',') {
            let n = name.trim().trim_end_matches('.').trim();
            if !n.is_empty() && n.len() <= NAME_LIMIT {
                names.push(n.to_owned());
            }
        }
    }
    names
}

/// Paragraphs of the game's words, trimmed, empty ones left out.
fn paragraphs(words: &str) -> Vec<String> {
    words
        .split("\n\n")
        .map(|p| p.split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|p| !p.is_empty())
        .collect()
}

/// The first `n` sentences of `words`.
fn sentences(words: &str, n: usize) -> String {
    let flat = words.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut out = String::new();
    let mut count = 0;
    for piece in flat.split_inclusive(". ") {
        out.push_str(piece);
        count += 1;
        if count >= n {
            break;
        }
    }
    out.trim().to_owned()
}

fn gender_options<'a>(
    options: &'a ChargenOptions,
    s: &CreationSnapshot,
) -> Option<&'a ChargenGenderOptions> {
    options
        .try_get_heritage(s.heritage_id)?
        .genders_by_key
        .get(&i32::try_from(s.gender_key).ok()?)
}

fn level_of(s: &CreationSnapshot, skill: u32) -> Class {
    s.skills
        .get(skill as usize)
        .copied()
        .unwrap_or(Class::Inactive)
}

/// The part words a heritage without nose and mouth uses, else the usual.
fn part_word(part: Part, captions: Option<&[String; 3]>) -> String {
    match (captions, part) {
        (Some(c), Part::Hair) => c[0].clone(),
        (Some(c), Part::Eyes) => c[1].clone(),
        (Some(c), Part::Skin) => c[2].clone(),
        _ => match part {
            Part::Hair => "Hair",
            Part::Eyes => "Eyes",
            Part::Nose => "Nose",
            Part::Mouth => "Mouth",
            Part::Skin => "Skin",
            Part::Headgear => "Headgear",
            Part::Shirt => "Shirt",
            Part::Trousers => "Trousers",
            Part::Footwear => "Footwear",
        }
        .to_owned(),
    }
}

/// A heritage without nose and mouth (and whose eyes have no styles).
fn three_parts(heritage: u32) -> bool {
    matches!(H(heritage), H::GEARKNIGHT | H::OLTHOI | H::OLTHOI_ACID)
}

/// The parts of `group` for `heritage`.
#[must_use]
pub fn parts_of(heritage: u32, group: Group) -> Vec<Part> {
    match group {
        Group::Face if three_parts(heritage) => vec![Part::Hair, Part::Eyes, Part::Skin],
        Group::Face => vec![Part::Hair, Part::Eyes, Part::Nose, Part::Mouth, Part::Skin],
        Group::Clothes if clothes_hidden(heritage) => Vec::new(),
        Group::Clothes => vec![Part::Headgear, Part::Shirt, Part::Trousers, Part::Footwear],
    }
}

fn has_styles(part: Part, heritage: u32) -> bool {
    style_slot(part).is_some() && !(three_parts(heritage) && part == Part::Eyes)
}

/// What the Studio is told besides the snapshot.
#[derive(Debug, Clone, Copy)]
pub struct Asked<'a> {
    /// The step shown.
    pub step: Step,
    /// The skill picked on the skills step.
    pub skill: Option<u32>,
    /// The appearance group.
    pub group: Group,
    /// The part picked.
    pub part: Part,
    /// The world's name.
    pub world: &'a str,
    /// The slot it goes in and how many there are.
    pub slots: (usize, usize),
    /// The part words of a heritage without nose and mouth.
    pub captions: Option<&'a [String; 3]>,
}

/// The Studio for `s`.
#[must_use]
#[allow(clippy::too_many_lines)]
pub fn studio_screen(
    s: &CreationSnapshot,
    options: &ChargenOptions,
    words: Words<'_>,
    asked: &Asked<'_>,
) -> StudioScreen {
    let heritage = options.try_get_heritage(s.heritage_id);
    let olthoi = is_olthoi(s.heritage_id);
    let steps = steps_of(s.heritage_id);
    let step = if steps.contains(&asked.step) {
        asked.step
    } else {
        Step::Heritage
    };
    let mut ids: Vec<u32> = options.heritages_by_id.keys().copied().collect();
    ids.sort_unstable();
    let bonus_of = |id: u32| {
        ac_ui::pregame::chargen::heritage::BONUS_SKILLS_KEY
            .iter()
            .find(|(h, _)| *h == id)
            .and_then(|(_, k)| words(k))
    };
    let heritages = ids
        .iter()
        .filter_map(|&id| {
            let h = options.try_get_heritage(id)?;
            let pk = is_olthoi(id);
            Some(HeritageTile {
                id,
                name: heritage_name(id, &h.name),
                sub: if pk {
                    "Always PK".to_owned()
                } else {
                    bonus_of(id).and_then(|b| masteries(&b)).unwrap_or_default()
                },
                pk,
            })
        })
        .collect();
    let mut genders: Vec<(u32, String)> = heritage
        .map(|h| {
            h.genders_by_key
                .iter()
                .filter_map(|(k, g)| Some((u32::try_from(*k).ok()?, g.name.clone())))
                .collect()
        })
        .unwrap_or_default();
    // Female first, as the boards draw it.
    genders.sort_by_key(|g| std::cmp::Reverse(g.0));
    let gender_name = genders
        .iter()
        .find(|(k, _)| *k == s.gender_key)
        .map(|(_, n)| n.clone())
        .unwrap_or_default();
    let heritage_name = heritage
        .map(|h| heritage_name(s.heritage_id, &h.name))
        .unwrap_or_default();
    let who = [gender_name.as_str(), heritage_name.as_str()]
        .into_iter()
        .filter(|w| !w.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    let heritage_text = bonus_of(s.heritage_id)
        .map(|b| paragraphs(&b))
        .unwrap_or_default();
    let starting_skills = words("ID_CharGen_Heritage_StartingSkills")
        .map(|w| w.split_whitespace().collect::<Vec<_>>().join(" "))
        .unwrap_or_default();
    let templates: Vec<(u32, String)> = heritage
        .map(|h| {
            h.templates
                .iter()
                .enumerate()
                .map(|(i, t)| (u32::try_from(i).unwrap_or(0), t.name.clone()))
                .collect()
        })
        .unwrap_or_default();
    let template_text = ac_ui::pregame::chargen::profession::DESCRIPTION_KEY
        .iter()
        .find(|(t, _)| *t == s.template)
        .and_then(|(_, k)| words(k))
        .map(|w| paragraphs(&w).into_iter().next().unwrap_or_default())
        .unwrap_or_default();
    let a = &s.attributes;
    let value = |attr: ChargenAttribute| match attr {
        ChargenAttribute::Strength => a.strength,
        ChargenAttribute::Endurance => a.endurance,
        ChargenAttribute::Coordination => a.coordination,
        ChargenAttribute::Quickness => a.quickness,
        ChargenAttribute::Focus => a.focus,
        ChargenAttribute::Self_ => a.self_,
    };
    let attributes = ATTRIBUTES
        .iter()
        .map(|&(attribute, name)| AttributeRow {
            attribute,
            name,
            value: value(attribute),
            locked: ac_ui::pregame::chargen::profession::is_locked(
                s.attribute_lock_mask,
                attribute,
            ),
        })
        .collect();
    // Skills: every skill the heritage can pay for, by group, by name.
    let credits_left = s.remaining_skill_credits;
    let mut lines: Vec<SkillLine> = Vec::new();
    let mut specialized = 0;
    if let Some(h) = heritage {
        for skill in 1..ChargenSkillAdvancementSet::SLOT_COUNT as u32 {
            if !(h.skill_costs_by_skill_id.contains_key(&skill)
                || options.global_skill_costs_by_skill_id.contains_key(&skill))
            {
                continue;
            }
            let level = level_of(s, skill);
            let min = options
                .try_get_skill_detail(skill)
                .map_or(1, |d| d.min_level);
            let b = bucket(level, min);
            let (train, spec) = costs(options, s.heritage_id, skill);
            if level == Class::Specialized {
                specialized += spec;
            }
            let score = (b != Bucket::UnuseableUntrained)
                .then(|| chargen_skill_score(options, skill, a, level));
            lines.push(SkillLine {
                id: skill,
                name: chargen_skill_name(options, skill),
                score,
                bucket: b,
                raise: None,
                lower: match level {
                    Class::Specialized if spec != 0 => Some(spec - train),
                    Class::Trained if train != 0 => Some(train),
                    _ => None,
                },
            });
            let _ = train;
        }
    }
    for line in &mut lines {
        let level = level_of(s, line.id);
        let (train, spec) = costs(options, s.heritage_id, line.id);
        line.raise = match level {
            Class::Specialized => None,
            Class::Trained if spec - train < 999 => Some((
                spec - train,
                credits_left >= spec - train && specialized + spec <= SPECIALIZED_LIMIT,
            )),
            Class::Trained => None,
            _ if train < 999 => Some((train, credits_left >= train)),
            _ => None,
        };
    }
    lines.sort_by(|x, y| x.name.cmp(&y.name));
    let mut skills = Vec::new();
    for (b, head) in [
        (Bucket::Specialized, "SPECIALIZED"),
        (Bucket::Trained, "TRAINED"),
        (Bucket::UseableUntrained, "USABLE UNTRAINED"),
        (Bucket::UnuseableUntrained, "UNUSABLE UNTRAINED"),
    ] {
        let group: Vec<&SkillLine> = lines.iter().filter(|l| l.bucket == b).collect();
        if group.is_empty() {
            continue;
        }
        let count = if b == Bucket::Specialized {
            format!("{specialized} of {SPECIALIZED_LIMIT} credits")
        } else {
            group.len().to_string()
        };
        skills.push(SkillRow::Head(head, count));
        skills.extend(group.into_iter().cloned().map(SkillRow::Skill));
    }
    let picked_skill = asked
        .skill
        .filter(|id| lines.iter().any(|l| l.id == *id))
        .or_else(|| {
            lines
                .iter()
                .find(|l| l.bucket == Bucket::Specialized)
                .map(|l| l.id)
        })
        .or_else(|| lines.first().map(|l| l.id));
    let skill_info = picked_skill.map_or_else(SkillInfo::default, |id| {
        let level = level_of(s, id);
        let line = lines.iter().find(|l| l.id == id);
        let detail = options.try_get_skill_detail(id);
        let (train, spec) = costs(options, s.heritage_id, id);
        SkillInfo {
            name: chargen_skill_name(options, id),
            state: match level {
                Class::Specialized => "Specialized",
                Class::Trained => "Trained",
                _ if line.is_some_and(|l| l.bucket == Bucket::UseableUntrained) => "Untrained",
                _ => "Cannot use untrained",
            },
            score: line
                .and_then(|l| l.score)
                .map_or_else(|| "—".to_owned(), |v| v.to_string()),
            text: detail.map(|d| d.description.clone()).unwrap_or_default(),
            formula: detail
                .map(|d| {
                    formula_text(&d.formula)
                        .trim_start_matches("Formula : ")
                        .to_owned()
                })
                .unwrap_or_default(),
            cost: match (train, spec - train) {
                (t, _) if t >= 999 => "Cannot be trained".to_owned(),
                (0, more) if more < 999 => format!("Trained free, specialize {more} more"),
                (0, _) => "Trained free".to_owned(),
                (t, more) if more < 999 => format!("Train {t}, specialize {more} more"),
                (t, _) => format!("Train {t}"),
            },
        }
    });
    // The look.
    let g = gender_options(options, s);
    let has_clothes = !clothes_hidden(s.heritage_id);
    let group = if has_clothes {
        asked.group
    } else {
        Group::Face
    };
    let ap = &s.appearance;
    let part_list = parts_of(s.heritage_id, group);
    let part = part_list
        .iter()
        .copied()
        .find(|p| *p == asked.part)
        .or_else(|| part_list.first().copied());
    let parts = part_list
        .iter()
        .map(|&p| {
            let sub = if p == Part::Skin {
                format!("Shade {}%", (ap.skin_shade * 100.0).round())
            } else if has_styles(p, s.heritage_id) {
                let v = style_slot(p).map_or(UNSET, |slot| ap.get(slot));
                if v == UNSET {
                    "None".to_owned()
                } else {
                    format!("Style {}", v + 1)
                }
            } else {
                "Colour".to_owned()
            };
            PartTab {
                part: p,
                label: part_word(p, asked.captions),
                sub,
            }
        })
        .collect();
    let choices = match (part, g) {
        (Some(p), Some(g)) => {
            let style = has_styles(p, s.heritage_id)
                .then(|| {
                    let count = usize::try_from(style_count(p, g)).unwrap_or(0);
                    let v = style_slot(p).map_or(UNSET, |slot| ap.get(slot));
                    let now = if v == UNSET { 0 } else { v as usize + 1 };
                    (now, count)
                })
                .filter(|(_, count)| *count > 0);
            let colors = color_slot(p).map(|slot| {
                let v = ap.get(slot);
                (color_count(p, g).min(9), (v != UNSET).then_some(v as usize))
            });
            let shade = (p != Part::Eyes)
                .then(|| shade_slot(p))
                .flatten()
                .map(|slot| shade_of(slot, ap).clamp(0.0, 1.0) as f32);
            PartChoices {
                style,
                may_be_none: p == Part::Headgear,
                colors: colors.filter(|(n, _)| *n > 0),
                shade,
                note: match p {
                    Part::Nose | Part::Mouth => {
                        Some("This part has a style only; its colour follows your skin.")
                    }
                    Part::Skin if !three_parts(s.heritage_id) => {
                        Some("Your nose and mouth take this shade too.")
                    }
                    _ => None,
                },
            }
        }
        _ => PartChoices::default(),
    };
    // The towns.
    let homes = [
        "Aluvian town",
        "Sho town",
        "Gharu'ndim town",
        "Viamontian town",
    ];
    let towns = options
        .starter_areas
        .iter()
        .take(4)
        .map(|area| TownCard {
            index: area.index,
            name: area.name.clone(),
            home: usize::try_from(area.index)
                .ok()
                .and_then(|i| homes.get(i))
                .copied()
                .unwrap_or(""),
            text: ac_ui::pregame::chargen::town::TOWN_TEXT_KEY
                .iter()
                .find(|(i, _)| *i == area.index)
                .and_then(|(_, k)| words(k))
                .map(|w| sentences(&w, 2))
                .unwrap_or_default(),
        })
        .collect::<Vec<_>>();
    let town_name = options
        .try_get_starter_area(s.start_area)
        .map(|a| a.name.clone());
    // The name.
    let suggestions = ac_ui::pregame::chargen::summary::NAME_SUGGESTIONS
        .iter()
        .find(|(h, _, _)| *h == s.heritage_id)
        .and_then(|&(_, male, female)| words(if s.gender_key == 2 { female } else { male }))
        .map(|w| name_examples(&w))
        .unwrap_or_default();
    let name = s.name.trim().to_owned();
    let points_left = s.remaining_attribute_credits;
    let checks = vec![
        Check {
            ok: s.heritage_id != 0 && s.gender_key != 0,
            text: "Heritage and look chosen".to_owned(),
            fix: Some(Step::Heritage),
        },
        Check {
            ok: olthoi || points_left == 0,
            text: if olthoi {
                "Attributes set by the server".to_owned()
            } else if points_left == 0 {
                format!("All {} attribute points spent", s.total_attribute_credits)
            } else {
                format!("{points_left} attribute points not spent")
            },
            fix: Some(Step::Profession),
        },
        Check {
            ok: true,
            text: if olthoi {
                "No skills to pick".to_owned()
            } else if credits_left > 0 {
                format!("{credits_left} skill credits left: they carry into the game")
            } else {
                "All skill credits spent".to_owned()
            },
            fix: None,
        },
        Check {
            ok: !name.is_empty(),
            text: if name.is_empty() {
                "No name yet".to_owned()
            } else {
                "Named".to_owned()
            },
            fix: None,
        },
    ];
    let template_name = if olthoi {
        None
    } else if s.template == TEMPLATE_UNSET {
        Some("None yet".to_owned())
    } else {
        templates
            .iter()
            .find(|(i, _)| *i == s.template)
            .map(|(_, n)| n.clone())
    };
    let facts = vec![
        Fact {
            label: "Heritage",
            value: who.clone(),
            go: Some(Step::Heritage),
        },
        Fact {
            label: "Profession",
            value: template_name.unwrap_or_else(|| "Not for Olthoi".to_owned()),
            go: (!olthoi).then_some(Step::Profession),
        },
        Fact {
            label: "Town",
            value: if olthoi {
                "Not for Olthoi".to_owned()
            } else {
                town_name.unwrap_or_else(|| "None yet".to_owned())
            },
            go: (!olthoi).then_some(Step::Town),
        },
        Fact {
            label: "Name",
            value: if name.is_empty() {
                "Not named yet".to_owned()
            } else {
                name.clone()
            },
            go: Some(Step::Name),
        },
    ];
    let mut chosen_skills: Vec<(String, bool, u32)> = lines
        .iter()
        .filter(|l| matches!(l.bucket, Bucket::Specialized | Bucket::Trained))
        .map(|l| {
            (
                l.name.clone(),
                l.bucket == Bucket::Specialized,
                l.score.unwrap_or(0),
            )
        })
        .collect();
    chosen_skills.sort_by_key(|(_, spec, _)| !*spec);
    StudioScreen {
        step,
        steps,
        olthoi,
        subtitle: format!(
            "{} · slot {} of {}",
            asked.world, asked.slots.0, asked.slots.1
        ),
        heritages,
        heritage: s.heritage_id,
        genders,
        gender: s.gender_key,
        who,
        heritage_text,
        starting_skills,
        heritage_note: heritage_note(s.heritage_id),
        templates,
        template: s.template,
        template_text,
        attributes,
        points: (points_left, s.total_attribute_credits),
        vitals: [a.endurance / 2, a.endurance, a.self_],
        skills,
        credits: (credits_left, s.total_skill_credits),
        specialized,
        skill_info,
        group,
        has_clothes,
        parts,
        part,
        choices,
        towns,
        town: s.start_area,
        name: s.name.clone(),
        suggestions,
        checks,
        facts,
        chosen_skills,
    }
}

/// Three names near one the server refused: the last two letters
/// swapped, a letter doubled or added, "of" between two words.
#[must_use]
pub fn near_names(name: &str) -> Vec<String> {
    let name = name.trim();
    let mut out: Vec<String> = Vec::new();
    let mut push = |n: String| {
        let ok = !n.is_empty()
            && n.chars().count() <= NAME_LIMIT
            && n.chars().all(ac_ui::pregame::chargen::summary::name_filter)
            && !n.eq_ignore_ascii_case(name)
            && !out.iter().any(|o| o.eq_ignore_ascii_case(&n));
        if ok {
            out.push(n);
        }
    };
    let words: Vec<&str> = name.split_whitespace().collect();
    let last = words.last().copied().unwrap_or("");
    let head = &words[..words.len().saturating_sub(1)];
    let with_last = |w: String| {
        let mut all: Vec<String> = head.iter().map(|s| (*s).to_owned()).collect();
        all.push(w);
        all.join(" ")
    };
    let chars: Vec<char> = last.chars().collect();
    if chars.len() >= 3 {
        let mut c = chars.clone();
        let n = c.len();
        c.swap(n - 1, n - 2);
        push(with_last(c.into_iter().collect()));
    }
    if words.len() >= 2 {
        push(format!(
            "{} of {}",
            words[..words.len() - 1].join(" "),
            last
        ));
    }
    let first: Vec<char> = words.first().copied().unwrap_or("").chars().collect();
    if let Some(&end) = first.last() {
        let more = if "aeiouy".contains(end.to_ascii_lowercase()) {
            format!("{}{}", first.iter().collect::<String>(), "n")
        } else {
            format!("{}{}", first.iter().collect::<String>(), "e")
        };
        let mut all = vec![more];
        all.extend(words.iter().skip(1).map(|w| (*w).to_owned()));
        push(all.join(" "));
    }
    if chars.len() >= 2 {
        let mut doubled: String = chars[..chars.len() - 1].iter().collect();
        doubled.push(chars[chars.len() - 2]);
        doubled.push(chars[chars.len() - 1]);
        push(with_last(doubled));
    }
    for end in ["e", "a", "on", "wyn"] {
        push(format!("{name}{end}"));
    }
    out.truncate(3);
    out
}

/// A colour from the dats' bytes.
#[must_use]
pub fn rgb(c: ac_core::chargen::ChargenSwatchRgb) -> Color {
    crate::ui::modern::barber::model::rgb(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_masteries_come_from_the_games_bonus_words() {
        assert_eq!(
            masteries("You gain mastery in Dagger and Bow which gives you a damage rating bonus"),
            Some("Dagger, Bow".to_owned())
        );
        assert_eq!(
            masteries("You gain mastery in Staff and Magic spells which gives you"),
            Some("Staff, Magic".to_owned())
        );
        assert_eq!(
            masteries("You gain mastery in Axe and Thrown Weapons which gives you"),
            Some("Axe, Thrown".to_owned())
        );
        assert_eq!(masteries("Nothing here."), None);
    }

    #[test]
    fn the_name_examples_come_from_the_games_naming_words() {
        let words = "Many Aluvian women have only a first name, with no surname. Examples: Lissith, Ulewinn, Sothlal.\n\nOthers use a first name, plus 'the' and a word. Examples: Domlan the True, Arryn the Cat, Ethew the Fair.\n\n";
        assert_eq!(
            name_examples(words),
            vec![
                "Lissith",
                "Ulewinn",
                "Sothlal",
                "Domlan the True",
                "Arryn the Cat",
                "Ethew the Fair"
            ]
        );
    }

    #[test]
    fn a_refused_name_offers_three_near_ones() {
        let near = near_names("Aerin Vale");
        assert_eq!(near.len(), 3);
        assert!(near.iter().all(|n| n != "Aerin Vale"));
        assert_eq!(near[0], "Aerin Vael");
        assert_eq!(near[1], "Aerin of Vale");
        let near = near_names("Quill");
        assert_eq!(near.len(), 3);
        assert!(
            near.iter()
                .all(|n| n.chars().all(ac_ui::pregame::chargen::summary::name_filter))
        );
    }

    #[test]
    fn olthoi_skip_profession_skills_and_town() {
        assert_eq!(
            steps_of(H::OLTHOI.0),
            vec![Step::Heritage, Step::Appearance, Step::Name]
        );
        assert_eq!(steps_of(H::ALUVIAN.0).len(), 6);
    }
}
