//! The modern interface's own recalls: commands a server has that retail
//! did not (Dreamweave's `/dn`, say), each with a name and an icon, shown
//! on one server or on every server. Kept on this computer, in section
//! `modernRecalls`, member `recalls`, in the player's order.

use super::{SettingsStore, object_member, set_member};
use crate::json::{Member, Node, Value};

/// One recall of the player's own.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CustomRecall {
    /// What the list calls it.
    pub name: String,
    /// What it sends, as if typed in chat.
    pub command: String,
    /// Its icon's name, as saved; the reader decides what an unknown one
    /// means.
    pub icon: String,
    /// The server (world) it shows on; none: every server.
    pub server: Option<String>,
}

impl SettingsStore {
    /// The player's own recalls, in their order; none when the section is
    /// absent or unreadable (entries without a name or command are left
    /// out).
    #[must_use]
    pub fn load_custom_recalls(&self) -> Vec<CustomRecall> {
        let Ok(Some(section)) = self.section("modernRecalls") else {
            return Vec::new();
        };
        let Some(Value::Array(entries)) = section
            .iter()
            .find(|m| m.name == "recalls")
            .map(|m| &m.value.value)
        else {
            return Vec::new();
        };
        entries
            .iter()
            .filter_map(|node| match &node.value {
                Value::Object(fields) => read_recall(fields),
                _ => None,
            })
            .collect()
    }

    /// Replaces the player's own recalls, keeping every other section.
    ///
    /// # Errors
    /// The file exists but cannot be read, or the write fails.
    pub fn save_custom_recalls(&self, recalls: &[CustomRecall]) -> std::io::Result<()> {
        let mut root = self.load_mutable_root()?;
        let entries = recalls
            .iter()
            .map(|r| {
                let mut fields = Vec::new();
                set_member(&mut fields, "name", Value::String(r.name.clone()));
                set_member(&mut fields, "command", Value::String(r.command.clone()));
                set_member(&mut fields, "icon", Value::String(r.icon.clone()));
                set_member(
                    &mut fields,
                    "server",
                    r.server.clone().map_or(Value::Null, Value::String),
                );
                Node::made(Value::Object(fields))
            })
            .collect();
        let section = object_member(&mut root, "modernRecalls");
        set_member(section, "recalls", Value::Array(entries));
        self.write_mutable_root(root)
    }
}

fn read_recall(fields: &[Member]) -> Option<CustomRecall> {
    let text = |name: &str| {
        fields
            .iter()
            .find(|m| m.name == name)
            .and_then(|m| match &m.value.value {
                Value::String(s) => Some(s.clone()),
                _ => None,
            })
    };
    let name = text("name").filter(|s| !s.trim().is_empty())?;
    let command = text("command").filter(|s| !s.trim().is_empty())?;
    Some(CustomRecall {
        name,
        command,
        icon: text("icon").unwrap_or_default(),
        server: text("server").filter(|s| !s.is_empty()),
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
            // Tests run side by side and the clock can read the same for
            // two of them: a counter keeps their folders apart.
            static NEXT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
            let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            let dir = std::env::temp_dir().join(format!(
                "openac-custom-recalls-{}-{nanos}-{n}",
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

    #[test]
    fn nothing_saved_is_no_recalls() {
        let temp = Temp::new();
        assert!(temp.store().load_custom_recalls().is_empty());
    }

    #[test]
    fn recalls_come_back_in_order_and_other_sections_stay() {
        let temp = Temp::new();
        let store = temp.store();
        store
            .save_plugin_ui(&crate::settings::PluginUiSettings {
                theme: "Moss".to_owned(),
                ..Default::default()
            })
            .unwrap();
        let recalls = vec![
            CustomRecall {
                name: "Driftnet".to_owned(),
                command: "/dn".to_owned(),
                icon: "portal".to_owned(),
                server: Some("Dreamweave".to_owned()),
            },
            CustomRecall {
                name: "Bank".to_owned(),
                command: "@bank".to_owned(),
                icon: "coin".to_owned(),
                server: None,
            },
        ];
        store.save_custom_recalls(&recalls).unwrap();
        assert_eq!(store.load_custom_recalls(), recalls);
        assert_eq!(store.load_plugin_ui().0.theme, "Moss");
    }
}
