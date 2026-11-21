//! The pre-game screens on Modern, in place of retail's: the connecting
//! screen and character selection (the Stage), the credits, and character
//! creation (the Studio). They show by the same rules as retail's
//! (`ac_ui::pregame::connection::ConnectionPresence`,
//! `ac_ui::pregame::characters::selection_shows`), cover the whole window,
//! and answer with the session commands retail's screens send.
//!
//! OPENAC: what the Stage shows of a character beyond its name (level,
//! heritage, look, last place, when last played) is remembered on this
//! computer while the character plays ([`WindowUi::remember_modern_character`]).

pub mod backdrops;
pub mod credits;
pub mod scenery;
pub mod stage;
pub mod studio;

use std::collections::HashMap;

use ac_clock::Instant;
use ac_core::chargen::ChargenOptions;
use ac_core::properties::PropertyInt;
use ac_host::settings::{RememberedCharacter, RememberedCharacters};
use ac_runtime::session::pregame::{ConnectionStatus, ConnectionView};
use ac_runtime::session::selection::{SelectionLifecycle, SelectionOperation, SelectionSnapshot};
use ac_runtime::session::{CharacterSelector, Generation, LifecycleCommand, SessionCommand};
use ac_ui::bundled_font::Weight;
use ac_ui::geometry::AnchorEdges;
use ac_ui::modern::Horizon;
use ac_ui::panels::PanelOutput;
use ac_ui::panels::items::emit;
use ac_ui::pregame::characters::selection_shows;
use ac_ui::pregame::connection::ConnectionPresence;
use ac_ui::pregame::credits::CreditsResources;
use ac_ui::tree::{Element, ElementId};

use self::credits::{CreditsFonts, CreditsView};
use self::scenery::Sky;
use self::stage::{
    Busy, Card, Connecting, Picked, RosterRow, StageAsk, StageFonts, StageMode, StageScreen,
    StageView, ago, countdown,
};
use crate::scene::Scene;
use crate::ui::modern::kit;
use crate::ui::window::WindowUi;
use crate::viewport::ChargenLook;

/// How often a playing character's record is saved again while nothing in
/// it changed (its "last played"), seconds.
const RESAVE_SECONDS: f64 = 300.0;

/// The modern pre-game screens and what they remember between frames.
#[derive(Debug)]
pub struct PreGameUi {
    /// The Stage (connecting, character selection).
    pub stage: ElementId,
    /// The picked character's figure on the Stage.
    pub figure: ElementId,
    /// The delete card's field.
    pub delete_field: ElementId,
    /// The credits.
    pub credits: ElementId,
    /// Character creation.
    pub studio: Option<studio::StudioUi>,
    /// The figure the Stage shows this frame.
    pub figure_look: Option<ChargenLook>,
    presence: ConnectionPresence,
    credits_open: bool,
    credits_resources: Option<CreditsResources>,
    exit_card: bool,
    enter_sent: Option<(Generation, u64)>,
    restore_sent: Option<(Generation, u64)>,
    remembered: Option<RememberedCharacters>,
    /// When each pending deletion was first seen: its seconds then and the
    /// clock.
    pending_seen: HashMap<u32, (u32, f64)>,
    /// When what the Stage shows of the character playing was last looked
    /// at (at most once a second).
    remembered_at: Option<Instant>,
    /// The pictures behind the Stage and the Studio.
    pub backdrops: backdrops::Slideshow,
    /// What they draw this frame.
    pub backdrop: backdrops::Backdrop,
    delete_word: String,
    exit_words: String,
    epoch: Instant,
    /// The record last saved for the character playing, and when.
    saved: Option<(String, RememberedCharacter, f64)>,
    active: bool,
}

impl PreGameUi {
    /// Seconds since the screens were made.
    fn clock(&self) -> f64 {
        Instant::now()
            .saturating_duration_since(self.epoch)
            .as_secs_f64()
    }

    /// Seconds since the screens were made (their spinners' clock).
    #[must_use]
    pub fn clock_now(&self) -> f64 {
        self.clock()
    }

    /// Whether a pre-game screen shows.
    #[must_use]
    pub fn is_active(&self) -> bool {
        self.active
    }
}

fn selection_key(s: &SelectionSnapshot) -> (Generation, u64) {
    (s.generation, s.revision)
}

fn lifecycle(ui: &mut ac_ui::UiRoot, command: LifecycleCommand) {
    emit(ui, PanelOutput::Session(SessionCommand::Lifecycle(command)));
}

