//! What the Salvage window shows, made from the staging, the item table
//! and the player's skill: the items to salvage with their material and
//! workmanship, the bags they make (one per material, with how many items
//! go in and their average workmanship; no amounts in advance), the player's
//! packs to add from with material chips, why a drop was refused, and what
//! a salvage produced.

use ac_core::items::{
    ClientObject, ClientObjectTable, PublicWeenieFlags, SalvageOutcome, is_salvage_suitable,
    salvage_material_name,
};
use ac_runtime::inventory::shop::group_thousands;
use ac_ui::draw::{Color, TextureRef};
use ac_ui::modern::theme::hex;
use ac_ui::modern::tone;
use ac_ui::panels::salvage_list::{SalvageRefusal, item_refusal};

use super::vendor_model::carried;

/// The salvaging skill's id.
pub const SALVAGING_SKILL: u32 = 40;

/// A material's colour (its family's: metals, stones, woods, cloths,
/// hides, gems).
#[must_use]
pub fn material_color(material: u32) -> Color {
    let name = salvage_material_name(material);
    match name {
        "Steel" | "Iron" | "Metal" | "Pyreal" => hex(0xB8_C2CC, 1.0),
        "Silver" | "White Quartz" | "White Sapphire" => hex(0xD8_DEE6, 1.0),
        "Gold" | "Brass" | "Bronze" | "Copper" => tone::gold(1.0),
        "Granite" | "Stone" | "Alabaster" | "Marble" | "Obsidian" | "Sandstone" | "Serpentine"
        | "Ceramic" | "Porcelain" => hex(0x9A_968C, 1.0),
        "Wood" | "Ebony" | "Mahogany" | "Oak" | "Pine" | "Teak" => hex(0xB0_8050, 1.0),
        "Cloth" | "Linen" | "Satin" | "Silk" | "Velvet" | "Wool" | "Ivory" => tone::warm(1.0),
        "Leather" | "Armoredillo Hide" | "Gromnie Hide" | "Reed Shark Hide" => hex(0xC0_8A5A, 1.0),
        "Unknown" => hex(0xB3_AA9C, 1.0),
        _ => hex(0xC9_A2FF, 1.0),
    }
}

/// One item to salvage.
#[derive(Debug, Clone, PartialEq)]
pub struct StagedRow {
    /// The item.
    pub id: u32,
    /// Its icon.
    pub icon: Option<TextureRef>,
    /// Its name.
    pub name: String,
    /// Where it is and what it is worth.
    pub sub: String,
    /// Its material, and the material's colour.
    pub material: String,
    pub color: Color,
    /// Its workmanship.
    pub workmanship: String,
}

/// A bag a salvage would make.
#[derive(Debug, Clone, PartialEq)]
pub struct BagRow {
    /// "Salvaged Steel".
    pub name: String,
    /// "2 items · workmanship 5.5".
    pub sub: String,
    /// The material's colour.
    pub color: Color,
}

/// One of the player's items in Add from packs.
#[derive(Debug, Clone, PartialEq)]
pub struct SalvagePick {
    /// The item.
    pub id: u32,
    /// Its icon.
    pub icon: Option<TextureRef>,
    /// Its name.
    pub name: String,
    /// Its material, workmanship and place, or why it cannot be salvaged.
    pub sub: String,
    /// It cannot be added (refused, or added already).
    pub off: bool,
    /// Added already.
    pub added: bool,
    /// It is refused (the line is the reason).
    pub refused: bool,
    /// Its material.
    pub material: u32,
}

/// A material chip in Add from packs.
#[derive(Debug, Clone, PartialEq)]
pub struct MaterialChip {
    /// The material (0: all).
    pub material: u32,
    /// Its name.
    pub name: String,
    /// How many items of it.
    pub count: usize,
    /// Its colour.
    pub color: Color,
}

/// A bag a salvage produced.
#[derive(Debug, Clone, PartialEq)]
pub struct ResultCard {
    /// "Salvaged Steel".
    pub name: String,
    /// "from 2 items".
    pub from: String,
    /// Its units.
    pub units: String,
    /// "workmanship 5.50".
    pub workmanship: String,
    /// The material's colour.
    pub color: Color,
}

/// What a salvage produced, as the window shows it.
#[derive(Debug, Clone, PartialEq)]
pub struct SalvageDone {
    /// "Salvaged 5 items into 4 bags".
    pub title: String,
    /// The bags.
    pub cards: Vec<ResultCard>,
    /// "Not salvaged: ..." when the server would not take some.
    pub refused: Option<String>,
}

