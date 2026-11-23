//! The map, a box over most of the screen: a bar across the top (the
//! title, the side column's switch, Esc), the game's own map image
//! in a square that zooms and pans, with the towns, the player's arrow,
//! their house, their pins and houses for sale on it; a card for a town or
//! pin clicked, a card to name a new pin (right click); and the side
//! column: where the player is and the Dereth date, Find a place, and what
//! the map shows. Underground the map dims and says so.

use ac_ui::modern::tone;
use std::sync::Arc;

use ac_ui::draw::{Color, RenderContext, TextureRef};
use ac_ui::event::{UiEvent, kind};
use ac_ui::modern::Horizon;
use ac_ui::modern::text::{self, TextStyle};
use ac_ui::modern::theme::hex;
use ac_ui::panels::map_house::MapArt;
use ac_ui::shapes::CornerRadii;
use ac_ui::tree::{Element, ElementId, Widget};
use ac_ui::{UiFont, UiRoot};

use super::model::{self, Town, away_short, away_words, coordinates_text};
use super::tiles::{TileDraw, ViewMetres};
use crate::ui::modern::hud::lines::stroke;
use crate::ui::modern::kit::{
    self, ButtonLook, Hits, Rect, bronze, gold, inside, put, put_mid, put_right, spaced,
};

/// The top bar's height.
pub const BAR: f32 = 56.0;
/// The room kept under the map square, inside the box's rounded edge.
const FOOT: f32 = 10.0;
/// The side column's width and its margin.
const SIDE_W: f32 = 340.0;
const SIDE_GAP: f32 = 24.0;
/// The zoom steps. Past [`ART_ZOOM`] only with a detailed map.
const ZOOMS: [f32; 16] = [
    1.0, 1.5, 2.0, 3.0, 4.0, 6.0, 8.0, 12.0, 16.0, 24.0, 32.0, 48.0, 64.0, 96.0, 128.0, 192.0,
];
/// The furthest zoom on the game's map alone.
pub const ART_ZOOM: f32 = 8.0;
/// From this zoom every town is named.
const NAME_ALL_ZOOM: f32 = 2.0;

const MAP_GLYPH: &[&[(f32, f32)]] = &[
    &[
        (3.0, 6.0),
        (9.0, 3.5),
        (15.0, 6.0),
        (21.0, 3.5),
        (21.0, 18.0),
        (15.0, 20.5),
        (9.0, 18.0),
        (3.0, 20.5),
        (3.0, 6.0),
    ],
    &[(9.0, 3.5), (9.0, 18.0)],
    &[(15.0, 6.0), (15.0, 20.5)],
];
const PLUS: &[&[(f32, f32)]] = &[&[(12.0, 5.0), (12.0, 19.0)], &[(5.0, 12.0), (19.0, 12.0)]];
const MINUS: &[&[(f32, f32)]] = &[&[(5.0, 12.0), (19.0, 12.0)]];
const TARGET: &[&[(f32, f32)]] = &[
    &[(12.0, 2.0), (12.0, 7.0)],
    &[(12.0, 17.0), (12.0, 22.0)],
    &[(2.0, 12.0), (7.0, 12.0)],
    &[(17.0, 12.0), (22.0, 12.0)],
];
const SIDE: &[&[(f32, f32)]] = &[
    &[
        (3.0, 4.0),
        (21.0, 4.0),
        (21.0, 20.0),
        (3.0, 20.0),
        (3.0, 4.0),
    ],
    &[(15.0, 4.0), (15.0, 20.0)],
];
const CROSS: &[&[(f32, f32)]] = &[&[(7.0, 7.0), (17.0, 17.0)], &[(17.0, 7.0), (7.0, 17.0)]];
const HOUSE: &[&[(f32, f32)]] = &[
    &[(3.0, 11.0), (12.0, 4.0), (21.0, 11.0)],
    &[(5.0, 10.0), (5.0, 20.0), (19.0, 20.0), (19.0, 10.0)],
];
const PIN: &[&[(f32, f32)]] = &[&[
    (12.0, 22.0),
    (6.0, 13.0),
    (5.0, 9.0),
    (7.0, 5.0),
    (12.0, 3.0),
    (17.0, 5.0),
    (19.0, 9.0),
    (18.0, 13.0),
    (12.0, 22.0),
]];

/// The fonts the map draws with.
#[derive(Debug, Clone, Default)]
pub struct MapFonts {
    /// The title.
    pub title: Option<Arc<UiFont>>,
    /// The big coordinates.
    pub big: Option<Arc<UiFont>>,
    /// Names, buttons.
    pub strong: Option<Arc<UiFont>>,
    /// Words.
    pub body: Option<Arc<UiFont>>,
    /// Quiet lines, town names.
    pub small: Option<Arc<UiFont>>,
    /// Spaced capitals, the vitals' figures.
    pub label: Option<Arc<UiFont>>,
}

/// Where the player is, as the map shows it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Me {
    /// Map coordinates (east, north); underground, where they were last
    /// outside.
    pub x: f64,
    pub y: f64,
    /// Which way they face, degrees clockwise from north.
    pub heading: f64,
}

/// What the map shows besides its art.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct MapScreen {
    /// The player (none: not known yet).
    pub me: Option<Me>,
    /// The player is underground (`me` is where they were last outside).
    pub underground: bool,
    /// The player's house.
    pub house: Option<(f64, f64)>,
    /// The player's pins: name and place.
    pub pins: Vec<(String, f64, f64)>,
    /// Houses for sale: place and type.
    pub for_sale: Vec<(f64, f64, u32)>,
    /// The Dereth date and hour.
    pub date: String,
    pub hour: String,
}

/// A place picked on the map.
#[derive(Debug, Clone, PartialEq)]
pub struct Pick {
    /// Its name.
    pub name: String,
    /// Its place.
    pub x: f64,
    pub y: f64,
    /// The player's pin (its index).
    pub pin: Option<usize>,
}

/// What the map's layers show.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Layers {
    /// Town names.
    pub names: bool,
    /// The player's house.
    pub house: bool,
    /// The player's pins.
    pub pins: bool,
    /// Houses for sale.
    pub for_sale: bool,
}

