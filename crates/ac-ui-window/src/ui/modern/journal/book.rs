//! The book reader, in place of retail's: the book's title (its
//! inscription when it has one), previous and next, a menu of every page
//! the book can hold with who wrote each, and the page. Another writer's
//! page is read only; a blank page or the player's own takes typing, saved
//! when the page is turned or the book closed.

use ac_ui::modern::tone;
use std::sync::Arc;

use ac_runtime::gameplay::{BookCommand, BookView, GameplayCommand};
use ac_runtime::session::SessionCommand;
use ac_ui::bundled_font::Weight;
use ac_ui::draw::RenderContext;
use ac_ui::event::{UiEvent, kind};
use ac_ui::geometry::AnchorEdges;
use ac_ui::modern::Horizon;
use ac_ui::modern::theme::hex;
use ac_ui::panels::items::emit;
use ac_ui::panels::{PanelOutput, panel_id};
use ac_ui::shapes::CornerRadii;
use ac_ui::tree::{Element, ElementId, Widget};
use ac_ui::widgets::Field;
use ac_ui::{UiFont, UiRoot};

use crate::ui::book_window::BOOK_WINDOW;
use crate::ui::modern::hud;
use crate::ui::modern::hud::lines::stroke;
use crate::ui::modern::kit::{self, Hits, Rect, bronze, gold, put_mid};
use crate::ui::window::WindowUi;

/// The window's name.
pub const BOOK: &str = "modern-book";

/// The window's size.
pub const WIDTH: f32 = 560.0;
pub const HEIGHT: f32 = 620.0;
const HEADER: f32 = 52.0;
const BAR: f32 = 48.0;
const FOOT: f32 = 52.0;
const PAD: f32 = 20.0;
const MENU_ROW: f32 = 30.0;

/// A closed book, the window's glyph.
const GLYPH: &[&[(f32, f32)]] = &[
    &[
        (5.0, 4.0),
        (17.0, 4.0),
        (19.0, 6.0),
        (19.0, 20.0),
        (7.0, 20.0),
        (5.0, 18.0),
        (5.0, 4.0),
    ],
    &[(5.0, 18.0), (17.0, 18.0), (19.0, 20.0)],
];
const PREV: &[&[(f32, f32)]] = &[&[(15.0, 6.0), (9.0, 12.0), (15.0, 18.0)]];
const NEXT: &[&[(f32, f32)]] = &[&[(9.0, 6.0), (15.0, 12.0), (9.0, 18.0)]];
const DOWN: &[&[(f32, f32)]] = &[&[(7.0, 10.0), (12.0, 15.0), (17.0, 10.0)]];
/// A pen, for the foot.
const PEN: &[&[(f32, f32)]] = &[&[
    (4.0, 20.0),
    (5.0, 15.0),
    (16.0, 4.0),
    (20.0, 8.0),
    (9.0, 19.0),
    (4.0, 20.0),
]];

/// The fonts the reader draws with.
#[derive(Debug, Clone, Default)]
pub struct BookFonts {
    /// The title.
    pub title: Option<Arc<UiFont>>,
    /// The page's name.
    pub strong: Option<Arc<UiFont>>,
    /// Who wrote it.
    pub small: Option<Arc<UiFont>>,
}

/// One page as the reader shows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BookPageRow {
    /// "Page 3".
    pub label: String,
    /// "by Mira", "Blank" or "Written before".
    pub by: String,
}

/// The book as the reader shows it.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct BookScreen {
    /// The title.
    pub title: String,
    /// Every page the book can hold.
    pub pages: Vec<BookPageRow>,
    /// The page read.
    pub current: usize,
    /// The page takes typing.
    pub writable: bool,
    /// The line under the page.
    pub foot: String,
}

/// What the reader asks of the window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BookAsk {
    /// Close the book.
    Close,
    /// Turn to a page.
    Turn(usize),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Hit {
    Ask(BookAsk),
    Menu,
}

