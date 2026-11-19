//! The bars themselves, apart from how they are drawn: what each slot
//! holds, its keys, each bar's place and direction; what a key, a drop or
//! an edit changes; the first layout a character gets, made from its retail
//! shortcuts and favourite spells; and the saved form.

use ac_core::items::ShortcutEntry;
use ac_host::settings::{BarData, BarLayoutData, SlotData};
use ac_ui::input::KeyChord;
use ac_ui::{Key, Modifiers};

use super::keys;
use crate::ui::modern::places::{Anchor, Place};

/// A slot's side.
pub const SLOT: f32 = 40.0;
/// The gap between slots.
pub const GAP: f32 = 3.0;
/// The bar's inside margin.
pub const PAD: f32 = 4.0;
/// The most slots a bar has.
pub const MAX_SLOTS: usize = 40;
/// The slots a new bar starts with.
pub const NEW_BAR_SLOTS: usize = 6;

/// The number row's keys, 1 to =.
pub const NUMBER_ROW: [Key; 12] = [
    Key::Number1,
    Key::Number2,
    Key::Number3,
    Key::Number4,
    Key::Number5,
    Key::Number6,
    Key::Number7,
    Key::Number8,
    Key::Number9,
    Key::Number0,
    Key::Minus,
    Key::Equal,
];

/// Which of a slot's two keys: the one for Peace, Melee and Missile, or the
/// one for Magic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeyMode {
    /// Peace, Melee and Missile.
    Normal,
    /// Magic.
    Magic,
}

impl KeyMode {
    /// Both.
    pub const ALL: [Self; 2] = [Self::Normal, Self::Magic];

    fn index(self) -> usize {
        match self {
            Self::Normal => 0,
            Self::Magic => 1,
        }
    }

    /// The modes it stands for, in words.
    #[must_use]
    pub fn words(self) -> &'static str {
        match self {
            Self::Normal => "Peace, melee and missile",
            Self::Magic => "Magic",
        }
    }
}

/// What a slot holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SlotContent {
    /// An item (by object id), with the name it had when it was put there.
    Item { id: u32, name: String },
    /// A spell.
    Spell { id: u32, name: String },
}

impl SlotContent {
    /// Its name as it was put there.
    #[must_use]
    pub fn name(&self) -> &str {
        match self {
            Self::Item { name, .. } | Self::Spell { name, .. } => name,
        }
    }

    /// It is a spell.
    #[must_use]
    pub fn is_spell(&self) -> bool {
        matches!(self, Self::Spell { .. })
    }
}

/// One slot.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Slot {
    /// What it holds.
    pub content: Option<SlotContent>,
    /// Its keys: outside Magic, in Magic.
    pub keys: [Option<KeyChord>; 2],
}

impl Slot {
    /// Its key in `mode`.
    #[must_use]
    pub fn key(&self, mode: KeyMode) -> Option<KeyChord> {
        self.keys[mode.index()]
    }

    /// Sets its key in `mode`.
    pub fn set_key(&mut self, mode: KeyMode, chord: Option<KeyChord>) {
        self.keys[mode.index()] = chord;
    }
}

/// Where a bar is: held to a point of the screen, moved from there.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BarPlace {
    /// The point.
    pub anchor: Anchor,
    /// Moved right from there.
    pub dx: f32,
    /// Moved down from there.
    pub dy: f32,
}

impl BarPlace {
    /// The place of a bar of `size` whose top left is at (`x`, `y`) on a
    /// `screen`: held to the point of the screen's third it sits in, so it
    /// keeps to its corner or edge when the screen changes.
    #[must_use]
    pub fn at(x: f32, y: f32, size: (f32, f32), screen: (f32, f32)) -> Self {
        let third = |at: f32, len: f32, of: f32| {
            let centre = at + len * 0.5;
            if centre < of / 3.0 {
                0
            } else if centre < of * 2.0 / 3.0 {
                1
            } else {
                2
            }
        };
        let anchor = match (third(x, size.0, screen.0), third(y, size.1, screen.1)) {
            (0, 0) => Anchor::TopLeft,
            (1, 0) => Anchor::Top,
            (2, 0) => Anchor::TopRight,
            (0, 1) => Anchor::Left,
            (1, 1) => Anchor::Center,
            (2, 1) => Anchor::Right,
            (0, _) => Anchor::BottomLeft,
            (1, _) => Anchor::Bottom,
            _ => Anchor::BottomRight,
        };
        let (fx, fy) = anchor.fractions();
        Self {
            anchor,
            dx: (x - screen.0 * fx + size.0 * fx).round(),
            dy: (y - screen.1 * fy + size.1 * fy).round(),
        }
    }

