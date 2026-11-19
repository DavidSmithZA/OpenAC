//! Adding (or changing) a recall of the player's own: a name, the command
//! (sent as if typed in chat: any / or @ command the server knows), an
//! icon, and whether it shows on this server only or on every server.

use std::sync::Arc;

use ac_host::settings::CustomRecall;
use ac_ui::draw::{Color, RenderContext};
use ac_ui::event::{UiEvent, kind};
use ac_ui::geometry::AnchorEdges;
use ac_ui::modern::Horizon;
use ac_ui::modern::text::{self, TextStyle};
use ac_ui::modern::theme::hex;
use ac_ui::shapes::CornerRadii;
use ac_ui::tree::{Element, ElementId, Widget};
use ac_ui::widgets::Field;
use ac_ui::{UiFont, UiRoot};

use super::recall::{OWN_ICONS, own_icon, recall_mark};

/// The dialog's width and height.
pub const WIDTH: f32 = 340.0;
pub const HEIGHT: f32 = 344.0;
const HEAD: f32 = 40.0;
const INSET: f32 = 14.0;
const FIELD_HEIGHT: f32 = 28.0;
const NAME_TOP: f32 = 70.0;
const COMMAND_TOP: f32 = 126.0;
const ICONS_TOP: f32 = 202.0;
const TILE: f32 = 30.0;
const SCOPE_TOP: f32 = 258.0;
const BUTTONS_TOP: f32 = HEIGHT - INSET - 28.0;
/// The most characters a name or a command takes.
const MAX_CHARACTERS: usize = 64;

/// How the dialog was left.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DialogOutcome {
    /// Cancelled.
    Cancelled,
    /// Added (none) or changed (its place in the saved list).
    Done(Option<usize>, CustomRecall),
}

/// The fonts the dialog uses.
#[derive(Debug, Clone, Default)]
pub struct DialogFonts {
    /// The title and the buttons.
    pub title: Option<Arc<UiFont>>,
    /// Labels and field text.
    pub body: Option<Arc<UiFont>>,
    /// Hints.
    pub caption: Option<Arc<UiFont>>,
}

/// The dialog.
#[derive(Debug)]
pub struct RecallDialog {
    theme: Horizon,
    fonts: DialogFonts,
    name: ElementId,
    command: ElementId,
    /// The icon picked.
    icon: String,
    /// Shown on every server.
    every_server: bool,
    /// The server played on (none: unknown, so every server).
    server: Option<String>,
    /// The recall changed (none: a new one).
    editing: Option<usize>,
    /// Why Add did nothing.
    problem: Option<&'static str>,
    hovered: Option<Hit>,
    /// How the player left it, for the window to act on.
    pub outcome: Option<DialogOutcome>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Hit {
    Icon(usize),
    ThisServer,
    EveryServer,
    Cancel,
    Done,
}

impl RecallDialog {
    /// Mounts the dialog, hidden, under `parent`, with its two fields.
    pub fn mount(
        ui: &mut UiRoot,
        parent: ElementId,
        theme: Horizon,
        fonts: DialogFonts,
    ) -> ElementId {
        let element = Element::at(0.0, 0.0, WIDTH, HEIGHT)
            .with_visible(false)
            .with_anchors(AnchorEdges::NONE);
        // The fields go in once the dialog exists; until then they name the
        // parent.
        let field = |ui: &mut UiRoot, top: f32, owner: ElementId| {
            let mut e = Element::at(
                INSET + 1.0,
                top + 1.0,
                WIDTH - 2.0 * INSET - 2.0,
                FIELD_HEIGHT - 2.0,
            )
            .with_anchors(AnchorEdges::NONE);
            Field::configure(&mut e);
            e.tab_stop = true;
            let mut f = Field::default();
            f.font = fonts.body.clone();
            f.one_line = true;
            f.selectable = true;
            f.record_history = false;
            f.max_characters = MAX_CHARACTERS;
            f.padding = 6.0;
            f.text_color = theme.palette.text;
            f.background_color = Color::TRANSPARENT;
            f.selection_color = hex(0xE0_B068, 0.35);
            f.set_editable_detached(&mut e, true);
            ui.add(owner, e, f)
        };
        let me = ui.add(
            parent,
            element,
            RecallDialog {
                theme,
                fonts: fonts.clone(),
                name: parent,
                command: parent,
                icon: OWN_ICONS[0].to_owned(),
                every_server: false,
                server: None,
                editing: None,
                problem: None,
                hovered: None,
                outcome: None,
            },
        );
        let name = field(ui, NAME_TOP, me);
        let command = field(ui, COMMAND_TOP, me);
        if let Some(d) = ui.widget_mut::<RecallDialog>(me) {
            d.name = name;
            d.command = command;
        }
        me
    }

