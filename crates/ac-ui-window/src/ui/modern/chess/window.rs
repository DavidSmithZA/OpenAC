//! The Chess window, in place of retail's Game Center: the board from the
//! player's side (Black's view when only watching) in the game's own
//! pieces, the last move marked; the two sides with whose move it is, a
//! status line, Offer stalemate and Resign (asked first). OPENAC: a piece
//! picked up shows where it can go (dots) and what it can take (a ring),
//! and the moves made are listed.

use std::sync::Arc;

use ac_core::chess::{ChessPiece, ChessPieceType, ChessTeam};
use ac_runtime::gameplay::{ChessCommand, GameplayCommand};
use ac_runtime::session::SessionCommand;
use ac_ui::bundled_font::Weight;
use ac_ui::draw::{Color, RenderContext, TextureRef};
use ac_ui::event::{UiEvent, kind};
use ac_ui::import::import_infos_rooted;
use ac_ui::modern::Horizon;
use ac_ui::modern::text::{self, TextStyle};
use ac_ui::modern::theme::hex;
use ac_ui::panels::items::emit;
use ac_ui::panels::mini_game::{MINI_GAME_LAYOUT_ID, MINI_GAME_ROOT_ID, read_piece_images};
use ac_ui::panels::{PanelOutput, panel_id};
use ac_ui::shapes::CornerRadii;
use ac_ui::tree::{Element, ElementId, Widget};
use ac_ui::{UiFont, UiRoot};

use super::model::{ChessScreen, SquareView, can_pick, chess_screen};
use crate::ui::indicator_panels::MINI_GAME_WINDOW;
use crate::ui::modern::hud;
use crate::ui::modern::kit::{self, Hits, MUTED, Rect, SOFT, bronze, gold, put, put_mid};
use crate::ui::window::WindowUi;

/// The window's name.
pub const CHESS: &str = "modern-chess";

/// The window's size.
pub const WIDTH: f32 = 800.0;
pub const HEIGHT: f32 = 560.0;
const HEADER: f32 = 52.0;
/// A square's side.
const SQUARE: f32 = 56.0;
/// The board's top left.
const BOARD_X: f32 = 38.0;
const BOARD_Y: f32 = 70.0;
/// The side column.
const SIDE_X: f32 = 500.0;
const SIDE_W: f32 = WIDTH - 20.0 - SIDE_X;
const SIDE_ROW: f32 = 44.0;
const BUTTON_H: f32 = 36.0;
const MOVE_ROW: f32 = 20.0;
/// The resign card's width.
const CARD_W: f32 = 360.0;

const LIGHT: Color = hex(0xD9_C7A3, 1.0);
const DARK: Color = hex(0x8C_6A44, 1.0);
const LIGHT_LAST: Color = hex(0xE3_CF8A, 1.0);
const DARK_LAST: Color = hex(0xA8_8A4E, 1.0);
const PICKED: Color = hex(0xC9_A35A, 1.0);

/// A chess piece, the window's glyph.
const GLYPH: &[&[(f32, f32)]] = &[
    &[(9.0, 21.0), (15.0, 21.0)],
    &[
        (8.0, 18.0),
        (16.0, 18.0),
        (15.0, 12.0),
        (9.0, 12.0),
        (8.0, 18.0),
    ],
    &[
        (9.0, 12.0),
        (7.0, 9.0),
        (9.0, 7.0),
        (8.0, 4.0),
        (16.0, 4.0),
        (15.0, 7.0),
        (17.0, 9.0),
        (15.0, 12.0),
    ],
];

/// The fonts the window draws with.
#[derive(Debug, Clone, Default)]
pub struct ChessFonts {
    /// The title and the card's title.
    pub title: Option<Arc<UiFont>>,
    /// The sides' names and the buttons.
    pub strong: Option<Arc<UiFont>>,
    /// The status line and the card's words.
    pub body: Option<Arc<UiFont>>,
    /// The moves.
    pub moves: Option<Arc<UiFont>>,
    /// The labels round the board, the header's note.
    pub small: Option<Arc<UiFont>>,
    /// Spaced capitals and the OPENAC tag.
    pub label: Option<Arc<UiFont>>,
}