impl Default for Layers {
    fn default() -> Self {
        Self {
            names: true,
            house: true,
            pins: true,
            for_sale: false,
        }
    }
}

/// What the map asks of the window.
#[derive(Debug, Clone, PartialEq)]
pub enum MapAsk {
    /// Close the map.
    Close,
    /// Copy words to the clipboard.
    Copy(String),
    /// Start a chat line with words.
    Chat(String),
    /// Save the new pin (its name is the field's) at a place.
    SavePin(f64, f64),
    /// Take a pin off.
    RemovePin(usize),
    /// Houses for sale were switched on (ask the server).
    ForSale,
}

#[derive(Debug, Clone, PartialEq)]
enum Hit {
    Ask(MapAsk),
    Pick(Pick),
    Unpick,
    ZoomIn,
    ZoomOut,
    Centre,
    Side,
    Layer(u8),
    CancelPin,
    Map,
    Swallow,
}

/// The map.
#[derive(Debug)]
pub struct MapView {
    fonts: MapFonts,
    /// The game's map art (none: the dats lack it).
    pub art: Option<MapArt>,
    towns: Vec<Town>,
    /// What it shows.
    pub screen: MapScreen,
    /// The layers.
    pub layers: Layers,
    /// The side column shows.
    pub side: bool,
    /// The screen was compact last frame.
    was_compact: bool,
    /// The place picked.
    pub pick: Option<Pick>,
    /// A new pin being named.
    pub new_pin: Option<(f64, f64)>,
    /// The search typed, and whether its field holds the keyboard.
    pub search: String,
    pub searching: bool,
    /// The pin's name field is empty.
    pub pin_empty: bool,
    /// What it asked for since the window last looked.
    pub asks: Vec<MapAsk>,
    zoom: usize,
    centre: (f32, f32),
    size: (f32, f32),
    hits: Hits<Hit>,
    held: Option<((f32, f32), (f32, f32))>,
    dragged: bool,
    /// The coordinates under the pointer.
    cursor: Option<(f64, f64)>,
    /// The detailed map's tiles to draw, bottom first.
    pub tiles: Vec<TileDraw>,
    /// The furthest zoom ([`ART_ZOOM`] on the game's map alone).
    max_zoom: f32,
}

impl MapView {
    /// The towns on the map (none before its art is set).
    #[must_use]
    pub fn towns(&self) -> &[Town] {
        &self.towns
    }

    /// The map in `theme`.
    #[must_use]
    pub fn new(fonts: MapFonts) -> Self {
        Self {
            fonts,
            art: None,
            towns: Vec::new(),
            screen: MapScreen::default(),
            layers: Layers::default(),
            side: true,
            was_compact: false,
            pick: None,
            new_pin: None,
            search: String::new(),
            searching: false,
            pin_empty: true,
            asks: Vec::new(),
            zoom: 0,
            centre: (0.0, 0.0),
            size: (1280.0, 720.0),
            hits: Hits::default(),
            held: None,
            dragged: false,
            cursor: None,
            tiles: Vec::new(),
            max_zoom: ART_ZOOM,
        }
    }

    /// The furthest zoom from now on; a zoom past it comes back to it.
    pub fn set_max_zoom(&mut self, max: f32) {
        self.max_zoom = max.max(ART_ZOOM);
        let top = self.top_zoom();
        if self.zoom > top {
            self.zoom = top;
            self.hold_centre();
        }
    }

    /// The furthest zoom step allowed.
    fn top_zoom(&self) -> usize {
        ZOOMS
            .iter()
            .rposition(|z| *z <= self.max_zoom + 1e-3)
            .unwrap_or(0)
    }

    /// What the map square shows, in world metres, and the screen's pixels
    /// a metre (none without the game's map art to place it).
    #[must_use]
    pub fn view_metres(&self) -> Option<ViewMetres> {
        let sq = self.square();
        if sq.2 <= 0.0 || sq.3 <= 0.0 {
            return None;
        }
        let corner = |sx: f32, sy: f32| {
            let art = self.art?;
            let region = self.region();
            let k = region / sq.2;
            let px = self.centre.0 - region * 0.5 + (sx - sq.0) * k;
            let py = self.centre.1 - region * 0.5 + (sy - sq.1) * k;
            let (x, y) = art.coordinates(f64::from(px), f64::from(py));
            Some(ac_world_map::index::metres(x, y))
        };
        let (west, north) = corner(sq.0, sq.1)?;
        let (east, south) = corner(sq.0 + sq.2, sq.1 + sq.3)?;
        let across = (east - west).abs().max(1.0);
        Some(((west, north, east, south), f64::from(sq.2) / across))
    }

    /// The screen pixels a metre at zoom `zoom`, the art and square as now.
    #[must_use]
    pub fn pixels_per_m_at(&self, zoom: f32) -> Option<f64> {
        let (_, now) = self.view_metres()?;
        Some(now * f64::from(zoom / ZOOMS[self.zoom]))
    }

    /// Gives the map its art.
    pub fn set_art(&mut self, art: MapArt) {
        self.towns = model::towns(&art);
        self.centre = (art.width * 0.5, art.height * 0.5);
        self.art = Some(art);
    }

    /// The map's size on the screen.
    pub fn set_size(&mut self, w: f32, h: f32) {
        self.size = (w, h);
    }

    /// The map centred on (`x`, `y`) map coordinates, zoomed in at least
    /// `zoom` steps, and that place marked when `name` is given.
    pub fn focus(&mut self, x: f64, y: f64, zoom: usize, name: Option<String>) {
        if let Some(art) = self.art {
            let (px, py) = art.pixel(x, y);
            self.centre = (px as f32, py as f32);
        }
        self.zoom = self.zoom.max(zoom).min(self.top_zoom());
        self.pick = name.map(|name| Pick {
            name,
            x,
            y,
            pin: None,
        });
        self.hold_centre();
    }

    /// A screen too narrow for the side column beside the map: it is a
    /// drawer from the bar over the map instead, shut until asked for.
    #[must_use]
    pub fn compact(&self) -> bool {
        self.size.0 <= SIDE_W + SIDE_GAP * 2.0 + 300.0
    }

