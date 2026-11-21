//! The full-screen map in the game window, in place of retail's map page:
//! M, F10 and the panel strip's Map open and close it, Esc closes it, and
//! the windows' Map buttons open it on their place. It reads the game's map
//! art from retail's layout, the player from the radar's feed, the house,
//! the houses for sale and the pins (kept per character on this computer).

use ac_host::settings::{MapPin, MapPins};
use ac_runtime::session::SessionCommand;
use ac_ui::bundled_font::Weight;
use ac_ui::geometry::AnchorEdges;
use ac_ui::import::import_infos_rooted;
use ac_ui::modern::Horizon;
use ac_ui::panels::PanelOutput;
use ac_ui::panels::items::emit;
use ac_ui::panels::map_house::{self, hour_text};
use ac_ui::radar::RadarCoordinates;
use ac_ui::tree::{Element, ElementId};

use super::map::{MapAsk, MapFonts, MapScreen, MapView, Me};
use super::model::{heading_of_yaw, landblock_coordinates};
use crate::ui::map_window::MAP_HOUSE_WINDOW;
use crate::ui::modern::kit;
use crate::ui::window::WindowUi;

/// The map's name.
pub const MAP: &str = "modern-map";

/// The map's elements and what it remembers.
#[derive(Debug)]
pub struct MapUi {
    /// The map.
    pub view: ElementId,
    /// Find a place's field; the new pin's name.
    pub search: ElementId,
    pub pin_name: ElementId,
    /// The pins (read once).
    pins: Option<MapPins>,
    /// Where the player was last outside.
    pub last_outside: Option<(f64, f64)>,
    /// The map has been centred on the player once.
    centred: bool,
    /// The character `last_outside` and `centred` are about ("world/name");
    /// another character starts them afresh.
    about: Option<String>,
}

/// Whether `cell` is a cell inside (a dungeon or a building).
fn indoors(cell: u32) -> bool {
    cell & 0xFFFF >= 0x0100
}

impl WindowUi {
    /// The map, hidden, in place of retail's map and house panel (the house
    /// has its own window).
    pub(in crate::ui) fn mount_modern_map(&mut self) {
        let theme = Horizon::current();
        let fonts = MapFonts {
            title: self.modern_font(Weight::SemiBold, 15.0),
            big: self.modern_font(Weight::SemiBold, 20.0),
            strong: self.modern_font(Weight::SemiBold, 12.0),
            body: self.modern_font(Weight::Regular, 12.0),
            small: self.modern_font(Weight::Regular, 11.0),
            label: self.modern_font(Weight::SemiBold, 9.5),
        };
        let field_font = self.modern_font(Weight::Regular, 12.5);
        let art = import_infos_rooted(
            &self.content,
            map_house::HOST_LAYOUT_ID,
            map_house::SLOT_ELEMENT_ID,
        )
        .and_then(|info| map_house::map_art(&info));
        let ui = &mut self.host.root;
        let (w, h) = ui.size();
        let mut view = MapView::new(fonts);
        view.set_size(w, h);
        if let Some(a) = art {
            view.set_art(a);
        }
        let mut element = Element::at(0.0, 0.0, w, h).with_anchors(AnchorEdges::NONE);
        element.name = Some(MAP.to_owned());
        let root = ui.root();
        let frame = ui.add(root, element, view);
        if let Err(e) = ui.register_window(MAP, frame, None, 0) {
            self.notices.push(format!("ui: modern map: {e:?}"));
            return;
        }
        let search = kit::mount_field(
            ui,
            frame,
            (0.0, 0.0, 200.0, 32.0),
            field_font.clone(),
            &theme,
            false,
        );
        let pin_name = kit::mount_field(
            ui,
            frame,
            (0.0, 0.0, 200.0, 30.0),
            field_font,
            &theme,
            false,
        );
        ui.set_visible(search, false);
        ui.set_visible(pin_name, false);
        ui.hide_window(MAP);
        let Some(modern) = self.modern.as_mut() else {
            return;
        };
        modern.map = Some(MapUi {
            view: frame,
            search,
            pin_name,
            pins: None,
            last_outside: None,
            about: None,
            centred: false,
        });
        modern.replace(MAP_HOUSE_WINDOW);
    }

    fn map_ui(&self) -> Option<&MapUi> {
        self.modern.as_ref()?.map.as_ref()
    }

    /// The map key, F10 or the panel strip's Map: the map opens or closes.
    /// None without the modern map.
    pub(in crate::ui) fn modern_toggle_map(&mut self) -> Option<bool> {
        self.map_ui()?;
        if self.host.root.is_window_visible(MAP) {
            self.close_modern_map();
        } else {
            self.open_modern_map();
        }
        Some(true)
    }

    /// Whether the map shows.
    pub(in crate::ui) fn modern_map_open(&self) -> bool {
        self.host.root.is_window_visible(MAP)
    }

