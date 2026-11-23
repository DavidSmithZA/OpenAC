//! The salvage window (layout 0x2100000C): a row of the items the player
//! is about to salvage with a salvaging tool, and the button that does it.
//!
//! Using a salvaging tool the player carries opens the window on it. Items
//! are staged by dropping them from the inventory (a pack stages its
//! suitable contents), and leave the row by being dragged out or double
//! clicked; a right click examines one, as on any item cell. The button
//! sends the staged items, last staged first. The
//! staging is the window's own; it follows the window's copy of the object
//! table at each update: the tool gone or no longer the player's closes
//! the window, a staged item gone, no longer the player's or no longer
//! suitable leaves the row. Drags, drops and the button act at the
//! next update.

use ac_runtime::inventory::{ItemCommand, ObjectRef};
use ac_runtime::session::SessionCommand;

use crate::event::{DragAcceptance, ItemDragPayload, ItemDragSource};
use crate::host::PanelController;
use crate::icons::IconSource;
use crate::import::ImportedLayout;
use crate::panels::PanelOutput;
use crate::panels::external_container::apply_horizontal_bar_art;
use crate::panels::items::{ItemsView, bind_click, emit, fill_slot};
use crate::panels::salvage_list::{SalvageList, drag_stages};
use crate::root::{ElementMessage, UiRoot};
use crate::tree::ElementId;
use crate::widgets::item_slot::{
    DEFAULT_DRAG_ACCEPT_SPRITE, DEFAULT_DRAG_REJECT_SPRITE, slot_element,
};
use crate::widgets::{ItemList, ItemSlot, ScrollSource, Scrollbar};

/// The window's layout.
pub const LAYOUT_ID: u32 = 0x2100_000C;
/// Its root.
pub const ROOT_ID: u32 = 0x1000_0073;
/// The staged items' row.
pub const ITEM_LIST_ID: u32 = 0x1000_0074;
/// The row's scrollbar.
pub const SCROLLBAR_ID: u32 = 0x1000_0075;
/// The salvage button.
pub const SALVAGE_BUTTON_ID: u32 = 0x1000_0076;
/// The close button.
pub const CLOSE_BUTTON_ID: u32 = 0x1000_0078;

/// The content's smallest width, pixels.
pub const MINIMUM_CONTENT_WIDTH: f32 = 240.0;

pub use super::salvage_list::{CANNOT_SALVAGE_NOW, INVALID_LIST};

/// The salvage window's controller.
#[derive(Debug)]
pub struct SalvageController {
    window: String,
    list: ElementId,
    button: ElementId,
    close: Option<ElementId>,
    staging: SalvageList,
    queue: Vec<(ElementId, ElementMessage)>,
    salvage_requested: bool,
}

fn slot_object(ui: &UiRoot, cell: ElementId) -> u32 {
    ui.widget::<ItemSlot>(cell).map_or(0, |s| s.object_id)
}

fn cells(ui: &UiRoot, list: ElementId) -> Vec<ElementId> {
    ui.widget::<ItemList>(list)
        .map(|l| l.cells().to_vec())
        .unwrap_or_default()
}

/// The row's cells show `items` in order, empty cells after them.
fn shows(ui: &UiRoot, list: ElementId, items: &[u32]) -> bool {
    let cells = cells(ui, list);
    cells.len() >= items.len()
        && cells
            .iter()
            .enumerate()
            .all(|(i, c)| slot_object(ui, *c) == items.get(i).copied().unwrap_or(0))
}

impl SalvageController {
    /// Bind to the imported window registered as `window` (`None` without
    /// the row or the button); `empty_sprite` is the row's empty cell. The
    /// window starts hidden.
    pub fn bind(
        ui: &mut UiRoot,
        layout: &ImportedLayout,
        window: &str,
        empty_sprite: u32,
    ) -> Option<Self> {
        let list = layout
            .find(ITEM_LIST_ID)
            .filter(|l| ui.widget::<ItemList>(*l).is_some())?;
        let button = layout
            .find(SALVAGE_BUTTON_ID)
            .filter(|b| bind_click(ui, *b))?;
        let close = layout.find(CLOSE_BUTTON_ID).filter(|b| bind_click(ui, *b));
        ui.with::<ItemList, _>(list, |l, ui| {
            l.flush(ui);
            l.columns = 1;
            l.single_row = true;
            l.horizontal_scroll = true;
            l.cell_width = 32.0;
            l.cell_height = 32.0;
            l.fill_visible_empty_slots = true;
            if empty_sprite != 0 {
                l.set_cell_empty_sprite(ui, empty_sprite);
            }
            l.empty_slot_factory = Some(Box::new(|ui: &mut UiRoot| {
                ui.insert(
                    slot_element(),
                    ItemSlot {
                        allow_drag_source: false,
                        drag_accept_sprite: DEFAULT_DRAG_ACCEPT_SPRITE,
                        drag_reject_sprite: DEFAULT_DRAG_REJECT_SPRITE,
                        ..ItemSlot::default()
                    },
                )
            }));
            // The empty row is laid out once at bind (its empty cells take
            // drops before anything is staged); later updates make cells
            // only when the staged items change.
            l.layout_cells(ui, list);
        });
        if let Some(bar) = layout.find(SCROLLBAR_ID)
            && let Some(s) = ui.widget_mut::<Scrollbar>(bar)
        {
            s.model = ScrollSource::Element(list);
            apply_horizontal_bar_art(s);
        }
        ui.set_enabled(button, false);
        ui.hide_window(window);
        Some(Self {
            window: window.to_owned(),
            list,
            button,
            close,
            staging: SalvageList::default(),
            queue: Vec::new(),
            salvage_requested: false,
        })
    }