    /// Follows the screen: on becoming compact the drawer starts shut.
    pub fn follow_screen(&mut self) {
        let compact = self.compact();
        if compact && !self.was_compact {
            self.side = false;
        }
        self.was_compact = compact;
    }

    /// The map square on the screen.
    #[must_use]
    pub fn square(&self) -> Rect {
        let (w, h) = self.size;
        let room = if self.side && !self.compact() {
            w - SIDE_W - SIDE_GAP * 2.0
        } else {
            w
        };
        let s = (h - BAR - FOOT).min(room).max(100.0);
        ((room - s) * 0.5, BAR, s, s)
    }

    /// The side column.
    fn side_rect(&self) -> Rect {
        let (w, h) = self.size;
        (
            w - SIDE_W - SIDE_GAP,
            BAR + 16.0,
            SIDE_W,
            h - BAR - 16.0 - SIDE_GAP,
        )
    }

    /// Where the search field goes (none: hidden).
    #[must_use]
    pub fn search_rect(&self) -> Option<Rect> {
        self.side.then(|| {
            let s = self.side_rect();
            (s.0 + 16.0, s.1 + 16.0 + 116.0 + 30.0, s.2 - 32.0, 32.0)
        })
    }

    /// Where the new pin's name field goes (none: hidden).
    #[must_use]
    pub fn pin_field_rect(&self) -> Option<Rect> {
        let (x, y) = self.new_pin?;
        let c = self.pin_card(x, y);
        Some((c.0 + 12.0, c.1 + 58.0, c.2 - 24.0, 30.0))
    }

    fn region(&self) -> f32 {
        self.art.map_or(256.0, |a| a.width) / ZOOMS[self.zoom]
    }

    fn hold_centre(&mut self) {
        let Some(art) = self.art else {
            return;
        };
        let half = self.region() * 0.5;
        // An axis the region does not fit in stays centred on the art.
        let hold = |c: f32, size: f32| {
            if size - half >= half {
                c.clamp(half, size - half)
            } else {
                size * 0.5
            }
        };
        self.centre.0 = hold(self.centre.0, art.width);
        self.centre.1 = hold(self.centre.1, art.height);
    }

    /// The screen point of map coordinates.
    fn to_screen(&self, x: f64, y: f64) -> Option<(f32, f32)> {
        let art = self.art?;
        let (px, py) = art.pixel(x, y);
        let sq = self.square();
        let region = self.region();
        let k = sq.2 / region;
        let left = self.centre.0 - region * 0.5;
        let top = self.centre.1 - region * 0.5;
        Some((sq.0 + (px as f32 - left) * k, sq.1 + (py as f32 - top) * k))
    }

    /// The map coordinates at a screen point.
    fn to_map(&self, sx: f32, sy: f32) -> Option<(f64, f64)> {
        let art = self.art?;
        let sq = self.square();
        if !inside(sq, sx, sy) {
            return None;
        }
        let region = self.region();
        let k = region / sq.2;
        let px = self.centre.0 - region * 0.5 + (sx - sq.0) * k;
        let py = self.centre.1 - region * 0.5 + (sy - sq.1) * k;
        Some(art.coordinates(f64::from(px), f64::from(py)))
    }

    fn zoom_by(&mut self, step: i32, at: Option<(f32, f32)>) {
        let before = at.and_then(|(x, y)| self.to_map(x, y));
        let z = (self.zoom as i32 + step).clamp(0, self.top_zoom() as i32);
        self.zoom = usize::try_from(z).unwrap_or(0);
        self.hold_centre();
        // The place under the pointer stays under it.
        if let (Some((mx, my)), Some((sx, sy))) = (before, at)
            && let Some((nx, ny)) = self.to_screen(mx, my)
        {
            let k = self.region() / self.square().2;
            self.centre.0 += (nx - sx) * k;
            self.centre.1 += (ny - sy) * k;
            self.hold_centre();
        }
    }

    fn pin_card(&self, x: f64, y: f64) -> Rect {
        let sq = self.square();
        let (sx, sy) = self.to_screen(x, y).unwrap_or((sq.0, sq.1));
        let (w, h) = (230.0, 140.0);
        let cx = (sx + 18.0).min(sq.0 + sq.2 - w - 8.0).max(sq.0 + 8.0);
        let cy = (sy - h * 0.5).clamp(sq.1 + 8.0, sq.1 + sq.3 - h - 8.0);
        (cx, cy, w, h)
    }

    fn pick_card(&self, p: &Pick) -> Rect {
        let sq = self.square();
        let (sx, sy) = self.to_screen(p.x, p.y).unwrap_or((sq.0, sq.1));
        let (w, h) = (230.0, 112.0);
        let cx = if sx + 16.0 + w < sq.0 + sq.2 - 8.0 {
            sx + 16.0
        } else {
            sx - 16.0 - w
        };
        let cy = (sy - h * 0.5).clamp(sq.1 + 8.0, sq.1 + sq.3 - h - 8.0);
        (cx.max(sq.0 + 8.0), cy, w, h)
    }

    fn click(&mut self, x: f32, y: f32) {
        let hit = self.hits.at(x, y);
        match hit {
            Some(Hit::Ask(a)) => {
                if let MapAsk::SavePin(..) | MapAsk::RemovePin(_) = a {
                    self.new_pin = None;
                    self.pick = None;
                }
                self.asks.push(a);
            }
            Some(Hit::Pick(p)) => {
                self.new_pin = None;
                if let Some(art) = self.art
                    && !inside(self.square(), x, y)
                {
                    // Picked from the list: bring it into view.
                    let (px, py) = art.pixel(p.x, p.y);
                    self.centre = (px as f32, py as f32);
                    self.hold_centre();
                }
                self.pick = Some(p);
            }
            Some(Hit::Unpick) => self.pick = None,
            Some(Hit::ZoomIn) => self.zoom_by(1, None),
            Some(Hit::ZoomOut) => self.zoom_by(-1, None),
            Some(Hit::Centre) => {
                if let Some(me) = self.screen.me {
                    self.focus(me.x, me.y, 0, None);
                }
            }
            Some(Hit::Side) => self.side = !self.side,
            Some(Hit::Layer(i)) => {
                let l = &mut self.layers;
                match i {
                    0 => l.names = !l.names,
                    1 => l.house = !l.house,
                    2 => l.pins = !l.pins,
                    _ => {
                        l.for_sale = !l.for_sale;
                        if l.for_sale {
                            self.asks.push(MapAsk::ForSale);
                        }
                    }
                }
            }
            Some(Hit::CancelPin) => self.new_pin = None,
            Some(Hit::Map) => {
                self.pick = None;
                self.new_pin = None;
            }
            Some(Hit::Swallow) | None => {}
        }
    }

