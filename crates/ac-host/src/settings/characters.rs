//! What the modern character selection remembers of each character played
//! on this computer (OPENAC: the server sends only names): its level,
//! heritage and gender, where it was last, when it was last played and
//! how it looked. Kept in section `modernCharacters`, a member per
//! character ("world/name").

use super::{SettingsStore, object_member, set_member};
use crate::json::{Member, Node, Value};

/// A character's look as it was last seen in the world: its body and the
/// changes worn on it (clothes and armour included).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RememberedLook {
    /// The body (a setup).
    pub setup: u32,
    /// The palette the colour changes start from (0: none).
    pub base_palette: u32,
    /// Colour changes: (palette, first colour, colours), in order.
    pub palettes: Vec<(u32, u32, u32)>,
    /// Texture changes: (part, old texture, new texture), in order.
    pub textures: Vec<(u8, u32, u32)>,
    /// Part changes: (part, new model), in order.
    pub parts: Vec<(u8, u32)>,
}

/// One character as last played here.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RememberedCharacter {
    /// Its level (0: not known).
    pub level: u32,
    /// Its heritage (0: not known).
    pub heritage: u32,
    /// Its gender (0: not known).
    pub gender: u32,
    /// The place it was last (a town's name, or empty).
    pub place: String,
    /// When it was last played, seconds since 1970 (0: not known).
    pub played: u64,
    /// How it looked.
    pub look: Option<RememberedLook>,
}

/// Every remembered character, by key ("world/name").
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RememberedCharacters {
    /// The characters, in the order first remembered.
    pub by_character: Vec<(String, RememberedCharacter)>,
}

impl RememberedCharacters {
    /// Character `key`, when remembered.
    #[must_use]
    pub fn of(&self, key: &str) -> Option<&RememberedCharacter> {
        self.by_character
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, c)| c)
    }

    /// Remembers character `key` as `character`.
    pub fn set(&mut self, key: &str, character: RememberedCharacter) {
        match self.by_character.iter_mut().find(|(k, _)| k == key) {
            Some((_, c)) => *c = character,
            None => self.by_character.push((key.to_owned(), character)),
        }
    }
}

fn number(n: impl ToString) -> Value {
    Value::Number(n.to_string())
}

fn numbers(list: impl IntoIterator<Item = u64>) -> Value {
    Value::Array(list.into_iter().map(|n| Node::made(number(n))).collect())
}

impl SettingsStore {
    /// The remembered characters; none when the section is absent or
    /// unreadable.
    #[must_use]
    pub fn load_remembered_characters(&self) -> RememberedCharacters {
        let Ok(Some(section)) = self.section("modernCharacters") else {
            return RememberedCharacters::default();
        };
        RememberedCharacters {
            by_character: section
                .iter()
                .filter_map(|m| Some((m.name.clone(), read_character(m)?)))
                .collect(),
        }
    }

    /// Replaces the remembered characters, keeping every other section.
    ///
    /// # Errors
    /// The file exists but cannot be read, or the write fails.
    pub fn save_remembered_characters(&self, all: &RememberedCharacters) -> std::io::Result<()> {
        let mut root = self.load_mutable_root()?;
        let section = object_member(&mut root, "modernCharacters");
        section.clear();
        for (key, c) in &all.by_character {
            let mut fields = Vec::new();
            set_member(&mut fields, "level", number(c.level));
            set_member(&mut fields, "heritage", number(c.heritage));
            set_member(&mut fields, "gender", number(c.gender));
            set_member(&mut fields, "place", Value::String(c.place.clone()));
            set_member(&mut fields, "played", number(c.played));
            if let Some(look) = &c.look {
                let mut l = Vec::new();
                set_member(&mut l, "setup", number(look.setup));
                set_member(&mut l, "basePalette", number(look.base_palette));
                set_member(
                    &mut l,
                    "palettes",
                    numbers(
                        look.palettes
                            .iter()
                            .flat_map(|&(p, o, n)| [u64::from(p), u64::from(o), u64::from(n)]),
                    ),
                );
                set_member(
                    &mut l,
                    "textures",
                    numbers(look.textures.iter().flat_map(|&(part, old, new)| {
                        [u64::from(part), u64::from(old), u64::from(new)]
                    })),
                );
                set_member(
                    &mut l,
                    "parts",
                    numbers(
                        look.parts
                            .iter()
                            .flat_map(|&(part, model)| [u64::from(part), u64::from(model)]),
                    ),
                );
                set_member(&mut fields, "look", Value::Object(l));
            }
            set_member(section, key, Value::Object(fields));
        }
        self.write_mutable_root(root)
    }
}