/// What the window asks of the game.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChessAsk {
    /// A square was clicked (window order).
    Square(usize),
    /// Let go of the piece picked up.
    Drop,
    /// Offer a stalemate, or take the offer back.
    Stalemate,
    /// Resign (the card was answered yes).
    Resign,
    /// Close the window; the game goes on.
    Close,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Hit {
    Ask(ChessAsk),
    AskResign,
    KeepPlaying,
    Card,
}

/// The Chess window's frame.
#[derive(Debug)]
pub struct ChessFrame {
    theme: Horizon,
    fonts: ChessFonts,
    /// What it shows.
    pub screen: ChessScreen,
    /// The board's name.
    pub board_name: String,
    /// The game's piece images: White's six, then Black's, each pawn,
    /// bishop, knight, rook, queen, king (0: none).
    pub pieces: [u32; 12],
    /// The resign question shows.
    pub resigning: bool,
    /// What it asked for since the window last looked.
    pub asks: Vec<ChessAsk>,
    hits: Hits<Hit>,
    scroll: kit::Scroll,
    /// The moves shown last frame, to follow the newest.
    seen_moves: usize,
}

impl ChessFrame {
    /// A frame in `theme`.
    #[must_use]
    pub fn new(theme: Horizon, fonts: ChessFonts) -> Self {
        Self {
            theme,
            fonts,
            screen: ChessScreen::default(),
            board_name: String::new(),
            pieces: [0; 12],
            resigning: false,
            asks: Vec::new(),
            hits: Hits::default(),
            scroll: kit::Scroll::default(),
            seen_moves: 0,
        }
    }

    /// Window square `index`'s place.
    #[must_use]
    pub fn square_rect(index: usize) -> Rect {
        let (col, row) = ((index % 8) as f32, (index / 8) as f32);
        (
            BOARD_X + col * SQUARE,
            BOARD_Y + row * SQUARE,
            SQUARE,
            SQUARE,
        )
    }

    /// Offer stalemate's and Resign's places.
    #[must_use]
    pub fn button_rects() -> (Rect, Rect) {
        let y = HEIGHT - 20.0 - BUTTON_H;
        let w = (SIDE_W - 8.0) * 0.5;
        ((SIDE_X, y, w, BUTTON_H), (SIDE_X + w + 8.0, y, w, BUTTON_H))
    }

    /// The resign card's place and its two buttons (Keep playing, Resign).
    #[must_use]
    pub fn card_rects(&self) -> (Rect, Rect, Rect) {
        let body = self.card_lines().len() as f32 * 19.0;
        let h = 18.0 + 22.0 + 10.0 + body + 14.0 + 34.0 + 18.0;
        let card = ((WIDTH - CARD_W) * 0.5, (HEIGHT - h) * 0.5, CARD_W, h);
        let by = card.1 + h - 18.0 - 34.0;
        let resign_w = kit::button_width("Resign", self.fonts.strong.as_ref()) + 4.0;
        let keep_w = kit::button_width("Keep playing", self.fonts.strong.as_ref());
        let resign = (card.0 + CARD_W - 18.0 - resign_w, by, resign_w, 34.0);
        let keep = (resign.0 - 8.0 - keep_w, by, keep_w, 34.0);
        (card, keep, resign)
    }

    fn card_lines(&self) -> Vec<String> {
        let words = if self.screen.playing {
            "Your opponent wins the game, and it is recorded as your loss."
        } else {
            "You leave the board."
        };
        self.fonts.body.as_ref().map_or_else(
            || vec![words.to_owned()],
            |f| kit::wrap(f, words, CARD_W - 36.0),
        )
    }

