//! The backdrops behind the pre-game screens (OPENAC): pictures of the
//! world, made from the client's own views of it, shown one at a time in a
//! random order, each for half a minute with a slow push in, the next
//! fading in over it. Only the picture shown (and the one fading out) is
//! decoded and held; all of them are let go while no pre-game screen
//! shows. A build without them (the page) shows the drawn skies instead.

use ac_clock::Instant;

use ac_dat::texture::DecodedTexture;
use ac_runtime::session::creation::{ChargenRandom, SeededRandom};
use ac_ui::draw::{Color, RenderContext, TextureRef};

use crate::ui::icons::RuntimeImages;

/// How long each picture shows, seconds.
pub const SHOWN_FOR: f32 = 30.0;
/// How long the next one takes to fade in, seconds.
pub const FADE: f32 = 1.5;
/// How far a picture pushes in while it shows (1.0: none).
const PUSH_IN: f32 = 1.05;

/// The pictures, as bundled: name and WebP bytes.
#[cfg(not(target_arch = "wasm32"))]
const PICTURES: &[(&str, &[u8])] = &[
    (
        "aluvian_holtburg_river_day",
        include_bytes!("../../../../../../assets/backdrops/aluvian_holtburg_river_day.webp"),
    ),
    (
        "aluvian_arwic_walls_day",
        include_bytes!("../../../../../../assets/backdrops/aluvian_arwic_walls_day.webp"),
    ),
    (
        "dusk_holtburg_river",
        include_bytes!("../../../../../../assets/backdrops/dusk_holtburg_river.webp"),
    ),
    (
        "unknown_mountain_range",
        include_bytes!("../../../../../../assets/backdrops/unknown_mountain_range.webp"),
    ),
    (
        "unknown_mountain_range_night",
        include_bytes!("../../../../../../assets/backdrops/unknown_mountain_range_night.webp"),
    ),
    (
        "studio_sky_day",
        include_bytes!("../../../../../../assets/backdrops/studio_sky_day.webp"),
    ),
    (
        "desert_yaraq_day",
        include_bytes!("../../../../../../assets/backdrops/desert_yaraq_day.webp"),
    ),
    (
        "desert_yaraq_dusk",
        include_bytes!("../../../../../../assets/backdrops/desert_yaraq_dusk.webp"),
    ),
    (
        "desert_zaikhal_mesas",
        include_bytes!("../../../../../../assets/backdrops/desert_zaikhal_mesas.webp"),
    ),
    (
        "sho_shoushi_day",
        include_bytes!("../../../../../../assets/backdrops/sho_shoushi_day.webp"),
    ),
    (
        "sho_hebianto_mountains",
        include_bytes!("../../../../../../assets/backdrops/sho_hebianto_mountains.webp"),
    ),
    (
        "sho_nanto_pagodas",
        include_bytes!("../../../../../../assets/backdrops/sho_nanto_pagodas.webp"),
    ),
    (
        "sho_lin_peak_dusk",
        include_bytes!("../../../../../../assets/backdrops/sho_lin_peak_dusk.webp"),
    ),
    (
        "coast_mayoi",
        include_bytes!("../../../../../../assets/backdrops/coast_mayoi.webp"),
    ),
    (
        "coast_kryst_isle",
        include_bytes!("../../../../../../assets/backdrops/coast_kryst_isle.webp"),
    ),
];

/// Where each picture's subject is, as shares of its width and height:
/// a screen of another shape keeps the part around it (the centre for a
/// picture not listed).
const FOCUS: &[(&str, (f32, f32))] = &[
    ("aluvian_holtburg_river_day", (0.55, 0.55)),
    ("dusk_holtburg_river", (0.55, 0.55)),
    ("aluvian_arwic_walls_day", (0.52, 0.58)),
    ("unknown_mountain_range", (0.50, 0.40)),
    ("unknown_mountain_range_night", (0.50, 0.40)),
    ("studio_sky_day", (0.50, 0.45)),
    ("desert_yaraq_day", (0.52, 0.45)),
    ("desert_yaraq_dusk", (0.50, 0.45)),
    ("desert_zaikhal_mesas", (0.42, 0.45)),
    ("sho_shoushi_day", (0.45, 0.55)),
    ("sho_hebianto_mountains", (0.42, 0.50)),
    ("sho_nanto_pagodas", (0.62, 0.55)),
    ("sho_lin_peak_dusk", (0.48, 0.45)),
    ("coast_mayoi", (0.55, 0.50)),
    ("coast_kryst_isle", (0.55, 0.45)),
];