    /// Escape: the map closes when it shows. True when it did.
    pub(in crate::ui) fn modern_map_escape(&mut self) -> bool {
        if !self.modern_map_open() {
            return false;
        }
        self.close_modern_map();
        true
    }

    fn open_modern_map(&mut self) {
        let Some(view) = self.map_ui().map(|m| m.view) else {
            return;
        };
        let ui = &mut self.host.root;
        let (w, h) = ui.size();
        if let Some(e) = ui.element_mut(view) {
            e.width = w;
            e.height = h;
        }
        ui.show_window(MAP);
        ui.bring_to_front(view);
    }

    fn close_modern_map(&mut self) {
        let Some(m) = self.map_ui() else {
            return;
        };
        let (search, pin_name) = (m.search, m.pin_name);
        let ui = &mut self.host.root;
        ui.hide_window(MAP);
        if let Some(focus) = ui.keyboard_focus()
            && (focus == search || focus == pin_name)
        {
            ui.set_keyboard_focus(None);
        }
    }

    /// Opens the map on map coordinates (`x` east, `y` north), the place
    /// named `name` marked.
    pub(in crate::ui) fn show_on_modern_map(&mut self, x: f64, y: f64, name: &str) {
        let Some(view) = self.map_ui().map(|m| m.view) else {
            self.run_page_key(crate::ui::panel_keys::PageKey::World { house: false });
            return;
        };
        self.open_modern_map();
        if let Some(v) = self.host.root.widget_mut::<MapView>(view) {
            v.focus(x, y, 2, Some(name.to_owned()));
        }
        if let Some(m) = self.modern.as_mut().and_then(|m| m.map.as_mut()) {
            m.centred = true;
        }
    }

    /// Free houses of type `kind` on the map (the server is asked).
    pub(in crate::ui) fn show_houses_for_sale(&mut self, _kind: u32) {
        let Some(view) = self.map_ui().map(|m| m.view) else {
            self.run_page_key(crate::ui::panel_keys::PageKey::World { house: false });
            return;
        };
        self.open_modern_map();
        if let Some(v) = self.host.root.widget_mut::<MapView>(view) {
            v.layers.for_sale = true;
        }
    }

    /// The character's key for the pins ("world/name").
    fn map_character(&self) -> Option<String> {
        let name = self.status.playing.as_deref()?;
        Some(format!(
            "{}/{name}",
            self.status.server_world.as_deref().unwrap_or("")
        ))
    }

    fn map_pins(&mut self) -> Vec<MapPin> {
        let key = self.map_character();
        let store = self.status.settings_store.clone();
        let Some(m) = self.modern.as_mut().and_then(|m| m.map.as_mut()) else {
            return Vec::new();
        };
        let pins = m.pins.get_or_insert_with(|| {
            store
                .as_ref()
                .map(ac_host::settings::SettingsStore::load_map_pins)
                .unwrap_or_default()
        });
        key.map(|k| pins.of(&k).to_vec()).unwrap_or_default()
    }

    fn change_map_pins(&mut self, change: impl FnOnce(&mut MapPins, &str)) {
        let Some(key) = self.map_character() else {
            return;
        };
        self.map_pins();
        let store = self.status.settings_store.clone();
        let Some(pins) = self
            .modern
            .as_mut()
            .and_then(|m| m.map.as_mut())
            .and_then(|m| m.pins.as_mut())
        else {
            return;
        };
        change(pins, &key);
        if let Some(store) = store
            && let Err(e) = store.save_map_pins(pins)
        {
            self.notices.push(format!("ui: saving the map pins: {e}"));
        }
    }

