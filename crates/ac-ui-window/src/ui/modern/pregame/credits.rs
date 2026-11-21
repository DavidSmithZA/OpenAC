//! The credits on Modern, from character selection: the game's pictures
//! rise on the left and its credit text on the right, over the dimmed
//! dusk of character selection. Any key or click, "Back to your
//! characters", or the end of the text returns. The text, the pictures
//! and the pace are the game's own, as retail shows them.

use std::sync::Arc;

use ac_ui::draw::{Color, RenderContext, TextureRef};
use ac_ui::event::{UiEvent, kind};
use ac_ui::modern::Horizon;
use ac_ui::modern::text::{self, TextStyle};
use ac_ui::modern::theme::hex;
use ac_ui::pregame::credits::{CreditsResources, duration_seconds};
use ac_ui::shapes::CornerRadii;
use ac_ui::tree::{Element, ElementId, Widget};
use ac_ui::{UiFont, UiRoot};

use super::scenery::{self, Sky};
use crate::ui::modern::hud::lines::stroke;
use crate::ui::modern::kit::{self, Rect, SOFT, bronze, gold, put_mid};

/// The fonts the credits draw with.
#[derive(Debug, Clone, Default)]
pub struct CreditsFonts {
    /// The title.
    pub title: Option<Arc<UiFont>>,
    /// The credit text.
    pub text: Option<Arc<UiFont>>,
    /// The button and the note.
    pub small: Option<Arc<UiFont>>,
}

const DIAMOND: &[&[(f32, f32)]] = &[
    &[
        (12.0, 2.0),
        (21.0, 12.0),
        (12.0, 22.0),
        (3.0, 12.0),
        (12.0, 2.0),
    ],
    &[
        (12.0, 7.0),
        (16.0, 12.0),
        (12.0, 17.0),
        (8.0, 12.0),
        (12.0, 7.0),
    ],
];
const BACK: &[&[(f32, f32)]] = &[&[(15.0, 6.0), (9.0, 12.0), (15.0, 18.0)]];

/// The width of the picture column and of the text column.
const PICTURES_W: f32 = 440.0;
const TEXT_W: f32 = 560.0;
/// The space between pictures.
const PICTURE_GAP: f32 = 28.0;
/// Where the text starts, a share of the screen's height from the top.
const TEXT_START: f32 = 0.62;

/// The full-screen credits.
#[derive(Debug)]
pub struct CreditsView {
    theme: Horizon,
    fonts: CreditsFonts,
    /// What they show.
    pub resources: Option<CreditsResources>,
    /// The screen's size.
    pub size: (f32, f32),
    /// The window's clock when they opened, seconds.
    pub started: f64,
    /// The window's clock now.
    pub clock: f64,
    /// Asked to return since the window last looked.
    pub leave: bool,
    lines: Vec<String>,
    wrapped_for: Option<(f32, usize)>,
    hover: Option<(f32, f32)>,
}

impl CreditsView {
    /// Credits in `theme`.
    #[must_use]
    pub fn new(theme: Horizon, fonts: CreditsFonts) -> Self {
        Self {
            theme,
            fonts,
            resources: None,
            size: (1440.0, 900.0),
            started: 0.0,
            clock: 0.0,
            leave: false,
            lines: Vec::new(),
            wrapped_for: None,
            hover: None,
        }
    }

    /// "Back to your characters".
    #[must_use]
    pub fn back_rect(&self) -> Rect {
        let w = kit::button_width("Back to your characters", self.fonts.small.as_ref()) + 22.0;
        (self.size.0 - 32.0 - w, 23.0, w, 34.0)
    }

    fn line_height(&self) -> f32 {
        self.fonts
            .text
            .as_ref()
            .map_or(18.0, |f| f.line_height + 4.0)
    }

    fn wrap(&mut self) {
        let Some(font) = self.fonts.text.clone() else {
            return;
        };
        let width = TEXT_W.min(self.size.0 * 0.45);
        let pieces = self
            .resources
            .as_ref()
            .map_or(0, |r| r.text_fragments.len());
        if self.wrapped_for == Some((width, pieces)) {
            return;
        }
        self.wrapped_for = Some((width, pieces));
        self.lines.clear();
        let all: String = self
            .resources
            .as_ref()
            .map(|r| r.text_fragments.concat())
            .unwrap_or_default();
        for paragraph in all.split('\n') {
            if paragraph.trim().is_empty() {
                self.lines.push(String::new());
            } else {
                self.lines.extend(kit::wrap(&font, paragraph.trim(), width));
            }
        }
    }

    /// How far through the text they are, 0 to 1.
    #[must_use]
    pub fn progress(&self) -> f32 {
        let Some(r) = &self.resources else {
            return 1.0;
        };
        let field = self.size.1 * TEXT_START;
        let text_h = self.lines.len() as f32 * self.line_height();
        let duration = duration_seconds(r.section_seconds, field, text_h, r.text_fragments.len());
        if duration <= 0.0 {
            return 1.0;
        }
        (((self.clock - self.started).max(0.0) / duration) as f32).clamp(0.0, 1.0)
    }

