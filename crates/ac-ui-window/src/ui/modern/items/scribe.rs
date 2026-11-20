//! Writing an inscription: a field laid over a sheet's writable box (the
//! inventory's item details and Inspect). As in retail, what was written
//! goes out when the field is left, and only when it changed; leaving it
//! empty clears a signed inscription.

use ac_runtime::inventory::{ItemCommand, ObjectRef};
use ac_ui::UiRoot;
use ac_ui::draw::Color;
use ac_ui::geometry::AnchorEdges;
use ac_ui::modern::Horizon;
use ac_ui::modern::theme::hex;
use ac_ui::tree::{Element, ElementId};
use ac_ui::widgets::Field;

use super::sheet_view::SheetView;

/// The field and what it is writing.
#[derive(Debug)]
pub struct Scribe {
    /// The field (a child of the sheet).
    pub field: ElementId,
    /// The item the field holds the words of.
    item: u32,
    /// Its inscription as the sheet last showed it.
    seen: String,
    /// What the field started from (or last sent): writing the same
    /// again sends nothing.
    old: String,
    /// The field had the keyboard last frame.
    focused: bool,
}

impl Scribe {
    /// A hidden field over `sheet`, writing in `font`.
    pub fn mount(
        ui: &mut UiRoot,
        sheet: ElementId,
        theme: Horizon,
        font: Option<std::sync::Arc<ac_ui::UiFont>>,
    ) -> Self {
        let mut e = Element::at(0.0, 0.0, 10.0, 10.0)
            .with_anchors(AnchorEdges::NONE)
            .with_visible(false);
        Field::configure(&mut e);
        let mut f = Field::default();
        f.font = font;
        f.one_line = false;
        f.selectable = true;
        f.padding = 8.0;
        f.record_history = false;
        f.clear_on_submit = false;
        f.text_color = theme.palette.text;
        f.background_color = Color::TRANSPARENT;
        f.selection_color = hex(0xE0_B068, 0.35);
        f.set_editable_detached(&mut e, true);
        let field = ui.add(sheet, e, f);
        Self {
            field,
            item: 0,
            seen: String::new(),
            old: String::new(),
            focused: false,
        }
    }

    /// Each frame: the field follows the sheet's writable box (hidden
    /// when it has none or it is scrolled out of view) and takes the
    /// sheet's words while not being written in. What to send when the
    /// player has finished writing.
    pub fn tick(&mut self, ui: &mut UiRoot, sheet: ElementId) -> Option<ItemCommand> {
        let (w, h) = ui.element(sheet).map(|e| (e.width, e.height))?;
        let (subject, words, at) = {
            let s = ui.widget_mut::<SheetView>(sheet)?;
            let subject = s.sheet().subject;
            let words = s.writable_inscription().map(str::to_owned);
            let at = s.writing_box(w, h);
            (subject, words, at)
        };
        let focused = ui.keyboard_focus() == Some(self.field);
        let mut send = None;
        let gone = words.is_none() || at.is_none() || subject != self.item;
        if self.focused && (!focused || gone) {
            let text = ui
                .widget::<Field>(self.field)
                .map(Field::text)
                .unwrap_or_default();
            if self.item != 0 && text != self.old {
                send = Some(ItemCommand::Inscribe {
                    item: ObjectRef::Guid(self.item),
                    text: text.clone(),
                });
                self.old = text;
            }
            if focused {
                ui.set_keyboard_focus(None);
            }
        }
        self.focused = focused && !gone;
        if !self.focused {
            // Words just sent stay until the sheet shows others.
            let words = words.unwrap_or_default();
            if subject != self.item || words != self.seen {
                self.item = subject;
                self.seen.clone_from(&words);
                self.old.clone_from(&words);
                if let Some(f) = ui.widget_mut::<Field>(self.field) {
                    f.set_text(&words);
                }
            }
        }
        let shown = at.filter(|_| self.item != 0 && subject == self.item);
        if let (Some((x, y, bw, bh)), Some(e)) = (shown, ui.element_mut(self.field)) {
            e.left = x;
            e.top = y;
            e.width = bw;
            e.height = bh;
        }
        ui.set_visible(self.field, shown.is_some());
        if let Some(s) = ui.widget_mut::<SheetView>(sheet) {
            s.covered = shown.is_some();
        }
        send
    }
}
