//! What the modern inventory shows, read from the object table: the
//! player's packs and how full they are, the items in view (one pack, or
//! every pack grouped by pack, by kind or not at all), the filter chips,
//! the search over every pack and the sort; the 24 equipment slots; burden
//! and pyreals. Nothing here draws or sends anything.

use std::cmp::Ordering;

use ac_core::items::placement::is_pack;
use ac_core::items::{AetheriaUnlockState, aetheria_unlocks};
use ac_core::items::{ClientObject, ClientObjectTable, EquipMask, ItemType, burden};

/// Main pack slots when the player names no capacity.
pub const MAIN_PACK_SLOTS: usize = 102;
/// Side pack slots when a pack names no capacity.
pub const SIDE_PACK_SLOTS: usize = 24;
/// The player's carried-burden property.
const ENCUMBRANCE_VAL_PROPERTY: u32 = 5;
/// The player's burden augmentation property.
const ENCUMBRANCE_AUG_PROPERTY: u32 = 0xE6;
/// The player's coin value property (the pyreals carried).
const COIN_VALUE_PROPERTY: u32 = 20;

/// Which packs the grid shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PackChoice {
    /// Every pack.
    All,
    /// The main pack (the player's own items).
    #[default]
    Main,
    /// One side pack.
    Side(u32),
}

/// How "All packs" is split up.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GroupBy {
    /// One run of items.
    None,
    /// A group per pack.
    #[default]
    Pack,
    /// A group per kind (the filter chips' kinds).
    Kind,
}

impl GroupBy {
    /// Every grouping, in the menu's order.
    pub const ALL: [Self; 3] = [Self::None, Self::Pack, Self::Kind];

    /// Its name in the menu and on the button.
    #[must_use]
    pub fn words(self) -> &'static str {
        match self {
            Self::None => "None",
            Self::Pack => "Pack",
            Self::Kind => "Kind",
        }
    }
}

/// The order items are shown in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SortBy {
    /// The pack's own order (what a drag arranges).
    #[default]
    Mine,
    /// By name.
    Name,
    /// By kind, then name.
    Type,
    /// Most valuable first.
    Value,
    /// Heaviest first.
    Burden,
    /// Best workmanship first.
    Workmanship,
    /// Newest first.
    Newest,
}

impl SortBy {
    /// Every order, in the menu's order.
    pub const ALL: [Self; 7] = [
        Self::Name,
        Self::Type,
        Self::Value,
        Self::Burden,
        Self::Workmanship,
        Self::Newest,
        Self::Mine,
    ];

    /// Its name in the menu and on the button.
    #[must_use]
    pub fn words(self) -> &'static str {
        match self {
            Self::Mine => "My order",
            Self::Name => "Name",
            Self::Type => "Type",
            Self::Value => "Value",
            Self::Burden => "Burden",
            Self::Workmanship => "Workmanship",
            Self::Newest => "Newest first",
        }
    }

    /// The menu's note beside it.
    #[must_use]
    pub fn note(self) -> &'static str {
        match self {
            Self::Type => "then name",
            Self::Mine => "drag to arrange",
            _ => "",
        }
    }
}

/// An item's kind, as the filter chips and the kind groups split them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Kind {
    /// Weapons, casters, armor, clothing and jewelry.
    Gear,
    /// Spell components, mana stones and scrolls.
    Magic,
    /// Food, potions and healing kits.
    Consumables,
    /// Salvage.
    Salvage,
    /// Everything else.
    Other,
}

impl Kind {
    /// Every kind, in the chips' order.
    pub const ALL: [Self; 5] = [
        Self::Gear,
        Self::Magic,
        Self::Consumables,
        Self::Salvage,
        Self::Other,
    ];

