//! The Trade window in the game window: it opens and closes with the
//! trade, shows both sides as the session keeps them, and sends the trade
//! commands retail's window sends. It remembers what the other player
//! offered when the player accepted, so a change after that is marked, and
//! the last thing the game refused.

use ac_runtime::inventory::{ItemCommand, ObjectRef, TradeView};
use ac_runtime::session::SessionCommand;
use ac_ui::bundled_font::Weight;
use ac_ui::event::ItemDragPayload;
use ac_ui::geometry::AnchorEdges;
use ac_ui::icons::IconSource;
use ac_ui::modern::Horizon;
use ac_ui::panels::PanelOutput;
use ac_ui::panels::items::emit;
use ac_ui::panels::vendor_rules::{PendingSplit, SplitOutcome};
use ac_ui::tree::{Element, ElementId};
use ac_ui::widgets::Field;

use super::trade_model::{self, TradeMemory};
use super::trade_window::{TradeAsk, TradeCard, TradeFonts, TradeFrame};
use crate::ui::modern::hud;
use crate::ui::modern::items::plugin_notes::{self, FootBand};
use crate::ui::modern::items::split::{SplitFonts, SplitPopover, SplitRequest};
use crate::ui::modern::kit;
use crate::ui::window::WindowUi;

/// The window's name.
pub const TRADE: &str = "modern-trade";

/// The Trade window's elements and what it remembers.
#[derive(Debug)]
pub struct TradeUi {
    /// The window.
    pub frame: ElementId,
    /// Add from packs' search field.
    pub search: ElementId,
    /// The plugins' band over the foot.
    pub band: ElementId,
    /// How many of a stack to offer.
    pub split: ElementId,
    /// What the window remembers of the trade.
    pub memory: TradeMemory,
    /// The trade shows.
    open: bool,
    /// The player had accepted, last frame.
    accepted: bool,
    /// The player took the acceptance back (or changed the trade) since.
    own_change: bool,
    /// The last refusal seen: item and reason.
    seen_refusal: (u32, u32),
    /// A split asked for to offer part of a stack.
    pending: Option<PendingSplit>,
}

impl WindowUi {
    /// The Trade window, hidden, in place of retail's secure-trade window.
    pub(in crate::ui) fn mount_modern_trade(&mut self) {
        let theme = Horizon::BRONZE;
        let fonts = TradeFonts {
            title: self.modern_font(Weight::SemiBold, 15.0),
            strong: self.modern_font(Weight::SemiBold, 12.5),
            small: self.modern_font(Weight::Regular, 11.0),
            tiny: self.modern_font(Weight::SemiBold, 10.0),
            body: self.modern_font(Weight::Regular, 12.0),
        };
        let split_fonts = SplitFonts {
            title: self.modern_font(Weight::SemiBold, 13.0),
            small: self.modern_font(Weight::Regular, 11.0),
            body: self.modern_font(Weight::SemiBold, 12.0),
        };
        let band_widget = FootBand::new(
            theme,
            self.modern_font(Weight::SemiBold, 9.5),
            self.modern_font(Weight::Regular, 10.5),
            self.modern_font(Weight::SemiBold, 12.0),
        );
        let search_font = fonts.body.clone();
        let frame = match hud::mount_piece(
            &mut self.host.root,
            TRADE,
            (super::trade_window::WIDTH, super::trade_window::HEIGHT),
            TradeFrame::new(theme, fonts),
        ) {
            Ok(id) => id,
            Err(e) => {
                self.notices.push(format!("ui: modern trade: {e}"));
                return;
            }
        };
        let ui = &mut self.host.root;
        if let Some(e) = ui.element_mut(frame) {
            e.draggable = true;
        }
        let search = kit::mount_search(
            ui,
            frame,
            TradeFrame::pick_search_rect(),
            search_font,
            &theme,
        );
        ui.set_visible(search, false);
        let band = ui.add(
            frame,
            Element::at(0.0, 0.0, super::trade_window::WIDTH, 1.0)
                .with_anchors(AnchorEdges::NONE)
                .with_visible(false),
            band_widget,
        );
        let split = SplitPopover::mount(ui, theme, split_fonts);
        ui.hide_window(TRADE);
        let Some(modern) = self.modern.as_mut() else {
            return;
        };
        modern.trade = Some(TradeUi {
            frame,
            search,
            band,
            split,
            memory: TradeMemory::default(),
            open: false,
            accepted: false,
            own_change: false,
            seen_refusal: (0, 0),
            pending: None,
        });
        modern.replace(crate::ui::social_panels::TRADE_WINDOW);
    }

