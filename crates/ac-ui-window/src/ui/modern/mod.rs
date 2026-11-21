//! OpenAC's modern interface, mounted over the retail one while the
//! launcher asks for it.
//!
//! Each modern piece names the retail windows it stands in for
//! ([`ModernLayer::replace`]); once the layer is mounted those are taken
//! down ([`ModernLayer::take_down`]): unregistered, their controllers
//! dropped and their elements freed, so nothing retail is mounted in game.
//! The window layout persistence only follows registered windows, so the
//! player's saved retail layout is kept for when Retail is picked again.
//!
//! Modern windows have names of their own, so their layout is saved apart
//! from the retail windows'; where each one first appears is in
//! [`places`].

pub mod barber;
pub mod bars;
pub mod character;
pub mod chat;
pub mod chess;
pub mod dealing;
pub mod dialogs;
pub mod fonts;
pub mod house;
pub mod hud;
pub mod items;
pub mod journal;
pub mod kit;
pub mod magic;
pub mod options;
pub mod places;
pub mod pregame;
pub mod social;
pub mod world;

use std::sync::Arc;

use ac_ui::bundled_font::Weight;
use ac_ui::geometry::AnchorEdges;
use ac_ui::layout_persistence::LayoutPersistence;
use ac_ui::modern::Horizon;
use ac_ui::modern::hover::HoverPane;
use ac_ui::modern::quick_move::QuickMoveOutlines;
use ac_ui::tree::Element;
use ac_ui::{ElementId, Modifiers, UiFont, UiHost};

use super::window::WindowUi;

/// The modern interface of one game window.
#[derive(Debug, Default)]
pub struct ModernLayer {
    /// The retail windows the built modern pieces stand in for.
    replaced: Vec<&'static str>,
    /// The Inter fonts baked so far.
    pub fonts: fonts::HorizonFonts,
    /// The layer quick move outlines the pieces in.
    pub outlines: Option<ElementId>,
    /// The vitals.
    pub vitals: Option<ElementId>,
    /// The target line (and the status messages under it).
    pub target: Option<ElementId>,
    /// The low-health glow round the screen's edge.
    pub edge_glow: Option<ElementId>,
    /// The revision of the last cast answer looked at.
    seen_cast: Option<u64>,
    /// The effects counts and status chips, top left.
    pub effects: Option<EffectsHud>,
    /// The radar, top right.
    pub radar: Option<ElementId>,
    /// The connection card the status line opens.
    pub connection_card: Option<ElementId>,
    /// When the round trip was last asked for (the window's clock).
    last_ping: Option<f64>,
    /// The card was open last frame.
    card_was_open: bool,
    /// The dialog that adds or changes a recall of the player's own.
    pub recall_dialog: Option<ElementId>,
    /// The player's own recalls (all servers), loaded once.
    own_recalls: Option<Vec<ac_host::settings::CustomRecall>>,
    /// The combat strip (and the jump meter), above the shortcut bars.
    pub combat: Option<ElementId>,
    /// The panel strip, bottom right: the band and its buttons in
    /// [`hud::panels::Panel::ALL`]'s order.
    pub panels: Option<(ElementId, [ElementId; 8])>,
    /// The player's own shortcut bars.
    pub bars: Option<bars::run::BarsUi>,
    /// The HUD pieces the player hid or locked (read once).
    pub pieces: Option<ac_host::settings::HudPieces>,
    /// The chat.
    pub chat: Option<chat::run::ChatUi>,
    /// The inventory.
    pub inventory: Option<items::run::InventoryUi>,
    /// The Inspect sheet.
    pub inspect: Option<items::inspect::InspectUi>,
    /// The Loot window.
    pub loot: Option<items::loot::LootUi>,
    /// The Effects window.
    pub effects_window: Option<magic::effects_run::EffectsUi>,
    /// The Spellbook.
    pub spellbook: Option<magic::spellbook_run::BookUi>,
    /// The Character window.
    pub character: Option<character::run::CharacterUi>,
    /// The Options window.
    pub options: Option<options::run::OptionsUi>,
    /// The Vendor window.
    pub vendor: Option<dealing::vendor_run::VendorUi>,
    /// The Trade window.
    pub trade: Option<dealing::trade_run::TradeUi>,
    /// The Salvage window.
    pub salvage: Option<dealing::salvage_run::SalvageUi>,
    /// The Social window and the fellows on screen.
    pub social: Option<social::run::SocialUi>,
    /// The Journal and the quests on screen.
    pub journal: Option<journal::run::JournalUi>,
    /// The book reader.
    pub book: Option<journal::book::BookUi>,
    /// The House window and the house sale window.
    pub house: Option<house::run::HouseUi>,
    /// The full-screen map.
    pub map: Option<world::run::MapUi>,
    /// The Chess window.
    pub chess: Option<chess::window::ChessUi>,
    /// The Barber window.
    pub barber: Option<barber::window::BarberUi>,
    /// The pre-game screens (connecting, selection, credits, creation).
    pub pregame: Option<pregame::PreGameUi>,
    /// The player as the radar last read them (the map's arrow).
    pub last_player: Option<ac_ui::panels::radar::RadarPlayer>,
    /// The player's vitals as last fed (the map's bar).
    pub last_vitals: Option<ac_ui::panels::vitals::VitalsView>,
    /// The question card and the invitations under the target.
    pub questions: Option<dialogs::run::QuestionsUi>,
    /// Packs used anywhere since the inventory last looked.
    pub opened_packs: Vec<u32>,
    /// The selected stack's split quantity (the target line shows it;
    /// drags move that many).
    pub split: ac_ui::panels::items::SelectionSplit,
    /// The games as the Chess window follows them (a game ended by
    /// resigning or with no result puts it away).
    pub chess_follow: ac_ui::panels::mini_game::MiniGameFollow,
}

/// The effects counts and status chips, and the pane their counts open.
#[derive(Debug)]
pub struct EffectsHud {
    /// The row holding the chips (the HUD piece).
    pub row: ElementId,
    /// The chips, in [`hud::effects::ChipKind::ALL`]'s order.
    pub chips: [ElementId; 5],
    /// The pane the helpful and harmful counts open.
    pub hover: HoverPane,
    /// The list inside it.
    pub list: ElementId,
    /// The effects as last read.
    pub now: hud::effects::EffectsOnYou,
    /// Times shown (the "show spell durations" option).
    pub show_times: bool,
    /// What the last read was made of: the enchantments' revision and
    /// the whole second.
    read_at: Option<(u64, i64)>,
    /// How long before a helpful effect ends its count warns, seconds (0:
    /// never; Options sets it).
    pub soon: f64,
}

impl EffectsHud {
    /// The chip of `kind`.
    #[must_use]
    pub fn chip(&self, kind: hud::effects::ChipKind) -> ElementId {
        let index = hud::effects::ChipKind::ALL
            .iter()
            .position(|k| *k == kind)
            .unwrap_or(0);
        self.chips[index]
    }
}

impl ModernLayer {
    /// The layer with nothing built in it yet: every retail window shows.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The retail windows the layer stands in for.
    #[must_use]
    pub fn replaced(&self) -> &[&'static str] {
        &self.replaced
    }

