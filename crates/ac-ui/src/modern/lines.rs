//! Icons drawn as lines, the way the design draws them: runs of points in
//! a 24-unit box, scaled into the size wanted.

use crate::draw::{Color, RenderContext};

/// Strokes `lines` (each a run of points in a 24-unit box) scaled into a
/// `size` box at (`x`, `y`).
pub fn stroke(
    rc: &mut RenderContext<'_>,
    x: f32,
    y: f32,
    size: f32,
    lines: &[&[(f32, f32)]],
    color: Color,
    width: f32,
) {
    let k = size / 24.0;
    for line in lines {
        for pair in line.windows(2) {
            let (a, b) = (pair[0], pair[1]);
            rc.smooth_line(
                x + a.0 * k,
                y + a.1 * k,
                x + b.0 * k,
                y + b.1 * k,
                color,
                width,
            );
        }
    }
}

/// A circle of radius `r` round (`cx`, `cy`) in the 24-unit box.
#[allow(clippy::too_many_arguments)]
pub fn circle(
    rc: &mut RenderContext<'_>,
    x: f32,
    y: f32,
    size: f32,
    (cx, cy): (f32, f32),
    r: f32,
    color: Color,
    width: f32,
) {
    let k = size / 24.0;
    let (cx, cy, r) = (x + cx * k, y + cy * k, r * k);
    rc.stroke_ellipse(cx - r, cy - r, r * 2.0, r * 2.0, color, width);
}

/// An arc of radius `r` round (`cx`, `cy`) in the 24-unit box, from
/// `start` radians (0 east, y down) through `sweep`.
#[allow(clippy::too_many_arguments)]
pub fn arc(
    rc: &mut RenderContext<'_>,
    x: f32,
    y: f32,
    size: f32,
    (cx, cy): (f32, f32),
    r: f32,
    (start, sweep): (f32, f32),
    color: Color,
    width: f32,
) {
    let k = size / 24.0;
    rc.stroke_arc(x + cx * k, y + cy * k, r * k, start, sweep, color, width);
}
