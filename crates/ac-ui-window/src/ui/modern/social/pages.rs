//! The Social window's allegiance page (with its Manage allegiance page)
//! and fellowship page.

use ac_ui::modern::theme::Horizon;
use ac_ui::modern::tone;
use std::sync::Arc;

use ac_ui::UiFont;
use ac_ui::draw::{Color, RenderContext};
use ac_ui::modern::text::{self, TextStyle};
use ac_ui::modern::theme::hex;
use ac_ui::shapes::CornerRadii;

use super::model::{AllegianceScreen, FellowRow, MemberCard};
use super::window::{
    BODY, FOOT, FellowSwitch, HEIGHT, Hit, Hits, ManageAsk, PAD, Slot, SocialAsk, SocialFrame,
    WIDTH,
};
use crate::ui::modern::hud::lines::stroke;
use crate::ui::modern::kit::{self, ButtonLook, Rect, bronze, gold};

/// A chevron pointing right.
const CHEVRON: &[&[(f32, f32)]] = &[&[(9.0, 6.0), (15.0, 12.0), (9.0, 18.0)]];
/// A chevron pointing left.
const BACK: &[&[(f32, f32)]] = &[&[(15.0, 6.0), (9.0, 12.0), (15.0, 18.0)]];
/// A crown, for the leader.
const CROWN: &[&[(f32, f32)]] = &[&[
    (3.0, 18.0),
    (4.0, 8.0),
    (9.0, 13.0),
    (12.0, 6.0),
    (15.0, 13.0),
    (20.0, 8.0),
    (21.0, 18.0),
    (3.0, 18.0),
]];
/// A sack, for shared loot.
const SACK: &[&[(f32, f32)]] = &[
    &[
        (9.0, 7.0),
        (5.0, 13.0),
        (5.0, 19.0),
        (19.0, 19.0),
        (19.0, 13.0),
        (15.0, 7.0),
    ],
    &[(9.0, 7.0), (8.0, 4.0), (16.0, 4.0), (15.0, 7.0), (9.0, 7.0)],
];
/// A speech bubble and a house, for the band's buttons.
const BUBBLE: &[&[(f32, f32)]] = &[&[
    (4.0, 5.0),
    (20.0, 5.0),
    (20.0, 16.0),
    (9.0, 16.0),
    (4.0, 20.0),
    (4.0, 5.0),
]];
const HOUSE: &[&[(f32, f32)]] = &[
    &[(3.0, 11.0), (12.0, 4.0), (21.0, 11.0)],
    &[(5.0, 10.0), (5.0, 20.0), (19.0, 20.0), (19.0, 10.0)],
    &[(10.0, 20.0), (10.0, 15.0), (14.0, 15.0), (14.0, 20.0)],
];

/// The vitals' colours: health, stamina, mana.
const VITAL_COLORS: [Color; 3] = [
    hex(0xC2_343C, 1.0),
    hex(0xB8_8A1E, 1.0),
    hex(0x2F_64C2, 1.0),
];

/// The office titles the game starts with.
const OFFICES: [&str; 3] = ["Speaker", "Seneschal", "Castellan"];

const CARD_W: f32 = 292.0;
const ROW: f32 = 44.0;
const SECTION_W: f32 = (WIDTH - 2.0 * PAD - 12.0) / 2.0;
const SECTION_H: f32 = 212.0;
const SECTION_TOP: [f32; 2] = [BODY + 60.0, BODY + 60.0 + SECTION_H + 10.0];

type Font<'a> = Option<&'a Arc<UiFont>>;

/// `words` in `font` at (`x`, `y`).
pub(super) fn put(
    rc: &mut RenderContext<'_>,
    font: Font<'_>,
    words: &str,
    x: f32,
    y: f32,
    c: Color,
) {
    if let Some(f) = font {
        text::draw(rc, f, words, x, y, TextStyle::plain(c));
    }
}

/// `words` in `font` centred down a band from `y`, `h` high.
pub(super) fn put_mid(
    rc: &mut RenderContext<'_>,
    font: Font<'_>,
    words: &str,
    x: f32,
    y: f32,
    h: f32,
    c: Color,
) {
    if let Some(f) = font {
        let ty = y + ((h - f.line_height) * 0.5).round();
        text::draw(rc, f, words, x, ty, TextStyle::plain(c));
    }
}

/// `words` ending at `right`, centred down a band from `y`, `h` high;
/// returns where they start.
pub(super) fn put_right(
    rc: &mut RenderContext<'_>,
    font: Font<'_>,
    words: &str,
    right: f32,
    y: f32,
    h: f32,
    c: Color,
) -> f32 {
    let Some(f) = font else {
        return right;
    };
    let w = text::measure(f, words, 0.0);
    let ty = y + ((h - f.line_height) * 0.5).round();
    text::draw(rc, f, words, right - w, ty, TextStyle::plain(c));
    right - w
}

/// Spaced capitals at (`x`, `y`).
pub(super) fn label(rc: &mut RenderContext<'_>, font: Font<'_>, words: &str, x: f32, y: f32) {
    if let Some(f) = font {
        text::draw(
            rc,
            f,
            words,
            x,
            y,
            TextStyle::plain(tone::soft(1.0)).tracked(kit::LABEL_TRACKING),
        );
    }
}

/// A button noted as `hit`.
#[allow(clippy::too_many_arguments)]
pub(super) fn button(
    rc: &mut RenderContext<'_>,
    frame: &SocialFrame,
    h: &mut Hits,
    r: Rect,
    words: &str,
    look: ButtonLook,
    enabled: bool,
    hit: Hit,
) {
    let lit = if enabled {
        h.add(r, hit)
    } else {
        h.add(r, Hit::Swallow)
    };
    kit::button(
        rc,
        &frame.theme,
        r,
        words,
        frame.fonts.strong.as_ref(),
        look,
        lit,
        enabled,
    );
}

/// A button `words` wide enough for its words, its right edge at `right`;
/// returns its left edge.
#[allow(clippy::too_many_arguments)]
pub(super) fn button_left_of(
    rc: &mut RenderContext<'_>,
    frame: &SocialFrame,
    h: &mut Hits,
    right: f32,
    y: f32,
    height: f32,
    words: &str,
    look: ButtonLook,
    enabled: bool,
    hit: Hit,
) -> f32 {
    let w = kit::button_width(words, frame.fonts.strong.as_ref());
    button(
        rc,
        frame,
        h,
        (right - w, y, w, height),
        words,
        look,
        enabled,
        hit,
    );
    right - w
}