/// What the Salvage window shows.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SalvageScreen {
    /// "Using an Ust · your Salvaging skill is 312".
    pub sub: String,
    /// The items to salvage.
    pub rows: Vec<StagedRow>,
    /// "2 items" / "nothing yet".
    pub count: String,
    /// The bags they make.
    pub bags: Vec<BagRow>,
    /// What the options allow ("Several materials at once").
    pub rule: String,
    /// The button's words.
    pub go: String,
    /// What a salvage produced (shown in place of the staging).
    pub done: Option<SalvageDone>,
}

fn workmanship(o: &ClientObject) -> f32 {
    o.workmanship
}

/// What the player is told for `why`.
#[must_use]
pub fn refusal_words(why: SalvageRefusal) -> String {
    match why {
        SalvageRefusal::IsTheTool => "That is the tool you are salvaging with.".to_owned(),
        SalvageRefusal::NotCarried => "You can only salvage what you carry.".to_owned(),
        SalvageRefusal::AlreadyListed => "It is on the list already.".to_owned(),
        SalvageRefusal::Retained => "Retained items cannot be salvaged.".to_owned(),
        SalvageRefusal::OtherMaterial { item, listed } => format!(
            "One material at a time: this is {}, the list is {}.",
            salvage_material_name(item),
            salvage_material_name(listed)
        ),
        SalvageRefusal::NoTool | SalvageRefusal::Gone | SalvageRefusal::Unsuitable => {
            "That cannot be salvaged.".to_owned()
        }
    }
}

/// Why `o` cannot be salvaged, if it cannot (`material` is the staged one
/// when one material at a time is allowed).
#[must_use]
pub fn refusal(o: &ClientObject, material: u32, allow_multiple: bool) -> Option<String> {
    item_refusal(o, material, allow_multiple).map(refusal_words)
}

/// The Salvage window's view of the staging (`staged`, of `material`) with
/// `tool`, the player's skill and options.
#[allow(clippy::too_many_arguments)]
pub fn screen(
    objects: &ClientObjectTable,
    player: u32,
    tool: u32,
    staged: &[u32],
    skill: Option<i32>,
    allow_multiple: bool,
    icon: &mut dyn FnMut(&ClientObject) -> Option<TextureRef>,
    names: &dyn Fn(&ClientObject) -> String,
) -> SalvageScreen {
    let tool_name = objects.get(tool).map(names).map_or_else(
        || "a salvaging tool".to_owned(),
        |n| {
            let vowel = n.starts_with(['A', 'E', 'I', 'O', 'U']);
            format!("{} {n}", if vowel { "an" } else { "a" })
        },
    );
    let sub = match skill {
        Some(s) => format!("Using {tool_name} \u{b7} your Salvaging skill is {s}"),
        None => format!("Using {tool_name}"),
    };
    let place = |id: u32| {
        let pack = objects.get(id).map_or(0, |o| o.container_id);
        if pack == player {
            "Main pack".to_owned()
        } else {
            objects
                .get(pack)
                .map_or_else(|| "Pack".to_owned(), |p| p.appropriate_name().into_owned())
        }
    };
    let mut rows = Vec::new();
    let mut by_material: Vec<(u32, Vec<f32>)> = Vec::new();
    for &id in staged {
        let Some(o) = objects.get(id) else {
            continue;
        };
        let material = o.material_type.unwrap_or(0);
        match by_material.iter_mut().find(|(m, _)| *m == material) {
            Some((_, w)) => w.push(workmanship(o)),
            None => by_material.push((material, vec![workmanship(o)])),
        }
        rows.push(StagedRow {
            id,
            icon: icon(o),
            name: names(o),
            sub: format!("{} \u{b7} worth {}p", place(id), group_thousands(o.value)),
            material: salvage_material_name(material).to_owned(),
            color: material_color(material),
            workmanship: format!("{:.0}", workmanship(o)),
        });
    }
    let bags = by_material
        .iter()
        .map(|(material, ws)| {
            let avg = ws.iter().sum::<f32>() / ws.len().max(1) as f32;
            let n = ws.len();
            BagRow {
                name: format!("Salvaged {}", salvage_material_name(*material)),
                sub: format!(
                    "{} \u{b7} workmanship {avg:.1}",
                    if n == 1 {
                        "1 item".to_owned()
                    } else {
                        format!("{n} items")
                    }
                ),
                color: material_color(*material),
            }
        })
        .collect();
    let n = rows.len();
    SalvageScreen {
        sub,
        count: match n {
            0 => "nothing yet".to_owned(),
            1 => "1 item".to_owned(),
            n => format!("{n} items"),
        },
        rows,
        bags,
        rule: if allow_multiple {
            "Several materials at once (Options, Gameplay)".to_owned()
        } else {
            "One material at a time (Options, Gameplay)".to_owned()
        },
        go: match n {
            0 => "Salvage".to_owned(),
            1 => "Salvage 1 item".to_owned(),
            n => format!("Salvage {n} items"),
        },
        done: None,
    }
}