/// The book reader's frame.
#[derive(Debug)]
pub struct BookFrame {
    fonts: BookFonts,
    /// What it shows.
    pub screen: BookScreen,
    /// The page list shows.
    pub menu: bool,
    /// What it asked for since the window last looked.
    pub asks: Vec<BookAsk>,
    hits: Hits<Hit>,
}

impl BookFrame {
    /// A frame in `theme`.
    #[must_use]
    pub fn new(fonts: BookFonts) -> Self {
        Self {
            fonts,
            screen: BookScreen::default(),
            menu: false,
            asks: Vec::new(),
            hits: Hits::default(),
        }
    }

    /// Where the page's text goes.
    #[must_use]
    pub fn page_rect() -> Rect {
        (
            PAD,
            HEADER + BAR,
            WIDTH - 2.0 * PAD,
            HEIGHT - HEADER - BAR - FOOT,
        )
    }
}

impl Widget for BookFrame {
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
                false
            }
            kind::HOVER_LEAVE => {
                self.hits.hover = None;
                false
            }
            kind::CLICK => {
                let was = self.menu;
                self.menu = false;
                match self.hits.at(x, y) {
                    Some(Hit::Ask(a)) => self.asks.push(a),
                    Some(Hit::Menu) => self.menu = !was,
                    None => {}
                }
                true
            }
            _ => false,
        }
    }

    #[allow(clippy::too_many_lines)]
    fn draw(&mut self, _ui: &mut UiRoot, _me: ElementId, rc: &mut RenderContext<'_>) {
        let p = Horizon::current().palette;
        let f = self.fonts.clone();
        let s = self.screen.clone();
        let mut h = Hits::new(self.hits.hover);
        kit::window_back(rc, &Horizon::current(), WIDTH, HEIGHT, HEADER, true);
        let close = kit::close_rect(WIDTH, HEADER);
        stroke(rc, 20.0, (HEADER - 18.0) * 0.5, 18.0, GLYPH, gold(1.0), 2.0);
        if let Some(t) = &f.title {
            let title = kit::fit(t, &s.title, close.0 - 56.0);
            put_mid(rc, Some(t), &title, 46.0, 0.0, HEADER, p.text);
        }
        let lit = h.add(close, Hit::Ask(BookAsk::Close));
        kit::close(rc, &Horizon::current(), close, lit);
        // The page bar.
        let y = HEADER + (BAR - 30.0) * 0.5;
        let last = s.pages.len().saturating_sub(1);
        let arrow = |rc: &mut RenderContext<'_>,
                     h: &mut Hits<Hit>,
                     x: f32,
                     glyph: &[&[(f32, f32)]],
                     to: usize,
                     on: bool| {
            let r = (x, y, 30.0, 30.0);
            let lit = on && h.add(r, Hit::Ask(BookAsk::Turn(to)));
            kit::button(
                rc,
                &Horizon::current(),
                r,
                "",
                None,
                kit::ButtonLook::Plain,
                lit,
                on,
            );
            stroke(rc, r.0 + 7.0, r.1 + 7.0, 16.0, glyph, tone::warm(1.0), 1.8);
        };
        arrow(
            rc,
            &mut h,
            12.0,
            PREV,
            s.current.saturating_sub(1),
            s.current > 0,
        );
        arrow(
            rc,
            &mut h,
            WIDTH - 42.0,
            NEXT,
            (s.current + 1).min(last),
            s.current < last,
        );
        let mb = (48.0, y, WIDTH - 96.0, 30.0);
        let lit = h.add(mb, Hit::Menu);
        let round = CornerRadii::all(4.0);
        rc.fill_rounded_rect(mb.0, mb.1, mb.2, mb.3, round, hex(0, 0.35));
        rc.stroke_rounded_rect(
            mb.0,
            mb.1,
            mb.2,
            mb.3,
            round,
            if self.menu || lit {
                gold(0.8)
            } else {
                bronze(0.45)
            },
            1.0,
        );
        if let Some(row) = s.pages.get(s.current) {
            put_mid(
                rc,
                f.strong.as_ref(),
                &row.label,
                mb.0 + 10.0,
                mb.1,
                mb.3,
                p.text,
            );
            let w = f
                .strong
                .as_ref()
                .map_or(60.0, |st| ac_ui::modern::text::measure(st, &row.label, 0.0));
            put_mid(
                rc,
                f.small.as_ref(),
                &row.by,
                mb.0 + 18.0 + w,
                mb.1,
                mb.3,
                tone::soft(1.0),
            );
        }
        let of = format!("of {}", s.pages.len());
        kit::put_right(
            rc,
            f.small.as_ref(),
            &of,
            mb.0 + mb.2 - 30.0,
            mb.1,
            mb.3,
            tone::muted(1.0),
        );
        stroke(
            rc,
            mb.0 + mb.2 - 24.0,
            mb.1 + 8.0,
            14.0,
            DOWN,
            tone::warm(1.0),
            1.6,
        );
        // The page.
        let pr = Self::page_rect();
        rc.fill_rounded_rect(pr.0, pr.1, pr.2, pr.3, round, tone::words(0.05));
        rc.stroke_rounded_rect(
            pr.0,
            pr.1,
            pr.2,
            pr.3,
            round,
            if s.writable { gold(0.5) } else { bronze(0.3) },
            1.0,
        );
        // The foot.
        stroke(
            rc,
            PAD,
            HEIGHT - FOOT + (FOOT - 14.0) * 0.5,
            14.0,
            PEN,
            tone::muted(1.0),
            1.4,
        );
        put_mid(
            rc,
            f.small.as_ref(),
            &s.foot,
            PAD + 22.0,
            HEIGHT - FOOT,
            FOOT,
            tone::muted(1.0),
        );
        self.hits = h;
    }
}

