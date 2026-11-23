//! The modern look's mouse cursors: one for each global cursor the game
//! picks ([`GlobalCursorKind`]), so they behave as the retail ones do.
//! Cream with a dark edge, readable on sky, grass and dungeon; bronze over
//! something the player can act on. The modes wear a badge at the lower
//! right (crossed blades, a blue spark, a ring, a turning arc), examine is
//! a lens, and choosing a target is a ring with four ticks.
//!
//! They are drawn here, from shapes on a 24-unit grid, at whatever size
//! the window wants: each pixel's cover comes from its distance to the
//! shape, so the edges are smooth at any size without image files.

use crate::cursor::GlobalCursorKind;

/// The size the cursors are drawn at, pixels.
pub const SIZE: u32 = 32;

/// The grid the shapes are drawn on, with room round it for the strokes
/// that reach past it: (first unit, units across).
const VIEW: (f32, f32) = (-1.5, 27.0);

/// An RGBA colour, components 0..=1, not premultiplied.
type Rgba = [f32; 4];

const DARK: Rgba = rgb(0x14_100C);
const CREAM: Rgba = rgb(0xF4_EBDC);
const GOLD: Rgba = rgb(0xE0_B068);
const BLUE: Rgba = rgb(0x8F_C1FF);
const RED: Rgba = rgb(0xFF_6B70);

const fn rgb(hex: u32) -> Rgba {
    [
        ((hex >> 16) & 0xFF) as f32 / 255.0,
        ((hex >> 8) & 0xFF) as f32 / 255.0,
        (hex & 0xFF) as f32 / 255.0,
        1.0,
    ]
}

const fn with_alpha(c: Rgba, a: f32) -> Rgba {
    [c[0], c[1], c[2], a]
}

/// A point on the grid.
type P = (f32, f32);

/// One shape, painted over the ones before it.
#[derive(Debug, Clone)]
enum Shape {
    /// A filled polygon.
    Fill(Vec<P>, Rgba),
    /// Lines through points, `width` wide, round at the joins and ends
    /// (closed: back to the first).
    Lines(Vec<P>, bool, f32, Rgba),
    /// A filled disk.
    Disk(P, f32, Rgba),
    /// A circle's outline, `width` wide.
    Ring(P, f32, f32, Rgba),
    /// Part of a circle's outline from `from` turning `turn` radians
    /// (clockwise on screen), `width` wide, round at its ends.
    Arc(P, f32, f32, f32, f32, Rgba),
}

/// The arrow's outline.
const ARROW: [P; 7] = [
    (4.0, 2.5),
    (4.0, 19.5),
    (8.6, 15.4),
    (11.6, 21.8),
    (14.6, 20.4),
    (11.6, 14.2),
    (17.6, 14.2),
];

/// The badge's middle.
const BADGE: P = (18.5, 18.5);

fn arrow(found: bool) -> Vec<Shape> {
    vec![
        Shape::Fill(ARROW.to_vec(), if found { GOLD } else { CREAM }),
        Shape::Lines(ARROW.to_vec(), true, 1.6, DARK),
    ]
}

fn badge(found: bool, marks: Vec<Shape>) -> Vec<Shape> {
    let mut out = arrow(found);
    out.push(Shape::Disk(BADGE, 5.5, DARK));
    out.push(Shape::Ring(BADGE, 5.5, 1.0, GOLD));
    out.extend(marks);
    out
}

fn combat() -> Vec<Shape> {
    vec![
        Shape::Lines(vec![(16.0, 16.0), (21.0, 21.0)], false, 1.5, CREAM),
        Shape::Lines(vec![(21.0, 16.0), (16.0, 21.0)], false, 1.5, CREAM),
    ]
}

fn magic() -> Vec<Shape> {
    vec![Shape::Fill(
        vec![
            (18.5, 14.8),
            (19.4, 17.6),
            (22.2, 18.5),
            (19.4, 19.4),
            (18.5, 22.2),
            (17.6, 19.4),
            (14.8, 18.5),
            (17.6, 17.6),
        ],
        BLUE,
    )]
}

fn use_mark() -> Vec<Shape> {
    vec![
        Shape::Ring(BADGE, 3.3, 0.9, CREAM),
        Shape::Disk(BADGE, 1.7, CREAM),
    ]
}

