//! The Social window's friends and ignored pages, and the menu of what to
//! hide from someone.

use ac_ui::draw::RenderContext;
use ac_ui::modern::text::{self, TextStyle};
use ac_ui::modern::theme::Horizon;
use ac_ui::modern::theme::hex;
use ac_ui::modern::tone;
use ac_ui::shapes::CornerRadii;

use super::model::{FriendRow, HIDEABLE, MOST_LISTED};
use super::pages::{button, button_left_of, input, put, put_mid, put_right};
use super::window::{BODY, HEIGHT, Hit, Hits, PAD, Slot, SocialAsk, SocialFrame, WIDTH};
use crate::ui::modern::hud::lines::stroke;
use crate::ui::modern::kit::{self, ButtonLook, bronze, gold};

const FRIEND_ROW: f32 = 40.0;
const IGNORED_ROW: f32 = 46.0;
const MENU_W: f32 = 260.0;
const MENU_ROW: f32 = 30.0;
/// A cross, to stop ignoring.
const CROSS: &[&[(f32, f32)]] = &[&[(7.0, 7.0), (17.0, 17.0)], &[(17.0, 7.0), (7.0, 17.0)]];
/// A small chevron down, on a menu button.
const DOWN: &[&[(f32, f32)]] = &[&[(7.0, 10.0), (12.0, 15.0), (17.0, 10.0)]];
/// A tick, in a menu's box.
const TICK: &[&[(f32, f32)]] = &[&[(6.0, 12.0), (10.0, 16.0), (18.0, 8.0)]];

/// The friend's name typed to add.
#[must_use]
pub fn friend_field_rect() -> (f32, f32, f32, f32) {
    (PAD, BODY + 12.0, 280.0, 34.0)
}

/// The name typed to ignore.
#[must_use]
pub fn ignore_field_rect() -> (f32, f32, f32, f32) {
    (PAD, BODY + 12.0, 260.0, 34.0)
}

impl SocialFrame {
    /// The friends page.
    #[allow(clippy::too_many_lines)]
    pub(super) fn draw_friends(&mut self, rc: &mut RenderContext<'_>, h: &mut Hits) {
        let f = self.fonts.clone();
        let people = self.screen.people.clone();
        let count = people.online.len() + people.offline.len();
        // The row along the top.
        let field = friend_field_rect();
        input(rc, self, Slot::FriendName, "Add a friend by name");
        let x = field.0 + field.2 + 8.0;
        let room = count < MOST_LISTED;
        button(
            rc,
            self,
            h,
            (x, field.1, 64.0, 34.0),
            "Add",
            ButtonLook::Plain,
            room && !self.is_empty(Slot::FriendName),
            Hit::Ask(SocialAsk::AddFriend),
        );
        put_mid(
            rc,
            f.small.as_ref(),
            &format!("{count} of {MOST_LISTED}"),
            x + 74.0,
            field.1,
            field.3,
            tone::muted(1.0),
        );
        let on = self.screen.appear_offline;
        let note = if on {
            "Friends see you as offline"
        } else {
            "Friends see when you are on"
        };
        let note_w = f
            .small
            .as_ref()
            .map_or(160.0, |s| text::measure(s, note, 0.0))
            .max(
                f.body
                    .as_ref()
                    .map_or(90.0, |b| text::measure(b, "Appear offline", 0.0)),
            );
        let sx = WIDTH - PAD - note_w - 48.0;
        let cy = field.1 + field.3 * 0.5;
        let r = (sx, cy - 10.0, 38.0, 20.0);
        let lit = h.add(
            (sx, field.1, note_w + 48.0, field.3),
            Hit::Ask(SocialAsk::AppearOffline(!on)),
        );
        kit::switch(rc, r, on, lit);
        put(
            rc,
            f.body.as_ref(),
            "Appear offline",
            sx + 48.0,
            field.1 + 1.0,
            tone::warm(1.0),
        );
        put(
            rc,
            f.small.as_ref(),
            note,
            sx + 48.0,
            field.1 + 18.0,
            tone::muted(1.0),
        );
        // The list.
        let room = (
            PAD,
            BODY + 58.0,
            WIDTH - 2.0 * PAD,
            HEIGHT - 16.0 - (BODY + 58.0),
        );
        let groups = [
            (format!("ONLINE · {}", people.online.len()), &people.online),
            (
                format!("OFFLINE · {}", people.offline.len()),
                &people.offline,
            ),
        ];
        let total: f32 = groups
            .iter()
            .filter(|(_, rows)| !rows.is_empty())
            .map(|(_, rows)| 28.0 + rows.len() as f32 * (FRIEND_ROW + 2.0))
            .sum();
        self.totals[2] = (total, room);
        self.scroll[2].hold(total, room.3);
        if count == 0 {
            put(
                rc,
                f.body.as_ref(),
                "No friends yet. Type a name above, or click a name in chat to add one.",
                room.0,
                room.1 + 8.0,
                tone::muted(1.0),
            );
        }
        rc.push_clip(room.0, room.1, room.2, room.3);
        let mut y = room.1 - self.scroll[2].at;
        for (heading, rows) in groups {
            if rows.is_empty() {
                continue;
            }
            kit::group_heading(
                rc,
                &Horizon::current(),
                f.label.as_ref(),
                room.0,
                y,
                28.0,
                room.0 + room.2 - 8.0,
                &heading,
                None,
                tone::soft(1.0),
            );
            y += 28.0;
            for row in rows {
                self.draw_friend(rc, h, row, (room.0, y, room.2 - 8.0, FRIEND_ROW), room);
                y += FRIEND_ROW + 2.0;
            }
        }
        rc.pop_clip();
        kit::scrollbar(
            rc,
            room.0 + room.2 - 5.0,
            room.1,
            room.3,
            total,
            self.scroll[2].at,
        );
    }