/// A switch with its words after it; returns the right end of the words.
#[allow(clippy::too_many_arguments)]
pub(super) fn switch_row(
    rc: &mut RenderContext<'_>,
    frame: &SocialFrame,
    h: &mut Hits,
    x: f32,
    cy: f32,
    on: bool,
    words: &str,
    hit: Hit,
) -> f32 {
    let r = (x, cy - 10.0, 38.0, 20.0);
    let font = frame.fonts.body.as_ref();
    let w = font.map_or(120.0, |f| text::measure(f, words, 0.0));
    let lit = h.add((x, cy - 13.0, 48.0 + w, 26.0), hit);
    kit::switch(rc, r, on, lit);
    put_mid(rc, font, words, x + 48.0, cy - 10.0, 20.0, tone::warm(1.0));
    x + 48.0 + w
}

/// The foot band all pages but the people lists share.
pub(super) fn foot(rc: &mut RenderContext<'_>) -> f32 {
    let y = HEIGHT - FOOT;
    rc.fill_rounded_rect(
        1.0,
        y,
        WIDTH - 2.0,
        FOOT - 1.0,
        CornerRadii::new(0.0, 0.0, 5.0, 5.0),
        hex(0, 0.2),
    );
    rc.fill(0.0, y, WIDTH, 1.0, bronze(0.35));
    y
}

/// A section's box with its spaced-capitals heading.
fn section(rc: &mut RenderContext<'_>, frame: &SocialFrame, r: Rect, heading: &str) {
    let round = CornerRadii::all(4.0);
    rc.fill_rounded_rect(r.0, r.1, r.2, r.3, round, hex(0, 0.28));
    rc.stroke_rounded_rect(r.0, r.1, r.2, r.3, round, bronze(0.3), 1.0);
    label(
        rc,
        frame.fonts.label.as_ref(),
        heading,
        r.0 + 12.0,
        r.1 + 12.0,
    );
}

/// The input box under field `slot` (its hint while empty).
pub(super) fn input(rc: &mut RenderContext<'_>, frame: &SocialFrame, slot: Slot, hint: &str) {
    if let Some(r) = frame.field_rect(slot) {
        kit::field_box(
            rc,
            &frame.theme,
            r,
            hint,
            frame.is_empty(slot),
            frame.focused == Some(slot),
            frame.fonts.body.as_ref(),
        );
    }
}

/// The name typed for a new fellowship.
#[must_use]
pub fn fellowship_name_rect() -> Rect {
    (PAD + 16.0, 112.0 + 62.0, 368.0 - 32.0, 34.0)
}

/// Where Manage allegiance's field `slot` goes.
#[must_use]
pub fn manage_field_rect(slot: Slot) -> Option<Rect> {
    let x = [PAD, PAD + SECTION_W + 12.0];
    let w = SECTION_W - 24.0;
    Some(match slot {
        Slot::AllegianceName => (x[0] + 12.0, SECTION_TOP[0] + 48.0, w - 92.0, 30.0),
        Slot::Motd => (x[0] + 12.0, SECTION_TOP[0] + 104.0, w, 54.0),
        Slot::Officer => (x[1] + 12.0, SECTION_TOP[0] + 48.0, w, 30.0),
        Slot::Approved => (x[0] + 12.0, SECTION_TOP[1] + 104.0, w - 96.0, 30.0),
        Slot::Banned => (x[1] + 12.0, SECTION_TOP[1] + 48.0, w, 30.0),
        _ => return None,
    })
}

impl SocialFrame {
    /// The allegiance page.
    #[allow(clippy::too_many_lines)]
    pub(super) fn draw_allegiance(&mut self, rc: &mut RenderContext<'_>, h: &mut Hits) {
        let Some(a) = self.screen.allegiance.clone() else {
            self.draw_no_allegiance(rc, h);
            return;
        };
        let p = Horizon::current().palette;
        let f = self.fonts.clone();
        // The band: the name, how many, the buttons.
        let mut right = WIDTH - PAD;
        right = self.icon_button(
            rc,
            h,
            right,
            BODY + 19.0,
            "Hometown",
            HOUSE,
            Hit::Ask(SocialAsk::Hometown),
        );
        right = self.icon_button(
            rc,
            h,
            right - 8.0,
            BODY + 19.0,
            "Allegiance chat",
            BUBBLE,
            Hit::Ask(SocialAsk::AllegianceChat),
        );
        if let Some(hf) = &f.heading {
            let name = kit::fit(hf, &a.name, right - PAD - 16.0);
            text::draw(rc, hf, &name, PAD, BODY + 12.0, TextStyle::plain(p.text));
        }
        put(
            rc,
            f.small.as_ref(),
            &a.sub,
            PAD,
            BODY + 36.0,
            tone::soft(1.0),
        );
        // The message of the day.
        let mut top = BODY + 66.0;
        if let Some((words, by)) = &a.motd {
            let r = (PAD, top, WIDTH - 2.0 * PAD, 40.0);
            let round = CornerRadii::all(4.0);
            rc.fill_rounded_rect(r.0, r.1, r.2, r.3, round, bronze(0.08));
            rc.stroke_rounded_rect(r.0, r.1, r.2, r.3, round, bronze(0.25), 1.0);
            label(rc, f.label.as_ref(), "MESSAGE", r.0 + 12.0, r.1 + 15.0);
            let by_x = put_right(
                rc,
                f.small.as_ref(),
                by,
                r.0 + r.2 - 12.0,
                r.1,
                r.3,
                tone::muted(1.0),
            );
            if let Some(b) = &f.body {
                let w = by_x - (r.0 + 86.0) - 12.0;
                let line = kit::fit(b, words, w);
                put_mid(rc, Some(b), &line, r.0 + 86.0, r.1, r.3, p.text);
            }
            top += 54.0;
        }
        self.draw_chain(rc, h, &a, top);
        let lists = top + 68.0;
        let bottom = HEIGHT - FOOT - 12.0;
        self.draw_vassals(rc, h, &a, lists, bottom);
        let picked = self.picked_member(&a);
        if let Some(card) = a.card(picked) {
            self.draw_member_card(
                rc,
                h,
                card,
                (WIDTH - PAD - CARD_W, lists, CARD_W, bottom - lists),
            );
        }
        // The foot.
        let y = foot(rc);
        let cy = y + FOOT * 0.5;
        let on = self.screen.ignore_allegiance;
        switch_row(
            rc,
            self,
            h,
            PAD,
            cy,
            on,
            "Ignore allegiance requests",
            Hit::Ask(SocialAsk::IgnoreAllegiance(!on)),
        );
        let mut right = WIDTH - PAD;
        if a.monarch {
            let tag_w = 58.0;
            let words = "Manage allegiance";
            let w = kit::button_width(words, f.strong.as_ref()) + tag_w;
            let r = (right - w, cy - 17.0, w, 34.0);
            let lit = h.add(r, Hit::Manage(true));
            kit::button(
                rc,
                &Horizon::current(),
                (r.0, r.1, r.2, r.3),
                "",
                None,
                ButtonLook::Plain,
                lit,
                true,
            );
            put_mid(rc, f.strong.as_ref(), words, r.0 + 14.0, r.1, r.3, p.text);
            kit::openac_tag(rc, f.label.as_ref(), r.0 + r.2 - tag_w, cy);
            right = r.0 - 8.0;
        }
        let has_patron = a.chain.iter().any(|c| !c.role.starts_with("YOU"));
        if !has_patron
            && let Some((id, name)) = self.screen.selected.clone()
            && a.card(id).is_none()
        {
            button_left_of(
                rc,
                self,
                h,
                right,
                cy - 17.0,
                34.0,
                &format!("Swear allegiance to {name}"),
                ButtonLook::Plain,
                true,
                Hit::Ask(SocialAsk::Swear(id, name)),
            );
        }
    }