/// "Female Aluvian": the gender's and the heritage's names in the game's
/// words.
#[must_use]
pub fn who(options: Option<&ChargenOptions>, heritage: u32, gender: u32) -> String {
    let Some(h) = options.and_then(|o| o.try_get_heritage(heritage)) else {
        return String::new();
    };
    let g = i32::try_from(gender)
        .ok()
        .and_then(|k| h.genders_by_key.get(&k))
        .map(|g| g.name.clone())
        .unwrap_or_default();
    let name = studio::model::heritage_name(heritage, &h.name);
    if g.is_empty() {
        name
    } else {
        format!("{g} {name}")
    }
}

/// The connecting screen's words for a connection `view` to `world`.
#[must_use]
pub fn connecting(view: Option<&ConnectionView>, world: &str) -> Connecting {
    let Some(v) = view else {
        return Connecting {
            world: world.to_owned(),
            connection_words: "Connecting".to_owned(),
            data_words: "Waiting".to_owned(),
            ..Connecting::default()
        };
    };
    let failed = matches!(
        v.status,
        ConnectionStatus::Failed | ConnectionStatus::Unsupported
    );
    let connection_words = if failed {
        "Failed"
    } else if v.connection_progress >= 1.0 {
        "Connected"
    } else {
        "Connecting"
    };
    let data_words = if failed {
        "Waiting"
    } else if v.update_progress >= 1.0 {
        "Up to date"
    } else if v.status == ConnectionStatus::CheckingData {
        "Checking for data updates"
    } else {
        "Waiting"
    };
    Connecting {
        world: world.to_owned(),
        connection: v.connection_progress,
        data: v.update_progress,
        connection_words: connection_words.to_owned(),
        data_words: data_words.to_owned(),
        failed,
        error: v.error.clone().unwrap_or_default(),
    }
}

/// What the Stage shows of selection `view`, with what this computer
/// remembers and the countdowns as of `clock`.
#[allow(clippy::too_many_arguments)]
fn stage_screen(
    view: &SelectionSnapshot,
    remembered: Option<&RememberedCharacters>,
    options: Option<&ChargenOptions>,
    pending_seen: &HashMap<u32, (u32, f64)>,
    clock: f64,
    now_secs: u64,
) -> StageScreen {
    let world = view.world_name.trim().to_owned();
    let left_of = |id: u32, seconds: u32| {
        let (at, seen) = pending_seen.get(&id).copied().unwrap_or((seconds, clock));
        let gone = (clock - seen).max(0.0) as u64;
        countdown(u64::from(at).saturating_sub(gone))
    };
    let rows: Vec<RosterRow> = view
        .entries
        .iter()
        .map(|e| {
            let memory = remembered.and_then(|r| r.of(&format!("{world}/{}", e.name)));
            let pending = e.is_pending_delete();
            let who = memory.map_or_else(String::new, |m| who(options, m.heritage, m.gender));
            let known = memory.is_some_and(|m| m.level > 0);
            let sub = if !known {
                "Not played on this computer yet".to_owned()
            } else if pending {
                format!("Deleted · {who}")
            } else {
                let place = memory.map_or("", |m| m.place.as_str());
                if place.is_empty() {
                    who
                } else {
                    format!("{who} · {place}")
                }
            };
            RosterRow {
                id: e.character_id,
                name: e.name.clone(),
                known,
                level: memory.map_or_else(String::new, |m| {
                    ac_ui::panels::appraisal::group_thousands(i64::from(m.level))
                }),
                sub,
                pending,
                left: if pending {
                    left_of(e.character_id, e.seconds_greyed_out)
                } else {
                    String::new()
                },
            }
        })
        .collect();
    let picked = view
        .entries
        .iter()
        .find(|e| e.character_id == view.highlighted_character_id)
        .map(|e| {
            let memory = remembered.and_then(|r| r.of(&format!("{world}/{}", e.name)));
            let known = memory.is_some_and(|m| m.level > 0);
            let pending = e.is_pending_delete();
            let (line1, line2, sky) = match memory.filter(|_| known) {
                Some(m) => {
                    let level = ac_ui::panels::appraisal::group_thousands(i64::from(m.level));
                    let who = who(options, m.heritage, m.gender);
                    let line1 = if who.is_empty() {
                        format!("Level {level}")
                    } else {
                        format!("Level {level} · {who}")
                    };
                    let when = ago(m.played, now_secs);
                    let line2 = match (m.place.is_empty(), when.is_empty()) {
                        (false, false) => format!("Last in {} · {when}", m.place),
                        (false, true) => format!("Last in {}", m.place),
                        (true, false) => format!("Last played {when}"),
                        (true, true) => String::new(),
                    };
                    let sky = if pending {
                        Sky::Faded
                    } else {
                        Sky::of_place(&m.place)
                    };
                    (line1, line2, sky)
                }
                None => (
                    "Not played on this computer yet".to_owned(),
                    "Its look and place show here after you play it once.".to_owned(),
                    if pending { Sky::Faded } else { Sky::Unknown },
                ),
            };
            Picked {
                id: e.character_id,
                name: e.name.clone(),
                line1,
                line2,
                known,
                pending,
                left: if pending {
                    left_of(e.character_id, e.seconds_greyed_out)
                } else {
                    String::new()
                },
                sky: Some(sky),
            }
        });
    let slots = view.slot_count.max(0);
    let used = i32::try_from(view.entries.len()).unwrap_or(i32::MAX);
    let free = (slots - used).max(0);
    StageScreen {
        mode: if view.entries.is_empty() {
            StageMode::Empty
        } else {
            StageMode::Choose
        },
        world,
        population: view.server_population,
        account: view.account_name.clone(),
        slots: format!("{used} of {slots} slots"),
        free: match free {
            1 => "1 free slot".to_owned(),
            n => format!("{n} free slots"),
        },
        can_create: view.buttons.can_create,
        rows,
        picked,
        can_enter: view.buttons.can_enter,
        can_delete: view.buttons.can_delete,
        can_restore: view.buttons.can_restore,
        ..StageScreen::default()
    }
}