/// The player's items Add from packs lists (of `filter`'s material; 0:
/// all), and the material chips over them.
#[allow(clippy::too_many_arguments)]
pub fn picks(
    objects: &ClientObjectTable,
    player: u32,
    tool: u32,
    staged: &[u32],
    material: u32,
    allow_multiple: bool,
    filter: u32,
    icon: &mut dyn FnMut(&ClientObject) -> Option<TextureRef>,
    names: &dyn Fn(&ClientObject) -> String,
) -> (Vec<MaterialChip>, Vec<SalvagePick>) {
    let mut rows = Vec::new();
    let mut chips: Vec<MaterialChip> = Vec::new();
    for (id, pack) in carried(objects, player) {
        if id == tool {
            continue;
        }
        let Some(o) = objects.get(id) else {
            continue;
        };
        let m = o.material_type.unwrap_or(0);
        // Only what could ever be salvaged is listed.
        if !is_salvage_suitable(o, 0, true) && !o.flags().contains(PublicWeenieFlags::RETAINED) {
            continue;
        }
        if m != 0 {
            match chips.iter_mut().find(|c| c.material == m) {
                Some(c) => c.count += 1,
                None => chips.push(MaterialChip {
                    material: m,
                    name: salvage_material_name(m).to_owned(),
                    count: 1,
                    color: material_color(m),
                }),
            }
        }
        if filter != 0 && m != filter {
            continue;
        }
        let added = staged.contains(&id);
        let refused = refusal(o, material, allow_multiple);
        let place = if pack == player {
            "Main pack".to_owned()
        } else {
            objects
                .get(pack)
                .map_or_else(|| "Pack".to_owned(), |p| p.appropriate_name().into_owned())
        };
        let sub = match (&refused, added) {
            (_, true) => format!(
                "{} \u{b7} workmanship {:.0} \u{b7} added",
                salvage_material_name(m),
                o.workmanship
            ),
            (Some(r), false) => r.clone(),
            (None, false) => format!(
                "{} \u{b7} workmanship {:.0} \u{b7} {place}",
                salvage_material_name(m),
                o.workmanship
            ),
        };
        rows.push(SalvagePick {
            id,
            icon: icon(o),
            name: names(o),
            sub,
            off: added || refused.is_some(),
            added,
            refused: refused.is_some() && !added,
            material: m,
        });
    }
    if filter != 0 && !chips.iter().any(|c| c.material == filter) {
        // Nothing of that material is left: everything shows.
        return picks(
            objects,
            player,
            tool,
            staged,
            material,
            allow_multiple,
            0,
            icon,
            names,
        );
    }
    chips.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.name.cmp(&b.name)));
    let all = MaterialChip {
        material: 0,
        name: "All".to_owned(),
        count: chips.iter().map(|c| c.count).sum(),
        color: tone::gold(1.0),
    };
    chips.insert(0, all);
    (chips, rows)
}

