//! The bundled sans fonts, baked into glyph atlases from their TrueType
//! outlines, with no operating-system font needed: Noto Sans Regular and
//! SemiBold (`assets/fonts/NotoSans/`), which the modern plugin themes draw
//! in place of the dat font, and Inter Regular, Medium and SemiBold
//! (`assets/fonts/Inter/`), the modern interface's typeface. Both are under
//! SIL OFL 1.1.
//!
//! Inter is baked with two things Noto is not, so plugin windows keep the
//! look they have: its kerning (pairs moved closer or apart, whole pixels),
//! and digits all as wide as the widest, so a number that changes does not
//! shift (what a font's tabular figures do; the rasteriser cannot apply
//! that feature itself).
//!
//! The atlas is laid out the way stb_truetype's simple baker lays it out
//! (OpenAC bakes with it): three code-point ranges, each baked into its own
//! 512 x 512 coverage page, the pages stacked vertically; glyphs placed left
//! to right in rows with one pixel of space, a new row starting under the
//! tallest glyph of the row before. Glyph boxes, advances and the line
//! metrics are read from the font's tables as that baker reads them; the
//! coverage inside each box is drawn by `fontdue`, so the pixels themselves
//! differ slightly from OpenAC's.

use std::sync::Arc;

use ac_dat::objects::FontCharDesc;
use ttf_parser::{Face, GlyphId};

use crate::draw::TextureRef;
use crate::font::{Atlas, UiFont};

pub use ac_plugin_ui::fonts::{NOTO_SANS_REGULAR, NOTO_SANS_SEMIBOLD};

/// Inter Regular, compiled in.
pub static INTER_REGULAR: &[u8] = include_bytes!("../../../assets/fonts/Inter/Inter-Regular.ttf");

/// Inter Medium, compiled in.
pub static INTER_MEDIUM: &[u8] = include_bytes!("../../../assets/fonts/Inter/Inter-Medium.ttf");

/// Inter SemiBold, compiled in.
pub static INTER_SEMIBOLD: &[u8] = include_bytes!("../../../assets/fonts/Inter/Inter-SemiBold.ttf");

/// The weights of the bundled fonts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Weight {
    /// Body text.
    #[default]
    Regular,
    /// Between the two (Inter only): labels and values that stand out a
    /// little.
    Medium,
    /// Titles and headings.
    SemiBold,
}

impl Weight {
    /// The Noto Sans file of this weight (Medium is the Regular file; Noto
    /// is bundled without one).
    #[must_use]
    pub fn file(self) -> &'static [u8] {
        match self {
            Self::Regular | Self::Medium => NOTO_SANS_REGULAR,
            Self::SemiBold => NOTO_SANS_SEMIBOLD,
        }
    }

    /// Added to a bundled font's id, above the bits its size takes, so no
    /// two weights share a cache key.
    fn id_offset(self) -> u32 {
        match self {
            Self::Regular => 0,
            Self::SemiBold => 0x0100_0000,
            Self::Medium => 0x0200_0000,
        }
    }
}

/// The bundled typefaces.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Family {
    /// Noto Sans: the plugin themes' font.
    #[default]
    NotoSans,
    /// Inter: the modern interface's font.
    Inter,
}

impl Family {
    /// The font file of `weight`.
    #[must_use]
    pub fn file(self, weight: Weight) -> &'static [u8] {
        match self {
            Self::NotoSans => weight.file(),
            Self::Inter => match weight {
                Weight::Regular => INTER_REGULAR,
                Weight::Medium => INTER_MEDIUM,
                Weight::SemiBold => INTER_SEMIBOLD,
            },
        }
    }

    /// Added to a bundled font's id, above its weight's, so no two
    /// families share a cache key.
    fn id_offset(self) -> u32 {
        match self {
            Self::NotoSans => 0,
            Self::Inter => 0x0400_0000,
        }
    }

    /// Baked with its kerning and with digits of one width.
    fn typesets(self) -> bool {
        self == Self::Inter
    }
}