    /// The top left of a bar of `size` here on a `screen`, kept on it.
    #[must_use]
    pub fn top_left(&self, size: (f32, f32), screen: (f32, f32)) -> (f32, f32) {
        let rect = Place {
            anchor: self.anchor,
            dx: self.dx,
            dy: self.dy,
            width: size.0,
            height: size.1,
        }
        .rect(screen);
        (rect.x.round(), rect.y.round())
    }
}

/// One bar.
#[derive(Debug, Clone, PartialEq)]
pub struct Bar {
    /// Its name.
    pub name: String,
    /// Where it is.
    pub place: BarPlace,
    /// It runs top to bottom.
    pub vertical: bool,
    /// It stays where it is, and its slots stay as they are.
    pub locked: bool,
    /// It is not shown (Edit layout shows it again).
    pub hidden: bool,
    /// Its slots.
    pub slots: Vec<Slot>,
}

impl Bar {
    /// An empty bar named `name` with `count` slots at `place`.
    #[must_use]
    pub fn new(name: &str, count: usize, place: BarPlace) -> Self {
        Self {
            name: name.to_owned(),
            place,
            vertical: false,
            locked: false,
            hidden: false,
            slots: vec![Slot::default(); count.clamp(1, MAX_SLOTS)],
        }
    }

    /// Its width and height.
    #[must_use]
    pub fn size(&self) -> (f32, f32) {
        bar_size(self.slots.len(), self.vertical)
    }

    /// Where slot `index` sits inside it.
    #[must_use]
    pub fn slot_origin(&self, index: usize) -> (f32, f32) {
        let along = PAD + index as f32 * (SLOT + GAP);
        if self.vertical {
            (PAD, along)
        } else {
            (along, PAD)
        }
    }

    /// Gives every slot in order its key in `mode` from `keys` (the rest
    /// none).
    fn with_keys(mut self, mode: KeyMode, keys: impl IntoIterator<Item = KeyChord>) -> Self {
        let mut keys = keys.into_iter();
        for slot in &mut self.slots {
            slot.set_key(mode, keys.next());
        }
        self
    }
}

/// The width and height of a bar of `count` slots.
#[must_use]
pub fn bar_size(count: usize, vertical: bool) -> (f32, f32) {
    let count = count.max(1) as f32;
    let long = PAD * 2.0 + SLOT * count + GAP * (count - 1.0);
    let short = PAD * 2.0 + SLOT;
    if vertical {
        (short, long)
    } else {
        (long, short)
    }
}

/// A slot: its bar and its place on it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SlotRef {
    /// The bar.
    pub bar: usize,
    /// The slot.
    pub slot: usize,
}

impl SlotRef {
    /// Slot `slot` of bar `bar`.
    #[must_use]
    pub fn new(bar: usize, slot: usize) -> Self {
        Self { bar, slot }
    }
}

/// What the player's retail toolbar and spell bars held, for the first
/// layout.
#[derive(Debug, Clone, Default)]
pub struct RetailBars {
    /// The toolbar's eighteen slots.
    pub shortcuts: Vec<Option<ShortcutEntry>>,
    /// The eight favourite spell bars.
    pub favorites: Vec<Vec<u32>>,
}

/// Every bar.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BarLayout {
    /// The bars, in the order they were made.
    pub bars: Vec<Bar>,
}

/// The Main bar's place: bottom centre, under the Spells bar.
const MAIN_PLACE: BarPlace = BarPlace {
    anchor: Anchor::Bottom,
    dx: 0.0,
    dy: -56.0,
};
/// The Spells bar's place: over the Main bar.
const SPELLS_PLACE: BarPlace = BarPlace {
    anchor: Anchor::Bottom,
    dx: 0.0,
    dy: -110.0,
};

