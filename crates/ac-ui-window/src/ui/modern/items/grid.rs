//! A scrolling grid (or list) of item cells under optional group
//! headings. Only the cells in sight are elements: a pool of them is
//! handed out again as the grid scrolls, so a pack of hundreds costs what
//! a screenful does. A heading folds its group away when clicked.

use ac_ui::modern::tone;
use std::collections::HashSet;
use std::sync::Arc;

use ac_ui::draw::{ColorVertex, RenderContext};
use ac_ui::event::{ItemDragSource, UiEvent, kind};
use ac_ui::modern::Horizon;
use ac_ui::modern::text::{self, TextStyle};
use ac_ui::modern::theme::hex;
use ac_ui::shapes::CornerRadii;
use ac_ui::tree::{Element, ElementId, Widget};
use ac_ui::{UiFont, UiRoot};

use super::cell::{CellLook, CellRequest, CellView, ItemCell};

/// The gap between cells.
const GAP: f32 = 4.0;
/// A heading's height.
const HEADING: f32 = 22.0;
/// The room between groups.
const GROUP_GAP: f32 = 10.0;
/// The scrollbar's width and the room kept for it.
const BAR: f32 = 6.0;
const BAR_ROOM: f32 = 12.0;
/// One wheel notch, units.
const WHEEL: f32 = 48.0;

/// One group the grid shows.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct GridGroup {
    /// Its key (a closed group stays closed while its items change).
    pub key: u64,
    /// The heading (none: no heading).
    pub title: Option<String>,
    /// The heading's right side.
    pub meta: String,
    /// The items' cells, in order.
    pub cells: Vec<CellView>,
    /// Empty cells after them.
    pub empty: usize,
    /// The pack the group's cells stand for (none: a mixed group).
    pub container: Option<u32>,
}

/// The fonts a grid draws with.
#[derive(Debug, Clone, Default)]
pub struct GridFonts {
    /// Headings.
    pub heading: Option<Arc<UiFont>>,
    /// Counts, stamps, the list's second line, the headings' right side.
    pub small: Option<Arc<UiFont>>,
    /// The list's names.
    pub name: Option<Arc<UiFont>>,
}

/// A run of the grid, top to bottom.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Run {
    /// Group `group`'s heading at `y`.
    Heading { group: usize, y: f32 },
    /// Group `group`'s cells from `first`, `count` of them, at `y`.
    Cells {
        group: usize,
        first: usize,
        count: usize,
        y: f32,
    },
}

/// The grid.
#[derive(Debug)]
pub struct ItemGrid {
    fonts: GridFonts,
    groups: Vec<GridGroup>,
    /// How cells are drawn.
    look: CellLook,
    /// Where drags from it say they came from.
    source: ItemDragSource,
    /// Nothing is dropped on its cells.
    no_drops: bool,
    closed: HashSet<u64>,
    scroll: f32,
    runs: Vec<Run>,
    /// The columns and a cell's width, as last laid out.
    columns: usize,
    cell_width: f32,
    content: f32,
    pool: Vec<ElementId>,
    /// The size last laid out for.
    laid: (f32, f32),
    dirty: bool,
    requests: Vec<CellRequest>,
    /// The thumb is held: the pointer's offset into it.
    holding: Option<f32>,
}

impl ItemGrid {
    /// An empty grid in `theme`.
    #[must_use]
    pub fn new(fonts: GridFonts) -> Self {
        Self {
            fonts,
            groups: Vec::new(),
            look: CellLook::Grid,
            source: ItemDragSource::Inventory,
            no_drops: false,
            closed: HashSet::new(),
            scroll: 0.0,
            runs: Vec::new(),
            columns: 1,
            cell_width: 40.0,
            content: 0.0,
            pool: Vec::new(),
            laid: (0.0, 0.0),
            dirty: true,
            requests: Vec::new(),
            holding: None,
        }
    }

    /// Shows `groups` in grid `me`.
    pub fn set_groups(ui: &mut UiRoot, me: ElementId, groups: Vec<GridGroup>) {
        ui.with::<Self, _>(me, |g, ui| {
            if g.groups != groups {
                g.groups = groups;
                g.place(ui, me);
            }
        });
    }

    /// Draws grid `me`'s cells as `look`.
    pub fn set_look(ui: &mut UiRoot, me: ElementId, look: CellLook) {
        ui.with::<Self, _>(me, |g, ui| {
            if g.look != look {
                g.look = look;
                g.place(ui, me);
            }
        });
    }

