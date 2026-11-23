//! The detailed map's tiles under the modern map (OPENAC): which to draw
//! for the place and zoom shown, which to ask for, and which to keep.
//!
//! The world level drawn is the coarsest at least as fine as the screen;
//! under it lie the loaded tiles of up to [`BACKSTOP`] coarser levels, so a
//! square never shows nothing while its own tile loads. Past the finest
//! world level, the towns' finer tiles are laid over where there are any.
//! Asked for: the backstop first, then the level shown, the ones between,
//! then the towns', each nearest the middle first. At most [`HELD`] tiles
//! are kept as textures; the least recently drawn go first.

use std::collections::HashMap;
use std::sync::Arc;

use ac_dat::texture::DecodedTexture;
use ac_ui::draw::TextureRef;
use ac_world_map::index::coordinates;
use ac_world_map::{LevelRef, Loaded, MapIndex, Tier, TileKey, WorldMaps};

use crate::ui::icons::RuntimeImages;

/// Coarser world levels drawn under the one shown.
pub const BACKSTOP: u8 = 3;
/// The most tiles kept as textures.
pub const HELD: usize = 288;
/// The most tiles made textures in one frame (the rest wait a frame).
pub const UPLOADS_PER_FRAME: usize = 24;

/// A tile to draw: its texture and its square in map coordinates.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TileDraw {
    /// The texture.
    pub texture: TextureRef,
    /// West and east (map `x`), north and south (map `y`).
    pub west: f64,
    pub north: f64,
    pub east: f64,
    pub south: f64,
    /// Half a pixel of the tile, as a share of its side: drawn from that
    /// far in, so its edge never samples what lies beside it.
    pub inset: f32,
}

/// What the map shows, in world metres: west, north, east, south, and the
/// screen's pixels a metre.
pub type ViewMetres = ((f64, f64, f64, f64), f64);

#[derive(Debug, Clone, Copy)]
struct Held {
    texture: TextureRef,
    used: u64,
}

/// The tiles held for the map.
#[derive(Debug, Default)]
pub struct MapTiles {
    tier: Option<Tier>,
    index: Option<Arc<MapIndex>>,
    held: HashMap<TileKey, Held>,
    frame: u64,
    last_want: Vec<TileKey>,
    /// Tiles read and not yet made textures.
    waiting: Vec<Loaded>,
}

/// The levels to draw at `screen` pixels a metre, bottom first, and the
/// order to ask for them in.
#[must_use]
pub fn plan(index: &MapIndex, screen: f64) -> (Vec<LevelRef>, Vec<LevelRef>) {
    let shown = index.world_level(screen);
    let coarsest = u8::try_from(index.world.len().saturating_sub(1)).unwrap_or(0);
    let backstop = shown.saturating_add(BACKSTOP).min(coarsest);
    let draw: Vec<LevelRef> = (shown..=backstop)
        .rev()
        .map(LevelRef::World)
        .chain(index.town_level(screen).map(LevelRef::Town))
        .collect();
    let mut ask = vec![LevelRef::World(backstop), LevelRef::World(shown)];
    ask.extend((shown + 1..backstop).rev().map(LevelRef::World));
    ask.extend(index.town_level(screen).map(LevelRef::Town));
    ask.dedup();
    (draw, ask)
}

impl MapTiles {
    /// No tiles.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The tier drawn.
    #[must_use]
    pub fn tier(&self) -> Option<Tier> {
        self.tier
    }

    /// The finest detail the tier drawn has, pixels a metre.
    #[must_use]
    pub fn finest(&self) -> Option<f64> {
        self.index.as_ref().map(|i| i.finest_pixels_per_m())
    }

    /// Lets go of every tile.
    pub fn release(&mut self, images: &RuntimeImages) {
        for (_, h) in self.held.drain() {
            images.release(h.texture);
        }
        self.last_want.clear();
        self.waiting.clear();
    }

    /// The map closed: every tile let go and nothing more asked for.
    pub fn close(&mut self, maps: &mut dyn WorldMaps, images: &RuntimeImages) {
        if self.held.is_empty() && self.last_want.is_empty() {
            return;
        }
        if let Some(tier) = self.tier {
            maps.want(tier, &[]);
        }
        self.release(images);
    }