/// The number row with `modifiers`.
fn number_row(modifiers: Modifiers) -> impl Iterator<Item = KeyChord> {
    NUMBER_ROW
        .into_iter()
        .map(move |key| KeyChord::key(key, modifiers))
}

impl BarLayout {
    /// The first layout of a character: the Main bar (keys 1 to =, and
    /// Shift with them in Magic) with the retail toolbar's numbered row;
    /// a second bar with its other row, when that held anything; the
    /// Spells bar (keys 1 to = in Magic) with the first favourite spell
    /// bar; and a bar for each other favourite bar that held spells, down
    /// the right edge. `item_name` and `spell_name` name what is put
    /// there.
    #[must_use]
    pub fn seeded(
        retail: &RetailBars,
        item_name: &dyn Fn(u32) -> String,
        spell_name: &dyn Fn(u32) -> String,
    ) -> Self {
        let content = |entry: &Option<ShortcutEntry>| {
            let e = (*entry)?;
            if e.object_id != 0 {
                Some(SlotContent::Item {
                    id: e.object_id,
                    name: item_name(e.object_id),
                })
            } else if e.spell_id != 0 {
                Some(SlotContent::Spell {
                    id: e.spell_id,
                    name: spell_name(e.spell_id),
                })
            } else {
                None
            }
        };
        let spell = |id: u32| SlotContent::Spell {
            id,
            name: spell_name(id),
        };
        let mut bars = Vec::new();
        let mut main = Bar::new("Main", NUMBER_ROW.len(), MAIN_PLACE)
            .with_keys(KeyMode::Normal, number_row(Modifiers::NONE))
            .with_keys(KeyMode::Magic, number_row(Modifiers::SHIFT));
        for (slot, entry) in main.slots.iter_mut().zip(retail.shortcuts.iter().take(9)) {
            slot.content = content(entry);
        }
        let mut spells = Bar::new(
            "Spells",
            retail
                .favorites
                .first()
                .map_or(0, Vec::len)
                .max(NUMBER_ROW.len()),
            SPELLS_PLACE,
        )
        .with_keys(KeyMode::Magic, number_row(Modifiers::NONE));
        if let Some(first) = retail.favorites.first() {
            for (slot, &id) in spells.slots.iter_mut().zip(first) {
                slot.content = Some(spell(id));
            }
        }
        bars.push(spells);
        bars.push(main);
        // The rest go down the right edge, side by side.
        let mut column = 0.0;
        let mut beside = |bars: &mut Vec<Bar>, mut bar: Bar| {
            bar.vertical = true;
            let (w, _) = bar.size();
            bar.place = BarPlace {
                anchor: Anchor::Right,
                dx: -16.0 - column,
                dy: 0.0,
            };
            column += w + 8.0;
            bars.push(bar);
        };
        let second: Vec<Option<SlotContent>> = retail
            .shortcuts
            .iter()
            .skip(9)
            .take(9)
            .map(content)
            .collect();
        if second.iter().any(Option::is_some) {
            let mut bar = Bar::new("Toolbar 2", second.len(), MAIN_PLACE);
            for (slot, c) in bar.slots.iter_mut().zip(second) {
                slot.content = c;
            }
            beside(&mut bars, bar);
        }
        for (tab, spells) in retail.favorites.iter().enumerate().skip(1) {
            if spells.is_empty() {
                continue;
            }
            let mut bar = Bar::new(&format!("Spells {}", tab + 1), spells.len(), MAIN_PLACE);
            for (slot, &id) in bar.slots.iter_mut().zip(spells) {
                slot.content = Some(spell(id));
            }
            beside(&mut bars, bar);
        }
        Self { bars }
    }

    /// The slot at `at`.
    #[must_use]
    pub fn slot(&self, at: SlotRef) -> Option<&Slot> {
        self.bars.get(at.bar)?.slots.get(at.slot)
    }

