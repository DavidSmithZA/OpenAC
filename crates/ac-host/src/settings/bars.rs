//! The modern interface's shortcut bars: any number of bars, each with its
//! place, its direction and its slots (an item or a spell, a key for most
//! modes and one for Magic). A character keeps a layout of its own or uses
//! the one shared by every character set to share it. Kept on this
//! computer, in section `modernBars`: member `shared` (a layout) and member
//! `characters` (each with `character`, `useShared` and `own`).
//!
//! The values are kept as the interface writes them (an anchor's name, a
//! key's stored name); the reader decides what an unknown one means.

use super::{SettingsStore, member, object_member, read_bool, read_float, read_string, set_member};
use crate::json::{Member, Node, Value};

/// Every character's bars.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SavedBars {
    /// The layout the characters set to share use.
    pub shared: Option<BarLayoutData>,
    /// Each character's choice and own layout.
    pub characters: Vec<CharacterBars>,
}

impl SavedBars {
    /// The entry of `character`, if it has one.
    #[must_use]
    pub fn character(&self, character: &str) -> Option<&CharacterBars> {
        self.characters.iter().find(|c| c.character == character)
    }

    /// The entry of `character`, made empty when it has none.
    pub fn character_mut(&mut self, character: &str) -> &mut CharacterBars {
        if let Some(i) = self
            .characters
            .iter()
            .position(|c| c.character == character)
        {
            return &mut self.characters[i];
        }
        self.characters.push(CharacterBars {
            character: character.to_owned(),
            ..CharacterBars::default()
        });
        let last = self.characters.len() - 1;
        &mut self.characters[last]
    }
}

/// One character's bars.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CharacterBars {
    /// Who: as the interface names a character (its world and name).
    pub character: String,
    /// It uses the shared layout.
    pub use_shared: bool,
    /// Its own layout (none: never made).
    pub own: Option<BarLayoutData>,
}

/// A layout: the bars.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BarLayoutData {
    /// The bars, in the order they were made.
    pub bars: Vec<BarData>,
}

/// One bar.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BarData {
    /// Its name.
    pub name: String,
    /// The screen point it is held to.
    pub anchor: String,
    /// Its offset from there.
    pub dx: f32,
    /// Its offset from there.
    pub dy: f32,
    /// It runs top to bottom.
    pub vertical: bool,
    /// It stays where it is (quick move and its slots' drags leave it).
    pub locked: bool,
    /// It is not shown.
    pub hidden: bool,
    /// Its slots.
    pub slots: Vec<SlotData>,
}

/// One slot.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SlotData {
    /// What it holds: `item`, `spell`, `recall`, or empty.
    pub kind: String,
    /// The item's object id or the spell's id.
    pub id: u32,
    /// The name it had when it was put there.
    pub name: String,
    /// Its key outside Magic.
    pub key: String,
    /// Its key in Magic.
    pub magic_key: String,
    /// A recall's command, sent as if typed in chat.
    pub command: String,
    /// A recall's icon, by name.
    pub icon: String,
}

impl SettingsStore {
    /// Every character's bars; none when the section is absent or
    /// unreadable.
    #[must_use]
    pub fn load_modern_bars(&self) -> SavedBars {
        let Ok(Some(section)) = self.section("modernBars") else {
            return SavedBars::default();
        };
        let shared = match member(&section, "shared").map(|n| &n.value) {
            Some(Value::Object(fields)) => Some(read_layout(fields)),
            _ => None,
        };
        let characters = match member(&section, "characters").map(|n| &n.value) {
            Some(Value::Array(entries)) => entries
                .iter()
                .filter_map(|node| match &node.value {
                    Value::Object(fields) => read_character(fields),
                    _ => None,
                })
                .collect(),
            _ => Vec::new(),
        };
        SavedBars { shared, characters }
    }

    /// Replaces every character's bars, keeping every other section.
    ///
    /// # Errors
    /// The file exists but cannot be read, or the write fails.
    pub fn save_modern_bars(&self, bars: &SavedBars) -> std::io::Result<()> {
        let mut root = self.load_mutable_root()?;
        let section = object_member(&mut root, "modernBars");
        set_member(
            section,
            "shared",
            bars.shared
                .as_ref()
                .map_or(Value::Null, |l| Value::Object(write_layout(l))),
        );
        let characters = bars
            .characters
            .iter()
            .map(|c| {
                let mut fields = Vec::new();
                set_member(&mut fields, "character", Value::String(c.character.clone()));
                set_member(&mut fields, "useShared", Value::Bool(c.use_shared));
                set_member(
                    &mut fields,
                    "own",
                    c.own
                        .as_ref()
                        .map_or(Value::Null, |l| Value::Object(write_layout(l))),
                );
                Node::made(Value::Object(fields))
            })
            .collect();
        set_member(section, "characters", Value::Array(characters));
        self.write_mutable_root(root)
    }
}

fn number(value: f32) -> Value {
    let value = if value.is_finite() { value } else { 0.0 };
    Value::Number(format!("{}", value.round() as i64))
}