/// One bundled font file: a family in a weight.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct FontFace {
    /// The typeface.
    pub family: Family,
    /// The weight.
    pub weight: Weight,
}

impl FontFace {
    /// Noto Sans in `weight` (what [`bake`] bakes).
    #[must_use]
    pub const fn noto(weight: Weight) -> Self {
        Self {
            family: Family::NotoSans,
            weight,
        }
    }

    /// Inter in `weight`.
    #[must_use]
    pub const fn inter(weight: Weight) -> Self {
        Self {
            family: Family::Inter,
            weight,
        }
    }

    /// The font file.
    #[must_use]
    pub fn file(self) -> &'static [u8] {
        self.family.file(self.weight)
    }

    fn id_offset(self) -> u32 {
        self.family.id_offset() + self.weight.id_offset()
    }
}

/// The size the client loads (pixels from the highest ascender to the
/// lowest descender).
pub const DEFAULT_PIXEL_HEIGHT: f32 = 16.0;

/// The width and height of one atlas page, pixels, up to
/// [`SMALL_PAGE_LIMIT`] pixels high.
pub const ATLAS_SIZE: u32 = 512;

/// The width and height of one atlas page of a bake taller than
/// [`SMALL_PAGE_LIMIT`], pixels.
pub const LARGE_ATLAS_SIZE: u32 = 1024;

/// The tallest bake whose ranges fit [`ATLAS_SIZE`] pages, pixels.
pub const SMALL_PAGE_LIMIT: f32 = 32.0;

/// The code points baked, one page each: first and count. Latin with its
/// extensions (U+0020..U+024F), Greek and Cyrillic (U+0370..U+052F), and
/// general punctuation and super/subscripts (U+2000..U+206F).
pub const RANGES: [(u32, u32); 3] = [(32, 560), (0x370, 448), (0x2000, 112)];

/// The smallest size [`bake`] accepts, pixels.
pub const MINIMUM_PIXEL_HEIGHT: f32 = 8.0;

/// The largest size [`bake`] accepts, pixels (a sharper bake of the largest
/// interface text for a display of twice the density).
pub const MAXIMUM_PIXEL_HEIGHT: f32 = 48.0;

/// A sharper bake's scale goes up in steps this size.
const SHARP_SCALE_STEP: f32 = 0.25;

/// What a character without a glyph draws as.
pub const FALLBACK_CHARACTER: char = '?';

/// Font ids for the bundled font start here: a cache key only, never a
/// dat id (dat fonts are `0x40......`). The size in 1/256 pixels and the
/// weight's offset are added.
pub const BUNDLED_FONT_ID_BASE: u32 = 0xF000_0000;

/// Why the atlas could not be baked.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum BundledFontError {
    /// The size is outside 8..=48 pixels (or not a number).
    #[error("font size must be between 8 and 48 pixels, not {0}")]
    SizeOutOfRange(f32),
    /// The font file could not be read.
    #[error("bundled font could not be read: {0}")]
    Unreadable(String),
    /// Not even the first glyph of a range fits its page.
    #[error("bundled font atlas does not fit the requested size")]
    DoesNotFit,
    /// A glyph's metrics do not fit the dat glyph record.
    #[error("glyph U+{0:04X} does not fit a font glyph record")]
    GlyphOutOfRange(u32),
}

/// The baked atlas: one coverage byte per pixel (drawn white with that
/// alpha), the line metrics and the glyph records.
#[derive(Debug, Clone, PartialEq)]
pub struct BakedFont {
    /// Width x height bytes, the coverage, rows top to bottom.
    pub coverage: Vec<u8>,
    /// Width, pixels.
    pub width: u32,
    /// Height, pixels (one page per range).
    pub height: u32,
    /// The size baked.
    pub pixel_height: f32,
    /// The font baked.
    pub face: FontFace,
    /// Line height, pixels.
    pub line_height: f32,
    /// The baseline below the line top, pixels.
    pub ascent: f32,
    /// One record per baked code point the font has a glyph for.
    pub glyphs: Vec<FontCharDesc>,
    /// The pairs the pen moves closer (negative) or apart between, whole
    /// pixels; empty for a family baked without kerning.
    pub kerning: Vec<(char, char, i8)>,
}

