//! What the Chess window shows, worked out from the game as the session
//! sends it: the board from the player's side, the piece picked up and
//! where it can go, the last move, the two sides, the status line and
//! the moves made.

use ac_core::chess::{ChessPiece, ChessPieceType, ChessSquare, ChessTeam, SIZE, square_name};
use ac_runtime::gameplay::{ChessStage, ChessView};

/// One of the 64 squares as the window shows it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SquareView {
    /// The board square under it.
    pub square: ChessSquare,
    /// A light square.
    pub light: bool,
    /// The piece on it.
    pub piece: Option<ChessPiece>,
    /// The piece picked up stands here.
    pub picked: bool,
    /// The picked piece can move here (a dot).
    pub target: bool,
    /// The picked piece can take what is here (a ring).
    pub capture: bool,
    /// The last move left or reached it.
    pub last: bool,
}

/// One side of the game.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SideView {
    /// "You", "Your opponent", or the colour when only watching.
    pub name: String,
    /// White's side.
    pub white: bool,
    /// "YOUR MOVE", "THEIR MOVE", or nothing.
    pub turn: &'static str,
    /// It is this side's move.
    pub moving: bool,
}

/// A numbered line of the moves list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MoveLine {
    /// The move's number.
    pub number: usize,
    /// White's move.
    pub white: String,
    /// Black's move, once made.
    pub black: String,
    /// Which of the two is the last move made (0: neither, 1: White's,
    /// 2: Black's).
    pub last: u8,
}

/// The window's whole picture.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ChessScreen {
    /// The squares, left to right and top to bottom.
    pub squares: Vec<SquareView>,
    /// The rank labels down the left, top first.
    pub ranks: [char; 8],
    /// The file labels along the bottom, left first.
    pub files: [char; 8],
    /// The side across the board first, the player's own last.
    pub sides: Vec<SideView>,
    /// The status line.
    pub status: String,
    /// The moves list.
    pub moves: Vec<MoveLine>,
    /// Offer stalemate and Resign can be used.
    pub playing: bool,
    /// The player has offered a stalemate.
    pub offering: bool,
}

/// The board square under window square `index` (left to right, top to
/// bottom): White sees its own back rank at the bottom, anyone else sees
/// the board turned round.
#[must_use]
pub fn square_at(index: usize, team: Option<ChessTeam>) -> ChessSquare {
    let index = i32::try_from(index).unwrap_or(0);
    let (column, row) = (index % SIZE, index / SIZE);
    if team == Some(ChessTeam::White) {
        ChessSquare::new(column, SIZE - 1 - row)
    } else {
        ChessSquare::new(SIZE - 1 - column, row)
    }
}

/// A piece's name, for the status line.
#[must_use]
pub fn piece_name(kind: ChessPieceType) -> &'static str {
    match kind {
        ChessPieceType::Pawn => "Pawn",
        ChessPieceType::Bishop => "Bishop",
        ChessPieceType::Knight => "Knight",
        ChessPieceType::Rook => "Rook",
        ChessPieceType::Queen => "Queen",
        ChessPieceType::King => "King",
    }
}

/// Whether the player may pick up the piece on `square` now.
#[must_use]
pub fn can_pick(view: &ChessView, square: ChessSquare) -> bool {
    view.stage == ChessStage::MyTurn
        && view
            .team
            .is_some_and(|t| view.board.get(square).is_some_and(|p| p.team == t))
}

