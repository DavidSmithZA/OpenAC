//! Quick move: with its key held, a modern piece is dragged (snapping into
//! line unless Shift is held) or right-clicked for its quick edits, whatever
//! the UI lock says; without the key, or for anything else, nothing changes.

mod common;

use ac_ui::MouseButton;
use ac_ui::event::Modifiers;
use ac_ui::root::{QUICK_MOVE_SNAP, UiOutput, UiRoot};
use ac_ui::tree::ElementId;
use common::*;

fn piece(ui: &mut UiRoot, left: f32, top: f32) -> ElementId {
    let mut e = at(left, top, 100.0, 40.0);
    e.quick_move = true;
    let root = ui.root();
    let id = ClickRecorder {
        handles_click: true,
        ..ClickRecorder::default()
    };
    ui.add(root, e, id)
}

fn place(ui: &UiRoot, id: ElementId) -> (f32, f32) {
    let e = ui.element(id).unwrap();
    (e.left, e.top)
}

fn drag(ui: &mut UiRoot, from: (i32, i32), to: (i32, i32)) {
    ui.on_mouse_down(MouseButton::Left, from.0, from.1, 0);
    ui.on_mouse_move(to.0, to.1);
    ui.on_mouse_up(MouseButton::Left, to.0, to.1, 0);
}

fn locked_root() -> UiRoot {
    let mut ui = root(800.0, 600.0);
    ui.set_ui_locked(true);
    ui.set_quick_move_key(Some(Modifiers::CTRL));
    ui
}

#[test]
fn with_the_key_held_a_piece_moves_though_the_interface_is_locked() {
    let mut ui = locked_root();
    let id = piece(&mut ui, 200.0, 200.0);
    ui.set_modifiers(Modifiers::CTRL);
    assert!(ui.quick_move_held());
    drag(&mut ui, (210, 210), (310, 250));
    assert_eq!(place(&ui, id), (300.0, 240.0));
    // The press was the move's, not a click on the piece.
    assert_eq!(ui.widget::<ClickRecorder>(id).unwrap().clicks, 0);
}

#[test]
fn without_the_key_a_locked_piece_stays_and_is_clicked() {
    let mut ui = locked_root();
    let id = piece(&mut ui, 200.0, 200.0);
    drag(&mut ui, (210, 210), (210, 210));
    assert_eq!(place(&ui, id), (200.0, 200.0));
    assert_eq!(ui.widget::<ClickRecorder>(id).unwrap().clicks, 1);
}

#[test]
fn quick_move_off_or_a_piece_not_marked_does_nothing() {
    let mut ui = locked_root();
    let root = ui.root();
    let plain = panel(&mut ui, root, window(400.0, 400.0, 100.0, 40.0));
    ui.set_modifiers(Modifiers::CTRL);
    drag(&mut ui, (410, 410), (450, 450));
    assert_eq!(place(&ui, plain), (400.0, 400.0));
    let id = piece(&mut ui, 200.0, 200.0);
    ui.set_quick_move_key(None);
    assert!(!ui.quick_move_held());
    drag(&mut ui, (210, 210), (260, 260));
    assert_eq!(place(&ui, id), (200.0, 200.0));
}

#[test]
fn a_piece_snaps_to_the_screen_edges_and_shift_places_it_freely() {
    let mut ui = locked_root();
    let id = piece(&mut ui, 200.0, 200.0);
    ui.set_modifiers(Modifiers::CTRL);
    // Within the snap of the left edge and of the bottom.
    let near = (QUICK_MOVE_SNAP - 2.0) as i32;
    drag(&mut ui, (200, 200), (near, 600 - 40 - near));
    assert_eq!(place(&ui, id), (0.0, 560.0));
    ui.set_modifiers(Modifiers::CTRL | Modifiers::SHIFT);
    drag(&mut ui, (10, 570), (16, 300));
    assert_eq!(place(&ui, id), (6.0, 290.0));
}

#[test]
fn a_piece_snaps_to_another_pieces_edges() {
    let mut ui = locked_root();
    let fixed = piece(&mut ui, 300.0, 100.0);
    let id = piece(&mut ui, 0.0, 300.0);
    ui.set_modifiers(Modifiers::CTRL);
    // Just right of the other piece, a little low: it lines up beside it.
    drag(&mut ui, (10, 310), (10 + 403, 310 - 197));
    assert_eq!(place(&ui, id), (400.0, 100.0));
    assert_eq!(place(&ui, fixed), (300.0, 100.0));
}

#[test]
fn a_right_click_with_the_key_asks_for_the_quick_edits_and_nothing_else() {
    let mut ui = locked_root();
    let id = piece(&mut ui, 200.0, 200.0);
    ui.set_modifiers(Modifiers::CTRL);
    ui.drain_output();
    ui.on_mouse_down(MouseButton::Right, 220, 215, 0);
    ui.on_mouse_up(MouseButton::Right, 220, 215, 0);
    let out = ui.drain_output();
    assert!(
        out.iter().any(|o| matches!(
            o,
            UiOutput::QuickMoveMenu { piece, x: 220, y: 215 } if *piece == id
        )),
        "{out:?}"
    );
    assert!(
        !out.iter().any(|o| matches!(o, UiOutput::WorldMouse { .. })),
        "{out:?}"
    );
}

#[test]
fn a_handle_of_the_pieces_own_keeps_its_press_while_the_key_is_held() {
    let mut ui = locked_root();
    let id = piece(&mut ui, 200.0, 200.0);
    let mut h = at(80.0, 10.0, 20.0, 20.0);
    h.quick_move_passes = true;
    let handle = ui.add(
        id,
        h,
        ClickRecorder {
            handles_click: true,
            ..ClickRecorder::default()
        },
    );
    ui.set_modifiers(Modifiers::CTRL);
    drag(&mut ui, (285, 215), (285, 215));
    assert_eq!(place(&ui, id), (200.0, 200.0), "the piece stays");
    assert_eq!(ui.widget::<ClickRecorder>(handle).unwrap().clicks, 1);
}