    /// One frame while the map shows: `tier` is drawn (none: the game's
    /// map alone) over `view`; what to draw, bottom first.
    pub fn tick(
        &mut self,
        maps: &mut dyn WorldMaps,
        images: &RuntimeImages,
        tier: Option<Tier>,
        view: Option<ViewMetres>,
    ) -> Vec<TileDraw> {
        self.frame += 1;
        if tier != self.tier || (tier.is_some() && self.index.is_none()) {
            self.release(images);
            maps.want(tier.unwrap_or(Tier::Standard), &[]);
            self.tier = tier;
            self.index = tier.and_then(|t| maps.index(t));
        }
        let (Some(tier), Some(index)) = (self.tier, self.index.clone()) else {
            return Vec::new();
        };
        self.waiting.extend(maps.take_loaded());
        let now = self.waiting.len().min(UPLOADS_PER_FRAME);
        for l in self.waiting.drain(..now).collect::<Vec<_>>() {
            if l.tier != tier || self.held.contains_key(&l.key) {
                continue;
            }
            let Some(p) = l.pixels else {
                continue;
            };
            let texture = images.add_plugin_image(vec![DecodedTexture {
                width: p.width,
                height: p.height,
                rgba: p.rgba,
            }]);
            self.held.insert(
                l.key,
                Held {
                    texture,
                    used: self.frame,
                },
            );
        }
        let Some(((west, north, east, south), screen)) = view else {
            return Vec::new();
        };
        let middle = ((west + east) * 0.5, (north + south) * 0.5);
        let (draw_levels, ask_levels) = plan(&index, screen);
        let mut out = Vec::new();
        for level in draw_levels {
            for key in index.tiles_over(level, (west, north, east, south), middle) {
                let Some(h) = self.held.get_mut(&key) else {
                    continue;
                };
                h.used = self.frame;
                let Some((w, n, e, s)) = index.tile_metres(key) else {
                    continue;
                };
                let (x0, y0) = coordinates(w, n);
                let (x1, y1) = coordinates(e, s);
                out.push(TileDraw {
                    texture: h.texture,
                    west: x0,
                    north: y0,
                    east: x1,
                    south: y1,
                    inset: 0.5 / index.tile_size as f32,
                });
            }
        }
        let want: Vec<TileKey> = ask_levels
            .into_iter()
            .flat_map(|level| index.tiles_over(level, (west, north, east, south), middle))
            .filter(|k| !self.held.contains_key(k))
            .collect();
        if want != self.last_want {
            maps.want(tier, &want);
            self.last_want = want;
        }
        self.trim(images);
        out
    }