fn busy() -> Vec<Shape> {
    // From the top, round past the bottom to the left.
    vec![Shape::Arc(
        BADGE,
        3.5,
        -std::f32::consts::FRAC_PI_2,
        1.5 * std::f32::consts::PI,
        1.6,
        GOLD,
    )]
}

fn examine(found: bool) -> Vec<Shape> {
    let lens = (10.0, 10.0);
    let handle = vec![(14.8, 14.8), (21.0, 21.0)];
    let ink = if found { GOLD } else { CREAM };
    let glass = if found {
        with_alpha(GOLD, 0.35)
    } else {
        with_alpha(CREAM, 0.18)
    };
    vec![
        Shape::Ring(lens, 6.5, 4.4, DARK),
        Shape::Lines(handle.clone(), false, 4.4, DARK),
        Shape::Disk(lens, 5.6, glass),
        Shape::Ring(lens, 6.5, 2.0, ink),
        Shape::Lines(handle, false, 2.0, ink),
    ]
}

fn target(ink: Rgba, centre: Vec<Shape>) -> Vec<Shape> {
    let middle = (12.0, 12.0);
    let ticks = |width, color| {
        [
            [(12.0, 0.5), (12.0, 5.5)],
            [(12.0, 18.5), (12.0, 23.5)],
            [(0.5, 12.0), (5.5, 12.0)],
            [(18.5, 12.0), (23.5, 12.0)],
        ]
        .map(|t| Shape::Lines(t.to_vec(), false, width, color))
    };
    let mut out = vec![Shape::Ring(middle, 8.0, 4.0, DARK)];
    out.extend(ticks(4.0, DARK));
    out.push(Shape::Ring(middle, 8.0, 2.0, ink));
    out.extend(ticks(1.8, ink));
    out.extend(centre);
    out
}

/// The shapes of cursor `kind`.
fn shapes(kind: GlobalCursorKind) -> Vec<Shape> {
    use GlobalCursorKind as G;
    match kind {
        G::Default => arrow(false),
        G::DefaultFound => arrow(true),
        G::MeleeOrMissile => badge(false, combat()),
        G::MeleeOrMissileFound => badge(true, combat()),
        G::Magic => badge(false, magic()),
        G::MagicFound => badge(true, magic()),
        G::Examine => examine(false),
        G::ExamineFound => examine(true),
        G::Use => badge(false, use_mark()),
        G::UseFound => badge(true, use_mark()),
        G::Busy => badge(false, busy()),
        G::BusyFound => badge(true, busy()),
        G::TargetPending => target(CREAM, vec![Shape::Ring((12.0, 12.0), 1.8, 1.2, CREAM)]),
        G::TargetValid => target(
            GOLD,
            vec![
                Shape::Disk((12.0, 12.0), 2.8, GOLD),
                Shape::Ring((12.0, 12.0), 2.8, 1.0, DARK),
            ],
        ),
        G::TargetInvalid => target(
            RED,
            vec![Shape::Lines(
                vec![(6.6, 6.6), (17.4, 17.4)],
                false,
                2.2,
                RED,
            )],
        ),
    }
}

/// Where cursor `kind` points, on the grid: the arrow's tip, the lens's
/// middle, the target's middle.
fn hot_spot(kind: GlobalCursorKind) -> P {
    use GlobalCursorKind as G;
    match kind {
        G::Examine | G::ExamineFound => (10.0, 10.0),
        G::TargetPending | G::TargetValid | G::TargetInvalid => (12.0, 12.0),
        _ => ARROW[0],
    }
}

/// A drawn cursor: `size` x `size` RGBA pixels (not premultiplied) and the
/// pixel it points with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DrawnCursor {
    /// Its side, pixels.
    pub size: u32,
    /// Row by row from the top, four bytes a pixel.
    pub rgba: Vec<u8>,
    /// The pixel it points with.
    pub hot_spot: (u32, u32),
}

