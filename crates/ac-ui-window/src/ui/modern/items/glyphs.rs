//! The glyphs an empty equipment slot shows: what goes there, drawn as
//! lines in a 24-unit box (the design's slot pictures, curves as runs of
//! points).

use ac_ui::draw::{Color, RenderContext};

use crate::ui::modern::hud::lines::{arc, circle, stroke};

/// What an equipment slot holds, as its empty glyph shows it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlotGlyph {
    Helm,
    Chest,
    Arm,
    Glove,
    Shirt,
    Cloak,
    Leg,
    Boot,
    Pants,
    Neck,
    Wrist,
    Ring,
    Trinket,
    Sword,
    Shield,
    Arrow,
    Sigil,
}

impl SlotGlyph {
    /// The glyph for each of the equipment slots, in their order.
    pub const SLOTS: [Self; 24] = [
        Self::Helm,
        Self::Chest,
        Self::Arm,
        Self::Arm,
        Self::Glove,
        Self::Shirt,
        Self::Cloak,
        Self::Chest,
        Self::Leg,
        Self::Leg,
        Self::Boot,
        Self::Pants,
        Self::Neck,
        Self::Wrist,
        Self::Wrist,
        Self::Ring,
        Self::Ring,
        Self::Trinket,
        Self::Sword,
        Self::Shield,
        Self::Arrow,
        Self::Sigil,
        Self::Sigil,
        Self::Sigil,
    ];

