//! A plugin's notes on items and its part of an item window's foot
//! (API 1.15) through the SDK, against the fake host that keeps them as
//! the client does: the player's choices come back as events.

use openac_plugin::testing::FakeHost;
use openac_plugin::ui::{ItemWindow, NoteTone, foot_button, item_note, menu_entry};
use openac_plugin::{Event, Host, Plugin, StartInfo};

#[derive(Default)]
struct Looter {
    heard: Vec<String>,
}

impl Plugin for Looter {
    fn enable(&mut self, _host: &mut Host<'_>, _info: &StartInfo) -> Result<(), String> {
        Ok(())
    }

    fn on_event(&mut self, host: &mut Host<'_>, event: Event) {
        match event {
            Event::ItemMenuChosen(choice) => {
                self.heard
                    .push(format!("menu {} on {:x}", choice.id, choice.object_id));
                // Its notes change with what the player said.
                let _ = host.set_item_notes(&[item_note(choice.object_id, "Skip", NoteTone::Bad)]);
            }
            Event::FootButtonPressed(press) => {
                self.heard
                    .push(format!("foot {} in {:?}", press.id, press.window));
            }
            _ => {}
        }
    }
}

#[test]
fn a_plugin_notes_items_and_hears_its_menu_entries_and_foot_buttons() {
    let mut host = FakeHost::new("example.looter");
    let mut plugin = Looter::default();
    host.enable(&mut plugin).unwrap();
    let sword = 0x8000_0105;
    host.with_host(|h| {
        let mut keep = item_note(sword, "Keep", NoteTone::Good);
        keep.lines = vec!["Rule: Sets 5+".into()];
        keep.menu = vec![menu_entry("never", "Never loot this")];
        assert!(h.set_item_notes(&[keep]));
        assert!(h.set_window_foot(
            ItemWindow::Loot,
            "1 kept",
            &[foot_button("loot-kept", "Loot 1 kept")]
        ));
        // Two notes on one item: refused, the first set stays.
        assert!(!h.set_item_notes(&[
            item_note(1, "A", NoteTone::Plain),
            item_note(1, "B", NoteTone::Plain)
        ]));
    });
    let notes = host.ui.item_notes("example.looter");
    assert_eq!(notes.len(), 1);
    assert_eq!(notes[0].mark, "Keep");
    assert_eq!(notes[0].tone, ac_plugin_ui::NoteTone::Good);
    assert_eq!(notes[0].lines, ["Rule: Sets 5+"]);

    host.ui_input(
        &mut plugin,
        ac_plugin_ui::UiInput::ItemMenu {
            owner: "example.looter".into(),
            id: "never".into(),
            object: sword,
        },
    );
    host.ui_input(
        &mut plugin,
        ac_plugin_ui::UiInput::FootButton {
            owner: "example.looter".into(),
            window: ac_plugin_ui::ItemWindow::Loot,
            id: "loot-kept".into(),
        },
    );
    assert_eq!(
        plugin.heard,
        ["menu never on 80000105", "foot loot-kept in Loot"]
    );
    assert_eq!(host.ui.item_notes("example.looter")[0].mark, "Skip");
}