fn write_layout(layout: &BarLayoutData) -> Vec<Member> {
    let bars = layout
        .bars
        .iter()
        .map(|bar| {
            let mut fields = Vec::new();
            set_member(&mut fields, "name", Value::String(bar.name.clone()));
            set_member(&mut fields, "anchor", Value::String(bar.anchor.clone()));
            set_member(&mut fields, "dx", number(bar.dx));
            set_member(&mut fields, "dy", number(bar.dy));
            set_member(&mut fields, "vertical", Value::Bool(bar.vertical));
            set_member(&mut fields, "locked", Value::Bool(bar.locked));
            set_member(&mut fields, "hidden", Value::Bool(bar.hidden));
            let slots = bar
                .slots
                .iter()
                .map(|slot| {
                    let mut s = Vec::new();
                    set_member(&mut s, "kind", Value::String(slot.kind.clone()));
                    set_member(&mut s, "id", Value::Number(slot.id.to_string()));
                    set_member(&mut s, "name", Value::String(slot.name.clone()));
                    set_member(&mut s, "key", Value::String(slot.key.clone()));
                    set_member(&mut s, "magicKey", Value::String(slot.magic_key.clone()));
                    if !slot.command.is_empty() {
                        set_member(&mut s, "command", Value::String(slot.command.clone()));
                        set_member(&mut s, "icon", Value::String(slot.icon.clone()));
                    }
                    Node::made(Value::Object(s))
                })
                .collect();
            set_member(&mut fields, "slots", Value::Array(slots));
            Node::made(Value::Object(fields))
        })
        .collect();
    let mut fields = Vec::new();
    set_member(&mut fields, "bars", Value::Array(bars));
    fields
}

fn read_layout(fields: &[Member]) -> BarLayoutData {
    let bars = match member(fields, "bars").map(|n| &n.value) {
        Some(Value::Array(entries)) => entries
            .iter()
            .filter_map(|node| match &node.value {
                Value::Object(fields) => Some(read_bar(fields)),
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    };
    BarLayoutData { bars }
}

fn read_bar(fields: &[Member]) -> BarData {
    let slots = match member(fields, "slots").map(|n| &n.value) {
        Some(Value::Array(entries)) => entries
            .iter()
            .map(|node| match &node.value {
                Value::Object(fields) => read_slot(fields),
                _ => SlotData::default(),
            })
            .collect(),
        _ => Vec::new(),
    };
    BarData {
        name: read_string(fields, "name", ""),
        anchor: read_string(fields, "anchor", ""),
        dx: read_float(fields, "dx", 0.0).unwrap_or(0.0),
        dy: read_float(fields, "dy", 0.0).unwrap_or(0.0),
        vertical: read_bool(fields, "vertical", false),
        locked: read_bool(fields, "locked", false),
        hidden: read_bool(fields, "hidden", false),
        slots,
    }
}

fn read_slot(fields: &[Member]) -> SlotData {
    let id = match member(fields, "id").map(|n| &n.value) {
        Some(Value::Number(raw)) => raw.parse::<u32>().unwrap_or(0),
        _ => 0,
    };
    SlotData {
        kind: read_string(fields, "kind", ""),
        id,
        name: read_string(fields, "name", ""),
        key: read_string(fields, "key", ""),
        magic_key: read_string(fields, "magicKey", ""),
        command: read_string(fields, "command", ""),
        icon: read_string(fields, "icon", ""),
    }
}

fn read_character(fields: &[Member]) -> Option<CharacterBars> {
    let character = read_string(fields, "character", "");
    if character.is_empty() {
        return None;
    }
    let own = match member(fields, "own").map(|n| &n.value) {
        Some(Value::Object(fields)) => Some(read_layout(fields)),
        _ => None,
    };
    Some(CharacterBars {
        character,
        use_shared: read_bool(fields, "useShared", false),
        own,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Temp(std::path::PathBuf);

    impl Temp {
        fn new() -> Self {
            let nanos = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.as_nanos());
            // Tests run side by side: a count keeps two in the same tick
            // apart.
            static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
            let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            let dir = std::env::temp_dir().join(format!(
                "openac-modern-bars-{}-{nanos}-{n}",
                std::process::id()
            ));
            std::fs::create_dir_all(&dir).unwrap();
            Self(dir)
        }

        fn store(&self) -> SettingsStore {
            SettingsStore::new(self.0.join("settings.json"))
        }
    }

    impl Drop for Temp {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn layout() -> BarLayoutData {
        BarLayoutData {
            bars: vec![BarData {
                name: "Main".to_owned(),
                anchor: "Bottom".to_owned(),
                dx: -12.0,
                dy: -40.0,
                vertical: false,
                locked: true,
                hidden: false,
                slots: vec![
                    SlotData {
                        kind: "item".to_owned(),
                        id: 0x8000_1234,
                        name: "Health Potion".to_owned(),
                        key: "Number1".to_owned(),
                        magic_key: "Shift+Number1".to_owned(),
                        ..SlotData::default()
                    },
                    SlotData {
                        kind: "recall".to_owned(),
                        name: "Driftnet".to_owned(),
                        command: "/dn".to_owned(),
                        icon: "flag".to_owned(),
                        ..SlotData::default()
                    },
                    SlotData::default(),
                ],
            }],
        }
    }

    #[test]
    fn nothing_saved_is_no_bars() {
        let temp = Temp::new();
        assert_eq!(temp.store().load_modern_bars(), SavedBars::default());
    }

    #[test]
    fn bars_come_back_as_saved_and_other_sections_stay() {
        let temp = Temp::new();
        let store = temp.store();
        store
            .save_plugin_ui(&crate::settings::PluginUiSettings {
                theme: "Moss".to_owned(),
                ..Default::default()
            })
            .unwrap();
        let mut saved = SavedBars {
            shared: Some(layout()),
            characters: Vec::new(),
        };
        let mine = saved.character_mut("Leafcull/Aerin Vale");
        mine.use_shared = false;
        mine.own = Some(layout());
        saved.character_mut("Leafcull/Osk").use_shared = true;
        store.save_modern_bars(&saved).unwrap();
        assert_eq!(store.load_modern_bars(), saved);
        assert_eq!(store.load_plugin_ui().0.theme, "Moss");
        assert!(saved.character("Leafcull/Osk").unwrap().use_shared);
        assert!(saved.character("Nobody").is_none());
    }
}