    /// A plain button with a glyph before its words, its right edge at
    /// `right`; returns its left edge.
    #[allow(clippy::too_many_arguments)]
    fn icon_button(
        &self,
        rc: &mut RenderContext<'_>,
        h: &mut Hits,
        right: f32,
        y: f32,
        words: &str,
        glyph: &[&[(f32, f32)]],
        hit: Hit,
    ) -> f32 {
        let f = self.fonts.strong.as_ref();
        let w = kit::button_width(words, f) + 20.0;
        let r = (right - w, y, w, 30.0);
        let lit = h.add(r, hit);
        kit::button(
            rc,
            &Horizon::current(),
            r,
            "",
            None,
            ButtonLook::Plain,
            lit,
            true,
        );
        stroke(rc, r.0 + 12.0, r.1 + 8.0, 14.0, glyph, tone::warm(1.0), 1.6);
        put_mid(
            rc,
            f,
            words,
            r.0 + 32.0,
            r.1,
            r.3,
            Horizon::current().palette.text,
        );
        r.0
    }

    /// The person picked, or the first vassal, or the player.
    fn picked_member(&self, a: &AllegianceScreen) -> u32 {
        if a.card(self.member).is_some() {
            return self.member;
        }
        a.vassals
            .first()
            .map(|v| v.id)
            .or_else(|| a.chain.last().map(|c| c.id))
            .unwrap_or(0)
    }

    fn draw_chain(&self, rc: &mut RenderContext<'_>, h: &mut Hits, a: &AllegianceScreen, top: f32) {
        let p = Horizon::current().palette;
        let f = &self.fonts;
        let n = a.chain.len().max(1) as f32;
        let gap = 22.0;
        let w = (WIDTH - 2.0 * PAD - gap * (n - 1.0)) / n;
        let picked = self.picked_member(a);
        for (i, c) in a.chain.iter().enumerate() {
            let x = PAD + i as f32 * (w + gap);
            let r = (x, top, w, 56.0);
            let lit = h.add(r, Hit::Member(c.id));
            kit::pick_row(rc, r, c.id == picked, lit);
            label(rc, f.label.as_ref(), c.role, x + 12.0, top + 11.0);
            kit::online_dot(rc, x + 12.0, top + 32.0, 7.0, c.online);
            let side_x = if c.side.is_empty() {
                x + w - 12.0
            } else {
                put_right(
                    rc,
                    f.small.as_ref(),
                    &c.side,
                    x + w - 12.0,
                    top + 24.0,
                    22.0,
                    tone::soft(1.0),
                ) - 8.0
            };
            if let Some(st) = &f.strong {
                let name = kit::fit(st, &c.full, side_x - (x + 26.0));
                put_mid(rc, Some(st), &name, x + 26.0, top + 24.0, 22.0, p.text);
            }
            if i + 1 < a.chain.len() {
                stroke(rc, x + w + 3.0, top + 20.0, 16.0, CHEVRON, bronze(0.7), 1.6);
            }
        }
    }

    fn draw_vassals(
        &mut self,
        rc: &mut RenderContext<'_>,
        h: &mut Hits,
        a: &AllegianceScreen,
        top: f32,
        bottom: f32,
    ) {
        let p = Horizon::current().palette;
        let f = self.fonts.clone();
        let w = WIDTH - 2.0 * PAD - CARD_W - 16.0;
        let count = format!("· {}", a.vassals.len());
        kit::group_heading(
            rc,
            &Horizon::current(),
            f.label.as_ref(),
            PAD,
            top,
            24.0,
            PAD + w - 80.0,
            "SWORN TO YOU",
            Some(&count),
            tone::soft(1.0),
        );
        if let Some(l) = &f.label {
            let words = "PASSED UP";
            let lw = text::measure(l, words, kit::LABEL_TRACKING);
            text::draw(
                rc,
                l,
                words,
                PAD + w - lw - 4.0,
                top + ((24.0 - l.line_height) * 0.5).round(),
                TextStyle::plain(tone::soft(1.0)).tracked(kit::LABEL_TRACKING),
            );
        }
        let room = (PAD, top + 28.0, w, bottom - top - 28.0);
        if a.vassals.is_empty() {
            put(
                rc,
                f.body.as_ref(),
                "Nobody is sworn to you yet.",
                PAD,
                room.1 + 8.0,
                tone::muted(1.0),
            );
        }
        let total = a.vassals.len() as f32 * (ROW + 2.0);
        self.totals[0] = (total, room);
        self.scroll[0].hold(total, room.3);
        let picked = self.picked_member(a);
        rc.push_clip(room.0, room.1, room.2, room.3);
        for (i, v) in a.vassals.iter().enumerate() {
            let y = room.1 + i as f32 * (ROW + 2.0) - self.scroll[0].at;
            if y + ROW < room.1 || y > room.1 + room.3 {
                continue;
            }
            let r = (room.0, y, room.2 - 8.0, ROW);
            let lit = h.add(r, Hit::Member(v.id)) && kit::hot(h.hover, room);
            kit::pick_row(rc, r, v.id == picked, lit);
            kit::online_dot(rc, r.0 + 10.0, y + 18.0, 8.0, v.online);
            let passed_x = put_right(
                rc,
                f.strong.as_ref(),
                &v.passed,
                r.0 + r.2 - 10.0,
                y,
                ROW,
                p.text,
            );
            let room_w = passed_x - (r.0 + 28.0) - 10.0;
            if let Some(st) = &f.strong {
                let name = kit::fit(st, &v.full, room_w);
                let ink = if v.online { p.text } else { tone::soft(1.0) };
                put(rc, Some(st), &name, r.0 + 28.0, y + 6.0, ink);
            }
            if let Some(sm) = &f.small {
                let sub = kit::fit(sm, &v.sub, room_w);
                put(rc, Some(sm), &sub, r.0 + 28.0, y + 25.0, tone::muted(1.0));
            }
        }
        rc.pop_clip();
        kit::scrollbar(
            rc,
            room.0 + room.2 - 5.0,
            room.1,
            room.3,
            total,
            self.scroll[0].at,
        );
    }