impl WindowUi {
    /// The modern pre-game screens, hidden, and retail's taken down.
    pub(in crate::ui) fn mount_modern_pregame(&mut self) {
        if let Some(mut screens) = self.pregame.take() {
            screens.dispose(&mut self.host.root, &self.content);
            self.mounted
                .retain(|m| *m != crate::ui::pregame::PREGAME_SCREENS);
        }
        let theme = Horizon::BRONZE;
        let fonts = StageFonts {
            hero: self.modern_font(Weight::SemiBold, 38.0),
            huge: self.modern_font(Weight::SemiBold, 39.0),
            big: self.modern_font(Weight::SemiBold, 16.5),
            strong: self.modern_font(Weight::SemiBold, 13.5),
            body: self.modern_font(Weight::Regular, 13.5),
            small: self.modern_font(Weight::Regular, 11.5),
            label: self.modern_font(Weight::SemiBold, 9.0),
        };
        let credits_fonts = CreditsFonts {
            title: self.modern_font(Weight::SemiBold, 16.0),
            text: self.modern_font(Weight::Regular, 14.0),
            small: self.modern_font(Weight::Medium, 12.5),
        };
        let field_font = self.modern_font(Weight::SemiBold, 14.0);
        let credits_resources = match ac_ui::pregame::credits_resources(&self.content) {
            Ok((_, r)) => Some(r),
            Err(e) => {
                self.notices.push(format!("ui: modern credits: {e}"));
                None
            }
        };
        let strings = ac_ui::pregame::character_strings(self.content.dats());
        let delete_word = strings.as_ref().map_or_else(
            || "DELETE".to_owned(),
            |s| s.delete_response.trim().to_owned(),
        );
        let exit_words = strings.as_ref().map_or_else(
            || "Are you sure you want to leave?".to_owned(),
            |s| s.confirm_exit.trim().to_owned(),
        );
        let ui = &mut self.host.root;
        let (w, h) = ui.size();
        let root = ui.root();
        let mut e = Element::at(0.0, 0.0, w, h).with_anchors(AnchorEdges::NONE);
        e.visible = false;
        e.accepts_focus = true;
        let stage = ui.add(root, e, StageView::new(theme, fonts));
        let figure = ui.add(
            stage,
            Element::at(0.0, 0.0, 10.0, 10.0)
                .with_anchors(AnchorEdges::NONE)
                .with_visible(false),
            ac_ui::widgets::Viewport::default(),
        );
        if let Some(e) = ui.element_mut(figure) {
            e.click_through = true;
        }
        let delete_field = kit::mount_field(
            ui,
            stage,
            (0.0, 0.0, 200.0, 36.0),
            field_font,
            &theme,
            false,
        );
        ui.set_visible(delete_field, false);
        let mut e = Element::at(0.0, 0.0, w, h).with_anchors(AnchorEdges::NONE);
        e.visible = false;
        e.accepts_focus = true;
        let mut view = CreditsView::new(theme, credits_fonts);
        view.resources.clone_from(&credits_resources);
        let credits = ui.add(root, e, view);
        let studio = self.mount_modern_studio();
        let Some(modern) = self.modern.as_mut() else {
            return;
        };
        modern.pregame = Some(PreGameUi {
            stage,
            figure,
            delete_field,
            credits,
            studio,
            figure_look: None,
            presence: ConnectionPresence::default(),
            credits_open: false,
            credits_resources,
            exit_card: false,
            enter_sent: None,
            restore_sent: None,
            remembered: None,
            pending_seen: HashMap::new(),
            remembered_at: None,
            backdrops: backdrops::Slideshow::new(),
            backdrop: backdrops::Backdrop::default(),
            delete_word,
            exit_words,
            epoch: Instant::now(),
            saved: None,
            active: false,
        });
    }