    #[allow(clippy::too_many_lines)]
    fn draw_bar(&self, rc: &mut RenderContext<'_>, h: &mut Hits<Hit>) {
        let p = Horizon::current().palette;
        let f = &self.fonts;
        let (w, _) = self.size;
        let r = Horizon::current().metrics.window_radius;
        rc.fill_rounded_rect(
            0.0,
            0.0,
            w,
            BAR,
            CornerRadii::new(r, r, 0.0, 0.0),
            tone::ground(0.98),
        );
        rc.fill(0.0, BAR - 1.0, w, 1.0, bronze(0.45));
        stroke(
            rc,
            24.0,
            (BAR - 20.0) * 0.5,
            20.0,
            MAP_GLYPH,
            gold(1.0),
            1.8,
        );
        put_mid(rc, f.title.as_ref(), "Map", 54.0, 0.0, BAR, p.text);
        let tw = f
            .title
            .as_ref()
            .map_or(30.0, |t| text::measure(t, "Map", 0.0));
        put_mid(
            rc,
            f.small.as_ref(),
            "M",
            64.0 + tw,
            0.0,
            BAR,
            tone::muted(1.0),
        );
        // Esc and the side column's switch.
        let ew = 66.0;
        let esc = (w - 16.0 - ew, (BAR - 32.0) * 0.5, ew, 32.0);
        let lit = h.add(esc, Hit::Ask(MapAsk::Close));
        rc.stroke_rounded_rect(
            esc.0,
            esc.1,
            esc.2,
            esc.3,
            CornerRadii::all(4.0),
            if lit { gold(0.8) } else { bronze(0.45) },
            1.0,
        );
        put_mid(
            rc,
            f.label.as_ref(),
            "Esc",
            esc.0 + 10.0,
            esc.1,
            esc.3,
            tone::soft(1.0),
        );
        stroke(
            rc,
            esc.0 + ew - 26.0,
            esc.1 + 8.0,
            16.0,
            CROSS,
            tone::warm(1.0),
            1.8,
        );
        let sb = (esc.0 - 8.0 - 32.0, esc.1, 32.0, 32.0);
        let lit = h.add(sb, Hit::Side);
        rc.stroke_rounded_rect(
            sb.0,
            sb.1,
            sb.2,
            sb.3,
            CornerRadii::all(4.0),
            if self.side || lit {
                gold(0.8)
            } else {
                bronze(0.45)
            },
            1.0,
        );
        stroke(
            rc,
            sb.0 + 7.0,
            sb.1 + 7.0,
            18.0,
            SIDE,
            if self.side {
                gold(1.0)
            } else {
                tone::warm(1.0)
            },
            1.6,
        );
    }