    fn slot_mut(&mut self, at: SlotRef) -> Option<&mut Slot> {
        self.bars.get_mut(at.bar)?.slots.get_mut(at.slot)
    }

    /// The slot of a shown bar whose key in `mode` is `chord`.
    #[must_use]
    pub fn slot_for_key(&self, chord: KeyChord, mode: KeyMode) -> Option<SlotRef> {
        self.bars
            .iter()
            .enumerate()
            .filter(|(_, b)| !b.hidden)
            .find_map(|(i, b)| {
                b.slots
                    .iter()
                    .position(|s| s.key(mode) == Some(chord))
                    .map(|j| SlotRef::new(i, j))
            })
    }

    /// Another slot (on any bar) whose key in `mode` is `chord`.
    #[must_use]
    pub fn clash(&self, chord: KeyChord, mode: KeyMode, except: SlotRef) -> Option<SlotRef> {
        self.bars.iter().enumerate().find_map(|(i, b)| {
            b.slots
                .iter()
                .enumerate()
                .find(|(j, s)| SlotRef::new(i, *j) != except && s.key(mode) == Some(chord))
                .map(|(j, _)| SlotRef::new(i, j))
        })
    }

    /// Gives slot `at` the key `chord` in `mode`. A slot that had it gives
    /// it up and takes `at`'s old key instead (the two swap).
    pub fn set_key(&mut self, at: SlotRef, mode: KeyMode, chord: Option<KeyChord>) {
        let Some(old) = self.slot(at).map(|s| s.key(mode)) else {
            return;
        };
        if let Some(chord) = chord
            && let Some(other) = self.clash(chord, mode, at)
            && let Some(s) = self.slot_mut(other)
        {
            s.set_key(mode, old);
        }
        if let Some(s) = self.slot_mut(at) {
            s.set_key(mode, chord);
        }
    }

    /// Puts `content` in slot `at` (what it held is gone).
    pub fn put(&mut self, at: SlotRef, content: Option<SlotContent>) {
        if let Some(s) = self.slot_mut(at) {
            s.content = content;
        }
    }

    /// Moves what slot `from` holds to slot `to`; what `to` held goes the
    /// other way. The keys stay with their slots.
    pub fn swap(&mut self, from: SlotRef, to: SlotRef) {
        if from == to {
            return;
        }
        let (Some(a), Some(b)) = (
            self.slot(from).map(|s| s.content.clone()),
            self.slot(to).map(|s| s.content.clone()),
        ) else {
            return;
        };
        self.put(from, b);
        self.put(to, a);
    }

    /// The first empty slot of a shown bar, the spell bars' first for a
    /// spell (the bars whose first key is in Magic only), the others' first
    /// for an item.
    #[must_use]
    pub fn first_empty(&self, spell: bool) -> Option<SlotRef> {
        let spell_bar = |b: &Bar| {
            b.slots
                .iter()
                .any(|s| s.key(KeyMode::Magic).is_some() && s.key(KeyMode::Normal).is_none())
        };
        let empty = |(i, b): (usize, &Bar)| {
            b.slots
                .iter()
                .position(|s| s.content.is_none())
                .map(|j| SlotRef::new(i, j))
        };
        let shown = || self.bars.iter().enumerate().filter(|(_, b)| !b.hidden);
        shown()
            .filter(|(_, b)| spell_bar(b) == spell)
            .find_map(empty)
            .or_else(|| shown().find_map(empty))
    }

    /// Bar `bar` has `slot` somewhere already.
    #[must_use]
    pub fn holds(&self, content: &SlotContent) -> bool {
        self.bars.iter().any(|b| {
            b.slots
                .iter()
                .any(|s| s.content.as_ref().is_some_and(|c| same(c, content)))
        })
    }

    /// Adds an empty slot at the end of bar `bar`.
    pub fn add_slot(&mut self, bar: usize) {
        if let Some(b) = self.bars.get_mut(bar)
            && b.slots.len() < MAX_SLOTS
        {
            b.slots.push(Slot::default());
        }
    }