    /// Its name on a chip.
    #[must_use]
    pub fn words(self) -> &'static str {
        match self {
            Self::Gear => "Gear",
            Self::Magic => "Magic",
            Self::Consumables => "Consumables",
            Self::Salvage => "Salvage",
            Self::Other => "Other",
        }
    }

    /// The kind of `item`.
    #[must_use]
    pub fn of(item: &ClientObject) -> Self {
        let t = item.item_type;
        if t.intersects(ItemType::TINKERING_MATERIAL) {
            Self::Salvage
        } else if item.valid_locations != EquipMask::NONE
            || t.intersects(
                ItemType::WEAPON_OR_CASTER
                    | ItemType::ARMOR
                    | ItemType::CLOTHING
                    | ItemType::JEWELRY,
            )
        {
            Self::Gear
        } else if t.intersects(ItemType::SPELL_COMPONENTS | ItemType::MANA_STONE)
            || item.spell_id.is_some_and(|s| s != 0)
        {
            Self::Magic
        } else if t.intersects(ItemType::FOOD)
            || item.useability.is_some() && item.cooldown_id.is_some()
        {
            Self::Consumables
        } else {
            Self::Other
        }
    }
}

/// One of the player's packs, as the pack rail shows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pack {
    /// The pack (the player's id for the main pack).
    pub id: u32,
    /// Its name.
    pub name: String,
    /// Loose items in it.
    pub used: usize,
    /// How many it holds.
    pub capacity: usize,
    /// The main pack.
    pub main: bool,
}

impl Pack {
    /// How full it is, 0..1.
    #[must_use]
    pub fn fill(&self) -> f32 {
        if self.capacity == 0 {
            return 0.0;
        }
        (self.used as f32 / self.capacity as f32).clamp(0.0, 1.0)
    }

    /// Free slots.
    #[must_use]
    pub fn free(&self) -> usize {
        self.capacity.saturating_sub(self.used)
    }

    /// "22/24".
    #[must_use]
    pub fn fill_words(&self) -> String {
        format!("{}/{}", self.used, self.capacity)
    }
}

/// The player's packs: the main pack, then each side pack in order.
#[must_use]
pub fn packs(objects: &ClientObjectTable, player: u32) -> Vec<Pack> {
    let loose = |container: u32| {
        objects
            .get_contents(container)
            .iter()
            .filter_map(|&g| objects.get(g))
            .filter(|o| o.currently_equipped_location == EquipMask::NONE && !is_pack(o))
            .count()
    };
    let main_capacity = objects
        .get(player)
        .map_or(0, |o| o.items_capacity)
        .try_into()
        .ok()
        .filter(|&c: &usize| c > 0)
        .unwrap_or(MAIN_PACK_SLOTS);
    let mut out = vec![Pack {
        id: player,
        name: "Main pack".to_owned(),
        used: loose(player),
        capacity: main_capacity,
        main: true,
    }];
    for &g in objects.get_contents(player) {
        let Some(o) = objects.get(g) else {
            continue;
        };
        if o.currently_equipped_location != EquipMask::NONE || !is_pack(o) {
            continue;
        }
        let capacity = usize::try_from(o.items_capacity)
            .ok()
            .filter(|&c| c > 0)
            .unwrap_or(SIDE_PACK_SLOTS);
        out.push(Pack {
            id: g,
            name: o.appropriate_name().into_owned(),
            used: loose(g),
            capacity,
            main: false,
        });
    }
    out
}

/// What the grid is asked to show.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ViewOptions {
    /// The packs.
    pub pack: PackChoice,
    /// How "All packs" (and a search) is split up.
    pub group: GroupBy,
    /// The order.
    pub sort: SortBy,
    /// The order reversed.
    pub reverse: bool,
    /// The one kind shown (none: all).
    pub kind: Option<Kind>,
    /// Words every shown item's name holds (any case); a search covers
    /// every pack.
    pub search: String,
}

/// One run of items under an optional heading.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Group {
    /// The heading's key (a pack id, or the kind), so a closed group stays
    /// closed while the items change.
    pub key: u64,
    /// The heading (none: no heading).
    pub title: Option<String>,
    /// The heading's right side ("22/24").
    pub meta: String,
    /// The items, in the order shown.
    pub items: Vec<u32>,
    /// Empty cells shown after them (a single pack's free slots, in its
    /// own order only).
    pub empty: usize,
    /// The pack a drop on an empty cell goes into (none: a mixed group).
    pub container: Option<u32>,
}