/// Cursor `kind` drawn `size` pixels square.
#[must_use]
pub fn draw(kind: GlobalCursorKind, size: u32) -> DrawnCursor {
    let scale = size as f32 / VIEW.1;
    let shapes = shapes(kind);
    let mut rgba = Vec::with_capacity((size * size * 4) as usize);
    for y in 0..size {
        for x in 0..size {
            let p = (
                (x as f32 + 0.5) / scale + VIEW.0,
                (y as f32 + 0.5) / scale + VIEW.0,
            );
            // Painted in order, premultiplied.
            let mut out = [0.0f32; 4];
            for s in &shapes {
                let (distance, color) = s.distance(p);
                let cover = (0.5 - distance * scale).clamp(0.0, 1.0) * color[3];
                if cover <= 0.0 {
                    continue;
                }
                for i in 0..3 {
                    out[i] = color[i] * cover + out[i] * (1.0 - cover);
                }
                out[3] = cover + out[3] * (1.0 - cover);
            }
            let a = out[3];
            let straight = |c: f32| {
                if a > 0.0 {
                    ((c / a).clamp(0.0, 1.0) * 255.0).round() as u8
                } else {
                    0
                }
            };
            rgba.extend([
                straight(out[0]),
                straight(out[1]),
                straight(out[2]),
                (a.clamp(0.0, 1.0) * 255.0).round() as u8,
            ]);
        }
    }
    let (hx, hy) = hot_spot(kind);
    let pixel = |v: f32| (((v - VIEW.0) * scale) as u32).min(size.saturating_sub(1));
    DrawnCursor {
        size,
        rgba,
        hot_spot: (pixel(hx), pixel(hy)),
    }
}

impl Shape {
    /// How far `p` is outside the shape (negative inside), in grid units,
    /// and its colour.
    fn distance(&self, p: P) -> (f32, Rgba) {
        match self {
            Shape::Fill(points, color) => {
                let edge = polyline_distance(points, true, p);
                let d = if inside(points, p) { -edge } else { edge };
                (d, *color)
            }
            Shape::Lines(points, closed, width, color) => {
                (polyline_distance(points, *closed, p) - width * 0.5, *color)
            }
            Shape::Disk(c, r, color) => (length(sub(p, *c)) - r, *color),
            Shape::Ring(c, r, width, color) => {
                ((length(sub(p, *c)) - r).abs() - width * 0.5, *color)
            }
            Shape::Arc(c, r, from, turn, width, color) => {
                let v = sub(p, *c);
                let angle = v.1.atan2(v.0);
                let along = (angle - from).rem_euclid(std::f32::consts::TAU);
                let d = if along <= *turn {
                    (length(v) - r).abs()
                } else {
                    let end = |a: f32| (c.0 + r * a.cos(), c.1 + r * a.sin());
                    length(sub(p, end(*from))).min(length(sub(p, end(from + turn))))
                };
                (d - width * 0.5, *color)
            }
        }
    }
}

fn sub(a: P, b: P) -> P {
    (a.0 - b.0, a.1 - b.1)
}

fn length(v: P) -> f32 {
    v.0.hypot(v.1)
}

/// How far `p` is from the line from `a` to `b`.
fn segment_distance(a: P, b: P, p: P) -> f32 {
    let ab = sub(b, a);
    let ap = sub(p, a);
    let len2 = ab.0 * ab.0 + ab.1 * ab.1;
    let t = if len2 > 0.0 {
        ((ap.0 * ab.0 + ap.1 * ab.1) / len2).clamp(0.0, 1.0)
    } else {
        0.0
    };
    length(sub(ap, (ab.0 * t, ab.1 * t)))
}

fn polyline_distance(points: &[P], closed: bool, p: P) -> f32 {
    let mut best = f32::INFINITY;
    for w in points.windows(2) {
        best = best.min(segment_distance(w[0], w[1], p));
    }
    if closed && let (Some(&first), Some(&last)) = (points.first(), points.last()) {
        best = best.min(segment_distance(last, first, p));
    }
    best
}

/// Whether `p` is inside the polygon (even-odd).
fn inside(points: &[P], p: P) -> bool {
    let mut odd = false;
    let n = points.len();
    for i in 0..n {
        let (a, b) = (points[i], points[(i + n - 1) % n]);
        if (a.1 > p.1) != (b.1 > p.1) && p.0 < (b.0 - a.0) * (p.1 - a.1) / (b.1 - a.1) + a.0 {
            odd = !odd;
        }
    }
    odd
}

#[cfg(test)]
mod tests {
    use super::*;
    use GlobalCursorKind as G;