    /// Stands in for retail window `name` from now on.
    pub fn replace(&mut self, name: &'static str) {
        if !self.replaced.contains(&name) {
            self.replaced.push(name);
        }
    }

    /// Where the modern windows first go.
    pub fn place_windows(layouts: &mut LayoutPersistence) {
        layouts.set_default_places(Box::new(|name, (w, h)| {
            places::default_place(name).map(|place| {
                let rect = place.rect((w as f32, h as f32));
                (rect.x, rect.y)
            })
        }));
    }

    /// Takes every replaced retail window down for good; returns the names
    /// taken down. Called once, after the modern pieces are mounted.
    pub fn take_down(&self, host: &mut UiHost) -> Vec<&'static str> {
        self.replaced
            .iter()
            .copied()
            .filter(|name| host.remove_window(name))
            .collect()
    }
}

/// The sizes of the vitals' words and values, and of the numbers that fall
/// off them, pixels to the em.
const VITALS_LABEL_SIZE: f32 = 11.0;
const VITALS_NUMBER_SIZE: f32 = 14.0;

/// The sizes of the target's name, its share, and the line's small text.
const TARGET_NAME_SIZE: f32 = 14.0;
const TARGET_SHARE_SIZE: f32 = 12.0;
const TARGET_SMALL_SIZE: f32 = 11.0;

/// The effects chips' counts, words and small text, and the pane's
/// headings, names and captions, pixels to the em.
const EFFECTS_FIGURE_SIZE: f32 = 13.0;
const EFFECTS_WORDS_SIZE: f32 = 12.0;
const EFFECTS_SMALL_SIZE: f32 = 11.0;
const EFFECTS_LABEL_SIZE: f32 = 9.5;
const EFFECTS_NAME_SIZE: f32 = 12.0;
const EFFECTS_CAPTION_SIZE: f32 = 10.5;

/// The radar's pill, letters and chip; its ring labels; its coordinates
/// and names, pixels to the em.
const RADAR_BOLD_SIZE: f32 = 11.0;
const RADAR_TINY_SIZE: f32 = 8.0;
const RADAR_SMALL_SIZE: f32 = 11.0;

/// The Recall list's names and small text, pixels to the em.
const RECALL_NAME_SIZE: f32 = 12.5;
const RECALL_SMALL_SIZE: f32 = 11.0;

/// The combat strip's mode, its heights and ammunition, and its Power or
/// Accuracy, pixels to the em.
const COMBAT_MODE_SIZE: f32 = 12.0;
const COMBAT_SMALL_SIZE: f32 = 11.0;
const COMBAT_WORD_SIZE: f32 = 10.5;

/// The panel strip's drop note, pixels to the em.
const PANEL_NOTE_SIZE: f32 = 11.5;

/// The connection card's title and body, pixels to the em.
const CARD_TITLE_SIZE: f32 = 13.5;
const CARD_BODY_SIZE: f32 = 12.0;

/// An item's mana now and at most (property ints).
const ITEM_CUR_MANA: u32 = 107;
const ITEM_MAX_MANA: u32 = 108;

/// The server's answer to a cast the mana could not pay for ("You don't
/// have enough Mana to cast this spell.").
pub const NOT_ENOUGH_MANA: u32 = 0x401;

/// The key that, held, lets modern pieces be moved whatever the lock says
/// (Options › Controls picks it).
#[must_use]
pub fn quick_move_modifier(key: ac_host::settings::QuickMoveKey) -> Modifiers {
    match key {
        ac_host::settings::QuickMoveKey::Ctrl => Modifiers::CTRL,
        ac_host::settings::QuickMoveKey::Alt => Modifiers::ALT,
        ac_host::settings::QuickMoveKey::Cmd => Modifiers::WIN,
    }
}

/// The hover labels' words, pixels to the em.
const TIP_SIZE: f32 = 12.0;

/// The size of the names quick move shows, pixels to the em.
const QUICK_MOVE_NAME_SIZE: f32 = 11.0;

impl WindowUi {
    /// Takes down the retail windows the modern pieces stand in for: they
    /// are no longer mounted, fed or shown.
    pub(super) fn take_down_replaced(&mut self) {
        let Some(modern) = self.modern.as_ref() else {
            return;
        };
        let gone = modern.take_down(&mut self.host);
        self.mounted.retain(|name| !gone.contains(name));
        self.stale.retain(|name| !gone.contains(name));
    }