    /// Lets the least recently drawn tiles go past [`HELD`].
    fn trim(&mut self, images: &RuntimeImages) {
        if self.held.len() <= HELD {
            return;
        }
        let mut by_use: Vec<(u64, TileKey)> = self.held.iter().map(|(k, h)| (h.used, *k)).collect();
        by_use.sort_unstable();
        let extra = self.held.len() - HELD;
        for (used, key) in by_use.into_iter().take(extra) {
            if used == self.frame {
                break;
            }
            if let Some(h) = self.held.remove(&key) {
                images.release(h.texture);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ac_world_map::index::WORLD_M;
    use ac_world_map::{MapsStatus, Pixels};

    /// A service with one tier whose index is given, loading every tile
    /// asked for at once.
    #[derive(Debug)]
    struct Fake {
        index: Arc<MapIndex>,
        asked: Vec<TileKey>,
        loaded: Vec<Loaded>,
    }

    impl WorldMaps for Fake {
        fn status(&self) -> MapsStatus {
            MapsStatus::empty()
        }
        fn revision(&self) -> u64 {
            0
        }
        fn ask(&mut self) {}
        fn download(&mut self, _: Tier) {}
        fn cancel(&mut self) {}
        fn remove(&mut self, _: Tier) {}
        fn index(&self, _: Tier) -> Option<Arc<MapIndex>> {
            Some(self.index.clone())
        }
        fn want(&mut self, tier: Tier, keys: &[TileKey]) {
            self.asked = keys.to_vec();
            self.loaded.extend(keys.iter().map(|k| Loaded {
                tier,
                key: *k,
                pixels: Some(Pixels {
                    width: 1,
                    height: 1,
                    rgba: vec![0; 4],
                }),
            }));
        }
        fn take_loaded(&mut self) -> Vec<Loaded> {
            std::mem::take(&mut self.loaded)
        }
    }

    fn index() -> MapIndex {
        let doc = br#"{"tile_size":256,"levels":[
            {"pixels_per_cell":4,"cols":32,"rows":32,"every":"w/0/{c}_{r}.webp"},
            {"pixels_per_cell":2,"cols":16,"rows":16,"every":"w/1/{c}_{r}.webp"},
            {"pixels_per_cell":1,"cols":8,"rows":8,"every":"w/2/{c}_{r}.webp"},
            {"pixels_per_cell":0.5,"cols":4,"rows":4,"every":"w/3/{c}_{r}.webp"},
            {"pixels_per_cell":0.25,"cols":2,"rows":2,"every":"w/4/{c}_{r}.webp"},
            {"pixels_per_cell":0.125,"cols":1,"rows":1,"every":"w/5/{c}_{r}.webp"}
        ],"towns":{"levels":[{"pixels_per_cell":8,"cols":64,"rows":64,"tiles":[
            {"c":30,"r":30,"file":"t/30_30.webp"}]}]}}"#;
        MapIndex::parse(doc).unwrap()
    }

    #[test]
    fn the_plan_lays_coarser_levels_under_the_one_shown_and_asks_the_backstop_first() {
        let i = index();
        // 3 pixels a cell: level 0 (4) is shown, 3 (0.5) is the backstop.
        let (draw, ask) = plan(&i, 3.0 / 24.0);
        assert_eq!(
            draw,
            vec![
                LevelRef::World(3),
                LevelRef::World(2),
                LevelRef::World(1),
                LevelRef::World(0)
            ]
        );
        assert_eq!(ask[0], LevelRef::World(3));
        assert_eq!(ask[1], LevelRef::World(0));
        // Past the finest world level, the towns' tiles go on top.
        let (draw, ask) = plan(&i, 6.0 / 24.0);
        assert_eq!(draw.last(), Some(&LevelRef::Town(0)));
        assert_eq!(ask.last(), Some(&LevelRef::Town(0)));
        // The whole world on screen: the coarsest level alone.
        let (draw, _) = plan(&i, 0.01 / 24.0);
        assert_eq!(draw, vec![LevelRef::World(5)]);
    }

    #[test]
    fn tiles_come_in_are_drawn_where_they_lie_and_go_with_the_tier() {
        let images = RuntimeImages::new();
        let mut maps = Fake {
            index: Arc::new(index()),
            asked: Vec::new(),
            loaded: Vec::new(),
        };
        let mut tiles = MapTiles::new();
        // The whole world on 512 screen pixels.
        let view = Some(((0.0, WORLD_M, WORLD_M, 0.0), 512.0 / WORLD_M));
        // The first frame asks; the next draws what came.
        assert!(
            tiles
                .tick(&mut maps, &images, Some(Tier::Hd), view)
                .is_empty()
        );
        assert!(!maps.asked.is_empty());
        let drawn = tiles.tick(&mut maps, &images, Some(Tier::Hd), view);
        assert!(!drawn.is_empty());
        // The one-tile level covers the world from its north-west corner.
        let top = drawn[0];
        assert!((top.west - coordinates(0.0, 0.0).0).abs() < 1e-9);
        assert!((top.north - coordinates(0.0, WORLD_M).1).abs() < 1e-9);
        // Nothing more to ask.
        maps.asked.clear();
        tiles.tick(&mut maps, &images, Some(Tier::Hd), view);
        assert!(maps.asked.is_empty());
        // The game's map: every tile let go.
        let before = images.take_released().len();
        assert!(tiles.tick(&mut maps, &images, None, view).is_empty());
        assert!(images.take_released().len() > before);
    }
}
