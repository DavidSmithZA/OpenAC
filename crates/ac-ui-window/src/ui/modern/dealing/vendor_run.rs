//! The Vendor window in the game window: it opens with the shop, shows
//! what the shop and the player's packs hold, and turns what the player
//! does in it into the vendor commands the retail window sends (the buy and
//! sell lists stay the session's, so either window shows the same lists).

use ac_core::items::ItemType;
use ac_runtime::inventory::{ItemCommand, ObjectRef, VendorCommand, VendorView, VendorViewReason};
use ac_runtime::session::SessionCommand;
use ac_ui::bundled_font::Weight;
use ac_ui::draw::Color;
use ac_ui::event::ItemDragPayload;
use ac_ui::geometry::AnchorEdges;
use ac_ui::icons::IconSource;
use ac_ui::modern::Horizon;
use ac_ui::panels::PanelOutput;
use ac_ui::panels::items::emit;
use ac_ui::panels::vendor::SPELL_COMPONENTS_MASK;
use ac_ui::panels::vendor_rules::{GuardedSplit, SplitOutcome, sell_check};
use ac_ui::tree::{Element, ElementId};
use ac_ui::widgets::Field;

use super::vendor_model::{
    self, Iconed, ShopFacts, VendorChoices, VendorScreen, VendorTab, refusal_limit,
};
use super::vendor_window::{ListAsk, Refused, VendorAsk, VendorFonts, VendorFrame, VendorList};
use crate::ui::modern::hud;
use crate::ui::modern::kit::{self, gold};
use crate::ui::window::WindowUi;

/// The window's name.
pub const VENDOR: &str = "modern-vendor";
/// How long a refusal shows, seconds.
const REFUSAL_SECONDS: f64 = 6.0;
/// The most digits a quantity takes.
const DIGITS: usize = 5;

/// The Vendor window's elements and what it remembers.
#[derive(Debug)]
pub struct VendorUi {
    /// The window.
    pub frame: ElementId,
    /// The list.
    pub list: ElementId,
    /// The search field.
    pub search: ElementId,
    /// The quantity field.
    pub quantity: ElementId,
    /// The plugins' band under the list.
    pub band: ElementId,
    /// What the player picked.
    pub choices: VendorChoices,
    /// The shop is showing (false: it closed).
    shown: bool,
    /// A split asked for to sell part of a stack, and whether it is sold
    /// at once (else put on the list).
    split: Option<(GuardedSplit, bool)>,
    /// When the refusal showed.
    refused_at: Option<ac_clock::Instant>,
    /// The shop opened (or the vendor was used again) since the last
    /// frame.
    opened: bool,
    /// `/fillcomps` filled the buy list since the last frame.
    filled: bool,
    /// The quantity the field was last set to.
    typed: Option<i32>,
}