/// The screen for `view` with the piece on `picked` picked up.
#[must_use]
pub fn chess_screen(view: &ChessView, picked: Option<ChessSquare>) -> ChessScreen {
    let picked = picked.filter(|s| can_pick(view, *s));
    let targets = picked.map(|s| view.board.targets(s)).unwrap_or_default();
    let last = view.moves.last().map(|m| (m.from, m.to));
    let squares = (0..64)
        .map(|i| {
            let square = square_at(i, view.team);
            let piece = view.board.get(square);
            let target = targets.contains(&square);
            SquareView {
                square,
                light: (square.x + square.y) % 2 == 1,
                piece,
                picked: picked == Some(square),
                target: target && piece.is_none(),
                capture: target && piece.is_some(),
                last: last.is_some_and(|(f, t)| f == square || t == square),
            }
        })
        .collect::<Vec<_>>();
    let mut ranks = ['1'; 8];
    let mut files = ['a'; 8];
    for i in 0..8 {
        let top = square_at(i * 8, view.team);
        ranks[i] = char::from(b'1' + u8::try_from(top.y).unwrap_or(0));
        let bottom = square_at(56 + i, view.team);
        files[i] = char::from(b'a' + u8::try_from(bottom.x).unwrap_or(0));
    }
    let playing = view.board_guid != 0;
    let to_move = view.board.turn();
    let moving_side = |white: bool| {
        playing
            && matches!(
                view.stage,
                ChessStage::MyTurn | ChessStage::MoveSent | ChessStage::OpponentsTurn
            )
            && (to_move == ChessTeam::White) == white
    };
    let sides = match view.team {
        Some(team) => {
            let mine = team == ChessTeam::White;
            let theirs = SideView {
                name: "Your opponent".to_owned(),
                white: !mine,
                turn: "",
                moving: view.stage == ChessStage::OpponentsTurn,
            };
            let own = SideView {
                name: "You".to_owned(),
                white: mine,
                turn: "",
                moving: view.stage == ChessStage::MyTurn,
            };
            vec![
                SideView {
                    turn: if theirs.moving { "THEIR MOVE" } else { "" },
                    ..theirs
                },
                SideView {
                    turn: if own.moving { "YOUR MOVE" } else { "" },
                    ..own
                },
            ]
        }
        None => [true, false]
            .into_iter()
            .map(|white| SideView {
                name: if white { "White" } else { "Black" }.to_owned(),
                white,
                turn: if moving_side(white) { "TO MOVE" } else { "" },
                moving: moving_side(white),
            })
            .collect(),
    };
    ChessScreen {
        squares,
        ranks,
        files,
        sides,
        status: status(view, picked, targets.len()),
        moves: move_lines(view),
        playing,
        offering: view.offering_stalemate,
    }
}

/// The status line.
fn status(view: &ChessView, picked: Option<ChessSquare>, targets: usize) -> String {
    let check = view.team.is_some_and(|t| view.board.is_in_check(t))
        && matches!(view.stage, ChessStage::MyTurn | ChessStage::OpponentsTurn);
    let mut words = match view.stage {
        ChessStage::NotPlaying if view.moves.is_empty() => {
            "No game. Use a board in the world to join one.".to_owned()
        }
        ChessStage::NotPlaying => {
            "The game is over. Use the board again for a new game.".to_owned()
        }
        ChessStage::Joining => "Joining the game\u{2026}".to_owned(),
        ChessStage::WaitingForStart => {
            "You have a seat. Waiting for the other player to sit down.".to_owned()
        }
        ChessStage::MyTurn => match picked.and_then(|s| view.board.get(s)) {
            Some(p) if targets == 0 => format!(
                "That {} has nowhere to go. Pick another piece.",
                piece_name(p.piece_type).to_lowercase()
            ),
            Some(p) => format!(
                "{} picked up. Dots show where it can go; a ringed piece can be taken.",
                piece_name(p.piece_type)
            ),
            None if check => "You are in check. Your move.".to_owned(),
            None => "Your move. Click one of your pieces, then a square.".to_owned(),
        },
        ChessStage::MoveSent => "Move sent. Waiting for the game to take it.".to_owned(),
        ChessStage::OpponentsTurn if view.team.is_none() => "Watching the game.".to_owned(),
        ChessStage::OpponentsTurn => "Waiting for your opponent to move.".to_owned(),
    };
    if view.offering_stalemate && view.board_guid != 0 {
        words.push_str(" You offered a stalemate.");
    }
    words
}

/// The moves made, two to a line.
#[must_use]
pub fn move_lines(view: &ChessView) -> Vec<MoveLine> {
    let mut lines: Vec<MoveLine> = Vec::new();
    let count = view.moves.len();
    for (i, m) in view.moves.iter().enumerate() {
        let is_last = i + 1 == count;
        if m.white || lines.is_empty() || !lines.last().is_some_and(|l| l.black.is_empty()) {
            lines.push(MoveLine {
                number: lines.len() + 1,
                white: if m.white {
                    m.written.clone()
                } else {
                    "\u{2026}".to_owned()
                },
                black: String::new(),
                last: 0,
            });
        }
        let line = lines.last_mut().expect("a line was just pushed");
        if m.white {
            if is_last {
                line.last = 1;
            }
        } else {
            line.black.clone_from(&m.written);
            if is_last {
                line.last = 2;
            }
        }
    }
    lines
}