    #[allow(clippy::too_many_lines)]
    fn draw_map(&mut self, rc: &mut RenderContext<'_>, h: &mut Hits<Hit>) {
        let p = Horizon::current().palette;
        let f = self.fonts.clone();
        let sq = self.square();
        rc.fill(sq.0, sq.1, sq.2, sq.3, hex(0x1B_262C, 1.0));
        h.add(sq, Hit::Map);
        rc.push_clip(sq.0, sq.1, sq.2, sq.3);
        if let Some(art) = self.art {
            let region = self.region();
            let (u0, v0) = (
                (self.centre.0 - region * 0.5) / art.width,
                (self.centre.1 - region * 0.5) / art.height,
            );
            let (u1, v1) = (u0 + region / art.width, v0 + region / art.height);
            rc.sprite(
                TextureRef::Surface(art.image),
                sq.0,
                sq.1,
                sq.2,
                sq.3,
                (u0, v0, u1, v1),
                Color::WHITE,
            );
        }
        // The detailed map over it, edges shared and each tile sampled half
        // a pixel in from its edge, so no seam shows.
        for t in &self.tiles {
            let (Some((x0, y0)), Some((x1, y1))) = (
                self.to_screen(t.west, t.north),
                self.to_screen(t.east, t.south),
            ) else {
                continue;
            };
            if x1 < sq.0 || y1 < sq.1 || x0 > sq.0 + sq.2 || y0 > sq.1 + sq.3 {
                continue;
            }
            rc.sprite(
                t.texture,
                x0,
                y0,
                x1 - x0,
                y1 - y0,
                (t.inset, t.inset, 1.0 - t.inset, 1.0 - t.inset),
                Color::WHITE,
            );
        }
        let zoom = ZOOMS[self.zoom];
        // Houses for sale.
        if self.layers.for_sale {
            for (x, y, _) in &self.screen.for_sale {
                if let Some((sx, sy)) = self.to_screen(*x, *y) {
                    rc.fill_rounded_rect(
                        sx - 4.0,
                        sy - 4.0,
                        8.0,
                        8.0,
                        CornerRadii::all(2.0),
                        hex(0x7F_D08A, 1.0),
                    );
                }
            }
        }
        // The towns.
        let shadow = hex(0, 0.95);
        for t in &self.towns {
            let Some((sx, sy)) = self.to_screen(t.x, t.y) else {
                continue;
            };
            let d = if t.big { 7.0 } else { 5.0 };
            let lit = h.add(
                (sx - 7.0, sy - 7.0, 14.0, 14.0),
                Hit::Pick(Pick {
                    name: t.name.to_owned(),
                    x: t.x,
                    y: t.y,
                    pin: None,
                }),
            );
            rc.fill_ellipse(
                sx - d * 0.5,
                sy - d * 0.5,
                d,
                d,
                if lit { gold(1.0) } else { tone::words(0.95) },
            );
            rc.stroke_ellipse(sx - d * 0.5, sy - d * 0.5, d, d, hex(0, 0.8), 1.0);
            if self.layers.names
                && (t.big || zoom >= NAME_ALL_ZOOM)
                && let Some(s) = &f.small
            {
                let style =
                    TextStyle::plain(if t.big { p.text } else { tone::warm(1.0) }).shadowed(shadow);
                text::draw(
                    rc,
                    s,
                    t.name,
                    sx + 6.0,
                    sy - s.line_height * 0.5 - 1.0,
                    style,
                );
            }
        }
        // The pins.
        if self.layers.pins {
            for (i, (name, x, y)) in self.screen.pins.iter().enumerate() {
                let Some((sx, sy)) = self.to_screen(*x, *y) else {
                    continue;
                };
                h.add(
                    (sx - 8.0, sy - 18.0, 16.0, 20.0),
                    Hit::Pick(Pick {
                        name: name.clone(),
                        x: *x,
                        y: *y,
                        pin: Some(i),
                    }),
                );
                stroke(rc, sx - 8.0, sy - 18.0, 16.0, PIN, gold(1.0), 2.0);
                if let Some(s) = &f.small {
                    text::draw(
                        rc,
                        s,
                        name,
                        sx + 8.0,
                        sy - 16.0,
                        TextStyle::plain(tone::gold_light(1.0)).shadowed(shadow),
                    );
                }
            }
        }
        // The house.
        if self.layers.house
            && let Some((x, y)) = self.screen.house
            && let Some((sx, sy)) = self.to_screen(x, y)
        {
            let r = (sx - 9.0, sy - 9.0, 18.0, 18.0);
            rc.fill_rounded_rect(r.0, r.1, r.2, r.3, CornerRadii::all(4.0), tone::ink(1.0));
            rc.stroke_rounded_rect(r.0, r.1, r.2, r.3, CornerRadii::all(4.0), gold(1.0), 1.5);
            stroke(rc, r.0 + 3.0, r.1 + 3.0, 12.0, HOUSE, gold(1.0), 1.5);
        }
        // Underground: the map dims.
        if self.screen.underground {
            rc.fill(sq.0, sq.1, sq.2, sq.3, hex(0x0C_0A08, 0.55));
        }
        // The player.
        if let Some(me) = self.screen.me
            && let Some((sx, sy)) = self.to_screen(me.x, me.y)
        {
            rc.fill_ellipse(sx - 13.0, sy - 13.0, 26.0, 26.0, gold(0.18));
            rc.stroke_ellipse(sx - 13.0, sy - 13.0, 26.0, 26.0, gold(0.6), 1.0);
            arrow(rc, sx, sy, me.heading as f32, !self.screen.underground);
        }
        // The new pin's ring.
        if let Some((x, y)) = self.new_pin
            && let Some((sx, sy)) = self.to_screen(x, y)
        {
            rc.stroke_ellipse(sx - 11.0, sy - 11.0, 22.0, 22.0, gold(1.0), 2.0);
        }
        rc.pop_clip();
        // Zoom.
        let zx = sq.0 + sq.2 - 10.0 - 30.0;
        for (i, (glyph, hit)) in [
            (PLUS, Hit::ZoomIn),
            (MINUS, Hit::ZoomOut),
            (TARGET, Hit::Centre),
        ]
        .into_iter()
        .enumerate()
        {
            let r = (zx, sq.1 + 10.0 + i as f32 * 34.0, 30.0, 30.0);
            let lit = h.add(r, hit);
            rc.fill_rounded_rect(
                r.0,
                r.1,
                r.2,
                r.3,
                CornerRadii::all(4.0),
                tone::ground(0.85),
            );
            rc.stroke_rounded_rect(
                r.0,
                r.1,
                r.2,
                r.3,
                CornerRadii::all(4.0),
                if lit { gold(1.0) } else { bronze(0.6) },
                1.0,
            );
            stroke(rc, r.0 + 7.0, r.1 + 7.0, 16.0, glyph, tone::warm(1.0), 1.8);
        }
        // The coordinates under the pointer.
        if let (Some((x, y)), Some(s)) = (self.cursor, &f.small) {
            let words = coordinates_text(x, y);
            let w = text::measure(s, &words, 0.0) + 16.0;
            let r = (sq.0 + 10.0, sq.1 + sq.3 - 8.0 - 22.0, w, 22.0);
            rc.fill_rounded_rect(r.0, r.1, r.2, r.3, CornerRadii::all(3.0), tone::ground(0.8));
            put_mid(rc, Some(s), &words, r.0 + 8.0, r.1, r.3, tone::soft(1.0));
        }
        if self.screen.underground {
            self.draw_underground(rc, h, sq);
        }
        if let Some(pick) = self.pick.clone() {
            self.draw_pick(rc, h, &pick);
        }
        if let Some((x, y)) = self.new_pin {
            self.draw_new_pin(rc, h, x, y);
        }
        if self.art.is_none() {
            put_mid(
                rc,
                f.body.as_ref(),
                "The game's map is not in its files.",
                sq.0 + 20.0,
                sq.1,
                40.0,
                tone::muted(1.0),
            );
        }
    }

    fn draw_underground(&self, rc: &mut RenderContext<'_>, h: &mut Hits<Hit>, sq: Rect) {
        let f = &self.fonts;
        let w = 360.0;
        let r = (sq.0 + (sq.2 - w) * 0.5, sq.1 + 40.0, w, 72.0);
        kit::panel(rc, r, 0.6);
        h.add(r, Hit::Swallow);
        put(
            rc,
            f.strong.as_ref(),
            "You are underground",
            r.0 + 14.0,
            r.1 + 12.0,
            Horizon::current().palette.text,
        );
        if let Some(s) = &f.small {
            let lines = kit::wrap(
                s,
                "The world map has no place for this dungeon. The hollow marker is where you were last outside.",
                w - 28.0,
            );
            kit::lines(
                rc,
                s,
                &lines,
                r.0 + 14.0,
                r.1 + 32.0,
                s.line_height + 3.0,
                tone::soft(1.0),
            );
        }
    }