    /// Whether a modern pre-game screen shows.
    #[must_use]
    pub fn modern_pregame_active(&self) -> bool {
        self.modern
            .as_ref()
            .and_then(|m| m.pregame.as_ref())
            .is_some_and(PreGameUi::is_active)
    }

    /// Each frame: which screen shows (by retail's rules), what it shows,
    /// and what its buttons asked, as session commands.
    #[allow(clippy::too_many_lines)]
    pub(in crate::ui) fn tick_modern_pregame(&mut self, scene: &mut Scene) {
        let Some(p) = self.modern.as_ref().and_then(|m| m.pregame.as_ref()) else {
            return;
        };
        let (stage, figure, delete_field, credits) = (p.stage, p.figure, p.delete_field, p.credits);
        let clock = p.clock();
        let now = Instant::now();
        if p.remembered.is_none()
            && let Some(store) = &self.status.settings_store
        {
            let loaded = store.load_remembered_characters();
            if let Some(p) = self.modern.as_mut().and_then(|m| m.pregame.as_mut()) {
                p.remembered = Some(loaded);
            }
        }
        let state = scene.pregame_mut();
        let refused = !std::mem::take(&mut state.refusals).is_empty();
        let connection = state.connection.clone();
        let selection = state.selection.clone();
        let options = state.options.clone();
        let direct = connection.as_ref().is_some_and(|c| c.direct_launch);
        let world_name = selection
            .as_ref()
            .map(|s| s.world_name.trim().to_owned())
            .filter(|w| !w.is_empty())
            .or_else(|| self.status.server_world.clone())
            .unwrap_or_else(|| "Dereth".to_owned());
        let Some(p) = self.modern.as_mut().and_then(|m| m.pregame.as_mut()) else {
            return;
        };
        if refused {
            p.enter_sent = None;
            p.restore_sent = None;
        }
        let connecting_shows = p.presence.shows(connection.as_ref(), now);
        let choosing = !connecting_shows && selection_shows(selection.as_ref(), direct);
        if !choosing {
            p.credits_open = false;
            p.exit_card = false;
        }
        // The countdowns of pending deletions run from when they were seen.
        if let Some(s) = &selection {
            for e in &s.entries {
                if e.is_pending_delete() {
                    p.pending_seen
                        .entry(e.character_id)
                        .and_modify(|seen| {
                            if seen.0 != e.seconds_greyed_out {
                                *seen = (e.seconds_greyed_out, clock);
                            }
                        })
                        .or_insert((e.seconds_greyed_out, clock));
                } else {
                    p.pending_seen.remove(&e.character_id);
                }
            }
        }
        let studio_open = p.studio.as_ref().is_some_and(|s| s.open);
        let now_secs = ac_clock::unix_time().as_secs();
        let mut screen = match (&selection, choosing) {
            (_, false) if connecting_shows => StageScreen {
                mode: StageMode::Connecting(connecting(connection.as_ref(), &world_name)),
                ..StageScreen::default()
            },
            (Some(s), true) => stage_screen(
                s,
                p.remembered.as_ref(),
                options.as_deref(),
                &p.pending_seen,
                clock,
                now_secs,
            ),
            _ => StageScreen::default(),
        };
        screen.version = crate::ui::pregame::version_line();
        screen.delete_word.clone_from(&p.delete_word);
        screen.exit_words.clone_from(&p.exit_words);
        if let Some(s) = selection.as_ref().filter(|_| choosing) {
            let current = selection_key(s);
            if p.enter_sent.is_some_and(|k| k != current) {
                p.enter_sent = None;
            }
            if p.restore_sent.is_some_and(|k| k != current) {
                p.restore_sent = None;
            }
            screen.busy =
                if s.lifecycle == SelectionLifecycle::EnteringWorld || p.enter_sent.is_some() {
                    Busy::Entering
                } else if p.restore_sent.is_some()
                    || matches!(
                        s.operation,
                        SelectionOperation::DeleteRequested
                            | SelectionOperation::DeleteAcknowledged
                            | SelectionOperation::RestoreRequested
                    )
                {
                    Busy::Wait
                } else {
                    Busy::None
                };
            screen.card = if let Some(error) = &s.error {
                Card::Error(error.message.trim().to_owned())
            } else if let Some(entry) = (s.pending_delete_character_id != 0)
                .then(|| {
                    s.entries
                        .iter()
                        .find(|e| e.character_id == s.pending_delete_character_id)
                })
                .flatten()
            {
                Card::Delete(entry.name.clone())
            } else if p.exit_card {
                Card::Exit
            } else {
                Card::None
            };
        }
        // The figure: the picked character's look as remembered.
        let look = selection.as_ref().filter(|_| choosing).and_then(|s| {
            let e = s
                .entries
                .iter()
                .find(|e| e.character_id == s.highlighted_character_id)?;
            let m = p
                .remembered
                .as_ref()?
                .of(&format!("{}/{}", s.world_name.trim(), e.name))?;
            Some(crate::ui::viewports::stage_look(
                m.look.as_ref()?,
                m.heritage,
                m.gender,
            ))
        });
        p.figure_look = look;
        let card_delete = matches!(screen.card, Card::Delete(_));
        let stage_shows = (connecting_shows || choosing) && !p.credits_open && !studio_open;
        let credits_shows = choosing && p.credits_open && !studio_open;
        let credits_resources = p.credits_resources.clone();
        // The pictures run while the Stage or the Studio shows, and are let
        // go otherwise.
        if stage_shows || studio_open {
            p.backdrop = p.backdrops.tick(&self.images, now);
        } else if !p.backdrop.layers.is_empty() {
            p.backdrops.release(&self.images);
            p.backdrop = backdrops::Backdrop::default();
        }
        let backdrop = p.backdrop.clone();
        let ui = &mut self.host.root;
        let size = ui.size();
        for (id, shows) in [(stage, stage_shows), (credits, credits_shows)] {
            let was = ui.element(id).is_some_and(|e| e.visible);
            if let Some(e) = ui.element_mut(id) {
                e.width = size.0;
                e.height = size.1;
            }
            if shows != was {
                ui.set_visible(id, shows);
                if shows {
                    ui.bring_to_front(id);
                    ui.set_keyboard_focus(Some(id));
                }
            }
        }
        // The delete card's field shows with the card.
        let typed = kit::field_text(ui, delete_field);
        let (field_rect, figure_rect, asks, picked_known) = match ui.widget_mut::<StageView>(stage)
        {
            Some(v) => {
                v.size = size;
                v.clock = clock;
                v.typed.clone_from(&typed);
                let known = screen.picked.as_ref().is_some_and(|p| p.known);
                v.screen = screen.clone();
                v.backdrop = backdrop;
                (
                    v.delete_field_rect(),
                    v.figure_rect(),
                    std::mem::take(&mut v.asks),
                    known,
                )
            }
            None => return,
        };
        let field_shows = stage_shows && card_delete;
        if ui.element(delete_field).is_some_and(|e| e.visible) != field_shows {
            ui.set_visible(delete_field, field_shows);
            if field_shows {
                kit::set_field_text(ui, delete_field, "");
                ui.set_keyboard_focus(Some(delete_field));
            }
        }
        if let Some(e) = ui.element_mut(delete_field) {
            e.left = field_rect.0 + 6.0;
            e.top = field_rect.1 + (field_rect.3 - 24.0) * 0.5;
            e.width = field_rect.2 - 12.0;
        }
        let figure_shows = stage_shows
            && matches!(screen.mode, StageMode::Choose)
            && picked_known
            && screen.card == Card::None;
        if let Some(e) = ui.element_mut(figure) {
            e.left = figure_rect.0;
            e.top = figure_rect.1;
            e.width = figure_rect.2;
            e.height = figure_rect.3;
        }
        if ui.element(figure).is_some_and(|e| e.visible) != figure_shows {
            ui.set_visible(figure, figure_shows);
        }
        if let Some(v) = ui.widget_mut::<CreditsView>(credits) {
            v.size = size;
            v.clock = clock;
            if v.resources.is_none() {
                v.resources = credits_resources;
            }
        }
        let leave_credits = ui
            .widget_mut::<CreditsView>(credits)
            .is_some_and(|v| std::mem::take(&mut v.leave));
        // What the screens asked.
        let mut open_studio = false;
        for ask in asks {
            let ui = &mut self.host.root;
            let Some(p) = self.modern.as_mut().and_then(|m| m.pregame.as_mut()) else {
                return;
            };
            let current = selection.as_ref().map(selection_key);
            let highlighted = selection.as_ref().map_or(0, |s| s.highlighted_character_id);
            match ask {
                StageAsk::Pick(id) => {
                    if id != highlighted {
                        lifecycle(ui, LifecycleCommand::Highlight(CharacterSelector::Id(id)));
                    }
                }
                StageAsk::Enter(id) => {
                    let can = selection.as_ref().is_some_and(|s| {
                        s.entries
                            .iter()
                            .any(|e| e.character_id == id && e.can_enter())
                    });
                    if can && p.enter_sent.is_none() {
                        if id != highlighted {
                            lifecycle(ui, LifecycleCommand::Highlight(CharacterSelector::Id(id)));
                        }
                        p.enter_sent = current;
                        lifecycle(ui, LifecycleCommand::Enter(Some(CharacterSelector::Id(id))));
                    }
                }
                StageAsk::Delete => {
                    if screen.can_delete {
                        lifecycle(ui, LifecycleCommand::RequestDelete(None));
                    }
                }
                StageAsk::ConfirmDelete => {
                    if typed.trim().eq_ignore_ascii_case(&p.delete_word) {
                        lifecycle(ui, LifecycleCommand::ConfirmDelete);
                    }
                }
                StageAsk::CloseCard => {
                    if p.exit_card {
                        p.exit_card = false;
                    } else {
                        lifecycle(ui, LifecycleCommand::Cancel);
                    }
                }
                StageAsk::Restore => {
                    if screen.can_restore {
                        p.restore_sent = current;
                        lifecycle(ui, LifecycleCommand::Restore(None));
                    }
                }
                StageAsk::Create => {
                    if screen.can_create {
                        open_studio = true;
                    }
                }
                StageAsk::Credits => p.credits_open = true,
                StageAsk::Exit => p.exit_card = true,
                StageAsk::ConfirmExit | StageAsk::Cancel => {
                    emit(ui, PanelOutput::Session(SessionCommand::Shutdown));
                }
            }
        }
        if leave_credits && let Some(p) = self.modern.as_mut().and_then(|m| m.pregame.as_mut()) {
            p.credits_open = false;
        }
        if let Some(p) = self.modern.as_mut().and_then(|m| m.pregame.as_mut())
            && p.credits_open
            && !credits_shows
        {
            // Opened this frame: the clock starts now.
            if let Some(v) = self.host.root.widget_mut::<CreditsView>(credits) {
                v.started = clock;
                v.leave = false;
            }
        }
        if open_studio {
            self.open_modern_studio(scene);
        }
        self.tick_modern_studio(scene, choosing);
        let Some(p) = self.modern.as_mut().and_then(|m| m.pregame.as_mut()) else {
            return;
        };
        let studio_open = p.studio.as_ref().is_some_and(|s| s.open);
        p.active = connecting_shows || choosing || studio_open;
        let active = p.active;
        self.hide_in_game_windows(active);
        if active {
            if self.host.root.default_text_input.is_some() {
                self.parked_text_input = self.host.root.default_text_input.take();
            }
        } else if let Some(input) = self.parked_text_input.take()
            && self.host.root.exists(input)
        {
            self.host.root.default_text_input = Some(input);
        }
        self.remember_modern_character(scene);
    }