impl WindowUi {
    /// The Vendor window, hidden, in place of retail's vendor window.
    pub(in crate::ui) fn mount_modern_vendor(&mut self) {
        let theme = Horizon::current();
        let fonts = VendorFonts {
            title: self.modern_font(Weight::SemiBold, 14.0),
            strong: self.modern_font(Weight::SemiBold, 12.5),
            small: self.modern_font(Weight::Regular, 11.0),
            label: self.modern_font(Weight::SemiBold, 10.0),
            body: self.modern_font(Weight::Regular, 12.0),
            tiny: self.modern_font(Weight::SemiBold, 10.0),
        };
        let frame = match hud::mount_piece(
            &mut self.host.root,
            VENDOR,
            (super::vendor_window::WIDTH, super::vendor_window::HEIGHT),
            VendorFrame::new(theme, fonts.clone()),
        ) {
            Ok(id) => id,
            Err(e) => {
                self.notices.push(format!("ui: modern vendor: {e}"));
                return;
            }
        };
        let search_font = fonts.body.clone();
        let quantity_font = fonts.strong.clone();
        let ui = &mut self.host.root;
        if let Some(e) = ui.element_mut(frame) {
            e.draggable = true;
        }
        let r = ui
            .widget::<VendorFrame>(frame)
            .map_or((216.0, 108.0, 440.0, 500.0), VendorFrame::list_rect);
        let list = ui.add(
            frame,
            Element::at(r.0, r.1, r.2, r.3).with_anchors(AnchorEdges::NONE),
            VendorList::new(fonts),
        );
        let (search_rect, q) = ui
            .widget::<VendorFrame>(frame)
            .map(|f| (f.search_rect(), f.quantity_rect()))
            .unwrap_or_default();
        let search = kit::mount_search(ui, frame, search_rect, search_font, &theme);
        let mut e =
            Element::at(q.0 + 1.0, q.1 + 2.0, q.2 - 2.0, q.3 - 4.0).with_anchors(AnchorEdges::NONE);
        Field::configure(&mut e);
        let mut f = Field::default();
        f.font = quantity_font;
        f.one_line = true;
        f.selectable = true;
        f.centered = true;
        f.padding = 2.0;
        f.record_history = false;
        f.clear_on_submit = false;
        f.stay_focused_after_submit = false;
        f.select_all_on_focus = true;
        f.max_characters = DIGITS;
        f.character_filter = Some(Box::new(|c: char| c.is_ascii_digit()));
        f.text_color = Horizon::current().palette.text;
        f.background_color = Color::TRANSPARENT;
        f.selection_color = gold(0.35);
        f.set_editable_detached(&mut e, true);
        let quantity = ui.add(frame, e, f);
        let band_widget = crate::ui::modern::items::plugin_notes::FootBand::new(
            self.modern_font(Weight::SemiBold, 9.5),
            self.modern_font(Weight::Regular, 10.5),
            self.modern_font(Weight::SemiBold, 12.0),
        );
        let ui = &mut self.host.root;
        let (band_x, band_w) = ui
            .widget::<VendorFrame>(frame)
            .map_or((216.0, 440.0), VendorFrame::band_span);
        let band = ui.add(
            frame,
            Element::at(band_x, 0.0, band_w, 1.0)
                .with_anchors(AnchorEdges::NONE)
                .with_visible(false),
            band_widget,
        );
        ui.hide_window(VENDOR);
        let Some(modern) = self.modern.as_mut() else {
            return;
        };
        modern.vendor = Some(VendorUi {
            frame,
            list,
            search,
            quantity,
            band,
            choices: VendorChoices::default(),
            shown: false,
            split: None,
            refused_at: None,
            opened: false,
            filled: false,
            typed: None,
        });
        modern.replace(crate::ui::social_panels::VENDOR_WINDOW);
    }

    fn vendor_mut(&mut self) -> Option<&mut VendorUi> {
        self.modern.as_mut().and_then(|m| m.vendor.as_mut())
    }

    fn vendor_send(&mut self, command: SessionCommand) {
        emit(&mut self.host.root, PanelOutput::Session(command));
    }

    fn vendor_list(&mut self, command: VendorCommand) {
        self.vendor_send(SessionCommand::Item(ItemCommand::Vendor(command)));
    }

    /// The shop's events as they came: an opening shows the window; a
    /// `/fillcomps` shows the components.
    pub(in crate::ui) fn note_modern_shop_events(&mut self, events: &[Option<Box<VendorView>>]) {
        let Some(v) = self.vendor_mut() else {
            return;
        };
        for e in events.iter().flatten() {
            match e.reason {
                VendorViewReason::Opened => v.opened = true,
                VendorViewReason::ComponentsFilled => v.filled = true,
                _ => {}
            }
        }
    }