    /// Where drags from grid `me` come from, and whether it takes drops.
    pub fn set_source(ui: &mut UiRoot, me: ElementId, source: ItemDragSource, drops: bool) {
        ui.with::<Self, _>(me, |g, ui| {
            g.source = source;
            g.no_drops = !drops;
            g.place(ui, me);
        });
    }

    /// Back to the top.
    pub fn scroll_home(ui: &mut UiRoot, me: ElementId) {
        ui.with::<Self, _>(me, |g, ui| {
            g.scroll = 0.0;
            g.place(ui, me);
        });
    }

    /// What grid `me`'s cells asked for since the window last looked.
    pub fn take_requests(ui: &mut UiRoot, me: ElementId) -> Vec<CellRequest> {
        let pool = ui
            .widget::<Self>(me)
            .map(|g| g.pool.clone())
            .unwrap_or_default();
        let mut out = ui
            .widget_mut::<Self>(me)
            .map(|g| std::mem::take(&mut g.requests))
            .unwrap_or_default();
        for c in pool {
            if let Some(cell) = ui.widget_mut::<ItemCell>(c) {
                out.append(&mut cell.requests);
            }
        }
        out
    }

    /// The groups shown.
    #[must_use]
    pub fn groups(&self) -> &[GridGroup] {
        &self.groups
    }

    fn room(&self) -> f32 {
        self.laid.1
    }

    fn most(&self) -> f32 {
        (self.content - self.room()).max(0.0)
    }

    /// Works out the runs for the grid's width, and puts a pooled cell on
    /// every cell in sight.
    fn place(&mut self, ui: &mut UiRoot, me: ElementId) {
        let Some((w, h)) = ui.element(me).map(|e| (e.width, e.height)) else {
            return;
        };
        self.laid = (w, h);
        self.dirty = false;
        let inner = (w - BAR_ROOM).max(1.0);
        let (columns, cell_width) = if self.look == CellLook::List {
            (1, inner)
        } else {
            let n = ((inner + GAP) / (self.look.least_width() + GAP))
                .floor()
                .max(1.0);
            (n as usize, ((inner - GAP * (n - 1.0)) / n).floor())
        };
        self.columns = columns;
        self.cell_width = cell_width;
        let row = self.look.height() + GAP;
        let mut runs = Vec::new();
        let mut y = 0.0;
        for (i, group) in self.groups.iter().enumerate() {
            if i > 0 {
                y += GROUP_GAP - GAP;
            }
            if group.title.is_some() {
                runs.push(Run::Heading { group: i, y });
                y += HEADING + 6.0;
                if self.closed.contains(&group.key) {
                    continue;
                }
            }
            let empty = if self.look == CellLook::List {
                0
            } else {
                group.empty
            };
            let n = group.cells.len() + empty;
            let mut first = 0;
            while first < n {
                let count = columns.min(n - first);
                runs.push(Run::Cells {
                    group: i,
                    first,
                    count,
                    y,
                });
                y += row;
                first += count;
            }
        }
        self.content = (y - GAP).max(0.0);
        self.runs = runs;
        self.scroll = self.scroll.clamp(0.0, self.most());
        // The cells in sight.
        let top = self.scroll;
        let bottom = top + h;
        let mut used = 0;
        let runs = self.runs.clone();
        for run in runs {
            let Run::Cells {
                group,
                first,
                count,
                y,
            } = run
            else {
                continue;
            };
            if y + row < top || y > bottom {
                continue;
            }
            for k in 0..count {
                let index = first + k;
                let id = if used < self.pool.len() {
                    self.pool[used]
                } else {
                    let c = ui.add(
                        me,
                        Element::at(0.0, 0.0, 40.0, 40.0)
                            .with_anchors(ac_ui::geometry::AnchorEdges::NONE),
                        ItemCell::new(
                            Horizon::current(),
                            self.fonts.small.clone(),
                            self.fonts.name.clone(),
                        ),
                    );
                    self.pool.push(c);
                    c
                };
                used += 1;
                let g = &self.groups[group];
                let view = g.cells.get(index).cloned().unwrap_or_default();
                if let Some(e) = ui.element_mut(id) {
                    e.left = k as f32 * (cell_width + GAP);
                    e.top = (y - top).round();
                    e.width = cell_width;
                    e.height = self.look.height();
                    e.visible = true;
                }
                if let Some(cell) = ui.widget_mut::<ItemCell>(id) {
                    cell.view = view;
                    cell.look = self.look;
                    cell.source = self.source;
                    cell.container = g.container;
                    cell.index = index;
                    cell.no_drops = self.no_drops;
                }
            }
        }
        for &c in &self.pool[used..] {
            if let Some(e) = ui.element_mut(c) {
                e.visible = false;
            }
            if let Some(cell) = ui.widget_mut::<ItemCell>(c) {
                cell.view = CellView::default();
            }
        }
    }

