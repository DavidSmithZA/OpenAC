//! The House window and the house sale window in the game window. H and
//! the panel strip's House open the House window (in place of retail's
//! house page); the guest choices are the chat box's `@house` commands,
//! and the guest list is asked for when the window first needs it. The
//! house sale window stands in for retail's slumlord window: it opens and
//! closes with the slumlord's profile and sends what retail's sends.

use ac_runtime::inventory::{HousePurchaseCommand, HousePurchaseTab, ItemCommand};
use ac_runtime::local_time::LocalZone;
use ac_runtime::session::SessionCommand;
use ac_ui::bundled_font::Weight;
use ac_ui::modern::Horizon;
use ac_ui::panels::PanelOutput;
use ac_ui::panels::items::emit;
use ac_ui::tree::ElementId;

use super::model::{self, SaleInput};
use super::sale::{SaleAsk, SaleFonts, SaleFrame};
use super::window::{HouseAsk, HouseFonts, HouseFrame};
use crate::ui::modern::dialogs::look::{GOLD, Glyph, Look, RED};
use crate::ui::modern::{hud, kit};
use crate::ui::social_panels::HOUSE_PURCHASE_WINDOW;
use crate::ui::window::{DialogAction, WindowUi};

/// The House window's name.
pub const HOUSE: &str = "modern-house";
/// The house sale window's name.
pub const SALE: &str = "modern-house-sale";

/// A character's level and allegiance rank (property ints).
const LEVEL: u32 = 25;
const ALLEGIANCE_RANK: u32 = 0x1E;

/// The House windows' elements and what they remember.
#[derive(Debug)]
pub struct HouseUi {
    /// The House window and its guest field.
    pub frame: ElementId,
    pub guest: ElementId,
    /// The house sale window.
    pub sale: ElementId,
    /// The local time zone dates are worded in.
    zone: LocalZone,
    /// The guest list was asked for since the House window opened.
    asked: bool,
    /// The house sale window showed last frame.
    sale_open: bool,
}

fn house_chat(line: &str) -> PanelOutput {
    PanelOutput::Session(SessionCommand::Chat(line.to_owned()))
}

fn purchase(command: HousePurchaseCommand) -> PanelOutput {
    PanelOutput::Session(SessionCommand::Item(ItemCommand::HousePurchase(command)))
}

impl WindowUi {
    /// The House window and the house sale window, hidden; the sale window
    /// in place of retail's slumlord window.
    pub(in crate::ui) fn mount_modern_house(&mut self) {
        let theme = Horizon::BRONZE;
        let fonts = HouseFonts {
            title: self.modern_font(Weight::SemiBold, 15.0),
            heading: self.modern_font(Weight::SemiBold, 16.0),
            strong: self.modern_font(Weight::SemiBold, 12.5),
            body: self.modern_font(Weight::Regular, 12.0),
            small: self.modern_font(Weight::Regular, 11.0),
            label: self.modern_font(Weight::SemiBold, 9.5),
        };
        let sale_fonts = SaleFonts {
            title: self.modern_font(Weight::SemiBold, 15.0),
            strong: self.modern_font(Weight::SemiBold, 12.5),
            body: self.modern_font(Weight::Regular, 12.0),
            small: self.modern_font(Weight::Regular, 11.0),
            label: self.modern_font(Weight::SemiBold, 9.5),
        };
        let field_font = self.modern_font(Weight::Regular, 12.0);
        let ui = &mut self.host.root;
        let frame = match hud::mount_piece(
            ui,
            HOUSE,
            (super::window::WIDTH, super::window::HEIGHT),
            HouseFrame::new(theme, fonts),
        ) {
            Ok(id) => id,
            Err(e) => {
                self.notices.push(format!("ui: modern house: {e}"));
                return;
            }
        };
        let sale = match hud::mount_piece(
            ui,
            SALE,
            (super::sale::WIDTH, super::sale::HEIGHT),
            SaleFrame::new(theme, sale_fonts),
        ) {
            Ok(id) => id,
            Err(e) => {
                self.notices.push(format!("ui: modern house sale: {e}"));
                return;
            }
        };
        for id in [frame, sale] {
            if let Some(e) = ui.element_mut(id) {
                e.draggable = true;
            }
        }
        let guest = kit::mount_field(
            ui,
            frame,
            (0.0, 0.0, 200.0, 30.0),
            field_font,
            &theme,
            false,
        );
        ui.set_visible(guest, false);
        ui.hide_window(HOUSE);
        ui.hide_window(SALE);
        let Some(modern) = self.modern.as_mut() else {
            return;
        };
        modern.house = Some(HouseUi {
            frame,
            guest,
            sale,
            zone: LocalZone::system(),
            asked: false,
            sale_open: false,
        });
        modern.replace(HOUSE_PURCHASE_WINDOW);
    }