    /// Takes the last slot off bar `bar` (a bar keeps one).
    pub fn remove_last_slot(&mut self, bar: usize) {
        if let Some(b) = self.bars.get_mut(bar)
            && b.slots.len() > 1
        {
            b.slots.pop();
        }
    }

    /// A new empty bar at `place`, named "Bar N" with the first N not taken; its
    /// index.
    pub fn new_bar(&mut self, place: BarPlace) -> usize {
        let mut n = 1;
        while self.bars.iter().any(|b| b.name == format!("Bar {n}")) {
            n += 1;
        }
        self.bars
            .push(Bar::new(&format!("Bar {n}"), NEW_BAR_SLOTS, place));
        self.bars.len() - 1
    }

    /// The saved form.
    #[must_use]
    pub fn to_data(&self) -> BarLayoutData {
        BarLayoutData {
            bars: self
                .bars
                .iter()
                .map(|b| BarData {
                    name: b.name.clone(),
                    anchor: anchor_name(b.place.anchor).to_owned(),
                    dx: b.place.dx,
                    dy: b.place.dy,
                    vertical: b.vertical,
                    locked: b.locked,
                    hidden: b.hidden,
                    slots: b
                        .slots
                        .iter()
                        .map(|s| {
                            let (kind, id, name) = match &s.content {
                                Some(SlotContent::Item { id, name }) => ("item", *id, name.clone()),
                                Some(SlotContent::Spell { id, name }) => {
                                    ("spell", *id, name.clone())
                                }
                                None => ("", 0, String::new()),
                            };
                            SlotData {
                                kind: kind.to_owned(),
                                id,
                                name,
                                key: s.key(KeyMode::Normal).map(keys::save).unwrap_or_default(),
                                magic_key: s
                                    .key(KeyMode::Magic)
                                    .map(keys::save)
                                    .unwrap_or_default(),
                            }
                        })
                        .collect(),
                })
                .collect(),
        }
    }

    /// The layout a saved form describes (an unknown anchor is bottom
    /// centre; an unknown content or key is none).
    #[must_use]
    pub fn from_data(data: &BarLayoutData) -> Self {
        Self {
            bars: data
                .bars
                .iter()
                .map(|b| Bar {
                    name: b.name.clone(),
                    place: BarPlace {
                        anchor: anchor_from_name(&b.anchor).unwrap_or(Anchor::Bottom),
                        dx: b.dx,
                        dy: b.dy,
                    },
                    vertical: b.vertical,
                    locked: b.locked,
                    hidden: b.hidden,
                    slots: if b.slots.is_empty() {
                        vec![Slot::default()]
                    } else {
                        b.slots
                            .iter()
                            .take(MAX_SLOTS)
                            .map(|s| Slot {
                                content: match s.kind.as_str() {
                                    "item" if s.id != 0 => Some(SlotContent::Item {
                                        id: s.id,
                                        name: s.name.clone(),
                                    }),
                                    "spell" if s.id != 0 => Some(SlotContent::Spell {
                                        id: s.id,
                                        name: s.name.clone(),
                                    }),
                                    _ => None,
                                },
                                keys: [keys::load(&s.key), keys::load(&s.magic_key)],
                            })
                            .collect()
                    },
                })
                .collect(),
        }
    }
}

/// The two hold the same item or spell.
fn same(a: &SlotContent, b: &SlotContent) -> bool {
    match (a, b) {
        (SlotContent::Item { id: x, .. }, SlotContent::Item { id: y, .. })
        | (SlotContent::Spell { id: x, .. }, SlotContent::Spell { id: y, .. }) => x == y,
        _ => false,
    }
}

const ANCHORS: [(Anchor, &str); 9] = [
    (Anchor::TopLeft, "TopLeft"),
    (Anchor::Top, "Top"),
    (Anchor::TopRight, "TopRight"),
    (Anchor::Left, "Left"),
    (Anchor::Center, "Center"),
    (Anchor::Right, "Right"),
    (Anchor::BottomLeft, "BottomLeft"),
    (Anchor::Bottom, "Bottom"),
    (Anchor::BottomRight, "BottomRight"),
];

fn anchor_name(anchor: Anchor) -> &'static str {
    ANCHORS
        .iter()
        .find(|(a, _)| *a == anchor)
        .map_or("Bottom", |(_, n)| n)
}

