//! The map pins each character placed in the modern interface. Kept on
//! this computer, in section `modernPins`: a member per character
//! ("world/name") listing its pins, each with a name and map coordinates.

use super::{SettingsStore, object_member, set_member};
use crate::json::{Member, Node, Value};

/// One pin: its name and map coordinates (`x` east, `y` north).
#[derive(Debug, Clone, PartialEq)]
pub struct MapPin {
    /// What the player called it.
    pub name: String,
    /// East (+) or west.
    pub x: f64,
    /// North (+) or south.
    pub y: f64,
}

/// The pins, by character key ("world/name").
#[derive(Debug, Clone, PartialEq, Default)]
pub struct MapPins {
    /// Each character's pins, in the order placed.
    pub by_character: Vec<(String, Vec<MapPin>)>,
}

impl MapPins {
    /// Character `key`'s pins.
    #[must_use]
    pub fn of(&self, key: &str) -> &[MapPin] {
        self.by_character
            .iter()
            .find(|(k, _)| k == key)
            .map_or(&[], |(_, pins)| pins.as_slice())
    }

    /// Adds a pin for character `key`.
    pub fn add(&mut self, key: &str, pin: MapPin) {
        match self.by_character.iter_mut().find(|(k, _)| k == key) {
            Some((_, pins)) => pins.push(pin),
            None => self.by_character.push((key.to_owned(), vec![pin])),
        }
    }

    /// Takes pin `index` off character `key`'s list; true when there was
    /// one.
    pub fn remove(&mut self, key: &str, index: usize) -> bool {
        let Some(at) = self.by_character.iter().position(|(k, _)| k == key) else {
            return false;
        };
        let pins = &mut self.by_character[at].1;
        if index >= pins.len() {
            return false;
        }
        pins.remove(index);
        if pins.is_empty() {
            self.by_character.remove(at);
        }
        true
    }
}

impl SettingsStore {
    /// The map pins; none when the section is absent or unreadable.
    #[must_use]
    pub fn load_map_pins(&self) -> MapPins {
        let Ok(Some(section)) = self.section("modernPins") else {
            return MapPins::default();
        };
        MapPins {
            by_character: section
                .iter()
                .map(|m| (m.name.clone(), pins(m)))
                .filter(|(_, pins)| !pins.is_empty())
                .collect(),
        }
    }

    /// Replaces the map pins, keeping every other section.
    ///
    /// # Errors
    /// The file exists but cannot be read, or the write fails.
    pub fn save_map_pins(&self, all: &MapPins) -> std::io::Result<()> {
        let mut root = self.load_mutable_root()?;
        let section = object_member(&mut root, "modernPins");
        section.clear();
        for (key, pins) in &all.by_character {
            let list = Value::Array(
                pins.iter()
                    .map(|p| {
                        let mut members = Vec::new();
                        set_member(&mut members, "name", Value::String(p.name.clone()));
                        set_member(&mut members, "x", Value::Number(format!("{:.2}", p.x)));
                        set_member(&mut members, "y", Value::Number(format!("{:.2}", p.y)));
                        Node::made(Value::Object(members))
                    })
                    .collect(),
            );
            set_member(section, key, list);
        }
        self.write_mutable_root(root)
    }
}

fn pins(member: &Member) -> Vec<MapPin> {
    let Value::Array(entries) = &member.value.value else {
        return Vec::new();
    };
    entries
        .iter()
        .filter_map(|node| {
            let Value::Object(fields) = &node.value else {
                return None;
            };
            let get = |name: &str| {
                fields
                    .iter()
                    .find(|m| m.name == name)
                    .map(|m| &m.value.value)
            };
            let number = |name: &str| match get(name) {
                Some(Value::Number(raw)) => raw.parse::<f64>().ok(),
                _ => None,
            };
            let name = match get("name") {
                Some(Value::String(s)) if !s.is_empty() => s.clone(),
                _ => return None,
            };
            Some(MapPin {
                name,
                x: number("x")?,
                y: number("y")?,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_characters_pins_come_back() {
        let dir = std::env::temp_dir().join(format!(
            "openac-map-pins-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.as_nanos())
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let store = SettingsStore::new(dir.join("settings.json"));
        assert_eq!(store.load_map_pins(), MapPins::default());
        let mut pins = MapPins::default();
        pins.add(
            "Coldeve/Aerin",
            MapPin {
                name: "Mana forge".to_owned(),
                x: 33.9,
                y: 42.1,
            },
        );
        pins.add(
            "Coldeve/Aerin",
            MapPin {
                name: "Lugian ridge".to_owned(),
                x: -17.5,
                y: 57.2,
            },
        );
        store.save_map_pins(&pins).unwrap();
        let mut back = store.load_map_pins();
        assert_eq!(back.of("Coldeve/Aerin").len(), 2);
        assert_eq!(back.of("Coldeve/Aerin")[1].name, "Lugian ridge");
        assert!((back.of("Coldeve/Aerin")[1].x + 17.5).abs() < 1e-9);
        assert!(back.remove("Coldeve/Aerin", 0));
        assert!(!back.remove("Coldeve/Aerin", 5));
        store.save_map_pins(&back).unwrap();
        assert_eq!(store.load_map_pins().of("Coldeve/Aerin").len(), 1);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