    fn piece_image(&self, piece: ChessPiece) -> u32 {
        let side = if piece.team == ChessTeam::White { 0 } else { 6 };
        let kind = match piece.piece_type {
            ChessPieceType::Pawn => 0,
            ChessPieceType::Bishop => 1,
            ChessPieceType::Knight => 2,
            ChessPieceType::Rook => 3,
            ChessPieceType::Queen => 4,
            ChessPieceType::King => 5,
        };
        self.pieces[side + kind]
    }

    fn draw_square(&self, rc: &mut RenderContext<'_>, r: Rect, q: &SquareView, lit: bool) {
        let mut back = match (q.light, q.last) {
            (true, false) => LIGHT,
            (false, false) => DARK,
            (true, true) => LIGHT_LAST,
            (false, true) => DARK_LAST,
        };
        if q.picked {
            back = PICKED;
        }
        rc.fill(r.0, r.1, r.2, r.3, back);
        if lit && !q.picked {
            rc.fill(r.0, r.1, r.2, r.3, hex(0xFF_FFFF, 0.14));
        }
        if q.capture {
            rc.rect_outline(r.0, r.1, r.2, r.3, hex(0xE5_484D, 0.85), 4.0);
        }
        if let Some(p) = q.piece {
            let image = self.piece_image(p);
            if image != 0 {
                let side = r.2 - 8.0;
                rc.sprite(
                    TextureRef::Surface(image),
                    r.0 + 4.0,
                    r.1 + 4.0,
                    side,
                    side,
                    (0.0, 0.0, 1.0, 1.0),
                    Color::WHITE,
                );
            } else {
                // No image in the dats: a disc with the piece's letter.
                let white = p.team == ChessTeam::White;
                let disc = if white {
                    hex(0xFB_F6EC, 1.0)
                } else {
                    hex(0x1A_140D, 1.0)
                };
                rc.fill_ellipse(r.0 + 10.0, r.1 + 10.0, r.2 - 20.0, r.3 - 20.0, disc);
                let letter = match p.piece_type {
                    ChessPieceType::Pawn => "P",
                    ChessPieceType::Bishop => "B",
                    ChessPieceType::Knight => "N",
                    ChessPieceType::Rook => "R",
                    ChessPieceType::Queen => "Q",
                    ChessPieceType::King => "K",
                };
                if let Some(f) = &self.fonts.title {
                    let w = text::measure(f, letter, 0.0);
                    let ink = if white {
                        hex(0x1A_140D, 1.0)
                    } else {
                        hex(0xFB_F6EC, 1.0)
                    };
                    text::draw(
                        rc,
                        f,
                        letter,
                        (r.0 + (r.2 - w) * 0.5).round(),
                        r.1 + ((r.3 - f.line_height) * 0.5).round(),
                        TextStyle::plain(ink),
                    );
                }
            }
        }
        if q.target {
            let d = 14.0;
            rc.fill_ellipse(
                r.0 + (r.2 - d) * 0.5,
                r.1 + (r.3 - d) * 0.5,
                d,
                d,
                hex(0x1A_140D, 0.45),
            );
        }
    }

