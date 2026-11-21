//! The skies the pre-game screens stand in: a vertical run of colours, two
//! rows of far hills, and the shades that keep the words readable. Each
//! place has its own sky (the desert towns warm, the Sho towns in
//! blossom, the coasts cool), the connecting screen a dark hall, and the
//! credits and an empty account a quiet dusk.

use ac_ui::draw::{Color, ColorVertex, RenderContext};
use ac_ui::modern::theme::hex;
use ac_ui::shapes::CornerRadii;

/// A sky.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sky {
    /// The desert towns.
    Desert,
    /// The coasts and islands.
    Coast,
    /// The Sho towns.
    Blossom,
    /// A character never played on this computer.
    Unknown,
    /// A character pending deletion.
    Faded,
    /// The connecting screen's dark hall.
    Portal,
    /// Everywhere else, the credits and an empty account.
    Dusk,
}

/// A sky's colours: (stop, colour) down the screen, the far and near
/// hills, and the glow behind the figure.
struct Palette {
    stops: &'static [(f32, u32)],
    far: u32,
    near: u32,
    glow: Color,
}

const DESERT: Palette = Palette {
    stops: &[
        (0.0, 0x24_172A),
        (0.34, 0x5E_3340),
        (0.56, 0xB8_694A),
        (0.64, 0xE3_A462),
        (0.66, 0x6A_3E26),
        (1.0, 0x2A_1A12),
    ],
    far: 0x4A_2C20,
    near: 0x24_160F,
    glow: hex(0xFF_B060, 0.45),
};
const COAST: Palette = Palette {
    stops: &[
        (0.0, 0x0B_1822),
        (0.36, 0x1F_3F4C),
        (0.58, 0x6E_9C9A),
        (0.64, 0xA9_C8BC),
        (0.66, 0x2E_4A48),
        (1.0, 0x0F_1C1E),
    ],
    far: 0x1E_3836,
    near: 0x0E_1F20,
    glow: hex(0x96_E2D6, 0.32),
};
const BLOSSOM: Palette = Palette {
    stops: &[
        (0.0, 0x16_0F28),
        (0.36, 0x3E_2652),
        (0.58, 0xA2_607E),
        (0.64, 0xE0_9AA6),
        (0.66, 0x4A_2A44),
        (1.0, 0x1A_1020),
    ],
    far: 0x3A_2240,
    near: 0x1C_1222,
    glow: hex(0xFF_A4C4, 0.36),
};
const UNKNOWN: Palette = Palette {
    stops: &[
        (0.0, 0x0A_0B10),
        (0.40, 0x19_1B24),
        (0.60, 0x33_3542),
        (0.64, 0x2A_2C36),
        (0.66, 0x15_161C),
        (1.0, 0x0B_0B0F),
    ],
    far: 0x19_1A22,
    near: 0x0E_0F14,
    glow: hex(0xC8_CCE4, 0.14),
};
const FADED: Palette = Palette {
    stops: &[
        (0.0, 0x12_1110),
        (0.40, 0x2E_2925),
        (0.60, 0x5E_554C),
        (0.64, 0x7A_6F62),
        (0.66, 0x2E_2924),
        (1.0, 0x14_1210),
    ],
    far: 0x2A_2520,
    near: 0x17_1411,
    glow: hex(0xD2_BEA0, 0.16),
};
const PORTAL: Palette = Palette {
    stops: &[
        (0.0, 0x0A_090C),
        (0.48, 0x14_1117),
        (0.66, 0x20_1913),
        (1.0, 0x0D_0A08),
    ],
    far: 0x15_111A,
    near: 0x0C_0A0D,
    glow: hex(0x96_6EE6, 0.18),
};
const DUSK: Palette = Palette {
    stops: &[
        (0.0, 0x10_0C0A),
        (0.38, 0x2E_2118),
        (0.58, 0x7A_5432),
        (0.64, 0xB9_8448),
        (0.66, 0x3A_2818),
        (1.0, 0x12_0D09),
    ],
    far: 0x2A_1E15,
    near: 0x14_0F0B,
    glow: hex(0xE0_B068, 0.35),
};

impl Sky {
    fn palette(self) -> &'static Palette {
        match self {
            Self::Desert => &DESERT,
            Self::Coast => &COAST,
            Self::Blossom => &BLOSSOM,
            Self::Unknown => &UNKNOWN,
            Self::Faded => &FADED,
            Self::Portal => &PORTAL,
            Self::Dusk => &DUSK,
        }
    }

    /// The glow behind a figure standing in this sky.
    #[must_use]
    pub fn glow(self) -> Color {
        self.palette().glow
    }

    /// The sky of the place a character was last (a town's name).
    #[must_use]
    pub fn of_place(place: &str) -> Self {
        const DESERT_TOWNS: &[&str] = &[
            "Al-Arqas",
            "Al-Jalima",
            "Ayan Baqur",
            "Dryreach",
            "Khayyaban",
            "Qalaba'r",
            "Samsur",
            "Tufa",
            "Uziz",
            "Xarabydun",
            "Yaraq",
            "Zaikhal",
        ];
        const SHO_TOWNS: &[&str] = &[
            "Baishi",
            "Hebian-to",
            "Kara",
            "Lin",
            "Nanto",
            "Sawato",
            "Shoushi",
            "Tou-Tou",
            "Yanshi",
        ];
        const COAST_TOWNS: &[&str] = &[
            "Aerlinthe Island",
            "Kryst",
            "Mayoi",
            "Singularity Caul Island",
            "Timaru",
            "Ulgrim's Island",
            "Wai Jhou",
        ];
        if DESERT_TOWNS.contains(&place) {
            Self::Desert
        } else if SHO_TOWNS.contains(&place) {
            Self::Blossom
        } else if COAST_TOWNS.contains(&place) {
            Self::Coast
        } else {
            Self::Dusk
        }
    }
}

