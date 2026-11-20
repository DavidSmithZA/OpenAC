//! The sheets the item screens show, built from what the client knows:
//! an item (the inventory's details and Inspect), a creature or player,
//! and a spell. Retail's appraisal text and rows are the source; the
//! sheets only arrange them.

use ac_core::items::{ClientObject, EquipMask, ItemType};
use ac_proto::appraise::IdentifyObjectResponse;

use super::sheet::{Entry, Head, Section, Sheet, Tab};

/// The inscription and its scribe (string properties).
const INSCRIPTION: u32 = 7;
const SCRIBE: u32 = 8;

/// What kind of thing `o` is, in a word or two.
pub fn kind_words(o: &ClientObject) -> &'static str {
    let t = o.item_type;
    if t.intersects(ItemType::CASTER) {
        "Caster"
    } else if t.intersects(ItemType::MELEE_WEAPON) {
        "Melee weapon"
    } else if t.intersects(ItemType::MISSILE_WEAPON) {
        "Missile weapon"
    } else if t.intersects(ItemType::ARMOR) {
        "Armor"
    } else if t.intersects(ItemType::CLOTHING) {
        "Clothing"
    } else if t.intersects(ItemType::JEWELRY) {
        "Jewelry"
    } else if t.intersects(ItemType::FOOD) {
        "Consumable"
    } else if t.intersects(ItemType::TINKERING_MATERIAL) {
        "Salvage"
    } else if t.intersects(ItemType::SPELL_COMPONENTS) {
        "Spell component"
    } else if t.intersects(ItemType::MANA_STONE) {
        "Mana stone"
    } else if t.intersects(ItemType::KEY) {
        "Key"
    } else if t.intersects(ItemType::GEM) {
        "Gem"
    } else if t.intersects(ItemType::WRITABLE) {
        "Writing"
    } else if t.intersects(ItemType::CONTAINER) {
        "Container"
    } else if t.intersects(ItemType::MONEY) {
        "Money"
    } else {
        "Item"
    }
}

/// The list's second line for `o`.
pub fn sub_line(o: &ClientObject) -> String {
    let mut parts = vec![kind_words(o).to_owned()];
    if o.value > 0 {
        parts.push(format!(
            "value {}",
            ac_ui::panels::appraisal::group_thousands(i64::from(o.value))
        ));
    }
    parts.push(format!(
        "burden {}",
        ac_ui::panels::appraisal::group_thousands(i64::from(o.burden))
    ));
    parts.join(" \u{b7} ")
}

/// The details' sheet for item `o`, from its appraisal when one came;
/// `owned`: the player carries it (its inscription may be theirs to write).
#[allow(clippy::too_many_arguments)]
pub fn item_sheet(
    o: &ClientObject,
    appraisal: Option<&IdentifyObjectResponse>,
    names: &dyn Fn(&ClientObject) -> String,
    spell: &dyn Fn(u32) -> Option<ac_core::spells::SpellMetadata>,
    tables: &ac_ui::panels::appraisal::NameTables,
    numbers: Option<&ac_core::player::LocalPlayerState>,
    me: &str,
    owned: bool,
    objects: &ac_core::items::ClientObjectTable,
    icons: &mut dyn ac_ui::icons::IconSource,
) -> Sheet {
    let worn = o.currently_equipped_location != EquipMask::NONE;
    let place = if worn {
        "worn".to_owned()
    } else {
        let pack = o.container_id;
        objects
            .get(pack)
            .filter(|p| p.object_id() != 0 && !p.name.is_empty())
            .map_or_else(
                || "in your pack".to_owned(),
                |p| {
                    if objects.get(pack).is_some_and(|c| c.items_capacity > 0)
                        && p.item_type.intersects(ItemType::CONTAINER)
                    {
                        format!("in {}", p.name)
                    } else {
                        "in Main pack".to_owned()
                    }
                },
            )
    };
    let sub = if o.stack_size > 1 {
        format!("{} \u{b7} stack of {}", kind_words(o), o.stack_size)
    } else {
        format!("{} \u{b7} {place}", kind_words(o))
    };
    let group = |v: i64| ac_ui::panels::appraisal::group_thousands(v);
    let mut facts = vec![
        ("Value".to_owned(), group(i64::from(o.value))),
        ("Burden".to_owned(), group(i64::from(o.burden))),
    ];
    if o.workmanship > 0.0 {
        facts.push((
            "Workmanship".to_owned(),
            format!("{}", o.workmanship.round() as i32),
        ));
    }
    let tabs = match appraisal {
        Some(a) => {
            let viewer = crate::ui::window::ViewerNames {
                tables,
                numbers,
                character_title: &|_| None,
            };
            let parts = ac_ui::panels::appraisal::item_report_in_parts(o, a, spell, &viewer);
            let weapon = o.item_type.intersects(ItemType::WEAPON_OR_CASTER);
            let writable = owned
                && o.public_weenie_bitfield.map_or(0, |f| f.0)
                    & ac_core::items::PublicWeenieFlags::INSCRIBABLE.0
                    != 0;
            super::sheet::item_tabs(&parts, weapon, inscription(a, me, writable))
        }
        None => vec![Tab {
            label: "Overview".to_owned(),
            sections: vec![Section {
                title: String::new(),
                count: String::new(),
                entries: vec![Entry::Para {
                    text: "Looking it over\u{2026}".to_owned(),
                    mark: super::sheet::Mark::None,
                }],
            }],
        }],
    };
    Sheet {
        subject: o.object_id(),
        head: Head {
            icon: icons.item_icon(
                o.item_type,
                o.icon_id,
                o.icon_underlay_id,
                o.icon_overlay_id,
                o.effects,
            ),
            name: names(o),
            name_color: None,
            sub,
            level: None,
            health: None,
            failed: false,
        },
        facts,
        tabs,
    }
}