    fn draw_pick(&self, rc: &mut RenderContext<'_>, h: &mut Hits<Hit>, pick: &Pick) {
        let p = Horizon::current().palette;
        let f = &self.fonts;
        let c = self.pick_card(pick);
        rc.soft_shadow(c.0, c.1, c.2, c.3, 6.0, 16.0, 40.0, hex(0, 0.6));
        rc.fill_rounded_rect(
            c.0,
            c.1,
            c.2,
            c.3,
            CornerRadii::all(6.0),
            tone::raised(0.97),
        );
        rc.stroke_rounded_rect(c.0, c.1, c.2, c.3, CornerRadii::all(6.0), bronze(0.8), 1.0);
        h.add(c, Hit::Swallow);
        let close = (c.0 + c.2 - 28.0, c.1 + 8.0, 20.0, 20.0);
        let lit = h.add(close, Hit::Unpick);
        stroke(
            rc,
            close.0 + 2.0,
            close.1 + 2.0,
            16.0,
            CROSS,
            if lit { p.accent } else { tone::soft(1.0) },
            1.6,
        );
        if let Some(st) = &f.strong {
            put(
                rc,
                Some(st),
                &kit::fit(st, &pick.name, c.2 - 44.0),
                c.0 + 12.0,
                c.1 + 10.0,
                p.text,
            );
        }
        let coords = coordinates_text(pick.x, pick.y);
        put(rc, f.body.as_ref(), &coords, c.0 + 12.0, c.1 + 32.0, p.text);
        if let Some(me) = self.screen.me {
            put(
                rc,
                f.small.as_ref(),
                &away_words((me.x, me.y), (pick.x, pick.y)),
                c.0 + 12.0,
                c.1 + 50.0,
                tone::soft(1.0),
            );
        }
        let by = c.1 + c.3 - 10.0 - 26.0;
        let b1 = (c.0 + 12.0, by, 112.0, 26.0);
        let lit = h.add(b1, Hit::Ask(MapAsk::Copy(coords.clone())));
        kit::button(
            rc,
            &Horizon::current(),
            b1,
            "Copy",
            f.strong.as_ref(),
            ButtonLook::Plain,
            lit,
            true,
        );
        let second = match pick.pin {
            Some(i) => ("Remove pin", MapAsk::RemovePin(i), ButtonLook::Danger),
            None => (
                "Link in chat",
                MapAsk::Chat(format!("{} {coords}", pick.name)),
                ButtonLook::Plain,
            ),
        };
        let b2 = (c.0 + 130.0, by, c.2 - 142.0, 26.0);
        let lit = h.add(b2, Hit::Ask(second.1));
        kit::button(
            rc,
            &Horizon::current(),
            b2,
            second.0,
            f.strong.as_ref(),
            second.2,
            lit,
            true,
        );
    }

    fn draw_new_pin(&self, rc: &mut RenderContext<'_>, h: &mut Hits<Hit>, x: f64, y: f64) {
        let p = Horizon::current().palette;
        let f = &self.fonts;
        let c = self.pin_card(x, y);
        rc.soft_shadow(c.0, c.1, c.2, c.3, 6.0, 16.0, 40.0, hex(0, 0.6));
        rc.fill_rounded_rect(
            c.0,
            c.1,
            c.2,
            c.3,
            CornerRadii::all(6.0),
            tone::raised(0.97),
        );
        rc.stroke_rounded_rect(c.0, c.1, c.2, c.3, CornerRadii::all(6.0), bronze(0.8), 1.0);
        h.add(c, Hit::Swallow);
        put(
            rc,
            f.strong.as_ref(),
            "Pin this spot",
            c.0 + 12.0,
            c.1 + 10.0,
            p.text,
        );
        kit::openac_tag(rc, f.label.as_ref(), c.0 + c.2 - 70.0, c.1 + 18.0);
        put(
            rc,
            f.small.as_ref(),
            &coordinates_text(x, y),
            c.0 + 12.0,
            c.1 + 32.0,
            tone::soft(1.0),
        );
        if let Some(r) = self.pin_field_rect() {
            kit::field_box(
                rc,
                &Horizon::current(),
                r,
                "Name the spot",
                self.pin_empty,
                true,
                f.body.as_ref(),
            );
        }
        let by = c.1 + c.3 - 10.0 - 28.0;
        let save = (c.0 + c.2 - 12.0 - 84.0, by, 84.0, 28.0);
        let lit = !self.pin_empty && h.add(save, Hit::Ask(MapAsk::SavePin(x, y)));
        kit::button(
            rc,
            &Horizon::current(),
            save,
            "Save pin",
            f.strong.as_ref(),
            ButtonLook::Primary,
            lit,
            !self.pin_empty,
        );
        let cancel = (save.0 - 6.0 - 70.0, by, 70.0, 28.0);
        let lit = h.add(cancel, Hit::CancelPin);
        kit::button(
            rc,
            &Horizon::current(),
            cancel,
            "Cancel",
            f.strong.as_ref(),
            ButtonLook::Plain,
            lit,
            true,
        );
    }

    /// With the side column shut: where the player is and the date, on
    /// the map's top corner.
    fn draw_where(&self, rc: &mut RenderContext<'_>) {
        let p = Horizon::current().palette;
        let f = &self.fonts;
        let coords = self.screen.me.map_or_else(
            || "Not known yet".to_owned(),
            |m| coordinates_text(m.x, m.y),
        );
        let (Some(st), Some(sm)) = (&f.strong, &f.small) else {
            return;
        };
        let w =
            text::measure(st, &coords, 0.0).max(text::measure(sm, &self.screen.date, 0.0)) + 28.0;
        let dated = !self.screen.date.is_empty();
        let date_h = if dated { 4.0 + sm.line_height } else { 0.0 };
        let r = (16.0, BAR + 16.0, w, 12.0 + st.line_height + date_h + 12.0);
        let round = CornerRadii::all(6.0);
        ac_ui::modern::theme::fill_glass(rc, r, round, tone::ground(0.9));
        rc.stroke_rounded_rect(r.0, r.1, r.2, r.3, round, bronze(0.5), 1.0);
        put(rc, Some(st), &coords, r.0 + 14.0, r.1 + 12.0, p.text);
        if dated {
            put(
                rc,
                Some(sm),
                &self.screen.date,
                r.0 + 14.0,
                r.1 + 12.0 + st.line_height + 4.0,
                tone::soft(1.0),
            );
        }
    }