/// The page list over the page (a child, so it draws over the page's
/// field).
#[derive(Debug)]
pub struct BookMenu {
    fonts: BookFonts,
    /// The pages and the one read.
    pub pages: Vec<BookPageRow>,
    pub current: usize,
    /// The page picked since the window last looked.
    pub picked: Option<usize>,
    hover: Option<(f32, f32)>,
    scroll: kit::Scroll,
}

impl BookMenu {
    /// The list in `theme`.
    #[must_use]
    pub fn new(fonts: BookFonts) -> Self {
        Self {
            fonts,
            pages: Vec::new(),
            current: 0,
            picked: None,
            hover: None,
            scroll: kit::Scroll::default(),
        }
    }

    /// Its place in the frame for `rows` pages.
    #[must_use]
    pub fn rect(rows: usize) -> Rect {
        let h = (rows.max(1) as f32 * MENU_ROW + 8.0).min(HEIGHT - HEADER - 44.0 - 16.0);
        (48.0, HEADER + 44.0, WIDTH - 96.0, h)
    }
}

impl Widget for BookMenu {
    fn hit_test(&self, _element: &Element, _x: f32, _y: f32) -> bool {
        true
    }

    fn handles_click(&self, _element: &Element) -> bool {
        true
    }

    fn receives_hover_mouse_move(&self) -> bool {
        true
    }

    fn on_event(&mut self, ui: &mut UiRoot, me: ElementId, e: &UiEvent) -> bool {
        if e.target != Some(me) {
            return false;
        }
        let (x, y) = (e.data1 as f32, e.data2 as f32);
        let h = ui.element(me).map_or(0.0, |el| el.height);
        match e.kind {
            kind::MOUSE_MOVE | kind::HOVER_ENTER => {
                self.hover = Some((x, y));
                false
            }
            kind::HOVER_LEAVE => {
                self.hover = None;
                false
            }
            kind::CLICK => {
                let i = ((y - 4.0 + self.scroll.at) / MENU_ROW).floor();
                if i >= 0.0 && (i as usize) < self.pages.len() && x >= 0.0 {
                    self.picked = Some(i as usize);
                }
                true
            }
            kind::SCROLL => {
                let total = self.pages.len() as f32 * MENU_ROW + 8.0;
                self.scroll.wheel(e.data0, MENU_ROW * 2.0, total, h);
                true
            }
            _ => false,
        }
    }

