//! What a barber window decides whatever it looks like: which slot of the
//! look each part's style is, how many styles and colours a part offers,
//! which colour is marked, and the style a part's pick or step asks for.

use ac_core::chargen::ChargenGenderOptions;
use ac_runtime::gameplay::BarberView;
use ac_runtime::session::creation::{AppearanceSlot, ShadeSlot, UNSET};

use crate::pregame::chargen::appearance::{Part, cycle_index};

/// The parts a barber changes, in the order a window offers them.
pub const PARTS: [Part; 5] = [Part::Hair, Part::Eyes, Part::Nose, Part::Mouth, Part::Skin];

/// A part's style slot (skin has none).
#[must_use]
pub fn style_slot(part: Part) -> Option<AppearanceSlot> {
    Some(match part {
        Part::Hair => AppearanceSlot::HairStyle,
        Part::Eyes => AppearanceSlot::EyesStrip,
        Part::Nose => AppearanceSlot::NoseStrip,
        Part::Mouth => AppearanceSlot::MouthStrip,
        _ => return None,
    })
}

/// A part's colour slot (hair and eyes only).
#[must_use]
pub fn color_slot(part: Part) -> Option<AppearanceSlot> {
    match part {
        Part::Hair => Some(AppearanceSlot::HairColor),
        Part::Eyes => Some(AppearanceSlot::EyeColor),
        _ => None,
    }
}

/// The shade a part's slider moves: the hair's own, the skin's for the
/// skin and the parts that follow it (none for the eyes).
#[must_use]
pub fn shade_slot(part: Part) -> Option<ShadeSlot> {
    match part {
        Part::Hair => Some(ShadeSlot::Hair),
        Part::Nose | Part::Mouth | Part::Skin => Some(ShadeSlot::Skin),
        _ => None,
    }
}

/// How many styles the part offers.
#[must_use]
pub fn style_count(part: Part, g: &ChargenGenderOptions) -> usize {
    match part {
        Part::Hair => g.hair_styles.len(),
        Part::Eyes => g.eye_strips.len(),
        Part::Nose => g.nose_strips.len(),
        Part::Mouth => g.mouth_strips.len(),
        _ => 0,
    }
}

/// How many swatches the part offers: the hair and eye colours, one skin
/// colour for the rest.
#[must_use]
pub fn color_count(part: Part, g: Option<&ChargenGenderOptions>) -> usize {
    match (part, g) {
        (Part::Hair, Some(g)) => g.hair_colors.len(),
        (Part::Eyes, Some(g)) => g.eye_colors.len(),
        (Part::Hair | Part::Eyes, None) => 0,
        _ => 1,
    }
}

/// The swatch the mark stands on: the hair colour (the first while none
/// is chosen), the eye colour (none while none is chosen), the one skin
/// colour for the rest.
#[must_use]
pub fn marked_swatch(part: Part, view: &BarberView) -> Option<usize> {
    let a = &view.appearance;
    match part {
        Part::Hair if a.hair_color == UNSET => Some(0),
        Part::Hair => Some(a.hair_color as usize),
        Part::Eyes if a.eye_color == UNSET => None,
        Part::Eyes => Some(a.eye_color as usize),
        _ => Some(0),
    }
}

/// Picking `part`: the style to set when the look's is outside the list
/// (past the end: the first; none: the last), else none.
#[must_use]
pub fn style_on_pick(
    part: Part,
    view: &BarberView,
    g: &ChargenGenderOptions,
) -> Option<(AppearanceSlot, u32)> {
    let slot = style_slot(part)?;
    let count = style_count(part, g);
    let current = view.appearance.get(slot);
    if current != UNSET && current as usize >= count {
        Some((slot, 0))
    } else if current == UNSET && count > 0 {
        Some((slot, (count - 1) as u32))
    } else {
        None
    }
}

/// Stepping `part`'s style by `step`, wrapping round.
#[must_use]
pub fn style_on_step(
    part: Part,
    view: &BarberView,
    g: &ChargenGenderOptions,
    step: i32,
) -> Option<(AppearanceSlot, u32)> {
    let slot = style_slot(part)?;
    let count = i32::try_from(style_count(part, g)).unwrap_or(i32::MAX);
    Some((
        slot,
        cycle_index(view.appearance.get(slot), step, count, false),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gender() -> ChargenGenderOptions {
        ChargenGenderOptions {
            hair_styles: vec![Default::default(); 4],
            hair_colors: vec![1, 2, 3],
            eye_colors: vec![4, 5],
            ..ChargenGenderOptions::default()
        }
    }

    #[test]
    fn the_mark_stands_on_the_chosen_colour_or_the_one_skin_colour() {
        let mut view = BarberView::default();
        view.appearance.hair_color = 2;
        view.appearance.eye_color = UNSET;
        assert_eq!(marked_swatch(Part::Hair, &view), Some(2));
        assert_eq!(marked_swatch(Part::Eyes, &view), None);
        assert_eq!(marked_swatch(Part::Nose, &view), Some(0));
        view.appearance.hair_color = UNSET;
        assert_eq!(marked_swatch(Part::Hair, &view), Some(0));
    }

    #[test]
    fn hair_and_eyes_offer_their_colours_and_the_rest_one() {
        let g = gender();
        assert_eq!(color_count(Part::Hair, Some(&g)), 3);
        assert_eq!(color_count(Part::Eyes, Some(&g)), 2);
        assert_eq!(color_count(Part::Skin, Some(&g)), 1);
        assert_eq!(color_count(Part::Mouth, None), 1);
    }

    #[test]
    fn a_pick_brings_a_style_outside_the_list_back_and_a_step_wraps() {
        let g = gender();
        let mut view = BarberView::default();
        view.appearance.hair_style = 9;
        assert_eq!(
            style_on_pick(Part::Hair, &view, &g),
            Some((AppearanceSlot::HairStyle, 0))
        );
        view.appearance.hair_style = UNSET;
        assert_eq!(
            style_on_pick(Part::Hair, &view, &g),
            Some((AppearanceSlot::HairStyle, 3))
        );
        view.appearance.hair_style = 1;
        assert_eq!(style_on_pick(Part::Hair, &view, &g), None);
        assert_eq!(style_on_pick(Part::Skin, &view, &g), None);
        view.appearance.hair_style = 3;
        assert_eq!(
            style_on_step(Part::Hair, &view, &g, 1),
            Some((AppearanceSlot::HairStyle, 0))
        );
        assert_eq!(shade_slot(Part::Mouth), Some(ShadeSlot::Skin));
        assert_eq!(shade_slot(Part::Eyes), None);
    }
}