/// The grid's contents.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct GridView {
    /// The title over the grid ("Main pack", "All packs", "Search").
    pub title: String,
    /// The count beside it.
    pub count: String,
    /// The groups.
    pub groups: Vec<Group>,
}

/// Facts the sort reads besides the object.
pub struct SortFacts<'a> {
    /// The displayed name.
    pub name: &'a dyn Fn(&ClientObject) -> String,
    /// When the item first came, larger is newer.
    pub arrived: &'a dyn Fn(u32) -> u64,
}

impl std::fmt::Debug for SortFacts<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SortFacts").finish_non_exhaustive()
    }
}

/// The grid's contents for `options`.
#[must_use]
pub fn grid(
    objects: &ClientObjectTable,
    player: u32,
    options: &ViewOptions,
    facts: &SortFacts<'_>,
) -> GridView {
    let all_packs = packs(objects, player);
    let search = options.search.trim().to_lowercase();
    let searching = !search.is_empty();
    let shown = |o: &ClientObject| {
        options.kind.is_none_or(|k| Kind::of(o) == k)
            && (!searching || (facts.name)(o).to_lowercase().contains(&search))
    };
    let loose_of = |container: u32| -> Vec<u32> {
        objects
            .get_contents(container)
            .iter()
            .copied()
            .filter(|&g| {
                objects.get(g).is_some_and(|o| {
                    o.currently_equipped_location == EquipMask::NONE && !is_pack(o) && shown(o)
                })
            })
            .collect()
    };
    let filtered = options.kind.is_some() || searching;
    let ordered = options.sort == SortBy::Mine && !options.reverse;
    let one = match options.pack {
        _ if searching => None,
        PackChoice::All => None,
        PackChoice::Main => all_packs.first(),
        PackChoice::Side(id) => all_packs.iter().find(|p| p.id == id),
    };
    if let Some(pack) = one {
        let mut items = loose_of(pack.id);
        sort(objects, &mut items, options, facts);
        let empty = if ordered && !filtered { pack.free() } else { 0 };
        return GridView {
            title: pack.name.clone(),
            count: format!(
                "{} of {} \u{b7} {} free",
                pack.used,
                pack.capacity,
                pack.free()
            ),
            groups: vec![Group {
                key: u64::from(pack.id),
                title: None,
                meta: String::new(),
                items,
                empty,
                container: Some(pack.id),
            }],
        };
    }
    let total: usize = all_packs.iter().map(|p| p.used).sum();
    let free: usize = all_packs.iter().map(Pack::free).sum();
    let mut groups = Vec::new();
    match options.group {
        GroupBy::Pack => {
            for pack in &all_packs {
                let mut items = loose_of(pack.id);
                if searching && items.is_empty() {
                    continue;
                }
                sort(objects, &mut items, options, facts);
                groups.push(Group {
                    key: u64::from(pack.id),
                    title: Some(pack.name.to_uppercase()),
                    meta: if pack.used == 0 {
                        format!("{} \u{b7} empty", pack.fill_words())
                    } else {
                        pack.fill_words()
                    },
                    items,
                    empty: 0,
                    container: Some(pack.id),
                });
            }
        }
        GroupBy::Kind => {
            let every: Vec<u32> = all_packs.iter().flat_map(|p| loose_of(p.id)).collect();
            for kind in Kind::ALL {
                let mut items: Vec<u32> = every
                    .iter()
                    .copied()
                    .filter(|&g| objects.get(g).is_some_and(|o| Kind::of(o) == kind))
                    .collect();
                if items.is_empty() {
                    continue;
                }
                sort(objects, &mut items, options, facts);
                groups.push(Group {
                    key: 1 << 40 | kind as u64,
                    title: Some(kind.words().to_uppercase()),
                    meta: items.len().to_string(),
                    items,
                    empty: 0,
                    container: None,
                });
            }
        }
        GroupBy::None => {
            let mut items: Vec<u32> = all_packs.iter().flat_map(|p| loose_of(p.id)).collect();
            sort(objects, &mut items, options, facts);
            groups.push(Group {
                key: 0,
                title: None,
                meta: String::new(),
                items,
                empty: 0,
                container: None,
            });
        }
    }
    let found: usize = groups.iter().map(|g| g.items.len()).sum();
    GridView {
        title: if searching {
            "Search".to_owned()
        } else {
            "All packs".to_owned()
        },
        count: if searching {
            format!("{found} found in every pack")
        } else {
            format!("{total} items \u{b7} {free} free")
        },
        groups,
    }
}