/// The inscription entry from an appraisal, when the item has one or can
/// take one (`me` is the player's name; `writable`: an inscribable item
/// the player carries). The player writes on it while nobody else has
/// signed it, as in retail.
pub fn inscription(a: &IdentifyObjectResponse, me: &str, writable: bool) -> Option<Entry> {
    let text = a
        .properties
        .strings
        .get(&INSCRIPTION)
        .cloned()
        .unwrap_or_default();
    let scribe = a
        .properties
        .strings
        .get(&SCRIBE)
        .cloned()
        .unwrap_or_default();
    let inscribable = a.properties.bools.get(&0x0E).copied().unwrap_or(false)
        || a.hook_profile.as_ref().is_some_and(|h| h.flags & 1 != 0);
    if text.trim().is_empty() && !inscribable && !writable {
        return None;
    }
    let by_me = !scribe.is_empty() && scribe.to_lowercase() == me.to_lowercase();
    let editable = writable && (scribe.is_empty() || by_me);
    let note = if !scribe.is_empty() && !by_me {
        format!("Signed by {scribe}. Only its writer can change it.")
    } else if !writable {
        if scribe.is_empty() {
            "Unsigned. Carry it to write on it.".to_owned()
        } else {
            "Signed by you. Carry it to change it.".to_owned()
        }
    } else if scribe.is_empty() {
        "Yours to write on: click the box, then click away to sign it.".to_owned()
    } else {
        "Signed by you. Click the box to change it; leave it empty to clear it.".to_owned()
    };
    Some(Entry::Inscription {
        text,
        editable,
        note,
    })
}

/// What the player just wrote on an item, kept in its answer until the
/// next one comes: their words signed with their name, or none.
pub fn note_inscription(a: &mut IdentifyObjectResponse, text: &str, me: &str) {
    let strings = &mut a.properties.strings;
    if text.is_empty() {
        strings.remove(&INSCRIPTION);
        strings.remove(&SCRIBE);
    } else {
        strings.insert(INSCRIPTION, text.to_owned());
        strings.insert(SCRIBE, me.to_owned());
    }
}

/// A creature's type, a character's gender and heritage (int properties).
const CREATURE_TYPE: u32 = 2;
const GENDER: u32 = 0x71;
const HERITAGE_GROUP: u32 = 0xBC;

fn int(a: &IdentifyObjectResponse, id: u32) -> i32 {
    a.properties.ints.get(&id).copied().unwrap_or(0)
}

fn row_of(r: &ac_ui::panels::appraisal::CreatureRow) -> Entry {
    use super::sheet::Mark;
    use ac_ui::panels::appraisal::CreatureValueStyle as S;
    Entry::Row {
        key: r.label.trim_end_matches(':').to_owned(),
        value: r.value.clone(),
        mark: match r.style {
            S::Positive => Mark::Up,
            S::Negative => Mark::Down,
            S::Normal | S::Incomplete => Mark::None,
        },
    }
}

