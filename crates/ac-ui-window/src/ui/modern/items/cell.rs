//! One item cell: a drawn tile with the item's icon, its stack count and
//! shortcut key, the selection, a cooldown's sweep and a request in flight;
//! in the list a row with the name and a line under it. A cell is an
//! element of its own so drags find it; it notes what was asked and the
//! window carries it out.

use std::sync::Arc;

use ac_ui::draw::{Color, RenderContext, TextureRef};
use ac_ui::event::{DragPayload, ItemDragPayload, ItemDragSource, UiEvent, kind};
use ac_ui::modern::Horizon;
use ac_ui::modern::text::{self, TextStyle};
use ac_ui::modern::theme::hex;
use ac_ui::shapes::CornerRadii;
use ac_ui::tree::{DragGhost, Element, ElementId, Widget};
use ac_ui::{UiFont, UiRoot};

/// How cells are drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CellLook {
    /// Tiles 40 high.
    #[default]
    Grid,
    /// Smaller tiles, 32 high.
    Compact,
    /// Rows with names.
    List,
}

impl CellLook {
    /// A tile's (or row's) height.
    #[must_use]
    pub fn height(self) -> f32 {
        match self {
            Self::Grid => 40.0,
            Self::Compact => 32.0,
            Self::List => 36.0,
        }
    }

    /// A tile's least width (a row takes the whole width).
    #[must_use]
    pub fn least_width(self) -> f32 {
        match self {
            Self::Grid => 38.0,
            Self::Compact => 30.0,
            Self::List => f32::INFINITY,
        }
    }
}

/// What a cell shows.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CellView {
    /// The item (0: an empty slot).
    pub id: u32,
    /// Its icon.
    pub icon: Option<TextureRef>,
    /// The stack's size, when more than one.
    pub count: Option<i32>,
    /// Its shortcut key, top left.
    pub stamp: Option<String>,
    /// The name (the list's rows, and the hover words).
    pub name: String,
    /// The line under the name in the list.
    pub sub: String,
    /// The name's colour in the list (none: the text colour).
    pub name_color: Option<Color>,
    /// Selected.
    pub selected: bool,
    /// A request about it is in flight.
    pub waiting: bool,
    /// The share of a cooldown still to run (0: none).
    pub cooldown: f32,
    /// Shown faint (a drag elsewhere is under way).
    pub faint: bool,
    /// Lit: what is being dragged may go here.
    pub lit: bool,
}

/// What a cell asks of the window.
#[derive(Debug, Clone, PartialEq)]
pub enum CellRequest {
    /// Select the item (a click).
    Select(u32),
    /// Use it (a double click).
    Use(u32),
    /// Its menu, at this screen point (a right click).
    Menu(u32, (f32, f32)),
    /// An item was let go on it.
    Dropped {
        /// The item carried.
        item: u32,
        /// Where it came from.
        from: ItemDragSource,
        /// The cell's place: its group's pack (none: a mixed group).
        container: Option<u32>,
        /// The cell's index in its group.
        index: usize,
        /// The item in the cell (0: an empty slot).
        onto: u32,
    },
    /// A drag from it began.
    Lifted(u32),
    /// A drag from it ended (wherever it landed).
    DragEnded(u32),
}

/// One cell.
#[derive(Debug)]
pub struct ItemCell {
    theme: Horizon,
    /// Count and stamp.
    small: Option<Arc<UiFont>>,
    /// The list's name.
    name: Option<Arc<UiFont>>,
    /// What it shows.
    pub view: CellView,
    /// How it is drawn.
    pub look: CellLook,
    /// Where a drag from it says it came from.
    pub source: ItemDragSource,
    /// Its group's pack.
    pub container: Option<u32>,
    /// Its index in its group.
    pub index: usize,
    /// Nothing is dropped on it.
    pub no_drops: bool,
    /// What it asked for since the window last looked.
    pub requests: Vec<CellRequest>,
    hovered: bool,
    pressed: bool,
    /// A drag is over it: whether it would take it.
    over: Option<bool>,
}

impl ItemCell {
    /// A cell in `theme`; counts in `small`, names in `name`.
    #[must_use]
    pub fn new(theme: Horizon, small: Option<Arc<UiFont>>, name: Option<Arc<UiFont>>) -> Self {
        Self {
            theme,
            small,
            name,
            view: CellView::default(),
            look: CellLook::Grid,
            source: ItemDragSource::Inventory,
            container: None,
            index: 0,
            no_drops: false,
            requests: Vec::new(),
            hovered: false,
            pressed: false,
            over: None,
        }
    }