    fn trade_mut(&mut self) -> Option<&mut TradeUi> {
        self.modern.as_mut().and_then(|m| m.trade.as_mut())
    }

    fn trade_send(&mut self, command: ItemCommand) {
        emit(
            &mut self.host.root,
            PanelOutput::Session(SessionCommand::Item(command)),
        );
    }

    /// Each frame: the window follows the trade; what was asked is done.
    #[allow(clippy::too_many_lines)]
    pub(in crate::ui) fn tick_modern_trade(&mut self) {
        let Some(t) = self.modern.as_ref().and_then(|m| m.trade.as_ref()) else {
            return;
        };
        let (frame, search, band, split) = (t.frame, t.search, t.band, t.split);
        let was_open = t.open;
        if !was_open && !self.facts.trade.open {
            // No trade: nothing asked of the closed window is kept for the
            // next one.
            let ui = &mut self.host.root;
            if let Some(f) = ui.widget_mut::<TradeFrame>(frame) {
                f.asks.clear();
            }
            SplitPopover::take(ui, split);
            FootBand::take(ui, band);
            return;
        }
        let trade = self.facts.trade.clone();
        if trade.open != was_open {
            let ui = &mut self.host.root;
            if trade.open {
                ui.show_window(TRADE);
                ui.bring_to_front(frame);
            } else {
                ui.hide_window(TRADE);
                SplitPopover::close(ui, split);
            }
            if let Some(f) = ui.widget_mut::<TradeFrame>(frame) {
                f.picking = false;
                f.card = None;
            }
            ui.set_visible(search, false);
            if let Some(t) = self.trade_mut() {
                t.open = trade.open;
                t.memory = TradeMemory::default();
                t.accepted = false;
                t.own_change = false;
                t.seen_refusal = (trade.last_failure_item, trade.last_failure_reason);
                t.pending = None;
            }
        }
        if !trade.open {
            return;
        }
        self.follow_trade_memory(&trade);
        // What the parts asked for.
        let ui = &mut self.host.root;
        let asks = ui
            .widget_mut::<TradeFrame>(frame)
            .map(|f| std::mem::take(&mut f.asks))
            .unwrap_or_default();
        let splits = SplitPopover::take(ui, split);
        for (owner, id) in FootBand::take(ui, band) {
            self.send_plugin_input(ac_plugin_ui::UiInput::FootButton {
                owner,
                window: ac_plugin_ui::ItemWindow::Trade,
                id,
            });
        }
        for ask in asks {
            self.trade_ask(ask, frame, search, split);
        }
        for SplitRequest::Split { item, amount } in splits {
            self.offer(item, i32::try_from(amount).unwrap_or(i32::MAX));
        }
        // A split for an offer: offer the new stack once it is in.
        if let Some(mut pending) = self.trade_mut().and_then(|t| t.pending) {
            let outcome = {
                let selected = self.player_view.and_then(|v| v.selected);
                let in_flight: Vec<u32> = self.facts.state.pending_item.into_iter().collect();
                let resolve = crate::ui::facts::composed_names(&self.status.names);
                let view = self.facts.items_view(selected, &in_flight, None, &resolve);
                pending.follow(&view)
            };
            let keep = match outcome {
                SplitOutcome::Arrived(g) => {
                    self.trade_send(ItemCommand::AddToTrade(ObjectRef::Guid(g)));
                    None
                }
                SplitOutcome::Waiting => Some(pending),
                SplitOutcome::Failed => None,
            };
            if let Some(t) = self.trade_mut() {
                t.pending = keep;
            }
        }
        self.feed_modern_trade(&trade, frame, search, band);
    }