    /// Draws the glyph `size` across at (`x`, `y`).
    pub fn draw(self, rc: &mut RenderContext<'_>, x: f32, y: f32, size: f32, color: Color) {
        let w = (size / 12.0).max(1.2);
        let lines = |rc: &mut RenderContext<'_>, l: &[&[(f32, f32)]]| {
            stroke(rc, x, y, size, l, color, w);
        };
        const SHIELD: &[(f32, f32)] = &[
            (12.0, 3.0),
            (20.0, 6.0),
            (20.0, 12.0),
            (19.0, 16.0),
            (16.0, 19.5),
            (12.0, 21.0),
            (8.0, 19.5),
            (5.0, 16.0),
            (4.0, 12.0),
            (4.0, 6.0),
            (12.0, 3.0),
        ];
        match self {
            Self::Helm => {
                arc(
                    rc,
                    x,
                    y,
                    size,
                    (12.0, 14.0),
                    7.0,
                    (std::f32::consts::PI, std::f32::consts::PI),
                    color,
                    w,
                );
                lines(
                    rc,
                    &[
                        &[(19.0, 14.0), (19.0, 18.0), (5.0, 18.0), (5.0, 14.0)],
                        &[(12.0, 7.0), (12.0, 4.0)],
                        &[(9.0, 18.0), (9.0, 14.0), (15.0, 14.0), (15.0, 18.0)],
                    ],
                );
            }
            Self::Chest | Self::Shield => lines(rc, &[SHIELD]),
            Self::Arm => lines(
                rc,
                &[&[
                    (7.0, 4.0),
                    (13.0, 4.0),
                    (16.0, 11.0),
                    (13.0, 20.0),
                    (8.0, 20.0),
                    (9.0, 11.0),
                    (7.0, 4.0),
                ]],
            ),
            Self::Glove => lines(
                rc,
                &[
                    &[
                        (7.0, 11.0),
                        (7.0, 5.0),
                        (8.5, 3.5),
                        (10.0, 5.0),
                        (10.0, 10.0),
                    ],
                    &[
                        (10.0, 10.0),
                        (10.0, 4.0),
                        (11.5, 2.5),
                        (13.0, 4.0),
                        (13.0, 10.0),
                    ],
                    &[
                        (13.0, 10.0),
                        (13.0, 5.0),
                        (14.5, 3.5),
                        (16.0, 5.0),
                        (16.0, 12.0),
                    ],
                    &[
                        (16.0, 12.0),
                        (16.0, 8.0),
                        (17.5, 6.5),
                        (19.0, 8.0),
                        (19.0, 14.0),
                        (17.0, 19.0),
                        (12.0, 21.0),
                        (8.0, 20.0),
                        (5.0, 16.0),
                        (3.0, 13.0),
                        (4.0, 11.5),
                        (5.5, 11.8),
                        (7.0, 14.0),
                    ],
                    &[(7.0, 11.0), (7.0, 14.0)],
                ],
            ),
            Self::Shirt => lines(
                rc,
                &[&[
                    (8.0, 3.0),
                    (3.0, 6.0),
                    (5.0, 11.0),
                    (7.0, 10.0),
                    (7.0, 21.0),
                    (17.0, 21.0),
                    (17.0, 10.0),
                    (19.0, 11.0),
                    (21.0, 6.0),
                    (16.0, 3.0),
                    (15.1, 5.1),
                    (13.0, 6.0),
                    (10.9, 5.1),
                    (10.0, 3.0),
                    (8.0, 3.0),
                ]],
            ),
            Self::Cloak => lines(
                rc,
                &[
                    &[
                        (8.0, 3.0),
                        (16.0, 3.0),
                        (17.0, 6.0),
                        (20.0, 21.0),
                        (4.0, 21.0),
                        (7.0, 6.0),
                        (8.0, 3.0),
                    ],
                    &[(12.0, 3.0), (12.0, 9.0)],
                ],
            ),
            Self::Leg => lines(
                rc,
                &[&[
                    (8.0, 3.0),
                    (16.0, 3.0),
                    (15.0, 12.0),
                    (16.0, 21.0),
                    (11.0, 21.0),
                    (10.0, 12.0),
                    (8.0, 3.0),
                ]],
            ),
            Self::Boot => lines(
                rc,
                &[&[
                    (5.0, 3.0),
                    (13.0, 3.0),
                    (13.0, 13.0),
                    (19.0, 16.0),
                    (20.0, 18.0),
                    (20.0, 21.0),
                    (5.0, 21.0),
                    (5.0, 3.0),
                ]],
            ),
            Self::Pants => lines(
                rc,
                &[&[
                    (6.0, 3.0),
                    (18.0, 3.0),
                    (19.0, 21.0),
                    (14.0, 21.0),
                    (12.0, 10.0),
                    (10.0, 21.0),
                    (5.0, 21.0),
                    (6.0, 3.0),
                ]],
            ),
            Self::Neck => {
                lines(
                    rc,
                    &[
                        &[
                            (5.0, 4.0),
                            (5.6, 8.0),
                            (7.5, 11.2),
                            (12.0, 13.0),
                            (16.5, 11.2),
                            (18.4, 8.0),
                            (19.0, 4.0),
                        ],
                        &[(12.0, 13.0), (12.0, 16.0)],
                    ],
                );
                circle(rc, x, y, size, (12.0, 18.0), 2.0, color, w);
            }
            Self::Wrist => {
                let k = size / 24.0;
                rc.stroke_ellipse(x + 6.0 * k, y + 8.0 * k, 12.0 * k, 8.0 * k, color, w);
                lines(
                    rc,
                    &[&[
                        (6.0, 12.0),
                        (6.0, 14.0),
                        (7.5, 16.6),
                        (12.0, 18.0),
                        (16.5, 16.6),
                        (18.0, 14.0),
                        (18.0, 12.0),
                    ]],
                );
            }
            Self::Ring => {
                circle(rc, x, y, size, (12.0, 15.0), 6.0, color, w);
                lines(
                    rc,
                    &[&[
                        (9.0, 4.0),
                        (15.0, 4.0),
                        (13.5, 9.0),
                        (10.5, 9.0),
                        (9.0, 4.0),
                    ]],
                );
            }
            Self::Trinket => lines(
                rc,
                &[&[
                    (12.0, 3.0),
                    (15.0, 9.0),
                    (21.0, 10.0),
                    (16.5, 14.0),
                    (17.5, 20.0),
                    (12.0, 17.0),
                    (6.5, 20.0),
                    (7.5, 14.0),
                    (3.0, 10.0),
                    (9.0, 9.0),
                    (12.0, 3.0),
                ]],
            ),
            Self::Sword => lines(
                rc,
                &[
                    &[
                        (14.5, 17.5),
                        (3.0, 6.0),
                        (3.0, 3.0),
                        (6.0, 3.0),
                        (17.5, 14.5),
                    ],
                    &[(13.0, 19.0), (19.0, 13.0)],
                    &[(16.0, 16.0), (20.0, 20.0)],
                    &[(19.0, 21.0), (21.0, 19.0)],
                ],
            ),
            Self::Arrow => lines(
                rc,
                &[
                    &[(4.0, 20.0), (20.0, 4.0)],
                    &[(14.0, 4.0), (20.0, 4.0), (20.0, 10.0)],
                    &[(4.0, 16.0), (4.0, 20.0), (8.0, 20.0)],
                ],
            ),
            Self::Sigil => lines(
                rc,
                &[&[
                    (12.0, 3.0),
                    (20.0, 12.0),
                    (12.0, 21.0),
                    (4.0, 12.0),
                    (12.0, 3.0),
                ]],
            ),
        }
    }
}