    /// Shows the dialog for a new recall (`editing` none) or for changing
    /// `recall`, on `server`; the name field takes the keyboard.
    pub fn show(
        ui: &mut UiRoot,
        me: ElementId,
        editing: Option<usize>,
        recall: &CustomRecall,
        server: Option<String>,
    ) {
        let Some(d) = ui.widget_mut::<RecallDialog>(me) else {
            return;
        };
        d.editing = editing;
        d.icon = if recall.icon.is_empty() {
            OWN_ICONS[0].to_owned()
        } else {
            recall.icon.clone()
        };
        d.every_server = server.is_none() || (editing.is_some() && recall.server.is_none());
        d.server = server;
        d.problem = None;
        d.outcome = None;
        let (name, command) = (d.name, d.command);
        for (id, text) in [(name, &recall.name), (command, &recall.command)] {
            if let Some(f) = ui.widget_mut::<Field>(id) {
                f.set_text(text);
            }
        }
        ui.set_visible(me, true);
        ui.set_keyboard_focus(Some(name));
    }

    fn hit(&self, x: f32, y: f32) -> Option<Hit> {
        let within = |left: f32, top: f32, w: f32, h: f32| {
            (left..left + w).contains(&x) && (top..top + h).contains(&y)
        };
        for i in 0..OWN_ICONS.len() {
            if within(INSET + i as f32 * (TILE + 6.0), ICONS_TOP, TILE, TILE) {
                return Some(Hit::Icon(i));
            }
        }
        let half = (WIDTH - 2.0 * INSET - 6.0) * 0.5;
        if within(INSET, SCOPE_TOP, half, 26.0) && self.server.is_some() {
            return Some(Hit::ThisServer);
        }
        if within(INSET + half + 6.0, SCOPE_TOP, half, 26.0) {
            return Some(Hit::EveryServer);
        }
        let (cancel, done) = Self::button_rects();
        if within(cancel.0, BUTTONS_TOP, cancel.1, 28.0) {
            return Some(Hit::Cancel);
        }
        if within(done.0, BUTTONS_TOP, done.1, 28.0) {
            return Some(Hit::Done);
        }
        None
    }

    fn button_rects() -> ((f32, f32), (f32, f32)) {
        let done = (WIDTH - INSET - 84.0, 84.0);
        let cancel = (done.0 - 8.0 - 84.0, 84.0);
        (cancel, done)
    }

    fn finish(&mut self, ui: &mut UiRoot, me: ElementId) {
        let text = |id| {
            ui.widget::<Field>(id)
                .map(|f| f.text().trim().to_owned())
                .unwrap_or_default()
        };
        let (name, command) = (text(self.name), text(self.command));
        if name.is_empty() || command.is_empty() {
            self.problem = Some("It needs a name and a command.");
            return;
        }
        let recall = CustomRecall {
            name,
            command,
            icon: self.icon.clone(),
            server: if self.every_server {
                None
            } else {
                self.server.clone()
            },
        };
        self.outcome = Some(DialogOutcome::Done(self.editing, recall));
        Self::hide(ui, me);
    }

    fn hide(ui: &mut UiRoot, me: ElementId) {
        ui.set_keyboard_focus(None);
        ui.set_visible(me, false);
    }

