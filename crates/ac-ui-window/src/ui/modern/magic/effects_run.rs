//! The Effects window in the game window, in place of retail's positive,
//! negative and vitae panels: the HUD's counts and those panels' keys open
//! it on the right tab; it reads the enchantments again when they change
//! and each second for the times while it shows.

use std::collections::HashMap;

use ac_core::player::{AttributeKind, VitalKind};
use ac_ui::bundled_font::Weight;
use ac_ui::modern::Horizon;
use ac_ui::panels::items::emit;
use ac_ui::panels::{PanelOutput, panel_id};
use ac_ui::tree::ElementId;

use super::effects::{EffectsList, ReadFrom, Stat};
use super::effects_window::{
    EffectsExtras, EffectsFonts, EffectsRequest, EffectsWindow, HEIGHT, WIDTH,
};
use crate::ui::modern::{hud, kit};
use crate::ui::window::WindowUi;

/// The window's name.
pub const EFFECTS_WINDOW: &str = "modern-effects-window";

/// The Effects window's elements and what it remembers.
#[derive(Debug)]
pub struct EffectsUi {
    /// The frame (the window).
    pub frame: ElementId,
    /// The search field.
    pub search: ElementId,
    /// What the last read was made of: the enchantments' revision and the
    /// whole second.
    read_at: Option<(u64, i64)>,
}

impl WindowUi {
    /// The Effects window, hidden, in place of retail's effects and vitae
    /// panels.
    pub(in crate::ui) fn mount_modern_effects_window(&mut self) {
        let fonts = EffectsFonts {
            title: self.modern_font(Weight::SemiBold, 15.0),
            tab: self.modern_font(Weight::SemiBold, 12.5),
            badge: self.modern_font(Weight::SemiBold, 10.5),
            body: self.modern_font(Weight::Medium, 12.0),
            name: self.modern_font(Weight::SemiBold, 12.5),
            small: self.modern_font(Weight::Regular, 10.5),
            time: self.modern_font(Weight::SemiBold, 12.0),
            label: self.modern_font(Weight::SemiBold, 9.5),
            heading: self.modern_font(Weight::SemiBold, 14.0),
            detail: self.modern_font(Weight::Regular, 12.0),
            strong: self.modern_font(Weight::SemiBold, 12.0),
            big: self.modern_font(Weight::SemiBold, 20.0),
        };
        let field_font = self.modern_font(Weight::Regular, 12.5);
        let window = EffectsWindow::new(Horizon::BRONZE, fonts);
        let frame =
            match hud::mount_piece(&mut self.host.root, EFFECTS_WINDOW, (WIDTH, HEIGHT), window) {
                Ok(id) => id,
                Err(e) => {
                    self.notices.push(format!("ui: modern effects window: {e}"));
                    return;
                }
            };
        let ui = &mut self.host.root;
        if let Some(e) = ui.element_mut(frame) {
            e.draggable = true;
        }
        let search = kit::mount_search(
            ui,
            frame,
            EffectsWindow::search_rect(),
            field_font,
            &Horizon::BRONZE,
        );
        ui.hide_window(EFFECTS_WINDOW);
        let Some(modern) = self.modern.as_mut() else {
            return;
        };
        modern.effects_window = Some(EffectsUi {
            frame,
            search,
            read_at: None,
        });
        modern.replace(crate::ui::indicator_panels::POSITIVE_EFFECTS_WINDOW);
        modern.replace(crate::ui::indicator_panels::NEGATIVE_EFFECTS_WINDOW);
        modern.replace(crate::ui::indicator_panels::VITAE_WINDOW);
    }

    /// A panel toggled: the helpful, harmful and vitae panels open (or
    /// close) the Effects window on their tab. False when it is not one of
    /// them or the window is not mounted.
    pub(in crate::ui) fn modern_effects_toggle(&mut self, panel: u32, close_only: bool) -> bool {
        let harmful = match panel {
            panel_id::POSITIVE_EFFECTS => false,
            panel_id::NEGATIVE_EFFECTS | panel_id::VITAE => true,
            _ => return false,
        };
        let Some(frame) = self
            .modern
            .as_ref()
            .and_then(|m| m.effects_window.as_ref())
            .map(|w| w.frame)
        else {
            return false;
        };
        let ui = &mut self.host.root;
        let open = ui.is_window_visible(EFFECTS_WINDOW);
        let on_tab = ui
            .widget::<EffectsWindow>(frame)
            .is_some_and(|w| w.harmful == harmful);
        if close_only || (open && on_tab) {
            if open && on_tab {
                ui.hide_window(EFFECTS_WINDOW);
            }
            return true;
        }
        if let Some(w) = ui.widget_mut::<EffectsWindow>(frame) {
            w.show_tab(harmful);
        }
        if !open {
            ui.show_window(EFFECTS_WINDOW);
            if let Some(w) = self.modern.as_mut().and_then(|m| m.effects_window.as_mut()) {
                w.read_at = None;
            }
        }
        self.host.root.bring_to_front(frame);
        true
    }