    /// The HUD pieces built so far, each standing in for its retail
    /// windows.
    pub(super) fn mount_hud(&mut self) {
        let label = self.modern_font(Weight::SemiBold, VITALS_LABEL_SIZE);
        let number = self.modern_font(Weight::SemiBold, VITALS_NUMBER_SIZE);
        let glow = hud::edge_glow::EdgeGlow::mount(&mut self.host.root);
        let vitals = hud::mount_piece(
            &mut self.host.root,
            hud::VITALS,
            (hud::vitals::WIDTH, hud::vitals::BAR_HEIGHT),
            hud::vitals::VitalsHud::new(Horizon::current(), label, number),
        );
        let fonts = (
            self.modern_font(Weight::SemiBold, TARGET_NAME_SIZE),
            self.modern_font(Weight::SemiBold, TARGET_SHARE_SIZE),
            self.modern_font(Weight::SemiBold, TARGET_SMALL_SIZE),
        );
        let target = hud::mount_piece(
            &mut self.host.root,
            hud::TARGET,
            (hud::target::WIDTH, hud::target::HEIGHT),
            hud::target::TargetLine::new(Horizon::current(), fonts.0, fonts.1, fonts.2.clone()),
        );
        if let Ok(id) = &target {
            hud::target::TargetLine::mount_split_field(&mut self.host.root, *id, fonts.2.clone());
        }
        let effects = self.mount_modern_effects();
        let card_fonts = hud::connection::CardFonts {
            title: self.modern_font(Weight::SemiBold, CARD_TITLE_SIZE),
            body: self.modern_font(Weight::Medium, CARD_BODY_SIZE),
            caption: self.modern_font(Weight::Regular, EFFECTS_CAPTION_SIZE),
        };
        let radar_fonts = hud::radar::RadarFonts {
            bold: self.modern_font(Weight::SemiBold, RADAR_BOLD_SIZE),
            tiny: self.modern_font(Weight::Regular, RADAR_TINY_SIZE),
            small: self.modern_font(Weight::Medium, RADAR_SMALL_SIZE),
        };
        let radar = hud::mount_piece(
            &mut self.host.root,
            hud::RADAR,
            (hud::radar::WIDTH, hud::radar::HEIGHT),
            hud::radar::RadarRing::new(radar_fonts),
        );
        let recall_fonts = hud::recall::RecallFonts {
            name: self.modern_font(Weight::SemiBold, RECALL_NAME_SIZE),
            small: self.modern_font(Weight::Regular, RECALL_SMALL_SIZE),
            label: self.modern_font(Weight::SemiBold, EFFECTS_LABEL_SIZE),
        };
        let dialog_fonts = hud::recall_dialog::DialogFonts {
            title: self.modern_font(Weight::SemiBold, CARD_BODY_SIZE),
            body: self.modern_font(Weight::Medium, CARD_BODY_SIZE),
            caption: self.modern_font(Weight::Regular, EFFECTS_CAPTION_SIZE),
        };
        let mut recall_dialog = None;
        if let Ok(radar) = &radar {
            let ui = &mut self.host.root;
            let element = Element::at(0.0, 0.0, hud::recall::WIDTH, 100.0)
                .with_visible(false)
                .with_anchors(AnchorEdges::NONE);
            let list = ui.add(
                *radar,
                element,
                hud::recall::RecallList::new(Horizon::current(), recall_fonts),
            );
            if let Some(r) = ui.widget_mut::<hud::radar::RadarRing>(*radar) {
                r.recall_list = Some(list);
            }
            recall_dialog = Some(hud::recall_dialog::RecallDialog::mount(
                ui,
                *radar,
                Horizon::current(),
                dialog_fonts,
            ));
        }
        let card = {
            let ui = &mut self.host.root;
            let root = ui.root();
            let element = Element::at(
                0.0,
                0.0,
                hud::connection::CARD_WIDTH,
                hud::connection::CARD_HEIGHT,
            )
            .with_visible(false)
            .with_anchors(AnchorEdges::NONE);
            Some(ui.add(
                root,
                element,
                hud::connection::ConnectionCard::new(Horizon::current(), card_fonts),
            ))
        };
        let combat = self.mount_modern_combat();
        let panels = self.mount_modern_panels();
        self.mount_modern_bars();
        self.mount_modern_chat();
        self.mount_modern_inventory();
        self.mount_modern_inspect();
        self.mount_modern_loot();
        self.mount_modern_effects_window();
        self.mount_modern_spellbook();
        self.mount_modern_character();
        self.mount_modern_options();
        self.mount_modern_vendor();
        self.mount_modern_trade();
        self.mount_modern_salvage();
        self.mount_modern_questions();
        self.mount_modern_social();
        self.mount_modern_journal();
        self.mount_modern_book();
        self.mount_modern_house();
        self.mount_modern_map();
        self.mount_modern_chess();
        self.mount_modern_barber();
        let Some(modern) = self.modern.as_mut() else {
            return;
        };
        modern.edge_glow = Some(glow);
        match combat {
            Ok(id) => {
                modern.combat = Some(id);
                modern.replace(super::status::JUMP_POWERBAR_WINDOW);
            }
            Err(e) => self.notices.push(format!("ui: modern combat strip: {e}")),
        }
        match panels {
            Ok(ids) => modern.panels = Some(ids),
            Err(e) => self.notices.push(format!("ui: modern panel strip: {e}")),
        }
        match effects {
            Ok(hud) => modern.effects = Some(hud),
            Err(e) => self.notices.push(format!("ui: modern effects: {e}")),
        }
        modern.connection_card = card;
        modern.recall_dialog = recall_dialog;
        match radar {
            Ok(id) => {
                modern.radar = Some(id);
                modern.replace(super::window::RADAR_WINDOW);
            }
            Err(e) => self.notices.push(format!("ui: modern radar: {e}")),
        }
        match target {
            Ok(id) => modern.target = Some(id),
            Err(e) => self.notices.push(format!("ui: modern target: {e}")),
        }
        match vitals {
            Ok(id) => {
                modern.vitals = Some(id);
                modern.replace(super::window::VITALS_WINDOW);
                modern.replace(super::window::SIDE_VITALS_WINDOW);
            }
            Err(e) => self.notices.push(format!("ui: modern vitals: {e}")),
        }
    }

    /// The panel strip: the band and a button for each panel.
    fn mount_modern_panels(&mut self) -> Result<(ElementId, [ElementId; 8]), String> {
        use hud::panels::{BUTTON, Panel, PanelButton, PanelStrip};
        let font = self.modern_font(Weight::SemiBold, PANEL_NOTE_SIZE);
        let ui = &mut self.host.root;
        let strip = hud::mount_piece(
            ui,
            hud::PANELS,
            (hud::panels::WIDTH, hud::panels::HEIGHT),
            PanelStrip::new(),
        )?;
        let top = (hud::panels::HEIGHT - BUTTON) * 0.5;
        let mut index = 0;
        let buttons = Panel::ALL.map(|panel| {
            let element = Element::at(PanelStrip::button_left(index), top, BUTTON, BUTTON)
                .with_anchors(AnchorEdges::NONE);
            index += 1;
            ui.add(
                strip,
                element,
                PanelButton::new(Horizon::current(), font.clone(), panel),
            )
        });
        Ok((strip, buttons))
    }

    /// The panel strip's buttons name the keys bound to their actions.
    pub(super) fn feed_modern_panel_keys(&mut self, bindings: &ac_ui::input::KeyBindings) {
        use ac_dat::objects::StringTable;
        let Some((_, buttons)) = self.modern.as_ref().and_then(|m| m.panels) else {
            return;
        };
        let dats = self.content.dats().clone();
        let strings = ac_ui::strings::StringResolver::new(move |id| {
            dats.get::<StringTable>(id).ok().flatten()
        });
        let names = ac_ui::input::key_names::RetailKeyNames::new(
            Box::new(move |table, hash| strings.resolve(table, hash, 0)),
            (self.os_key_names)(),
        );
        let ui = &mut self.host.root;
        for id in buttons {
            if let Some(b) = ui.widget_mut::<hud::panels::PanelButton>(id) {
                b.key = bindings
                    .for_action(b.panel.action())
                    .map(|binding| names.describe(binding.chord))
                    .find(|name| !name.is_empty());
            }
        }
    }