    #[allow(clippy::too_many_lines)]
    fn draw_member_card(&self, rc: &mut RenderContext<'_>, h: &mut Hits, c: &MemberCard, r: Rect) {
        let p = Horizon::current().palette;
        let f = &self.fonts;
        let round = CornerRadii::all(4.0);
        rc.fill_rounded_rect(r.0, r.1, r.2, r.3, round, hex(0, 0.28));
        rc.stroke_rounded_rect(r.0, r.1, r.2, r.3, round, bronze(0.3), 1.0);
        h.add(r, Hit::Swallow);
        let status_color = if c.status == "You" || c.online {
            kit::GOOD
        } else {
            tone::muted(1.0)
        };
        let sx = put_right(
            rc,
            f.small.as_ref(),
            c.status,
            r.0 + r.2 - 14.0,
            r.1 + 12.0,
            20.0,
            status_color,
        );
        if let Some(nf) = &f.name {
            let name = kit::fit(nf, &c.full, sx - (r.0 + 14.0) - 8.0);
            put_mid(rc, Some(nf), &name, r.0 + 14.0, r.1 + 12.0, 20.0, p.text);
        }
        let mut y = r.1 + 42.0;
        for (k, v) in &c.rows {
            put_mid(rc, f.body.as_ref(), k, r.0 + 14.0, y, 22.0, tone::warm(1.0));
            put_right(rc, f.strong.as_ref(), v, r.0 + r.2 - 14.0, y, 22.0, p.text);
            y += 22.0;
        }
        let mut bottom = r.1 + r.3 - 12.0;
        if let Some(part) = &c.parting {
            let b = (r.0 + 14.0, bottom - 30.0, r.2 - 28.0, 30.0);
            button(
                rc,
                self,
                h,
                b,
                &part.button,
                ButtonLook::Danger,
                true,
                Hit::Ask(SocialAsk::Part(c.id)),
            );
            bottom = b.1 - 6.0;
        }
        if c.other {
            let w = (r.2 - 28.0 - 12.0) / 3.0;
            let y = bottom - 30.0;
            let x = r.0 + 14.0;
            button(
                rc,
                self,
                h,
                (x, y, w, 30.0),
                "Tell",
                ButtonLook::Plain,
                c.online,
                Hit::Ask(SocialAsk::Tell(c.name.clone())),
            );
            button(
                rc,
                self,
                h,
                (x + w + 6.0, y, w, 30.0),
                "Invite",
                ButtonLook::Plain,
                c.online,
                Hit::Ask(SocialAsk::Invite(c.id)),
            );
            button(
                rc,
                self,
                h,
                (x + 2.0 * (w + 6.0), y, w, 30.0),
                if c.friend { "Friend" } else { "Add friend" },
                ButtonLook::Plain,
                !c.friend,
                Hit::Ask(SocialAsk::Befriend(c.name.clone())),
            );
        }
    }

    fn draw_no_allegiance(&self, rc: &mut RenderContext<'_>, h: &mut Hits) {
        let p = Horizon::current().palette;
        let f = &self.fonts;
        let cx = WIDTH * 0.5;
        let mut y = BODY + (HEIGHT - FOOT - BODY) * 0.5 - 90.0;
        stroke(rc, cx - 16.0, y, 32.0, CROWN, gold(0.8), 1.8);
        y += 44.0;
        if let Some(hf) = &f.heading {
            text::draw_centred(
                rc,
                hf,
                "You have no allegiance yet",
                cx,
                y,
                TextStyle::plain(p.text),
            );
            y += hf.line_height + 10.0;
        }
        if let Some(b) = &f.body {
            let words = "Swear to a patron and part of the experience you earn passes up to them. Anyone who swears to you becomes your vassal.";
            for line in kit::wrap(b, words, 420.0) {
                text::draw_centred(rc, b, &line, cx, y, TextStyle::plain(tone::soft(1.0)));
                y += b.line_height + 4.0;
            }
        }
        y += 12.0;
        let (words, enabled, hit) = match &self.screen.selected {
            Some((id, name)) => (
                format!("Swear allegiance to {name}"),
                true,
                Hit::Ask(SocialAsk::Swear(*id, name.clone())),
            ),
            None => ("Select someone to swear to".to_owned(), false, Hit::Swallow),
        };
        let w = kit::button_width(&words, f.strong.as_ref()) + 8.0;
        button(
            rc,
            self,
            h,
            (cx - w * 0.5, y, w, 38.0),
            &words,
            ButtonLook::Primary,
            enabled,
            hit,
        );
        y += 50.0;
        if let Some(s) = &f.small {
            text::draw_centred(
                rc,
                s,
                "Uses the player you have selected. They are asked to accept.",
                cx,
                y,
                TextStyle::plain(tone::muted(1.0)),
            );
        }
        let fy = foot(rc);
        let on = self.screen.ignore_allegiance;
        switch_row(
            rc,
            self,
            h,
            PAD,
            fy + FOOT * 0.5,
            on,
            "Ignore allegiance requests",
            Hit::Ask(SocialAsk::IgnoreAllegiance(!on)),
        );
    }