    /// The house key or the panel strip's House: the House window opens or
    /// closes. None without the modern window.
    pub(in crate::ui) fn modern_toggle_house(&mut self) -> Option<bool> {
        self.modern.as_ref()?.house.as_ref()?;
        let ui = &mut self.host.root;
        if ui.is_window_visible(HOUSE) {
            ui.hide_window(HOUSE);
        } else {
            ui.show_window(HOUSE);
            if let Some(id) = ui.window(HOUSE).map(|w| w.frame) {
                ui.bring_to_front(id);
            }
        }
        Some(true)
    }

    /// Whether the House window shows.
    pub(in crate::ui) fn modern_house_open(&self) -> bool {
        self.host.root.is_window_visible(HOUSE)
    }

    fn house_offset(&self) -> i32 {
        self.modern
            .as_ref()
            .and_then(|m| m.house.as_ref())
            .map_or(0, |h| h.zone.utc_offset_seconds(ac_clock::unix_millis()))
    }

    /// Each frame: the House window fed and its asks done while it shows;
    /// the house sale window follows the slumlord window.
    pub(in crate::ui) fn tick_modern_house(&mut self) {
        let Some(h) = self.modern.as_ref().and_then(|m| m.house.as_ref()) else {
            return;
        };
        let (frame, guest, sale, asked) = (h.frame, h.guest, h.sale, h.asked);
        self.tick_modern_sale(sale);
        let open = self.host.root.is_window_visible(HOUSE);
        if !open {
            let ui = &mut self.host.root;
            if ui.keyboard_focus() == Some(guest) {
                ui.set_keyboard_focus(None);
            }
            if let Some(h) = self.modern.as_mut().and_then(|m| m.house.as_mut()) {
                h.asked = false;
            }
            return;
        }
        let view = self.facts.house.clone().unwrap_or_default();
        let screen = model::house_screen(&view, self.house_offset());
        // Who may come in: asked for once a window opens on an owned house.
        if !asked && screen.owned.is_some() && screen.access.is_none() {
            emit(&mut self.host.root, house_chat("@house guest list"));
            if let Some(h) = self.modern.as_mut().and_then(|m| m.house.as_mut()) {
                h.asked = true;
            }
        }
        let ui = &mut self.host.root;
        let words = kit::field_text(ui, guest);
        let focused = ui.keyboard_focus() == Some(guest);
        let (asks, rect) = match ui.widget_mut::<HouseFrame>(frame) {
            Some(f) => {
                if f.screen != screen {
                    f.screen = screen;
                }
                f.guest_empty = words.trim().is_empty();
                f.guest_focused = focused;
                (std::mem::take(&mut f.asks), f.guest_field_rect())
            }
            None => return,
        };
        match rect {
            Some(r) => {
                if let Some(e) = ui.element_mut(guest) {
                    e.left = r.0 + 6.0;
                    e.top = r.1 + (r.3 - 24.0) * 0.5;
                    e.width = r.2 - 12.0;
                    e.height = 24.0;
                }
                ui.set_visible(guest, true);
            }
            None => {
                ui.set_visible(guest, false);
                if focused {
                    ui.set_keyboard_focus(None);
                }
            }
        }
        for ask in asks {
            self.house_ask(ask, guest);
        }
    }

    fn house_ask(&mut self, ask: HouseAsk, guest: ElementId) {
        let line = match ask {
            HouseAsk::Close => {
                self.host.root.hide_window(HOUSE);
                return;
            }
            HouseAsk::ShowOnMap(cell) => {
                if let Some(c) = ac_ui::radar::RadarCoordinates::from_cell(cell) {
                    self.show_on_modern_map(c.x, c.y, "Your house");
                }
                return;
            }
            HouseAsk::ForSale(kind) => {
                let name = match kind {
                    2 => "villa",
                    3 => "mansion",
                    _ => "cottage",
                };
                emit(&mut self.host.root, house_chat(&format!("@hslist {name}")));
                self.show_houses_for_sale(kind);
                return;
            }
            HouseAsk::Open(on) => {
                if on {
                    "@house open".to_owned()
                } else {
                    "@house close".to_owned()
                }
            }
            HouseAsk::AllegianceIn(on) => format!(
                "@house guest {}",
                if on {
                    "add_allegiance"
                } else {
                    "remove_allegiance"
                }
            ),
            HouseAsk::AllegianceStorage(on) => format!(
                "@house storage {}",
                if on {
                    "add_allegiance"
                } else {
                    "remove_allegiance"
                }
            ),
            HouseAsk::Storage(name, on) => format!(
                "@house storage {} {name}",
                if on { "add" } else { "remove" }
            ),
            HouseAsk::RemoveGuest(name) => format!("@house guest remove {name}"),
            HouseAsk::AddGuest => {
                let name = kit::field_text(&self.host.root, guest).trim().to_owned();
                if name.is_empty() {
                    return;
                }
                kit::set_field_text(&mut self.host.root, guest, "");
                format!("@house guest add {name}")
            }
            HouseAsk::ListGuests => "@house guest list".to_owned(),
            HouseAsk::SendOut => "@house boot_all".to_owned(),
            HouseAsk::Abandon => {
                let look = Look::new(
                    "Abandon your house?",
                    Glyph::Warn,
                    RED,
                    "Abandon",
                    "Keep it",
                )
                .dangerous();
                self.modern_ask_as(
                    look,
                    "The house and everything left in it are lost, and you may buy another."
                        .to_owned(),
                    DialogAction::Panel(house_chat("@house abandon")),
                );
                return;
            }
        };
        emit(&mut self.host.root, house_chat(&line));
        // The server's guest list follows the change.
        if line.starts_with("@house guest") && !line.ends_with(" list")
            || line.starts_with("@house storage")
            || line.starts_with("@house open")
            || line.starts_with("@house close")
        {
            emit(&mut self.host.root, house_chat("@house guest list"));
        }
    }