    #[allow(clippy::too_many_lines)]
    fn draw_side(&mut self, rc: &mut RenderContext<'_>, h: &mut Hits<Hit>) {
        let p = self.theme.palette;
        let f = self.fonts.clone();
        let s = &self.screen;
        let mut y = BOARD_Y;
        for side in &s.sides {
            let r = (SIDE_X, y, SIDE_W, SIDE_ROW);
            let round = CornerRadii::all(4.0);
            let (edge, fill) = if side.moving {
                (gold(1.0), gold(0.12))
            } else {
                (bronze(0.25), hex(0, 0.22))
            };
            rc.fill_rounded_rect(r.0, r.1, r.2, r.3, round, fill);
            rc.stroke_rounded_rect(r.0, r.1, r.2, r.3, round, edge, 1.0);
            let disc = if side.white {
                hex(0xFB_F6EC, 1.0)
            } else {
                hex(0x1A_140D, 1.0)
            };
            let cy = r.1 + r.3 * 0.5;
            rc.fill_ellipse(r.0 + 12.0, cy - 7.0, 14.0, 14.0, disc);
            rc.stroke_ellipse(r.0 + 12.0, cy - 7.0, 14.0, 14.0, bronze(0.6), 1.0);
            put_mid(
                rc,
                f.strong.as_ref(),
                &side.name,
                r.0 + 36.0,
                r.1,
                r.3,
                p.text,
            );
            if !side.turn.is_empty() {
                let ink = if side.moving { gold(1.0) } else { MUTED };
                kit::put_right(
                    rc,
                    f.label.as_ref(),
                    side.turn,
                    r.0 + r.2 - 12.0,
                    r.1,
                    r.3,
                    ink,
                );
            }
            y += SIDE_ROW + 10.0;
        }
        // The status line.
        let lines = f.body.as_ref().map_or_else(
            || vec![s.status.clone()],
            |b| kit::wrap(b, &s.status, SIDE_W - 24.0),
        );
        let sh = lines.len() as f32 * 19.0 + 20.0;
        let round = CornerRadii::all(4.0);
        rc.fill_rounded_rect(SIDE_X, y, SIDE_W, sh, round, gold(0.10));
        rc.stroke_rounded_rect(SIDE_X, y, SIDE_W, sh, round, gold(0.45), 1.0);
        for (i, line) in lines.iter().enumerate() {
            put(
                rc,
                f.body.as_ref(),
                line,
                SIDE_X + 12.0,
                y + 10.0 + i as f32 * 19.0,
                p.text,
            );
        }
        y += sh + 14.0;
        // The moves.
        let mut x = SIDE_X;
        if let Some(l) = &f.label {
            text::draw(
                rc,
                l,
                "MOVES",
                x,
                y,
                TextStyle::plain(SOFT).tracked(kit::LABEL_TRACKING),
            );
            x += text::measure(l, "MOVES", kit::LABEL_TRACKING) + 8.0;
        }
        let cy = y + 6.0;
        let tag_w = 60.0;
        rc.fill(
            x,
            cy,
            (SIDE_X + SIDE_W - tag_w - 8.0 - x).max(0.0),
            1.0,
            bronze(0.22),
        );
        kit::openac_tag(rc, f.label.as_ref(), SIDE_X + SIDE_W - tag_w, cy);
        y += 22.0;
        let (stalemate, resign) = Self::button_rects();
        let room = (stalemate.1 - 12.0 - y).max(0.0);
        let total = s.moves.len() as f32 * MOVE_ROW;
        if s.moves.len() != self.seen_moves {
            self.seen_moves = s.moves.len();
            self.scroll.at = (total - room).max(0.0);
        }
        self.scroll.hold(total, room);
        rc.push_clip(SIDE_X, y, SIDE_W, room);
        if s.moves.is_empty() {
            put(rc, f.moves.as_ref(), "No moves yet.", SIDE_X, y, MUTED);
        }
        let col = (SIDE_W - 24.0) * 0.5;
        for (i, m) in s.moves.iter().enumerate() {
            let ty = y + i as f32 * MOVE_ROW - self.scroll.at;
            put(
                rc,
                f.moves.as_ref(),
                &format!("{}.", m.number),
                SIDE_X,
                ty,
                MUTED,
            );
            let ink = |on: bool| if on { gold(1.0) } else { p.text };
            let (wf, bf) = (
                if m.last == 1 { &f.strong } else { &f.moves },
                if m.last == 2 { &f.strong } else { &f.moves },
            );
            put(
                rc,
                wf.as_ref(),
                &m.white,
                SIDE_X + 24.0,
                ty,
                ink(m.last == 1),
            );
            put(
                rc,
                bf.as_ref(),
                &m.black,
                SIDE_X + 24.0 + col,
                ty,
                ink(m.last == 2),
            );
        }
        rc.pop_clip();
        if total > room {
            kit::scrollbar(rc, SIDE_X + SIDE_W - 4.0, y, room, total, self.scroll.at);
        }
        // The buttons.
        let words = if s.offering {
            "Take back stalemate"
        } else {
            "Offer stalemate"
        };
        let lit = s.playing && h.add(stalemate, Hit::Ask(ChessAsk::Stalemate));
        kit::button(
            rc,
            &self.theme,
            stalemate,
            words,
            f.strong.as_ref(),
            kit::ButtonLook::Plain,
            lit,
            s.playing,
        );
        let lit = s.playing && h.add(resign, Hit::AskResign);
        kit::button(
            rc,
            &self.theme,
            resign,
            "Resign\u{2026}",
            f.strong.as_ref(),
            kit::ButtonLook::Danger,
            lit,
            s.playing,
        );
    }

