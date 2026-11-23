//! A tier's index: its levels and which tiles each has.
//!
//! Every level is laid out the same way: pixel (0, 0) is the world's
//! north-west corner, x runs east from landblock column 0's west edge and
//! y runs south from landblock row 254's north edge, at the level's pixels
//! a 24 m cell; past the world is clear. World levels cover everything,
//! each half the detail of the one before, down to one tile; town levels
//! are finer and hold only the tiles around the towns.
//!
//! Map coordinates (`x` east, `y` north, the game's 0.1 a cell) are
//! `x * 240 + 24468` metres from the west edge and `y * 240 + 24468` from
//! the south edge.

use std::collections::HashMap;

use ac_host_core::json::{Node, Value};

use crate::manifest::{ManifestError, number, text};
use crate::part::check_path;

/// A cell's side, metres.
pub const CELL_M: f64 = 24.0;
/// The world's side, metres (2040 cells).
pub const WORLD_M: f64 = 2040.0 * CELL_M;
/// Map coordinate 0 from the west (or south) edge, metres.
pub const CENTRE_M: f64 = 24_468.0;
/// The most levels of either kind an index may hold.
const MAX_LEVELS: usize = 16;
/// The most tiles a listed level may name.
const MAX_LISTED: usize = 200_000;

/// World metres (east from the west edge, north from the south edge) of
/// map coordinates.
#[must_use]
pub fn metres(x: f64, y: f64) -> (f64, f64) {
    (x * 240.0 + CENTRE_M, y * 240.0 + CENTRE_M)
}

/// Map coordinates of world metres.
#[must_use]
pub fn coordinates(east_m: f64, north_m: f64) -> (f64, f64) {
    ((east_m - CENTRE_M) / 240.0, (north_m - CENTRE_M) / 240.0)
}

/// Which level: a world level or a town level, by its place in
/// [`MapIndex::world`] or [`MapIndex::towns`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum LevelRef {
    /// A world level (0 the finest).
    World(u8),
    /// A town level (0 the coarsest).
    Town(u8),
}

/// One tile of a level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TileKey {
    /// Its level.
    pub level: LevelRef,
    /// Its column (from the west) and row (from the north).
    pub c: u32,
    pub r: u32,
}

/// Where a level's tiles are.
#[derive(Debug, Clone, PartialEq)]
enum Tiles {
    /// Every tile of the grid, at the pattern with `{c}` and `{r}` filled.
    Every(String),
    /// Only these.
    Listed(HashMap<(u32, u32), String>),
}

/// One level.
#[derive(Debug, Clone, PartialEq)]
pub struct Level {
    /// Pixels a 24 m cell.
    pub pixels_per_cell: f64,
    /// Its grid.
    pub cols: u32,
    pub rows: u32,
    tiles: Tiles,
}

impl Level {
    /// Pixels a metre.
    #[must_use]
    pub fn pixels_per_m(&self) -> f64 {
        self.pixels_per_cell / CELL_M
    }

    /// The path of tile (`c`, `r`) in the tier's parts, when it has one.
    #[must_use]
    pub fn path(&self, c: u32, r: u32) -> Option<String> {
        if c >= self.cols || r >= self.rows {
            return None;
        }
        match &self.tiles {
            Tiles::Every(pattern) => Some(fill(pattern, c, r)),
            Tiles::Listed(list) => list.get(&(c, r)).cloned(),
        }
    }

    /// Whether it has tile (`c`, `r`).
    #[must_use]
    pub fn has(&self, c: u32, r: u32) -> bool {
        c < self.cols
            && r < self.rows
            && match &self.tiles {
                Tiles::Every(_) => true,
                Tiles::Listed(list) => list.contains_key(&(c, r)),
            }
    }

    /// How many tiles it has.
    #[must_use]
    pub fn count(&self) -> usize {
        match &self.tiles {
            Tiles::Every(_) => self.cols as usize * self.rows as usize,
            Tiles::Listed(list) => list.len(),
        }
    }
}

fn fill(pattern: &str, c: u32, r: u32) -> String {
    pattern
        .replace("{c}", &c.to_string())
        .replace("{r}", &r.to_string())
}

/// A tier's levels.
#[derive(Debug, Clone, PartialEq)]
pub struct MapIndex {
    /// A tile's side, pixels.
    pub tile_size: u32,
    /// The world levels, finest first.
    pub world: Vec<Level>,
    /// The town levels, coarsest first.
    pub towns: Vec<Level>,
}