/// `items` in `options`' order.
fn sort(
    objects: &ClientObjectTable,
    items: &mut [u32],
    options: &ViewOptions,
    facts: &SortFacts<'_>,
) {
    let name = |g: u32| {
        objects
            .get(g)
            .map(|o| (facts.name)(o).to_lowercase())
            .unwrap_or_default()
    };
    let int = |g: u32, f: fn(&ClientObject) -> i64| objects.get(g).map_or(0, f);
    match options.sort {
        SortBy::Mine => {}
        SortBy::Name => items.sort_by_cached_key(|&g| name(g)),
        SortBy::Type => items.sort_by_cached_key(|&g| {
            let kind = objects.get(g).map_or(Kind::Other, Kind::of);
            let t = objects.get(g).map_or(0, |o| o.item_type.bits());
            (kind, t, name(g))
        }),
        SortBy::Value => items.sort_by_key(|&g| std::cmp::Reverse(int(g, |o| i64::from(o.value)))),
        SortBy::Burden => {
            items.sort_by_key(|&g| std::cmp::Reverse(int(g, |o| i64::from(o.burden))));
        }
        SortBy::Workmanship => items.sort_by(|&a, &b| {
            let w = |g| objects.get(g).map_or(0.0, |o| o.workmanship);
            w(b).partial_cmp(&w(a)).unwrap_or(Ordering::Equal)
        }),
        SortBy::Newest => items.sort_by_key(|&g| std::cmp::Reverse((facts.arrived)(g))),
    }
    if options.reverse {
        items.reverse();
    }
}

/// Where an equipment slot sits in the window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlotRow {
    /// The column left of the figure.
    Left,
    /// The column right of the figure.
    Right,
    /// The jewelry row under it.
    Jewelry,
    /// The held things and Aetheria row.
    Held,
}

/// One of the 24 equipment slots.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EquipSlot {
    /// Its name.
    pub label: &'static str,
    /// The body locations it shows.
    pub mask: EquipMask,
    /// Where it sits.
    pub row: SlotRow,
    /// The Aetheria bit that opens it (none: always open).
    pub unlock: AetheriaUnlockState,
}

const fn slot(label: &'static str, mask: EquipMask, row: SlotRow) -> EquipSlot {
    EquipSlot {
        label,
        mask,
        row,
        unlock: AetheriaUnlockState::NONE,
    }
}

/// The weapon slot's locations.
const WEAPON: EquipMask = EquipMask(
    EquipMask::MELEE_WEAPON.bits()
        | EquipMask::MISSILE_WEAPON.bits()
        | EquipMask::HELD.bits()
        | EquipMask::TWO_HANDED.bits(),
);