    /// Remembers what the other player offered when the player accepted,
    /// whether a later change took the acceptance back, and the last
    /// refusal.
    fn follow_trade_memory(&mut self, trade: &TradeView) {
        let refusal = (trade.last_failure_item, trade.last_failure_reason);
        let seen = self
            .modern
            .as_ref()
            .and_then(|m| m.trade.as_ref())
            .map(|t| t.seen_refusal);
        let fresh_refusal = (seen != Some(refusal) && refusal.1 != 0).then(|| {
            let names = crate::ui::facts::composed_names(&self.status.names);
            let name = self
                .facts
                .objects
                .get(refusal.0)
                .map(names)
                .or_else(|| trade.item_names.get(&refusal.0).cloned())
                .unwrap_or_else(|| "Something".to_owned());
            let words = ac_core::chat::weenie_errors::resolve(refusal.1, None)
                .map_or_else(|| "The game refused it.".to_owned(), |(t, _)| t);
            (name, words)
        });
        let Some(t) = self.trade_mut() else {
            return;
        };
        t.seen_refusal = refusal;
        if let Some(r) = fresh_refusal {
            t.memory.refused = Some(r);
        }
        if trade.self_accepted && !t.accepted {
            t.memory.accepted_with = Some(trade.partner_items.clone());
            t.memory.taken_back = false;
            t.memory.refused = None;
            t.own_change = false;
        } else if !trade.self_accepted && t.accepted {
            let changed = t
                .memory
                .accepted_with
                .as_ref()
                .is_some_and(|was| *was != trade.partner_items);
            t.memory.taken_back = changed && !t.own_change;
            if !t.memory.taken_back {
                t.memory.accepted_with = None;
            }
        }
        if trade.self_items.is_empty() && trade.partner_items.is_empty() {
            t.memory.taken_back = false;
            t.memory.accepted_with = None;
        }
        t.accepted = trade.self_accepted;
    }

    fn trade_ask(&mut self, ask: TradeAsk, frame: ElementId, search: ElementId, split: ElementId) {
        match ask {
            TradeAsk::Close => self.trade_send(ItemCommand::CloseTrade),
            TradeAsk::Accept => self.trade_send(ItemCommand::AcceptTrade),
            TradeAsk::Withdraw => {
                if let Some(t) = self.trade_mut() {
                    t.own_change = true;
                }
                self.trade_send(ItemCommand::DeclineTrade);
            }
            TradeAsk::Clear => {
                if let Some(t) = self.trade_mut() {
                    t.own_change = true;
                }
                self.trade_send(ItemCommand::ResetTrade);
            }
            TradeAsk::Pick(on) => {
                let ui = &mut self.host.root;
                if let Some(f) = ui.widget_mut::<TradeFrame>(frame) {
                    f.picking = on;
                    f.card = None;
                }
                ui.set_visible(search, on);
                if on {
                    if let Some(f) = ui.widget_mut::<Field>(search) {
                        f.set_text("");
                    }
                    ui.set_keyboard_focus(Some(search));
                } else if ui.keyboard_focus() == Some(search) {
                    ui.set_keyboard_focus(None);
                }
            }
            TradeAsk::Offer(id, at) => {
                let Some(o) = self.facts.objects.get(id) else {
                    return;
                };
                let stack = o.stack_size.max(1);
                if stack > 1 {
                    let names = crate::ui::facts::composed_names(&self.status.names);
                    let name = names(o);
                    let (fx, fy) = self.host.root.screen_position(frame);
                    SplitPopover::open(
                        &mut self.host.root,
                        split,
                        id,
                        &name,
                        stack.unsigned_abs(),
                        (fx + at.0, fy + at.1),
                    );
                } else {
                    self.offer(id, 1);
                }
            }
            TradeAsk::Card(id) => {
                let card = id.and_then(|id| self.trade_card(id));
                if let Some(c) = &card {
                    emit(&mut self.host.root, PanelOutput::SelectObject(c.id));
                }
                if let Some(f) = self.host.root.widget_mut::<TradeFrame>(frame) {
                    f.card = card;
                }
            }
            TradeAsk::Details(id) => self.trade_send(ItemCommand::Appraise(ObjectRef::Guid(id))),
            TradeAsk::Dropped(p) => self.trade_drop(&p),
        }
    }

