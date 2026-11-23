//! Numbered picks laid out in the room a column has: a style chooser's
//! "1, 2, 3 …" buttons, as many to a row as keeps them on few rows, paged
//! round the picked one when even that is not enough (a stepper beside
//! them reaches the rest).

/// A pick's height when there are few.
pub const PICK_HEIGHT: f32 = 34.0;
/// A pick's height when there are many (more than [`DENSE_FROM`]).
pub const DENSE_PICK_HEIGHT: f32 = 28.0;
/// More picks than this draw at [`DENSE_PICK_HEIGHT`].
pub const DENSE_FROM: usize = 16;
/// The space between picks, across and down.
pub const PICK_GAP: f32 = 4.0;
/// The fewest picks to a row.
pub const MIN_COLS: usize = 8;
/// The most picks to a row.
pub const MAX_COLS: usize = 12;

/// How the picks are laid out: at least [`MIN_COLS`] to a row, up to
/// [`MAX_COLS`], and paged round the picked one when even that is not
/// enough.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PickGrid {
    /// Picks to a row.
    pub cols: usize,
    /// Rows shown.
    pub rows: usize,
    /// The first pick shown (from 0).
    pub first: usize,
    /// One past the last pick shown.
    pub last: usize,
    /// A pick's width.
    pub width: f32,
    /// A pick's height.
    pub height: f32,
}

impl PickGrid {
    /// The grid for `count` picks, `now` picked (from 1; 0: none), in a
    /// column `width` wide with `room` of height.
    #[must_use]
    pub fn fit(count: usize, now: usize, room: f32, width: f32) -> Self {
        let height = if count > DENSE_FROM {
            DENSE_PICK_HEIGHT
        } else {
            PICK_HEIGHT
        };
        let rows_fit = (((room + PICK_GAP) / (height + PICK_GAP)).floor() as usize).max(1);
        let cols = count.div_ceil(rows_fit).clamp(MIN_COLS, MAX_COLS);
        let page = (rows_fit * cols).max(1);
        // A pick past the last one (the view not caught up yet) pages as
        // the last.
        let now = now.clamp(1, count.max(1));
        let first = (now - 1) / page * page;
        let last = (first + page).min(count).max(first);
        let rows = (last - first).div_ceil(cols).max(1);
        Self {
            cols,
            rows,
            first,
            last,
            width: (width - (cols - 1) as f32 * PICK_GAP) / cols as f32,
            height,
        }
    }

    /// A grid of a set `cols` to a row and at most `rows` rows of
    /// full-size picks, paged round `now` (from 1; 0: none), in a column
    /// `width` wide.
    #[must_use]
    pub fn paged(count: usize, now: usize, cols: usize, rows: usize, width: f32) -> Self {
        let cols = cols.max(1);
        let page = (cols * rows.max(1)).max(1);
        let now = now.clamp(1, count.max(1));
        let first = (now - 1) / page * page;
        let last = (first + page).min(count).max(first);
        Self {
            cols,
            rows: (last - first).div_ceil(cols).max(1),
            first,
            last,
            width: (width - (cols - 1) as f32 * PICK_GAP) / cols as f32,
            height: PICK_HEIGHT,
        }
    }

    /// The rectangle of pick `i` (one of `first..last`), the grid's top
    /// left at `(x, y)`.
    #[must_use]
    pub fn cell(&self, i: usize, x: f32, y: f32) -> (f32, f32, f32, f32) {
        let k = i.saturating_sub(self.first);
        (
            x + (k % self.cols) as f32 * (self.width + PICK_GAP),
            y + (k / self.cols) as f32 * (self.height + PICK_GAP),
            self.width,
            self.height,
        )
    }

    /// The height the shown rows take.
    #[must_use]
    pub fn height_used(&self) -> f32 {
        self.rows as f32 * (self.height + PICK_GAP) - PICK_GAP
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn many_picks_take_more_to_a_row_and_page_when_they_must() {
        let g = PickGrid::fit(12, 4, 200.0, 300.0);
        assert_eq!((g.cols, g.rows, g.first, g.last), (8, 2, 0, 12));
        let g = PickGrid::fit(50, 6, 170.0, 300.0);
        assert_eq!(g.cols, 10);
        assert_eq!((g.first, g.last), (0, 50));
        let g = PickGrid::fit(50, 40, 60.0, 300.0);
        assert_eq!(g.cols, 12);
        assert_eq!((g.first, g.last), (24, 48), "the page holding 40");
    }

    #[test]
    fn picks_fill_the_width_and_step_by_the_gap() {
        let g = PickGrid::fit(8, 1, 100.0, 8.0 * 30.0 + 7.0 * PICK_GAP);
        assert!((g.width - 30.0).abs() < 1e-4);
        assert_eq!(g.cell(0, 10.0, 20.0), (10.0, 20.0, 30.0, PICK_HEIGHT));
        assert_eq!(g.cell(7, 10.0, 20.0).0, 10.0 + 7.0 * (30.0 + PICK_GAP));
        assert!((g.height_used() - PICK_HEIGHT).abs() < 1e-4);
    }

    #[test]
    fn a_set_grid_pages_by_whole_pages() {
        let g = PickGrid::paged(50, 41, 8, 3, 300.0);
        assert_eq!((g.cols, g.rows, g.first, g.last), (8, 3, 24, 48));
        assert!((g.height - PICK_HEIGHT).abs() < 1e-4);
        let g = PickGrid::paged(50, 50, 8, 3, 300.0);
        assert_eq!((g.rows, g.first, g.last), (1, 48, 50));
    }

    #[test]
    fn nothing_picked_shows_the_first_page() {
        let g = PickGrid::fit(30, 0, 34.0, 300.0);
        assert_eq!((g.first, g.rows), (0, 1));
    }
}
