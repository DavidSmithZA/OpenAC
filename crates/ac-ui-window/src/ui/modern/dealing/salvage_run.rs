//! The Salvage window in the game window: it opens when a salvaging tool
//! is used, stages what the player drops or adds (with the retail rules,
//! shared), salvages, and shows the bags the salvage produced.

use ac_core::items::PublicWeenieFlags;
use ac_runtime::character::option_id;
use ac_runtime::inventory::{ItemCommand, ObjectRef};
use ac_runtime::session::SessionCommand;
use ac_ui::bundled_font::Weight;
use ac_ui::event::ItemDragPayload;
use ac_ui::icons::IconSource;
use ac_ui::modern::Horizon;
use ac_ui::panels::PanelOutput;
use ac_ui::panels::items::{ItemsView, emit};
use ac_ui::panels::salvage_list::SalvageList;
use ac_ui::tree::ElementId;

use super::salvage_model::{self, SALVAGING_SKILL, SalvageDone, refusal};
use super::salvage_window::{SalvageAsk, SalvageFonts, SalvageFrame};
use crate::ui::modern::hud;
use crate::ui::window::WindowUi;

/// The window's name.
pub const SALVAGE: &str = "modern-salvage";

/// The Salvage window's elements and what it remembers.
#[derive(Debug)]
pub struct SalvageUi {
    /// The window.
    pub frame: ElementId,
    /// What is staged, and the tool.
    pub staging: SalvageList,
    /// Add from packs' material (0: all).
    chip: u32,
    /// How many of each material the last salvage sent, until its answer.
    sent: Option<Vec<(u32, usize)>>,
    /// What the last salvage produced (shown in place of the staging).
    done: Option<SalvageDone>,
}

impl WindowUi {
    /// The Salvage window, hidden, in place of retail's salvage window.
    pub(in crate::ui) fn mount_modern_salvage(&mut self) {
        let theme = Horizon::BRONZE;
        let fonts = SalvageFonts {
            title: self.modern_font(Weight::SemiBold, 15.0),
            strong: self.modern_font(Weight::SemiBold, 12.5),
            small: self.modern_font(Weight::Regular, 11.0),
            tiny: self.modern_font(Weight::SemiBold, 10.5),
            body: self.modern_font(Weight::Regular, 12.0),
        };
        let frame = match hud::mount_piece(
            &mut self.host.root,
            SALVAGE,
            (super::salvage_window::WIDTH, super::salvage_window::HEIGHT),
            SalvageFrame::new(theme, fonts),
        ) {
            Ok(id) => id,
            Err(e) => {
                self.notices.push(format!("ui: modern salvage: {e}"));
                return;
            }
        };
        let ui = &mut self.host.root;
        if let Some(e) = ui.element_mut(frame) {
            e.draggable = true;
        }
        ui.hide_window(SALVAGE);
        let Some(modern) = self.modern.as_mut() else {
            return;
        };
        modern.salvage = Some(SalvageUi {
            frame,
            staging: SalvageList::default(),
            chip: 0,
            sent: None,
            done: None,
        });
        modern.replace(crate::ui::salvage_window::SALVAGE_WINDOW);
    }

    fn salvage_mut(&mut self) -> Option<&mut SalvageUi> {
        self.modern.as_mut().and_then(|m| m.salvage.as_mut())
    }