    /// Each frame: the window follows the shop; what was asked is done;
    /// the window shows the shop as it stands.
    #[allow(clippy::too_many_lines)]
    pub(in crate::ui) fn tick_modern_vendor(&mut self) {
        let Some(v) = self.modern.as_ref().and_then(|m| m.vendor.as_ref()) else {
            return;
        };
        let (frame, list, search, quantity, band) = (v.frame, v.list, v.search, v.quantity, v.band);
        // The shop opened (or the vendor was used again), or closed.
        let open = self.facts.shop.is_some();
        let opened = v.opened || (open && !v.shown);
        let filled = v.filled;
        if !open && v.shown {
            self.host.root.hide_window(VENDOR);
            if let Some(v) = self.vendor_mut() {
                v.shown = false;
                v.split = None;
            }
            return;
        }
        // Put away by the player while the shop stays open (it closes when
        // the player walks off): nothing to show until it is used again.
        if !opened && !self.host.root.is_window_visible(VENDOR) {
            return;
        }
        let Some(shop) = self.facts.shop.clone() else {
            return;
        };
        // While the leave card shows, only the card takes the pointer.
        {
            let ui = &mut self.host.root;
            let leaving = ui.widget::<VendorFrame>(frame).is_some_and(|f| f.leaving);
            for id in [list, search, quantity, band] {
                if let Some(e) = ui.element_mut(id) {
                    e.click_through = leaving;
                }
            }
        }
        if opened {
            let ui = &mut self.host.root;
            ui.show_window(VENDOR);
            ui.bring_to_front(frame);
            if let Some(l) = ui.widget_mut::<VendorList>(list) {
                l.to_top();
            }
            if let Some(f) = ui.widget_mut::<VendorFrame>(frame) {
                f.leaving = false;
                f.refused = None;
            }
            if let Some(f) = ui.widget_mut::<Field>(search) {
                f.set_text("");
            }
            if let Some(v) = self.vendor_mut() {
                v.choices = VendorChoices {
                    only_taken: v.choices.only_taken,
                    ..VendorChoices::default()
                };
                v.refused_at = None;
            }
        }
        if filled && let Some(v) = self.vendor_mut() {
            // `/fillcomps` filled the buy list: the components show.
            v.choices.tab = VendorTab::Buy;
            v.choices.category = SPELL_COMPONENTS_MASK;
        }
        let expired = self.vendor_mut().is_some_and(|v| {
            v.shown = true;
            v.opened = false;
            v.filled = false;
            let done = v
                .refused_at
                .is_some_and(|t| t.elapsed().as_secs_f64() > REFUSAL_SECONDS);
            if done {
                v.refused_at = None;
            }
            done
        });
        if expired && let Some(f) = self.host.root.widget_mut::<VendorFrame>(frame) {
            f.refused = None;
        }
        // What the parts asked for.
        let ui = &mut self.host.root;
        let asks = ui
            .widget_mut::<VendorFrame>(frame)
            .map(|f| std::mem::take(&mut f.asks))
            .unwrap_or_default();
        let list_asks = ui
            .widget_mut::<VendorList>(list)
            .map(|l| std::mem::take(&mut l.asks))
            .unwrap_or_default();
        for (owner, id) in
            crate::ui::modern::items::plugin_notes::FootBand::take(&mut self.host.root, band)
        {
            self.send_plugin_input(ac_plugin_ui::UiInput::FootButton {
                owner,
                window: ac_plugin_ui::ItemWindow::Vendor,
                id,
            });
        }
        for ask in asks {
            self.vendor_ask(ask, &shop, frame, list, quantity);
        }
        for ask in list_asks {
            match ask {
                ListAsk::Choose(id) => {
                    if let Some(v) = self.vendor_mut() {
                        v.choices.chosen = Some(id);
                        v.choices.quantity = None;
                    }
                    emit(&mut self.host.root, PanelOutput::SelectObject(id));
                }
                ListAsk::Add(id) => {
                    if let Some(v) = self.vendor_mut() {
                        v.choices.chosen = Some(id);
                        v.choices.quantity = None;
                    }
                    self.vendor_add(&shop, id, None);
                }
                ListAsk::Inspect(id) => {
                    self.vendor_send(SessionCommand::Item(ItemCommand::Appraise(
                        ObjectRef::Guid(id),
                    )));
                }
                ListAsk::Dropped(p) => self.vendor_drop(&shop, &p, frame),
            }
        }
        // A split for a sale: stage or sell the new stack once it is in.
        let split = self.vendor_mut().and_then(|v| v.split.take());
        if let Some((mut pending, now)) = split {
            let clock = self.status.clock.elapsed().as_secs_f64();
            let outcome = self.with_vendor_items(|items| pending.follow(items, clock));
            let keep = match outcome {
                SplitOutcome::Arrived(g) => {
                    if now {
                        self.vendor_send(SessionCommand::Item(ItemCommand::Sell(vec![(
                            ObjectRef::Guid(g),
                            1,
                        )])));
                    } else {
                        self.vendor_list(VendorCommand::AddToSellList(ObjectRef::Guid(g)));
                    }
                    None
                }
                SplitOutcome::Waiting => Some((pending, now)),
                SplitOutcome::Failed => None,
            };
            if let Some(v) = self.vendor_mut() {
                v.split = keep;
            }
        }
        // What was typed.
        let ui = &self.host.root;
        let words = kit::field_text(ui, search);
        let typed = kit::field_text(ui, quantity);
        let typed_focus = ui.keyboard_focus() == Some(quantity);
        let search_focus = ui.keyboard_focus() == Some(search);
        if let Some(v) = self.vendor_mut() {
            v.choices.search = words.clone();
            if typed_focus && let Ok(n) = typed.parse::<i32>() {
                v.choices.quantity = Some(n.max(1));
            }
        }
        let feet = crate::ui::modern::items::plugin_notes::feet_of(
            &self.facts.plugin_ui,
            ac_plugin_ui::ItemWindow::Vendor,
        );
        let band_h = crate::ui::modern::items::plugin_notes::band_height(feet.len());
        if let Some(f) = self.host.root.widget_mut::<VendorFrame>(frame) {
            f.band = band_h;
        }
        let band_top = self
            .host
            .root
            .widget::<VendorFrame>(frame)
            .map_or(0.0, VendorFrame::band_top);
        let (_, band_w) = self
            .host
            .root
            .widget::<VendorFrame>(frame)
            .map_or((216.0, 440.0), VendorFrame::band_span);
        crate::ui::modern::items::plugin_notes::FootBand::show(
            &mut self.host.root,
            band,
            feet,
            band_top,
            band_w,
        );
        self.feed_modern_vendor(&shop, frame, list, quantity, typed_focus);
        if let Some(f) = self.host.root.widget_mut::<VendorFrame>(frame) {
            f.search_empty = words.is_empty();
            f.search_focused = search_focus;
        }
    }

