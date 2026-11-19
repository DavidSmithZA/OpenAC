//! The modern interface's text: Inter in its three weights, baked the first
//! time a piece asks for a weight at a size and kept for the window's life.
//! Sizes are the design's (pixels to the em, as `Horizon`'s text sizes);
//! each bake is also made sharp for a high-density display.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use ac_ui::UiFont;
use ac_ui::bundled_font::{self, BundledFontError, FontFace, Weight};

/// A design size as a key: quarter pixels to the em.
fn size_key(em: f32) -> u32 {
    (em * 4.0).round().max(0.0) as u32
}

/// One baked font: its face, the pixel height it was baked at, the font.
type Baked = (FontFace, f32, Arc<UiFont>);

/// The Inter fonts baked so far.
#[derive(Debug, Default)]
pub struct HorizonFonts {
    baked: HashMap<(Weight, u32), Baked>,
    /// Sizes that could not be baked: asked again, they are not tried
    /// again.
    failed: HashSet<(Weight, u32)>,
    /// Why, not yet taken.
    failures: Vec<String>,
}

impl HorizonFonts {
    /// Inter in `weight` at `em` pixels to the em: the kept bake, or one
    /// `bake` makes now (given the pixel height and the face). None when the
    /// size cannot be baked.
    pub fn get(
        &mut self,
        weight: Weight,
        em: f32,
        bake: impl FnOnce(f32, FontFace) -> Result<Arc<UiFont>, BundledFontError>,
    ) -> Option<Arc<UiFont>> {
        let key = (weight, size_key(em));
        if let Some((_, _, font)) = self.baked.get(&key) {
            return Some(font.clone());
        }
        if self.failed.contains(&key) {
            return None;
        }
        let face = FontFace::inter(weight);
        let made = bundled_font::pixel_height_for_em(face, em)
            .and_then(|height| bake(height, face).map(|font| (height, font)));
        match made {
            Ok((height, font)) => {
                self.baked.insert(key, (face, height, font.clone()));
                Some(font)
            }
            Err(error) => {
                self.failed.insert(key);
                self.failures
                    .push(format!("ui: Inter {weight:?} at {em} px: {error}"));
                None
            }
        }
    }

    /// Every font baked: its face, pixel height and the font.
    pub fn baked(&self) -> impl Iterator<Item = Baked> + '_ {
        self.baked.values().cloned()
    }

    /// Why sizes could not be baked since the last call.
    pub fn take_failures(&mut self) -> Vec<String> {
        std::mem::take(&mut self.failures)
    }
}

#[cfg(test)]
mod tests {
    use ac_ui::draw::TextureRef;

    use super::*;

    fn bake(height: f32, face: FontFace) -> Result<Arc<UiFont>, BundledFontError> {
        let atlas = bundled_font::bake_face(height, face)?;
        Ok(Arc::new(atlas.create_font(TextureRef::Runtime(1))))
    }

    #[test]
    fn a_size_is_baked_once() {
        let mut fonts = HorizonFonts::default();
        let mut bakes = 0;
        let mut counted = |height, face| {
            bakes += 1;
            bake(height, face)
        };
        let first = fonts.get(Weight::Medium, 12.0, &mut counted).unwrap();
        let again = fonts.get(Weight::Medium, 12.0, &mut counted).unwrap();
        assert!(Arc::ptr_eq(&first, &again));
        fonts.get(Weight::SemiBold, 12.0, &mut counted).unwrap();
        assert_eq!(bakes, 2);
        assert_eq!(fonts.baked().count(), 2);
    }

    #[test]
    fn a_size_too_big_is_refused_once() {
        let mut fonts = HorizonFonts::default();
        let mut bakes = 0;
        let mut counted = |height, face| {
            bakes += 1;
            bake(height, face)
        };
        assert!(fonts.get(Weight::Regular, 80.0, &mut counted).is_none());
        assert!(fonts.get(Weight::Regular, 80.0, &mut counted).is_none());
        assert_eq!(bakes, 1);
        assert_eq!(fonts.take_failures().len(), 1);
        assert!(fonts.take_failures().is_empty());
    }
}