    /// Each frame while the map shows: it follows the screen's size, the
    /// player, the house, the pins and the houses for sale; its fields are
    /// placed and what it asked for done.
    #[allow(clippy::too_many_lines)]
    pub(in crate::ui) fn tick_modern_map(&mut self) {
        if self.map_ui().is_none() {
            return;
        }
        // Another character: where it was and the centring start afresh.
        let character = self.map_character();
        if let Some(m) = self.modern.as_mut().and_then(|m| m.map.as_mut())
            && m.about != character
        {
            m.about = character;
            m.last_outside = None;
            m.centred = false;
        }
        let Some(m) = self.map_ui() else {
            return;
        };
        let (view, search, pin_name, centred) = (m.view, m.search, m.pin_name, m.centred);
        // Where the player was last outside is kept even while closed.
        let player = self.modern.as_ref().and_then(|m| m.last_player);
        let outside = player
            .filter(|p| !indoors(p.cell_id))
            .and_then(|p| RadarCoordinates::from_cell(p.cell_id))
            .map(|c| (c.x, c.y));
        if let Some(o) = outside
            && let Some(m) = self.modern.as_mut().and_then(|m| m.map.as_mut())
        {
            m.last_outside = Some(o);
        }
        if !self.host.root.is_window_visible(MAP) {
            return;
        }
        let last_outside = self.map_ui().and_then(|m| m.last_outside);
        let underground = player.is_some_and(|p| indoors(p.cell_id));
        let me = player.and_then(|p| {
            let (x, y) = if underground { last_outside? } else { outside? };
            Some(Me {
                x,
                y,
                heading: heading_of_yaw(p.yaw_radians),
            })
        });
        let house = self
            .facts
            .house
            .as_ref()
            .and_then(|h| h.position_cell)
            .and_then(RadarCoordinates::from_cell)
            .map(|c| (c.x, c.y));
        let for_sale = self
            .facts
            .house
            .as_ref()
            .map(|h| {
                h.for_sale
                    .iter()
                    .flat_map(|(kind, _, blocks)| {
                        blocks.iter().filter_map(move |b| {
                            landblock_coordinates(*b).map(|(x, y)| (x, y, *kind))
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();
        let pins = self
            .map_pins()
            .into_iter()
            .map(|p| (p.name, p.x, p.y))
            .collect();
        let vitals = self
            .modern
            .as_ref()
            .and_then(|m| m.last_vitals)
            .map_or_else(Default::default, |v| {
                let one = |r: ac_ui::panels::vitals::VitalView| {
                    let words = match (r.current, r.max) {
                        (Some(c), Some(m)) => format!("{c} / {m}"),
                        _ => String::new(),
                    };
                    (r.fill(), words)
                };
                [one(v.health), one(v.stamina), one(v.mana)]
            });
        let (date, hour) = self.map_state.calendar.map_or_else(
            || (String::new(), String::new()),
            |c| {
                (
                    format!("{:?} {}, {} P.Y.", c.month, c.day, c.year),
                    hour_text(c.hour),
                )
            },
        );
        let screen = MapScreen {
            me,
            underground,
            house,
            pins,
            for_sale,
            vitals,
            date,
            hour,
        };
        let ui = &mut self.host.root;
        let (w, h) = ui.size();
        if let Some(e) = ui.element_mut(view)
            && ((e.width - w).abs() > 0.5 || (e.height - h).abs() > 0.5)
        {
            e.width = w;
            e.height = h;
        }
        let typed = kit::field_text(ui, search);
        let pin_typed = kit::field_text(ui, pin_name);
        let focus = ui.keyboard_focus();
        let (asks, search_rect, pin_rect) = match ui.widget_mut::<MapView>(view) {
            Some(v) => {
                v.set_size(w, h);
                if !centred && let Some(me) = screen.me {
                    v.focus(me.x, me.y, 0, None);
                }
                if v.screen != screen {
                    v.screen = screen;
                }
                v.search.clone_from(&typed);
                v.searching = focus == Some(search);
                v.pin_empty = pin_typed.trim().is_empty();
                (
                    std::mem::take(&mut v.asks),
                    v.search_rect(),
                    v.pin_field_rect(),
                )
            }
            None => return,
        };
        if !centred
            && let Some(m) = self.modern.as_mut().and_then(|m| m.map.as_mut())
            && player.is_some()
        {
            m.centred = true;
        }
        let ui = &mut self.host.root;
        for (field, rect, search_box) in [(search, search_rect, true), (pin_name, pin_rect, false)]
        {
            match rect {
                Some(r) => {
                    let left = if search_box { r.0 + 29.0 } else { r.0 + 6.0 };
                    if let Some(e) = ui.element_mut(field) {
                        e.left = left;
                        e.top = r.1 + (r.3 - 24.0) * 0.5;
                        e.width = r.0 + r.2 - 6.0 - left;
                        e.height = 24.0;
                    }
                    if !ui.element(field).is_some_and(|e| e.visible) {
                        ui.set_visible(field, true);
                        if !search_box {
                            kit::set_field_text(ui, field, "");
                            ui.set_keyboard_focus(Some(field));
                        }
                    }
                }
                None => {
                    if ui.element(field).is_some_and(|e| e.visible) {
                        ui.set_visible(field, false);
                    }
                    if ui.keyboard_focus() == Some(field) {
                        ui.set_keyboard_focus(None);
                    }
                }
            }
        }
        for ask in asks {
            match ask {
                MapAsk::Close => self.close_modern_map(),
                MapAsk::Copy(words) => {
                    if let Err(Some(e)) = self.clipboard.set_text(&words) {
                        self.notices
                            .push(format!("ui: copying the coordinates: {e}"));
                    }
                }
                MapAsk::Chat(words) => {
                    self.close_modern_map();
                    self.start_modern_chat_line(&format!("{words} "));
                }
                MapAsk::SavePin(x, y) => {
                    let name = pin_typed.trim().to_owned();
                    if !name.is_empty() {
                        self.change_map_pins(|pins, key| pins.add(key, MapPin { name, x, y }));
                    }
                }
                MapAsk::RemovePin(i) => self.change_map_pins(|pins, key| {
                    pins.remove(key, i);
                }),
                MapAsk::ForSale => emit(
                    &mut self.host.root,
                    PanelOutput::Session(SessionCommand::Chat("@hslist cottage".to_owned())),
                ),
            }
        }
    }
}