impl BakedFont {
    /// The UI font over this atlas once it is a texture: missing printable
    /// characters draw as `?`.
    #[must_use]
    pub fn create_font(&self, texture: TextureRef) -> UiFont {
        UiFont::new(
            BUNDLED_FONT_ID_BASE + self.face.id_offset() + (self.pixel_height * 256.0) as u32,
            Atlas {
                texture,
                width: self.width,
                height: self.height,
            },
            None,
            self.line_height,
            self.ascent,
            self.glyphs.iter().copied(),
            0,
            0,
        )
        .with_fallback(FALLBACK_CHARACTER)
        .with_kerning(self.kerning.iter().copied())
    }
}

/// The bundled font in both weights, once its atlases are textures: what
/// the modern themes draw a window's text in.
#[derive(Debug, Clone)]
pub struct ModernFonts {
    /// Body text.
    pub regular: Arc<UiFont>,
    /// Titles and headings.
    pub semibold: Arc<UiFont>,
}

/// The tallest sharper bake the plugin themes' fonts are made at, pixels
/// (their atlases stay on small pages).
pub const PLUGIN_SHARP_LIMIT: f32 = SMALL_PAGE_LIMIT;

/// The scale a sharper bake of a plugin theme font of `pixel_height` is made
/// at for a display with `pixels_per_unit`: that density rounded down to a
/// quarter step, so each texel lands on one framebuffer pixel, and no larger
/// than [`PLUGIN_SHARP_LIMIT`] allows. 1 or below means no sharper bake.
#[must_use]
pub fn sharp_scale_for(pixel_height: f32, pixels_per_unit: f32) -> f32 {
    sharp_scale_within(pixel_height, pixels_per_unit, PLUGIN_SHARP_LIMIT)
}

/// [`sharp_scale_for`] with the tallest bake allowed given: the modern
/// interface's fonts take [`MAXIMUM_PIXEL_HEIGHT`], so its text is sharp on a
/// display of twice the density at every size it uses.
#[must_use]
pub fn sharp_scale_within(pixel_height: f32, pixels_per_unit: f32, tallest: f32) -> f32 {
    if !pixels_per_unit.is_finite() {
        return 1.0;
    }
    let limit = pixels_per_unit.min(tallest.min(MAXIMUM_PIXEL_HEIGHT) / pixel_height);
    (limit / SHARP_SCALE_STEP + 1e-3).floor() * SHARP_SCALE_STEP
}

/// Bakes Noto Sans at `pixel_height` (8..=48 pixels) in `weight`.
///
/// # Errors
///
/// The size is out of range, or the atlas or a glyph record cannot hold it.
pub fn bake(pixel_height: f32, weight: Weight) -> Result<BakedFont, BundledFontError> {
    bake_face(pixel_height, FontFace::noto(weight))
}

/// The pixel height (highest ascender to lowest descender, what [`bake`]
/// takes) of `font` set at `em` pixels to the em: what a design's font size
/// is.
///
/// # Errors
///
/// The font file could not be read.
pub fn pixel_height_for_em(font: FontFace, em: f32) -> Result<f32, BundledFontError> {
    let face =
        Face::parse(font.file(), 0).map_err(|e| BundledFontError::Unreadable(e.to_string()))?;
    let hhea = face.tables().hhea;
    let span = (i32::from(hhea.ascender) - i32::from(hhea.descender)) as f32;
    Ok(em * span / f32::from(face.units_per_em()))
}