/// The slots, in the window's order: down the left column, down the
/// right, the jewelry row, the held row.
pub const EQUIP_SLOTS: [EquipSlot; 24] = [
    slot("Head", EquipMask::HEAD_WEAR, SlotRow::Left),
    slot("Chest armor", EquipMask::CHEST_ARMOR, SlotRow::Left),
    slot("Upper arm armor", EquipMask::UPPER_ARM_ARMOR, SlotRow::Left),
    slot("Lower arm armor", EquipMask::LOWER_ARM_ARMOR, SlotRow::Left),
    slot("Hands", EquipMask::HAND_WEAR, SlotRow::Left),
    slot("Shirt", EquipMask::CHEST_WEAR, SlotRow::Left),
    slot("Cloak", EquipMask::CLOAK, SlotRow::Right),
    slot("Abdomen armor", EquipMask::ABDOMEN_ARMOR, SlotRow::Right),
    slot(
        "Upper leg armor",
        EquipMask::UPPER_LEG_ARMOR,
        SlotRow::Right,
    ),
    slot(
        "Lower leg armor",
        EquipMask::LOWER_LEG_ARMOR,
        SlotRow::Right,
    ),
    slot("Feet", EquipMask::FOOT_WEAR, SlotRow::Right),
    slot("Pants", EquipMask::UPPER_LEG_WEAR, SlotRow::Right),
    slot("Neck", EquipMask::NECK_WEAR, SlotRow::Jewelry),
    slot("Left wrist", EquipMask::WRIST_WEAR_LEFT, SlotRow::Jewelry),
    slot("Right wrist", EquipMask::WRIST_WEAR_RIGHT, SlotRow::Jewelry),
    slot("Left ring", EquipMask::FINGER_WEAR_LEFT, SlotRow::Jewelry),
    slot("Right ring", EquipMask::FINGER_WEAR_RIGHT, SlotRow::Jewelry),
    slot("Trinket", EquipMask::TRINKET_ONE, SlotRow::Jewelry),
    slot("Weapon", WEAPON, SlotRow::Held),
    slot("Shield or off hand", EquipMask::SHIELD, SlotRow::Held),
    slot("Ammunition", EquipMask::MISSILE_AMMO, SlotRow::Held),
    EquipSlot {
        label: "Blue Aetheria",
        mask: EquipMask::SIGIL_ONE,
        row: SlotRow::Held,
        unlock: AetheriaUnlockState::BLUE,
    },
    EquipSlot {
        label: "Yellow Aetheria",
        mask: EquipMask::SIGIL_TWO,
        row: SlotRow::Held,
        unlock: AetheriaUnlockState::YELLOW,
    },
    EquipSlot {
        label: "Red Aetheria",
        mask: EquipMask::SIGIL_THREE,
        row: SlotRow::Held,
        unlock: AetheriaUnlockState::RED,
    },
];

/// What one equipment slot shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct WornSlot {
    /// The item worn there (one covering several slots shows in each).
    pub item: Option<u32>,
    /// The slot is open (an Aetheria slot needs its quest).
    pub open: bool,
}

/// What each of [`EQUIP_SLOTS`] shows.
#[must_use]
pub fn worn(objects: &ClientObjectTable, player: u32) -> [WornSlot; 24] {
    let equipped: Vec<&ClientObject> = objects
        .objects()
        .filter(|o| {
            o.currently_equipped_location != EquipMask::NONE
                && (o.wielder_id == player || o.container_id == player)
        })
        .collect();
    let unlocked = aetheria_unlocks(objects.get(player));
    let mut out = [WornSlot::default(); 24];
    for (slot, out) in EQUIP_SLOTS.iter().zip(out.iter_mut()) {
        out.item = equipped
            .iter()
            .find(|o| o.currently_equipped_location.intersects(slot.mask))
            .map(|o| o.object_id());
        out.open = slot.unlock == AetheriaUnlockState::NONE || unlocked.intersects(slot.unlock);
    }
    out
}

/// The slots `item` may go in (what lights up while it is dragged): its
/// own locations, and the shield slot for a melee weapon (the off hand).
#[must_use]
pub fn slots_for(item: &ClientObject) -> Vec<usize> {
    EQUIP_SLOTS
        .iter()
        .enumerate()
        .filter(|(_, s)| {
            item.valid_locations.intersects(s.mask)
                || s.mask == EquipMask::SHIELD
                    && item.valid_locations.intersects(EquipMask::MELEE_WEAPON)
        })
        .map(|(i, _)| i)
        .collect()
}