    /// Tell the session which items are staged when that changed: they
    /// are marked as in a trade while staged (moves, uses and merges refuse
    /// them; the inventory draws the trade overlay on them).
    fn sync_marks(&mut self, ui: &mut UiRoot) {
        if let Some(marks) = self.staging.marks() {
            emit(ui, PanelOutput::Session(marks));
        }
    }

    /// The staged items' row.
    #[must_use]
    pub fn list(&self) -> ElementId {
        self.list
    }

    /// The salvage button.
    #[must_use]
    pub fn button(&self) -> ElementId {
        self.button
    }

    /// The tool the window is open on (0: none).
    #[must_use]
    pub fn tool(&self) -> u32 {
        self.staging.tool()
    }

    /// The staged items, in staging order.
    pub fn staged(&self) -> impl Iterator<Item = u32> + '_ {
        self.staging.items().iter().copied()
    }

    /// How many items are staged.
    #[must_use]
    pub fn item_count(&self) -> usize {
        self.staging.items().len()
    }

    /// Open the window on `tool` (a salvaging tool the player carries):
    /// the staging starts empty.
    pub fn open(&mut self, ui: &mut UiRoot, tool: u32, view: &ItemsView<'_>) {
        if self.staging.open(tool, view) {
            ui.show_window(&self.window);
        }
    }

    /// `item` may be staged: the window is open on a usable tool, and the
    /// item is the player's, not the tool, not staged yet, and either a
    /// container holding items (packs in it do not count) or suitable for
    /// salvage (of the staged material when one material at a time is
    /// allowed).
    #[must_use]
    pub fn can_add(&self, item: u32, view: &ItemsView<'_>, allow_multiple: bool) -> bool {
        !self.salvage_requested && self.staging.can_add(item, view, allow_multiple)
    }

    /// Stage `item` (a pack stages its suitable contents, saying so).
    /// False when it may not be staged.
    pub fn add_item(
        &mut self,
        ui: &mut UiRoot,
        item: u32,
        view: &ItemsView<'_>,
        allow_multiple: bool,
    ) -> bool {
        if self.salvage_requested {
            return false;
        }
        let Some(notices) = self.staging.add(item, view, allow_multiple) else {
            return false;
        };
        for n in notices {
            emit(ui, PanelOutput::Notice(n));
        }
        true
    }

    /// Take `item` out of the row.
    pub fn remove_item(&mut self, item: u32) {
        self.staging.remove(item);
    }

    /// Salvage the staged items: every one still there, the player's
    /// and suitable, else the list is refused; the items go last
    /// staged first and the row empties. False when nothing was sent.
    pub fn salvage(&mut self, ui: &mut UiRoot, view: &ItemsView<'_>) -> bool {
        match self.staging.salvage(view) {
            Ok(Some(command)) => {
                emit(ui, PanelOutput::Session(command));
                true
            }
            Ok(None) => false,
            Err(words) => {
                emit(ui, PanelOutput::Notice(words.to_owned()));
                false
            }
        }
    }

    /// Close the window (its staging empties).
    pub fn close(&mut self, ui: &mut UiRoot) {
        self.forget();
        self.sync_marks(ui);
        ui.hide_window(&self.window);
    }

    fn forget(&mut self) {
        self.staging.forget();
        self.queue.clear();
        self.salvage_requested = false;
    }

    /// Follow the window's copy of the world: open on `opened` tools, act
    /// on what was queued, drop what is no longer valid, and show the row.
    pub fn update(
        &mut self,
        ui: &mut UiRoot,
        view: &ItemsView<'_>,
        opened: &[u32],
        allow_multiple: bool,
        icons: &mut dyn IconSource,
    ) {
        for &tool in opened {
            self.open(ui, tool, view);
        }
        if self.staging.tool() == 0 {
            self.queue.clear();
            self.salvage_requested = false;
            self.sync_marks(ui);
            return;
        }
        if !self.staging.follow(view) {
            self.close(ui);
            return;
        }
        let queued = std::mem::take(&mut self.queue);
        for (element, message) in queued {
            self.handle(ui, element, &message, view, allow_multiple);
        }
        if std::mem::take(&mut self.salvage_requested) {
            self.salvage(ui, view);
        }
        self.sync_marks(ui);
        self.populate(ui, view, icons);
    }

    fn handle(
        &mut self,
        ui: &mut UiRoot,
        element: ElementId,
        message: &ElementMessage,
        view: &ItemsView<'_>,
        allow_multiple: bool,
    ) {
        match message {
            ElementMessage::ItemDragLifted(p) if p.obj_id != 0 => self.remove_item(p.obj_id),
            ElementMessage::ItemDragEntered(p) => {
                let a = self.drag_over(p, view, allow_multiple);
                if let Some(s) = ui.widget_mut::<ItemSlot>(element) {
                    s.set_drag_acceptance(a);
                }
            }
            ElementMessage::ItemDropped(p)
                if self.drag_over(p, view, allow_multiple) == DragAcceptance::Accept =>
            {
                self.add_item(ui, p.obj_id, view, allow_multiple);
            }
            _ => {}
        }
    }

    /// Whether a drag over the row would stage its item.
    #[must_use]
    pub fn drag_over(
        &self,
        p: &ItemDragPayload,
        view: &ItemsView<'_>,
        allow_multiple: bool,
    ) -> DragAcceptance {
        if drag_stages(p) && self.can_add(p.obj_id, view, allow_multiple) {
            DragAcceptance::Accept
        } else {
            DragAcceptance::Reject
        }
    }

    /// Show the staged items (each with the trade overlay) and enable the
    /// button when there are some. The row's cells are made again only
    /// when the staged items changed; otherwise each cell is refreshed in
    /// place, so the cell under the pointer lives from a press to its
    /// release (a right click examines it, a double click or a drag takes
    /// it out) although the window is fed every frame.
    pub fn populate(&mut self, ui: &mut UiRoot, view: &ItemsView<'_>, icons: &mut dyn IconSource) {
        let list = self.list;
        let items = self.staging.items().to_vec();
        if !shows(ui, list, &items) {
            ui.with::<ItemList, _>(list, |l, ui| {
                l.defer_layout();
                l.flush(ui);
            });
            for (i, id) in items.iter().enumerate() {
                let mut s = ItemSlot {
                    slot_index: i32::try_from(i).unwrap_or(i32::MAX),
                    drag_accept_sprite: DEFAULT_DRAG_ACCEPT_SPRITE,
                    drag_reject_sprite: DEFAULT_DRAG_REJECT_SPRITE,
                    ..ItemSlot::default()
                };
                // Filled before it joins the list: the list trims empty
                // cells past its visible width when it lays out, and an
                // unfilled cell would count as one.
                fill_slot(&mut s, view.get(*id), view, icons);
                s.source_kind = ItemDragSource::Salvage;
                let cell = ui.insert(slot_element(), s);
                ui.with::<ItemList, _>(list, |l, ui| l.add_item(ui, list, cell));
            }
            ui.with::<ItemList, _>(list, |l, ui| l.end_defer_layout(ui, list));
        }
        for (cell, id) in cells(ui, list).into_iter().zip(&items) {
            if let Some(s) = ui.widget_mut::<ItemSlot>(cell) {
                fill_slot(s, view.get(*id), view, icons);
                s.show_trade_overlay = true;
            }
        }
        ui.set_enabled(self.button, !items.is_empty());
    }
}

