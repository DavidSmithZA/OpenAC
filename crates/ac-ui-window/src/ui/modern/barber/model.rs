//! What the Barber window shows, worked out from the barber as the
//! session sends it: the parts the heritage offers, the picked part's
//! styles, colours and shade, the heritage's one option, and a line of
//! what has changed since the barber opened.

use ac_core::chargen::{ChargenGenderOptions, ChargenSwatchRgb};
use ac_runtime::gameplay::BarberView;
use ac_runtime::session::creation::{CreationAppearance, UNSET};
use ac_ui::draw::Color;
use ac_ui::panels::barber_rules::{
    PARTS, color_count, marked_swatch, shade_slot, style_count, style_slot,
};
use ac_ui::pregame::chargen::appearance::Part;

/// A part's tab.
#[derive(Debug, Clone, PartialEq)]
pub struct PartTab {
    /// The part.
    pub part: Part,
    /// "Hair", or the heritage's own word.
    pub label: String,
    /// "Style 4", "Shade 40%".
    pub sub: String,
    /// Picked.
    pub on: bool,
}

/// A colour to pick.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Swatch {
    /// Its colour (grey while the dats have not given it).
    pub color: Color,
    /// The look has it.
    pub on: bool,
}

/// The window's whole picture.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BarberScreen {
    /// "Female Aluvian".
    pub subtitle: String,
    /// The parts.
    pub parts: Vec<PartTab>,
    /// The picked part.
    pub part: Option<Part>,
    /// The picked style (from 1) and how many there are, when the part
    /// has styles.
    pub style: Option<(usize, usize)>,
    /// The colours, when the part has its own.
    pub swatches: Vec<Swatch>,
    /// The shade (0 to 1) and the colours along its track, when the part
    /// has one.
    pub shade: Option<(f32, Vec<Color>)>,
    /// A note under the part's choices (a part whose colour follows the
    /// skin, the skin that other parts follow).
    pub note: Option<&'static str>,
    /// The heritage's one option: its words and whether it is on.
    pub toggle: Option<(String, bool)>,
    /// What changed since the barber opened.
    pub changed: String,
}

/// The look and option when the barber opened, for "Hold for before"
/// and the changed line.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Before {
    /// The look.
    pub appearance: CreationAppearance,
    /// The heritage's option was on.
    pub toggle_on: bool,
    /// The body the preview showed.
    pub preview_setup: u32,
}

impl Before {
    /// The barber as it opened.
    #[must_use]
    pub fn of(view: &BarberView) -> Self {
        Self {
            appearance: view.appearance,
            toggle_on: view.toggle_on,
            preview_setup: view.preview_setup,
        }
    }
}

/// What the window reads besides the barber.
#[derive(Debug, Clone, Default)]
pub struct BarberFacts<'a> {
    /// The character's gender's choices.
    pub gender: Option<&'a ChargenGenderOptions>,
    /// The heritage's name ("Aluvian").
    pub heritage_name: String,
    /// The part words a heritage without nose and mouth uses (hair, eyes,
    /// skin).
    pub captions: Option<[String; 3]>,
    /// The option's words.
    pub toggle_words: Option<String>,
    /// The picked part's colours.
    pub colors: [Option<ChargenSwatchRgb>; 9],
    /// The colours along the picked part's shade.
    pub shade_stops: Vec<ChargenSwatchRgb>,
}

/// A heritage without nose and mouth (its eyes have no styles either).
#[must_use]
pub fn three_parts(heritage: u32) -> bool {
    ac_ui::pregame::chargen::appearance::clothes_hidden(heritage)
}

/// The parts `heritage` offers, in order.
#[must_use]
pub fn parts_of(heritage: u32) -> Vec<Part> {
    if three_parts(heritage) {
        vec![Part::Hair, Part::Eyes, Part::Skin]
    } else {
        PARTS.to_vec()
    }
}

/// A colour from the dats' bytes.
#[must_use]
pub fn rgb(c: ChargenSwatchRgb) -> Color {
    Color::rgba(
        f32::from(c.r) / 255.0,
        f32::from(c.g) / 255.0,
        f32::from(c.b) / 255.0,
        1.0,
    )
}

/// A part's own word.
fn part_word(part: Part) -> &'static str {
    match part {
        Part::Hair => "Hair",
        Part::Eyes => "Eyes",
        Part::Nose => "Nose",
        Part::Mouth => "Mouth",
        _ => "Skin",
    }
}