/// The player's burden.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Burden {
    /// Carried.
    pub carried: i32,
    /// A full load.
    pub capacity: i32,
    /// The load in percent (0..=300).
    pub percent: i32,
}

impl Burden {
    /// The player's burden; `strength` is its current strength.
    #[must_use]
    pub fn of(objects: &ClientObjectTable, player: u32, strength: Option<i32>) -> Self {
        let p = objects.get(player);
        let aug = p.map_or(0, |o| o.properties.get_int(ENCUMBRANCE_AUG_PROPERTY, 0));
        let capacity = burden::encumbrance_capacity(strength.unwrap_or(10), aug);
        let carried = p
            .and_then(|o| o.properties.ints.get(&ENCUMBRANCE_VAL_PROPERTY).copied())
            .unwrap_or_else(|| objects.sum_carried_burden(player));
        let load = burden::load_ratio(capacity, carried);
        Self {
            carried,
            capacity,
            percent: burden::load_to_percent(load),
        }
    }

    /// The burden with `more` carried.
    #[must_use]
    pub fn with(self, more: i32) -> Self {
        let carried = self.carried.saturating_add(more);
        Self {
            carried,
            percent: burden::load_to_percent(burden::load_ratio(self.capacity, carried)),
            ..self
        }
    }

    /// Over a full load (slowed).
    #[must_use]
    pub fn slowed(self) -> bool {
        self.percent > 100
    }
}

