//! Character creation on Modern (the Studio).

use crate::scene::Scene;
use crate::ui::window::WindowUi;

/// The Studio's elements and what it remembers.
#[derive(Debug)]
pub struct StudioUi {
    /// It shows.
    pub open: bool,
}

impl WindowUi {
    pub(in crate::ui) fn mount_modern_studio(&mut self) -> Option<StudioUi> {
        None
    }

    pub(in crate::ui) fn open_modern_studio(&mut self, _scene: &Scene) {}

    pub(in crate::ui) fn tick_modern_studio(&mut self, _scene: &Scene, _choosing: bool) {}

    pub(in crate::ui) fn modern_studio_key(&mut self, _key: ac_ui::event::Key) -> bool {
        false
    }
}