    /// A key while a modern pre-game screen shows: Escape closes a card
    /// (or asks to leave), Enter enters the world or answers the card, the
    /// arrows walk the roster, and any key leaves the credits. True when
    /// the key was the screens'.
    pub(in crate::ui) fn modern_pregame_key(&mut self, key: ac_ui::event::Key) -> bool {
        use ac_ui::event::Key;
        let Some(p) = self.modern.as_ref().and_then(|m| m.pregame.as_ref()) else {
            return false;
        };
        if !p.active {
            return false;
        }
        if p.studio.as_ref().is_some_and(|s| s.open) {
            return self.modern_studio_key(key);
        }
        let (stage, credits) = (p.stage, p.credits);
        let ui = &mut self.host.root;
        if ui.element(credits).is_some_and(|e| e.visible) {
            if let Some(v) = ui.widget_mut::<CreditsView>(credits) {
                v.leave = true;
            }
            return true;
        }
        let Some(v) = ui.widget_mut::<StageView>(stage) else {
            return false;
        };
        let s = &v.screen;
        let card = s.card.clone();
        let picked = s.picked.as_ref().map(|p| (p.id, p.pending));
        let rows: Vec<u32> = s.rows.iter().map(|r| r.id).collect();
        let ask = match key {
            Key::Escape => Some(match card {
                Card::None => StageAsk::Exit,
                _ => StageAsk::CloseCard,
            }),
            Key::Enter => match card {
                Card::Delete(_) => Some(StageAsk::ConfirmDelete),
                Card::Exit => Some(StageAsk::ConfirmExit),
                Card::Error(_) => Some(StageAsk::CloseCard),
                Card::None => match picked {
                    Some((id, false)) if matches!(s.mode, StageMode::Choose) => {
                        Some(StageAsk::Enter(id))
                    }
                    _ => None,
                },
            },
            Key::Up | Key::Down if card == Card::None && !rows.is_empty() => {
                let at = picked
                    .and_then(|(id, _)| rows.iter().position(|r| *r == id))
                    .unwrap_or(0);
                let next = if key == Key::Up {
                    at.saturating_sub(1)
                } else {
                    (at + 1).min(rows.len() - 1)
                };
                Some(StageAsk::Pick(rows[next]))
            }
            _ => None,
        };
        match ask {
            Some(a) => {
                v.asks.push(a);
                true
            }
            None => false,
        }
    }

