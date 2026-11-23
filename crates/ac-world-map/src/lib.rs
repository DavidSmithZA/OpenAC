//! The detailed world map (OPENAC). The game's own map is one small
//! picture; this is the world drawn from above in 256 px WebP tiles, a
//! pyramid of levels from one tile for all of Dereth down to 32 pixels a
//! 24 m cell, with finer tiles around the towns. It comes as an optional
//! download in four tiers (Standard, HD, Ultra, Extreme), each a manifest
//! and a few parts of at most 16 MB with their SHA-256.
//!
//! The parts are kept on disk as they came: a tile is read from its part
//! at its offset, so a tier is a handful of files, its install is whole or
//! absent, and a download that stops picks up at the first part missing.
//!
//! This crate reads the manifest, the index and the parts everywhere;
//! with the `net` feature it also downloads, keeps the tiers on disk and
//! reads and decodes tiles on a thread of its own ([`service`]). A window
//! sees it all through [`WorldMaps`].

pub mod index;
pub mod manifest;
pub mod part;
#[cfg(feature = "net")]
pub mod service;
#[cfg(feature = "net")]
pub mod store;

use std::sync::Arc;

pub use index::{LevelRef, MapIndex, TileKey};

/// Where the tiers are published by default: a release whose assets are
/// each tier's manifest (`atlas-map-<tier>.json`) and parts.
pub const DEFAULT_SOURCE: &str =
    "https://github.com/DavidSmithZA/OpenAC-Aethrix/releases/download/atlas-map-v1/";

/// A tier of detail.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Tier {
    /// 4 pixels a cell; towns 8 and 16.
    Standard,
    /// 8 pixels a cell; towns 16 and 32.
    Hd,
    /// 16 pixels a cell; towns 32.
    Ultra,
    /// 32 pixels a cell everywhere.
    Extreme,
}

impl Tier {
    /// Every tier, least detailed first.
    pub const ALL: [Self; 4] = [Self::Standard, Self::Hd, Self::Ultra, Self::Extreme];

    /// Its name in files and settings.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Standard => "standard",
            Self::Hd => "hd",
            Self::Ultra => "ultra",
            Self::Extreme => "extreme",
        }
    }

    /// The tier named `name`.
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|t| t.name() == name)
    }

    /// Its name for the player.
    #[must_use]
    pub const fn words(self) -> &'static str {
        match self {
            Self::Standard => "Standard",
            Self::Hd => "HD",
            Self::Ultra => "Ultra",
            Self::Extreme => "Extreme",
        }
    }

    /// About how big its download is, bytes, until its manifest says.
    #[must_use]
    pub const fn about_bytes(self) -> u64 {
        match self {
            Self::Standard => 11_405_355,
            Self::Hd => 35_459_142,
            Self::Ultra => 125_110_536,
            Self::Extreme => 885_048_736,
        }
    }

    /// Its manifest's file name in the release.
    #[must_use]
    pub fn manifest_file(self) -> String {
        format!("atlas-map-{}.json", self.name())
    }
}

/// What is under way.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Busy {
    /// Asking the release what each tier is.
    Asking,
    /// Downloading a tier: bytes so far of the whole.
    Downloading {
        /// The tier.
        tier: Tier,
        /// Bytes downloaded and checked (parts already on disk count).
        done: u64,
        /// The download's size.
        total: u64,
    },
    /// Deleting a tier's files.
    Removing(Tier),
}

/// One tier as things stand.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TierState {
    /// The tier.
    pub tier: Tier,
    /// The release on disk, when installed.
    pub installed: Option<u32>,
    /// The release published and its size, once asked.
    pub offered: Option<(u32, u64)>,
}

impl TierState {
    /// Its download's size: the release's when known, else about.
    #[must_use]
    pub fn bytes(&self) -> u64 {
        self.offered.map_or(self.tier.about_bytes(), |o| o.1)
    }

    /// A newer release is out than the one installed.
    #[must_use]
    pub fn update_out(&self) -> bool {
        matches!((self.installed, self.offered), (Some(i), Some((o, _))) if o > i)
    }
}

/// Every tier's state and what is under way.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapsStatus {
    /// Each tier, in [`Tier::ALL`]'s order.
    pub tiers: [TierState; 4],
    /// What is under way.
    pub busy: Option<Busy>,
    /// The last thing that failed, in the player's words.
    pub failed: Option<String>,
}

impl MapsStatus {
    /// Nothing installed, nothing asked.
    #[must_use]
    pub fn empty() -> Self {
        Self {
            tiers: Tier::ALL.map(|tier| TierState {
                tier,
                installed: None,
                offered: None,
            }),
            busy: None,
            failed: None,
        }
    }

    /// One tier's state.
    #[must_use]
    pub fn tier(&self, tier: Tier) -> &TierState {
        &self.tiers[tier as usize]
    }