    fn with_vendor_items<R>(&self, f: impl FnOnce(&ac_ui::panels::items::ItemsView<'_>) -> R) -> R {
        let selected = self.player_view.and_then(|v| v.selected);
        let pending: Vec<u32> = self.facts.state.pending_item.into_iter().collect();
        let resolve = crate::ui::facts::composed_names(&self.status.names);
        let view = self.facts.items_view(selected, &pending, None, &resolve);
        f(&view)
    }

    #[allow(clippy::too_many_lines)]
    fn vendor_ask(
        &mut self,
        ask: VendorAsk,
        shop: &VendorView,
        frame: ElementId,
        list: ElementId,
        quantity: ElementId,
    ) {
        let choices = self
            .vendor_mut()
            .map(|v| v.choices.clone())
            .unwrap_or_default();
        let shown = self
            .host
            .root
            .widget::<VendorFrame>(frame)
            .map(|f| f.screen.clone())
            .unwrap_or_default();
        let to_top = |s: &mut Self| {
            if let Some(l) = s.host.root.widget_mut::<VendorList>(list) {
                l.to_top();
            }
        };
        match ask {
            VendorAsk::Close => {
                if shop.lists.buying.is_empty() && shop.lists.selling.is_empty() {
                    self.host.root.hide_window(VENDOR);
                } else if let Some(f) = self.host.root.widget_mut::<VendorFrame>(frame) {
                    f.leaving = true;
                }
            }
            VendorAsk::Stay => {
                if let Some(f) = self.host.root.widget_mut::<VendorFrame>(frame) {
                    f.leaving = false;
                }
            }
            VendorAsk::Leave => {
                if let Some(f) = self.host.root.widget_mut::<VendorFrame>(frame) {
                    f.leaving = false;
                }
                self.host.root.hide_window(VENDOR);
            }
            VendorAsk::Tab(tab) => {
                if let Some(v) = self.vendor_mut() {
                    v.choices.tab = tab;
                    v.choices.chosen = None;
                    v.choices.quantity = None;
                }
                to_top(self);
            }
            VendorAsk::Rail(key) => {
                if let Some(v) = self.vendor_mut() {
                    match v.choices.tab {
                        VendorTab::Buy => v.choices.category = key,
                        VendorTab::Sell => v.choices.pack = key,
                    }
                    v.choices.chosen = None;
                    v.choices.quantity = None;
                }
                to_top(self);
            }
            VendorAsk::OnlyTaken(on) => {
                if let Some(v) = self.vendor_mut() {
                    v.choices.only_taken = on;
                }
                if let Some(f) = self.host.root.widget_mut::<VendorFrame>(frame) {
                    f.only_taken = on;
                }
            }
            VendorAsk::Sort => {
                if let Some(v) = self.vendor_mut() {
                    v.choices.sort = v.choices.sort.next();
                }
            }
            VendorAsk::Less | VendorAsk::More | VendorAsk::Most => {
                if let Some(c) = &shown.chosen {
                    let n = match ask {
                        VendorAsk::Less => c.quantity - 1,
                        VendorAsk::More => c.quantity + 1,
                        _ => c.most,
                    }
                    .clamp(1, c.most.max(1));
                    if let Some(v) = self.vendor_mut() {
                        v.choices.quantity = Some(n);
                    }
                    if self.host.root.keyboard_focus() == Some(quantity) {
                        self.host.root.set_keyboard_focus(None);
                    }
                }
            }
            VendorAsk::Now => {
                let Some(c) = &shown.chosen else {
                    return;
                };
                match choices.tab {
                    VendorTab::Buy => {
                        self.vendor_send(SessionCommand::Item(ItemCommand::Buy(vec![(
                            ObjectRef::Guid(c.id),
                            c.quantity,
                        )])));
                    }
                    VendorTab::Sell => self.vendor_sell(c.id, c.quantity, true),
                }
            }
            VendorAsk::Add => {
                if let Some(c) = &shown.chosen {
                    self.vendor_add(shop, c.id, Some(c.quantity));
                }
            }
            VendorAsk::Inspect => {
                if let Some(c) = &shown.chosen {
                    self.vendor_send(SessionCommand::Item(ItemCommand::Appraise(
                        ObjectRef::Guid(c.id),
                    )));
                }
            }
            VendorAsk::Clear => self.vendor_list(match choices.tab {
                VendorTab::Buy => VendorCommand::ClearBuyList,
                VendorTab::Sell => VendorCommand::ClearSellList,
            }),
            VendorAsk::Remove(id) => self.vendor_list(match choices.tab {
                // The whole row of a stacking kind, else one (retail).
                VendorTab::Buy => VendorCommand::RemoveFromBuyList {
                    item: ObjectRef::Guid(id),
                    amount: shop
                        .items
                        .iter()
                        .find(|i| i.item_guid == id)
                        .map_or(-1, ac_ui::panels::vendor_rules::removal_amount),
                },
                VendorTab::Sell => VendorCommand::RemoveFromSellList(ObjectRef::Guid(id)),
            }),
            VendorAsk::All => self.vendor_list(match choices.tab {
                VendorTab::Buy => VendorCommand::BuyAll,
                VendorTab::Sell => VendorCommand::SellAll,
            }),
            VendorAsk::Restock => {
                let adds = shown.restock.map(|r| r.adds).unwrap_or_default();
                for (item, amount) in adds {
                    self.vendor_list(VendorCommand::AddToBuyList {
                        item: ObjectRef::Guid(item),
                        amount,
                    });
                }
            }
            VendorAsk::Dropped(p) => self.vendor_drop(shop, &p, frame),
        }
    }

    /// Puts `id` on the list of the tab shown: `quantity` of a listing (one
    /// by default), or `quantity` of one of the player's items (all of it
    /// by default; part of a stack is split off first).
    fn vendor_add(&mut self, shop: &VendorView, id: u32, quantity: Option<i32>) {
        let tab = self.vendor_mut().map_or(VendorTab::Buy, |v| v.choices.tab);
        match tab {
            VendorTab::Buy => {
                if shop.items.iter().any(|i| i.item_guid == id) {
                    self.vendor_list(VendorCommand::AddToBuyList {
                        item: ObjectRef::Guid(id),
                        amount: quantity.unwrap_or(1).max(1),
                    });
                }
            }
            VendorTab::Sell => {
                let stack = self
                    .facts
                    .objects
                    .get(id)
                    .map_or(1, |o| o.stack_size.max(1));
                self.vendor_sell(id, quantity.unwrap_or(stack), false);
            }
        }
    }

    /// Sells `quantity` of `id` at once (`now`), or puts it on the sell
    /// list; part of a stack is split off first.
    fn vendor_sell(&mut self, id: u32, quantity: i32, now: bool) {
        let Some(o) = self.facts.objects.get(id).cloned() else {
            return;
        };
        let stack = o.stack_size.max(1);
        let has_contents = !self.facts.objects.get_contents(id).is_empty();
        if quantity < stack && !has_contents {
            let clock = self.status.clock.elapsed().as_secs_f64();
            match self.with_vendor_items(|items| GuardedSplit::start(&o, quantity, items, clock)) {
                Ok((pending, command, words)) => {
                    if let Some(v) = self.vendor_mut() {
                        v.split = Some((pending, now));
                    }
                    self.vendor_send(command);
                    emit(&mut self.host.root, PanelOutput::Notice(words));
                }
                Err(words) => emit(&mut self.host.root, PanelOutput::Notice(words.to_owned())),
            }
            return;
        }
        if now {
            self.vendor_send(SessionCommand::Item(ItemCommand::Sell(vec![(
                ObjectRef::Guid(id),
                1,
            )])));
            let staged = self
                .facts
                .shop
                .as_ref()
                .is_some_and(|s| s.lists.selling.iter().any(|r| r.item_guid == id));
            if staged && self.facts.state.can_make_request {
                self.vendor_list(VendorCommand::RemoveFromSellList(ObjectRef::Guid(id)));
            }
        } else {
            self.vendor_list(VendorCommand::AddToSellList(ObjectRef::Guid(id)));
        }
    }

    /// An item from the packs let go on the window: refused where it was
    /// dropped with the vendor's reason, else put on the sell list.
    fn vendor_drop(&mut self, shop: &VendorView, p: &ItemDragPayload, frame: ElementId) {
        let check = self.with_vendor_items(|items| sell_check(items, &shop.profile, p.obj_id));
        if let Err(r) = check {
            let names = crate::ui::facts::composed_names(&self.status.names);
            let vendor = shop.name.clone().unwrap_or_else(|| "The vendor".to_owned());
            let refused = self.facts.objects.get(p.obj_id).map(|o| Refused {
                name: names(o),
                reason: r.message().to_owned(),
                limit: refusal_limit(shop, o, &vendor),
            });
            if let Some(f) = self.host.root.widget_mut::<VendorFrame>(frame) {
                f.refused = refused;
            }
            if let Some(v) = self.vendor_mut() {
                v.refused_at = Some(ac_clock::Instant::now());
            }
            return;
        }
        if let Some(v) = self.vendor_mut() {
            v.choices.tab = VendorTab::Sell;
            v.choices.chosen = Some(p.obj_id);
            v.choices.quantity = None;
        }
        // The split amount picked for the selected stack goes, as retail's
        // drop does (the rest of the stack stays in the pack).
        let amount = i32::try_from(self.drag_amount(p.obj_id)).unwrap_or(i32::MAX);
        let stack = self
            .facts
            .objects
            .get(p.obj_id)
            .map_or(1, |o| o.stack_size.max(1));
        if amount > 0 && amount < stack {
            self.vendor_sell(p.obj_id, amount, false);
            return;
        }
        self.vendor_list(VendorCommand::AddToSellList(ObjectRef::Guid(p.obj_id)));
    }

    fn feed_modern_vendor(
        &mut self,
        shop: &VendorView,
        frame: ElementId,
        list: ElementId,
        quantity: ElementId,
        typing: bool,
    ) {
        let choices = self
            .vendor_mut()
            .map(|v| v.choices.clone())
            .unwrap_or_default();
        let desired = self
            .facts
            .numbers()
            .and_then(|n| n.spellbook())
            .map(|b| b.desired_components().clone());
        let pyreals = self
            .facts
            .objects
            .get(self.facts.state.player)
            .map_or(0, |p| {
                p.properties
                    .get_int(ac_core::properties::PropertyInt::COIN_VALUE.0, 0)
            });
        let selected = self.player_view.and_then(|v| v.selected);
        let pending: Vec<u32> = self.facts.state.pending_item.into_iter().collect();
        let Self {
            facts,
            icons,
            status,
            ..
        } = self;
        let resolve = crate::ui::facts::composed_names(&status.names);
        let items = facts.items_view(selected, &pending, None, &resolve);
        let shop_facts = ShopFacts {
            view: shop,
            items: &items,
            desired: desired.as_ref(),
            pyreals,
        };
        let mut icon = |what: Iconed<'_>| match what {
            Iconed::Listing(i) => icons.item_icon(
                ItemType(i.item_type.unwrap_or(0)),
                i.icon_id,
                i.icon_underlay_id,
                i.icon_overlay_id,
                i.effects,
            ),
            Iconed::Object(o) => icons.item_icon(
                o.item_type,
                o.icon_id,
                o.icon_underlay_id,
                o.icon_overlay_id,
                o.effects,
            ),
        };
        let plugin_ui = &facts.plugin_ui;
        let mark = |id: u32| {
            crate::ui::modern::items::plugin_notes::mark_on(plugin_ui, &Horizon::current(), id)
        };
        let screen: VendorScreen =
            vendor_model::screen(&shop_facts, &choices, &mut icon, &resolve, &mark);
        let chosen = screen.chosen.as_ref().map(|c| (c.id, c.quantity));
        let rows = screen.rows.clone();
        let Some((search, band)) = self.vendor_mut().map(|v| (v.search, v.band)) else {
            return;
        };
        // A screen too small for it gets the narrow window.
        let width = super::width_for(
            self.host.root.size().0,
            super::vendor_window::WIDTH,
            super::vendor_window::NARROW_WIDTH,
        );
        let (list_rect, search_rect, quantity_rect, band_span, rail_open) = {
            let ui = &mut self.host.root;
            let Some(f) = ui.widget_mut::<VendorFrame>(frame) else {
                return;
            };
            if f.width != width {
                f.width = width;
                f.rail_open = false;
            }
            f.screen = screen;
            (
                f.list_rect(),
                f.search_rect(),
                f.quantity_rect(),
                VendorFrame::band_span(f),
                f.rail_open,
            )
        };
        let ui = &mut self.host.root;
        if let Some(e) = ui.element_mut(frame)
            && (e.width - width).abs() > 0.5
        {
            e.width = width;
        }
        if let Some(l) = ui.widget_mut::<VendorList>(list) {
            l.rows = rows;
            l.chosen = chosen.map(|c| c.0);
        }
        // The open drop-down lies over the list: the list leaves it the
        // pointer.
        if let Some(e) = ui.element_mut(list) {
            e.left = list_rect.0;
            e.width = list_rect.2;
            e.top = list_rect.1;
            e.height = list_rect.3;
            if rail_open {
                e.click_through = true;
            }
        }
        if let Some(e) = ui.element_mut(search) {
            e.left = search_rect.0;
            e.width = search_rect.2;
        }
        if let Some(e) = ui.element_mut(quantity) {
            e.left = quantity_rect.0 + 1.0;
        }
        if let Some(e) = ui.element_mut(band) {
            e.left = band_span.0;
            e.width = band_span.1;
        }
        // The quantity field shows the chosen quantity unless it is being
        // typed in.
        let shown_quantity = chosen.map(|c| c.1);
        ui.set_visible(quantity, shown_quantity.is_some());
        // (Compared with what the field holds: typing past the most leaves
        // a number in it that is not the one used.)
        if !typing
            && let Some(n) = shown_quantity
            && kit::field_text(&self.host.root, quantity) != n.to_string()
        {
            if let Some(f) = self.host.root.widget_mut::<Field>(quantity) {
                f.set_text(&n.to_string());
            }
            if let Some(v) = self.vendor_mut() {
                v.typed = Some(n);
            }
        }
    }
}