    fn scroll_by(&mut self, ui: &mut UiRoot, me: ElementId, by: f32) {
        let next = (self.scroll + by).clamp(0.0, self.most());
        if next != self.scroll {
            self.scroll = next;
            self.place(ui, me);
        }
    }

    /// The thumb's top and height in the grid.
    fn thumb(&self) -> Option<(f32, f32)> {
        let room = self.room();
        if self.content <= room || room <= 0.0 {
            return None;
        }
        let height = (room * room / self.content).max(18.0);
        let top = (room - height) * (self.scroll / self.most());
        Some((top, height))
    }
}

impl Widget for ItemGrid {
    fn clips_children(&self) -> bool {
        true
    }

    fn hit_test(&self, _element: &Element, _x: f32, _y: f32) -> bool {
        true
    }

    fn handles_click(&self, _element: &Element) -> bool {
        true
    }

    fn on_event(&mut self, ui: &mut UiRoot, me: ElementId, e: &UiEvent) -> bool {
        match e.kind {
            kind::SCROLL => {
                let by = if e.data0 > 0 { -WHEEL } else { WHEEL };
                self.scroll_by(ui, me, by);
                true
            }
            kind::MOUSE_DOWN if e.target == Some(me) => {
                let (x, y) = (e.data1 as f32, e.data2 as f32);
                if x >= self.laid.0 - BAR_ROOM
                    && let Some((top, height)) = self.thumb()
                {
                    if y >= top && y <= top + height {
                        self.holding = Some(y - top);
                    } else {
                        let page = if y < top { -self.room() } else { self.room() };
                        self.scroll_by(ui, me, page);
                    }
                    ui.set_capture(me);
                }
                true
            }
            kind::MOUSE_MOVE if e.target == Some(me) => {
                if let (Some(off), Some((_, height))) = (self.holding, self.thumb()) {
                    let track = (self.room() - height).max(1.0);
                    let top = e.data2 as f32 - off;
                    let at = (top / track).clamp(0.0, 1.0) * self.most();
                    self.scroll_by(ui, me, at - self.scroll);
                }
                false
            }
            kind::MOUSE_UP | kind::CAPTURE_CHANGED => {
                self.holding = None;
                false
            }
            kind::CLICK if e.target == Some(me) => {
                let y = e.data2 as f32 + self.scroll;
                let hit = self.runs.iter().find_map(|r| match *r {
                    Run::Heading { group, y: top } if y >= top && y < top + HEADING => Some(group),
                    _ => None,
                });
                if let Some(g) = hit {
                    let key = self.groups[g].key;
                    if !self.closed.remove(&key) {
                        self.closed.insert(key);
                    }
                    self.place(ui, me);
                }
                true
            }
            _ => false,
        }
    }

    fn tick(&mut self, ui: &mut UiRoot, me: ElementId, _dt: f64) {
        let size = ui.element(me).map(|e| (e.width, e.height));
        if self.dirty || size.is_some_and(|s| s != self.laid) {
            self.place(ui, me);
        }
    }