/// The part's shade (hair's own, the skin's for the rest), 0 to 1.
fn shade_of(part: Part, a: &CreationAppearance) -> f64 {
    let slot = shade_slot(part).unwrap_or(ac_runtime::session::creation::ShadeSlot::Skin);
    ac_ui::pregame::chargen::appearance::shade_of(slot, a)
}

/// Whether the part has styles to step through.
fn has_styles(part: Part, heritage: u32) -> bool {
    style_slot(part).is_some() && !(three_parts(heritage) && part == Part::Eyes)
}

/// The screen for `view` with `part` picked.
#[must_use]
pub fn barber_screen(
    view: &BarberView,
    part: Part,
    before: &Before,
    facts: &BarberFacts<'_>,
) -> BarberScreen {
    let a = &view.appearance;
    let three = three_parts(view.heritage);
    let label = |p: Part| -> String {
        match (&facts.captions, p) {
            (Some(c), Part::Hair) => c[0].clone(),
            (Some(c), Part::Eyes) => c[1].clone(),
            (Some(c), Part::Skin) => c[2].clone(),
            _ => part_word(p).to_owned(),
        }
    };
    let style_number = |p: Part| {
        style_slot(p)
            .map(|s| a.get(s))
            .filter(|v| *v != UNSET)
            .map(|v| v as usize + 1)
    };
    let parts = parts_of(view.heritage)
        .into_iter()
        .map(|p| {
            let sub = if p == Part::Skin {
                format!("Shade {}%", (a.skin_shade * 100.0).round())
            } else if has_styles(p, view.heritage) {
                style_number(p).map_or_else(|| "None".to_owned(), |n| format!("Style {n}"))
            } else {
                "Colour".to_owned()
            };
            PartTab {
                part: p,
                label: label(p),
                sub,
                on: p == part,
            }
        })
        .collect();
    let style = facts
        .gender
        .filter(|_| has_styles(part, view.heritage))
        .map(|g| (style_number(part).unwrap_or(0), style_count(part, g)))
        .filter(|(_, count)| *count > 0);
    let own_colors = matches!(part, Part::Hair | Part::Eyes);
    let count = if own_colors {
        color_count(part, facts.gender)
    } else {
        0
    };
    let marked = marked_swatch(part, view);
    let swatches = (0..count.min(9))
        .map(|i| Swatch {
            color: facts.colors[i].map_or(Color::rgba(0.5, 0.5, 0.5, 1.0), rgb),
            on: marked == Some(i),
        })
        .collect();
    let shade = (part != Part::Eyes && shade_slot(part).is_some()).then(|| {
        (
            shade_of(part, a).clamp(0.0, 1.0) as f32,
            facts.shade_stops.iter().copied().map(rgb).collect(),
        )
    });
    let toggle = view.toggle.map(|_| {
        (
            facts
                .toggle_words
                .clone()
                .unwrap_or_else(|| "Heritage option".to_owned()),
            view.toggle_on,
        )
    });
    BarberScreen {
        subtitle: subtitle(facts.gender, &facts.heritage_name),
        parts,
        part: Some(part),
        style,
        swatches,
        shade,
        note: match part {
            Part::Nose | Part::Mouth if !three => {
                Some("This part has a style only; its colour follows your skin.")
            }
            Part::Skin if !three => Some("Your nose and mouth take this shade too."),
            _ => None,
        },
        toggle,
        changed: changed_line(view, before, facts),
    }
}