/// A square's name with what stands on it, for a hover label.
#[must_use]
pub fn square_words(s: &SquareView) -> String {
    match s.piece {
        Some(p) => format!(
            "{} {} {}",
            square_name(s.square),
            if p.team == ChessTeam::White {
                "white"
            } else {
                "black"
            },
            piece_name(p.piece_type).to_lowercase()
        ),
        None => square_name(s.square),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ac_core::chess::ChessBoard;
    use ac_runtime::gameplay::ChessPlayed;

    fn sq(name: &str) -> ChessSquare {
        let b = name.as_bytes();
        ChessSquare::new(i32::from(b[0] - b'a'), i32::from(b[1] - b'1'))
    }

    fn view(team: Option<ChessTeam>, stage: ChessStage) -> ChessView {
        ChessView {
            revision: 1,
            stage,
            board_guid: 0x8000_0020,
            team,
            offering_stalemate: false,
            games_ended: 0,
            board: ChessBoard::standard(),
            moves: Vec::new(),
        }
    }

    #[test]
    fn white_sees_its_back_rank_at_the_bottom_and_black_the_board_turned() {
        let s = chess_screen(&view(Some(ChessTeam::White), ChessStage::MyTurn), None);
        assert_eq!(s.ranks[0], '8');
        assert_eq!(s.files, ['a', 'b', 'c', 'd', 'e', 'f', 'g', 'h']);
        assert_eq!(s.squares[56].square, sq("a1"));
        assert!(!s.squares[56].light, "a1 is dark");
        let s = chess_screen(
            &view(Some(ChessTeam::Black), ChessStage::OpponentsTurn),
            None,
        );
        assert_eq!(s.ranks[0], '1');
        assert_eq!(s.files[0], 'h');
        assert_eq!(s.sides[1].name, "You");
        assert!(!s.sides[1].white);
        assert_eq!(s.sides[0].turn, "THEIR MOVE");
    }

    #[test]
    fn a_picked_piece_shows_its_dots_and_rings_and_only_on_the_players_move() {
        let mut v = view(Some(ChessTeam::White), ChessStage::MyTurn);
        let s = chess_screen(&v, Some(sq("g1")));
        let at =
            |s: &ChessScreen, name: &str| *s.squares.iter().find(|q| q.square == sq(name)).unwrap();
        assert!(at(&s, "g1").picked);
        assert!(at(&s, "f3").target && at(&s, "h3").target);
        assert!(!at(&s, "e2").target);
        assert!(s.status.starts_with("Knight picked up"));
        v.board.make_move(sq("e2"), sq("e4"));
        v.board.make_move(sq("d7"), sq("d5"));
        let s = chess_screen(&v, Some(sq("e4")));
        assert!(at(&s, "d5").capture);
        v.stage = ChessStage::OpponentsTurn;
        let s = chess_screen(&v, Some(sq("e4")));
        assert!(!at(&s, "e4").picked, "not the player's move");
    }

    #[test]
    fn the_moves_go_two_to_a_line_with_the_last_marked() {
        let mut v = view(Some(ChessTeam::White), ChessStage::MyTurn);
        for (white, from, to, w) in [
            (true, "e2", "e4", "e4"),
            (false, "e7", "e5", "e5"),
            (true, "g1", "f3", "Nf3"),
        ] {
            v.moves.push(ChessPlayed {
                white,
                from: sq(from),
                to: sq(to),
                written: w.to_owned(),
            });
        }
        let lines = move_lines(&v);
        assert_eq!(lines.len(), 2);
        assert_eq!(
            (lines[0].white.as_str(), lines[0].black.as_str()),
            ("e4", "e5")
        );
        assert_eq!((lines[1].number, lines[1].last), (2, 1));
        let s = chess_screen(&v, None);
        assert_eq!(s.squares.iter().filter(|q| q.last).count(), 2);
    }
}