/// The width and height of each atlas page of a bake `pixel_height` high.
#[must_use]
pub fn page_size(pixel_height: f32) -> u32 {
    if pixel_height <= SMALL_PAGE_LIMIT {
        ATLAS_SIZE
    } else {
        LARGE_ATLAS_SIZE
    }
}

/// Bakes a bundled font at `pixel_height` (8..=48 pixels).
///
/// # Errors
///
/// The size is out of range, or the atlas or a glyph record cannot hold it.
pub fn bake_face(pixel_height: f32, font: FontFace) -> Result<BakedFont, BundledFontError> {
    if !pixel_height.is_finite()
        || !(MINIMUM_PIXEL_HEIGHT..=MAXIMUM_PIXEL_HEIGHT).contains(&pixel_height)
    {
        return Err(BundledFontError::SizeOutOfRange(pixel_height));
    }
    let file = font.file();
    let page_side = page_size(pixel_height);
    let face = Face::parse(file, 0).map_err(|e| BundledFontError::Unreadable(e.to_string()))?;
    let hhea = face.tables().hhea;
    let (ascent, descent, gap) = (
        i32::from(hhea.ascender),
        i32::from(hhea.descender),
        i32::from(hhea.line_gap),
    );
    let baker = Baker::new(file, &face, pixel_height)?;
    let baseline = (ascent as f32 * baker.scale).round_ties_even();
    let page = (page_side * page_side) as usize;
    let mut coverage = vec![0u8; page * RANGES.len()];
    let mut glyphs = Vec::new();
    for (range, &(first, count)) in RANGES.iter().enumerate() {
        let page_coverage = &mut coverage[range * page..(range + 1) * page];
        let baked = baker.bake_range(page_coverage, page_side as i32, first, count)?;
        let page_top = range as i32 * page_side as i32;
        for (i, g) in baked.iter().enumerate() {
            let codepoint = first + i as u32;
            if char::from_u32(codepoint)
                .and_then(|c| face.glyph_index(c))
                .is_none()
            {
                continue;
            }
            glyphs.push(glyph_record(codepoint, g, page_top, baseline)?);
        }
    }
    let kerning = if font.family.typesets() {
        one_width_digits(&mut glyphs);
        kerning(&face, &glyphs, baker.scale)
    } else {
        Vec::new()
    };
    Ok(BakedFont {
        coverage,
        width: page_side,
        height: page_side * RANGES.len() as u32,
        pixel_height,
        face: font,
        line_height: ((ascent - descent + gap) as f32 * baker.scale).ceil(),
        ascent: baseline,
        glyphs,
        kerning,
    })
}

/// Widens every digit to the widest one's advance, the extra split either
/// side, so numbers keep their place as they change.
fn one_width_digits(glyphs: &mut [FontCharDesc]) {
    let is_digit = |g: &FontCharDesc| (u16::from(b'0')..=u16::from(b'9')).contains(&g.unicode);
    let Some(widest) = glyphs
        .iter()
        .filter(|g| is_digit(g))
        .map(|g| g.advance())
        .max()
    else {
        return;
    };
    for g in glyphs.iter_mut().filter(|g| is_digit(g)) {
        let extra = widest - g.advance();
        let before = extra / 2;
        let (Ok(new_before), Ok(new_after)) = (
            i8::try_from(i32::from(g.horizontal_offset_before) + before),
            i8::try_from(i32::from(g.horizontal_offset_after) + extra - before),
        ) else {
            continue;
        };
        g.horizontal_offset_before = new_before;
        g.horizontal_offset_after = new_after;
    }
}