    const ALL: [GlobalCursorKind; 15] = [
        G::Default,
        G::DefaultFound,
        G::MeleeOrMissile,
        G::MeleeOrMissileFound,
        G::Magic,
        G::MagicFound,
        G::Examine,
        G::ExamineFound,
        G::Use,
        G::UseFound,
        G::Busy,
        G::BusyFound,
        G::TargetPending,
        G::TargetValid,
        G::TargetInvalid,
    ];

    fn pixel(c: &DrawnCursor, x: u32, y: u32) -> [u8; 4] {
        let i = ((y * c.size + x) * 4) as usize;
        [c.rgba[i], c.rgba[i + 1], c.rgba[i + 2], c.rgba[i + 3]]
    }

    /// The grid point `p`'s pixel.
    fn at(c: &DrawnCursor, p: P) -> [u8; 4] {
        let scale = c.size as f32 / VIEW.1;
        pixel(
            c,
            ((p.0 - VIEW.0) * scale) as u32,
            ((p.1 - VIEW.0) * scale) as u32,
        )
    }

    #[test]
    fn every_cursor_is_drawn_whole_and_different() {
        let drawn: Vec<DrawnCursor> = ALL.iter().map(|k| draw(*k, SIZE)).collect();
        for (k, c) in ALL.iter().zip(&drawn) {
            assert_eq!(c.rgba.len(), (SIZE * SIZE * 4) as usize, "{k:?}");
            assert!(
                c.rgba.chunks(4).any(|p| p[3] == 255),
                "{k:?} has solid pixels"
            );
            assert!(
                c.rgba.chunks(4).any(|p| p[3] == 0),
                "{k:?} has clear pixels"
            );
            assert!(c.hot_spot.0 < SIZE && c.hot_spot.1 < SIZE);
        }
        for i in 0..drawn.len() {
            for j in i + 1..drawn.len() {
                assert_ne!(drawn[i].rgba, drawn[j].rgba, "{:?} {:?}", ALL[i], ALL[j]);
            }
        }
    }

    #[test]
    fn the_arrow_is_cream_and_turns_bronze_over_something() {
        let inside = (6.0, 12.0);
        let plain = at(&draw(G::Default, SIZE), inside);
        let found = at(&draw(G::DefaultFound, SIZE), inside);
        assert_eq!(&plain[..3], &[0xF4, 0xEB, 0xDC]);
        assert_eq!(&found[..3], &[0xE0, 0xB0, 0x68]);
        assert_eq!(plain[3], 255);
        // Its tip is where it points.
        let c = draw(G::Default, SIZE);
        let scale = SIZE as f32 / VIEW.1;
        assert_eq!(
            c.hot_spot,
            (
                ((4.0 - VIEW.0) * scale) as u32,
                ((2.5 - VIEW.0) * scale) as u32
            )
        );
    }

    #[test]
    fn the_target_points_with_its_middle_and_is_red_where_it_cannot_be_used() {
        let c = draw(G::TargetInvalid, SIZE);
        assert_eq!(c.hot_spot, (SIZE / 2, SIZE / 2));
        let ring = at(&c, (12.0, 4.0));
        assert_eq!(&ring[..3], &[0xFF, 0x6B, 0x70]);
    }

    /// Each cursor's raw RGBA, at 32 and 64 pixels, in
    /// `OPENAC_SNAPSHOT_DIR` (for a look at them).
    #[test]
    #[ignore = "writes pictures"]
    fn the_cursors() {
        let dir = std::env::var_os("OPENAC_SNAPSHOT_DIR").map_or_else(
            || std::env::temp_dir().join("openac-snapshots"),
            std::path::PathBuf::from,
        );
        std::fs::create_dir_all(&dir).unwrap();
        for size in [SIZE, 64] {
            for k in ALL {
                let c = draw(k, size);
                let path = dir.join(format!("cursor-{size}-{k:?}.rgba"));
                std::fs::write(&path, &c.rgba).unwrap();
            }
        }
    }

    #[test]
    fn they_draw_at_any_size() {
        let big = draw(G::Magic, 64);
        assert_eq!(big.rgba.len(), 64 * 64 * 4);
        let small = draw(G::Magic, SIZE).hot_spot;
        assert!(big.hot_spot.0.abs_diff(small.0 * 2) <= 1);
        assert!(big.hot_spot.1.abs_diff(small.1 * 2) <= 1);
    }
}