impl PanelController for SalvageController {
    fn on_hidden(&mut self, ui: &mut UiRoot) {
        self.forget();
        self.sync_marks(ui);
    }

    fn on_element_message(
        &mut self,
        ui: &mut UiRoot,
        element: ElementId,
        message: &ElementMessage,
    ) -> bool {
        if Some(element) == self.close {
            if matches!(message, ElementMessage::Clicked { .. }) {
                self.close(ui);
            }
            return true;
        }
        if element == self.button {
            if matches!(message, ElementMessage::Clicked { .. }) {
                self.salvage_requested = true;
            }
            return true;
        }
        if ui.parent(element) != Some(self.list) && element != self.list {
            return false;
        }
        match message {
            // Right-click examines a staged item like any other item cell.
            ElementMessage::RightClicked => {
                let id = slot_object(ui, element);
                if id != 0 {
                    emit(ui, PanelOutput::SelectObject(id));
                    emit(
                        ui,
                        PanelOutput::Session(SessionCommand::Item(ItemCommand::Appraise(
                            ObjectRef::Guid(id),
                        ))),
                    );
                }
                true
            }
            ElementMessage::DoubleClicked => {
                let id = slot_object(ui, element);
                if id != 0 {
                    self.remove_item(id);
                }
                true
            }
            ElementMessage::ItemDragLifted(_)
            | ElementMessage::ItemDragEntered(_)
            | ElementMessage::ItemDropped(_) => {
                self.queue.push((element, message.clone()));
                true
            }
            _ => false,
        }
    }
}