    /// Manage allegiance (OPENAC), over the allegiance page.
    #[allow(clippy::too_many_lines)]
    pub(super) fn draw_manage(&self, rc: &mut RenderContext<'_>, h: &mut Hits) {
        let p = Horizon::current().palette;
        let f = &self.fonts;
        let name = self
            .screen
            .allegiance
            .as_ref()
            .map(|a| a.name.clone())
            .unwrap_or_default();
        let locked = self.screen.allegiance.as_ref().is_some_and(|a| a.locked);
        // The bar.
        let bar = (0.0, BODY, WIDTH, 48.0);
        rc.fill(1.0, bar.1 + bar.3 - 1.0, WIDTH - 2.0, 1.0, bronze(0.2));
        let back = (12.0, BODY + 9.0, 110.0, 30.0);
        let lit = h.add(back, Hit::Manage(false));
        if lit {
            rc.fill_rounded_rect(
                back.0,
                back.1,
                back.2,
                back.3,
                CornerRadii::all(4.0),
                hex(0xFF_FFFF, 0.05),
            );
        }
        stroke(
            rc,
            back.0 + 4.0,
            back.1 + 7.0,
            16.0,
            BACK,
            tone::warm(1.0),
            1.8,
        );
        put_mid(
            rc,
            f.strong.as_ref(),
            "Allegiance",
            back.0 + 22.0,
            back.1,
            back.3,
            tone::warm(1.0),
        );
        let mut x = back.0 + back.2 + 8.0;
        if let Some(nf) = &f.name {
            let words = format!("Manage {name}");
            let words = kit::fit(nf, &words, 300.0);
            put_mid(rc, Some(nf), &words, x, bar.1, bar.3, p.text);
            x += text::measure(nf, &words, 0.0) + 10.0;
        }
        kit::openac_tag(rc, f.label.as_ref(), x, bar.1 + bar.3 * 0.5);
        put_right(
            rc,
            f.small.as_ref(),
            "Only the monarch sees this. Officers use @ commands.",
            WIDTH - PAD,
            bar.1,
            bar.3,
            tone::muted(1.0),
        );
        let xs = [PAD, PAD + SECTION_W + 12.0];
        let w = SECTION_W;
        let manage = |a: ManageAsk| Hit::Ask(SocialAsk::Manage(a));
        // Name and message.
        let (sx, sy) = (xs[0], SECTION_TOP[0]);
        section(rc, self, (sx, sy, w, SECTION_H), "NAME AND MESSAGE");
        put(
            rc,
            f.small.as_ref(),
            "Allegiance name",
            sx + 12.0,
            sy + 32.0,
            tone::soft(1.0),
        );
        input(rc, self, Slot::AllegianceName, "A new name");
        button(
            rc,
            self,
            h,
            (sx + w - 12.0 - 86.0, sy + 48.0, 86.0, 30.0),
            "Rename",
            ButtonLook::Plain,
            !self.is_empty(Slot::AllegianceName),
            manage(ManageAsk::Rename),
        );
        put(
            rc,
            f.small.as_ref(),
            "Message of the day",
            sx + 12.0,
            sy + 88.0,
            tone::soft(1.0),
        );
        input(rc, self, Slot::Motd, "What every member sees here");
        let by = sy + SECTION_H - 12.0 - 28.0;
        let left = button_left_of(
            rc,
            self,
            h,
            sx + w - 12.0,
            by,
            28.0,
            "Post message",
            ButtonLook::Primary,
            !self.is_empty(Slot::Motd),
            manage(ManageAsk::PostMotd),
        );
        button_left_of(
            rc,
            self,
            h,
            left - 6.0,
            by,
            28.0,
            "Clear",
            ButtonLook::Plain,
            true,
            manage(ManageAsk::ClearMotd),
        );
        // Officers.
        let (sx, sy) = (xs[1], SECTION_TOP[0]);
        section(rc, self, (sx, sy, w, SECTION_H), "OFFICERS");
        self.link(
            rc,
            h,
            "List them in chat",
            sx + w - 12.0,
            sy + 8.0,
            manage(ManageAsk::ListOfficers),
        );
        put(
            rc,
            f.small.as_ref(),
            "Member, or an office's new title",
            sx + 12.0,
            sy + 32.0,
            tone::soft(1.0),
        );
        input(rc, self, Slot::Officer, "Name");
        let ow = (w - 24.0 - 8.0) / 3.0;
        for (i, title) in OFFICES.iter().enumerate() {
            let level = i as u32 + 1;
            let r = (sx + 12.0 + i as f32 * (ow + 4.0), sy + 86.0, ow, 40.0);
            let lit = h.add(r, Hit::Office(level));
            let on = self.office == level;
            let round = CornerRadii::all(4.0);
            rc.fill_rounded_rect(
                r.0,
                r.1,
                r.2,
                r.3,
                round,
                if on { gold(0.14) } else { hex(0, 0.25) },
            );
            rc.stroke_rounded_rect(
                r.0,
                r.1,
                r.2,
                r.3,
                round,
                if on {
                    gold(1.0)
                } else if lit {
                    bronze(0.6)
                } else {
                    bronze(0.3)
                },
                1.0,
            );
            if let Some(st) = &f.strong {
                text::draw_centred(
                    rc,
                    st,
                    title,
                    r.0 + r.2 * 0.5,
                    r.1 + 5.0,
                    TextStyle::plain(p.text),
                );
            }
            if let Some(sm) = &f.small {
                text::draw_centred(
                    rc,
                    sm,
                    &format!("Office {level}"),
                    r.0 + r.2 * 0.5,
                    r.1 + 22.0,
                    TextStyle::plain(tone::muted(1.0)),
                );
            }
        }
        let by = sy + 134.0;
        let left = button_left_of(
            rc,
            self,
            h,
            sx + w - 12.0,
            by,
            28.0,
            "Appoint",
            ButtonLook::Primary,
            !self.is_empty(Slot::Officer),
            manage(ManageAsk::Appoint(self.office)),
        );
        let left = button_left_of(
            rc,
            self,
            h,
            left - 6.0,
            by,
            28.0,
            "Remove office",
            ButtonLook::Plain,
            !self.is_empty(Slot::Officer),
            manage(ManageAsk::RemoveOffice),
        );
        button_left_of(
            rc,
            self,
            h,
            left - 6.0,
            by,
            28.0,
            "Rename office",
            ButtonLook::Plain,
            !self.is_empty(Slot::Officer),
            manage(ManageAsk::TitleOffice(self.office)),
        );
        if let Some(sm) = &f.small {
            let note = "Rename office gives the office picked the words typed above as its title.";
            kit::lines(
                rc,
                sm,
                &kit::wrap(sm, note, w - 24.0),
                sx + 12.0,
                sy + 172.0,
                15.0,
                tone::muted(1.0),
            );
        }
        // Joining.
        let (sx, sy) = (xs[0], SECTION_TOP[1]);
        section(rc, self, (sx, sy, w, SECTION_H), "JOINING");
        let r = (sx + 12.0, sy + 36.0, 38.0, 20.0);
        let lit = h.add(r, manage(ManageAsk::Lock(!locked)));
        kit::switch(rc, r, locked, lit);
        put(
            rc,
            f.strong.as_ref(),
            "Locked",
            sx + 60.0,
            sy + 32.0,
            p.text,
        );
        if let Some(sm) = &f.small {
            let note = "Nobody new may swear in, except the one person approved below.";
            kit::lines(
                rc,
                sm,
                &kit::wrap(sm, note, w - 72.0),
                sx + 60.0,
                sy + 50.0,
                15.0,
                tone::muted(1.0),
            );
        }
        put(
            rc,
            f.small.as_ref(),
            "Approved to swear in",
            sx + 12.0,
            sy + 88.0,
            tone::soft(1.0),
        );
        input(rc, self, Slot::Approved, "Name");
        button(
            rc,
            self,
            h,
            (sx + w - 12.0 - 90.0, sy + 104.0, 90.0, 30.0),
            "Approve",
            ButtonLook::Plain,
            !self.is_empty(Slot::Approved),
            manage(ManageAsk::Approve),
        );
        // Bans and allegiance chat.
        let (sx, sy) = (xs[1], SECTION_TOP[1]);
        section(rc, self, (sx, sy, w, SECTION_H), "BANS AND ALLEGIANCE CHAT");
        self.link(
            rc,
            h,
            "List bans in chat",
            sx + w - 12.0,
            sy + 8.0,
            manage(ManageAsk::ListBans),
        );
        put(
            rc,
            f.small.as_ref(),
            "Character",
            sx + 12.0,
            sy + 32.0,
            tone::soft(1.0),
        );
        input(rc, self, Slot::Banned, "Name");
        let named = !self.is_empty(Slot::Banned);
        let bw = (w - 24.0 - 6.0) / 2.0;
        let grid = [
            ("Ban from allegiance", ManageAsk::Ban),
            ("Lift ban", ManageAsk::LiftBan),
            ("Gag in chat", ManageAsk::Gag),
            ("Remove from chat", ManageAsk::PutOutOfChat),
        ];
        for (i, (words, ask)) in grid.into_iter().enumerate() {
            let col = (i % 2) as f32;
            let row = (i / 2) as f32;
            let r = (
                sx + 12.0 + col * (bw + 6.0),
                sy + 88.0 + row * 34.0,
                bw,
                28.0,
            );
            button(rc, self, h, r, words, ButtonLook::Plain, named, manage(ask));
        }
        if let Some(sm) = &f.small {
            let note = "The server answers these lists in chat, so they are not kept here.";
            kit::lines(
                rc,
                sm,
                &kit::wrap(sm, note, w - 24.0),
                sx + 12.0,
                sy + 162.0,
                15.0,
                tone::muted(1.0),
            );
        }
    }