    /// The tier drawn when `wanted` is chosen: it when installed, else the
    /// most detailed one installed.
    #[must_use]
    pub fn drawn(&self, wanted: Tier) -> Option<Tier> {
        if self.tier(wanted).installed.is_some() {
            return Some(wanted);
        }
        Tier::ALL
            .into_iter()
            .rev()
            .find(|t| self.tier(*t).installed.is_some())
    }
}

/// A tile read and decoded: RGBA, rows top first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pixels {
    /// Width, pixels.
    pub width: u32,
    /// Height, pixels.
    pub height: u32,
    /// Four bytes a pixel.
    pub rgba: Vec<u8>,
}

/// A tile asked for, back: its pixels, or none when it could not be read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Loaded {
    /// The tier it is of.
    pub tier: Tier,
    /// Which tile.
    pub key: TileKey,
    /// Its pixels.
    pub pixels: Option<Pixels>,
}

/// The detailed map as a window sees it.
pub trait WorldMaps: std::fmt::Debug {
    /// Every tier's state and what is under way.
    fn status(&self) -> MapsStatus;
    /// Goes up whenever [`WorldMaps::status`] changes.
    fn revision(&self) -> u64;
    /// Asks the release what each tier is (size, release), once until
    /// asked again; quietly, nothing fails aloud.
    fn ask(&mut self);
    /// Downloads `tier` (or brings it up to date). One at a time.
    fn download(&mut self, tier: Tier);
    /// Stops the download under way; what is on disk is kept for the next.
    fn cancel(&mut self);
    /// Deletes `tier`'s files.
    fn remove(&mut self, tier: Tier);
    /// `tier`'s index, when installed.
    fn index(&self, tier: Tier) -> Option<Arc<MapIndex>>;
    /// Asks for tiles to be read and decoded, most wanted first; replaces
    /// what was asked before and not yet started.
    fn want(&mut self, tier: Tier, keys: &[TileKey]);
    /// The tiles read since last asked.
    fn take_loaded(&mut self) -> Vec<Loaded>;
}

/// No detailed map (the page, tests): nothing installed, nothing offered.
#[derive(Debug, Default)]
pub struct NoMaps;

impl WorldMaps for NoMaps {
    fn status(&self) -> MapsStatus {
        MapsStatus::empty()
    }
    fn revision(&self) -> u64 {
        0
    }
    fn ask(&mut self) {}
    fn download(&mut self, _tier: Tier) {}
    fn cancel(&mut self) {}
    fn remove(&mut self, _tier: Tier) {}
    fn index(&self, _tier: Tier) -> Option<Arc<MapIndex>> {
        None
    }
    fn want(&mut self, _tier: Tier, _keys: &[TileKey]) {}
    fn take_loaded(&mut self) -> Vec<Loaded> {
        Vec::new()
    }
}

/// Bytes in the player's words: "885 MB", "11.4 MB", "640 KB".
#[must_use]
pub fn size_words(bytes: u64) -> String {
    let mb = bytes as f64 / 1_000_000.0;
    if mb >= 100.0 {
        format!("{mb:.0} MB")
    } else if mb >= 1.0 {
        format!("{mb:.1} MB")
    } else {
        format!("{:.0} KB", (bytes as f64 / 1000.0).max(1.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tiers_name_themselves_both_ways() {
        for t in Tier::ALL {
            assert_eq!(Tier::from_name(t.name()), Some(t));
        }
        assert_eq!(Tier::from_name("game"), None);
        assert_eq!(Tier::Extreme.manifest_file(), "atlas-map-extreme.json");
    }

    #[test]
    fn the_tier_drawn_is_the_one_chosen_else_the_most_detailed_installed() {
        let mut s = MapsStatus::empty();
        assert_eq!(s.drawn(Tier::Hd), None);
        s.tiers[Tier::Standard as usize].installed = Some(1);
        s.tiers[Tier::Ultra as usize].installed = Some(1);
        assert_eq!(s.drawn(Tier::Standard), Some(Tier::Standard));
        assert_eq!(s.drawn(Tier::Extreme), Some(Tier::Ultra));
    }

    #[test]
    fn an_update_is_a_newer_release_than_the_one_installed() {
        let mut t = MapsStatus::empty().tiers[0];
        assert!(!t.update_out());
        t.installed = Some(1);
        t.offered = Some((1, 10));
        assert!(!t.update_out());
        t.offered = Some((2, 10));
        assert!(t.update_out());
        assert_eq!(t.bytes(), 10);
    }

    #[test]
    fn sizes_read_as_the_player_would_say_them() {
        assert_eq!(size_words(885_048_736), "885 MB");
        assert_eq!(size_words(11_405_355), "11.4 MB");
        assert_eq!(size_words(640_000), "640 KB");
    }
}