    /// Offers `amount` of `id`: the whole item, or part of a stack split
    /// off first.
    fn offer(&mut self, id: u32, amount: i32) {
        if let Some(t) = self.trade_mut() {
            t.own_change = true;
        }
        let Some(o) = self.facts.objects.get(id).cloned() else {
            return;
        };
        if amount >= o.stack_size.max(1) {
            self.trade_send(ItemCommand::AddToTrade(ObjectRef::Guid(id)));
            return;
        }
        match PendingSplit::start(&o, amount, self.facts.state.can_make_request) {
            Ok((pending, command, _)) => {
                if let Some(t) = self.trade_mut() {
                    t.pending = Some(pending);
                }
                emit(&mut self.host.root, PanelOutput::Session(command));
            }
            Err(_) => emit(
                &mut self.host.root,
                PanelOutput::Notice("Cannot split the stack to offer it".to_owned()),
            ),
        }
    }

    fn trade_drop(&mut self, p: &ItemDragPayload) {
        if let Some(t) = self.trade_mut() {
            t.own_change = true;
        }
        self.trade_send(ItemCommand::AddToTrade(ObjectRef::Guid(p.obj_id)));
    }

    fn trade_card(&mut self, id: u32) -> Option<TradeCard> {
        let theirs = self.facts.trade.partner_items.contains(&id);
        let names = crate::ui::facts::composed_names(&self.status.names);
        let Self { facts, icons, .. } = self;
        let o = facts.objects.get(id)?;
        let mut facts_rows = vec![(
            "Value".to_owned(),
            format!("{}p", ac_runtime::inventory::shop::group_thousands(o.value)),
        )];
        if o.workmanship > 0.0 {
            facts_rows.push(("Workmanship".to_owned(), format!("{:.0}", o.workmanship)));
        }
        if o.stack_size > 1 {
            facts_rows.push(("In stack".to_owned(), o.stack_size.to_string()));
        }
        facts_rows.push(("Burden".to_owned(), o.burden.to_string()));
        Some(TradeCard {
            id,
            icon: icons.item_icon(
                o.item_type,
                o.icon_id,
                o.icon_underlay_id,
                o.icon_overlay_id,
                o.effects,
            ),
            name: names(o),
            sub: super::vendor_model::category_of(o.item_type.bits())
                .map_or_else(|| "Item".to_owned(), |c| c.0.to_owned()),
            facts: facts_rows,
            theirs,
        })
    }

    fn feed_modern_trade(
        &mut self,
        trade: &TradeView,
        frame: ElementId,
        search: ElementId,
        band: ElementId,
    ) {
        let memory = self
            .trade_mut()
            .map(|t| t.memory.clone())
            .unwrap_or_default();
        let picking = self
            .host
            .root
            .widget::<TradeFrame>(frame)
            .is_some_and(|f| f.picking);
        let words = kit::field_text(&self.host.root, search);
        let player = self.facts.state.player;
        let feet = plugin_notes::feet_of(&self.facts.plugin_ui, ac_plugin_ui::ItemWindow::Trade);
        let band_h = plugin_notes::band_height(feet.len());
        let Self {
            facts,
            icons,
            status,
            ..
        } = self;
        let names = crate::ui::facts::composed_names(&status.names);
        let me = facts
            .objects
            .get(player)
            .map(|o| o.name.clone())
            .unwrap_or_default();
        let mut icon = |o: &ac_core::items::ClientObject| {
            icons.item_icon(
                o.item_type,
                o.icon_id,
                o.icon_underlay_id,
                o.icon_overlay_id,
                o.effects,
            )
        };
        let plugin_ui = &facts.plugin_ui;
        let mark = |id: u32| plugin_notes::mark_on(plugin_ui, &Horizon::BRONZE, id);
        let screen = trade_model::screen(
            trade,
            &facts.objects,
            &me,
            &memory,
            &mut icon,
            &names,
            &mark,
        );
        let pick = if picking {
            trade_model::pick_rows(
                &facts.objects,
                player,
                &trade.self_items,
                &words,
                &mut icon,
                &names,
            )
        } else {
            Vec::new()
        };
        let ui = &mut self.host.root;
        let band_top = {
            let Some(f) = ui.widget_mut::<TradeFrame>(frame) else {
                return;
            };
            f.screen = screen;
            f.pick = pick;
            f.pick_empty = words.is_empty();
            f.band = band_h;
            if let Some(c) = &f.card
                && !trade.self_items.contains(&c.id)
                && !trade.partner_items.contains(&c.id)
            {
                f.card = None;
            }
            f.band_top()
        };
        FootBand::show(ui, band, feet, band_top, super::trade_window::WIDTH);
    }
}