    fn draw_card(&self, rc: &mut RenderContext<'_>, h: &mut Hits<Hit>) {
        let f = &self.fonts;
        rc.fill_rounded_rect(
            0.0,
            0.0,
            WIDTH,
            HEIGHT,
            CornerRadii::all(self.theme.metrics.window_radius),
            hex(0x0A_0806, 0.6),
        );
        let (card, keep, resign) = self.card_rects();
        h.add((0.0, 0.0, WIDTH, HEIGHT), Hit::Card);
        h.add(card, Hit::Card);
        let round = CornerRadii::all(6.0);
        rc.soft_shadow(card.0, card.1, card.2, card.3, 6.0, 16.0, 40.0, hex(0, 0.6));
        rc.fill_rounded_rect(card.0, card.1, card.2, card.3, round, hex(0x18_130E, 0.99));
        rc.stroke_rounded_rect(card.0, card.1, card.2, card.3, round, bronze(0.8), 1.0);
        put(
            rc,
            f.title.as_ref(),
            "Resign the game?",
            card.0 + 18.0,
            card.1 + 18.0,
            self.theme.palette.text,
        );
        for (i, line) in self.card_lines().iter().enumerate() {
            put(
                rc,
                f.body.as_ref(),
                line,
                card.0 + 18.0,
                card.1 + 50.0 + i as f32 * 19.0,
                kit::WARM,
            );
        }
        let lit = h.add(keep, Hit::KeepPlaying);
        kit::button(
            rc,
            &self.theme,
            keep,
            "Keep playing",
            f.strong.as_ref(),
            kit::ButtonLook::Plain,
            lit,
            true,
        );
        let lit = h.add(resign, Hit::Ask(ChessAsk::Resign));
        let rr = CornerRadii::all(4.0);
        rc.fill_rounded_rect(
            resign.0,
            resign.1,
            resign.2,
            resign.3,
            rr,
            hex(0xE5_484D, if lit { 1.0 } else { 0.9 }),
        );
        if let Some(sf) = &f.strong {
            let w = text::measure(sf, "Resign", 0.0);
            text::draw(
                rc,
                sf,
                "Resign",
                (resign.0 + (resign.2 - w) * 0.5).round(),
                resign.1 + ((resign.3 - sf.line_height) * 0.5).round(),
                TextStyle::plain(Color::WHITE),
            );
        }
    }
}