    /// A gold link ending at `right`.
    fn link(
        &self,
        rc: &mut RenderContext<'_>,
        h: &mut Hits,
        words: &str,
        right: f32,
        y: f32,
        hit: Hit,
    ) {
        let Some(f) = &self.fonts.strong else {
            return;
        };
        let w = text::measure(f, words, 0.0);
        let r = (right - w - 6.0, y, w + 12.0, 22.0);
        let lit = h.add(r, hit);
        put_mid(
            rc,
            Some(f),
            words,
            r.0 + 6.0,
            r.1,
            r.3,
            gold(if lit { 1.0 } else { 0.85 }),
        );
    }

    /// The fellowship page.
    #[allow(clippy::too_many_lines)]
    pub(super) fn draw_fellowship(&mut self, rc: &mut RenderContext<'_>, h: &mut Hits) {
        let Some(fel) = self.screen.fellowship.clone() else {
            self.draw_no_fellowship(rc, h);
            return;
        };
        let p = Horizon::current().palette;
        let f = self.fonts.clone();
        // The band.
        let mut right = WIDTH - PAD;
        if fel.locked {
            right = chip(rc, &f, right, BODY + 22.0, "Locked", gold(1.0), gold(0.6)) - 8.0;
        }
        let (words, ink, edge) = if fel.open {
            ("Open", kit::GOOD, hex(0x7F_D08A, 0.5))
        } else {
            ("Closed", tone::soft(1.0), bronze(0.4))
        };
        right = chip(rc, &f, right, BODY + 22.0, words, ink, edge) - 12.0;
        if let Some(hf) = &f.heading {
            let name = kit::fit(hf, &fel.name, right - PAD);
            text::draw(rc, hf, &name, PAD, BODY + 12.0, TextStyle::plain(p.text));
        }
        put(
            rc,
            f.small.as_ref(),
            &fel.sub,
            PAD,
            BODY + 36.0,
            tone::soft(1.0),
        );
        // The roster.
        let top = BODY + 68.0;
        let cols = Columns::new();
        let header = [
            ("FELLOW", PAD + 10.0, false),
            ("LEVEL", cols.level + 56.0, true),
            ("SHARE", cols.share + 56.0, true),
            ("HEALTH", cols.vitals[0], false),
            ("STAMINA", cols.vitals[1], false),
            ("MANA", cols.vitals[2], false),
        ];
        if let Some(l) = &f.label {
            for (words, x, right_aligned) in header {
                let w = text::measure(l, words, kit::LABEL_TRACKING);
                let x = if right_aligned { x - w } else { x };
                text::draw(
                    rc,
                    l,
                    words,
                    x,
                    top + ((24.0 - l.line_height) * 0.5).round(),
                    TextStyle::plain(tone::soft(1.0)).tracked(kit::LABEL_TRACKING),
                );
            }
        }
        let settings_top = HEIGHT - FOOT - 12.0 - 70.0;
        let room = (
            PAD,
            top + 26.0,
            WIDTH - 2.0 * PAD,
            settings_top - 8.0 - (top + 26.0),
        );
        let total = fel.rows.len() as f32 * (ROW + 2.0) + fel.departed.len() as f32 * 30.0;
        self.totals[1] = (total, room);
        self.scroll[1].hold(total, room.3);
        rc.push_clip(room.0, room.1, room.2, room.3);
        let mut y = room.1 - self.scroll[1].at;
        for row in &fel.rows {
            let r = (room.0, y, room.2, ROW);
            let lit = h.add(r, Hit::Fellow(row.id)) && kit::hot(h.hover, room);
            kit::pick_row(rc, r, self.fellow == row.id, lit);
            self.draw_fellow(rc, &cols, row, y);
            y += ROW + 2.0;
        }
        for (name, words) in &fel.departed {
            kit::online_dot(rc, room.0 + 10.0, y + 11.0, 8.0, false);
            let mut x = room.0 + 28.0;
            if let Some(st) = &f.strong {
                put_mid(rc, Some(st), name, x, y, 30.0, tone::warm(1.0));
                x += text::measure(st, name, 0.0) + 5.0;
            }
            put_mid(rc, f.body.as_ref(), words, x, y, 30.0, tone::soft(1.0));
            y += 30.0;
        }
        rc.pop_clip();
        kit::scrollbar(
            rc,
            room.0 + room.2 - 5.0,
            room.1,
            room.3,
            total,
            self.scroll[1].at,
        );
        self.draw_fellow_switches(rc, h, settings_top, true);
        // The foot.
        let fy = foot(rc);
        let by = fy + (FOOT - 34.0) * 0.5;
        let members: Vec<u32> = fel.rows.iter().map(|r| r.id).collect();
        let me = fel.rows.iter().find(|r| r.you).map(|r| r.id);
        let target = self
            .screen
            .selected
            .clone()
            .filter(|(id, _)| !members.contains(id));
        let recruit_ok = target.is_some() && !fel.full && (fel.lead || fel.open);
        let (words, hit) = match &target {
            Some((id, name)) => (format!("Recruit {name}"), Hit::Ask(SocialAsk::Recruit(*id))),
            None => ("Recruit".to_owned(), Hit::Swallow),
        };
        let w = kit::button_width(&words, f.strong.as_ref());
        button(
            rc,
            self,
            h,
            (PAD, by, w, 34.0),
            &words,
            ButtonLook::Plain,
            recruit_ok,
            hit,
        );
        let mut x = PAD + w + 8.0;
        let picked = fel
            .rows
            .iter()
            .find(|r| r.id == self.fellow)
            .filter(|r| Some(r.id) != me);
        if fel.lead {
            let name = picked.map_or("someone", |r| r.name.as_str());
            let id = picked.map_or(0, |r| r.id);
            for (words, ask) in [
                (format!("Make {name} leader"), SocialAsk::MakeLeader(id)),
                (format!("Dismiss {name}"), SocialAsk::Dismiss(id)),
            ] {
                let words = self.fit_button(&words, 170.0);
                let w = kit::button_width(&words, f.strong.as_ref());
                button(
                    rc,
                    self,
                    h,
                    (x, by, w, 34.0),
                    &words,
                    ButtonLook::Plain,
                    picked.is_some(),
                    Hit::Ask(ask),
                );
                x += w + 8.0;
            }
            let words = if fel.open {
                "Close recruiting"
            } else {
                "Open recruiting"
            };
            let w = kit::button_width(words, f.strong.as_ref());
            button(
                rc,
                self,
                h,
                (x, by, w, 34.0),
                words,
                ButtonLook::Plain,
                true,
                Hit::Ask(SocialAsk::SetOpen(!fel.open)),
            );
            x += w + 8.0;
        }
        let mut right = WIDTH - PAD;
        if fel.lead {
            right = button_left_of(
                rc,
                self,
                h,
                right,
                by,
                34.0,
                "Disband",
                ButtonLook::Danger,
                true,
                Hit::Ask(SocialAsk::Disband),
            ) - 8.0;
        }
        right = button_left_of(
            rc,
            self,
            h,
            right,
            by,
            34.0,
            "Leave",
            ButtonLook::Plain,
            true,
            Hit::Ask(SocialAsk::Leave),
        ) - 12.0;
        if !fel.lead
            && !fel.leader.is_empty()
            && let Some(s) = &f.small
        {
            let hint = format!(
                "Only {} can dismiss, hand on the lead or close it.",
                fel.leader
            );
            let hint = kit::fit(s, &hint, right - x);
            put_right(rc, Some(s), &hint, right, by, 34.0, tone::muted(1.0));
        }
    }

