//! The modern chat's tabs: each one's name, the text classes it shows, the
//! channel it talks on, and whether it is popped out into a window of its
//! own (and how see-through that window is). Kept on this computer, in
//! section `modernChat`, member `tabs`, in the player's order.

use super::{SettingsStore, member, object_member, read_bool, read_float, read_string, set_member};
use crate::json::{Member, Node, Value};

/// One saved chat tab.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ChatTabData {
    /// Its stable number (its window's name is made from it).
    pub id: u32,
    /// What it is called.
    pub name: String,
    /// The text classes it shows, bit `n` = class `n`.
    pub filter: u64,
    /// The channel it talks on, by name (empty: it keeps the last one).
    pub talk: String,
    /// It is popped out into its own window.
    pub window: bool,
    /// Its window's background opacity, 0..=1.
    pub opacity: f32,
    /// Where its window was left (its top left), once it has been placed.
    pub place: Option<(f32, f32)>,
}

impl SettingsStore {
    /// The saved chat tabs; none when the section is absent or unreadable.
    #[must_use]
    pub fn load_chat_tabs(&self) -> Vec<ChatTabData> {
        let Ok(Some(section)) = self.section("modernChat") else {
            return Vec::new();
        };
        let Some(Value::Array(entries)) = member(&section, "tabs").map(|n| &n.value) else {
            return Vec::new();
        };
        entries
            .iter()
            .filter_map(|node| match &node.value {
                Value::Object(fields) => read_tab(fields),
                _ => None,
            })
            .collect()
    }

    /// Replaces the saved chat tabs, keeping every other section.
    ///
    /// # Errors
    /// The file exists but cannot be read, or the write fails.
    pub fn save_chat_tabs(&self, tabs: &[ChatTabData]) -> std::io::Result<()> {
        let mut root = self.load_mutable_root()?;
        let entries = tabs
            .iter()
            .map(|t| {
                let mut f = Vec::new();
                set_member(&mut f, "id", Value::Number(t.id.to_string()));
                set_member(&mut f, "name", Value::String(t.name.clone()));
                // As text: a 64-bit mask does not survive a JSON number.
                set_member(&mut f, "filter", Value::String(format!("{:x}", t.filter)));
                set_member(&mut f, "talk", Value::String(t.talk.clone()));
                set_member(&mut f, "window", Value::Bool(t.window));
                set_member(
                    &mut f,
                    "opacity",
                    Value::Number(format!("{:.2}", t.opacity.clamp(0.0, 1.0))),
                );
                if let Some((x, y)) = t.place {
                    set_member(&mut f, "x", Value::Number(format!("{}", x.round() as i64)));
                    set_member(&mut f, "y", Value::Number(format!("{}", y.round() as i64)));
                }
                Node::made(Value::Object(f))
            })
            .collect();
        let section = object_member(&mut root, "modernChat");
        set_member(section, "tabs", Value::Array(entries));
        self.write_mutable_root(root)
    }
}

fn read_tab(fields: &[Member]) -> Option<ChatTabData> {
    let name = read_string(fields, "name", "");
    if name.trim().is_empty() {
        return None;
    }
    let id = match member(fields, "id").map(|n| &n.value) {
        Some(Value::Number(raw)) => raw.parse::<u32>().ok()?,
        _ => return None,
    };
    let filter = u64::from_str_radix(&read_string(fields, "filter", "0"), 16).unwrap_or(0);
    Some(ChatTabData {
        id,
        name,
        filter,
        talk: read_string(fields, "talk", ""),
        window: read_bool(fields, "window", false),
        opacity: read_float(fields, "opacity", 0.62).unwrap_or(0.62),
        place: match (
            read_float(fields, "x", f32::NAN),
            read_float(fields, "y", f32::NAN),
        ) {
            (Ok(x), Ok(y)) if x.is_finite() && y.is_finite() => Some((x, y)),
            _ => None,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tabs_come_back_in_order_with_their_whole_filters() {
        let dir = std::env::temp_dir().join(format!(
            "openac-chat-tabs-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.as_nanos())
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let store = SettingsStore::new(dir.join("settings.json"));
        assert!(store.load_chat_tabs().is_empty());
        let tabs = vec![
            ChatTabData {
                id: 1,
                name: "Main".to_owned(),
                filter: 0xFFFF_FFFF_FBFF_FFFF,
                talk: String::new(),
                window: false,
                opacity: 0.62,
                place: None,
            },
            ChatTabData {
                id: 7,
                name: "Trade watch".to_owned(),
                filter: 0x3800_0008,
                talk: "Trade".to_owned(),
                window: true,
                opacity: 0.78,
                place: Some((520.0, 140.0)),
            },
        ];
        store.save_chat_tabs(&tabs).unwrap();
        assert_eq!(store.load_chat_tabs(), tabs);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
