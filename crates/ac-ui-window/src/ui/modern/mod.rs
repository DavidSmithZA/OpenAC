//! OpenAC's modern interface, mounted over the retail one while the
//! launcher asks for it.
//!
//! The modern interface is built a screen at a time. Each modern piece
//! names the retail windows it stands in for ([`ModernLayer::replaced`]);
//! those stay hidden, and the window layout persistence leaves them alone
//! so the player's retail layout is kept for when Retail is picked again.
//! Every retail window nothing stands in for yet shows as before, so the
//! game is fully playable whatever has been built.
//!
//! Modern windows have names of their own, so their layout is saved apart
//! from the retail windows'; where each one first appears is in
//! [`places`].

pub mod places;

use ac_ui::UiHost;
use ac_ui::layout_persistence::LayoutPersistence;

/// The modern interface of one game window.
#[derive(Debug, Default)]
pub struct ModernLayer {
    /// The retail windows the built modern pieces stand in for.
    replaced: Vec<&'static str>,
}

impl ModernLayer {
    /// The layer with nothing built in it yet: every retail window shows.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The retail windows the layer stands in for.
    #[must_use]
    pub fn replaced(&self) -> &[&'static str] {
        &self.replaced
    }

    /// Stands in for retail window `name` from now on.
    pub fn replace(&mut self, name: &'static str) {
        if !self.replaced.contains(&name) {
            self.replaced.push(name);
        }
    }

    /// Tells the window layout persistence to leave the replaced windows
    /// alone.
    pub fn leave_alone(&self, host: &mut UiHost, layouts: &mut LayoutPersistence) {
        layouts.set_left_alone(host, self.replaced.iter().copied());
    }

    /// Hides every replaced retail window something showed (a key, a
    /// toolbar button, the game itself); called before each frame is drawn,
    /// so none is ever seen.
    pub fn hide_replaced(&self, host: &mut UiHost) {
        for name in &self.replaced {
            if host.root.is_window_visible(name) {
                host.root.hide_window(name);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use ac_ui::tree::Element;
    use ac_ui::widgets::Panel;

    use super::*;

    fn host_with(names: &[&str]) -> UiHost {
        let mut host = UiHost::new();
        host.root.set_size(800.0, 600.0);
        for name in names {
            let root = host.root.root();
            let frame = host.root.add(
                root,
                Element::at(10.0, 10.0, 100.0, 100.0),
                Panel::default(),
            );
            host.root.register_window(name, frame, None, 0).unwrap();
        }
        host
    }

    #[test]
    fn an_empty_layer_hides_nothing() {
        let mut host = host_with(&["radar", "chat"]);
        ModernLayer::new().hide_replaced(&mut host);
        assert!(host.root.is_window_visible("radar"));
        assert!(host.root.is_window_visible("chat"));
    }

    #[test]
    fn a_replaced_window_stays_hidden_whoever_shows_it() {
        let mut host = host_with(&["radar", "chat"]);
        let mut layer = ModernLayer::new();
        layer.replace("radar");
        layer.replace("radar");
        assert_eq!(layer.replaced(), ["radar"]);
        layer.hide_replaced(&mut host);
        assert!(!host.root.is_window_visible("radar"));
        host.root.show_window("radar");
        layer.hide_replaced(&mut host);
        assert!(!host.root.is_window_visible("radar"));
        assert!(host.root.is_window_visible("chat"));
    }
}