    fn takes(&self, payload: Option<&DragPayload>) -> Option<(u32, ItemDragSource)> {
        if self.no_drops {
            return None;
        }
        let p = payload?.get::<ItemDragPayload>()?;
        (p.obj_id != 0 && !p.source_kind.is_alias() && p.obj_id != self.view.id)
            .then_some((p.obj_id, p.source_kind))
    }
}

impl Widget for ItemCell {
    fn hit_test(&self, _element: &Element, _x: f32, _y: f32) -> bool {
        true
    }

    fn handles_click(&self, _element: &Element) -> bool {
        true
    }

    fn is_drag_source(&self, _element: &Element) -> bool {
        self.view.id != 0 && !self.view.waiting
    }

    fn drag_payload(&mut self, _ui: &mut UiRoot, me: ElementId) -> Option<DragPayload> {
        if self.view.id == 0 {
            return None;
        }
        let mut p = ItemDragPayload::new(
            self.view.id,
            self.source,
            i32::try_from(self.index).unwrap_or(-1),
        );
        p.source_cell = Some(me);
        Some(DragPayload::new(p))
    }

    fn drag_ghost(&mut self, _ui: &mut UiRoot, _me: ElementId) -> Option<DragGhost> {
        self.view.icon.map(|texture| DragGhost {
            texture,
            width: 32,
            height: 32,
        })
    }

    fn set_drag_source_active(
        &mut self,
        _ui: &mut UiRoot,
        _me: ElementId,
        active: bool,
        _payload: Option<&DragPayload>,
    ) {
        self.requests.push(if active {
            CellRequest::Lifted(self.view.id)
        } else {
            CellRequest::DragEnded(self.view.id)
        });
    }

    fn on_event(&mut self, ui: &mut UiRoot, me: ElementId, e: &UiEvent) -> bool {
        if e.target != Some(me) {
            return false;
        }
        match e.kind {
            kind::MOUSE_MOVE | kind::HOVER_ENTER => {
                self.hovered = true;
                false
            }
            kind::HOVER_LEAVE => {
                self.hovered = false;
                false
            }
            kind::MOUSE_DOWN => {
                self.pressed = true;
                true
            }
            kind::MOUSE_UP | kind::CAPTURE_CHANGED => {
                self.pressed = false;
                true
            }
            kind::CLICK => {
                if self.view.id != 0 {
                    self.requests.push(CellRequest::Select(self.view.id));
                }
                true
            }
            kind::DOUBLE_CLICK => {
                if self.view.id != 0 {
                    self.requests.push(CellRequest::Use(self.view.id));
                }
                true
            }
            kind::RIGHT_CLICK => {
                if self.view.id != 0 {
                    let (x, y) = ui.screen_position(me);
                    self.requests.push(CellRequest::Menu(
                        self.view.id,
                        (x + e.data1 as f32, y + e.data2 as f32),
                    ));
                }
                true
            }
            kind::DRAG_ENTER => {
                self.over = Some(self.takes(e.payload.as_ref()).is_some());
                true
            }
            kind::DRAG_OVER => {
                self.over = None;
                true
            }
            kind::DROP_RELEASED => {
                self.over = None;
                if let Some((item, from)) = self.takes(e.payload.as_ref()) {
                    self.requests.push(CellRequest::Dropped {
                        item,
                        from,
                        container: self.container,
                        index: self.index,
                        onto: self.view.id,
                    });
                }
                true
            }
            _ => false,
        }
    }

    fn tick(&mut self, ui: &mut UiRoot, _me: ElementId, _dt: f64) {
        if self.over.is_some() && ui.drag_payload().is_none() {
            self.over = None;
        }
    }

    fn tooltip_text(&self, _element: &Element) -> Option<String> {
        (!self.view.name.is_empty() && self.look != CellLook::List).then(|| self.view.name.clone())
    }