impl MapIndex {
    /// The index in a manifest's `map` member.
    ///
    /// # Errors
    /// A member missing or of the wrong kind, a pattern or path that is not
    /// allowed, or no world level.
    pub fn read(map: &Node) -> Result<Self, ManifestError> {
        let tile_size = u32::try_from(number(map, "tile_size")?.round() as i64)
            .ok()
            .filter(|s| (16..=4096).contains(s))
            .ok_or(ManifestError::Bad("tile_size"))?;
        let mut world = levels(member(map, "levels").ok_or(ManifestError::Missing("levels"))?)?;
        if world.is_empty() {
            return Err(ManifestError::Bad("levels"));
        }
        let mut towns = match member(map, "towns").and_then(|t| member(t, "levels")) {
            Some(l) => levels(l)?,
            None => Vec::new(),
        };
        world.sort_by(|a, b| b.pixels_per_cell.total_cmp(&a.pixels_per_cell));
        towns.sort_by(|a, b| a.pixels_per_cell.total_cmp(&b.pixels_per_cell));
        Ok(Self {
            tile_size,
            world,
            towns,
        })
    }

    /// The index in a JSON document (a manifest's `map` member alone).
    ///
    /// # Errors
    /// Not JSON, or as [`MapIndex::read`].
    pub fn parse(bytes: &[u8]) -> Result<Self, ManifestError> {
        let doc = ac_host_core::json::parse(bytes).map_err(|_| ManifestError::NotJson)?;
        Self::read(&doc)
    }

    /// A level.
    #[must_use]
    pub fn level(&self, level: LevelRef) -> Option<&Level> {
        match level {
            LevelRef::World(i) => self.world.get(usize::from(i)),
            LevelRef::Town(i) => self.towns.get(usize::from(i)),
        }
    }

    /// A tile's path in the tier's parts.
    #[must_use]
    pub fn path(&self, key: TileKey) -> Option<String> {
        self.level(key.level)?.path(key.c, key.r)
    }

    /// The finest detail any level has, pixels a metre.
    #[must_use]
    pub fn finest_pixels_per_m(&self) -> f64 {
        self.world
            .iter()
            .chain(&self.towns)
            .map(Level::pixels_per_m)
            .fold(0.0, f64::max)
    }

    /// The world level to show at `screen` pixels a metre: the coarsest at
    /// least as fine, else the finest.
    #[must_use]
    pub fn world_level(&self, screen: f64) -> u8 {
        let i = self
            .world
            .iter()
            .rposition(|l| l.pixels_per_m() >= screen)
            .unwrap_or(0);
        u8::try_from(i).unwrap_or(0)
    }

    /// The town level laid over the world at `screen` pixels a metre: none
    /// while the finest world level is fine enough; else the coarsest at
    /// least as fine, else the finest.
    #[must_use]
    pub fn town_level(&self, screen: f64) -> Option<u8> {
        let finest_world = self.world.first()?.pixels_per_m();
        if self.towns.is_empty() || finest_world >= screen * 0.999 {
            return None;
        }
        let i = self
            .towns
            .iter()
            .position(|l| l.pixels_per_m() >= screen)
            .unwrap_or(self.towns.len() - 1);
        u8::try_from(i).ok()
    }

    /// A tile's square in world metres: west, north, east, south.
    #[must_use]
    pub fn tile_metres(&self, key: TileKey) -> Option<(f64, f64, f64, f64)> {
        let level = self.level(key.level)?;
        let side = f64::from(self.tile_size) / level.pixels_per_m();
        let west = f64::from(key.c) * side;
        let north = WORLD_M - f64::from(key.r) * side;
        Some((west, north, west + side, north - side))
    }

