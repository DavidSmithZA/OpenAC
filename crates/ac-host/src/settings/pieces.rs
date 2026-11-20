//! The modern interface's HUD pieces the player hid or locked in place
//! (the shortcut bars keep their own). Kept on this computer, in section
//! `modernPieces`, members `hidden` and `locked`: lists of piece names.

use super::{SettingsStore, object_member, set_member};
use crate::json::{Member, Node, Value};

/// Which HUD pieces are hidden or locked, by name.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct HudPieces {
    /// Not shown (Edit layout brings them back).
    pub hidden: Vec<String>,
    /// Quick move leaves them where they are.
    pub locked: Vec<String>,
}

impl HudPieces {
    /// Piece `name` is hidden.
    #[must_use]
    pub fn is_hidden(&self, name: &str) -> bool {
        self.hidden.iter().any(|n| n == name)
    }

    /// Piece `name` is locked.
    #[must_use]
    pub fn is_locked(&self, name: &str) -> bool {
        self.locked.iter().any(|n| n == name)
    }

    /// Hides piece `name` (or shows it); true when that changed anything.
    pub fn set_hidden(&mut self, name: &str, on: bool) -> bool {
        set_in(&mut self.hidden, name, on)
    }

    /// Locks piece `name` (or unlocks it); true when that changed anything.
    pub fn set_locked(&mut self, name: &str, on: bool) -> bool {
        set_in(&mut self.locked, name, on)
    }
}

fn set_in(list: &mut Vec<String>, name: &str, on: bool) -> bool {
    let there = list.iter().position(|n| n == name);
    match (there, on) {
        (None, true) => {
            list.push(name.to_owned());
            true
        }
        (Some(i), false) => {
            list.remove(i);
            true
        }
        _ => false,
    }
}

impl SettingsStore {
    /// The hidden and locked HUD pieces; none when the section is absent
    /// or unreadable.
    #[must_use]
    pub fn load_hud_pieces(&self) -> HudPieces {
        let Ok(Some(section)) = self.section("modernPieces") else {
            return HudPieces::default();
        };
        HudPieces {
            hidden: names(&section, "hidden"),
            locked: names(&section, "locked"),
        }
    }

    /// Replaces the hidden and locked HUD pieces, keeping every other
    /// section.
    ///
    /// # Errors
    /// The file exists but cannot be read, or the write fails.
    pub fn save_hud_pieces(&self, pieces: &HudPieces) -> std::io::Result<()> {
        let mut root = self.load_mutable_root()?;
        let list = |names: &[String]| {
            Value::Array(
                names
                    .iter()
                    .map(|n| Node::made(Value::String(n.clone())))
                    .collect(),
            )
        };
        let section = object_member(&mut root, "modernPieces");
        set_member(section, "hidden", list(&pieces.hidden));
        set_member(section, "locked", list(&pieces.locked));
        self.write_mutable_root(root)
    }
}

fn names(section: &[Member], member: &str) -> Vec<String> {
    let Some(Value::Array(entries)) = section
        .iter()
        .find(|m| m.name == member)
        .map(|m| &m.value.value)
    else {
        return Vec::new();
    };
    entries
        .iter()
        .filter_map(|node| match &node.value {
            Value::String(s) if !s.is_empty() => Some(s.clone()),
            _ => None,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hidden_and_locked_pieces_come_back() {
        let dir = std::env::temp_dir().join(format!(
            "openac-hud-pieces-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.as_nanos())
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let store = SettingsStore::new(dir.join("settings.json"));
        assert_eq!(store.load_hud_pieces(), HudPieces::default());
        let mut pieces = HudPieces::default();
        assert!(pieces.set_hidden("modern-radar", true));
        assert!(!pieces.set_hidden("modern-radar", true));
        assert!(pieces.set_locked("modern-vitals", true));
        store.save_hud_pieces(&pieces).unwrap();
        let back = store.load_hud_pieces();
        assert!(back.is_hidden("modern-radar"));
        assert!(back.is_locked("modern-vitals"));
        assert!(!back.is_locked("modern-radar"));
        let mut back = back;
        assert!(back.set_hidden("modern-radar", false));
        assert!(back.hidden.is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