/// The font's kerning among the baked characters, rounded to whole pixels;
/// pairs that round to nothing, and any with a digit (whose widths are
/// fixed), are left out.
fn kerning(face: &Face<'_>, glyphs: &[FontCharDesc], scale: f32) -> Vec<(char, char, i8)> {
    let mut by_glyph: std::collections::HashMap<u16, Vec<char>> = std::collections::HashMap::new();
    for record in glyphs {
        let Some(c) = char::from_u32(u32::from(record.unicode)) else {
            continue;
        };
        if c.is_ascii_digit() {
            continue;
        }
        if let Some(id) = face.glyph_index(c) {
            by_glyph.entry(id.0).or_default().push(c);
        }
    }
    let ids: Vec<GlyphId> = by_glyph.keys().map(|&id| GlyphId(id)).collect();
    let mut pairs = Vec::new();
    for ((left, right), units) in ac_plugin_ui::fonts::kerning_pairs(face, &ids) {
        let pixels = (units as f32 * scale).round_ties_even();
        let Ok(pixels) = i8::try_from(pixels as i32) else {
            continue;
        };
        if pixels == 0 {
            continue;
        }
        let (Some(lefts), Some(rights)) = (by_glyph.get(&left), by_glyph.get(&right)) else {
            continue;
        };
        for &a in lefts {
            for &b in rights {
                pairs.push((a, b, pixels));
            }
        }
    }
    pairs.sort_unstable();
    pairs
}

/// One baked glyph's place in its page and its placement, as the simple
/// baker records it; all zero for a glyph that did not fit.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
struct BakedChar {
    x0: i32,
    y0: i32,
    x1: i32,
    y1: i32,
    xoff: f32,
    yoff: f32,
    xadvance: f32,
}

/// The dat glyph record of one baked glyph: its place in the stacked
/// atlas, the pen movement before (the box's left edge, rounded) and after
/// (the rounded advance less the width and the movement before), and the
/// drop from the line top (baseline plus the box's top). Rounding is to
/// even, as .NET's `MathF.Round`.
fn glyph_record(
    codepoint: u32,
    g: &BakedChar,
    page_top: i32,
    baseline: f32,
) -> Result<FontCharDesc, BundledFontError> {
    let out = || BundledFontError::GlyphOutOfRange(codepoint);
    let width = g.x1 - g.x0;
    let before = g.xoff.round_ties_even() as i32;
    let after = g.xadvance.round_ties_even() as i32 - width - before;
    let down = (baseline + g.yoff.round_ties_even()) as i32;
    Ok(FontCharDesc {
        unicode: u16::try_from(codepoint).map_err(|_| out())?,
        offset_x: u16::try_from(g.x0).map_err(|_| out())?,
        offset_y: u16::try_from(g.y0 + page_top).map_err(|_| out())?,
        width: u8::try_from(width).map_err(|_| out())?,
        height: u8::try_from(g.y1 - g.y0).map_err(|_| out())?,
        horizontal_offset_before: i8::try_from(before).map_err(|_| out())?,
        horizontal_offset_after: i8::try_from(after).map_err(|_| out())?,
        vertical_offset_before: i8::try_from(down).map_err(|_| out())?,
    })
}

/// The font at one size: its tables for the boxes and advances, fontdue
/// for the coverage.
struct Baker<'a> {
    face: &'a Face<'a>,
    raster: fontdue::Font,
    /// Font units to pixels: ascender to descender is the pixel height.
    scale: f32,
    /// The same scale as fontdue takes it: pixels per em.
    em: f32,
}

impl<'a> Baker<'a> {
    fn new(file: &[u8], face: &'a Face<'a>, pixel_height: f32) -> Result<Self, BundledFontError> {
        let hhea = face.tables().hhea;
        let scale = pixel_height / (i32::from(hhea.ascender) - i32::from(hhea.descender)) as f32;
        let em = scale * f32::from(face.units_per_em());
        let raster = fontdue::Font::from_bytes(
            file,
            fontdue::FontSettings {
                collection_index: 0,
                // Outlines are flattened for the size they are drawn at.
                scale: em,
                load_substitutions: false,
            },
        )
        .map_err(|e| BundledFontError::Unreadable(e.to_owned()))?;
        Ok(Self {
            face,
            raster,
            scale,
            em,
        })
    }