/// The sheet for a creature or a character from its appraisal. A
/// character's is split into Stats, Allegiance, Armor and About; a
/// creature's is one column.
#[allow(clippy::too_many_lines)]
pub fn creature_sheet(
    o: Option<&ClientObject>,
    a: &IdentifyObjectResponse,
    character: bool,
    names: &dyn ac_ui::panels::appraisal::AppraisalNames,
    theme: ac_ui::modern::Horizon,
) -> Sheet {
    use ac_ui::panels::appraisal as ap;
    let p = &a.properties;
    let name = o.map_or_else(
        || {
            p.strings
                .get(&1)
                .cloned()
                .unwrap_or_else(|| "Unknown".to_owned())
        },
        |o| names.appropriate_name(o),
    );
    let sub = if character {
        let gender = int(a, GENDER);
        let group = int(a, HERITAGE_GROUP);
        let fallback = if group == 0 {
            names.creature(int(a, CREATURE_TYPE))
        } else {
            String::new()
        };
        let killer = o.map_or("", |o| ap::player_killer_display(o.flags()));
        let who = ap::gender_heritage_display(gender, group, &fallback);
        if killer.is_empty() {
            who
        } else {
            format!("{who} \u{b7} {killer}")
        }
    } else {
        names.creature(int(a, CREATURE_TYPE))
    };
    let profile = a.creature_profile.as_ref();
    let health = profile
        .filter(|pr| pr.health_max > 0)
        .map(|pr| pr.health as f32 / pr.health_max as f32);
    let rows = profile
        .map(|pr| ap::creature_rows(pr, a.success))
        .unwrap_or_default();
    let mut attributes = Section {
        title: "ATTRIBUTES".to_owned(),
        ..Section::default()
    };
    let mut vitals = Section {
        title: "VITALS".to_owned(),
        ..Section::default()
    };
    let colours = [
        theme.palette.enemy,
        theme.palette.stamina,
        theme.palette.mana,
    ];
    let shares = profile.map_or([None; 3], |pr| {
        let share = |c: u32, m: u32| (m > 0).then(|| c as f32 / m as f32);
        let at = pr.attributes;
        [
            share(pr.health, pr.health_max),
            at.and_then(|at| share(at.stamina, at.stamina_max)),
            at.and_then(|at| share(at.mana, at.mana_max)),
        ]
    });
    // A failed assessment knows only the health.
    let unknown = |r: &ac_ui::panels::appraisal::CreatureRow| {
        let mut e = row_of(r);
        if let Entry::Row { value, .. } = &mut e {
            *value = "???".to_owned();
        }
        e
    };
    for (i, r) in rows.iter().enumerate() {
        if !a.success && i != 6 {
            if i < 6 {
                attributes.entries.push(unknown(r));
            } else {
                vitals.entries.push(unknown(r));
            }
            continue;
        }
        if i < 6 {
            attributes.entries.push(row_of(r));
        } else {
            let k = i - 6;
            match shares[k] {
                Some(share) if a.success => vitals.entries.push(Entry::Bar {
                    key: r.label.clone(),
                    value: r.value.replace(" (", "  (").replace(" %)", "%)"),
                    share,
                    color: colours[k],
                }),
                _ => vitals.entries.push(row_of(r)),
            }
        }
    }
    // The extra rows come in runs between blank rows.
    let faction = if character {
        names.local_faction_bits()
    } else {
        0
    };
    let extra = ap::creature_extra_rows(p, a.armor_levels.as_ref(), character, faction);
    let (mut alleg, mut armor, mut ratings, mut about) = (
        Section {
            title: "ALLEGIANCE".to_owned(),
            ..Section::default()
        },
        Section {
            title: "ARMOR LEVEL".to_owned(),
            ..Section::default()
        },
        Section {
            title: "RATINGS".to_owned(),
            ..Section::default()
        },
        Section {
            title: "ABOUT".to_owned(),
            ..Section::default()
        },
    );
    for r in &extra {
        if r.label.trim().is_empty() || r.label.starts_with("* =") {
            continue;
        }
        let e = row_of(r);
        if !a.success {
            continue;
        }
        if r.label.contains("Dmg") || r.label.starts_with("DoT") {
            ratings.entries.push(e);
        } else if r.label.contains('/') {
            armor.entries.push(e);
        } else if matches!(
            r.label.trim_end_matches(':'),
            "Society" | "Monarch" | "Patron" | "Allegiance" | "Vassals" | "Followers"
        ) || r.label.starts_with("Allegiance")
        {
            alleg.entries.push(e);
        } else {
            about.entries.push(e);
        }
    }
    let alleg_name = ap::allegiance_display(p);
    if character && !alleg_name.trim().is_empty() {
        alleg.entries.insert(
            0,
            Entry::Row {
                key: "Name".to_owned(),
                value: alleg_name,
                mark: super::sheet::Mark::None,
            },
        );
    }
    let keep = |s: Section| (!s.entries.is_empty()).then_some(s);
    let tabs = if character {
        let mut tabs = Vec::new();
        let stats: Vec<Section> = [attributes, vitals, ratings]
            .into_iter()
            .filter_map(keep)
            .collect();
        for (label, sections) in [
            ("Stats", stats),
            ("Allegiance", keep(alleg).into_iter().collect()),
            ("Armor", keep(armor).into_iter().collect()),
            ("About", keep(about).into_iter().collect()),
        ] {
            if !sections.is_empty() {
                tabs.push(Tab {
                    label: label.to_owned(),
                    sections,
                });
            }
        }
        tabs
    } else {
        vec![Tab {
            label: String::new(),
            sections: [attributes, vitals, ratings, about]
                .into_iter()
                .filter_map(keep)
                .collect(),
        }]
    };
    Sheet {
        subject: a.guid,
        head: Head {
            icon: None,
            name,
            name_color: (!character).then_some(ac_ui::modern::theme::hex(0xFF_9A6B, 1.0)),
            sub,
            level: Some(ap::level_display(p)),
            health,
            failed: !a.success,
        },
        facts: Vec::new(),
        tabs,
    }
}

