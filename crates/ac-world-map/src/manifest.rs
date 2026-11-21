//! A tier's manifest: its release, its parts (file, size, SHA-256) and its
//! index. It comes from the network, so everything is checked and nothing
//! panics.

use ac_host_core::json::{Node, Value, parse};

use crate::index::MapIndex;

/// The manifest's format name.
pub const FORMAT: &str = "aethrix.mappack";
/// The manifest version understood.
pub const VERSION: u32 = 1;
/// The largest manifest read, bytes.
pub const MAX_BYTES: usize = 4 * 1024 * 1024;
/// The largest part, bytes.
pub const MAX_PART: u64 = 16 * 1024 * 1024;
/// The most parts a tier may have.
const MAX_PARTS: usize = 256;

/// Why a manifest is not acceptable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ManifestError {
    /// Not JSON.
    NotJson,
    /// Another format or version.
    Format,
    /// A member missing.
    Missing(&'static str),
    /// A member of the wrong kind or out of range.
    Bad(&'static str),
}

impl std::fmt::Display for ManifestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotJson => write!(f, "the map's manifest is not JSON"),
            Self::Format => write!(
                f,
                "the map's manifest is of a format this client does not know"
            ),
            Self::Missing(m) => write!(f, "the map's manifest has no {m}"),
            Self::Bad(m) => write!(f, "the map's manifest has a bad {m}"),
        }
    }
}

impl std::error::Error for ManifestError {}

/// One download part.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Part {
    /// Its file name in the release (and on disk).
    pub file: String,
    /// Its size, bytes.
    pub bytes: u64,
    /// Its SHA-256, lower-case hex.
    pub sha256: String,
}

/// A tier's manifest.
#[derive(Debug, Clone, PartialEq)]
pub struct Manifest {
    /// The tier's name as the manifest gives it.
    pub name: String,
    /// The release number.
    pub release: u32,
    /// The parts, in order (the coarsest levels first).
    pub parts: Vec<Part>,
    /// The tier's levels.
    pub index: MapIndex,
}

impl Manifest {
    /// Reads a manifest.
    ///
    /// # Errors
    /// Not JSON, another format, or a member missing or bad.
    pub fn read(bytes: &[u8]) -> Result<Self, ManifestError> {
        if bytes.len() > MAX_BYTES {
            return Err(ManifestError::Bad("size"));
        }
        let doc = parse(bytes).map_err(|_| ManifestError::NotJson)?;
        if text(&doc, "format")? != FORMAT || number(&doc, "version")? != f64::from(VERSION) {
            return Err(ManifestError::Format);
        }
        let name = text(&doc, "name")?;
        let release = number(&doc, "release")?;
        if release.fract() != 0.0 || !(1.0..=1_000_000.0).contains(&release) {
            return Err(ManifestError::Bad("release"));
        }
        let Some(Node {
            value: Value::Array(list),
            ..
        }) = find(&doc, "parts")
        else {
            return Err(ManifestError::Missing("parts"));
        };
        if list.is_empty() || list.len() > MAX_PARTS {
            return Err(ManifestError::Bad("parts"));
        }
        let mut parts = Vec::with_capacity(list.len());
        for p in list {
            let file = text(p, "file")?;
            if !safe_file(&file) || parts.iter().any(|q: &Part| q.file == file) {
                return Err(ManifestError::Bad("part file"));
            }
            let bytes = number(p, "bytes")?;
            if bytes.fract() != 0.0 || bytes < 12.0 || bytes > MAX_PART as f64 {
                return Err(ManifestError::Bad("part bytes"));
            }
            let sha256 = text(p, "sha256")?.to_ascii_lowercase();
            if sha256.len() != 64 || !sha256.bytes().all(|b| b.is_ascii_hexdigit()) {
                return Err(ManifestError::Bad("part sha256"));
            }
            parts.push(Part {
                file,
                bytes: bytes as u64,
                sha256,
            });
        }
        let index = MapIndex::read(find(&doc, "map").ok_or(ManifestError::Missing("map"))?)?;
        Ok(Self {
            name,
            release: release as u32,
            parts,
            index,
        })
    }

    /// The download's size, bytes.
    #[must_use]
    pub fn bytes(&self) -> u64 {
        self.parts.iter().map(|p| p.bytes).sum()
    }
}

/// A part's file name: one plain name, nothing that could leave a folder.
fn safe_file(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 100
        && !name.starts_with('.')
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"_-.".contains(&b))
}

fn find<'a>(node: &'a Node, name: &str) -> Option<&'a Node> {
    match &node.value {
        Value::Object(members) => members.iter().find(|m| m.name == name).map(|m| &m.value),
        _ => None,
    }
}

/// A number member.
pub(crate) fn number(node: &Node, name: &'static str) -> Result<f64, ManifestError> {
    match find(node, name).map(|n| &n.value) {
        Some(Value::Number(s)) => s
            .parse::<f64>()
            .ok()
            .filter(|v| v.is_finite())
            .ok_or(ManifestError::Bad(name)),
        Some(_) => Err(ManifestError::Bad(name)),
        None => Err(ManifestError::Missing(name)),
    }
}

/// A string member.
pub(crate) fn text(node: &Node, name: &'static str) -> Result<String, ManifestError> {
    match find(node, name).map(|n| &n.value) {
        Some(Value::String(s)) => Ok(s.clone()),
        Some(_) => Err(ManifestError::Bad(name)),
        None => Err(ManifestError::Missing(name)),
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// A manifest for one part of `bytes` with `sha256`.
    pub(crate) fn one_part(release: u32, bytes: usize, sha256: &str) -> String {
        format!(
            r#"{{"format":"aethrix.mappack","version":1,"name":"standard","release":{release},
            "download_bytes":{bytes},
            "parts":[{{"file":"atlas-map-standard-1.bin","bytes":{bytes},"sha256":"{sha256}","entries":2}}],
            "map":{{"tile_size":256,"levels":[
                {{"pixels_per_cell":0.125,"cols":1,"rows":1,"every":"webp/world/0/{{c}}_{{r}}.webp"}}
            ],"towns":{{"levels":[
                {{"pixels_per_cell":8,"cols":64,"rows":64,"tiles":[{{"c":28,"r":3,"file":"webp/towns/8/28_3.webp"}}]}}
            ]}}}}}}"#
        )
    }

    #[test]
    fn a_manifest_reads_its_release_parts_and_levels() {
        let m = Manifest::read(one_part(3, 100, &"ab".repeat(32)).as_bytes()).unwrap();
        assert_eq!(m.name, "standard");
        assert_eq!(m.release, 3);
        assert_eq!(m.parts.len(), 1);
        assert_eq!(m.bytes(), 100);
        assert_eq!(m.index.world.len(), 1);
        assert_eq!(m.index.towns.len(), 1);
    }

    #[test]
    fn a_manifest_out_of_bounds_is_refused() {
        let good = one_part(1, 100, &"ab".repeat(32));
        assert!(Manifest::read(good.as_bytes()).is_ok());
        for (from, to) in [
            ("aethrix.mappack", "other"),
            (r#""version":1"#, r#""version":2"#),
            ("atlas-map-standard-1.bin", "../x.bin"),
            (&"ab".repeat(32), "zz"),
            (r#""bytes":100"#, r#""bytes":99999999"#),
            (r#""release":1"#, r#""release":0"#),
        ] {
            let bad = good.replacen(from, to, 1);
            assert!(Manifest::read(bad.as_bytes()).is_err(), "{from} -> {to}");
        }
        assert_eq!(Manifest::read(b"{"), Err(ManifestError::NotJson));
    }
}