    fn draw(&mut self, ui: &mut UiRoot, me: ElementId, rc: &mut RenderContext<'_>) {
        let p = Horizon::current().palette;
        let (w, h) = ui.element(me).map_or((0.0, 0.0), |e| (e.width, e.height));
        let round = CornerRadii::all(6.0);
        rc.soft_shadow(0.0, 0.0, w, h, 6.0, 16.0, 40.0, hex(0, 0.6));
        rc.fill_rounded_rect(0.0, 0.0, w, h, round, tone::raised(0.99));
        rc.stroke_rounded_rect(0.0, 0.0, w, h, round, bronze(0.8), 1.0);
        let total = self.pages.len() as f32 * MENU_ROW + 8.0;
        self.scroll.hold(total, h);
        rc.push_clip(0.0, 0.0, w, h);
        for (i, row) in self.pages.iter().enumerate() {
            let r = (
                4.0,
                4.0 + i as f32 * MENU_ROW - self.scroll.at,
                w - 8.0,
                MENU_ROW,
            );
            let on = i == self.current;
            if on || kit::hot(self.hover, r) {
                rc.fill_rounded_rect(
                    r.0,
                    r.1,
                    r.2,
                    r.3,
                    CornerRadii::all(4.0),
                    if on { gold(0.14) } else { hex(0xFF_FFFF, 0.05) },
                );
            }
            put_mid(
                rc,
                self.fonts.strong.as_ref(),
                &row.label,
                r.0 + 8.0,
                r.1,
                r.3,
                p.text,
            );
            let ink = if row.by == "Blank" {
                tone::muted(1.0)
            } else {
                tone::soft(1.0)
            };
            put_mid(
                rc,
                self.fonts.small.as_ref(),
                &row.by,
                r.0 + 74.0,
                r.1,
                r.3,
                ink,
            );
        }
        rc.pop_clip();
        kit::scrollbar(rc, w - 7.0, 4.0, h - 8.0, total, self.scroll.at);
    }
}

/// The reader's elements and what it remembers.
#[derive(Debug)]
pub struct BookUi {
    /// The window.
    pub frame: ElementId,
    /// The page's text.
    pub page: ElementId,
    /// The page list.
    pub menu: ElementId,
    /// The book's revision last shown.
    rendered: Option<u64>,
    /// The book shown last frame (0: none).
    was_open: u32,
    /// A book was open when the game last said (a book opening shows the
    /// reader, a book closing hides it).
    told_open: bool,
}

/// The reader's screen for `view`.
#[must_use]
pub fn book_screen(view: &BookView, player: u32, book_name: &str) -> BookScreen {
    let pages = (0..view.max_num_pages.max(0))
        .map(|i| {
            let page = usize::try_from(i).ok().and_then(|i| view.pages.get(i));
            let by = match page {
                None => "Blank".to_owned(),
                Some(p) if p.author_id == player && player != 0 => "by you".to_owned(),
                Some(p) if p.author_name.trim().is_empty() => "Written before".to_owned(),
                Some(p) => format!("by {}", p.author_name),
            };
            BookPageRow {
                label: format!("Page {}", i + 1),
                by,
            }
        })
        .collect();
    let current = usize::try_from(view.current_page).unwrap_or(0);
    let page = view.pages.get(current);
    let writable = page
        .is_none_or(|p| p.text_included != 0 && (p.ignore_author != 0 || p.author_id == player));
    let foot = match page {
        _ if view.request_pending => "Fetching the page…".to_owned(),
        None => {
            "This page is blank. What you write is saved when you turn the page or close the book."
                .to_owned()
        }
        Some(p) if p.text_included == 0 => "Fetching the page…".to_owned(),
        Some(_) if writable => {
            "What you write is saved when you turn the page or close the book.".to_owned()
        }
        Some(p) if p.author_name.trim().is_empty() => {
            "Written before. It cannot be changed.".to_owned()
        }
        Some(p) => format!("Written by {}. Only they can change it.", p.author_name),
    };
    BookScreen {
        title: if view.scribe_id != 0 && !view.inscription.is_empty() {
            view.inscription.clone()
        } else {
            book_name.to_owned()
        },
        pages,
        current,
        writable,
        foot,
    }
}