    fn draw_pictures(&self, rc: &mut RenderContext<'_>, travel: f32) {
        let Some(r) = &self.resources else {
            return;
        };
        if r.picture_ids.is_empty() {
            return;
        }
        let (w, h) = self.size;
        let x0 = (w * 0.11).min(160.0);
        let col_w = PICTURES_W.min(w * 0.32);
        rc.push_clip(x0, 0.0, col_w, h);
        // The strip repeats its pictures; each keeps its shape at the
        // column's width.
        let sizes: Vec<(u32, f32)> = r
            .picture_ids
            .iter()
            .filter_map(|&id| {
                let (pw, ph) = rc.sprite_size(id)?;
                Some((id, col_w * ph / pw))
            })
            .collect();
        if !sizes.is_empty() {
            let cycle: f32 = sizes.iter().map(|(_, ph)| ph + PICTURE_GAP).sum();
            let start = h - travel % cycle.max(1.0);
            let mut y = start - cycle;
            let mut i = 0usize;
            while y < h {
                let (id, ph) = sizes[i % sizes.len()];
                if y + ph > 0.0 {
                    rc.sprite(
                        TextureRef::Surface(id),
                        x0,
                        y,
                        col_w,
                        ph,
                        (0.0, 0.0, 1.0, 1.0),
                        Color::WHITE,
                    );
                    rc.stroke_rounded_rect(
                        x0,
                        y,
                        col_w,
                        ph,
                        CornerRadii::all(4.0),
                        bronze(0.45),
                        1.0,
                    );
                }
                y += ph + PICTURE_GAP;
                i += 1;
            }
        }
        rc.pop_clip();
    }

    fn draw_text(&self, rc: &mut RenderContext<'_>, top: f32) {
        let Some(font) = &self.fonts.text else {
            return;
        };
        let (w, h) = self.size;
        let col_w = TEXT_W.min(w * 0.45);
        let x0 = (w * 0.49).max(w - col_w - 120.0);
        let cx = x0 + col_w * 0.5;
        let lh = self.line_height();
        rc.push_clip(x0, 0.0, col_w, h);
        for (i, line) in self.lines.iter().enumerate() {
            let y = top + i as f32 * lh;
            if y + lh < 0.0 || y > h || line.is_empty() {
                continue;
            }
            let lw = text::measure(font, line, 0.0);
            // The roles are in gold; the names (in capitals) in the text's
            // colour.
            let names = !line.chars().any(char::is_lowercase);
            let ink = if names {
                hex(0xF4_EBDC, 0.9)
            } else {
                gold(1.0)
            };
            text::draw(
                rc,
                font,
                line,
                (cx - lw * 0.5).round(),
                y,
                TextStyle::plain(ink),
            );
        }
        rc.pop_clip();
    }
}

/// The top and bottom of the screen fade into the night, so the pictures
/// and the text rise out of it and into it.
fn fade_ends(rc: &mut RenderContext<'_>, w: f32, h: f32) {
    let band = h * 0.2;
    let night = hex(0x0C_0907, 0.92);
    let clear = Color { a: 0.0, ..night };
    rc.fill_vertical_gradient(0.0, 0.0, w, band, CornerRadii::all(0.0), night, clear);
    rc.fill_vertical_gradient(0.0, h - band, w, band, CornerRadii::all(0.0), clear, night);
}

impl Widget for CreditsView {
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
        match e.kind {
            kind::MOUSE_MOVE | kind::HOVER_ENTER => {
                self.hover = Some((e.data1 as f32, e.data2 as f32));
                false
            }
            kind::HOVER_LEAVE => {
                self.hover = None;
                false
            }
            kind::CLICK | kind::RIGHT_CLICK | kind::SCROLL => {
                self.leave = true;
                true
            }
            _ => false,
        }
    }

    fn draw(&mut self, _ui: &mut UiRoot, _me: ElementId, rc: &mut RenderContext<'_>) {
        self.wrap();
        let (w, h) = self.size;
        scenery::draw(rc, w, h, Sky::Dusk, false);
        rc.fill(0.0, 0.0, w, h, hex(0x08_0604, 0.55));
        let progress = self.progress();
        let text_h = self.lines.len() as f32 * self.line_height();
        // The text starts part way up, so the screen is never empty, and
        // rises until it has gone past the top.
        let start = h * TEXT_START;
        let travel = (start + text_h) * progress;
        self.draw_pictures(rc, travel);
        self.draw_text(rc, start - travel);
        fade_ends(rc, w, h);
        if progress >= 1.0 {
            self.leave = true;
        }
        let f = &self.fonts;
        let p = self.theme.palette;
        stroke(rc, 32.0, 29.0, 22.0, DIAMOND, gold(1.0), 1.8);
        put_mid(rc, f.title.as_ref(), "Credits", 64.0, 22.0, 36.0, p.text);
        let back = self.back_rect();
        let lit = kit::hot(self.hover, back);
        let round = CornerRadii::all(4.0);
        rc.fill_rounded_rect(
            back.0,
            back.1,
            back.2,
            back.3,
            round,
            hex(0x14_100C, if lit { 0.8 } else { 0.6 }),
        );
        rc.stroke_rounded_rect(
            back.0,
            back.1,
            back.2,
            back.3,
            round,
            bronze(if lit { 0.8 } else { 0.5 }),
            1.0,
        );
        stroke(rc, back.0 + 12.0, back.1 + 10.0, 14.0, BACK, kit::WARM, 2.2);
        put_mid(
            rc,
            f.small.as_ref(),
            "Back to your characters",
            back.0 + 32.0,
            back.1,
            back.3,
            p.text,
        );
        if let Some(sm) = &f.small {
            let note = "Any key or click goes back. The text and pictures are the game's own, as retail shows them.";
            let nw = text::measure(sm, note, 0.0);
            text::draw(
                rc,
                sm,
                note,
                ((w - nw) * 0.5).round(),
                h - 26.0 - sm.line_height,
                TextStyle::plain(SOFT),
            );
        }
    }
}