/// "Female Aluvian".
fn subtitle(gender: Option<&ChargenGenderOptions>, heritage: &str) -> String {
    let gender = gender.map(|g| g.name.trim()).unwrap_or_default();
    [gender, heritage.trim()]
        .into_iter()
        .filter(|w| !w.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

/// "Changed: hair style, hair colour", or "Nothing changed yet."
#[must_use]
pub fn changed_line(view: &BarberView, before: &Before, facts: &BarberFacts<'_>) -> String {
    let (a, b) = (&view.appearance, &before.appearance);
    let three = three_parts(view.heritage);
    let word = |p: Part| match (&facts.captions, p) {
        (Some(c), Part::Hair) => c[0].to_lowercase(),
        (Some(c), Part::Eyes) => c[1].to_lowercase(),
        (Some(c), Part::Skin) => c[2].to_lowercase(),
        _ => part_word(p).to_lowercase(),
    };
    let shade_moved = |x: f64, y: f64| (x - y).abs() > 0.004;
    let mut changes: Vec<String> = Vec::new();
    if a.hair_style != b.hair_style {
        changes.push(format!("{} style", word(Part::Hair)));
    }
    if a.hair_color != b.hair_color {
        changes.push(format!("{} colour", word(Part::Hair)));
    }
    if shade_moved(a.hair_shade, b.hair_shade) {
        changes.push(format!("{} shade", word(Part::Hair)));
    }
    if a.eyes_strip != b.eyes_strip && !three {
        changes.push(word(Part::Eyes));
    }
    if a.eye_color != b.eye_color {
        changes.push(format!("{} colour", word(Part::Eyes)));
    }
    if !three && a.nose_strip != b.nose_strip {
        changes.push("nose".to_owned());
    }
    if !three && a.mouth_strip != b.mouth_strip {
        changes.push("mouth".to_owned());
    }
    if shade_moved(a.skin_shade, b.skin_shade) {
        changes.push(word(Part::Skin));
    }
    if view.toggle.is_some() && view.toggle_on != before.toggle_on {
        changes.push(
            facts
                .toggle_words
                .as_deref()
                .map_or_else(|| "heritage option".to_owned(), str::to_lowercase),
        );
    }
    if changes.is_empty() {
        "Nothing changed yet.".to_owned()
    } else {
        format!("Changed: {}", changes.join(", "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ac_core::chargen::{ChargenHairStyle, ChargenHeritageGroup as H};

    fn gender() -> ChargenGenderOptions {
        ChargenGenderOptions {
            name: "Female".to_owned(),
            hair_styles: vec![ChargenHairStyle::default(); 12],
            hair_colors: vec![11, 12, 13],
            eye_colors: vec![21, 22],
            eye_strips: vec![Default::default(); 8],
            nose_strips: vec![Default::default(); 6],
            mouth_strips: vec![Default::default(); 6],
            ..ChargenGenderOptions::default()
        }
    }

    fn view() -> BarberView {
        let mut v = BarberView {
            open: true,
            heritage: 1,
            ..BarberView::default()
        };
        v.appearance.hair_style = 3;
        v.appearance.hair_color = 1;
        v.appearance.hair_shade = 0.5;
        v.appearance.skin_shade = 0.4;
        v
    }

    #[test]
    fn the_hair_shows_its_styles_colours_and_shade() {
        let g = gender();
        let v = view();
        let facts = BarberFacts {
            gender: Some(&g),
            heritage_name: "Aluvian".to_owned(),
            ..BarberFacts::default()
        };
        let s = barber_screen(&v, Part::Hair, &Before::of(&v), &facts);
        assert_eq!(s.subtitle, "Female Aluvian");
        assert_eq!(s.parts.len(), 5);
        assert_eq!(s.parts[0].sub, "Style 4");
        assert_eq!(s.parts[4].sub, "Shade 40%");
        assert_eq!(s.style, Some((4, 12)));
        assert_eq!(s.swatches.len(), 3);
        assert!(s.swatches[1].on);
        assert_eq!(s.shade.as_ref().map(|x| x.0), Some(0.5));
        assert_eq!(s.changed, "Nothing changed yet.");
        let s = barber_screen(&v, Part::Nose, &Before::of(&v), &facts);
        assert!(s.note.is_some() && s.swatches.is_empty());
        assert!(s.shade.is_some(), "the nose moves the skin's shade");
        let s = barber_screen(&v, Part::Eyes, &Before::of(&v), &facts);
        assert!(s.shade.is_none());
    }

    #[test]
    fn the_changed_line_names_what_moved_since_opening() {
        let g = gender();
        let before = Before::of(&view());
        let mut v = view();
        v.appearance.hair_style = 5;
        v.appearance.hair_color = 2;
        v.appearance.skin_shade = 0.6;
        let facts = BarberFacts {
            gender: Some(&g),
            ..BarberFacts::default()
        };
        assert_eq!(
            changed_line(&v, &before, &facts),
            "Changed: hair style, hair colour, skin"
        );
    }

    #[test]
    fn a_heritage_without_nose_and_mouth_has_three_parts_in_its_own_words() {
        let g = gender();
        let mut v = view();
        v.heritage = H::GEARKNIGHT.0;
        let facts = BarberFacts {
            gender: Some(&g),
            captions: Some(["Plating".to_owned(), "Optics".to_owned(), "Hull".to_owned()]),
            ..BarberFacts::default()
        };
        let s = barber_screen(&v, Part::Eyes, &Before::of(&v), &facts);
        let labels: Vec<&str> = s.parts.iter().map(|p| p.label.as_str()).collect();
        assert_eq!(labels, ["Plating", "Optics", "Hull"]);
        assert_eq!(s.style, None, "no eye styles");
    }
}
