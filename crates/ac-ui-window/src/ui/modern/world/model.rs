//! The map's places worked out from the game's map art: the 53 towns at
//! their map coordinates, and how far and which way a place is.

use ac_ui::panels::map_house::{MAP_LOCATIONS, MapArt};

/// A town on the map.
#[derive(Debug, Clone, PartialEq)]
pub struct Town {
    /// Its name.
    pub name: &'static str,
    /// Its map coordinates (east, north).
    pub x: f64,
    pub y: f64,
    /// One of the larger towns (named on the map at every zoom).
    pub big: bool,
}

/// The towns of `art`, in the game's order.
#[must_use]
pub fn towns(art: &MapArt) -> Vec<Town> {
    MAP_LOCATIONS
        .iter()
        .map(|t| {
            let (x, y) = art.coordinates(
                f64::from(t.x) + f64::from(t.width) * 0.5,
                f64::from(t.y) + f64::from(t.height) * 0.5,
            );
            Town {
                name: t.name,
                x,
                y,
                big: t.width >= 9,
            }
        })
        .collect()
}

/// Map coordinates as the windows write them ("42.1N, 33.9E").
#[must_use]
pub fn coordinates_text(x: f64, y: f64) -> String {
    let axis = |v: f64, neg: &str, pos: &str| {
        let r = (v * 10.0).round() / 10.0;
        let letter = if r < 0.0 {
            neg
        } else if r > 0.0 {
            pos
        } else {
            ""
        };
        format!("{:.1}{letter}", r.abs())
    };
    format!("{}, {}", axis(y, "S", "N"), axis(x, "W", "E"))
}

/// The eight ways, from north clockwise.
const WAYS: [(&str, &str); 8] = [
    ("north", "N"),
    ("north-east", "NE"),
    ("east", "E"),
    ("south-east", "SE"),
    ("south", "S"),
    ("south-west", "SW"),
    ("west", "W"),
    ("north-west", "NW"),
];

/// Which of the eight ways a bearing points (degrees clockwise from
/// north): its word and letters.
#[must_use]
pub fn way(bearing: f64) -> (&'static str, &'static str) {
    let i = ((bearing.rem_euclid(360.0) + 22.5) / 45.0).floor() as usize % 8;
    WAYS[i]
}

/// How far (map units) and which way (degrees clockwise from north)
/// (`x`, `y`) is from (`fx`, `fy`).
#[must_use]
pub fn bearing(fx: f64, fy: f64, x: f64, y: f64) -> (f64, f64) {
    let (dx, dy) = (x - fx, y - fy);
    let distance = (dx * dx + dy * dy).sqrt();
    let degrees = dx.atan2(dy).to_degrees().rem_euclid(360.0);
    (distance, degrees)
}

/// "4.2 to the north-east of you" (or "where you are" when close).
#[must_use]
pub fn away_words(from: (f64, f64), to: (f64, f64)) -> String {
    let (d, b) = bearing(from.0, from.1, to.0, to.1);
    if d < 0.05 {
        return "Where you are".to_owned();
    }
    format!("{d:.1} to the {} of you", way(b).0)
}

/// "4.2 NE", for a list.
#[must_use]
pub fn away_short(from: (f64, f64), to: (f64, f64)) -> String {
    let (d, b) = bearing(from.0, from.1, to.0, to.1);
    if d < 0.05 {
        return "here".to_owned();
    }
    format!("{d:.1} {}", way(b).1)
}

/// The compass bearing (degrees clockwise from north) of a yaw in radians
/// counter-clockwise from east.
#[must_use]
pub fn heading_of_yaw(yaw: f32) -> f64 {
    (90.0 - f64::from(yaw).to_degrees()).rem_euclid(360.0)
}

/// A landblock's map coordinates (its middle), for a house for sale.
#[must_use]
pub fn landblock_coordinates(landblock: u32) -> Option<(f64, f64)> {
    let lx = f64::from((landblock >> 24) & 0xFF);
    let ly = f64::from((landblock >> 16) & 0xFF);
    // 0.8 map units to a landblock (eight cells of 0.1); the middle of the
    // block, as the radar counts cells.
    Some((lx * 0.8 - 101.5, ly * 0.8 - 101.5))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ways_and_distances_read_as_a_compass() {
        assert_eq!(way(0.0).0, "north");
        assert_eq!(way(44.0).0, "north-east");
        assert_eq!(way(181.0).1, "S");
        assert_eq!(way(-90.0).0, "west");
        let (d, b) = bearing(0.0, 0.0, 3.0, 4.0);
        assert!((d - 5.0).abs() < 1e-9);
        assert_eq!(way(b).0, "north-east");
        assert_eq!(
            away_words((0.0, 0.0), (0.0, -2.0)),
            "2.0 to the south of you"
        );
        assert_eq!(away_short((0.0, 0.0), (-3.0, 0.0)), "3.0 W");
    }

    #[test]
    fn coordinates_read_north_south_first() {
        assert_eq!(coordinates_text(33.94, 42.06), "42.1N, 33.9E");
        assert_eq!(coordinates_text(-12.3, -0.04), "0.0, 12.3W");
    }

    #[test]
    fn a_yaw_from_east_turns_into_a_compass_heading() {
        assert!((heading_of_yaw(0.0) - 90.0).abs() < 1e-6, "facing east");
        let north = heading_of_yaw(std::f32::consts::FRAC_PI_2);
        assert!(!(1e-3..=360.0 - 1e-3).contains(&north), "north: {north}");
    }

    #[test]
    fn the_towns_sit_inside_the_world() {
        let art = MapArt {
            image: 1,
            width: 256.0,
            height: 256.0,
            area: (3, 252, 3, 252),
        };
        let t = towns(&art);
        assert_eq!(t.len(), 53);
        assert!(t.iter().all(|t| t.x.abs() < 103.0 && t.y.abs() < 103.0));
        let holtburg = t.iter().find(|t| t.name == "Holtburg").unwrap();
        assert!(holtburg.big);
        // Holtburg is north and east of the middle.
        assert!(holtburg.x > 0.0 && holtburg.y > 0.0);
    }
}