    /// The tiles `level` has over the world square `view` (west, north,
    /// east, south, metres), nearest `middle` (east, north) first.
    #[must_use]
    pub fn tiles_over(
        &self,
        level: LevelRef,
        view: (f64, f64, f64, f64),
        middle: (f64, f64),
    ) -> Vec<TileKey> {
        let Some(l) = self.level(level) else {
            return Vec::new();
        };
        let side = f64::from(self.tile_size) / l.pixels_per_m();
        let (west, north, east, south) = view;
        let col = |m: f64| (m / side).floor();
        let row = |m: f64| ((WORLD_M - m) / side).floor();
        let clamp = |v: f64, n: u32| v.clamp(0.0, f64::from(n.saturating_sub(1))) as u32;
        if east < 0.0 || west > WORLD_M || north < 0.0 || south > WORLD_M {
            return Vec::new();
        }
        let (c0, c1) = (clamp(col(west), l.cols), clamp(col(east), l.cols));
        let (r0, r1) = (clamp(row(north), l.rows), clamp(row(south), l.rows));
        let mut out = Vec::new();
        for r in r0..=r1 {
            for c in c0..=c1 {
                if l.has(c, r) {
                    out.push(TileKey { level, c, r });
                }
            }
        }
        let away = |k: &TileKey| {
            let cx = (f64::from(k.c) + 0.5) * side;
            let cy = WORLD_M - (f64::from(k.r) + 0.5) * side;
            (cx - middle.0).powi(2) + (cy - middle.1).powi(2)
        };
        out.sort_by(|a, b| away(a).total_cmp(&away(b)));
        out
    }
}

fn member<'a>(node: &'a Node, name: &str) -> Option<&'a Node> {
    match &node.value {
        Value::Object(members) => members.iter().find(|m| m.name == name).map(|m| &m.value),
        _ => None,
    }
}

fn levels(node: &Node) -> Result<Vec<Level>, ManifestError> {
    let Value::Array(items) = &node.value else {
        return Err(ManifestError::Bad("levels"));
    };
    if items.len() > MAX_LEVELS {
        return Err(ManifestError::Bad("levels"));
    }
    items.iter().map(level).collect()
}

fn whole(node: &Node, name: &'static str, max: u32) -> Result<u32, ManifestError> {
    let v = number(node, name)?;
    if v.fract() != 0.0 || v < 0.0 || v > f64::from(max) {
        return Err(ManifestError::Bad(name));
    }
    Ok(v as u32)
}

