//! A download part: many of the tier's files in one.
//!
//! Layout, little-endian: `AXMP`, `u32` version (1), `u32` entry count;
//! per entry `u16` path length, the path (UTF-8), `u32` data length; then
//! every entry's data, in entry order. No entry spans two parts.
//!
//! A part comes from the network, so reading it checks everything and
//! never panics: lengths against what is there, and each path, which must
//! be relative, of `a-z 0-9 _ - .` and `/`, with no empty, `.` or `..`
//! piece.

use std::io::Read;

/// The four bytes a part starts with.
pub const MAGIC: [u8; 4] = *b"AXMP";
/// The part format's version.
pub const VERSION: u32 = 1;
/// The most entries one part may hold.
pub const MAX_ENTRIES: usize = 65_536;
/// The longest path an entry may have, bytes.
pub const MAX_PATH: usize = 200;

/// Why a part cannot be read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PartError {
    /// It does not start with [`MAGIC`].
    NotAPart,
    /// A version this client does not know.
    Version(u32),
    /// More entries than [`MAX_ENTRIES`].
    TooManyEntries(usize),
    /// A path that is not allowed.
    BadPath,
    /// It ends before what it says it holds.
    Truncated,
    /// Bytes are left after the last entry.
    Trailing,
}

impl std::fmt::Display for PartError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotAPart => write!(f, "not a map part"),
            Self::Version(v) => write!(f, "map part version {v} is unknown"),
            Self::TooManyEntries(n) => write!(f, "a map part with {n} entries"),
            Self::BadPath => write!(f, "a map part names a file it may not"),
            Self::Truncated => write!(f, "a map part ends early"),
            Self::Trailing => write!(f, "a map part has bytes after its last file"),
        }
    }
}

impl std::error::Error for PartError {}

/// Whether `path` may name a file in a part.
///
/// # Errors
/// [`PartError::BadPath`].
pub fn check_path(path: &str) -> Result<(), PartError> {
    let ok = !path.is_empty()
        && path.len() <= MAX_PATH
        && path
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b"_-./".contains(&b))
        && path
            .split('/')
            .all(|piece| !piece.is_empty() && piece != "." && piece != "..");
    if ok { Ok(()) } else { Err(PartError::BadPath) }
}

/// One file in a part: its path and where its bytes are in the part.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// Its path within the tier.
    pub path: String,
    /// Where its bytes start in the part.
    pub offset: u64,
    /// How many bytes.
    pub len: u32,
}

/// Reads a part's table from its start, checking that its data fits in
/// `part_len` bytes exactly.
///
/// # Errors
/// What is wrong with it, or the reader's failure as [`PartError::Truncated`].
pub fn read_table(mut reader: impl Read, part_len: u64) -> Result<Vec<Entry>, PartError> {
    let mut head = [0u8; 12];
    reader
        .read_exact(&mut head)
        .map_err(|_| PartError::Truncated)?;
    if head[0..4] != MAGIC {
        return Err(PartError::NotAPart);
    }
    let version = u32::from_le_bytes([head[4], head[5], head[6], head[7]]);
    if version != VERSION {
        return Err(PartError::Version(version));
    }
    let count = u32::from_le_bytes([head[8], head[9], head[10], head[11]]) as usize;
    if count > MAX_ENTRIES {
        return Err(PartError::TooManyEntries(count));
    }
    let mut table_len: u64 = 12;
    let mut raw = Vec::with_capacity(count);
    for _ in 0..count {
        let mut len = [0u8; 2];
        reader
            .read_exact(&mut len)
            .map_err(|_| PartError::Truncated)?;
        let path_len = usize::from(u16::from_le_bytes(len));
        if path_len > MAX_PATH {
            return Err(PartError::BadPath);
        }
        let mut path = vec![0u8; path_len];
        reader
            .read_exact(&mut path)
            .map_err(|_| PartError::Truncated)?;
        let path = String::from_utf8(path).map_err(|_| PartError::BadPath)?;
        check_path(&path)?;
        let mut data = [0u8; 4];
        reader
            .read_exact(&mut data)
            .map_err(|_| PartError::Truncated)?;
        raw.push((path, u32::from_le_bytes(data)));
        table_len += 2 + path_len as u64 + 4;
        if table_len > part_len {
            return Err(PartError::Truncated);
        }
    }
    let mut offset = table_len;
    let mut entries = Vec::with_capacity(count);
    for (path, len) in raw {
        entries.push(Entry { path, offset, len });
        offset += u64::from(len);
    }
    match offset.cmp(&part_len) {
        std::cmp::Ordering::Greater => Err(PartError::Truncated),
        std::cmp::Ordering::Less => Err(PartError::Trailing),
        std::cmp::Ordering::Equal => Ok(entries),
    }
}

/// A part holding `files`, in order (tests, and the store's own checks).
#[must_use]
pub fn write(files: &[(&str, &[u8])]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&MAGIC);
    out.extend_from_slice(&VERSION.to_le_bytes());
    out.extend_from_slice(&(files.len() as u32).to_le_bytes());
    for (path, data) in files {
        out.extend_from_slice(&(path.len() as u16).to_le_bytes());
        out.extend_from_slice(path.as_bytes());
        out.extend_from_slice(&(data.len() as u32).to_le_bytes());
    }
    for (_, data) in files {
        out.extend_from_slice(data);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_part_reads_back_where_each_file_is() {
        let part = write(&[("webp/a.webp", b"abc"), ("webp/b.webp", b"defgh")]);
        let table = read_table(part.as_slice(), part.len() as u64).unwrap();
        assert_eq!(table.len(), 2);
        let b = &table[1];
        assert_eq!(b.path, "webp/b.webp");
        let at = b.offset as usize;
        assert_eq!(&part[at..at + b.len as usize], b"defgh");
    }

    #[test]
    fn a_broken_part_is_refused() {
        let part = write(&[("webp/a.webp", b"abc")]);
        let n = part.len() as u64;
        assert_eq!(
            read_table(&part[..n as usize - 1], n - 1),
            Err(PartError::Truncated)
        );
        assert_eq!(read_table(part.as_slice(), n + 1), Err(PartError::Trailing));
        let mut other = part.clone();
        other[0] = b'X';
        assert_eq!(read_table(other.as_slice(), n), Err(PartError::NotAPart));
        let bad = write(&[("../a.webp", b"abc")]);
        assert_eq!(
            read_table(bad.as_slice(), bad.len() as u64),
            Err(PartError::BadPath)
        );
        let upper = write(&[("A.webp", b"abc")]);
        assert_eq!(
            read_table(upper.as_slice(), upper.len() as u64),
            Err(PartError::BadPath)
        );
    }
}