/// Where the picture `name`'s subject is (shares of its width and height).
#[must_use]
pub fn focus(name: &str) -> (f32, f32) {
    FOCUS
        .iter()
        .find(|(n, _)| *n == name)
        .map_or((0.5, 0.5), |(_, f)| *f)
}

/// The part of a `picture`-sized picture a `screen`-sized one shows,
/// pushed in by `zoom`: the screen's shape, as large as fits, around
/// `focus` and inside the picture (u0, v0, u1, v1).
#[must_use]
pub fn cover(
    picture: (f32, f32),
    screen: (f32, f32),
    zoom: f32,
    focus: (f32, f32),
) -> (f32, f32, f32, f32) {
    let ((pw, ph), (w, h)) = (picture, screen);
    let (mut uw, mut vh) = if pw / ph > w / h {
        ((w / h) * ph / pw, 1.0)
    } else {
        (1.0, (h / w) * pw / ph)
    };
    uw /= zoom.max(1.0);
    vh /= zoom.max(1.0);
    let u0 = (focus.0 - uw * 0.5).clamp(0.0, 1.0 - uw);
    let v0 = (focus.1 - vh * 0.5).clamp(0.0, 1.0 - vh);
    (u0, v0, u0 + uw, v0 + vh)
}

/// The page carries no pictures (they would make it megabytes larger).
#[cfg(target_arch = "wasm32")]
const PICTURES: &[(&str, &[u8])] = &[];

/// Where the bundled picture at `index` has its subject.
fn focus_of(index: usize) -> (f32, f32) {
    PICTURES
        .get(index)
        .map_or((0.5, 0.5), |(name, _)| focus(name))
}

/// The bundled pictures' names, in their bundled order.
#[must_use]
pub fn names() -> Vec<&'static str> {
    PICTURES.iter().map(|(name, _)| *name).collect()
}

/// A picture decoded and held: its texture and size.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Held {
    index: usize,
    texture: TextureRef,
    size: (f32, f32),
    /// When it started to show, seconds on the slideshow's clock.
    since: f32,
}

/// One picture to draw: its texture, the part of it shown and how opaque.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Layer {
    /// The texture.
    pub texture: TextureRef,
    /// The picture's size, pixels.
    pub size: (f32, f32),
    /// How far it has pushed in (1.0: not at all).
    pub zoom: f32,
    /// Where its subject is (see [`focus`]).
    pub focus: (f32, f32),
    /// Its opacity.
    pub alpha: f32,
}

/// What a screen draws behind it this frame: up to two pictures, the one
/// fading out under the one fading in. Empty: the drawn sky.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Backdrop {
    /// The pictures, bottom first.
    pub layers: Vec<Layer>,
}

impl Backdrop {
    /// Draws the pictures over `w` by `h`, each covering it (cut to fit,
    /// centred). False when there is nothing to draw.
    pub fn draw(&self, rc: &mut RenderContext<'_>, w: f32, h: f32) -> bool {
        if self.layers.is_empty() || w <= 0.0 || h <= 0.0 {
            return false;
        }
        for l in &self.layers {
            let (pw, ph) = l.size;
            if pw <= 0.0 || ph <= 0.0 {
                continue;
            }
            // Cover: the share of the picture that fits the screen's shape,
            // made smaller by the push in, around the subject.
            let uv = cover((pw, ph), (w, h), l.zoom, l.focus);
            rc.sprite(
                l.texture,
                0.0,
                0.0,
                w,
                h,
                uv,
                Color {
                    a: l.alpha.clamp(0.0, 1.0),
                    ..Color::WHITE
                },
            );
        }
        true
    }
}