    /// The house sale window follows the slumlord's profile: it opens and
    /// closes with it, and while it shows is fed and does what it asked.
    fn tick_modern_sale(&mut self, sale: ElementId) {
        let view = self.facts.house_purchase.clone();
        let was = self
            .modern
            .as_ref()
            .and_then(|m| m.house.as_ref())
            .is_some_and(|h| h.sale_open);
        let open = view.is_open();
        if open != was {
            let ui = &mut self.host.root;
            if open {
                ui.show_window(SALE);
                ui.bring_to_front(sale);
                if let Some(f) = ui.widget_mut::<SaleFrame>(sale) {
                    // A house the player owns opens on its maintenance.
                    let mine = view.profile.as_ref().is_some_and(|p| {
                        p.owner_guid != 0 && p.owner_guid == self.facts.state.player
                    });
                    f.tab = if mine {
                        HousePurchaseTab::Rent
                    } else {
                        HousePurchaseTab::Buy
                    };
                }
            } else {
                ui.hide_window(SALE);
            }
            if let Some(h) = self.modern.as_mut().and_then(|m| m.house.as_mut()) {
                h.sale_open = open;
            }
        }
        if !open {
            return;
        }
        // Closed by the player (the window's close or a key).
        if !self.host.root.is_window_visible(SALE) {
            emit(&mut self.host.root, purchase(HousePurchaseCommand::Close));
            if let Some(h) = self.modern.as_mut().and_then(|m| m.house.as_mut()) {
                h.sale_open = false;
            }
            return;
        }
        let player = self.facts.state.player;
        let tab = self
            .host
            .root
            .widget::<SaleFrame>(sale)
            .map_or(HousePurchaseTab::Buy, |f| f.tab);
        let ints = self
            .facts
            .objects
            .get(player)
            .map(|o| o.properties.ints.clone())
            .unwrap_or_default();
        let buy_another = self
            .facts
            .house
            .as_ref()
            .map_or(0, |h| h.buy_another_unix_s);
        let screen = model::sale_screen(&SaleInput {
            view: &view,
            tab,
            objects: &self.facts.objects,
            player,
            level: ints.get(&LEVEL).copied().unwrap_or(0),
            rank: ints.get(&ALLEGIANCE_RANK).copied().unwrap_or(0),
            buy_another_unix_s: buy_another,
            offset: self.house_offset(),
        });
        let asks = match self.host.root.widget_mut::<SaleFrame>(sale) {
            Some(f) => {
                if let Some(s) = screen
                    && f.screen != s
                {
                    f.screen = s;
                }
                std::mem::take(&mut f.asks)
            }
            None => return,
        };
        for ask in asks {
            let out = match ask {
                SaleAsk::Close => purchase(HousePurchaseCommand::Close),
                SaleAsk::Add(wcid) => {
                    for item in model::payers(&self.facts.objects, player, wcid) {
                        emit(
                            &mut self.host.root,
                            purchase(HousePurchaseCommand::Stage { tab, item }),
                        );
                    }
                    continue;
                }
                SaleAsk::TakeOff(item) => purchase(HousePurchaseCommand::Unstage { tab, item }),
                SaleAsk::Dropped(p) => purchase(HousePurchaseCommand::Stage {
                    tab,
                    item: p.obj_id,
                }),
                SaleAsk::Send => {
                    let Some(profile) = &view.profile else {
                        continue;
                    };
                    let send = purchase(HousePurchaseCommand::Submit {
                        tab,
                        slumlord: profile.slumlord_guid,
                    });
                    match view.confirmation(tab, player) {
                        Some(question) => {
                            let title = if tab == HousePurchaseTab::Buy {
                                "Buy this house?"
                            } else {
                                "Pay someone else's maintenance?"
                            };
                            let yes = if tab == HousePurchaseTab::Buy {
                                "Buy"
                            } else {
                                "Pay"
                            };
                            self.modern_ask_as(
                                Look::new(title, Glyph::Question, GOLD, yes, "Not now"),
                                question.to_owned(),
                                DialogAction::Either(send, purchase(HousePurchaseCommand::Close)),
                            );
                            continue;
                        }
                        None => send,
                    }
                }
            };
            emit(&mut self.host.root, out);
        }
    }
}