    fn draw(&mut self, _ui: &mut UiRoot, me: ElementId, rc: &mut RenderContext<'_>) {
        let Some((w, h)) = _ui.element(me).map(|e| (e.width, e.height)) else {
            return;
        };
        let p = self.theme.palette;
        let v = &self.view;
        let round = CornerRadii::all(self.theme.metrics.slot_radius);
        let list = self.look == CellLook::List;
        let filled = v.id != 0;
        let fade = if v.faint || v.waiting { 0.5 } else { 1.0 };
        // The tile (in the list, the icon's tile at the left).
        let side = if list { h - 4.0 } else { h.min(w) };
        let (tx, ty) = if list {
            (2.0, 2.0)
        } else {
            ((w - side) * 0.5, 0.0)
        };
        if list {
            let bg = if v.selected {
                p.accent_tint
            } else if self.hovered {
                hex(0xFF_FFFF, 0.04)
            } else {
                Color::TRANSPARENT
            };
            rc.fill_rounded_rect(0.0, 0.0, w, h, round, bg);
        }
        let tile = if filled { hex(0, 0.42) } else { hex(0, 0.28) };
        let (bx, bw) = if list { (tx, side) } else { (0.0, w) };
        rc.fill_rounded_rect(bx, ty, bw, side, round, tile);
        if let Some(icon) = v.icon.filter(|_| filled) {
            let s = if self.look == CellLook::Compact {
                26.0
            } else {
                32.0
            };
            let tint = Color::rgba(1.0, 1.0, 1.0, fade);
            rc.sprite(
                icon,
                (bx + (bw - s) * 0.5).round(),
                (ty + (side - s) * 0.5).round(),
                s,
                s,
                (0.0, 0.0, 1.0, 1.0),
                tint,
            );
        }
        if v.cooldown > 0.0 && filled {
            let share = v.cooldown.min(1.0);
            let hh = side * share;
            rc.fill_rounded_rect(bx, ty + side - hh, bw, hh, round, hex(0, 0.55));
        }
        // The edge.
        let (edge, width) = if self.over == Some(true) || v.lit || v.selected && !list {
            (p.accent, 2.0)
        } else if self.hovered && filled {
            (p.accent, 1.0)
        } else if filled {
            (hex(0xC9_A062, 0.3), 1.0)
        } else {
            (hex(0xC9_A062, 0.14), 1.0)
        };
        if self.over == Some(true) || v.lit {
            rc.soft_shadow(bx, ty, bw, side, 4.0, 0.0, 10.0, hex(0xE0_B068, 0.45));
        }
        rc.stroke_rounded_rect(bx, ty, bw, side, round, edge, width);
        if self.pressed && self.hovered && filled {
            rc.fill_rounded_rect(bx, ty, bw, side, round, hex(0, 0.25));
        }
        if self.over == Some(false) {
            rc.fill_rounded_rect(bx, ty, bw, side, round, hex(0, 0.35));
        }
        if !filled {
            return;
        }
        if let Some(f) = &self.small {
            let shadow = p.text_shadow;
            if let Some(stamp) = &v.stamp {
                text::draw(
                    rc,
                    f,
                    stamp,
                    bx + 3.0,
                    ty + 1.0,
                    TextStyle::plain(p.accent).shadowed(shadow),
                );
            }
            if let Some(count) = v.count.filter(|_| !list) {
                let words = ac_ui::modern::numbers::compact(i64::from(count));
                let tw = text::measure(f, &words, 0.0);
                text::draw(
                    rc,
                    f,
                    &words,
                    (bx + bw - 3.0 - tw).round(),
                    ty + side - f.line_height - 1.0,
                    TextStyle::plain(p.text).shadowed(shadow),
                );
            }
        }
        if list {
            let x = bx + bw + 10.0;
            let room = (w - x - 8.0).max(0.0);
            if let Some(f) = &self.name {
                let name = match v.count {
                    Some(c) => format!("{} \u{d7}{c}", v.name),
                    None => v.name.clone(),
                };
                let name = super::fit(f, &name, room);
                let ink = v.name_color.unwrap_or(p.text);
                text::draw(
                    rc,
                    f,
                    &name,
                    x,
                    (h * 0.5 - f.line_height).round().max(2.0),
                    TextStyle::plain(Color::rgba(ink.r, ink.g, ink.b, ink.a * fade)),
                );
            }
            if let Some(f) = &self.small
                && !v.sub.is_empty()
            {
                let sub = super::fit(f, &v.sub, room);
                text::draw(
                    rc,
                    f,
                    &sub,
                    x,
                    (h * 0.5 + 1.0).round(),
                    TextStyle::plain(p.text_soft),
                );
            }
            if v.selected {
                rc.stroke_rounded_rect(0.0, 0.0, w, h, round, p.accent, 1.0);
            }
        }
    }
}