impl WindowUi {
    /// The book reader, hidden, in place of retail's.
    pub(in crate::ui) fn mount_modern_book(&mut self) {
        let theme = Horizon::current();
        let fonts = BookFonts {
            title: self.modern_font(Weight::SemiBold, 15.0),
            strong: self.modern_font(Weight::SemiBold, 12.5),
            small: self.modern_font(Weight::Regular, 11.5),
        };
        let page_font = self.modern_font(Weight::Regular, 14.0);
        let frame = match hud::mount_piece(
            &mut self.host.root,
            BOOK,
            (WIDTH, HEIGHT),
            BookFrame::new(fonts.clone()),
        ) {
            Ok(id) => id,
            Err(e) => {
                self.notices.push(format!("ui: modern book: {e}"));
                return;
            }
        };
        let ui = &mut self.host.root;
        if let Some(e) = ui.element_mut(frame) {
            e.draggable = true;
        }
        let pr = BookFrame::page_rect();
        let page = kit::mount_field(
            ui,
            frame,
            (pr.0 + 14.0, pr.1 + 14.0, pr.2 - 28.0, pr.3 - 28.0),
            page_font,
            &theme,
            true,
        );
        let mr = BookMenu::rect(10);
        let menu = ui.add(
            frame,
            Element::at(mr.0, mr.1, mr.2, mr.3)
                .with_anchors(AnchorEdges::NONE)
                .with_visible(false),
            BookMenu::new(fonts),
        );
        ui.hide_window(BOOK);
        let Some(modern) = self.modern.as_mut() else {
            return;
        };
        modern.book = Some(BookUi {
            frame,
            page,
            menu,
            rendered: None,
            was_open: 0,
            told_open: false,
        });
        modern.replace(BOOK_WINDOW);
        self.panels.stand_in(panel_id::BOOK, BOOK);
    }

    /// What is typed on the page, when it is the reader's to write.
    fn book_typed(&self, page: ElementId) -> Option<String> {
        self.host
            .root
            .widget::<Field>(page)
            .filter(|f| f.editable())
            .map(Field::text)
    }

    /// The game opened or closed a book: the reader shows or goes.
    pub(in crate::ui) fn follow_modern_book(&mut self, dirty: crate::ui::facts::Dirty) {
        if !dirty.book {
            return;
        }
        let open = self.facts.book.as_ref().is_some_and(|v| v.book_guid != 0);
        let Some(b) = self.modern.as_mut().and_then(|m| m.book.as_mut()) else {
            return;
        };
        if b.told_open == open {
            return;
        }
        b.told_open = open;
        self.panels
            .set_visibility(&mut self.host.root, panel_id::BOOK, open);
    }