    /// An int of the player's: his object's property, else his own
    /// numbers'.
    fn player_int(&self, key: PropertyInt) -> i32 {
        let player = self.facts.state.player;
        let from_object = self
            .facts
            .objects
            .get(player)
            .map_or(0, |o| o.properties.get_int(key.0, 0));
        if from_object > 0 {
            return from_object;
        }
        self.facts
            .numbers()
            .and_then(|n| n.int_properties().get(&key.0).copied())
            .unwrap_or(0)
    }

    /// While a character plays on Modern, what the Stage will show of it
    /// is kept on this computer: its level, heritage, gender, the town
    /// nearest where it was last outside, its look, and when (saved when
    /// any of these change, and every few minutes).
    pub(in crate::ui) fn remember_modern_character(&mut self, scene: &Scene) {
        let (Some(name), Some(world)) = (
            self.status.playing.clone(),
            self.status.server_world.clone(),
        ) else {
            return;
        };
        if self.modern_pregame_active() || !scene.in_world() {
            return;
        }
        let now = Instant::now();
        let Some(p) = self.modern.as_mut().and_then(|m| m.pregame.as_mut()) else {
            return;
        };
        if p.remembered_at
            .is_some_and(|at| now.duration_since(at).as_secs_f32() < 1.0)
        {
            return;
        }
        p.remembered_at = Some(now);
        let Some(player) = scene.player() else {
            return;
        };
        let Some(setup) = player.setup_id else {
            return;
        };
        let look = crate::ui::viewports::remembered_look(&crate::appearance::AppearanceKey::new(
            setup,
            &player.model,
        ));
        let level = u32::try_from(self.player_int(PropertyInt::LEVEL)).unwrap_or(0);
        if level == 0 {
            return;
        }
        let heritage = u32::try_from(self.player_int(PropertyInt::HERITAGE_GROUP)).unwrap_or(0);
        let gender = u32::try_from(self.player_int(PropertyInt::GENDER)).unwrap_or(0);
        let place = self.nearest_town().unwrap_or_default();
        let key = format!("{world}/{name}");
        let record = RememberedCharacter {
            level,
            heritage,
            gender,
            place,
            played: 0,
            look: Some(look),
        };
        let store = self.status.settings_store.clone();
        let Some(p) = self.modern.as_mut().and_then(|m| m.pregame.as_mut()) else {
            return;
        };
        let clock = p.clock();
        let due = match &p.saved {
            Some((k, r, at)) => *k != key || *r != record || clock - at >= RESAVE_SECONDS,
            None => true,
        };
        if !due {
            return;
        }
        p.saved = Some((key.clone(), record.clone(), clock));
        let remembered = p.remembered.get_or_insert_with(|| {
            store
                .as_ref()
                .map(ac_host::settings::SettingsStore::load_remembered_characters)
                .unwrap_or_default()
        });
        let mut kept = record;
        kept.played = ac_clock::unix_time().as_secs();
        if kept.place.is_empty()
            && let Some(old) = remembered.of(&key)
        {
            kept.place.clone_from(&old.place);
        }
        remembered.set(&key, kept);
        if let Some(store) = store
            && let Err(e) = store.save_remembered_characters(remembered)
        {
            self.notices
                .push(format!("ui: remembering the character: {e}"));
        }
    }