fn anchor_from_name(name: &str) -> Option<Anchor> {
    ANCHORS.iter().find(|(_, n)| *n == name).map(|(a, _)| *a)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(k: Key) -> KeyChord {
        KeyChord::key(k, Modifiers::NONE)
    }

    fn shift(k: Key) -> KeyChord {
        KeyChord::key(k, Modifiers::SHIFT)
    }

    fn retail() -> RetailBars {
        let mut shortcuts = vec![None; 18];
        shortcuts[0] = Some(ShortcutEntry {
            index: 0,
            object_id: 0x8000_0001,
            spell_id: 0,
        });
        shortcuts[2] = Some(ShortcutEntry {
            index: 2,
            object_id: 0x8000_0002,
            spell_id: 0,
        });
        let mut favorites = vec![Vec::new(); 8];
        favorites[0] = vec![27, 28];
        favorites[3] = vec![40, 41, 42];
        RetailBars {
            shortcuts,
            favorites,
        }
    }

    fn seeded() -> BarLayout {
        BarLayout::seeded(&retail(), &|id| format!("Item {id:x}"), &|id| {
            format!("Spell {id}")
        })
    }

    #[test]
    fn the_first_layout_follows_the_retail_bars() {
        let layout = seeded();
        let names: Vec<&str> = layout.bars.iter().map(|b| b.name.as_str()).collect();
        assert_eq!(names, ["Spells", "Main", "Spells 4"]);
        let main = &layout.bars[1];
        assert_eq!(main.slots.len(), 12);
        assert_eq!(
            main.slots[0].content,
            Some(SlotContent::Item {
                id: 0x8000_0001,
                name: "Item 80000001".to_owned()
            })
        );
        assert_eq!(main.slots[1].content, None);
        assert_eq!(main.slots[0].key(KeyMode::Normal), Some(key(Key::Number1)));
        assert_eq!(main.slots[11].key(KeyMode::Magic), Some(shift(Key::Equal)));
        let spells = &layout.bars[0];
        assert_eq!(spells.slots[0].key(KeyMode::Normal), None);
        assert_eq!(spells.slots[0].key(KeyMode::Magic), Some(key(Key::Number1)));
        assert!(
            spells.slots[1]
                .content
                .as_ref()
                .is_some_and(SlotContent::is_spell)
        );
        assert!(layout.bars[2].vertical);
        assert_eq!(layout.bars[2].slots.len(), 3);
    }

    #[test]
    fn a_key_finds_its_slot_by_mode() {
        let layout = seeded();
        assert_eq!(
            layout.slot_for_key(key(Key::Number1), KeyMode::Normal),
            Some(SlotRef::new(1, 0))
        );
        assert_eq!(
            layout.slot_for_key(key(Key::Number1), KeyMode::Magic),
            Some(SlotRef::new(0, 0))
        );
        assert_eq!(
            layout.slot_for_key(shift(Key::Number1), KeyMode::Magic),
            Some(SlotRef::new(1, 0))
        );
        let mut hidden = layout.clone();
        hidden.bars[1].hidden = true;
        assert_eq!(
            hidden.slot_for_key(key(Key::Number1), KeyMode::Normal),
            None
        );
    }

    #[test]
    fn a_taken_key_swaps() {
        let mut layout = seeded();
        // Main slot 1 takes 2; slot 2 gets 1 back.
        layout.set_key(SlotRef::new(1, 0), KeyMode::Normal, Some(key(Key::Number2)));
        assert_eq!(
            layout
                .slot(SlotRef::new(1, 0))
                .unwrap()
                .key(KeyMode::Normal),
            Some(key(Key::Number2))
        );
        assert_eq!(
            layout
                .slot(SlotRef::new(1, 1))
                .unwrap()
                .key(KeyMode::Normal),
            Some(key(Key::Number1))
        );
        // The other mode is untouched.
        assert_eq!(
            layout.slot(SlotRef::new(1, 0)).unwrap().key(KeyMode::Magic),
            Some(shift(Key::Number1))
        );
        layout.set_key(SlotRef::new(1, 0), KeyMode::Normal, None);
        assert_eq!(
            layout
                .slot(SlotRef::new(1, 0))
                .unwrap()
                .key(KeyMode::Normal),
            None
        );
    }

    #[test]
    fn slots_swap_and_keys_stay() {
        let mut layout = seeded();
        layout.swap(SlotRef::new(1, 0), SlotRef::new(0, 5));
        assert!(layout.slot(SlotRef::new(1, 0)).unwrap().content.is_none());
        assert!(matches!(
            layout.slot(SlotRef::new(0, 5)).unwrap().content,
            Some(SlotContent::Item {
                id: 0x8000_0001,
                ..
            })
        ));
        assert_eq!(
            layout.slot(SlotRef::new(0, 5)).unwrap().key(KeyMode::Magic),
            Some(key(Key::Number6))
        );
    }

    #[test]
    fn a_spell_goes_to_a_spell_bar_and_an_item_to_another() {
        let layout = seeded();
        assert_eq!(layout.first_empty(true), Some(SlotRef::new(0, 2)));
        assert_eq!(layout.first_empty(false), Some(SlotRef::new(1, 1)));
        assert!(layout.holds(&SlotContent::Spell {
            id: 41,
            name: String::new()
        }));
        assert!(!layout.holds(&SlotContent::Item {
            id: 41,
            name: String::new()
        }));
    }

    #[test]
    fn slots_are_added_and_taken_off_but_one_stays() {
        let mut layout = BarLayout::default();
        let i = layout.new_bar(MAIN_PLACE);
        assert_eq!(layout.bars[i].name, "Bar 1");
        assert_eq!(layout.bars[i].slots.len(), NEW_BAR_SLOTS);
        layout.add_slot(i);
        assert_eq!(layout.bars[i].slots.len(), NEW_BAR_SLOTS + 1);
        for _ in 0..20 {
            layout.remove_last_slot(i);
        }
        assert_eq!(layout.bars[i].slots.len(), 1);
        assert_eq!(layout.new_bar(MAIN_PLACE), 1);
        assert_eq!(layout.bars[1].name, "Bar 2");
        let mut seeded = seeded();
        let i = seeded.new_bar(MAIN_PLACE);
        assert_eq!(seeded.bars[i].name, "Bar 1");
    }

    #[test]
    fn the_saved_form_reads_back_the_same() {
        let mut layout = seeded();
        layout.bars[1].locked = true;
        layout.bars[2].hidden = true;
        let data = layout.to_data();
        assert_eq!(BarLayout::from_data(&data), layout);
        assert_eq!(data.bars[1].slots[0].magic_key, "Shift+Number1");
    }

    #[test]
    fn a_place_keeps_to_its_third_of_the_screen() {
        let screen = (1440.0, 900.0);
        let size = bar_size(12, false);
        let x = (1440.0 - size.0) / 2.0;
        let place = BarPlace::at(x, 796.0, size, screen);
        assert_eq!(place.anchor, Anchor::Bottom);
        assert_eq!(place.top_left(size, screen), (x.round(), 796.0));
        // On a bigger screen it stays as far from the bottom, centred.
        let big = (1920.0, 1080.0);
        let (bx, by) = place.top_left(size, big);
        assert_eq!(by, 1080.0 - (900.0 - 796.0));
        assert!((bx - (1920.0 - size.0) / 2.0).abs() <= 1.0);
        assert_eq!(
            BarPlace::at(10.0, 10.0, size, screen).anchor,
            Anchor::TopLeft
        );
    }

    #[test]
    fn bars_are_sized_by_their_slots() {
        assert_eq!(bar_size(1, false), (48.0, 48.0));
        assert_eq!(bar_size(12, false), (8.0 + 480.0 + 33.0, 48.0));
        assert_eq!(bar_size(3, true), (48.0, 8.0 + 120.0 + 6.0));
        let bar = Bar::new("x", 3, MAIN_PLACE);
        assert_eq!(bar.slot_origin(2), (PAD + 2.0 * (SLOT + GAP), PAD));
    }
}