    fn draw(&mut self, _ui: &mut UiRoot, _me: ElementId, rc: &mut RenderContext<'_>) {
        let p = Horizon::current().palette;
        let (w, h) = self.laid;
        let inner = (w - BAR_ROOM).max(1.0);
        if let Some(f) = &self.fonts.heading {
            for run in &self.runs {
                let Run::Heading { group, y } = *run else {
                    continue;
                };
                let y = y - self.scroll;
                if y + HEADING < 0.0 || y > h {
                    continue;
                }
                let g = &self.groups[group];
                let open = !self.closed.contains(&g.key);
                let mid = y + HEADING * 0.5;
                // The chevron.
                let c = p.text_soft;
                let pts: [(f32, f32); 3] = if open {
                    [(1.0, mid - 2.0), (9.0, mid - 2.0), (5.0, mid + 3.0)]
                } else {
                    [(3.0, mid - 4.0), (8.0, mid), (3.0, mid + 4.0)]
                };
                let v = |(x, y): (f32, f32)| ColorVertex { x, y, color: c };
                let _ = rc.fill_convex_polygon(&[v(pts[0]), v(pts[1]), v(pts[2])]);
                let title = g.title.as_deref().unwrap_or_default();
                let tw = text::measure(f, title, 0.8);
                text::draw(
                    rc,
                    f,
                    title,
                    16.0,
                    (mid - f.line_height * 0.5).round(),
                    TextStyle::plain(tone::warm(1.0)).tracked(0.8),
                );
                let meta_w = self
                    .fonts
                    .small
                    .as_ref()
                    .map_or(0.0, |s| text::measure(s, &g.meta, 0.0));
                if let Some(s) = &self.fonts.small {
                    text::draw(
                        rc,
                        s,
                        &g.meta,
                        (inner - meta_w).round(),
                        (mid - s.line_height * 0.5).round(),
                        TextStyle::plain(p.text_soft),
                    );
                }
                let x0 = 16.0 + tw + 8.0;
                let x1 = inner - meta_w - 8.0;
                if x1 > x0 {
                    rc.fill(x0, mid.round(), x1 - x0, 1.0, tone::bronze(0.25));
                }
            }
        }
        if let Some((top, height)) = self.thumb() {
            let x = w - BAR;
            rc.fill_rounded_rect(x, 0.0, BAR, h, CornerRadii::all(3.0), hex(0xFF_FFFF, 0.06));
            rc.fill_rounded_rect(
                x,
                top,
                BAR,
                height,
                CornerRadii::all(3.0),
                tone::bronze(0.7),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn group(key: u64, title: Option<&str>, n: u32, empty: usize) -> GridGroup {
        GridGroup {
            key,
            title: title.map(str::to_owned),
            meta: String::new(),
            cells: (1..=n)
                .map(|id| CellView {
                    id,
                    ..CellView::default()
                })
                .collect(),
            empty,
            container: Some(7),
        }
    }

    fn mount() -> (UiRoot, ElementId) {
        let mut ui = UiRoot::new();
        let root = ui.root();
        // Eight columns of 38 + 4 fit 352 - 12.
        let g = ui.add(
            root,
            Element::at(0.0, 0.0, 352.0, 200.0),
            ItemGrid::new(GridFonts::default()),
        );
        (ui, g)
    }

    fn shown(ui: &UiRoot, g: ElementId) -> Vec<(u32, usize, f32)> {
        ui.children(g)
            .into_iter()
            .filter(|&c| ui.element(c).is_some_and(|e| e.visible))
            .filter_map(|c| {
                let cell = ui.widget::<ItemCell>(c)?;
                Some((cell.view.id, cell.index, ui.element(c)?.top))
            })
            .collect()
    }

    #[test]
    fn only_the_cells_in_sight_are_elements() {
        let (mut ui, g) = mount();
        ItemGrid::set_groups(&mut ui, g, vec![group(1, None, 30, 72)]);
        let cells = shown(&ui, g);
        // 200 high, rows of 44: five rows in sight (the fifth in part).
        assert_eq!(cells.len(), 40);
        assert_eq!(cells[0], (1, 0, 0.0));
        assert_eq!(cells[8], (9, 8, 44.0));
        // Past the items, empty slots.
        assert_eq!(cells[35].0, 0);
        // Scrolled down a notch, the pool is handed out again.
        ui.with::<ItemGrid, _>(g, |w, ui| w.scroll_by(ui, g, WHEEL));
        let cells = shown(&ui, g);
        assert_eq!(cells.len(), 40);
        assert_eq!(cells[0], (9, 8, -4.0));
        assert_eq!(ui.children(g).len(), 40);
    }

    #[test]
    fn a_heading_folds_its_group() {
        let (mut ui, g) = mount();
        ItemGrid::set_groups(
            &mut ui,
            g,
            vec![group(1, Some("MAIN"), 3, 0), group(2, Some("PACK"), 2, 0)],
        );
        assert_eq!(shown(&ui, g).len(), 5);
        let click = UiEvent::new(0, g, kind::CLICK).with_data(0, 20, 4);
        ui.send_event(g, &click);
        let cells = shown(&ui, g);
        assert_eq!(cells.len(), 2);
        assert_eq!(cells[0].1, 0);
    }

    #[test]
    fn the_list_has_one_row_per_item_and_no_empty_slots() {
        let (mut ui, g) = mount();
        ItemGrid::set_look(&mut ui, g, CellLook::List);
        ItemGrid::set_groups(&mut ui, g, vec![group(1, None, 3, 20)]);
        let cells = shown(&ui, g);
        assert_eq!(
            cells.iter().map(|c| c.2).collect::<Vec<_>>(),
            [0.0, 44.0, 88.0]
        );
    }
}