/// What `outcome` produced, with how many of the salvaged items were of
/// each material (`from`).
#[must_use]
pub fn done(outcome: &SalvageOutcome, from: &[(u32, usize)], refused: &[String]) -> SalvageDone {
    let items: usize = from.iter().map(|(_, n)| n).sum();
    let bags = outcome.bags.len();
    let title = if bags == 0 {
        "Nothing was salvaged".to_owned()
    } else {
        format!(
            "Salvaged {} into {}",
            if items == 1 {
                "1 item".to_owned()
            } else {
                format!("{items} items")
            },
            if bags == 1 {
                "1 bag".to_owned()
            } else {
                format!("{bags} bags")
            }
        )
    };
    let cards = outcome
        .bags
        .iter()
        .map(|b| {
            let n = from
                .iter()
                .find(|(m, _)| *m == b.material_type)
                .map_or(0, |(_, n)| *n);
            ResultCard {
                name: format!("Salvaged {}", salvage_material_name(b.material_type)),
                from: if n == 1 {
                    "from 1 item".to_owned()
                } else {
                    format!("from {n} items")
                },
                units: b.units.to_string(),
                workmanship: format!("workmanship {:.2}", b.workmanship),
                color: material_color(b.material_type),
            }
        })
        .collect();
    SalvageDone {
        title,
        cards,
        refused: (!refused.is_empty()).then(|| format!("Not salvaged: {}.", refused.join(", "))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ac_core::items::{EquipMask, ItemType, SalvageBag};

    const ME: u32 = 0x5000_0001;
    const UST: u32 = 0x8000_0001;
    /// Steel and Granite in the game's material list.
    const STEEL: u32 = 64;
    const GRANITE: u32 = 67;

    fn put(t: &mut ClientObjectTable, o: ClientObject, container: u32) {
        let id = o.object_id();
        let mut ev = Vec::new();
        t.add_or_update(o, &mut ev);
        if container != 0 {
            t.move_item(id, container, -1, EquipMask::NONE, None, &mut ev);
        }
    }

    fn item(t: &mut ClientObjectTable, id: u32, name: &str, material: u32, ws: f32) {
        let mut o = ClientObject::new(id);
        o.name = name.into();
        o.item_type = ItemType::MELEE_WEAPON;
        o.material_type = Some(material);
        o.workmanship = ws;
        o.value = 1000;
        put(t, o, ME);
    }

    fn table() -> ClientObjectTable {
        let mut t = ClientObjectTable::new();
        let mut me = ClientObject::new(ME);
        me.items_capacity = 102;
        put(&mut t, me, 0);
        let mut ust = ClientObject::new(UST);
        ust.name = "Ust".into();
        ust.item_type = ItemType::TINKERING_TOOL;
        put(&mut t, ust, ME);
        item(&mut t, 0x8000_0010, "Steel Longsword", STEEL, 6.0);
        item(&mut t, 0x8000_0011, "Steel Shield", STEEL, 5.0);
        item(&mut t, 0x8000_0012, "Granite Mace", GRANITE, 4.0);
        t
    }

    #[test]
    fn the_materials_are_the_games() {
        assert_eq!(salvage_material_name(STEEL), "Steel");
        assert_eq!(salvage_material_name(GRANITE), "Granite");
    }

    #[test]
    fn one_bag_per_material_with_the_average_workmanship() {
        let t = table();
        let s = screen(
            &t,
            ME,
            UST,
            &[0x8000_0010, 0x8000_0011, 0x8000_0012],
            Some(312),
            true,
            &mut |_| None,
            &|o| o.name.clone(),
        );
        assert_eq!(s.sub, "Using an Ust \u{b7} your Salvaging skill is 312");
        let bags: Vec<(&str, &str)> = s
            .bags
            .iter()
            .map(|b| (b.name.as_str(), b.sub.as_str()))
            .collect();
        assert_eq!(
            bags,
            [
                ("Salvaged Steel", "2 items \u{b7} workmanship 5.5"),
                ("Salvaged Granite", "1 item \u{b7} workmanship 4.0"),
            ]
        );
        assert_eq!(s.go, "Salvage 3 items");
    }

    #[test]
    fn one_material_at_a_time_refuses_another() {
        let t = table();
        let (chips, rows) = picks(
            &t,
            ME,
            UST,
            &[0x8000_0010],
            STEEL,
            false,
            0,
            &mut |_| None,
            &|o| o.name.clone(),
        );
        assert_eq!(chips[0].count, 3);
        let mace = rows.iter().find(|r| r.name == "Granite Mace").unwrap();
        assert!(mace.refused);
        assert!(
            mace.sub.starts_with("One material at a time"),
            "{}",
            mace.sub
        );
        let sword = rows.iter().find(|r| r.name == "Steel Longsword").unwrap();
        assert!(sword.added && sword.off);
    }

    #[test]
    fn the_result_says_what_each_bag_came_from() {
        let outcome = SalvageOutcome {
            bags: vec![SalvageBag {
                material_type: STEEL,
                workmanship: 5.5,
                units: 64,
            }],
            unsuitable: Vec::new(),
        };
        let d = done(&outcome, &[(STEEL, 2)], &[]);
        assert_eq!(d.title, "Salvaged 2 items into 1 bag");
        assert_eq!(d.cards[0].from, "from 2 items");
        assert_eq!(d.cards[0].workmanship, "workmanship 5.50");
    }
}