    /// Each frame: the reader follows the open book; turning a page or
    /// closing the book saves what was typed.
    pub(in crate::ui) fn tick_modern_book(&mut self) {
        let Some(b) = self.modern.as_ref().and_then(|m| m.book.as_ref()) else {
            return;
        };
        let (frame, page, menu, rendered, was_open) =
            (b.frame, b.page, b.menu, b.rendered, b.was_open);
        let view = self.facts.book.clone().unwrap_or_default();
        let open = self.host.root.is_window_visible(BOOK);
        // The reader went away with the book open: save and close it.
        if !open && was_open != 0 && view.book_guid == was_open {
            let typed = self.book_typed(page);
            emit(
                &mut self.host.root,
                PanelOutput::Session(SessionCommand::Gameplay(GameplayCommand::Book(
                    BookCommand::CloseWriting(typed),
                ))),
            );
        }
        let now_open = if open { view.book_guid } else { 0 };
        if let Some(b) = self.modern.as_mut().and_then(|m| m.book.as_mut()) {
            b.was_open = now_open;
        }
        if !open {
            let ui = &mut self.host.root;
            if ui.keyboard_focus() == Some(page) {
                ui.set_keyboard_focus(None);
            }
            return;
        }
        if rendered != Some(view.revision) {
            let player = self.facts.state.player;
            let names = crate::ui::facts::composed_names(&self.status.names);
            let name = self
                .facts
                .objects
                .get(view.book_guid)
                .map(names)
                .unwrap_or_default();
            let screen = book_screen(&view, player, &name);
            let text = usize::try_from(view.current_page)
                .ok()
                .and_then(|i| view.pages.get(i))
                .filter(|p| p.text_included != 0)
                .map(|p| p.page_text.clone())
                .unwrap_or_default();
            let most = usize::try_from(view.max_num_chars_per_page).unwrap_or(0);
            let ui = &mut self.host.root;
            let writable = screen.writable;
            ui.with::<Field, _>(page, |f, ui| {
                f.set_text(&text);
                f.set_editable(ui, page, writable);
                if most > 0 {
                    f.max_characters = most;
                }
            });
            if let Some(m) = ui.widget_mut::<BookMenu>(menu) {
                m.pages.clone_from(&screen.pages);
                m.current = screen.current;
            }
            if let Some(f) = ui.widget_mut::<BookFrame>(frame) {
                f.screen = screen;
            }
            if let Some(b) = self.modern.as_mut().and_then(|m| m.book.as_mut()) {
                b.rendered = Some(view.revision);
            }
        }
        // The page list.
        let ui = &mut self.host.root;
        let (asks, show_menu, rows) = ui
            .widget_mut::<BookFrame>(frame)
            .map(|f| (std::mem::take(&mut f.asks), f.menu, f.screen.pages.len()))
            .unwrap_or_default();
        let picked = ui
            .widget_mut::<BookMenu>(menu)
            .and_then(|m| m.picked.take());
        let r = BookMenu::rect(rows);
        if let Some(e) = ui.element_mut(menu) {
            e.height = r.3;
        }
        let shows = ui.element(menu).is_some_and(|e| e.visible);
        if shows != show_menu {
            ui.set_visible(menu, show_menu);
        }
        let mut turns: Vec<usize> = Vec::new();
        if let Some(i) = picked {
            turns.push(i);
            if let Some(f) = ui.widget_mut::<BookFrame>(frame) {
                f.menu = false;
            }
        }
        for ask in asks {
            match ask {
                BookAsk::Close => {
                    self.panels
                        .set_visibility(&mut self.host.root, panel_id::BOOK, false);
                }
                BookAsk::Turn(i) => turns.push(i),
            }
        }
        for i in turns {
            let typed = self.book_typed(page);
            emit(
                &mut self.host.root,
                PanelOutput::Session(SessionCommand::Gameplay(GameplayCommand::Book(
                    BookCommand::TurnWriting {
                        page: i32::try_from(i).unwrap_or(0),
                        typed,
                    },
                ))),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ac_proto::book::BookPage;

    #[test]
    fn a_page_is_the_readers_to_write_when_blank_or_theirs() {
        let page = |author: u32, name: &str| BookPage {
            author_id: author,
            author_name: name.to_owned(),
            author_account: String::new(),
            text_included: 1,
            ignore_author: 0,
            page_text: "Words".to_owned(),
        };
        let mut view = BookView {
            book_guid: 9,
            max_num_pages: 3,
            pages: vec![page(7, "Mira"), page(1, "Aerin")],
            ..BookView::default()
        };
        let s = book_screen(&view, 1, "A Book");
        assert_eq!(s.title, "A Book");
        assert_eq!(s.pages.len(), 3);
        assert_eq!(s.pages[0].by, "by Mira");
        assert_eq!(s.pages[1].by, "by you");
        assert_eq!(s.pages[2].by, "Blank");
        assert!(!s.writable, "Mira's page");
        assert_eq!(s.foot, "Written by Mira. Only they can change it.");
        view.current_page = 1;
        assert!(book_screen(&view, 1, "").writable);
        view.current_page = 2;
        assert!(book_screen(&view, 1, "").writable, "a blank page");
        view.scribe_id = 3;
        view.inscription = "Field Notes".to_owned();
        assert_eq!(book_screen(&view, 1, "A Book").title, "Field Notes");
    }
}