    fn draw_friend(
        &self,
        rc: &mut RenderContext<'_>,
        h: &mut Hits,
        row: &FriendRow,
        r: (f32, f32, f32, f32),
        room: (f32, f32, f32, f32),
    ) {
        let p = Horizon::current().palette;
        let f = &self.fonts;
        let on = self.friend == row.id;
        let lit = h.add(r, Hit::Friend(row.id)) && kit::hot(h.hover, room);
        kit::pick_row(rc, r, on, lit);
        kit::online_dot(rc, r.0 + 10.0, r.1 + 16.0, 8.0, row.online);
        let mut x = r.0 + 28.0;
        if let Some(st) = &f.strong {
            let ink = if row.online { p.text } else { tone::soft(1.0) };
            put_mid(rc, Some(st), &row.name, x, r.1, r.3, ink);
            x += text::measure(st, &row.name, 0.0) + 10.0;
        }
        if let Some(s) = &f.small {
            for tag in &row.tags {
                let w = text::measure(s, tag, 0.0) + 12.0;
                let t = (x, r.1 + (r.3 - 18.0) * 0.5, w, 18.0);
                rc.stroke_rounded_rect(t.0, t.1, t.2, t.3, CornerRadii::all(3.0), bronze(0.3), 1.0);
                put_mid(rc, Some(s), tag, t.0 + 6.0, t.1, t.3, tone::soft(1.0));
                x += w + 6.0;
            }
        }
        if on {
            let by = r.1 + (r.3 - 28.0) * 0.5;
            let mut right = r.0 + r.2 - 6.0;
            right = button_left_of(
                rc,
                self,
                h,
                right,
                by,
                28.0,
                "Remove",
                ButtonLook::Danger,
                true,
                Hit::Ask(SocialAsk::Unfriend(row.name.clone())),
            ) - 6.0;
            right = button_left_of(
                rc,
                self,
                h,
                right,
                by,
                28.0,
                "Invite to fellowship",
                ButtonLook::Plain,
                row.online,
                Hit::Ask(SocialAsk::Invite(row.id)),
            ) - 6.0;
            button_left_of(
                rc,
                self,
                h,
                right,
                by,
                28.0,
                "Tell",
                ButtonLook::Plain,
                row.online,
                Hit::Ask(SocialAsk::Tell(row.name.clone())),
            );
        }
    }