    /// One frame of the panel strip: what was clicked or dropped is done,
    /// and the buttons follow their panels, the house's maintenance and an
    /// item dragged over Inventory.
    pub(super) fn tick_modern_panels(&mut self) {
        use hud::panels::{Panel, PanelButton, drop_note};
        let Some((_, buttons)) = self.modern.as_ref().and_then(|m| m.panels) else {
            return;
        };
        let mut clicked = Vec::new();
        let mut dropped = Vec::new();
        let mut over = false;
        for &id in &buttons {
            if let Some(b) = self.host.root.widget_mut::<PanelButton>(id) {
                if std::mem::take(&mut b.clicked) {
                    clicked.push(b.panel);
                }
                if let Some(item) = b.dropped.take() {
                    dropped.push(item);
                }
                over |= b.over.is_some();
            }
        }
        let target_mode = self.facts.state.mode != ac_runtime::inventory::InteractionMode::None;
        for panel in clicked {
            if panel == Panel::Inventory && target_mode {
                // In a use or examine mode, Inventory targets the player.
                ac_ui::panels::items::offer_target(
                    &mut self.host.root,
                    ac_runtime::inventory::ObjectRef::Player,
                );
            } else {
                self.handle_input_action(panel.action());
            }
        }
        let player = self.facts.state.player;
        for item in dropped {
            // The player's own item goes into the preferred pack, anything
            // else straight into the main pack; the session tells them
            // apart (retail's backpack button).
            if player != 0 && self.facts.objects.get(item).is_some() {
                ac_ui::panels::items::emit(
                    &mut self.host.root,
                    ac_ui::panels::PanelOutput::Session(ac_runtime::session::SessionCommand::Item(
                        ac_runtime::inventory::ItemCommand::DropOnBackpack(
                            ac_runtime::inventory::ObjectRef::Guid(item),
                        ),
                    )),
                );
            }
        }
        let note = over.then(|| {
            let resolve = super::facts::composed_names(&self.status.names);
            let view = self.facts.items_view(None, &[], None, &resolve);
            let capacity = view.get(player).map_or(0, |o| o.items_capacity);
            let held = ac_ui::panels::inventory::count_loose_contents(&view, player);
            drop_note(held, capacity)
        });
        let due = self.facts.house.as_ref().is_some_and(|h| {
            h.owns_house
                && h.lines
                    .iter()
                    .any(|(_, c)| *c == ac_runtime::gameplay::HouseLineColor::RentNotPaid)
        });
        // The main panels follow the windows standing in for retail's.
        let shows = |panel| self.panels.is_visible(&self.host.root, panel);
        let character = shows(ac_ui::panels::panel_id::CHARACTER);
        let inventory = shows(ac_ui::panels::panel_id::INVENTORY);
        let spellbook = shows(ac_ui::panels::panel_id::MAGIC);
        let options = shows(ac_ui::panels::panel_id::OPTIONS);
        let social = shows(ac_ui::panels::panel_id::SOCIAL);
        let house_open = self.modern_house_open();
        let map_open = self.modern_map_open();
        let journal = shows(ac_ui::panels::panel_id::JOURNAL);
        let ui = &mut self.host.root;
        for id in buttons {
            let Some(panel) = ui.widget::<PanelButton>(id).map(|b| b.panel) else {
                continue;
            };
            let open = match panel {
                Panel::Character => character,
                Panel::Inventory => inventory,
                Panel::Spellbook => spellbook,
                Panel::Journal => journal,
                Panel::Map => map_open,
                Panel::House => house_open,
                Panel::Social => social,
                Panel::Options => options,
            };
            if let Some(b) = ui.widget_mut::<PanelButton>(id) {
                b.open = open;
                b.dot = panel == Panel::House && due;
                if b.over.is_some() {
                    if b.note != note {
                        b.note.clone_from(&note);
                    }
                } else {
                    b.note = None;
                }
            }
        }
    }

    /// The combat strip.
    fn mount_modern_combat(&mut self) -> Result<ElementId, String> {
        let fonts = hud::combat::StripFonts {
            mode: self.modern_font(Weight::SemiBold, COMBAT_MODE_SIZE),
            small: self.modern_font(Weight::SemiBold, COMBAT_SMALL_SIZE),
            word: self.modern_font(Weight::SemiBold, COMBAT_WORD_SIZE),
        };
        hud::mount_piece(
            &mut self.host.root,
            hud::COMBAT,
            (hud::combat::WIDTH, hud::combat::HEIGHT),
            hud::combat::CombatStrip::new(fonts),
        )
    }

    /// The combat strip follows the mode, the attack, a jump being charged
    /// and, in Missile, the ammunition. False without the strip.
    pub(super) fn feed_modern_combat(&mut self, mode: ac_core::combat::CombatMode) -> bool {
        use hud::combat::{Ammo, CombatStrip, CombatView};
        let Some(id) = self.modern.as_ref().and_then(|m| m.combat) else {
            return false;
        };
        let view = self.player_view;
        let ammo = (mode == ac_core::combat::CombatMode::MISSILE).then(|| {
            let player = self.facts.state.player;
            let equipped = if player != 0 {
                self.facts.objects.get_equipped_by(player)
            } else {
                Vec::new()
            };
            let ammo = ac_core::items::ToolbarAmmo::resolve(&equipped);
            if ammo.is_visible() {
                Ammo::Loaded {
                    count: ammo.display_count,
                    name: self
                        .facts
                        .objects
                        .get(ammo.object_id)
                        .map(|o| o.name.clone())
                        .unwrap_or_default(),
                }
            } else {
                Ammo::Out
            }
        });
        let next = CombatView {
            mode: view.map(|_| mode),
            attack: view.map(|v| v.attack).unwrap_or_default(),
            jump: view.and_then(|v| v.jump_charge),
            ammo,
        };
        if let Some(strip) = self.host.root.widget_mut::<CombatStrip>(id)
            && strip.view != next
        {
            strip.set_view(next);
        }
        true
    }

    /// The status line above the radar and its card follow the link and
    /// the game's hour; the round trip is asked for on arrival and every
    /// [`hud::connection::PING_SECONDS`], and again when the card opens.
    pub(super) fn tick_modern_connection(&mut self) {
        let Some(modern) = self.modern.as_ref() else {
            return;
        };
        let (Some(radar), Some(card)) = (modern.radar, modern.connection_card) else {
            return;
        };
        let now = self.status.clock.elapsed().as_secs_f64();
        let link = self.status.link.unwrap_or(ac_runtime::session::LinkStatus {
            connected: true,
            ..Default::default()
        });
        let hour = self
            .map_state
            .calendar
            .map(|date| ac_ui::panels::map_house::hour_text(date.hour));
        let in_world = self.player_view.is_some();
        let ui = &mut self.host.root;
        // The card's close button closes it.
        let closed = ui
            .widget_mut::<hud::connection::ConnectionCard>(card)
            .is_some_and(|c| std::mem::take(&mut c.closed));
        let open = ui
            .widget_mut::<hud::radar::RadarRing>(radar)
            .is_some_and(|r| {
                if closed {
                    r.card_open = false;
                }
                r.status.hour = hour;
                r.status.link = link;
                r.status.dot_lit = (now / hud::connection::FLASH_SECONDS).floor() as i64 % 2 == 0;
                r.card_open
            });
        if open {
            let (x, y) = ui.screen_position(radar);
            let screen = ui.size();
            // Beside the radar (on the side with room), under the line.
            let beside = x - hud::connection::CARD_WIDTH - 4.0;
            let left = if beside >= 0.0 {
                beside
            } else {
                (x + hud::radar::WIDTH + 4.0).min((screen.0 - hud::connection::CARD_WIDTH).max(0.0))
            };
            if let Some(e) = ui.element_mut(card) {
                e.left = left;
                e.top = y + hud::connection::BAND + 6.0;
            }
            if let Some(c) = ui.widget_mut::<hud::connection::ConnectionCard>(card) {
                c.link = link;
            }
        }
        if ui.element(card).is_some_and(|e| e.visible()) != open {
            ui.set_visible(card, open);
            if open {
                ui.bring_to_front(card);
            }
        }
        let Some(modern) = self.modern.as_mut() else {
            return;
        };
        let just_opened = open && !modern.card_was_open;
        modern.card_was_open = open;
        let due = modern
            .last_ping
            .is_none_or(|t| now - t >= hud::connection::PING_SECONDS);
        if in_world && link.connected && (due || just_opened) {
            modern.last_ping = Some(now);
            ac_ui::panels::items::emit(
                &mut self.host.root,
                ac_ui::panels::PanelOutput::RequestLinkPing,
            );
        }
    }