/// The far hills' outline over a `w` wide screen, `h` high: (share of the
/// width, share of the height).
const FAR_HILLS: &[(f32, f32)] = &[
    (0.0, 0.622),
    (0.111, 0.578),
    (0.208, 0.600),
    (0.292, 0.567),
    (0.389, 0.522),
    (0.486, 0.578),
    (0.569, 0.556),
    (0.667, 0.529),
    (0.764, 0.578),
    (0.861, 0.551),
    (0.931, 0.533),
    (1.0, 0.547),
];
const NEAR_HILLS: &[(f32, f32)] = &[
    (0.0, 0.742),
    (0.139, 0.707),
    (0.264, 0.727),
    (0.389, 0.718),
    (0.528, 0.704),
    (0.653, 0.738),
    (0.778, 0.718),
    (0.875, 0.702),
    (0.958, 0.724),
    (1.0, 0.720),
];

/// Hills along `outline` filled down to the bottom of the screen.
fn hills(rc: &mut RenderContext<'_>, w: f32, h: f32, outline: &[(f32, f32)], color: Color) {
    let mut tris = Vec::with_capacity(outline.len() * 6);
    let v = |x: f32, y: f32| ColorVertex { x, y, color };
    // A smooth line through the outline: each span eased with a cosine.
    let steps = 8;
    let mut points = Vec::new();
    for pair in outline.windows(2) {
        let (a, b) = (pair[0], pair[1]);
        for s in 0..steps {
            let t = s as f32 / steps as f32;
            let e = (1.0 - (t * std::f32::consts::PI).cos()) * 0.5;
            points.push((a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * e));
        }
    }
    if let Some(&last) = outline.last() {
        points.push(last);
    }
    for pair in points.windows(2) {
        let (x0, y0) = (pair[0].0 * w, pair[0].1 * h);
        let (x1, y1) = (pair[1].0 * w, pair[1].1 * h);
        tris.extend([
            v(x0, y0),
            v(x1, y1),
            v(x1, h),
            v(x0, y0),
            v(x1, h),
            v(x0, h),
        ]);
    }
    rc.triangles(&tris);
}

/// The sky over the whole `w` by `h` screen, with its hills and the shades
/// that darken the foot (and the left edge, under a list).
pub fn draw(rc: &mut RenderContext<'_>, w: f32, h: f32, sky: Sky, left_shade: bool) {
    let p = sky.palette();
    for pair in p.stops.windows(2) {
        let (a, b) = (pair[0], pair[1]);
        let y = a.0 * h;
        let height = (b.0 - a.0) * h;
        rc.fill_vertical_gradient(
            0.0,
            y,
            w,
            height + 0.5,
            CornerRadii::all(0.0),
            hex(a.1, 1.0),
            hex(b.1, 1.0),
        );
    }
    hills(rc, w, h, FAR_HILLS, hex(p.far, 1.0));
    hills(rc, w, h, NEAR_HILLS, hex(p.near, 1.0));
    let foot = 320.0_f32.min(h * 0.4);
    rc.fill_vertical_gradient(
        0.0,
        h - foot,
        w,
        foot,
        CornerRadii::all(0.0),
        hex(0, 0.0),
        hex(0, 0.55),
    );
    if left_shade {
        rc.fill_horizontal_gradient(
            0.0,
            0.0,
            520.0_f32.min(w * 0.45),
            h,
            CornerRadii::all(0.0),
            hex(0, 0.55),
            hex(0, 0.0),
        );
    }
}

/// A soft glow: rings of `color` fading out from the middle of `r`.
pub fn glow(rc: &mut RenderContext<'_>, cx: f32, cy: f32, rx: f32, ry: f32, color: Color) {
    // Many faint rings, thinning towards the edge, so no ring shows.
    let rings = 36;
    for i in 0..rings {
        let t = 1.0 - i as f32 / rings as f32;
        let c = Color {
            a: color.a * 2.2 / rings as f32 * (1.0 - t * t * 0.6),
            ..color
        };
        rc.fill_ellipse(cx - rx * t, cy - ry * t, rx * 2.0 * t, ry * 2.0 * t, c);
    }
}

/// The lit plinth a figure stands on, centred on (`cx`, `cy`).
pub fn plinth(rc: &mut RenderContext<'_>, cx: f32, cy: f32, w: f32, h: f32) {
    glow(rc, cx, cy, w * 0.5, h * 0.5, hex(0xE0_B068, 0.32));
    rc.stroke_ellipse(cx - w * 0.5, cy - h * 0.5, w, h, hex(0xC9_A062, 0.4), 1.0);
}

#[cfg(test)]
mod tests {
    use super::Sky;

    #[test]
    fn a_place_tints_the_sky_by_its_lands() {
        assert_eq!(Sky::of_place("Ayan Baqur"), Sky::Desert);
        assert_eq!(Sky::of_place("Shoushi"), Sky::Blossom);
        assert_eq!(Sky::of_place("Mayoi"), Sky::Coast);
        assert_eq!(Sky::of_place("Holtburg"), Sky::Dusk);
        assert_eq!(Sky::of_place(""), Sky::Dusk);
    }
}