    fn with_salvage_items<R>(&self, f: impl FnOnce(&ItemsView<'_>) -> R) -> R {
        let selected = self.player_view.and_then(|v| v.selected);
        let pending: Vec<u32> = self.facts.state.pending_item.into_iter().collect();
        let resolve = crate::ui::facts::composed_names(&self.status.names);
        let view = self.facts.items_view(selected, &pending, None, &resolve);
        f(&view)
    }

    /// Each frame (before retail's salvage window reads them): the tools
    /// used open the window; what was asked is done; a salvage's answer
    /// shows its bags.
    #[allow(clippy::too_many_lines)]
    pub(in crate::ui) fn tick_modern_salvage(&mut self) {
        let Some(frame) = self
            .modern
            .as_ref()
            .and_then(|m| m.salvage.as_ref())
            .map(|s| s.frame)
        else {
            return;
        };
        let opened = std::mem::take(&mut self.facts.salvage_opens);
        let outcomes = std::mem::take(&mut self.facts.salvaged);
        let allow_multiple = self
            .facts
            .social
            .as_ref()
            .is_some_and(|s| s.options.get(option_id::SALVAGE_MULTIPLE));
        // A tool used: the window opens on it, empty.
        for tool in opened {
            let mut staging = self
                .salvage_mut()
                .map(|s| std::mem::take(&mut s.staging))
                .unwrap_or_default();
            let ok = self.with_salvage_items(|view| staging.open(tool, view));
            if let Some(s) = self.salvage_mut() {
                s.staging = staging;
                if ok {
                    s.done = None;
                    s.sent = None;
                }
            }
            if ok {
                let ui = &mut self.host.root;
                ui.show_window(SALVAGE);
                ui.bring_to_front(frame);
                if let Some(f) = ui.widget_mut::<SalvageFrame>(frame) {
                    f.picking = false;
                    f.refused = None;
                }
            }
        }
        // A salvage's answer.
        for outcome in outcomes {
            let sent = self.salvage_mut().and_then(|s| s.sent.take());
            if let Some(from) = sent {
                let names = crate::ui::facts::composed_names(&self.status.names);
                let refused: Vec<String> = outcome
                    .unsuitable
                    .iter()
                    .map(|&id| {
                        self.facts
                            .objects
                            .get(id)
                            .map_or_else(|| "item".to_owned(), names)
                    })
                    .collect();
                let done = salvage_model::done(&outcome, &from, &refused);
                if let Some(s) = self.salvage_mut() {
                    s.done = Some(done);
                }
            }
        }
        let Some(mut staging) = self.salvage_mut().map(|s| std::mem::take(&mut s.staging)) else {
            return;
        };
        if staging.tool() == 0 {
            self.salvage_finish(staging, frame);
            return;
        }
        if !self.with_salvage_items(|view| staging.follow(view)) {
            staging.forget();
            self.host.root.hide_window(SALVAGE);
            self.salvage_finish(staging, frame);
            return;
        }
        // What the frame asked for.
        let asks = self
            .host
            .root
            .widget_mut::<SalvageFrame>(frame)
            .map(|f| std::mem::take(&mut f.asks))
            .unwrap_or_default();
        for ask in asks {
            match ask {
                SalvageAsk::Close => {
                    staging.forget();
                    self.host.root.hide_window(SALVAGE);
                }
                SalvageAsk::Pick(on) => {
                    if let Some(f) = self.host.root.widget_mut::<SalvageFrame>(frame) {
                        f.picking = on;
                    }
                }
                SalvageAsk::Chip(m) => {
                    if let Some(s) = self.salvage_mut() {
                        s.chip = m;
                    }
                }
                SalvageAsk::Add(id) => self.salvage_add(&mut staging, id, allow_multiple, frame),
                SalvageAsk::Dropped(p) => {
                    self.salvage_drop(&mut staging, &p, allow_multiple, frame);
                }
                SalvageAsk::Remove(id) => staging.remove(id),
                SalvageAsk::Inspect(id) => emit(
                    &mut self.host.root,
                    PanelOutput::Session(SessionCommand::Item(ItemCommand::Appraise(
                        ObjectRef::Guid(id),
                    ))),
                ),
                SalvageAsk::Clear => staging.clear(),
                SalvageAsk::Salvage => {
                    let mut from: Vec<(u32, usize)> = Vec::new();
                    for id in staging.items() {
                        let m = self
                            .facts
                            .objects
                            .get(*id)
                            .and_then(|o| o.material_type)
                            .unwrap_or(0);
                        match from.iter_mut().find(|(k, _)| *k == m) {
                            Some((_, n)) => *n += 1,
                            None => from.push((m, 1)),
                        }
                    }
                    match self.with_salvage_items(|view| staging.salvage(view)) {
                        Ok(Some(command)) => {
                            emit(&mut self.host.root, PanelOutput::Session(command));
                            if let Some(s) = self.salvage_mut() {
                                s.sent = Some(from);
                            }
                        }
                        Ok(None) => {}
                        Err(words) => {
                            emit(&mut self.host.root, PanelOutput::Notice(words.to_owned()));
                        }
                    }
                    if let Some(f) = self.host.root.widget_mut::<SalvageFrame>(frame) {
                        f.picking = false;
                    }
                }
                SalvageAsk::More => {
                    if let Some(s) = self.salvage_mut() {
                        s.done = None;
                    }
                }
            }
        }
        self.feed_modern_salvage(&staging, frame, allow_multiple);
        self.salvage_finish(staging, frame);
    }

    /// Puts the staging back and tells the session what is marked.
    fn salvage_finish(&mut self, mut staging: SalvageList, _frame: ElementId) {
        if let Some(marks) = staging.marks() {
            emit(&mut self.host.root, PanelOutput::Session(marks));
        }
        if let Some(s) = self.salvage_mut() {
            s.staging = staging;
        }
    }

    fn salvage_add(
        &mut self,
        staging: &mut SalvageList,
        id: u32,
        allow_multiple: bool,
        frame: ElementId,
    ) {
        let added = self.with_salvage_items(|view| staging.add(id, view, allow_multiple));
        match added {
            Some(notices) => {
                for n in notices {
                    emit(&mut self.host.root, PanelOutput::Notice(n));
                }
                if let Some(f) = self.host.root.widget_mut::<SalvageFrame>(frame) {
                    f.refused = None;
                }
            }
            None => self.salvage_refused(staging, id, allow_multiple, frame),
        }
    }

    fn salvage_drop(
        &mut self,
        staging: &mut SalvageList,
        p: &ItemDragPayload,
        allow_multiple: bool,
        frame: ElementId,
    ) {
        self.salvage_add(staging, p.obj_id, allow_multiple, frame);
    }

    /// Shows why `id` could not be staged, where it was dropped.
    fn salvage_refused(
        &mut self,
        staging: &SalvageList,
        id: u32,
        allow_multiple: bool,
        frame: ElementId,
    ) {
        let names = crate::ui::facts::composed_names(&self.status.names);
        let player = self.facts.state.player;
        let Some(o) = self.facts.objects.get(id) else {
            return;
        };
        let why = if id == staging.tool() {
            "That is the tool you are salvaging with.".to_owned()
        } else if !self.facts.objects.is_owned_by_object(id, player) {
            "You can only salvage what you carry.".to_owned()
        } else if staging.items().contains(&id) {
            "It is on the list already.".to_owned()
        } else if o.flags().contains(PublicWeenieFlags::RETAINED) {
            "Retained items cannot be salvaged.".to_owned()
        } else {
            refusal(o, staging.material(), allow_multiple)
                .unwrap_or_else(|| "That cannot be salvaged.".to_owned())
        };
        let name = names(o);
        if let Some(f) = self.host.root.widget_mut::<SalvageFrame>(frame) {
            f.refused = Some((name, why));
        }
    }

    fn feed_modern_salvage(
        &mut self,
        staging: &SalvageList,
        frame: ElementId,
        allow_multiple: bool,
    ) {
        let (chip, done) = self
            .salvage_mut()
            .map(|s| (s.chip, s.done.clone()))
            .unwrap_or_default();
        let picking = self
            .host
            .root
            .widget::<SalvageFrame>(frame)
            .is_some_and(|f| f.picking);
        let player = self.facts.state.player;
        let skill = self
            .facts
            .numbers()
            .and_then(|n| n.get_effective_skill(SALVAGING_SKILL));
        let Self {
            facts,
            icons,
            status,
            ..
        } = self;
        let names = crate::ui::facts::composed_names(&status.names);
        let mut icon = |o: &ac_core::items::ClientObject| {
            icons.item_icon(
                o.item_type,
                o.icon_id,
                o.icon_underlay_id,
                o.icon_overlay_id,
                o.effects,
            )
        };
        let mut screen = salvage_model::screen(
            &facts.objects,
            player,
            staging.tool(),
            staging.items(),
            skill,
            allow_multiple,
            &mut icon,
            &names,
        );
        screen.done = done;
        let (chips, pick) = if picking {
            salvage_model::picks(
                &facts.objects,
                player,
                staging.tool(),
                staging.items(),
                staging.material(),
                allow_multiple,
                chip,
                &mut icon,
                &names,
            )
        } else {
            (Vec::new(), Vec::new())
        };
        if let Some(f) = self.host.root.widget_mut::<SalvageFrame>(frame) {
            f.screen = screen;
            f.chips = chips;
            f.pick = pick;
            f.chip = chip;
        }
    }
}