    /// The modern radar shows `scene`, and its Recall list what the
    /// player can use.
    pub(super) fn feed_modern_radar(&mut self, scene: &ac_ui::panels::radar::RadarScene) {
        if let Some(m) = self.modern.as_mut() {
            m.last_player = scene.player;
        }
        let Some(id) = self.modern.as_ref().and_then(|m| m.radar) else {
            return;
        };
        let facts = self.recall_facts(scene);
        let Self { host, icons, .. } = self;
        let ui = &mut host.root;
        let list = ui
            .widget_mut::<hud::radar::RadarRing>(id)
            .and_then(|radar| {
                radar.set_scene(scene);
                radar.recall_list
            });
        if let Some(list) = list
            && let Some(l) = ui.widget_mut::<hud::recall::RecallList>(list)
        {
            l.set_facts(facts, &mut |spell| {
                ac_ui::icons::IconSource::spell_icon(icons, spell)
            });
            // The list may have grown or shrunk while open.
            let height = l.height();
            if let Some(e) = ui.element_mut(list)
                && e.height != height
            {
                e.height = height;
            }
        }
    }

    /// What decides which recalls the player can use.
    fn recall_facts(&self, scene: &ac_ui::panels::radar::RadarScene) -> hud::recall::RecallFacts {
        let traits = scene.player.map(|p| {
            ac_ui::radar::ObjectTraits::from_public_weenie_description(
                p.item_type,
                p.pwd_bitfield,
                0,
            )
        });
        let social = self.facts.social.as_ref().filter(|s| s.in_world);
        let spells = self
            .facts
            .numbers()
            .and_then(|n| n.spellbook())
            .map(|book| {
                hud::recall::recall_spells(
                    book.learned_spells()
                        .filter_map(|id| book.try_get_metadata(id).map(|m| (id, m.name.clone()))),
                )
            })
            .unwrap_or_default();
        hud::recall::RecallFacts {
            owns_house: self
                .facts
                .house
                .as_ref()
                .filter(|h| h.has_received_notice)
                .map(|h| h.owns_house),
            in_allegiance: social
                .map(|s| s.allegiance.as_ref().is_some_and(|a| a.monarch.is_some())),
            player_killer: traits.is_some_and(|t| t.is_player_killer),
            pk_lite: traits.is_some_and(|t| t.is_pk_lite),
            magic: self.facts.state.combat_mode == ac_core::combat::CombatMode::MAGIC,
            spells,
            server: self.status.server_world.clone(),
            own: self.shown_own_recalls(),
        }
    }

