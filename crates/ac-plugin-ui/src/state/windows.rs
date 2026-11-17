//! The screen and the registered windows as the attached window last
//! reported them, and the placings plugins ask of it. The window places a
//! window a frame later; the answer here is where the same rules put it
//! ([`WindowReport::fit`]), and the window's next report is where it went.

use super::PluginUi;
use crate::model::UiChange;
use crate::windows::{WindowApplied, WindowLayout, WindowParts, WindowReport};

/// What the session knows of the window's screen and registered windows.
#[derive(Debug, Default)]
pub(super) struct WindowMirror {
    /// The screen, once the attached window reported it.
    screen: Option<(i32, i32)>,
    /// The registered windows, in registration order.
    windows: Vec<WindowReport>,
    /// The screen changed since the session last took the change.
    screen_changed: Option<(i32, i32)>,
    /// A window was registered or unregistered since the session last
    /// took the change.
    windows_changed: bool,
}

impl WindowMirror {
    /// The window reported the screen and its windows.
    pub(super) fn report(&mut self, screen: (i32, i32), windows: Vec<WindowReport>) {
        if self.screen != Some(screen) {
            self.screen = Some(screen);
            self.screen_changed = Some(screen);
        }
        let names_differ = self.windows.len() != windows.len()
            || self
                .windows
                .iter()
                .zip(&windows)
                .any(|(was, now)| was.name != now.name);
        if names_differ {
            self.windows_changed = true;
        }
        self.windows = windows;
    }

    /// The window left: nothing is known until another reports.
    pub(super) fn forget(&mut self) {
        self.screen = None;
        self.windows.clear();
        self.screen_changed = None;
        self.windows_changed = false;
    }
}

impl PluginUi {
    /// The screen the interface lays out in, pixels: none until an
    /// attached window reported it.
    #[must_use]
    pub fn screen_size(&self) -> Option<(i32, i32)> {
        self.window_mirror.screen
    }

    /// The registered windows as the attached window last reported them, in
    /// registration order (none without a window).
    #[must_use]
    pub fn registered_windows(&self) -> &[WindowReport] {
        &self.window_mirror.windows
    }

    /// Asks the window to place registered window `name`: the `parts` of
    /// `layout`, within the window's limits and the screen, a shown state
    /// only for a window the game does not show and hide by itself. The
    /// window applies it the way it restores a saved layout and saves the
    /// result for the character, as the player's own change. The answer is
    /// where those rules put it.
    ///
    /// # Errors
    /// No window is attached, or none of its windows is called `name`.
    pub fn apply_window(
        &mut self,
        name: &str,
        layout: &WindowLayout,
        parts: WindowParts,
    ) -> Result<WindowApplied, String> {
        let Some(screen) = self.window_mirror.screen.filter(|_| self.window_attached) else {
            return Err("no window is attached".to_owned());
        };
        let Some(report) = self.window_mirror.windows.iter().find(|w| w.name == name) else {
            return Err(format!("no registered window is called '{name}'"));
        };
        let applied = WindowApplied {
            layout: report.fit(layout, parts, screen),
            shown_ignored: parts.shown && report.state_managed,
        };
        self.changes.push(UiChange::WindowApply {
            name: name.to_owned(),
            layout: report.merged(layout, parts),
            parts,
        });
        Ok(applied)
    }

    /// The screen's new size, once, when it changed since the last call
    /// (the first report of an attached window is a change).
    pub fn take_screen_change(&mut self) -> Option<(i32, i32)> {
        self.window_mirror.screen_changed.take()
    }

    /// Whether a window was registered or unregistered since the last
    /// call (the first report of an attached window with any is such a
    /// change).
    pub fn take_windows_change(&mut self) -> bool {
        std::mem::take(&mut self.window_mirror.windows_changed)
    }
}

#[cfg(test)]
mod tests {
    use crate::PluginUi;
    use crate::model::{UiChange, UiInput};
    use crate::windows::{WindowLayout, WindowOwner, WindowParts, WindowReport};