    /// The town nearest where the player was last outside.
    fn nearest_town(&self) -> Option<String> {
        let m = self.modern.as_ref()?;
        let (x, y) = m.map.as_ref()?.last_outside?;
        let view = self
            .host
            .root
            .widget::<crate::ui::modern::world::map::MapView>(m.map.as_ref()?.view)?;
        view.towns()
            .iter()
            .min_by(|a, b| {
                let da = (a.x - x).powi(2) + (a.y - y).powi(2);
                let db = (b.x - x).powi(2) + (b.y - y).powi(2);
                da.total_cmp(&db)
            })
            .map(|t| t.name.to_owned())
    }
}

#[cfg(test)]
mod tests {
    use ac_runtime::session::pregame::{ConnectionStatus, ConnectionView};
    use ac_runtime::session::selection::{SelectionEntry, SelectionSnapshot};

    use super::*;

    fn snapshot() -> SelectionSnapshot {
        let mut s = SelectionSnapshot {
            world_name: "Leafcull".to_owned(),
            account_name: "ashfall".to_owned(),
            slot_count: 11,
            server_population: Some(312),
            highlighted_character_id: 2,
            ..SelectionSnapshot::default()
        };
        s.entries = vec![
            SelectionEntry {
                active_index: 0,
                character_id: 1,
                name: "Aerin Vale".to_owned(),
                seconds_greyed_out: 0,
            },
            SelectionEntry {
                active_index: 1,
                character_id: 2,
                name: "Quill".to_owned(),
                seconds_greyed_out: 0,
            },
            SelectionEntry {
                active_index: 2,
                character_id: 3,
                name: "Old Mule".to_owned(),
                seconds_greyed_out: 2832,
            },
        ];
        s
    }