    fn draw_toggle(
        &self,
        rc: &mut RenderContext<'_>,
        left: f32,
        w: f32,
        word: &str,
        on: bool,
        live: bool,
    ) {
        let p = self.theme.palette;
        let radius = CornerRadii::all(self.theme.metrics.radius);
        let fill = if on { p.accent_tint } else { hex(0, 0.3) };
        rc.fill_rounded_rect(left, SCOPE_TOP, w, 26.0, radius, fill);
        let edge = if on { p.accent } else { p.trim_faint };
        rc.stroke_rounded_rect(left, SCOPE_TOP, w, 26.0, radius, edge, 1.0);
        if let Some(f) = &self.fonts.body {
            let ink = if !live {
                p.muted
            } else if on {
                p.accent
            } else {
                p.text
            };
            let word = super::target::fit(f, word, w - 12.0);
            let tw = text::measure(f, &word, 0.0);
            let ty = SCOPE_TOP + ((26.0 - f.line_height) * 0.5).round();
            text::draw(
                rc,
                f,
                &word,
                left + (w - tw) * 0.5,
                ty,
                TextStyle::plain(ink),
            );
        }
    }
}

impl Widget for RecallDialog {
    fn hit_test(&self, _element: &Element, _x: f32, _y: f32) -> bool {
        true
    }

    fn handles_click(&self, _element: &Element) -> bool {
        true
    }

    fn on_event(&mut self, ui: &mut UiRoot, me: ElementId, e: &UiEvent) -> bool {
        if e.target != Some(me) {
            return false;
        }
        let (x, y) = (e.data1 as f32, e.data2 as f32);
        match e.kind {
            kind::MOUSE_MOVE | kind::HOVER_ENTER => {
                self.hovered = self.hit(x, y);
                true
            }
            kind::HOVER_LEAVE => {
                self.hovered = None;
                true
            }
            kind::CLICK => {
                match self.hit(x, y) {
                    Some(Hit::Icon(i)) => OWN_ICONS[i].clone_into(&mut self.icon),
                    Some(Hit::ThisServer) => self.every_server = false,
                    Some(Hit::EveryServer) => self.every_server = true,
                    Some(Hit::Cancel) => {
                        self.outcome = Some(DialogOutcome::Cancelled);
                        Self::hide(ui, me);
                    }
                    Some(Hit::Done) => self.finish(ui, me),
                    None => {}
                }
                true
            }
            _ => true,
        }
    }