    #[allow(clippy::too_many_lines)]
    fn draw_side(&self, rc: &mut RenderContext<'_>, h: &mut Hits<Hit>) {
        let p = Horizon::current().palette;
        let f = &self.fonts;
        let s = self.side_rect();
        rc.soft_shadow(s.0, s.1, s.2, s.3, 6.0, 18.0, 40.0, hex(0, 0.5));
        rc.fill_rounded_rect(
            s.0,
            s.1,
            s.2,
            s.3,
            CornerRadii::all(6.0),
            tone::ground(0.95),
        );
        rc.stroke_rounded_rect(s.0, s.1, s.2, s.3, CornerRadii::all(6.0), bronze(0.6), 1.0);
        h.add(s, Hit::Swallow);
        let x = s.0 + 16.0;
        let w = s.2 - 32.0;
        // Where you are.
        let r = (x, s.1 + 16.0, w, 104.0);
        kit::panel(rc, r, 0.3);
        spaced(
            rc,
            f.label.as_ref(),
            "WHERE YOU ARE",
            r.0 + 14.0,
            r.1 + 12.0,
        );
        let me = self.screen.me;
        let (coords, near) = match me {
            Some(m) => {
                let nearest = self.towns.iter().min_by(|a, b| {
                    let da = model::bearing(m.x, m.y, a.x, a.y).0;
                    let db = model::bearing(m.x, m.y, b.x, b.y).0;
                    da.total_cmp(&db)
                });
                let facing = model::way(m.heading).0;
                let near = if self.screen.underground {
                    format!("Last outside at {}", coordinates_text(m.x, m.y))
                } else {
                    nearest.map_or_else(String::new, |t| {
                        let away = away_words((m.x, m.y), (t.x, t.y)).replace(" of you", "");
                        format!("{} is {away} · facing {facing}", t.name)
                    })
                };
                (coordinates_text(m.x, m.y), near)
            }
            None => ("Not known yet".to_owned(), String::new()),
        };
        put(rc, f.big.as_ref(), &coords, r.0 + 14.0, r.1 + 28.0, p.text);
        if let Some(sm) = &f.small {
            put(
                rc,
                Some(sm),
                &kit::fit(sm, &near, r.2 - 28.0),
                r.0 + 14.0,
                r.1 + 54.0,
                tone::warm(1.0),
            );
        }
        rc.fill(r.0 + 14.0, r.1 + 74.0, r.2 - 28.0, 1.0, bronze(0.2));
        put_mid(
            rc,
            f.small.as_ref(),
            &self.screen.date,
            r.0 + 14.0,
            r.1 + 76.0,
            24.0,
            tone::soft(1.0),
        );
        put_right(
            rc,
            f.strong.as_ref(),
            &self.screen.hour,
            r.0 + r.2 - 14.0,
            r.1 + 76.0,
            24.0,
            p.text,
        );
        // Find a place.
        let mut y = r.1 + r.3 + 14.0;
        let found = self.found();
        let title = if self.search.trim().is_empty() {
            "FIND A PLACE · NEAREST".to_owned()
        } else {
            format!("FIND · {} FOUND", found.len())
        };
        spaced(rc, f.label.as_ref(), &title, x, y + 3.0);
        kit::openac_tag(rc, f.label.as_ref(), x + w - 58.0, y + 8.0);
        y += 22.0;
        if let Some(sr) = self.search_rect() {
            kit::search_box(
                rc,
                &Horizon::current(),
                sr,
                "Find a town or pin",
                self.search.is_empty(),
                self.searching,
                f.body.as_ref(),
            );
            y = sr.1 + sr.3 + 6.0;
        }
        for (name, px, py, pin, away) in found.iter().take(6) {
            let rr = (x, y, w, 30.0);
            let on = self.pick.as_ref().is_some_and(|p| p.name == *name);
            let lit = h.add(
                rr,
                Hit::Pick(Pick {
                    name: name.clone(),
                    x: *px,
                    y: *py,
                    pin: *pin,
                }),
            );
            if on || lit {
                rc.fill_rounded_rect(
                    rr.0,
                    rr.1,
                    rr.2,
                    rr.3,
                    CornerRadii::all(4.0),
                    if on { gold(0.12) } else { hex(0xFF_FFFF, 0.04) },
                );
            }
            if on {
                rc.stroke_rounded_rect(
                    rr.0,
                    rr.1,
                    rr.2,
                    rr.3,
                    CornerRadii::all(4.0),
                    gold(1.0),
                    1.0,
                );
            }
            if pin.is_some() {
                rc.fill_rounded_rect(
                    rr.0 + 8.0,
                    rr.1 + 11.5,
                    7.0,
                    7.0,
                    CornerRadii::all(1.0),
                    gold(1.0),
                );
            } else {
                rc.fill_ellipse(rr.0 + 8.0, rr.1 + 11.5, 7.0, 7.0, p.text);
            }
            put_mid(rc, f.strong.as_ref(), name, rr.0 + 24.0, rr.1, rr.3, p.text);
            put_right(
                rc,
                f.small.as_ref(),
                away,
                rr.0 + rr.2 - 8.0,
                rr.1,
                rr.3,
                tone::soft(1.0),
            );
            y += 30.0;
        }
        if found.is_empty() {
            put_mid(
                rc,
                f.body.as_ref(),
                "Nothing by that name.",
                x,
                y,
                30.0,
                tone::muted(1.0),
            );
            y += 30.0;
        }
        // What the map shows.
        y += 12.0;
        spaced(rc, f.label.as_ref(), "SHOW", x, y);
        y += 20.0;
        let l = self.layers;
        for (i, (words, on, openac)) in [
            ("Town names", l.names, false),
            ("Your house", l.house, false),
            ("Your pins", l.pins, true),
            ("Houses for sale", l.for_sale, true),
        ]
        .into_iter()
        .enumerate()
        {
            let rr = (x, y, w, 28.0);
            let lit = h.add(rr, Hit::Layer(i as u8));
            kit::switch(rc, (x, y + 5.0, 34.0, 18.0), on, lit);
            put_mid(
                rc,
                f.body.as_ref(),
                words,
                x + 44.0,
                y,
                28.0,
                tone::warm(1.0),
            );
            if openac {
                kit::openac_tag(rc, f.label.as_ref(), x + w - 58.0, y + 14.0);
            }
            y += 28.0;
        }
        if let Some(sm) = &f.small {
            let note = "Right-click the map to pin a spot. Houses for sale asks the server which cottages are free.";
            let lines = kit::wrap(sm, note, w);
            kit::lines(
                rc,
                sm,
                &lines,
                x,
                y + 6.0,
                sm.line_height + 3.0,
                tone::muted(1.0),
            );
        }
    }