    /// `words` cut short to fit a button `most` wide.
    fn fit_button(&self, words: &str, most: f32) -> String {
        self.fonts
            .strong
            .as_ref()
            .map_or_else(|| words.to_owned(), |f| kit::fit(f, words, most - 28.0))
    }

    fn draw_fellow(&self, rc: &mut RenderContext<'_>, cols: &Columns, row: &FellowRow, y: f32) {
        let p = Horizon::current().palette;
        let f = &self.fonts;
        let mut x = PAD + 10.0;
        if row.leader {
            stroke(rc, x, y + 14.0, 15.0, CROWN, gold(1.0), 1.5);
            x += 20.0;
        }
        let you_w = if row.you { 36.0 } else { 0.0 };
        if let Some(st) = &f.strong {
            let name = kit::fit(st, &row.name, cols.level - 12.0 - x - you_w);
            put_mid(rc, Some(st), &name, x, y, ROW, p.text);
            x += text::measure(st, &name, 0.0) + 7.0;
        }
        if row.you
            && let Some(l) = &f.label
        {
            let r = (x, y + (ROW - 16.0) * 0.5, 30.0, 16.0);
            rc.fill_rounded_rect(r.0, r.1, r.2, r.3, CornerRadii::all(3.0), bronze(0.18));
            text::draw_centred(
                rc,
                l,
                "YOU",
                r.0 + r.2 * 0.5,
                r.1 + ((16.0 - l.line_height) * 0.5).round(),
                TextStyle::plain(tone::warm(1.0)),
            );
        }
        put_right(
            rc,
            f.strong.as_ref(),
            &row.level.to_string(),
            cols.level + 56.0,
            y,
            ROW,
            p.text,
        );
        put_right(
            rc,
            f.strong.as_ref(),
            &row.share,
            cols.share + 56.0,
            y,
            ROW,
            tone::warm(1.0),
        );
        for (i, (now, most)) in row.vitals.iter().enumerate() {
            let share = if *most > 0 {
                *now as f32 / *most as f32
            } else {
                0.0
            };
            let x = cols.vitals[i];
            kit::meter(rc, (x, y + 13.0, 110.0, 6.0), share, VITAL_COLORS[i]);
            let ink = if share < 0.4 {
                kit::HARM
            } else {
                tone::soft(1.0)
            };
            let words = format!(
                "{} / {}",
                ac_ui::modern::numbers::group_thousands(i64::from(*now)),
                ac_ui::modern::numbers::group_thousands(i64::from(*most))
            );
            put(rc, f.small.as_ref(), &words, x, y + 23.0, ink);
        }
        let loot = if row.loot {
            gold(1.0)
        } else {
            tone::muted(0.35)
        };
        stroke(rc, cols.loot + 2.0, y + 12.0, 20.0, SACK, loot, 1.5);
    }

