//! Where each modern window first appears: a corner, edge or the centre of
//! the screen, an offset from it, and a size. The window is placed there
//! when it is made; the layout persistence takes that as its default and
//! keeps the player's own place after.

use ac_ui::geometry::Rect;

/// The point of the screen a window is held to; the same point of the
/// window sits on it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Anchor {
    /// The top left corner.
    TopLeft,
    /// The middle of the top edge.
    Top,
    /// The top right corner.
    TopRight,
    /// The middle of the left edge.
    Left,
    /// The centre.
    Center,
    /// The middle of the right edge.
    Right,
    /// The bottom left corner.
    BottomLeft,
    /// The middle of the bottom edge.
    Bottom,
    /// The bottom right corner.
    BottomRight,
}

impl Anchor {
    /// How far across and down the point is, as fractions of a width and
    /// a height.
    const fn fractions(self) -> (f32, f32) {
        match self {
            Self::TopLeft => (0.0, 0.0),
            Self::Top => (0.5, 0.0),
            Self::TopRight => (1.0, 0.0),
            Self::Left => (0.0, 0.5),
            Self::Center => (0.5, 0.5),
            Self::Right => (1.0, 0.5),
            Self::BottomLeft => (0.0, 1.0),
            Self::Bottom => (0.5, 1.0),
            Self::BottomRight => (1.0, 1.0),
        }
    }
}

/// Where a window first appears.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Place {
    /// The point it is held to.
    pub anchor: Anchor,
    /// Moved right (negative: left) from there.
    pub dx: f32,
    /// Moved down (negative: up) from there.
    pub dy: f32,
    /// Its width.
    pub width: f32,
    /// Its height.
    pub height: f32,
}

impl Place {
    /// The window's rectangle on a screen of `screen` units, kept on the
    /// screen where it fits.
    #[must_use]
    pub fn rect(&self, screen: (f32, f32)) -> Rect {
        let (fx, fy) = self.anchor.fractions();
        let x = screen.0 * fx - self.width * fx + self.dx;
        let y = screen.1 * fy - self.height * fy + self.dy;
        let x = x.min(screen.0 - self.width).max(0.0);
        let y = y.min(screen.1 - self.height).max(0.0);
        Rect::new(x, y, self.width, self.height)
    }
}

/// Every modern window's first place, by window name (filled in as the
/// windows are built).
const PLACES: &[(&str, Place)] = &[
    (
        super::hud::VITALS,
        Place {
            anchor: Anchor::Top,
            dx: 0.0,
            dy: 14.0,
            width: super::hud::vitals::WIDTH,
            height: super::hud::vitals::BAR_HEIGHT,
        },
    ),
    (
        super::hud::TARGET,
        Place {
            anchor: Anchor::Top,
            dx: 0.0,
            dy: 46.0,
            width: super::hud::target::WIDTH,
            height: super::hud::target::HEIGHT,
        },
    ),
    (
        super::hud::EFFECTS,
        Place {
            anchor: Anchor::TopLeft,
            dx: 16.0,
            dy: 14.0,
            width: super::hud::effects::CHIP_HEIGHT,
            height: super::hud::effects::CHIP_HEIGHT,
        },
    ),
    (
        super::hud::RADAR,
        Place {
            anchor: Anchor::TopRight,
            dx: -6.0,
            dy: 12.0,
            width: super::hud::radar::WIDTH,
            height: super::hud::radar::HEIGHT,
        },
    ),
];

/// Where modern window `name` first appears.
#[must_use]
pub fn default_place(name: &str) -> Option<Place> {
    PLACES
        .iter()
        .find(|(window, _)| *window == name)
        .map(|(_, place)| *place)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn place(anchor: Anchor, dx: f32, dy: f32) -> Place {
        Place {
            anchor,
            dx,
            dy,
            width: 200.0,
            height: 100.0,
        }
    }

    #[test]
    fn a_window_sits_on_its_anchor() {
        let screen = (1000.0, 800.0);
        assert_eq!(
            place(Anchor::TopLeft, 10.0, 20.0).rect(screen),
            Rect::new(10.0, 20.0, 200.0, 100.0)
        );
        assert_eq!(
            place(Anchor::Top, 0.0, 12.0).rect(screen),
            Rect::new(400.0, 12.0, 200.0, 100.0)
        );
        assert_eq!(
            place(Anchor::BottomRight, -16.0, -16.0).rect(screen),
            Rect::new(784.0, 684.0, 200.0, 100.0)
        );
        assert_eq!(
            place(Anchor::Center, 0.0, 0.0).rect(screen),
            Rect::new(400.0, 350.0, 200.0, 100.0)
        );
    }

    #[test]
    fn the_vitals_start_top_centre() {
        let place = default_place(crate::ui::modern::hud::VITALS).unwrap();
        let rect = place.rect((1920.0, 1080.0));
        assert_eq!((rect.x, rect.y), ((1920.0 - rect.w) / 2.0, 14.0));
    }

    #[test]
    fn a_window_is_kept_on_the_screen() {
        let screen = (300.0, 150.0);
        assert_eq!(
            place(Anchor::TopRight, 50.0, -40.0).rect(screen),
            Rect::new(100.0, 0.0, 200.0, 100.0)
        );
    }
}