    /// The ignored page.
    #[allow(clippy::too_many_lines)]
    pub(super) fn draw_ignored(&mut self, rc: &mut RenderContext<'_>, h: &mut Hits) {
        let p = Horizon::current().palette;
        let f = self.fonts.clone();
        let people = self.screen.people.clone();
        let count = people.ignored.len();
        // The row along the top.
        let field = ignore_field_rect();
        input(rc, self, Slot::IgnoreName, "Name to ignore");
        let named = !self.is_empty(Slot::IgnoreName) && count < MOST_LISTED;
        let mut x = field.0 + field.2 + 8.0;
        for (words, account) in [("Ignore character", false), ("Ignore whole account", true)] {
            let w = kit::button_width(words, f.strong.as_ref());
            button(
                rc,
                self,
                h,
                (x, field.1, w, 34.0),
                words,
                ButtonLook::Plain,
                named,
                Hit::Ask(SocialAsk::Ignore(account)),
            );
            x += w + 8.0;
        }
        put_right(
            rc,
            f.small.as_ref(),
            &format!("{count} of {MOST_LISTED}"),
            WIDTH - PAD,
            field.1,
            field.3,
            tone::muted(1.0),
        );
        // The people.
        let top = BODY + 58.0;
        kit::group_heading(
            rc,
            &Horizon::current(),
            f.label.as_ref(),
            PAD,
            top,
            28.0,
            WIDTH - PAD - 120.0,
            "PEOPLE",
            None,
            tone::soft(1.0),
        );
        if let Some(l) = &f.label {
            let words = "WHAT IS HIDDEN";
            let w = text::measure(l, words, kit::LABEL_TRACKING);
            text::draw(
                rc,
                l,
                words,
                WIDTH - PAD - w,
                top + ((28.0 - l.line_height) * 0.5).round(),
                TextStyle::plain(tone::soft(1.0)).tracked(kit::LABEL_TRACKING),
            );
        }
        let everyone_top = HEIGHT - 16.0 - 40.0 - 46.0 - 38.0;
        let room = (
            PAD,
            top + 28.0,
            WIDTH - 2.0 * PAD,
            everyone_top - (top + 28.0) - 4.0,
        );
        let total = count as f32 * (IGNORED_ROW + 2.0);
        self.totals[3] = (total, room);
        self.scroll[3].hold(total, room.3);
        if count == 0 {
            put(
                rc,
                f.body.as_ref(),
                "Nobody is ignored.",
                room.0,
                room.1 + 8.0,
                tone::muted(1.0),
            );
        }
        let mut menu_at = None;
        rc.push_clip(room.0, room.1, room.2, room.3);
        for (i, row) in people.ignored.iter().enumerate() {
            let y = room.1 + i as f32 * (IGNORED_ROW + 2.0) - self.scroll[3].at;
            if y + IGNORED_ROW < room.1 || y > room.1 + room.3 {
                continue;
            }
            let r = (room.0, y, room.2 - 8.0, IGNORED_ROW);
            let open = self.menu == Some(Some(i));
            kit::pick_row(rc, r, open, false);
            put(
                rc,
                f.strong.as_ref(),
                &row.name,
                r.0 + 12.0,
                y + 7.0,
                p.text,
            );
            put(
                rc,
                f.small.as_ref(),
                row.scope,
                r.0 + 12.0,
                y + 26.0,
                tone::muted(1.0),
            );
            let cross = (r.0 + r.2 - 6.0 - 30.0, y + 8.0, 30.0, 30.0);
            let lit = h.add(
                cross,
                Hit::Ask(SocialAsk::Unignore(row.name.clone(), row.account)),
            ) && kit::hot(h.hover, room);
            if lit {
                rc.fill_rounded_rect(
                    cross.0,
                    cross.1,
                    cross.2,
                    cross.3,
                    CornerRadii::all(4.0),
                    hex(0xFF_FFFF, 0.06),
                );
            }
            stroke(
                rc,
                cross.0 + 7.0,
                cross.1 + 7.0,
                16.0,
                CROSS,
                if lit { kit::HARM } else { tone::soft(1.0) },
                1.6,
            );
            let what = self.what_button(
                rc,
                h,
                &row.what,
                cross.0 - 6.0,
                y + 8.0,
                !row.account,
                Hit::WhatMenu(Some(i)),
            );
            if open {
                menu_at = Some((what, Some(row.name.clone()), row.kinds.clone()));
            }
        }
        rc.pop_clip();
        kit::scrollbar(
            rc,
            room.0 + room.2 - 5.0,
            room.1,
            room.3,
            total,
            self.scroll[3].at,
        );
        // Everyone.
        kit::group_heading(
            rc,
            &Horizon::current(),
            f.label.as_ref(),
            PAD,
            everyone_top,
            28.0,
            WIDTH - PAD,
            "FROM EVERYONE",
            None,
            tone::soft(1.0),
        );
        let r = (
            PAD,
            everyone_top + 34.0,
            WIDTH - 2.0 * PAD - 8.0,
            IGNORED_ROW,
        );
        let open = self.menu == Some(None);
        kit::pick_row(rc, r, open, false);
        put(
            rc,
            f.strong.as_ref(),
            "Everyone",
            r.0 + 12.0,
            r.1 + 7.0,
            p.text,
        );
        put(
            rc,
            f.small.as_ref(),
            "Hidden whoever sends it",
            r.0 + 12.0,
            r.1 + 26.0,
            tone::muted(1.0),
        );
        let what = self.what_button(
            rc,
            h,
            &people.everyone,
            r.0 + r.2 - 6.0 - 36.0,
            r.1 + 8.0,
            true,
            Hit::WhatMenu(None),
        );
        if open {
            menu_at = Some((what, None, people.everyone_kinds.clone()));
        }
        if let Some(s) = &f.small {
            put(
                rc,
                Some(s),
                "Ignoring a whole account hides every character on it, including ones made later. Click a name in chat to ignore someone from there.",
                PAD,
                HEIGHT - 16.0 - s.line_height,
                tone::muted(1.0),
            );
        }
        if let Some((at, who, kinds)) = menu_at {
            self.draw_menu(rc, h, at, who, &kinds);
        }
    }