    /// The four switches under YOUR SETTINGS, from `top`.
    fn draw_fellow_switches(
        &self,
        rc: &mut RenderContext<'_>,
        h: &mut Hits,
        top: f32,
        _member: bool,
    ) {
        let s = &self.screen;
        kit::group_heading(
            rc,
            &Horizon::current(),
            self.fonts.label.as_ref(),
            PAD,
            top,
            18.0,
            WIDTH - PAD,
            "YOUR SETTINGS",
            None,
            tone::soft(1.0),
        );
        let rows = [
            ("Share my loot", FellowSwitch::ShareLoot, s.share_loot),
            (
                "Ignore fellowship requests",
                FellowSwitch::IgnoreRequests,
                s.ignore_fellowship,
            ),
            (
                "Accept requests automatically",
                FellowSwitch::AutoAccept,
                s.auto_accept,
            ),
            (
                "Show fellows on screen",
                FellowSwitch::OnScreen,
                self.on_screen,
            ),
        ];
        let col_w = (WIDTH - 2.0 * PAD - 24.0) / 2.0;
        for (i, (words, which, on)) in rows.into_iter().enumerate() {
            let x = PAD + (i % 2) as f32 * (col_w + 24.0);
            let cy = top + 22.0 + 13.0 + (i / 2) as f32 * 32.0;
            let end = switch_row(
                rc,
                self,
                h,
                x,
                cy,
                on,
                words,
                Hit::Ask(SocialAsk::Switch(which, !on)),
            );
            if which == FellowSwitch::OnScreen {
                kit::openac_tag(rc, self.fonts.label.as_ref(), end + 10.0, cy);
            }
        }
    }

    #[allow(clippy::too_many_lines)]
    fn draw_no_fellowship(&mut self, rc: &mut RenderContext<'_>, h: &mut Hits) {
        let p = Horizon::current().palette;
        let f = self.fonts.clone();
        let top = 112.0;
        let w = 368.0;
        let round = CornerRadii::all(4.0);
        // Start one.
        let r = (PAD, top, w, 210.0);
        rc.fill_rounded_rect(r.0, r.1, r.2, r.3, round, hex(0, 0.28));
        rc.stroke_rounded_rect(r.0, r.1, r.2, r.3, round, bronze(0.45), 1.0);
        put(
            rc,
            f.name.as_ref(),
            "Start a fellowship",
            r.0 + 16.0,
            r.1 + 16.0,
            p.text,
        );
        put(
            rc,
            f.small.as_ref(),
            "Name",
            r.0 + 16.0,
            r.1 + 44.0,
            tone::soft(1.0),
        );
        input(rc, self, Slot::FellowshipName, "Your fellowship's name");
        let cy = r.1 + 124.0;
        let on = self.screen.share_xp;
        let sw = (r.0 + 16.0, cy - 10.0, 38.0, 20.0);
        let lit = h.add(sw, Hit::Ask(SocialAsk::Switch(FellowSwitch::ShareXp, !on)));
        kit::switch(rc, sw, on, lit);
        put(
            rc,
            f.strong.as_ref(),
            "Share experience",
            r.0 + 66.0,
            cy - 15.0,
            p.text,
        );
        put(
            rc,
            f.small.as_ref(),
            "Chosen now; it cannot be changed later.",
            r.0 + 66.0,
            cy + 3.0,
            tone::muted(1.0),
        );
        let named = !self.is_empty(Slot::FellowshipName);
        button(
            rc,
            self,
            h,
            (r.0 + 16.0, r.1 + r.3 - 16.0 - 38.0, 100.0, 38.0),
            "Create",
            ButtonLook::Primary,
            named,
            Hit::Ask(SocialAsk::Create),
        );
        // Or join one.
        let r = (PAD + w + 16.0, top, WIDTH - 2.0 * PAD - w - 16.0, 210.0);
        rc.stroke_rounded_rect(r.0, r.1, r.2, r.3, round, bronze(0.25), 1.0);
        put(
            rc,
            f.name.as_ref(),
            "Or join one",
            r.0 + 16.0,
            r.1 + 16.0,
            p.text,
        );
        if let Some(b) = &f.body {
            let mut y = r.1 + 46.0;
            for para in [
                "A leader can recruit you, and so can anyone in an open fellowship. You are asked first, unless you accept requests automatically below.",
                "A fellowship holds up to nine people.",
            ] {
                let lines = kit::wrap(b, para, r.2 - 32.0);
                y = kit::lines(
                    rc,
                    b,
                    &lines,
                    r.0 + 16.0,
                    y,
                    b.line_height + 5.0,
                    tone::soft(1.0),
                ) + 8.0;
            }
        }
        self.draw_fellow_switches(rc, h, HEIGHT - 24.0 - 70.0, false);
    }
}

/// A pill with `words` ending at `right`, centred on `cy`; returns its
/// left edge.
fn chip(
    rc: &mut RenderContext<'_>,
    f: &super::window::SocialFonts,
    right: f32,
    cy: f32,
    words: &str,
    ink: Color,
    edge: Color,
) -> f32 {
    let Some(l) = &f.strong else {
        return right;
    };
    let w = text::measure(l, words, 0.0) + 30.0;
    let r = (right - w, cy - 12.0, w, 24.0);
    rc.stroke_rounded_rect(r.0, r.1, r.2, r.3, CornerRadii::all(12.0), edge, 1.0);
    rc.fill_ellipse(r.0 + 10.0, cy - 3.0, 6.0, 6.0, ink);
    put_mid(rc, Some(l), words, r.0 + 22.0, r.1, r.3, ink);
    r.0
}

/// Where the roster's columns start.
struct Columns {
    level: f32,
    share: f32,
    vitals: [f32; 3],
    loot: f32,
}

impl Columns {
    fn new() -> Self {
        let right = WIDTH - PAD - 10.0;
        let loot = right - 24.0;
        let mana = loot - 12.0 - 110.0;
        let stamina = mana - 12.0 - 110.0;
        let health = stamina - 12.0 - 110.0;
        let share = health - 12.0 - 56.0;
        let level = share - 12.0 - 56.0;
        Self {
            level,
            share,
            vitals: [health, stamina, mana],
            loot,
        }
    }
}