fn level(node: &Node) -> Result<Level, ManifestError> {
    let pixels_per_cell = number(node, "pixels_per_cell")?;
    if !(1.0 / 64.0..=256.0).contains(&pixels_per_cell) {
        return Err(ManifestError::Bad("pixels_per_cell"));
    }
    let cols = whole(node, "cols", 65_536)?;
    let rows = whole(node, "rows", 65_536)?;
    if cols == 0 || rows == 0 {
        return Err(ManifestError::Bad("cols"));
    }
    let tiles = if let Some(pattern) = member(node, "every") {
        let Value::String(p) = &pattern.value else {
            return Err(ManifestError::Bad("every"));
        };
        if p.matches("{c}").count() != 1 || p.matches("{r}").count() != 1 {
            return Err(ManifestError::Bad("every"));
        }
        check_path(&fill(p, 0, 0)).map_err(|_| ManifestError::Bad("every"))?;
        Tiles::Every(p.clone())
    } else {
        let Some(Node {
            value: Value::Array(list),
            ..
        }) = member(node, "tiles")
        else {
            return Err(ManifestError::Missing("tiles"));
        };
        if list.len() > MAX_LISTED {
            return Err(ManifestError::Bad("tiles"));
        }
        let mut tiles = HashMap::with_capacity(list.len());
        for t in list {
            let c = whole(t, "c", cols - 1)?;
            let r = whole(t, "r", rows - 1)?;
            let file = text(t, "file")?;
            check_path(&file).map_err(|_| ManifestError::Bad("file"))?;
            tiles.insert((c, r), file);
        }
        Tiles::Listed(tiles)
    };
    Ok(Level {
        pixels_per_cell,
        cols,
        rows,
        tiles,
    })
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use ac_host_core::json::parse;

    /// A small index: world levels at 4, 2 and 1 pixels a cell (one tile
    /// of 256 at 1 covers 256 cells), a town level at 8 with two tiles.
    pub(crate) fn small() -> MapIndex {
        let doc = br#"{
            "tile_size": 256,
            "levels": [
                {"pixels_per_cell": 1.0, "cols": 8, "rows": 8, "every": "webp/world/2/{c}_{r}.webp"},
                {"pixels_per_cell": 4.0, "cols": 32, "rows": 32, "every": "webp/world/0/{c}_{r}.webp"},
                {"pixels_per_cell": 2.0, "cols": 16, "rows": 16, "every": "webp/world/1/{c}_{r}.webp"}
            ],
            "towns": {"levels": [
                {"pixels_per_cell": 8.0, "cols": 64, "rows": 64, "tiles": [
                    {"c": 28, "r": 3, "file": "webp/towns/8/28_3.webp"},
                    {"c": 29, "r": 3, "file": "webp/towns/8/29_3.webp"}
                ]}
            ]}
        }"#;
        MapIndex::read(&parse(doc).unwrap()).unwrap()
    }

    #[test]
    fn world_levels_are_finest_first_and_tiles_have_their_paths() {
        let i = small();
        assert_eq!(i.world[0].pixels_per_cell, 4.0);
        assert_eq!(i.world[2].pixels_per_cell, 1.0);
        let k = TileKey {
            level: LevelRef::World(0),
            c: 3,
            r: 17,
        };
        assert_eq!(i.path(k).as_deref(), Some("webp/world/0/3_17.webp"));
        let off = TileKey { c: 32, ..k };
        assert_eq!(i.path(off), None);
        let town = TileKey {
            level: LevelRef::Town(0),
            c: 29,
            r: 3,
        };
        assert_eq!(i.path(town).as_deref(), Some("webp/towns/8/29_3.webp"));
        assert_eq!(i.path(TileKey { c: 30, ..town }), None);
        assert_eq!(i.towns[0].count(), 2);
    }

    #[test]
    fn the_level_shown_is_the_coarsest_at_least_as_fine_as_the_screen() {
        let i = small();
        // 1 pixel a cell is 1/24 a metre.
        assert_eq!(i.world_level(0.01), 2);
        assert_eq!(i.world_level(1.0 / 24.0), 2);
        assert_eq!(i.world_level(1.5 / 24.0), 1);
        assert_eq!(i.world_level(3.0 / 24.0), 0);
        assert_eq!(i.world_level(9.0 / 24.0), 0);
        // Towns come in only past the finest world level.
        assert_eq!(i.town_level(3.0 / 24.0), None);
        assert_eq!(i.town_level(5.0 / 24.0), Some(0));
        assert_eq!(i.town_level(50.0 / 24.0), Some(0));
    }

    #[test]
    fn tiles_over_a_view_are_the_ones_it_touches_nearest_the_middle_first() {
        let i = small();
        // One tile at 1 pixel a cell is 256 cells: 6144 m.
        let side = 6144.0;
        let view = (
            side * 0.5,
            WORLD_M - side * 0.5,
            side * 1.5,
            WORLD_M - side * 1.5,
        );
        let keys = i.tiles_over(LevelRef::World(2), view, (side * 1.4, WORLD_M - side * 1.4));
        assert_eq!(keys.len(), 4);
        assert_eq!((keys[0].c, keys[0].r), (1, 1));
        let m = i.tile_metres(keys[0]).unwrap();
        assert_eq!(m, (side, WORLD_M - side, side * 2.0, WORLD_M - side * 2.0));
        // Outside the world: nothing.
        assert!(
            i.tiles_over(LevelRef::World(2), (-9.0, -1.0, -5.0, -3.0), (0.0, 0.0))
                .is_empty()
        );
        // A town level only has what it lists.
        let all = (0.0, WORLD_M, WORLD_M, 0.0);
        assert_eq!(i.tiles_over(LevelRef::Town(0), all, (0.0, 0.0)).len(), 2);
    }

    #[test]
    fn map_coordinates_and_metres_are_one_another_back() {
        let (e, n) = metres(42.1, -33.6);
        let (x, y) = coordinates(e, n);
        assert!((x - 42.1).abs() < 1e-9 && (y + 33.6).abs() < 1e-9);
        assert_eq!(metres(0.0, 0.0), (CENTRE_M, CENTRE_M));
    }

    #[test]
    fn a_pattern_or_path_that_could_leave_the_folder_is_refused() {
        for bad in [
            r#"{"tile_size":256,"levels":[{"pixels_per_cell":1,"cols":1,"rows":1,"every":"../{c}_{r}.webp"}]}"#,
            r#"{"tile_size":256,"levels":[{"pixels_per_cell":1,"cols":1,"rows":1,"every":"w/{c}.webp"}]}"#,
            r#"{"tile_size":256,"levels":[{"pixels_per_cell":1,"cols":1,"rows":1,"tiles":[{"c":0,"r":0,"file":"/etc/x"}]}]}"#,
            r#"{"tile_size":256,"levels":[{"pixels_per_cell":1,"cols":1,"rows":1,"tiles":[{"c":1,"r":0,"file":"a.webp"}]}]}"#,
            r#"{"tile_size":256,"levels":[]}"#,
        ] {
            assert!(
                MapIndex::read(&parse(bad.as_bytes()).unwrap()).is_err(),
                "{bad}"
            );
        }
    }
}
