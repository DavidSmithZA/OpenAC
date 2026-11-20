//! The quests each character shows on screen in the modern interface.
//! Kept on this computer, in section `modernQuests`: a member per
//! character ("world/name") listing the contract ids shown.

use super::{SettingsStore, object_member, set_member};
use crate::json::{Member, Node, Value};

/// The quests shown on screen, by character key ("world/name").
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct QuestsOnScreen {
    /// Each character's shown contract ids, in the order shown.
    pub by_character: Vec<(String, Vec<u32>)>,
}

impl QuestsOnScreen {
    /// The contract ids character `key` shows.
    #[must_use]
    pub fn shown(&self, key: &str) -> &[u32] {
        self.by_character
            .iter()
            .find(|(k, _)| k == key)
            .map_or(&[], |(_, ids)| ids.as_slice())
    }

    /// Shows contract `id` for character `key` (or stops); true when that
    /// changed anything.
    pub fn set_shown(&mut self, key: &str, id: u32, on: bool) -> bool {
        let at = match self.by_character.iter().position(|(k, _)| k == key) {
            Some(i) => i,
            None => {
                if !on {
                    return false;
                }
                self.by_character.push((key.to_owned(), Vec::new()));
                self.by_character.len() - 1
            }
        };
        let ids = &mut self.by_character[at].1;
        let there = ids.iter().position(|i| *i == id);
        match (there, on) {
            (None, true) => ids.push(id),
            (Some(i), false) => {
                ids.remove(i);
            }
            _ => return false,
        }
        if ids.is_empty() {
            self.by_character.remove(at);
        }
        true
    }
}

impl SettingsStore {
    /// The quests shown on screen; none when the section is absent or
    /// unreadable.
    #[must_use]
    pub fn load_quests_on_screen(&self) -> QuestsOnScreen {
        let Ok(Some(section)) = self.section("modernQuests") else {
            return QuestsOnScreen::default();
        };
        QuestsOnScreen {
            by_character: section
                .iter()
                .filter_map(|m| ids(m).map(|ids| (m.name.clone(), ids)))
                .filter(|(_, ids)| !ids.is_empty())
                .collect(),
        }
    }

    /// Replaces the quests shown on screen, keeping every other section.
    ///
    /// # Errors
    /// The file exists but cannot be read, or the write fails.
    pub fn save_quests_on_screen(&self, quests: &QuestsOnScreen) -> std::io::Result<()> {
        let mut root = self.load_mutable_root()?;
        let section = object_member(&mut root, "modernQuests");
        section.clear();
        for (key, ids) in &quests.by_character {
            let list = Value::Array(
                ids.iter()
                    .map(|id| Node::made(Value::Number(id.to_string())))
                    .collect(),
            );
            set_member(section, key, list);
        }
        self.write_mutable_root(root)
    }
}

fn ids(member: &Member) -> Option<Vec<u32>> {
    let Value::Array(entries) = &member.value.value else {
        return None;
    };
    Some(
        entries
            .iter()
            .filter_map(|node| match &node.value {
                Value::Number(raw) => raw.parse::<u32>().ok(),
                _ => None,
            })
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_characters_shown_quests_come_back() {
        let dir = std::env::temp_dir().join(format!(
            "openac-quests-on-screen-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.as_nanos())
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let store = SettingsStore::new(dir.join("settings.json"));
        assert_eq!(store.load_quests_on_screen(), QuestsOnScreen::default());
        let mut quests = QuestsOnScreen::default();
        assert!(quests.set_shown("Coldeve/Aerin", 7, true));
        assert!(!quests.set_shown("Coldeve/Aerin", 7, true));
        assert!(quests.set_shown("Coldeve/Aerin", 9, true));
        assert!(quests.set_shown("Coldeve/Osk", 3, true));
        store.save_quests_on_screen(&quests).unwrap();
        let mut back = store.load_quests_on_screen();
        assert_eq!(back.shown("Coldeve/Aerin"), [7, 9]);
        assert_eq!(back.shown("Coldeve/Osk"), [3]);
        assert!(back.shown("Coldeve/Mira").is_empty());
        assert!(back.set_shown("Coldeve/Osk", 3, false));
        store.save_quests_on_screen(&back).unwrap();
        assert!(
            store
                .load_quests_on_screen()
                .shown("Coldeve/Osk")
                .is_empty()
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