fn field<'a>(fields: &'a [Member], name: &str) -> Option<&'a Value> {
    fields
        .iter()
        .find(|m| m.name == name)
        .map(|m| &m.value.value)
}

fn read_u64(fields: &[Member], name: &str) -> u64 {
    match field(fields, name) {
        Some(Value::Number(raw)) => raw.parse().unwrap_or(0),
        _ => 0,
    }
}

fn read_u32(fields: &[Member], name: &str) -> u32 {
    u32::try_from(read_u64(fields, name)).unwrap_or(0)
}

fn read_list(fields: &[Member], name: &str) -> Vec<u32> {
    let Some(Value::Array(items)) = field(fields, name) else {
        return Vec::new();
    };
    items
        .iter()
        .map(|n| match &n.value {
            Value::Number(raw) => raw.parse().unwrap_or(0),
            _ => 0,
        })
        .collect()
}

fn read_look(fields: &[Member]) -> Option<RememberedLook> {
    let setup = read_u32(fields, "setup");
    if setup == 0 {
        return None;
    }
    let part = |n: u32| u8::try_from(n).unwrap_or(0);
    Some(RememberedLook {
        setup,
        base_palette: read_u32(fields, "basePalette"),
        palettes: read_list(fields, "palettes")
            .chunks_exact(3)
            .map(|c| (c[0], c[1], c[2]))
            .collect(),
        textures: read_list(fields, "textures")
            .chunks_exact(3)
            .map(|c| (part(c[0]), c[1], c[2]))
            .collect(),
        parts: read_list(fields, "parts")
            .chunks_exact(2)
            .map(|c| (part(c[0]), c[1]))
            .collect(),
    })
}

fn read_character(member: &Member) -> Option<RememberedCharacter> {
    let Value::Object(fields) = &member.value.value else {
        return None;
    };
    Some(RememberedCharacter {
        level: read_u32(fields, "level"),
        heritage: read_u32(fields, "heritage"),
        gender: read_u32(fields, "gender"),
        place: match field(fields, "place") {
            Some(Value::String(s)) => s.clone(),
            _ => String::new(),
        },
        played: read_u64(fields, "played"),
        look: match field(fields, "look") {
            Some(Value::Object(l)) => read_look(l),
            _ => None,
        },
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
            let dir = std::env::temp_dir()
                .join(format!("openac-remembered-{}-{nanos}", std::process::id()));
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

    #[test]
    fn nothing_saved_remembers_no_one() {
        let temp = Temp::new();
        assert!(
            temp.store()
                .load_remembered_characters()
                .by_character
                .is_empty()
        );
    }

    #[test]
    fn characters_come_back_with_their_looks_and_other_sections_stay() {
        let temp = Temp::new();
        let store = temp.store();
        store
            .save_custom_recalls(&[crate::settings::CustomRecall {
                name: "Bank".to_owned(),
                command: "@bank".to_owned(),
                ..Default::default()
            }])
            .unwrap();
        let mut all = RememberedCharacters::default();
        all.set(
            "Leafcull/Aerin Vale",
            RememberedCharacter {
                level: 4783,
                heritage: 1,
                gender: 2,
                place: "Ayan Baqur".to_owned(),
                played: 1_800_000_000,
                look: Some(RememberedLook {
                    setup: 0x0200_004E,
                    base_palette: 0x0400_007E,
                    palettes: vec![(0x0400_1071, 24, 8), (0x0400_02BD, 0, 2048)],
                    textures: vec![(16, 0x0500_0001, 0x0500_0002)],
                    parts: vec![(16, 0x0100_0001), (9, 0x0100_1234)],
                }),
            },
        );
        all.set("Leafcull/Quill", RememberedCharacter::default());
        store.save_remembered_characters(&all).unwrap();
        assert_eq!(store.load_remembered_characters(), all);
        assert_eq!(store.load_custom_recalls().len(), 1);
        // Remembering again replaces, in place.
        let mut again = all.clone();
        again.set(
            "Leafcull/Quill",
            RememberedCharacter {
                level: 3,
                ..Default::default()
            },
        );
        assert_eq!(again.by_character.len(), 2);
        assert_eq!(again.of("Leafcull/Quill").map(|c| c.level), Some(3));
    }
}