    /// The places the side column lists: the nearest towns, or the towns
    /// and pins matching the search (name, place, pin index, how far).
    fn found(&self) -> Vec<(String, f64, f64, Option<usize>, String)> {
        let from = self.screen.me.map_or((0.0, 0.0), |m| (m.x, m.y));
        let q = self.search.trim().to_lowercase();
        let mut out: Vec<(String, f64, f64, Option<usize>, String)> = Vec::new();
        for (i, (name, x, y)) in self.screen.pins.iter().enumerate() {
            if !q.is_empty() && name.to_lowercase().contains(&q) {
                out.push((name.clone(), *x, *y, Some(i), away_short(from, (*x, *y))));
            }
        }
        let mut towns: Vec<&Town> = self
            .towns
            .iter()
            .filter(|t| q.is_empty() || t.name.to_lowercase().contains(&q))
            .collect();
        towns.sort_by(|a, b| {
            let da = model::bearing(from.0, from.1, a.x, a.y).0;
            let db = model::bearing(from.0, from.1, b.x, b.y).0;
            da.total_cmp(&db)
        });
        out.extend(towns.into_iter().map(|t| {
            (
                t.name.to_owned(),
                t.x,
                t.y,
                None,
                away_short(from, (t.x, t.y)),
            )
        }));
        out
    }
}

/// The player's arrow at (`x`, `y`) pointing `heading` degrees clockwise
/// from north; hollow when `solid` is false.
fn arrow(rc: &mut RenderContext<'_>, x: f32, y: f32, heading: f32, solid: bool) {
    let a = heading.to_radians();
    let (s, c) = a.sin_cos();
    let pt = |dx: f32, dy: f32| (x + dx * c - dy * s, y + dx * s + dy * c);
    let tip = pt(0.0, -10.0);
    let left = pt(-7.0, 8.0);
    let mid = pt(0.0, 4.0);
    let right = pt(7.0, 8.0);
    let ink = gold(1.0);
    if solid {
        let v = |(x, y): (f32, f32)| ac_ui::draw::ColorVertex { x, y, color: ink };
        rc.triangles(&[v(tip), v(left), v(mid), v(tip), v(mid), v(right)]);
    }
    let lines = [tip, left, mid, right, tip];
    for pair in lines.windows(2) {
        rc.line(pair[0].0, pair[0].1, pair[1].0, pair[1].1, ink, 2.0);
    }
}

impl Widget for MapView {
    fn hit_test(&self, _element: &Element, _x: f32, _y: f32) -> bool {
        true
    }

    fn handles_click(&self, _element: &Element) -> bool {
        true
    }

    fn receives_hover_mouse_move(&self) -> bool {
        true
    }

    fn on_event(&mut self, _ui: &mut UiRoot, me: ElementId, e: &UiEvent) -> bool {
        if e.target != Some(me) {
            return false;
        }
        let (x, y) = (e.data1 as f32, e.data2 as f32);
        match e.kind {
            kind::MOUSE_MOVE | kind::HOVER_ENTER => {
                self.hits.hover = Some((x, y));
                self.cursor = self.to_map(x, y);
                if let Some(((sx, sy), (cx, cy))) = self.held {
                    if (x - sx).abs() + (y - sy).abs() > 4.0 {
                        self.dragged = true;
                    }
                    if self.dragged {
                        let k = self.region() / self.square().2;
                        self.centre = (cx - (x - sx) * k, cy - (y - sy) * k);
                        self.hold_centre();
                    }
                    return true;
                }
                false
            }
            kind::HOVER_LEAVE => {
                self.hits.hover = None;
                self.cursor = None;
                false
            }
            kind::MOUSE_DOWN => {
                self.dragged = false;
                if inside(self.square(), x, y)
                    && matches!(self.hits.at(x, y), Some(Hit::Map | Hit::Pick(_)))
                {
                    self.held = Some(((x, y), self.centre));
                }
                true
            }
            kind::MOUSE_UP | kind::CAPTURE_CHANGED => {
                self.held = None;
                true
            }
            kind::CLICK => {
                if !std::mem::take(&mut self.dragged) {
                    self.click(x, y);
                }
                true
            }
            kind::RIGHT_CLICK => {
                if matches!(self.hits.at(x, y), Some(Hit::Map | Hit::Pick(_)))
                    && let Some((mx, my)) = self.to_map(x, y)
                {
                    self.pick = None;
                    self.new_pin = Some((mx, my));
                }
                true
            }
            kind::SCROLL => {
                if inside(self.square(), x, y) {
                    self.zoom_by(e.data0.signum(), Some((x, y)));
                }
                true
            }
            _ => false,
        }
    }

    fn draw(&mut self, _ui: &mut UiRoot, _me: ElementId, rc: &mut RenderContext<'_>) {
        let mut h = Hits::new(self.hits.hover);
        let (w, hh) = self.size;
        let theme = Horizon::current();
        rc.soft_shadow(0.0, 0.0, w, hh, 6.0, 18.0, 50.0, hex(0, 0.55));
        rc.fill_rounded_rect(
            0.0,
            0.0,
            w,
            hh,
            CornerRadii::all(theme.metrics.window_radius),
            hex(0x11_181D, 1.0),
        );
        self.follow_screen();
        self.draw_map(rc, &mut h);
        if self.side {
            self.draw_side(rc, &mut h);
        } else {
            self.draw_where(rc);
        }
        self.draw_bar(rc, &mut h);
        rc.stroke_rounded_rect(
            0.0,
            0.0,
            w,
            hh,
            CornerRadii::all(theme.metrics.window_radius),
            bronze(0.8),
            1.0,
        );
        self.hits = h;
    }
}