impl Widget for ChessFrame {
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
                match self.hits.at(x, y) {
                    Some(Hit::Ask(a)) => {
                        if a == ChessAsk::Resign {
                            self.resigning = false;
                        }
                        self.asks.push(a);
                    }
                    Some(Hit::AskResign) => self.resigning = true,
                    Some(Hit::KeepPlaying) => self.resigning = false,
                    Some(Hit::Card) | None => {}
                }
                true
            }
            kind::RIGHT_CLICK => {
                self.asks.push(ChessAsk::Drop);
                true
            }
            kind::SCROLL => {
                let total = self.screen.moves.len() as f32 * MOVE_ROW;
                self.scroll.wheel(e.data0, MOVE_ROW * 2.0, total, 200.0);
                true
            }
            _ => false,
        }
    }

    fn draw(&mut self, _ui: &mut UiRoot, _me: ElementId, rc: &mut RenderContext<'_>) {
        let f = self.fonts.clone();
        let mut h = Hits::new(if self.resigning {
            None
        } else {
            self.hits.hover
        });
        kit::window_back(rc, &self.theme, WIDTH, HEIGHT, HEADER, true);
        let end = kit::title(rc, &self.theme, f.title.as_ref(), GLYPH, "Chess", HEADER);
        put_mid(
            rc,
            f.small.as_ref(),
            &self.board_name,
            end + 12.0,
            0.0,
            HEADER,
            MUTED,
        );
        let close = kit::close_rect(WIDTH, HEADER);
        let lit = h.add(close, Hit::Ask(ChessAsk::Close));
        kit::close(rc, &self.theme, close, lit);
        // The board.
        let side = SQUARE * 8.0;
        let squares = self.screen.squares.clone();
        for (i, q) in squares.iter().enumerate() {
            let r = Self::square_rect(i);
            let lit = h.add(r, Hit::Ask(ChessAsk::Square(i)));
            self.draw_square(
                rc,
                r,
                q,
                lit && (q.target || q.capture || q.piece.is_some()),
            );
        }
        rc.stroke_rounded_rect(
            BOARD_X - 0.5,
            BOARD_Y - 0.5,
            side + 1.0,
            side + 1.0,
            CornerRadii::all(3.0),
            bronze(0.6),
            1.0,
        );
        for i in 0..8 {
            let rank = self.screen.ranks[i].to_string();
            put_mid(
                rc,
                f.small.as_ref(),
                &rank,
                20.0,
                BOARD_Y + i as f32 * SQUARE,
                SQUARE,
                MUTED,
            );
            let file = self.screen.files[i].to_string();
            if let Some(sf) = &f.small {
                let w = text::measure(sf, &file, 0.0);
                text::draw(
                    rc,
                    sf,
                    &file,
                    (BOARD_X + i as f32 * SQUARE + (SQUARE - w) * 0.5).round(),
                    BOARD_Y + side + 4.0,
                    TextStyle::plain(MUTED),
                );
            }
        }
        self.draw_side(rc, &mut h);
        if self.resigning {
            self.draw_card(rc, &mut h);
        }
        self.hits = h;
    }
}

/// The Chess window's element and what it remembers.
#[derive(Debug)]
pub struct ChessUi {
    /// The window.
    pub frame: ElementId,
    /// The square the player picked a piece up from.
    picked: Option<ac_core::chess::ChessSquare>,
    /// The game's revision and the pick last shown.
    shown: Option<(u64, Option<ac_core::chess::ChessSquare>)>,
}

fn chess_command(ui: &mut UiRoot, command: ChessCommand) {
    emit(
        ui,
        PanelOutput::Session(SessionCommand::Gameplay(GameplayCommand::Chess(command))),
    );
}