    #[test]
    fn the_stage_shows_what_this_computer_remembers() {
        let mut remembered = RememberedCharacters::default();
        remembered.set(
            "Leafcull/Aerin Vale",
            RememberedCharacter {
                level: 4783,
                place: "Ayan Baqur".to_owned(),
                played: 1_800_000_000 - 86_400,
                ..RememberedCharacter::default()
            },
        );
        let mut s = snapshot();
        s.highlighted_character_id = 1;
        let screen = stage_screen(
            &s,
            Some(&remembered),
            None,
            &HashMap::new(),
            0.0,
            1_800_000_000,
        );
        assert_eq!(screen.slots, "3 of 11 slots");
        assert_eq!(screen.free, "8 free slots");
        assert_eq!(screen.rows[0].level, "4,783");
        assert_eq!(screen.rows[1].sub, "Not played on this computer yet");
        assert!(screen.rows[2].pending);
        assert_eq!(screen.rows[2].left, "47:12");
        let picked = screen.picked.expect("one picked");
        assert_eq!(picked.line1, "Level 4,783");
        assert_eq!(picked.line2, "Last in Ayan Baqur · yesterday");
        assert_eq!(picked.sky, Some(Sky::Desert));
    }

    #[test]
    fn a_pending_deletion_counts_down_from_when_it_was_seen() {
        let s = snapshot();
        let seen: HashMap<u32, (u32, f64)> = [(3, (2832, 10.0))].into_iter().collect();
        let screen = stage_screen(&s, None, None, &seen, 70.0, 0);
        assert_eq!(screen.rows[2].left, "46:12");
    }

    #[test]
    fn an_account_with_no_characters_is_the_empty_stage() {
        let mut s = snapshot();
        s.entries.clear();
        let screen = stage_screen(&s, None, None, &HashMap::new(), 0.0, 0);
        assert_eq!(screen.mode, StageMode::Empty);
        assert_eq!(screen.free, "11 free slots");
    }

    #[test]
    fn the_connecting_words_follow_the_connection() {
        let mut v = ConnectionView {
            status: ConnectionStatus::CheckingData,
            connection_progress: 1.0,
            update_progress: 0.4,
            error: None,
            started: Instant::now(),
            direct_launch: false,
        };
        let c = connecting(Some(&v), "Leafcull");
        assert_eq!(
            (c.connection_words.as_str(), c.data_words.as_str()),
            ("Connected", "Checking for data updates")
        );
        v.status = ConnectionStatus::Failed;
        v.error = Some("The server did not answer.".to_owned());
        let c = connecting(Some(&v), "Leafcull");
        assert!(c.failed);
        assert_eq!(c.connection_words, "Failed");
        assert_eq!(c.error, "The server did not answer.");
    }
}
