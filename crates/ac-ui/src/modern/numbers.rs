//! Numbers short enough for a chip or a bar's end. Levels run into the
//! thousands and experience into the hundreds of billions on many servers,
//! so nothing may assume a few digits: a number that will not fit is
//! shortened, never cut off.

pub use crate::panels::appraisal::group_thousands;

/// Below this a number is shown whole, with separators ("99,999").
const WHOLE_BELOW: u64 = 100_000;

/// The units, each a thousand times the last.
const UNITS: [&str; 6] = ["K", "M", "B", "T", "Q", "Qi"];

/// `value` in at most about six characters: whole with separators below
/// 100,000, otherwise three figures and a unit ("123K", "4.56M", "12.3B").
/// The figures are cut, not rounded, so a number never reads as more than
/// it is (999,999 is "999K", not "1.00M").
#[must_use]
pub fn compact(value: i64) -> String {
    let magnitude = value.unsigned_abs();
    if magnitude < WHOLE_BELOW {
        return group_thousands(value);
    }
    let sign = if value < 0 { "-" } else { "" };
    let mut unit = 0;
    let mut scale: u64 = 1_000;
    while unit + 1 < UNITS.len() && magnitude / scale >= 1_000 {
        scale *= 1_000;
        unit += 1;
    }
    let whole = magnitude / scale;
    // Wide enough that a hundred times the largest remainder fits.
    let rest = u128::from(magnitude % scale);
    let scale = u128::from(scale);
    let text = if whole >= 100 {
        whole.to_string()
    } else if whole >= 10 {
        format!("{whole}.{}", rest * 10 / scale)
    } else {
        format!("{whole}.{:02}", rest * 100 / scale)
    };
    format!("{sign}{text}{}", UNITS[unit])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn small_numbers_are_whole() {
        assert_eq!(compact(0), "0");
        assert_eq!(compact(275), "275");
        assert_eq!(compact(4_812), "4,812");
        assert_eq!(compact(99_999), "99,999");
        assert_eq!(compact(-12_500), "-12,500");
    }

    #[test]
    fn large_numbers_keep_three_figures() {
        assert_eq!(compact(100_000), "100K");
        assert_eq!(compact(999_999), "999K");
        assert_eq!(compact(1_000_000), "1.00M");
        assert_eq!(compact(4_567_890), "4.56M");
        assert_eq!(compact(12_345_678), "12.3M");
        assert_eq!(compact(191_226_310_247), "191B");
        assert_eq!(compact(1_250_000_000_000), "1.25T");
        assert_eq!(compact(-3_400_000), "-3.40M");
    }

    #[test]
    fn the_largest_numbers_still_fit() {
        assert_eq!(compact(i64::MAX), "9.22Qi");
        assert_eq!(compact(i64::MIN), "-9.22Qi");
    }
}