    /// The player's own recalls shown on this server, with their place in
    /// the saved list.
    fn shown_own_recalls(&self) -> Vec<(usize, ac_host::settings::CustomRecall)> {
        let server = self.status.server_world.as_deref();
        self.modern
            .as_ref()
            .and_then(|m| m.own_recalls.as_ref())
            .map(|all| {
                all.iter()
                    .enumerate()
                    .filter(|(_, r)| r.server.is_none() || r.server.as_deref() == server)
                    .map(|(i, r)| (i, r.clone()))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// One frame of the player's own recalls: loaded once the settings
    /// file is known; what the list and the dialog asked for is done and
    /// saved.
    pub(super) fn tick_modern_recalls(&mut self) {
        use hud::recall::OwnRequest;
        use hud::recall_dialog::{DialogOutcome, RecallDialog};
        let Some(modern) = self.modern.as_mut() else {
            return;
        };
        if modern.own_recalls.is_none()
            && let Some(store) = &self.status.settings_store
        {
            modern.own_recalls = Some(store.load_custom_recalls());
        }
        let (Some(radar), Some(dialog)) = (modern.radar, modern.recall_dialog) else {
            return;
        };
        let ui = &mut self.host.root;
        let list = ui
            .widget::<hud::radar::RadarRing>(radar)
            .and_then(|r| r.recall_list);
        let requests = list
            .and_then(|l| ui.widget_mut::<hud::recall::RecallList>(l))
            .map(|l| std::mem::take(&mut l.requests))
            .unwrap_or_default();
        let outcome = ui
            .widget_mut::<RecallDialog>(dialog)
            .and_then(|d| d.outcome.take());
        if requests.is_empty() && outcome.is_none() {
            return;
        }
        let server = self.status.server_world.clone();
        let shown: Vec<usize> = self
            .shown_own_recalls()
            .into_iter()
            .map(|(i, _)| i)
            .collect();
        let Some(modern) = self.modern.as_mut() else {
            return;
        };
        let all = modern.own_recalls.get_or_insert_with(Vec::new);
        let mut changed = false;
        for request in requests {
            match request {
                OwnRequest::Add | OwnRequest::Edit(_) => {
                    let (editing, recall) = match request {
                        OwnRequest::Edit(i) => (Some(i), all.get(i).cloned().unwrap_or_default()),
                        _ => (None, ac_host::settings::CustomRecall::default()),
                    };
                    let ui = &mut self.host.root;
                    let (left, top) = hud::radar::beside(
                        ui,
                        radar,
                        hud::recall_dialog::WIDTH,
                        hud::recall_dialog::HEIGHT,
                    );
                    if let Some(e) = ui.element_mut(dialog) {
                        e.left = left;
                        e.top = top;
                    }
                    RecallDialog::show(ui, dialog, editing, &recall, server.clone());
                    ui.bring_to_front(radar);
                }
                OwnRequest::Remove(i) if i < all.len() => {
                    all.remove(i);
                    changed = true;
                }
                OwnRequest::Move(i, step) => {
                    // Swap with the neighbour shown on this server.
                    let at = shown.iter().position(|&s| s == i);
                    let other = at
                        .and_then(|a| a.checked_add_signed(step as isize))
                        .and_then(|b| shown.get(b).copied());
                    if let Some(j) = other
                        && i < all.len()
                        && j < all.len()
                    {
                        all.swap(i, j);
                        changed = true;
                    }
                }
                OwnRequest::Remove(_) => {}
            }
        }
        if let Some(DialogOutcome::Done(editing, recall)) = outcome {
            match editing {
                Some(i) if i < all.len() => all[i] = recall,
                _ => all.push(recall),
            }
            changed = true;
        }
        if changed {
            let saved = all.clone();
            if let Some(store) = &self.status.settings_store
                && let Err(e) = store.save_custom_recalls(&saved)
            {
                self.notices.push(format!("ui: saving your recalls: {e}"));
            }
        }
    }

    /// The effects counts and status chips, each a child of one HUD piece,
    /// and the pane the counts open.
    fn mount_modern_effects(&mut self) -> Result<EffectsHud, String> {
        use hud::effects::{
            CHIP_HEIGHT, Chip, ChipFonts, ChipKind, ChipRow, EffectsPane, PANE_WIDTH, PaneFonts,
        };
        let chip_fonts = ChipFonts {
            figure: self.modern_font(Weight::SemiBold, EFFECTS_FIGURE_SIZE),
            words: self.modern_font(Weight::SemiBold, EFFECTS_WORDS_SIZE),
            small: self.modern_font(Weight::SemiBold, EFFECTS_SMALL_SIZE),
        };
        let pane_fonts = PaneFonts {
            label: self.modern_font(Weight::SemiBold, EFFECTS_LABEL_SIZE),
            name: self.modern_font(Weight::Medium, EFFECTS_NAME_SIZE),
            figure: chip_fonts.words.clone(),
            caption: self.modern_font(Weight::Regular, EFFECTS_CAPTION_SIZE),
        };
        let ui = &mut self.host.root;
        let row = hud::mount_piece(
            ui,
            hud::EFFECTS,
            (CHIP_HEIGHT, CHIP_HEIGHT),
            ChipRow::new(chip_fonts.small.clone()),
        )?;
        let chips = ChipKind::ALL.map(|kind| {
            let element = Element::at(0.0, 0.0, 0.0, CHIP_HEIGHT)
                .with_visible(false)
                .with_anchors(AnchorEdges::NONE);
            ui.add(
                row,
                element,
                Chip::new(Horizon::current(), chip_fonts.clone(), kind),
            )
        });
        let hover = HoverPane::mount(ui, Horizon::current(), PANE_WIDTH, 100.0);
        let list = ui.add(
            hover.element(),
            Element::at(0.0, 0.0, PANE_WIDTH, 100.0).with_anchors(AnchorEdges::NONE),
            EffectsPane::new(pane_fonts),
        );
        Ok(EffectsHud {
            row,
            chips,
            hover,
            list,
            now: hud::effects::EffectsOnYou::default(),
            show_times: true,
            read_at: None,
            soon: hud::effects::DEFAULT_SOON_SECONDS,
        })
    }

    /// The effects counts and the status chips follow the player: the
    /// effects (read again when they change, and each second for the times
    /// left), vitae, the load and a chess game.
    pub(super) fn feed_modern_effects(&mut self) {
        use hud::effects::{ChipView, Tone, burden_tone, burden_words, chess_words, effect_chips};
        if self
            .modern
            .as_ref()
            .and_then(|m| m.effects.as_ref())
            .is_none()
        {
            return;
        }
        let show_times = self.show_spell_durations();
        let soon = f64::from(self.settings().display.modern_hud.effect_warning_minutes) * 60.0;
        let now = self.facts.session_now().unwrap_or(0.0);
        let book = self.facts.numbers().and_then(|n| n.spellbook());
        let key = book.map(|b| (b.enchantment_revision(), now.floor() as i64));
        let read = book.map(|b| hud::effects::EffectsOnYou::read(b, now));
        let load = ac_ui::panels::indicators::burden_load(
            &self.facts.objects,
            self.facts.state.player,
            self.facts.strength(),
        );
        let chess = self.facts.chess.as_ref().map(|c| c.stage);
        let Some(fx) = self.modern.as_mut().and_then(|m| m.effects.as_mut()) else {
            return;
        };
        let changed = key != fx.read_at || show_times != fx.show_times || soon != fx.soon;
        if changed {
            fx.read_at = key;
            fx.show_times = show_times;
            fx.soon = soon;
            fx.now = read.unwrap_or_default();
        }
        let [helpful, harmful, vitae] = effect_chips(&fx.now, show_times, fx.soon);
        let burden = burden_tone(load).map(|tone| ChipView {
            tone,
            figure: String::new(),
            words: burden_words(load),
        });
        let chess = chess.and_then(chess_words).map(|words| ChipView {
            tone: Tone::Calm,
            figure: String::new(),
            words: words.to_owned(),
        });
        let views = [helpful, harmful, vitae, burden, chess];
        let (row, chips, open_for) = (fx.row, fx.chips, fx.hover.open_for());
        let mut moved = false;
        let ui = &mut self.host.root;
        for (id, view) in chips.into_iter().zip(views) {
            if let Some(chip) = ui.widget_mut::<hud::effects::Chip>(id) {
                let open = open_for == Some(id);
                if chip.view != view || chip.open != open {
                    chip.view = view;
                    chip.open = open;
                    moved = true;
                }
            }
        }
        if moved {
            hud::effects::lay_out(ui, row, &chips);
        }
        // A count that went away takes its pane with it.
        if let Some(fx) = self.modern.as_mut().and_then(|m| m.effects.as_mut())
            && let Some(open) = fx.hover.open_for()
            && !self.host.root.element(open).is_some_and(|e| e.visible())
        {
            fx.hover.close(&mut self.host.root);
        }
        if changed {
            self.fill_effects_pane(false);
        }
    }

    /// The frame rate line under the effects counts: `words` while it is
    /// asked for, else none. False without the modern effects piece (the
    /// retail display shows it then).
    pub(super) fn feed_modern_frame_rate(&mut self, words: Option<String>) -> bool {
        let Some(row) = self
            .modern
            .as_ref()
            .and_then(|m| m.effects.as_ref())
            .map(|fx| fx.row)
        else {
            return false;
        };
        if let Some(r) = self.host.root.widget_mut::<hud::effects::ChipRow>(row)
            && r.frame_rate != words
        {
            r.frame_rate = words;
        }
        true
    }

    /// One frame of the effects pane: opens, moves or closes it for the
    /// count the pointer rests on, and fills it when it opens.
    pub(super) fn tick_modern_effects(&mut self, dt: f64) {
        let Some(fx) = self.modern.as_mut().and_then(|m| m.effects.as_mut()) else {
            return;
        };
        let counts = [
            fx.chip(hud::effects::ChipKind::Helpful),
            fx.chip(hud::effects::ChipKind::Harmful),
        ];
        let was = fx.hover.open_for();
        let opened = fx.hover.tick(&mut self.host.root, &counts, dt as f32);
        let now = fx.hover.open_for();
        if opened.is_some() {
            self.fill_effects_pane(true);
        }
        if was != now {
            let ui = &mut self.host.root;
            for id in counts {
                if let Some(chip) = ui.widget_mut::<hud::effects::Chip>(id) {
                    chip.open = now == Some(id);
                }
            }
        }
    }

    /// Fills the open pane with the effects of its count's kind, sized to
    /// them (`fresh`: it has just opened, so the list starts at the top).
    fn fill_effects_pane(&mut self, fresh: bool) {
        let Self {
            modern,
            host,
            icons,
            ..
        } = self;
        let Some(fx) = modern.as_mut().and_then(|m| m.effects.as_mut()) else {
            return;
        };
        let Some(open) = fx.hover.open_for() else {
            return;
        };
        let harmful = open == fx.chip(hud::effects::ChipKind::Harmful);
        let view = hud::effects::PaneView::of(&fx.now, harmful, fx.show_times, fx.soon, |spell| {
            ac_ui::icons::IconSource::spell_icon(icons, spell)
        });
        let ui = &mut host.root;
        let height = ui
            .widget_mut::<hud::effects::EffectsPane>(fx.list)
            .map(|pane| {
                if fresh {
                    pane.scroll_to_top();
                }
                pane.show(view);
                pane.height()
            });
        if let Some(height) = height {
            for id in [fx.hover.element(), fx.list] {
                if let Some(e) = ui.element_mut(id) {
                    e.height = height;
                }
            }
        }
    }

    /// The player's last cast answered: one the mana could not pay for
    /// shakes the modern mana bar and marks what the spell costs. Each
    /// answer is looked at once; the first one seen is only noted.
    pub(super) fn feed_modern_cast(
        &mut self,
        cast: ac_runtime::gameplay::magic::SpellCastCompletion,
    ) {
        let Some(modern) = self.modern.as_mut() else {
            return;
        };
        let first = modern.seen_cast.is_none();
        if modern.seen_cast.replace(cast.revision) == Some(cast.revision)
            || first
            || cast.weenie_error != NOT_ENOUGH_MANA
        {
            return;
        }
        let Some(bars) = modern.vitals else {
            return;
        };
        let cost = self
            .facts
            .numbers()
            .and_then(|n| n.spellbook())
            .and_then(|b| b.try_get_metadata(cast.spell_id))
            .map_or(0, |spell| u32::try_from(spell.mana_cost).unwrap_or(0));
        if let Some(hud) = self.host.root.widget_mut::<hud::vitals::VitalsHud>(bars) {
            hud.refuse_mana(cost);
        }
    }

    /// The modern target line, when it is mounted.
    pub(super) fn modern_target_line(&mut self) -> Option<&mut hud::target::TargetLine> {
        let id = self.modern.as_ref()?.target?;
        self.host.root.widget_mut::<hud::target::TargetLine>(id)
    }

    /// The target line follows the selection: its name as it shows
    /// selected, what Use does to it, its health, or an item of the
    /// player's, its mana.
    pub(super) fn feed_modern_target(&mut self, scene: &crate::scene::Scene) {
        if self.modern.as_ref().and_then(|m| m.target).is_none() {
            return;
        }
        let selected = self.player_view.and_then(|v| v.selected);
        let target = selected.map(|guid| {
            let facts = super::status::selection_facts(&self.facts.state, selected, &self.status);
            let resolve = super::facts::composed_names(&self.status.names);
            let view = self.facts.items_view(selected, &[], None, &resolve);
            let stack = ac_ui::panels::selected_object::selection_stack(&view, guid);
            let name = ac_ui::panels::selected_object::selection_name(&view, &facts, guid, stack)
                .unwrap_or_else(|| format!("0x{guid:08X}"));
            let own = view.objects.is_owned_by_object(guid, view.player);
            let (item_type, flags) = match scene.object_by_guid(guid) {
                Some(o) if o.radar.item_type != 0 => {
                    (o.radar.item_type, o.radar.public_weenie_bitfield)
                }
                _ => view.get(guid).map_or((0, 0), |o| {
                    (
                        o.item_type.bits(),
                        o.public_weenie_bitfield.map_or(0, |f| f.bits()),
                    )
                }),
            };
            let class = ac_plugin_abi::world::classify(item_type, flags);
            let health = (facts.health.health_target && stack <= 1)
                .then_some(facts.health.percent)
                .flatten();
            let mana = (own && facts.mana > 0.0).then(|| {
                let numbers = view.get(guid).and_then(|o| {
                    let now = o.properties.get_int(ITEM_CUR_MANA, -1);
                    let max = o.properties.get_int(ITEM_MAX_MANA, -1);
                    (now >= 0 && max > 0).then_some((now, max))
                });
                hud::target::ItemMana {
                    share: facts.mana,
                    numbers,
                }
            });
            hud::target::TargetView {
                guid,
                name,
                own,
                use_kind: hud::target::UseKind::of(class),
                health,
                mana,
            }
        });
        if let Some(line) = self.modern_target_line()
            && line.target != target
        {
            line.target = target;
        }
        self.feed_modern_split();
    }

    /// The target line's split amount follows the split quantity drags
    /// use, and sets it when the player moves it.
    fn feed_modern_split(&mut self) {
        let Some(modern) = self.modern.as_mut() else {
            return;
        };
        let Some(id) = modern.target else {
            return;
        };
        let chosen = self
            .host
            .root
            .widget_mut::<hud::target::TargetLine>(id)
            .and_then(|line| line.split_chosen.take());
        if let Some(value) = chosen {
            modern.split.set_value(value);
        }
        let selected = self.player_view.and_then(|v| v.selected);
        let q = modern.split.quantity();
        let split = (selected.is_some() && q.maximum() > 1).then(|| hud::target::SplitView {
            value: q.value(),
            stack: q.maximum(),
        });
        self.host
            .root
            .with::<hud::target::TargetLine, _>(id, |line, ui| line.show_split(ui, split));
    }

    /// The selected stack's split quantity drags use: the modern target
    /// line's on Modern, else the retail toolbar's.
    pub(super) fn split_quantity(&mut self) -> Option<ac_core::items::StackSplitQuantity> {
        if let Some(modern) = self.modern.as_ref() {
            return Some(modern.split.quantity().clone());
        }
        self.host
            .with_controller::<ac_ui::panels::toolbar::ToolbarWindowController, _>(
                super::window::TOOLBAR_WINDOW,
                |c, _| c.split.quantity().clone(),
            )
    }

    /// The split key: the target line's amount takes the keyboard. False
    /// without the modern target line.
    pub(super) fn modern_split_key(&mut self) -> bool {
        let Some(id) = self.modern.as_ref().and_then(|m| m.target) else {
            return false;
        };
        self.host
            .root
            .with::<hud::target::TargetLine, _>(id, |line, ui| line.type_split(ui))
            .is_some()
    }

    /// The vitals' new values, for the modern vitals.
    pub(super) fn feed_modern_vitals(&mut self, vitals: &ac_ui::panels::vitals::VitalsView) {
        if let Some(m) = self.modern.as_mut() {
            m.last_vitals = Some(*vitals);
        }
        let Some(bars) = self.modern.as_ref().and_then(|m| m.vitals) else {
            return;
        };
        if let Some(hud) = self.host.root.widget_mut::<hud::vitals::VitalsHud>(bars) {
            hud.apply(vitals);
        }
    }

    /// The vitals follow their Options rows (animation, numbers, the low
    /// health share), and the screen's edge glows while health is low and
    /// the glow is on. Every frame, so a change shows at once.
    pub(super) fn follow_modern_vitals_settings(&mut self) {
        let Some(modern) = self.modern.as_ref() else {
            return;
        };
        let (bars, glow) = (modern.vitals, modern.edge_glow);
        let s = self.settings().display.modern_hud;
        // The quick-move key follows Options at once.
        if modern.outlines.is_some() {
            let key = quick_move_modifier(s.quick_move_key);
            if self.host.root.quick_move_key() != Some(key) {
                self.host.root.set_quick_move_key(Some(key));
            }
        }
        let threshold = if s.vital_numbers_percent > 0 {
            s.vital_numbers_percent as f32 / 100.0
        } else {
            f32::INFINITY
        };
        let mut low = false;
        if let Some(bars) = bars
            && let Some(hud) = self.host.root.widget_mut::<hud::vitals::VitalsHud>(bars)
        {
            hud.animate = s.animate_vitals;
            hud.number_threshold = threshold;
            hud.low_health = s.low_health_percent as f32 / 100.0;
            low = hud.health_is_low();
        }
        if let Some(glow) = glow
            && let Some(edge) = self.host.root.widget_mut::<hud::edge_glow::EdgeGlow>(glow)
        {
            edge.on = low && s.low_health_edge_glow;
        }
    }

    /// The HUD pieces follow what the player hid or locked: a hidden one
    /// stays hidden whoever shows it, a locked one is left out of quick
    /// move. Read from the settings the first time.
    pub(super) fn follow_modern_pieces(&mut self) {
        let Some(modern) = self.modern.as_mut() else {
            return;
        };
        let pieces = modern.pieces.get_or_insert_with(|| {
            self.status
                .settings_store
                .as_ref()
                .map(ac_host::settings::SettingsStore::load_hud_pieces)
                .unwrap_or_default()
        });
        let ui = &mut self.host.root;
        for name in hud::PIECES {
            let Some(frame) = ui.window(name).map(|w| w.frame) else {
                continue;
            };
            let movable = !pieces.is_locked(name);
            if let Some(e) = ui.element_mut(frame)
                && e.quick_move != movable
            {
                e.quick_move = movable;
            }
            if pieces.is_hidden(name) && ui.is_window_visible(name) {
                ui.hide_window(name);
            }
        }
    }

    /// Hides HUD piece `name` (or shows it again) and saves that.
    pub(super) fn set_modern_piece_hidden(&mut self, name: &'static str, on: bool) {
        let changed = self
            .modern
            .as_mut()
            .and_then(|m| m.pieces.as_mut())
            .is_some_and(|p| p.set_hidden(name, on));
        if !on {
            self.host.root.show_window(name);
        }
        if changed {
            self.save_modern_pieces();
        }
    }

    /// Locks HUD piece `name` in place (or unlocks it) and saves that.
    pub(super) fn set_modern_piece_locked(&mut self, name: &'static str, on: bool) {
        let changed = self
            .modern
            .as_mut()
            .and_then(|m| m.pieces.as_mut())
            .is_some_and(|p| p.set_locked(name, on));
        if changed {
            self.follow_modern_pieces();
            self.save_modern_pieces();
        }
    }

    /// The HUD pieces hidden and locked, by name.
    pub(super) fn modern_pieces(&self) -> Option<&ac_host::settings::HudPieces> {
        self.modern.as_ref()?.pieces.as_ref()
    }

    fn save_modern_pieces(&mut self) {
        let Some(pieces) = self.modern.as_ref().and_then(|m| m.pieces.clone()) else {
            return;
        };
        if let Some(store) = &self.status.settings_store
            && let Err(e) = store.save_hud_pieces(&pieces)
        {
            self.notices.push(format!("ui: saving the HUD pieces: {e}"));
        }
    }

    /// Quick move on: its key, and the layer that outlines and names every
    /// modern piece while the key is held; and the pieces' hover labels.
    pub(super) fn mount_quick_move(&mut self) {
        let key = quick_move_modifier(self.settings().display.modern_hud.quick_move_key);
        self.host.root.set_quick_move_key(Some(key));
        let font = self.modern_font(Weight::SemiBold, QUICK_MOVE_NAME_SIZE);
        let outlines = QuickMoveOutlines::mount(&mut self.host.root, font);
        // The modern pieces' hover labels (the retail tooltip needs a dat
        // layout the pieces do not name).
        let tip_font = self.modern_font(Weight::Medium, TIP_SIZE);
        ac_ui::modern::tip::TipLayer::mount(&mut self.host.root, Horizon::current(), tip_font);
        if let Some(modern) = self.modern.as_mut() {
            modern.outlines = Some(outlines);
        }
    }

    /// Inter in `weight` at `em` pixels to the em, for a modern piece:
    /// baked the first time it is asked for, its sharper bake following
    /// for a high-density display. None without the modern layer, or when
    /// the size cannot be baked (said in the notices).
    pub fn modern_font(&mut self, weight: Weight, em: f32) -> Option<Arc<UiFont>> {
        let modern = self.modern.as_mut()?;
        let known = modern.fonts.baked().count();
        let images = &self.images;
        let font = modern.fonts.get(weight, em, |height, face| {
            let mut atlas = ac_ui::bundled_font::bake_face(height, face)?;
            let coverage = std::mem::take(&mut atlas.coverage);
            let texture = images.add_coverage(atlas.width, atlas.height, coverage);
            Ok(Arc::new(atlas.create_font(texture)))
        });
        let baked_now = modern.fonts.baked().count() != known;
        let failures = modern.fonts.take_failures();
        self.notices.extend(failures);
        if baked_now {
            self.want_sharp_fonts_again();
        }
        font
    }
}

/// The modern look for the saved plugin theme `saved`: Bronze under
/// Classic, otherwise Bronze recoloured by the theme's seven colours.
#[must_use]
pub fn look_for_theme(saved: &str) -> Horizon {
    let Some(t) = ac_plugin_ui::ThemeKind::from_saved(saved).palette() else {
        return Horizon::BRONZE;
    };
    let c = |c: ac_plugin_ui::Color8| {
        let c = c.to_rgba();
        ac_ui::draw::Color::rgba(c.r, c.g, c.b, c.a)
    };
    Horizon::themed(&ac_ui::modern::theme::ThemeColours {
        background: c(t.background),
        field: c(t.field),
        border: c(t.border),
        text: c(t.text),
        muted: c(t.muted),
        accent: c(t.accent),
        selected: c(t.selected),
    })
}

impl WindowUi {
    /// The modern look follows the saved theme (the plugin dock's): made
    /// again when the setting changes, and set for this frame's drawing.
    pub(in crate::ui) fn follow_modern_theme(&mut self) {
        let saved = &self.settings().plugin_ui.theme;
        let look = match &self.modern_theme {
            Some((name, look)) if name == saved => *look,
            _ => {
                let look = look_for_theme(saved);
                self.modern_theme = Some((saved.clone(), look));
                look
            }
        };
        Horizon::set_current(look);
    }
}

#[cfg(test)]
mod tests {
    use ac_ui::widgets::Panel;

    use super::*;

    fn host_with(names: &[&str]) -> UiHost {
        let mut host = UiHost::new();
        host.root.set_size(800.0, 600.0);
        for name in names {
            let root = host.root.root();
            let frame = host.root.add(
                root,
                Element::at(10.0, 10.0, 100.0, 100.0),
                Panel::default(),
            );
            host.root.register_window(name, frame, None, 0).unwrap();
        }
        host
    }

    #[test]
    fn an_empty_layer_takes_nothing_down() {
        let mut host = host_with(&["radar", "chat"]);
        assert!(ModernLayer::new().take_down(&mut host).is_empty());
        assert!(host.root.is_window_visible("radar"));
        assert!(host.root.is_window_visible("chat"));
    }

    #[test]
    fn a_replaced_window_is_taken_down() {
        let mut host = host_with(&["radar", "chat"]);
        let mut layer = ModernLayer::new();
        layer.replace("radar");
        layer.replace("radar");
        assert_eq!(layer.replaced(), ["radar"]);
        assert_eq!(layer.take_down(&mut host), ["radar"]);
        assert!(host.root.window("radar").is_none());
        assert!(!host.root.show_window("radar"));
        assert!(host.root.is_window_visible("chat"));
    }
}