/// The slideshow.
#[derive(Debug)]
pub struct Slideshow {
    random: SeededRandom,
    clock: Option<Instant>,
    showing: Option<Held>,
    leaving: Option<Held>,
    /// Pictures that would not decode (never asked for again).
    broken: Vec<usize>,
}

impl Default for Slideshow {
    fn default() -> Self {
        Self {
            random: SeededRandom::from_clock(),
            clock: None,
            showing: None,
            leaving: None,
            broken: Vec::new(),
        }
    }
}

impl Slideshow {
    /// No picture held yet.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Whether any pictures are bundled.
    #[must_use]
    pub fn has_pictures() -> bool {
        !PICTURES.is_empty()
    }

    /// The name of the picture showing.
    #[must_use]
    pub fn showing(&self) -> Option<&'static str> {
        self.showing
            .and_then(|h| PICTURES.get(h.index))
            .map(|p| p.0)
    }

    /// One frame while a pre-game screen shows: the next picture comes in
    /// when this one has had its time; what to draw. `now` is the
    /// window's clock.
    pub fn tick(&mut self, images: &RuntimeImages, now: Instant) -> Backdrop {
        let start = *self.clock.get_or_insert(now);
        let t = now.duration_since(start).as_secs_f32();
        let due = self.showing.is_none_or(|s| t - s.since >= SHOWN_FOR);
        if due && let Some(next) = self.load_next(images, t) {
            if let Some(old) = self.leaving.take() {
                images.release(old.texture);
            }
            self.leaving = self.showing.replace(next);
        }
        // The one fading out goes once the new one is in.
        if let (Some(s), Some(old)) = (self.showing, self.leaving)
            && t - s.since >= FADE
        {
            images.release(old.texture);
            self.leaving = None;
        }
        let zoom =
            |h: &Held| 1.0 + (PUSH_IN - 1.0) * ((t - h.since) / (SHOWN_FOR + FADE)).clamp(0.0, 1.0);
        let mut layers = Vec::with_capacity(2);
        if let Some(old) = &self.leaving {
            layers.push(Layer {
                texture: old.texture,
                size: old.size,
                zoom: zoom(old),
                focus: focus_of(old.index),
                alpha: 1.0,
            });
        }
        if let Some(s) = &self.showing {
            let fade_in = if self.leaving.is_some() {
                ((t - s.since) / FADE).clamp(0.0, 1.0)
            } else {
                1.0
            };
            layers.push(Layer {
                texture: s.texture,
                size: s.size,
                zoom: zoom(s),
                focus: focus_of(s.index),
                alpha: fade_in * fade_in * (3.0 - 2.0 * fade_in),
            });
        }
        Backdrop { layers }
    }

    /// Lets go of every picture (no pre-game screen shows); the next show
    /// starts with a fresh pick.
    pub fn release(&mut self, images: &RuntimeImages) {
        for h in [self.showing.take(), self.leaving.take()]
            .into_iter()
            .flatten()
        {
            images.release(h.texture);
        }
        self.clock = None;
    }

    /// Decodes a picture other than the one showing, picked at random.
    fn load_next(&mut self, images: &RuntimeImages, since: f32) -> Option<Held> {
        let candidates: Vec<usize> = (0..PICTURES.len())
            .filter(|i| !self.broken.contains(i) && self.showing.is_none_or(|s| s.index != *i))
            .collect();
        if candidates.is_empty() {
            return None;
        }
        let max = u32::try_from(candidates.len()).unwrap_or(u32::MAX);
        let index = candidates[self.random.next(max) as usize % candidates.len()];
        let Some((width, height, rgba)) = decode(PICTURES[index].1) else {
            self.broken.push(index);
            return None;
        };
        let levels = ac_plugin_ui::mips::build(rgba, width, height)
            .into_iter()
            .map(|l| DecodedTexture {
                width: l.width,
                height: l.height,
                rgba: l.rgba,
            })
            .collect();
        Some(Held {
            index,
            texture: images.add_plugin_image(levels),
            size: (width as f32, height as f32),
            since,
        })
    }
}