    /// The pixel box a glyph's outline touches, y down, relative to the
    /// pen on the baseline: the font's own box for the glyph, scaled and
    /// rounded outwards. A glyph without an outline (a space) has an empty
    /// box.
    fn bitmap_box(&self, glyph: GlyphId) -> (i32, i32, i32, i32) {
        let Some(b) = self.face.tables().glyf.and_then(|t| t.bbox(glyph)) else {
            return (0, 0, 0, 0);
        };
        (
            (f32::from(b.x_min) * self.scale).floor() as i32,
            (-f32::from(b.y_max) * self.scale).floor() as i32,
            (f32::from(b.x_max) * self.scale).ceil() as i32,
            (-f32::from(b.y_min) * self.scale).ceil() as i32,
        )
    }

    /// Bakes `count` code points from `first` into one `size` x `size`
    /// coverage page, packing as the simple baker does. A code point the
    /// font lacks takes the missing glyph's box. When a glyph no longer
    /// fits, it and the rest of the range keep empty records (the baker
    /// reports a partial bake, which OpenAC takes for success); only a
    /// range whose first glyph does not fit fails.
    fn bake_range(
        &self,
        pixels: &mut [u8],
        size: i32,
        first: u32,
        count: u32,
    ) -> Result<Vec<BakedChar>, BundledFontError> {
        let mut baked = vec![BakedChar::default(); count as usize];
        let (mut x, mut y, mut bottom_y) = (1, 1, 1);
        for (i, slot) in baked.iter_mut().enumerate() {
            let glyph = char::from_u32(first + i as u32)
                .and_then(|c| self.face.glyph_index(c))
                .unwrap_or(GlyphId(0));
            let advance = self.face.glyph_hor_advance(glyph).unwrap_or(0);
            let (x0, y0, x1, y1) = self.bitmap_box(glyph);
            let (gw, gh) = (x1 - x0, y1 - y0);
            if x + gw + 1 >= size {
                y = bottom_y;
                x = 1;
            }
            if y + gh + 1 >= size {
                if i == 0 {
                    return Err(BundledFontError::DoesNotFit);
                }
                break;
            }
            self.draw_glyph(glyph, (x0, y0, gw, gh), pixels, size, (x, y));
            *slot = BakedChar {
                x0: x,
                y0: y,
                x1: x + gw,
                y1: y + gh,
                xoff: x0 as f32,
                yoff: y0 as f32,
                xadvance: self.scale * f32::from(advance),
            };
            x += gw + 1;
            if y + gh + 1 > bottom_y {
                bottom_y = y + gh + 1;
            }
        }
        Ok(baked)
    }

    /// Draws a glyph's coverage into its box at `at` in a page `stride`
    /// pixels wide. fontdue sizes its bitmap from the outline it flattened,
    /// which lies inside the font's box; it is placed by its own offsets
    /// and clipped to the box.
    fn draw_glyph(
        &self,
        glyph: GlyphId,
        (x0, y0, gw, gh): (i32, i32, i32, i32),
        pixels: &mut [u8],
        stride: i32,
        (at_x, at_y): (i32, i32),
    ) {
        if gw <= 0 || gh <= 0 {
            return;
        }
        let (metrics, coverage) = self.raster.rasterize_indexed(glyph.0, self.em);
        let (w, h) = (metrics.width as i32, metrics.height as i32);
        let left = metrics.xmin - x0;
        let top = -(metrics.ymin + h) - y0;
        for row in 0..h {
            let by = top + row;
            if !(0..gh).contains(&by) {
                continue;
            }
            for col in 0..w {
                let bx = left + col;
                if !(0..gw).contains(&bx) {
                    continue;
                }
                let source = (row * w + col) as usize;
                let target = ((at_y + by) * stride + at_x + bx) as usize;
                pixels[target] = coverage[source];
            }
        }
    }
}

#[cfg(test)]
mod tests;
