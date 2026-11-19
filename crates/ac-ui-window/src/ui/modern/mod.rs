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

pub mod fonts;
pub mod hud;
pub mod places;

use std::sync::Arc;

use ac_ui::bundled_font::Weight;
use ac_ui::layout_persistence::LayoutPersistence;
use ac_ui::modern::Horizon;
use ac_ui::modern::quick_move::QuickMoveOutlines;
use ac_ui::{ElementId, Modifiers, UiFont, UiHost};

use super::window::WindowUi;

/// The modern interface of one game window.
#[derive(Debug, Default)]
pub struct ModernLayer {
    /// The retail windows the built modern pieces stand in for.
    replaced: Vec<&'static str>,
    /// The Inter fonts baked so far.
    pub fonts: fonts::HorizonFonts,
    /// The layer quick move outlines the pieces in.
    pub outlines: Option<ElementId>,
    /// The vitals.
    pub vitals: Option<ElementId>,
    /// The low-health glow round the screen's edge.
    pub edge_glow: Option<ElementId>,
    /// The revision of the last cast answer looked at.
    seen_cast: Option<u64>,
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
    /// alone, and where the modern windows first go.
    pub fn leave_alone(&self, host: &mut UiHost, layouts: &mut LayoutPersistence) {
        layouts.set_left_alone(host, self.replaced.iter().copied());
        layouts.set_default_places(Box::new(|name, (w, h)| {
            places::default_place(name).map(|place| {
                let rect = place.rect((w as f32, h as f32));
                (rect.x, rect.y)
            })
        }));
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

/// The sizes of the vitals' words and values, and of the numbers that fall
/// off them, pixels to the em.
const VITALS_LABEL_SIZE: f32 = 11.0;
const VITALS_NUMBER_SIZE: f32 = 14.0;

/// The server's answer to a cast the mana could not pay for ("You don't
/// have enough Mana to cast this spell.").
pub const NOT_ENOUGH_MANA: u32 = 0x401;

/// The key that, held, lets modern pieces be moved whatever the lock says.
pub const QUICK_MOVE_KEY: Modifiers = Modifiers::CTRL;

/// The size of the names quick move shows, pixels to the em.
const QUICK_MOVE_NAME_SIZE: f32 = 11.0;

impl WindowUi {
    /// The HUD pieces built so far, each standing in for its retail
    /// windows.
    pub(super) fn mount_hud(&mut self) {
        let label = self.modern_font(Weight::SemiBold, VITALS_LABEL_SIZE);
        let number = self.modern_font(Weight::SemiBold, VITALS_NUMBER_SIZE);
        let glow = hud::edge_glow::EdgeGlow::mount(&mut self.host.root);
        let vitals = hud::mount_piece(
            &mut self.host.root,
            hud::VITALS,
            (hud::vitals::WIDTH, hud::vitals::BAR_HEIGHT),
            hud::vitals::VitalsHud::new(Horizon::BRONZE, label, number),
        );
        let Some(modern) = self.modern.as_mut() else {
            return;
        };
        modern.edge_glow = Some(glow);
        match vitals {
            Ok(id) => {
                modern.vitals = Some(id);
                modern.replace(super::window::VITALS_WINDOW);
                modern.replace(super::window::SIDE_VITALS_WINDOW);
            }
            Err(e) => self.notices.push(format!("ui: modern vitals: {e}")),
        }
    }

    /// The player's last cast answered: one the mana could not pay for
    /// shakes the modern mana bar and marks what the spell costs. Each
    /// answer is looked at once; the first one seen is only noted.
    pub(super) fn feed_modern_cast(
        &mut self,
        cast: ac_runtime::gameplay::magic::SpellCastCompletion,
    ) {
        let Some(modern) = self.modern.as_mut() else {
            return;
        };
        let first = modern.seen_cast.is_none();
        if modern.seen_cast.replace(cast.revision) == Some(cast.revision)
            || first
            || cast.weenie_error != NOT_ENOUGH_MANA
        {
            return;
        }
        let Some(bars) = modern.vitals else {
            return;
        };
        let cost = self
            .facts
            .numbers()
            .and_then(|n| n.spellbook())
            .and_then(|b| b.try_get_metadata(cast.spell_id))
            .map_or(0, |spell| u32::try_from(spell.mana_cost).unwrap_or(0));
        if let Some(hud) = self.host.root.widget_mut::<hud::vitals::VitalsHud>(bars) {
            hud.refuse_mana(cost);
        }
    }

    /// The vitals' new values, for the modern vitals and the edge glow.
    pub(super) fn feed_modern_vitals(&mut self, vitals: &ac_ui::panels::vitals::VitalsView) {
        let Some(modern) = self.modern.as_ref() else {
            return;
        };
        let (bars, glow) = (modern.vitals, modern.edge_glow);
        let mut low = false;
        if let Some(bars) = bars
            && let Some(hud) = self.host.root.widget_mut::<hud::vitals::VitalsHud>(bars)
        {
            hud.apply(vitals);
            low = hud.health_is_low();
        }
        if let Some(glow) = glow
            && let Some(edge) = self.host.root.widget_mut::<hud::edge_glow::EdgeGlow>(glow)
        {
            edge.on = low;
        }
    }

    /// Quick move on: its key, and the layer that outlines and names every
    /// modern piece while the key is held.
    pub(super) fn mount_quick_move(&mut self) {
        self.host.root.set_quick_move_key(Some(QUICK_MOVE_KEY));
        let font = self.modern_font(Weight::SemiBold, QUICK_MOVE_NAME_SIZE);
        let outlines = QuickMoveOutlines::mount(&mut self.host.root, Horizon::BRONZE, font);
        if let Some(modern) = self.modern.as_mut() {
            modern.outlines = Some(outlines);
        }
    }

    /// Inter in `weight` at `em` pixels to the em, for a modern piece:
    /// baked the first time it is asked for, its sharper bake following
    /// for a high-density display. None without the modern layer, or when
    /// the size cannot be baked (said in the notices).
    pub fn modern_font(&mut self, weight: Weight, em: f32) -> Option<Arc<UiFont>> {
        let modern = self.modern.as_mut()?;
        let known = modern.fonts.baked().count();
        let images = &self.images;
        let font = modern.fonts.get(weight, em, |height, face| {
            let mut atlas = ac_ui::bundled_font::bake_face(height, face)?;
            let coverage = std::mem::take(&mut atlas.coverage);
            let texture = images.add_coverage(atlas.width, atlas.height, coverage);
            Ok(Arc::new(atlas.create_font(texture)))
        });
        let baked_now = modern.fonts.baked().count() != known;
        let failures = modern.fonts.take_failures();
        self.notices.extend(failures);
        if baked_now {
            self.want_sharp_fonts_again();
        }
        font
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