    fn draw(&mut self, _ui: &mut UiRoot, _me: ElementId, rc: &mut RenderContext<'_>) {
        let p = self.theme.palette;
        let (w, h) = (WIDTH, HEIGHT);
        let radius = CornerRadii::all(self.theme.metrics.window_radius);
        rc.soft_shadow(0.0, 0.0, w, h, 6.0, 12.0, 30.0, p.shadow);
        rc.fill_rounded_rect(0.0, 0.0, w, h, radius, p.popup);
        rc.stroke_rounded_rect(0.0, 0.0, w, h, radius, p.trim, 1.0);
        rc.fill(0.0, HEAD - 1.0, w, 1.0, p.trim_faint);
        recall_mark(rc, INSET, (HEAD - 14.0) * 0.5, p.accent);
        if let Some(f) = &self.fonts.title {
            let title = if self.editing.is_some() {
                "Change a recall"
            } else {
                "Add a recall"
            };
            let ty = ((HEAD - f.line_height) * 0.5).round();
            text::draw(rc, f, title, INSET + 22.0, ty, TextStyle::plain(p.text));
        }
        let label = |rc: &mut RenderContext<'_>, words: &str, top: f32| {
            if let Some(f) = &self.fonts.body {
                text::draw(
                    rc,
                    f,
                    words,
                    INSET,
                    top - f.line_height - 4.0,
                    TextStyle::plain(p.text_soft),
                );
            }
        };
        label(rc, "Name", NAME_TOP);
        label(rc, "Command", COMMAND_TOP);
        label(rc, "Icon", ICONS_TOP);
        label(rc, "Show it on", SCOPE_TOP);
        // The fields' wells.
        for top in [NAME_TOP, COMMAND_TOP] {
            let fw = w - 2.0 * INSET;
            rc.fill_rounded_rect(INSET, top, fw, FIELD_HEIGHT, CornerRadii::all(3.0), p.well);
            rc.stroke_rounded_rect(
                INSET,
                top,
                fw,
                FIELD_HEIGHT,
                CornerRadii::all(3.0),
                p.trim_faint,
                1.0,
            );
        }
        if let Some(f) = &self.fonts.caption {
            text::draw(
                rc,
                f,
                "Sent as if you typed it in chat: any / or @ command the server knows.",
                INSET,
                COMMAND_TOP + FIELD_HEIGHT + 5.0,
                TextStyle::plain(p.muted),
            );
        }
        for (i, icon) in OWN_ICONS.iter().enumerate() {
            let left = INSET + i as f32 * (TILE + 6.0);
            let on = self.icon == *icon;
            let fill = if on {
                p.accent_tint
            } else if self.hovered == Some(Hit::Icon(i)) {
                hex(0x2A_2116, 0.85)
            } else {
                hex(0, 0.3)
            };
            rc.fill_rounded_rect(left, ICONS_TOP, TILE, TILE, CornerRadii::all(3.0), fill);
            let edge = if on { p.accent } else { p.trim_faint };
            rc.stroke_rounded_rect(
                left,
                ICONS_TOP,
                TILE,
                TILE,
                CornerRadii::all(3.0),
                edge,
                1.0,
            );
            own_icon(
                rc,
                icon,
                left + (TILE - 14.0) * 0.5,
                ICONS_TOP + (TILE - 14.0) * 0.5,
                p.accent,
            );
        }
        let half = (w - 2.0 * INSET - 6.0) * 0.5;
        let this = match &self.server {
            Some(server) => format!("{server} only"),
            None => "This server only".to_owned(),
        };
        self.draw_toggle(
            rc,
            INSET,
            half,
            &this,
            !self.every_server,
            self.server.is_some(),
        );
        self.draw_toggle(
            rc,
            INSET + half + 6.0,
            half,
            "Every server",
            self.every_server,
            true,
        );
        if let (Some(problem), Some(f)) = (self.problem, &self.fonts.caption) {
            text::draw(
                rc,
                f,
                problem,
                INSET,
                SCOPE_TOP + 34.0,
                TextStyle::plain(p.bad_text),
            );
        }
        let (cancel, done) = Self::button_rects();
        let radius = CornerRadii::all(self.theme.metrics.radius);
        let hover = |hit| self.hovered == Some(hit);
        rc.fill_rounded_rect(
            cancel.0,
            BUTTONS_TOP,
            cancel.1,
            28.0,
            radius,
            if hover(Hit::Cancel) {
                hex(0x2A_2116, 0.85)
            } else {
                hex(0, 0.3)
            },
        );
        rc.stroke_rounded_rect(
            cancel.0,
            BUTTONS_TOP,
            cancel.1,
            28.0,
            radius,
            p.trim_soft,
            1.0,
        );
        let done_fill = if hover(Hit::Done) {
            p.accent
        } else {
            p.accent_fill
        };
        rc.fill_rounded_rect(done.0, BUTTONS_TOP, done.1, 28.0, radius, done_fill);
        if let Some(f) = &self.fonts.title {
            let ty = BUTTONS_TOP + ((28.0 - f.line_height) * 0.5).round();
            for (rect, word, ink) in [
                (cancel, "Cancel", p.text),
                (
                    done,
                    if self.editing.is_some() {
                        "Save"
                    } else {
                        "Add"
                    },
                    p.accent_ink,
                ),
            ] {
                let tw = text::measure(f, word, 0.0);
                text::draw(
                    rc,
                    f,
                    word,
                    rect.0 + (rect.1 - tw) * 0.5,
                    ty,
                    TextStyle::plain(ink),
                );
            }
        }
    }
}