    /// The button naming what is hidden, ending at `right`; returns its
    /// rectangle.
    #[allow(clippy::too_many_arguments)]
    fn what_button(
        &self,
        rc: &mut RenderContext<'_>,
        h: &mut Hits,
        words: &str,
        right: f32,
        y: f32,
        enabled: bool,
        hit: Hit,
    ) -> (f32, f32, f32, f32) {
        let f = &self.fonts;
        let words = f
            .strong
            .as_ref()
            .map_or_else(|| words.to_owned(), |st| kit::fit(st, words, 260.0));
        let w = f
            .strong
            .as_ref()
            .map_or(100.0, |st| text::measure(st, &words, 0.0))
            + 20.0
            + if enabled { 18.0 } else { 0.0 };
        let r = (right - w, y, w, 30.0);
        let lit = if enabled { h.add(r, hit) } else { false };
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
        put_mid(
            rc,
            f.strong.as_ref(),
            &words,
            r.0 + 10.0,
            r.1,
            r.3,
            Horizon::current().palette.text,
        );
        if enabled {
            stroke(
                rc,
                r.0 + r.2 - 22.0,
                r.1 + 8.0,
                14.0,
                DOWN,
                tone::warm(1.0),
                1.6,
            );
        }
        r
    }

    /// The menu of what to hide from `who` (none: everyone) under `at`.
    fn draw_menu(
        &self,
        rc: &mut RenderContext<'_>,
        h: &mut Hits,
        at: (f32, f32, f32, f32),
        who: Option<String>,
        kinds: &std::collections::HashSet<u32>,
    ) {
        let p = Horizon::current().palette;
        let f = &self.fonts;
        let items: Vec<_> = HIDEABLE
            .iter()
            .filter(|k| who.is_some() || k.kind != 1)
            .collect();
        let height = 30.0 + items.len() as f32 * MENU_ROW + 8.0;
        let x = (at.0 + at.2 - MENU_W).max(PAD);
        let mut y = at.1 + at.3 + 4.0;
        if y + height > HEIGHT - 8.0 {
            y = at.1 - 4.0 - height;
        }
        let b = (x, y, MENU_W, height);
        rc.soft_shadow(b.0, b.1, b.2, b.3, 6.0, 16.0, 40.0, hex(0, 0.6));
        rc.fill_rounded_rect(
            b.0,
            b.1,
            b.2,
            b.3,
            CornerRadii::all(6.0),
            tone::raised(0.98),
        );
        rc.stroke_rounded_rect(b.0, b.1, b.2, b.3, CornerRadii::all(6.0), bronze(0.8), 1.0);
        h.add(b, Hit::Swallow);
        let title = who.as_ref().map_or_else(
            || "Hide from everyone".to_owned(),
            |n| format!("Hide from {n}"),
        );
        put(
            rc,
            f.small.as_ref(),
            &title,
            b.0 + 14.0,
            b.1 + 9.0,
            tone::soft(1.0),
        );
        let all = kinds.contains(&1);
        for (i, k) in items.iter().enumerate() {
            let r = (
                b.0 + 6.0,
                b.1 + 30.0 + i as f32 * MENU_ROW,
                b.2 - 12.0,
                MENU_ROW,
            );
            let on = kinds.contains(&k.kind) || (all && who.is_some());
            let lit = h.add(
                r,
                Hit::Ask(SocialAsk::Hide(
                    who.clone(),
                    k.kind,
                    !kinds.contains(&k.kind),
                )),
            );
            if lit {
                rc.fill_rounded_rect(
                    r.0,
                    r.1,
                    r.2,
                    r.3,
                    CornerRadii::all(4.0),
                    hex(0xFF_FFFF, 0.05),
                );
            }
            let bx = (r.0 + 6.0, r.1 + 7.0, 16.0, 16.0);
            rc.fill_rounded_rect(
                bx.0,
                bx.1,
                bx.2,
                bx.3,
                CornerRadii::all(3.0),
                if on { gold(1.0) } else { hex(0, 0.0) },
            );
            rc.stroke_rounded_rect(
                bx.0,
                bx.1,
                bx.2,
                bx.3,
                CornerRadii::all(3.0),
                if on { gold(1.0) } else { gold(0.6) },
                1.5,
            );
            if on {
                stroke(rc, bx.0, bx.1, 16.0, TICK, tone::ink(1.0), 2.0);
            }
            let font = if k.kind == 1 {
                f.strong.as_ref()
            } else {
                f.body.as_ref()
            };
            put_mid(rc, font, k.words, r.0 + 32.0, r.1, r.3, p.text);
        }
    }
}