/// An item sheet in one column (a portal, a chest, a door: not the
/// player's own things).
#[must_use]
pub fn one_column(mut sheet: Sheet) -> Sheet {
    let sections = sheet.tabs.drain(..).flat_map(|t| t.sections).collect();
    sheet.tabs = vec![Tab {
        label: String::new(),
        sections,
    }];
    sheet.facts.clear();
    sheet
}

/// A spell's sheet: school, mana, duration and range, what it does, and
/// its components with whether the player carries each.
#[must_use]
pub fn spell_sheet(
    spell: &ac_core::spells::SpellMetadata,
    icon: Option<ac_ui::draw::TextureRef>,
    magic_skill: u32,
    components: &[ac_ui::panels::appraisal::SpellExamineComponent],
) -> Sheet {
    use super::sheet::Mark;
    use ac_ui::panels::appraisal as ap;
    let mut casting = Section {
        title: "CASTING".to_owned(),
        ..Section::default()
    };
    for line in [
        ap::spell_mana_text(spell),
        ap::spell_duration_text(spell),
        ap::spell_range_text(spell, magic_skill),
    ] {
        if let Some((k, v)) = line.split_once(": ") {
            casting.entries.push(Entry::Row {
                key: k.to_owned(),
                value: v.trim_end_matches('.').to_owned(),
                mark: Mark::None,
            });
        }
    }
    casting.entries.push(Entry::Row {
        key: "Your skill".to_owned(),
        value: magic_skill.to_string(),
        mark: Mark::None,
    });
    let mut sections = vec![casting];
    if !spell.description.trim().is_empty() {
        sections.push(Section {
            title: "DESCRIPTION".to_owned(),
            count: String::new(),
            entries: vec![Entry::Para {
                text: spell.description.clone(),
                mark: Mark::None,
            }],
        });
    }
    if !components.is_empty() {
        sections.push(Section {
            title: "COMPONENTS".to_owned(),
            count: components.len().to_string(),
            entries: components
                .iter()
                .map(|c| Entry::Row {
                    key: c.name.clone(),
                    value: if c.owned { "carried" } else { "none" }.to_owned(),
                    mark: if c.owned { Mark::Met } else { Mark::Unmet },
                })
                .collect(),
        });
    }
    Sheet {
        subject: spell.spell_id | 0x8000_0000,
        head: Head {
            icon,
            name: spell.name.clone(),
            name_color: None,
            sub: spell.school.clone(),
            level: None,
            health: None,
            failed: false,
        },
        facts: Vec::new(),
        tabs: vec![Tab {
            label: String::new(),
            sections,
        }],
    }
}
