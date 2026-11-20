//! Invitations that do not stop play (OpenAC's own): someone asking to
//! swear allegiance to the player, or inviting the player into a
//! fellowship, shows as a small card under the target with Decline and
//! Accept (or Join). Play goes on; the cards wait for an answer (or for the
//! game to take the question back), and chat notes each one as it comes.
//! Retail asks these in the middle of the screen and stops play.

use std::sync::Arc;

use ac_ui::draw::{Color, RenderContext};
use ac_ui::event::{UiEvent, kind};
use ac_ui::geometry::AnchorEdges;
use ac_ui::modern::Horizon;
use ac_ui::modern::text::{self, TextStyle};
use ac_ui::modern::theme::hex;
use ac_ui::shapes::CornerRadii;
use ac_ui::tree::{Element, ElementId, Widget};
use ac_ui::{UiFont, UiRoot};

use super::card::draw_glyph;
use super::look::Look;
use crate::ui::modern::kit::{self, Rect, bronze, gold};

/// A card's width, and the room between cards.
pub const WIDTH: f32 = 380.0;
const GAP: f32 = 8.0;
const PAD: f32 = 14.0;
const BUTTON: f32 = 30.0;
/// The line under the cards.
const HINT: &str = "Keep playing: these wait for an answer, and say so in chat";

/// One invitation.
#[derive(Debug, Clone, PartialEq)]
pub struct Invite {
    /// Its number (the window's).
    pub context: u32,
    /// How it looks (title, icon, the buttons' words).
    pub look: Look,
    /// What it says.
    pub text: String,
}

/// The fonts the cards draw with.
#[derive(Debug, Clone, Default)]
pub struct InviteFonts {
    /// The title and the buttons.
    pub strong: Option<Arc<UiFont>>,
    /// What it says, the hint.
    pub body: Option<Arc<UiFont>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Hit {
    Yes(usize),
    No(usize),
}

/// The invitation cards (see the module documentation).
#[derive(Debug)]
pub struct InviteCards {
    theme: Horizon,
    fonts: InviteFonts,
    /// The invitations, oldest first.
    pub invites: Vec<Invite>,
    hovered: Option<Hit>,
    /// The answers given since the window last looked: (number, yes).
    pub answers: Vec<(u32, bool)>,
}

impl InviteCards {
    /// The cards, hidden.
    pub fn mount(ui: &mut UiRoot, theme: Horizon, fonts: InviteFonts) -> ElementId {
        let root = ui.root();
        let e = Element::at(0.0, 0.0, WIDTH, 10.0)
            .with_visible(false)
            .with_anchors(AnchorEdges::NONE);
        ui.add(
            root,
            e,
            Self {
                theme,
                fonts,
                invites: Vec::new(),
                hovered: None,
                answers: Vec::new(),
            },
        )
    }

    /// Shows `invites` on `me` with its top centre at (`centre`, `top`);
    /// hidden when there are none.
    pub fn show(ui: &mut UiRoot, me: ElementId, invites: Vec<Invite>, centre: f32, top: f32) {
        let Some(c) = ui.widget_mut::<Self>(me) else {
            return;
        };
        let any = !invites.is_empty();
        if c.invites != invites {
            c.invites = invites;
            c.hovered = None;
        }
        let h = c.height();
        if let Some(e) = ui.element_mut(me) {
            e.left = (centre - WIDTH * 0.5).round();
            e.top = top.round();
            e.height = h;
        }
        if ui.element(me).is_some_and(|e| e.visible) != any {
            ui.set_visible(me, any);
        }
    }

    /// The answers given on `me` since the last call.
    pub fn take(ui: &mut UiRoot, me: ElementId) -> Vec<(u32, bool)> {
        ui.widget_mut::<Self>(me)
            .map(|c| std::mem::take(&mut c.answers))
            .unwrap_or_default()
    }

    fn card_height(&self, invite: &Invite) -> f32 {
        let lines = self.fonts.body.as_ref().map_or(1, |f| {
            kit::wrap(f, &invite.text, self.words_width(invite))
                .len()
                .max(1)
        });
        let step = self
            .fonts
            .body
            .as_ref()
            .map_or(16.0, |f| f.line_height + 3.0);
        let title = self
            .fonts
            .strong
            .as_ref()
            .map_or(16.0, |f| f.line_height + 2.0);
        (title + lines as f32 * step + 2.0 * 12.0).max(BUTTON + 24.0)
    }

    fn buttons_width(&self, invite: &Invite) -> (f32, f32) {
        let w = |words: &str| {
            self.fonts
                .strong
                .as_ref()
                .map_or(56.0, |f| text::measure(f, words, 0.0))
        };
        (w(&invite.look.no) + 20.0, w(&invite.look.yes) + 24.0)
    }

    fn words_width(&self, invite: &Invite) -> f32 {
        let (no, yes) = self.buttons_width(invite);
        WIDTH - 2.0 * PAD - 30.0 - 12.0 - no - 8.0 - yes - 12.0
    }

    fn height(&self) -> f32 {
        let cards: f32 = self.invites.iter().map(|i| self.card_height(i) + GAP).sum();
        cards + 20.0
    }