    fn report(name: &str, state_managed: bool) -> WindowReport {
        WindowReport {
            name: name.into(),
            owner: WindowOwner::Client,
            title: None,
            layout: WindowLayout {
                x: 10.0,
                y: 10.0,
                width: 200.0,
                height: 100.0,
                visible: false,
                collapsed: false,
                maximized: false,
            },
            state_managed,
            min_size: (50.0, 50.0),
            max_size: (500.0, 500.0),
        }
    }

    fn windows(names: &[&str]) -> Vec<WindowReport> {
        names.iter().map(|n| report(n, *n == "vendor")).collect()
    }

    #[test]
    fn the_screen_and_the_windows_are_what_the_window_last_reported() {
        let mut ui = PluginUi::new();
        let input = |screen, names: &[&str]| UiInput::Windows {
            screen,
            windows: windows(names),
        };
        // Without a window nothing is known.
        ui.input(input((800, 600), &["radar"]));
        assert_eq!(ui.screen_size(), None);
        assert!(ui.registered_windows().is_empty());
        ui.set_window_attached(true);
        ui.input(input((800, 600), &["radar"]));
        assert_eq!(ui.screen_size(), Some((800, 600)));
        assert_eq!(ui.take_screen_change(), Some((800, 600)));
        assert!(ui.take_windows_change());
        // A move is no change of either.
        let mut moved = windows(&["radar"]);
        moved[0].layout.x = 300.0;
        ui.input(UiInput::Windows {
            screen: (800, 600),
            windows: moved,
        });
        assert_eq!(ui.take_screen_change(), None);
        assert!(!ui.take_windows_change());
        assert_eq!(ui.registered_windows()[0].layout.x, 300.0);
        // A new screen, a window registered.
        ui.input(input((1280, 720), &["radar", "vendor"]));
        assert_eq!(ui.take_screen_change(), Some((1280, 720)));
        assert!(ui.take_windows_change());
        // The window leaves: nothing is known, and no change is told.
        ui.set_window_attached(false);
        assert_eq!(ui.screen_size(), None);
        assert!(ui.registered_windows().is_empty());
        assert_eq!(ui.take_screen_change(), None);
        assert!(!ui.take_windows_change());
    }

    #[test]
    fn a_placing_is_answered_at_once_and_asked_of_the_window() {
        let mut ui = PluginUi::new();
        let asked = WindowLayout {
            x: 700.0,
            y: 20.0,
            width: 300.0,
            height: 100.0,
            visible: true,
            collapsed: false,
            maximized: false,
        };
        assert!(ui.apply_window("radar", &asked, WindowParts::ALL).is_err());
        ui.set_window_attached(true);
        ui.input(UiInput::Windows {
            screen: (800, 600),
            windows: windows(&["radar", "vendor"]),
        });
        ui.take_changes();
        let error = ui
            .apply_window("nowhere", &asked, WindowParts::ALL)
            .unwrap_err();
        assert!(error.contains("nowhere"), "{error}");
        let radar = ui.apply_window("radar", &asked, WindowParts::ALL).unwrap();
        assert_eq!((radar.layout.x, radar.layout.width), (500.0, 300.0));
        assert!(radar.layout.visible);
        assert!(!radar.shown_ignored);
        let vendor = ui.apply_window("vendor", &asked, WindowParts::ALL).unwrap();
        assert!(!vendor.layout.visible);
        assert!(vendor.shown_ignored);
        let size_only = WindowParts {
            size: true,
            ..WindowParts::default()
        };
        ui.apply_window("radar", &asked, size_only).unwrap();
        let changes = ui.take_changes();
        assert_eq!(changes.len(), 3);
        // The window is asked for the whole layout: the parts not asked for
        // are where the window was.
        assert_eq!(
            changes[2],
            UiChange::WindowApply {
                name: "radar".into(),
                layout: WindowLayout {
                    width: 300.0,
                    height: 100.0,
                    ..report("radar", false).layout
                },
                parts: size_only,
            }
        );
        // What the session knows stays the window's report until the next.
        assert_eq!(ui.registered_windows()[0].layout.x, 10.0);
    }
}
