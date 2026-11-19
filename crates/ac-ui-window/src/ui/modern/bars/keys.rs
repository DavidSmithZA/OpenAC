//! A slot's keys: how one is saved ("Shift+Number1"), how it is named in
//! words ("Shift+1") and how it is drawn in a slot's corner (a small mark
//! for each modifier, then the key: the marks are drawn as lines, since the
//! interface's font has no modifier symbols baked).

use ac_ui::draw::{Color, RenderContext};
use ac_ui::input::KeyChord;
use ac_ui::input::chord::{InputKey, key_from_name, key_name};
use ac_ui::modern::text::{self, TextStyle};
use ac_ui::{Key, Modifiers, UiFont};

/// The modifiers in the order they are written.
const MODIFIERS: [(Modifiers, &str); 4] = [
    (Modifiers::CTRL, "Ctrl"),
    (Modifiers::ALT, "Alt"),
    (Modifiers::SHIFT, "Shift"),
    (Modifiers::WIN, "Win"),
];

/// The keyboard chord of `key` with `modifiers`, or none for a key that
/// cannot be a slot's (a modifier alone, Escape, an unknown key).
#[must_use]
pub fn slot_chord(key: Key, modifiers: Modifiers) -> Option<KeyChord> {
    let modifier_key = matches!(
        key,
        Key::ShiftLeft
            | Key::ShiftRight
            | Key::ControlLeft
            | Key::ControlRight
            | Key::AltLeft
            | Key::AltRight
            | Key::SuperLeft
            | Key::SuperRight
    );
    if modifier_key || matches!(key, Key::Escape | Key::Unknown) {
        return None;
    }
    Some(KeyChord::key(key, modifiers))
}

/// The keyboard key and modifiers of `chord`, when it is a keyboard chord.
#[must_use]
pub fn keyboard(chord: KeyChord) -> Option<(Key, Modifiers)> {
    match chord.key {
        InputKey::Key(key) if chord.device == 0 => Some((key, chord.modifiers)),
        _ => None,
    }
}

/// How `chord` is saved: the held modifiers' names and the key's name,
/// joined by `+` ("Shift+Number1").
#[must_use]
pub fn save(chord: KeyChord) -> String {
    let Some((key, modifiers)) = keyboard(chord) else {
        return String::new();
    };
    let mut parts: Vec<&str> = MODIFIERS
        .iter()
        .filter(|(m, _)| modifiers.contains(*m))
        .map(|(_, name)| *name)
        .collect();
    parts.push(key_name(key));
    parts.join("+")
}

/// The chord a saved name names; none for an empty or unknown one.
#[must_use]
pub fn load(saved: &str) -> Option<KeyChord> {
    let mut parts: Vec<&str> = saved.split('+').map(str::trim).collect();
    let key = key_from_name(parts.pop()?)?;
    let mut modifiers = Modifiers::NONE;
    for part in parts {
        let (m, _) = MODIFIERS.iter().find(|(_, name)| *name == part)?;
        modifiers = modifiers | *m;
    }
    slot_chord(key, modifiers)
}

/// The key's own short name, as a slot's corner shows it ("1", "=", "F1",
/// "Q", "Num1").
#[must_use]
pub fn key_word(key: Key) -> String {
    let name = key_name(key);
    if let Some(digit) = name.strip_prefix("Number") {
        return digit.to_owned();
    }
    if let Some(rest) = name.strip_prefix("Keypad") {
        return match rest {
            "Decimal" => "Num.".to_owned(),
            "Divide" => "Num/".to_owned(),
            "Multiply" => "Num*".to_owned(),
            "Subtract" => "Num-".to_owned(),
            "Add" => "Num+".to_owned(),
            "Enter" => "NumEnter".to_owned(),
            "Equal" => "Num=".to_owned(),
            digit => format!("Num{digit}"),
        };
    }
    match key {
        Key::Minus => "-".to_owned(),
        Key::Equal => "=".to_owned(),
        Key::Apostrophe => "'".to_owned(),
        Key::Comma => ",".to_owned(),
        Key::Period => ".".to_owned(),
        Key::Slash => "/".to_owned(),
        Key::Semicolon => ";".to_owned(),
        Key::LeftBracket => "[".to_owned(),
        Key::RightBracket => "]".to_owned(),
        Key::BackSlash => "\\".to_owned(),
        Key::GraveAccent => "`".to_owned(),
        Key::Space => "Space".to_owned(),
        Key::Backspace => "Bksp".to_owned(),
        Key::Delete => "Del".to_owned(),
        Key::Insert => "Ins".to_owned(),
        Key::PageUp => "PgUp".to_owned(),
        Key::PageDown => "PgDn".to_owned(),
        _ => name.to_owned(),
    }
}

/// The name of the "Win" modifier where the game runs.
fn win_word() -> &'static str {
    if cfg!(target_os = "macos") {
        "Cmd"
    } else {
        "Win"
    }
}

/// `chord` in words ("Shift+1", "Ctrl+F1"), for hover labels and the key
/// pop-up.
#[must_use]
pub fn words(chord: KeyChord) -> String {
    let Some((key, modifiers)) = keyboard(chord) else {
        return String::new();
    };
    let mut parts: Vec<String> = MODIFIERS
        .iter()
        .filter(|(m, _)| modifiers.contains(*m))
        .map(|(m, name)| {
            if *m == Modifiers::WIN {
                win_word().to_owned()
            } else {
                (*name).to_owned()
            }
        })
        .collect();
    parts.push(key_word(key));
    parts.join("+")
}