impl WindowUi {
    /// The Chess window, hidden, in place of retail's Game Center.
    pub(in crate::ui) fn mount_modern_chess(&mut self) {
        let theme = Horizon::BRONZE;
        let fonts = ChessFonts {
            title: self.modern_font(Weight::SemiBold, 15.0),
            strong: self.modern_font(Weight::SemiBold, 12.5),
            body: self.modern_font(Weight::Regular, 12.5),
            moves: self.modern_font(Weight::Regular, 12.0),
            small: self.modern_font(Weight::Regular, 11.0),
            label: self.modern_font(Weight::SemiBold, 9.5),
        };
        let mut frame_widget = ChessFrame::new(theme, fonts);
        frame_widget.pieces =
            import_infos_rooted(&self.content, MINI_GAME_LAYOUT_ID, MINI_GAME_ROOT_ID)
                .map(|info| read_piece_images(&info))
                .unwrap_or_default();
        let frame =
            match hud::mount_piece(&mut self.host.root, CHESS, (WIDTH, HEIGHT), frame_widget) {
                Ok(id) => id,
                Err(e) => {
                    self.notices.push(format!("ui: modern chess: {e}"));
                    return;
                }
            };
        let ui = &mut self.host.root;
        if let Some(e) = ui.element_mut(frame) {
            e.draggable = true;
        }
        ui.hide_window(CHESS);
        let Some(modern) = self.modern.as_mut() else {
            return;
        };
        modern.chess = Some(ChessUi {
            frame,
            picked: None,
            shown: None,
        });
        modern.replace(MINI_GAME_WINDOW);
        self.panels.stand_in(panel_id::MINI_GAME, CHESS);
    }

    /// Each frame: the window follows the game; its clicks pick pieces up,
    /// move them, offer a stalemate or resign.
    pub(in crate::ui) fn tick_modern_chess(&mut self) {
        let Some(c) = self.modern.as_ref().and_then(|m| m.chess.as_ref()) else {
            return;
        };
        let (frame, mut picked, shown) = (c.frame, c.picked, c.shown);
        let open = self.host.root.is_window_visible(CHESS);
        let view = self.facts.chess.clone().unwrap_or_default();
        let asks = self
            .host
            .root
            .widget_mut::<ChessFrame>(frame)
            .map(|f| std::mem::take(&mut f.asks))
            .unwrap_or_default();
        if !open {
            if let Some(f) = self.host.root.widget_mut::<ChessFrame>(frame) {
                f.resigning = false;
            }
            if let Some(c) = self.modern.as_mut().and_then(|m| m.chess.as_mut()) {
                c.picked = None;
            }
            return;
        }
        let squares: Vec<_> = self
            .host
            .root
            .widget::<ChessFrame>(frame)
            .map(|f| f.screen.squares.iter().map(|q| q.square).collect())
            .unwrap_or_default();
        for ask in asks {
            match ask {
                ChessAsk::Square(i) => {
                    let Some(&square) = squares.get(i) else {
                        continue;
                    };
                    match picked {
                        Some(from) if from == square => picked = None,
                        _ if can_pick(&view, square) => picked = Some(square),
                        Some(from) => {
                            chess_command(
                                &mut self.host.root,
                                ChessCommand::Move { from, to: square },
                            );
                            picked = None;
                        }
                        None => {}
                    }
                }
                ChessAsk::Drop => picked = None,
                ChessAsk::Stalemate => {
                    chess_command(&mut self.host.root, ChessCommand::ToggleStalemate);
                }
                ChessAsk::Resign => {
                    chess_command(&mut self.host.root, ChessCommand::Resign);
                    picked = None;
                }
                ChessAsk::Close => {
                    self.panels
                        .set_visibility(&mut self.host.root, panel_id::MINI_GAME, false);
                }
            }
        }
        let picked = picked.filter(|s| can_pick(&view, *s));
        let key = (view.revision, picked);
        if shown != Some(key) {
            let screen = chess_screen(&view, picked);
            let names = crate::ui::facts::composed_names(&self.status.names);
            let name = self
                .facts
                .objects
                .get(view.board_guid)
                .map(names)
                .unwrap_or_default();
            if let Some(f) = self.host.root.widget_mut::<ChessFrame>(frame) {
                f.screen = screen;
                if !name.is_empty() {
                    f.board_name = name;
                }
            }
        }
        if let Some(c) = self.modern.as_mut().and_then(|m| m.chess.as_mut()) {
            c.picked = picked;
            c.shown = Some(key);
        }
    }
}
