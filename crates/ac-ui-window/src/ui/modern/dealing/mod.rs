//! Dealing with others: the Vendor, Trade and Salvage windows.

pub mod salvage_model;
pub mod salvage_run;
pub mod salvage_window;
pub mod trade_model;
pub mod trade_run;
pub mod trade_window;
pub mod vendor_model;
pub mod vendor_run;
pub mod vendor_window;

/// The room kept between a window and the screen's edges, units.
pub const SCREEN_MARGIN: f32 = 24.0;

/// The width a window takes on a screen `screen` units wide: `wide` when
/// it fits with room to spare, else its narrow form.
#[must_use]
pub fn width_for(screen: f32, wide: f32, narrow: f32) -> f32 {
    if screen >= wide + 2.0 * SCREEN_MARGIN {
        wide
    } else {
        narrow
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_window_narrows_on_a_screen_it_does_not_fit() {
        assert_eq!(width_for(1440.0, 1040.0, 792.0), 1040.0);
        assert_eq!(width_for(1088.0, 1040.0, 792.0), 1040.0);
        assert_eq!(width_for(1024.0, 1040.0, 792.0), 792.0);
    }
}