/// The width a modifier's mark takes in a corner of `font`'s size, with
/// its gap.
fn mark_width(font: &UiFont) -> f32 {
    (font.line_height * 0.62).round() + 1.0
}

/// The width of `chord` drawn in a corner by [`draw`].
#[must_use]
pub fn drawn_width(font: &UiFont, chord: KeyChord) -> f32 {
    let Some((key, modifiers)) = keyboard(chord) else {
        return 0.0;
    };
    let marks = MODIFIERS
        .iter()
        .filter(|(m, _)| modifiers.contains(*m))
        .count();
    marks as f32 * mark_width(font) + text::measure(font, &key_word(key), 0.0)
}

/// Draws `chord` with its top left at (`x`, `y`): a mark for each modifier
/// (Shift an outlined arrow, Ctrl a caret, Alt the option sign, Win or Cmd
/// a small square), then the key.
pub fn draw(
    rc: &mut RenderContext<'_>,
    font: &UiFont,
    chord: KeyChord,
    x: f32,
    y: f32,
    color: Color,
    shadow: Option<Color>,
) {
    let Some((key, modifiers)) = keyboard(chord) else {
        return;
    };
    let size = (font.line_height * 0.62).round();
    let top = y + (font.line_height - size) * 0.5;
    let mut left = x;
    for (m, _) in MODIFIERS {
        if !modifiers.contains(m) {
            continue;
        }
        if let Some(s) = shadow {
            mark(rc, m, left, top + 1.0, size, s);
        }
        mark(rc, m, left, top, size, color);
        left += mark_width(font);
    }
    let style = TextStyle::plain(color);
    let style = match shadow {
        Some(s) => style.shadowed(s),
        None => style,
    };
    text::draw(rc, font, &key_word(key), left, y, style);
}

/// One modifier's mark in a `size` box at (`x`, `y`).
fn mark(rc: &mut RenderContext<'_>, modifier: Modifiers, x: f32, y: f32, size: f32, color: Color) {
    let w = 1.1;
    let p = |u: f32, v: f32| (x + u * size, y + v * size);
    let line = |rc: &mut RenderContext<'_>, a: (f32, f32), b: (f32, f32)| {
        rc.smooth_line(a.0, a.1, b.0, b.1, color, w);
    };
    if modifier == Modifiers::SHIFT {
        // An outlined arrow pointing up.
        let run = [
            p(0.5, 0.0),
            p(1.0, 0.5),
            p(0.75, 0.5),
            p(0.75, 1.0),
            p(0.25, 1.0),
            p(0.25, 0.5),
            p(0.0, 0.5),
            p(0.5, 0.0),
        ];
        for pair in run.windows(2) {
            line(rc, pair[0], pair[1]);
        }
    } else if modifier == Modifiers::CTRL {
        // A caret.
        line(rc, p(0.05, 0.75), p(0.5, 0.2));
        line(rc, p(0.5, 0.2), p(0.95, 0.75));
    } else if modifier == Modifiers::ALT {
        // The option sign.
        line(rc, p(0.0, 0.25), p(0.35, 0.25));
        line(rc, p(0.35, 0.25), p(0.7, 0.85));
        line(rc, p(0.7, 0.85), p(1.0, 0.85));
        line(rc, p(0.6, 0.25), p(1.0, 0.25));
    } else {
        // Win or Cmd: a small square.
        let (a, b) = (p(0.1, 0.15), p(0.9, 0.95));
        rc.stroke_rounded_rect(
            a.0,
            a.1,
            b.0 - a.0,
            b.1 - a.1,
            ac_ui::shapes::CornerRadii::all(1.5),
            color,
            w,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_key_is_saved_and_read_back() {
        let chord = KeyChord::key(Key::Number1, Modifiers::SHIFT | Modifiers::CTRL);
        assert_eq!(save(chord), "Ctrl+Shift+Number1");
        assert_eq!(load("Ctrl+Shift+Number1"), Some(chord));
        assert_eq!(load("F1"), Some(KeyChord::key(Key::F1, Modifiers::NONE)));
        assert_eq!(load(""), None);
        assert_eq!(load("Hyper+F1"), None);
        assert_eq!(load("ShiftLeft"), None);
    }

    #[test]
    fn keys_are_named_short() {
        assert_eq!(key_word(Key::Number0), "0");
        assert_eq!(key_word(Key::Equal), "=");
        assert_eq!(key_word(Key::Keypad7), "Num7");
        assert_eq!(key_word(Key::F12), "F12");
        assert_eq!(key_word(Key::Q), "Q");
        assert_eq!(
            words(KeyChord::key(Key::Minus, Modifiers::SHIFT)),
            "Shift+-"
        );
    }

    #[test]
    fn a_modifier_alone_is_no_slot_key() {
        assert!(slot_chord(Key::ShiftLeft, Modifiers::SHIFT).is_none());
        assert!(slot_chord(Key::Escape, Modifiers::NONE).is_none());
        assert!(slot_chord(Key::Number5, Modifiers::ALT).is_some());
    }
}