    /// Each card's rectangle, its no and its yes.
    fn rects(&self) -> Vec<(Rect, Rect, Rect)> {
        let mut y = 0.0;
        let mut out = Vec::new();
        for invite in &self.invites {
            let h = self.card_height(invite);
            let (no_w, yes_w) = self.buttons_width(invite);
            let by = y + (h - BUTTON) * 0.5;
            let yes = (WIDTH - PAD - yes_w, by, yes_w, BUTTON);
            let no = (yes.0 - 8.0 - no_w, by, no_w, BUTTON);
            out.push(((0.0, y, WIDTH, h), no, yes));
            y += h + GAP;
        }
        out
    }

    fn hit(&self, x: f32, y: f32) -> Option<Hit> {
        self.rects()
            .iter()
            .enumerate()
            .find_map(|(i, (_, no, yes))| {
                if kit::inside(*yes, x, y) {
                    Some(Hit::Yes(i))
                } else if kit::inside(*no, x, y) {
                    Some(Hit::No(i))
                } else {
                    None
                }
            })
    }
}

impl Widget for InviteCards {
    fn hit_test(&self, _element: &Element, x: f32, y: f32) -> bool {
        self.rects()
            .iter()
            .any(|(card, _, _)| kit::inside(*card, x, y))
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
                self.hovered = self.hit(x, y);
                true
            }
            kind::HOVER_LEAVE => {
                self.hovered = None;
                true
            }
            kind::CLICK => {
                match self.hit(x, y) {
                    Some(Hit::Yes(i)) => {
                        if let Some(v) = self.invites.get(i) {
                            self.answers.push((v.context, true));
                        }
                    }
                    Some(Hit::No(i)) => {
                        if let Some(v) = self.invites.get(i) {
                            self.answers.push((v.context, false));
                        }
                    }
                    None => {}
                }
                true
            }
            _ => true,
        }
    }

    fn draw(&mut self, _ui: &mut UiRoot, _me: ElementId, rc: &mut RenderContext<'_>) {
        let p = self.theme.palette;
        let rects = self.rects();
        for (i, (invite, (card, no, yes))) in self.invites.iter().zip(&rects).enumerate() {
            let round = CornerRadii::all(self.theme.metrics.window_radius);
            rc.soft_shadow(card.0, card.1, card.2, card.3, 6.0, 10.0, 30.0, hex(0, 0.5));
            rc.fill_rounded_rect(card.0, card.1, card.2, card.3, round, hex(0x18_130E, 0.94));
            rc.stroke_rounded_rect(card.0, card.1, card.2, card.3, round, bronze(0.8), 1.0);
            let iy = card.1 + (card.3 - 30.0) * 0.5;
            rc.fill_ellipse(
                PAD,
                iy,
                30.0,
                30.0,
                Color {
                    a: 0.16,
                    ..invite.look.tint
                },
            );
            draw_glyph(
                rc,
                invite.look.glyph,
                PAD + 7.0,
                iy + 7.0,
                16.0,
                invite.look.tint,
            );
            let tx = PAD + 30.0 + 12.0;
            let room = self.words_width(invite);
            let mut ty = card.1 + 12.0;
            if let Some(f) = &self.fonts.strong {
                let title = kit::fit(f, &invite.look.title, room);
                text::draw(rc, f, &title, tx, ty, TextStyle::plain(p.text));
                ty += f.line_height + 2.0;
            }
            if let Some(f) = &self.fonts.body {
                let lines = kit::wrap(f, &invite.text, room);
                kit::lines(rc, f, &lines, tx, ty, f.line_height + 3.0, kit::WARM);
            }
            let radius = CornerRadii::all(4.0);
            let no_lit = self.hovered == Some(Hit::No(i));
            let yes_lit = self.hovered == Some(Hit::Yes(i));
            rc.fill_rounded_rect(
                no.0,
                no.1,
                no.2,
                no.3,
                radius,
                if no_lit {
                    hex(0xFF_FFFF, 0.06)
                } else {
                    Color::TRANSPARENT
                },
            );
            rc.stroke_rounded_rect(no.0, no.1, no.2, no.3, radius, bronze(0.45), 1.0);
            rc.fill_rounded_rect(
                yes.0,
                yes.1,
                yes.2,
                yes.3,
                radius,
                gold(if yes_lit { 1.0 } else { 0.9 }),
            );
            rc.stroke_rounded_rect(yes.0, yes.1, yes.2, yes.3, radius, gold(1.0), 1.0);
            if let Some(f) = &self.fonts.strong {
                for (r, words, ink) in [
                    (no, &invite.look.no, p.text),
                    (yes, &invite.look.yes, hex(0x1A_140D, 1.0)),
                ] {
                    text::draw_centred(
                        rc,
                        f,
                        words,
                        r.0 + r.2 * 0.5,
                        r.1 + ((r.3 - f.line_height) * 0.5).round(),
                        TextStyle::plain(ink),
                    );
                }
            }
        }
        if let (Some((last, _, _)), Some(f)) = (rects.last(), &self.fonts.body) {
            let y = last.1 + last.3 + GAP;
            text::draw_centred(
                rc,
                f,
                HINT,
                WIDTH * 0.5,
                y,
                TextStyle::plain(kit::WARM).shadowed(hex(0, 0.9)),
            );
        }
    }
}