    /// Each frame while it shows: the effects read again when they changed
    /// and each second; the search; what the frame asked for.
    pub(in crate::ui) fn tick_modern_effects_window(&mut self) {
        let Some((frame, search)) = self
            .modern
            .as_ref()
            .and_then(|m| m.effects_window.as_ref())
            .map(|w| (w.frame, w.search))
        else {
            return;
        };
        let open = self.host.root.is_window_visible(EFFECTS_WINDOW);
        if !open {
            if self.host.root.keyboard_focus() == Some(search) {
                self.host.root.set_keyboard_focus(None);
            }
            return;
        }
        let asks = {
            let ui = &mut self.host.root;
            let typed = kit::field_text(ui, search);
            let focused = ui.keyboard_focus() == Some(search);
            ui.widget_mut::<EffectsWindow>(frame)
                .map(|w| {
                    w.search = typed;
                    w.searching = focused;
                    std::mem::take(&mut w.requests)
                })
                .unwrap_or_default()
        };
        for ask in asks {
            match ask {
                EffectsRequest::Close => {
                    self.host.root.hide_window(EFFECTS_WINDOW);
                }
                EffectsRequest::CastAgain(spell) => {
                    if !self.modern_cast(spell) {
                        self.notices
                            .push("Go into Magic mode, or hold a caster, to cast.".to_owned());
                    }
                }
                EffectsRequest::ShowItem(item) => {
                    emit(&mut self.host.root, PanelOutput::SelectObject(item));
                    self.panels
                        .set_visibility(&mut self.host.root, panel_id::INVENTORY, true);
                }
                EffectsRequest::Settings => {
                    let _ = self.panels.toggle(&mut self.host.root, panel_id::OPTIONS);
                }
            }
        }
        self.read_modern_effects_window(frame);
    }

    /// The effects read again when the enchantments changed or a second
    /// went by.
    fn read_modern_effects_window(&mut self, frame: ElementId) {
        let now = self.facts.session_now().unwrap_or(0.0);
        let Some(numbers) = self.facts.numbers() else {
            return;
        };
        let Some(book) = numbers.spellbook() else {
            return;
        };
        let key = Some((book.enchantment_revision(), now.floor() as i64));
        let fresh = self
            .modern
            .as_ref()
            .and_then(|m| m.effects_window.as_ref())
            .is_some_and(|w| w.read_at == key);
        if fresh {
            return;
        }
        let player = self.facts.state.player;
        let objects = &self.facts.objects;
        let name_of = |id: u32| objects.get(id).map(|o| o.name.clone());
        let value_of = |stat: Stat| -> Option<i32> {
            match stat {
                Stat::Attribute(id) => AttributeKind::ALL
                    .into_iter()
                    .find(|k| k.id() == id)
                    .and_then(|k| numbers.get_effective_attribute(k)),
                Stat::Vital(key) => {
                    let kind = match key {
                        1 => VitalKind::Health,
                        3 => VitalKind::Stamina,
                        5 => VitalKind::Mana,
                        _ => return None,
                    };
                    numbers
                        .get_max_approx(kind)
                        .and_then(|v| i32::try_from(v).ok())
                }
                Stat::Skill(id) => numbers.get_skill_value(id, None).map(|v| v.effective_level),
            }
        };
        let list = EffectsList::read(&ReadFrom {
            book,
            now,
            player,
            name_of: &name_of,
            value_of: &value_of,
        });
        let mut icons = HashMap::new();
        for e in list.helpful.iter().chain(&list.harmful) {
            icons.entry(e.spell_id).or_insert_with(|| {
                ac_ui::icons::IconSource::spell_icon(&mut self.icons, e.spell_id)
            });
        }
        let known: Vec<u32> = list
            .helpful
            .iter()
            .chain(&list.harmful)
            .map(|e| e.spell_id)
            .filter(|s| book.knows(*s))
            .collect();
        let vitae_experience = list.vitae.map(|v| {
            i64::from(ac_ui::panels::indicators::vitae_experience_to_recover(
                v,
                &self.facts.objects,
                player,
            ))
        });
        let known: HashMap<u32, String> = known
            .into_iter()
            .map(|s| (s, self.modern_bar_spot(s)))
            .collect();
        let (show_times, soon) = self
            .modern
            .as_ref()
            .and_then(|m| m.effects.as_ref())
            .map_or((true, hud::effects::DEFAULT_SOON_SECONDS), |fx| {
                (fx.show_times, fx.soon)
            });
        if let Some(w) = self.host.root.widget_mut::<EffectsWindow>(frame) {
            w.list = list;
            w.extras = EffectsExtras {
                icons,
                known,
                vitae_experience,
                show_times,
                soon,
            };
        }
        if let Some(w) = self.modern.as_mut().and_then(|m| m.effects_window.as_mut()) {
            w.read_at = key;
        }
    }
}