/// The pyreals the player carries.
#[must_use]
pub fn pyreals(objects: &ClientObjectTable, player: u32) -> i64 {
    if let Some(&coins) = objects
        .get(player)
        .and_then(|o| o.properties.ints.get(&COIN_VALUE_PROPERTY))
    {
        return i64::from(coins);
    }
    objects
        .objects()
        .filter(|o| o.item_type.intersects(ItemType::MONEY))
        .filter(|o| objects.is_owned_by_object(o.object_id(), player))
        .map(|o| i64::from(o.value))
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    const ME: u32 = 0x5000_0001;

    fn put(t: &mut ClientObjectTable, o: ClientObject, container: u32, slot: i32) {
        let id = o.object_id();
        let mut ev = Vec::new();
        t.add_or_update(o, &mut ev);
        if container != 0 {
            t.move_item(id, container, slot, EquipMask::NONE, None, &mut ev);
        }
    }

    fn item(
        t: &mut ClientObjectTable,
        id: u32,
        name: &str,
        ty: ItemType,
        container: u32,
        slot: i32,
    ) {
        let mut o = ClientObject::new(id);
        o.name = name.into();
        o.item_type = ty;
        put(t, o, container, slot);
    }

    fn table() -> ClientObjectTable {
        let mut t = ClientObjectTable::new();
        let mut me = ClientObject::new(ME);
        me.name = "Me".into();
        me.items_capacity = 102;
        put(&mut t, me, 0, 0);
        item(&mut t, 0x8000_0001, "Sword", ItemType::MELEE_WEAPON, ME, 0);
        item(&mut t, 0x8000_0002, "Apple", ItemType::FOOD, ME, 1);
        let mut pack = ClientObject::new(0x8000_0010);
        pack.name = "Pack".into();
        pack.item_type = ItemType::CONTAINER;
        pack.items_capacity = 24;
        put(&mut t, pack, ME, 2);
        item(
            &mut t,
            0x8000_0011,
            "Iron Salvage",
            ItemType::TINKERING_MATERIAL,
            0x8000_0010,
            0,
        );
        item(&mut t, 0x8000_0012, "Zircon", ItemType::GEM, 0x8000_0010, 1);
        t
    }

    fn facts() -> (impl Fn(&ClientObject) -> String, impl Fn(u32) -> u64) {
        (|o: &ClientObject| o.name.clone(), |g: u32| u64::from(g))
    }

    fn view(t: &ClientObjectTable, options: &ViewOptions) -> GridView {
        let (name, arrived) = facts();
        grid(
            t,
            ME,
            options,
            &SortFacts {
                name: &name,
                arrived: &arrived,
            },
        )
    }

    #[test]
    fn the_packs_are_the_main_pack_then_each_side_pack() {
        let t = table();
        let p = packs(&t, ME);
        assert_eq!(p.len(), 2);
        assert_eq!((p[0].main, p[0].used, p[0].capacity), (true, 2, 102));
        assert_eq!(
            (p[1].name.as_str(), p[1].used, p[1].fill_words()),
            ("Pack", 2, "2/24".into())
        );
    }

    #[test]
    fn one_pack_shows_its_items_in_its_order_and_its_free_slots() {
        let t = table();
        let g = view(&t, &ViewOptions::default());
        assert_eq!(g.title, "Main pack");
        assert_eq!(g.count, "2 of 102 \u{b7} 100 free");
        assert_eq!(g.groups[0].items, [0x8000_0001, 0x8000_0002]);
        assert_eq!(g.groups[0].empty, 100);
        // Sorted, no empty cells.
        let g = view(
            &t,
            &ViewOptions {
                sort: SortBy::Name,
                ..ViewOptions::default()
            },
        );
        assert_eq!(g.groups[0].items, [0x8000_0002, 0x8000_0001]);
        assert_eq!(g.groups[0].empty, 0);
    }

    #[test]
    fn all_packs_groups_by_pack_or_by_kind() {
        let t = table();
        let all = ViewOptions {
            pack: PackChoice::All,
            ..ViewOptions::default()
        };
        let g = view(&t, &all);
        assert_eq!(g.count, "4 items \u{b7} 122 free");
        let titles: Vec<_> = g.groups.iter().map(|g| g.title.clone().unwrap()).collect();
        assert_eq!(titles, ["MAIN PACK", "PACK"]);
        let g = view(
            &t,
            &ViewOptions {
                group: GroupBy::Kind,
                ..all
            },
        );
        let titles: Vec<_> = g.groups.iter().map(|g| g.title.clone().unwrap()).collect();
        assert_eq!(titles, ["GEAR", "CONSUMABLES", "SALVAGE", "OTHER"]);
    }

    #[test]
    fn a_search_covers_every_pack_and_a_chip_keeps_one_kind() {
        let t = table();
        let g = view(
            &t,
            &ViewOptions {
                search: "iron".into(),
                ..ViewOptions::default()
            },
        );
        assert_eq!(g.title, "Search");
        assert_eq!(g.groups.len(), 1);
        assert_eq!(g.groups[0].items, [0x8000_0011]);
        let g = view(
            &t,
            &ViewOptions {
                kind: Some(Kind::Gear),
                ..ViewOptions::default()
            },
        );
        assert_eq!(g.groups[0].items, [0x8000_0001]);
        assert_eq!(g.groups[0].empty, 0);
    }

    #[test]
    fn a_coat_shows_in_every_slot_it_covers_and_aetheria_waits_for_its_quest() {
        let mut t = table();
        let mut coat = ClientObject::new(0x8000_0020);
        coat.currently_equipped_location = EquipMask::CHEST_ARMOR | EquipMask::UPPER_ARM_ARMOR;
        coat.valid_locations = coat.currently_equipped_location;
        coat.wielder_id = ME;
        put(&mut t, coat, 0, 0);
        let w = worn(&t, ME);
        assert_eq!(w[1].item, Some(0x8000_0020));
        assert_eq!(w[2].item, Some(0x8000_0020));
        assert_eq!(w[0].item, None);
        assert!(w[0].open && !w[21].open);
        let c = t.get(0x8000_0020).unwrap();
        assert_eq!(slots_for(c), [1, 2]);
    }

    #[test]
    fn a_melee_weapon_may_also_go_in_the_off_hand() {
        let mut o = ClientObject::new(1);
        o.valid_locations = EquipMask::MELEE_WEAPON;
        assert_eq!(slots_for(&o), [18, 19]);
    }
}