/// A bundled WebP as RGBA pixels.
fn decode(bytes: &[u8]) -> Option<(u32, u32, Vec<u8>)> {
    let mut decoder = image_webp::WebPDecoder::new(std::io::Cursor::new(bytes)).ok()?;
    let (width, height) = decoder.dimensions();
    let mut pixels = vec![0; decoder.output_buffer_size()?];
    decoder.read_image(&mut pixels).ok()?;
    let rgba = if decoder.has_alpha() {
        pixels
    } else {
        pixels
            .chunks_exact(3)
            .flat_map(|rgb| [rgb[0], rgb[1], rgb[2], 255])
            .collect()
    };
    Some((width, height, rgba))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_screen_of_another_shape_keeps_the_part_around_the_subject() {
        let picture = (3840.0, 2160.0);
        // 16:9 on 16:9: all of it, whatever the focus.
        assert_eq!(
            cover(picture, (1920.0, 1080.0), 1.0, (0.62, 0.55)),
            (0.0, 0.0, 1.0, 1.0)
        );
        // 4:3 keeps three quarters of the width, around the subject.
        let (u0, v0, u1, v1) = cover(picture, (1024.0, 768.0), 1.0, (0.62, 0.55));
        assert!((u1 - u0 - 0.75).abs() < 1e-4 && v0 == 0.0 && v1 == 1.0);
        assert!((u0 - 0.245).abs() < 1e-4, "{u0}");
        // A subject near the edge: the cut stops at the picture's edge.
        let (u0, _, u1, _) = cover(picture, (1024.0, 768.0), 1.0, (0.95, 0.5));
        assert!((u1 - 1.0).abs() < 1e-4 && (u0 - 0.25).abs() < 1e-4);
        // 21:9 keeps the width and part of the height, around the subject.
        let (u0, v0, u1, v1) = cover(picture, (2560.0, 1080.0), 1.0, (0.5, 0.40));
        assert!(u0 == 0.0 && u1 == 1.0);
        assert!(((v0 + v1) * 0.5 - 0.40).abs() < 1e-4 && v1 - v0 < 1.0);
        // Every bundled picture has its subject listed.
        for name in names() {
            assert!(FOCUS.iter().any(|(n, _)| *n == name), "{name}");
        }
    }

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn every_bundled_picture_decodes_at_its_size() {
        for (name, bytes) in PICTURES {
            let (w, h, rgba) = decode(bytes).unwrap_or_else(|| panic!("{name} decodes"));
            assert_eq!((w, h), (3840, 2160), "{name}");
            assert_eq!(rgba.len(), 3840 * 2160 * 4, "{name}");
        }
    }

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn a_picture_shows_for_its_time_then_the_next_fades_in_over_it() {
        let images = RuntimeImages::new();
        let mut show = Slideshow::new();
        let t0 = Instant::now();
        let first = show.tick(&images, t0);
        assert_eq!(first.layers.len(), 1);
        assert!((first.layers[0].alpha - 1.0).abs() < 1e-6);
        let name = show.showing();
        // Half way through its time: still alone, pushed in a little.
        let mid = show.tick(&images, t0 + std::time::Duration::from_secs(15));
        assert_eq!(mid.layers.len(), 1);
        assert!(mid.layers[0].zoom > 1.0);
        // Its time is up: another picture fades in over it.
        let swap = show.tick(&images, t0 + std::time::Duration::from_secs(30));
        assert_eq!(swap.layers.len(), 2);
        assert!(swap.layers[1].alpha < 0.01);
        assert_ne!(show.showing(), name);
        // Faded in: the old one is let go.
        let after = show.tick(&images, t0 + std::time::Duration::from_secs(32));
        assert_eq!(after.layers.len(), 1);
        assert_eq!(images.take_released().len(), 1);
        // Released: nothing is held.
        show.release(&images);
        assert_eq!(images.take_released().len(), 1);
    }
}
