//! Character creation on Modern (the Studio), in place of retail's
//! wizard: six steps (Heritage, Profession, Skills, Appearance, Town,
//! Name; Olthoi skip Profession, Skills and Town) around the figure on its
//! plinth. Every choice is the creation request retail's wizard sends;
//! the Studio shows the session's creation state. Opening it rolls a
//! random character and turns its gender to the other one, as retail's
//! does.

pub mod model;
pub mod view;

use ac_core::chargen::ChargenPalSetSource;
use ac_core::chargen::ChargenSkillAdvancementClass as Class;
use ac_runtime::session::creation::{
    AppearanceSlot, ChargenRandom, CreationCommand, CreationRejection, CreationSnapshot,
    SeededRandom, UNSET,
};
use ac_runtime::session::{Generation, LifecycleCommand, SessionCommand};
use ac_ui::bundled_font::Weight;
use ac_ui::draw::Color;
use ac_ui::geometry::AnchorEdges;
use ac_ui::modern::Horizon;
use ac_ui::panels::PanelOutput;
use ac_ui::panels::items::emit;
use ac_ui::pregame::chargen::PageContext;
use ac_ui::pregame::chargen::appearance::{
    Part, PreviewRequest, SwatchSourcesDyn, color_slot, pick_command, shade_slot, step_command,
    style_slot, swatch_colors,
};
use ac_ui::pregame::chargen::heritage::HeritagePage;
use ac_ui::pregame::chargen::profession::ProfessionPage;
use ac_ui::pregame::chargen::skills::{advance, retreat};
use ac_ui::pregame::chargen::summary::name_filter;
use ac_ui::pregame::chargen::town::TownPage;
use ac_ui::strings::compute_hash;
use ac_ui::tree::{Element, ElementId};

use self::model::{Asked, Group, NAME_LIMIT, Step, near_names, rgb, studio_screen};
use self::view::{StudioAsk, StudioCard, StudioFonts, StudioView};
use crate::scene::Scene;
use crate::ui::modern::kit;
use crate::ui::window::WindowUi;
use crate::viewport::ChargenLook;

type Key = (Generation, u64);

fn key(s: &CreationSnapshot) -> Key {
    (s.generation, s.revision)
}

/// The Studio's elements and what it remembers.
#[derive(Debug)]
pub struct StudioUi {
    /// It shows.
    pub open: bool,
    /// The full-screen view.
    pub view: ElementId,
    /// The figure.
    pub figure: ElementId,
    /// The name field.
    pub name_field: ElementId,
    /// The figure this frame, and whether it is the close (face) view.
    pub figure_look: Option<(ChargenLook, bool)>,
    /// Turns and zooms of the figure asked for since the viewports looked.
    pub turns: Vec<PreviewRequest>,
    step: Step,
    group: Group,
    part: Part,
    /// The roll sent on opening, until its answer (then the gender turns).
    roll_sent: Option<Key>,
    /// A finish sent, until its answer.
    finish_sent: Option<Key>,
    /// The refusal last shown.
    refusal_seen: Option<CreationRejection>,
    /// The headgear taken off while the face is shown.
    held_headgear: Option<u32>,
    card: StudioCard,
    last_shade: Option<f32>,
    random: SeededRandom,
    /// The name last sent to the session (sent again only when it changes).
    name_sent: Option<String>,
    /// The creation screens' words, each table read once.
    strings: std::rc::Rc<ac_ui::strings::StringResolver>,
}

fn create(ui: &mut ac_ui::UiRoot, command: CreationCommand) {
    emit(
        ui,
        PanelOutput::Session(SessionCommand::Lifecycle(LifecycleCommand::Create(command))),
    );
}

impl WindowUi {
    /// The Studio, hidden.
    pub(in crate::ui) fn mount_modern_studio(&mut self) -> Option<StudioUi> {
        let theme = Horizon::current();
        let fonts = StudioFonts {
            title: self.modern_font(Weight::SemiBold, 18.0),
            heading: self.modern_font(Weight::SemiBold, 19.0),
            strong: self.modern_font(Weight::SemiBold, 12.5),
            body: self.modern_font(Weight::Regular, 13.0),
            small: self.modern_font(Weight::Regular, 11.5),
            label: self.modern_font(Weight::SemiBold, 9.0),
        };
        let field_font = self.modern_font(Weight::SemiBold, 19.0);
        let dats = self.content.dats().clone();
        let strings = std::rc::Rc::new(ac_ui::strings::StringResolver::new(move |id| {
            dats.get::<ac_dat::objects::StringTable>(id).ok().flatten()
        }));
        self.miss_caches
            .strings
            .push(std::rc::Rc::downgrade(&strings));
        let ui = &mut self.host.root;
        let (w, h) = ui.size();
        let root = ui.root();
        let mut e = Element::at(0.0, 0.0, w, h).with_anchors(AnchorEdges::NONE);
        e.visible = false;
        e.accepts_focus = true;
        let view = ui.add(root, e, StudioView::new(fonts));
        let figure = ui.add(
            view,
            Element::at(0.0, 0.0, 10.0, 10.0)
                .with_anchors(AnchorEdges::NONE)
                .with_visible(false),
            ac_ui::widgets::Viewport::default(),
        );
        if let Some(e) = ui.element_mut(figure) {
            e.click_through = true;
        }
        let name_field =
            kit::mount_field(ui, view, (0.0, 0.0, 300.0, 46.0), field_font, &theme, false);
        ui.set_visible(name_field, false);
        Some(StudioUi {
            open: false,
            view,
            figure,
            name_field,
            figure_look: None,
            turns: Vec::new(),
            step: Step::Heritage,
            group: Group::Face,
            part: Part::Hair,
            roll_sent: None,
            finish_sent: None,
            refusal_seen: None,
            name_sent: None,
            strings,
            held_headgear: None,
            card: StudioCard::None,
            last_shade: None,
            random: SeededRandom::from_clock(),
        })
    }

    fn studio(&mut self) -> Option<&mut StudioUi> {
        self.modern
            .as_mut()
            .and_then(|m| m.pregame.as_mut())
            .and_then(|p| p.studio.as_mut())
    }

    /// Opens the Studio on its first step and rolls a random character.
    pub(in crate::ui) fn open_modern_studio(&mut self, scene: &Scene) {
        let current = scene.pregame().creation.as_ref().map(key);
        let Some(s) = self.studio() else {
            return;
        };
        s.open = true;
        s.step = Step::Heritage;
        s.group = Group::Face;
        s.part = Part::Hair;
        s.card = StudioCard::None;
        s.finish_sent = None;
        s.held_headgear = None;
        s.last_shade = None;
        s.roll_sent = current.or(Some((Generation(0), 0)));
        let (view, field) = (s.view, s.name_field);
        let ui = &mut self.host.root;
        kit::set_field_text(ui, field, "");
        if let Some(v) = ui.widget_mut::<StudioView>(view) {
            v.skill = None;
            v.skills_scroll = kit::Scroll::default();
            v.asks.clear();
        }
        create(ui, CreationCommand::RandomizeCharacter);
    }

    /// The picked part's colours and the colours along its shade.
    fn studio_colors(
        &self,
        part: Part,
        s: &CreationSnapshot,
        options: &ac_core::chargen::ChargenOptions,
    ) -> (Vec<Color>, Vec<Color>) {
        let Some(g) = ac_ui::pregame::chargen::appearance::gender_of(s, options) else {
            return (Vec::new(), Vec::new());
        };
        let c = self.viewports.catalog();
        let colors = swatch_colors(part, g, &s.appearance, c as &dyn SwatchSourcesDyn)
            .into_iter()
            .map(|c| c.map_or(Color::rgba(0.5, 0.5, 0.5, 1.0), rgb))
            .collect();
        let source = crate::ui::modern::barber::model::shade_source(part, &s.appearance, g);
        let stops = source
            .and_then(|(set, sample)| {
                let palettes = c.pal_set(set)?.palette_ids.clone();
                Some(
                    crate::ui::modern::barber::model::shade_indices(palettes.len(), 8)
                        .into_iter()
                        .filter_map(|i| (c as &dyn SwatchSourcesDyn).direct(palettes[i], sample))
                        .map(rgb)
                        .collect(),
                )
            })
            .unwrap_or_default();
        (colors, stops)
    }

    /// The part words of a heritage without nose and mouth.
    fn studio_captions(&self, heritage: u32) -> Option<[String; 3]> {
        let keys = ac_ui::pregame::chargen::appearance::three_part_caption_keys(heritage)?;
        let strings = self.content.strings();
        let words = keys.map(|k| {
            strings
                .resolve(ac_ui::panels::barber::CAPTION_TABLE, compute_hash(k), 0)
                .unwrap_or_default()
        });
        words.iter().all(|w| !w.is_empty()).then_some(words)
    }

    /// Each frame while open: what it shows follows the creation state,
    /// and what it asked for goes to the session.
    #[allow(clippy::too_many_lines)]
    pub(in crate::ui) fn tick_modern_studio(&mut self, scene: &Scene, choosing: bool) {
        let clock = self
            .modern
            .as_ref()
            .and_then(|m| m.pregame.as_ref())
            .map_or(0.0, |p| p.clock_now());
        let snapshot = scene.pregame().creation.clone();
        let options = scene.pregame().options.clone();
        let selection = scene.pregame().selection.clone();
        let Some(s) = self.studio() else {
            return;
        };
        let (view, figure, field) = (s.view, s.figure, s.name_field);
        let active = snapshot.as_ref().is_some_and(|c| c.is_active);
        if s.open && (!active || !choosing) {
            s.open = false;
        }
        let open = s.open;
        let ui = &mut self.host.root;
        let size = ui.size();
        if ui.element(view).is_some_and(|e| e.visible) != open {
            ui.set_visible(view, open);
            if open {
                ui.bring_to_front(view);
                ui.set_keyboard_focus(Some(view));
            }
        }
        let (Some(snap), Some(options), true) = (snapshot, options, open) else {
            if let Some(s) = self.studio() {
                s.figure_look = None;
            }
            let ui = &mut self.host.root;
            ui.set_visible(field, false);
            ui.set_visible(figure, false);
            return;
        };
        let current = key(&snap);
        let mut commands: Vec<CreationCommand> = Vec::new();
        let Some(s) = self.studio() else {
            return;
        };
        // The roll's answer: turn to the other gender, as retail does.
        if s.roll_sent.is_some_and(|sent| sent != current) {
            s.roll_sent = None;
            match snap.gender_key {
                1 => commands.push(CreationCommand::SelectGender(2)),
                2 => commands.push(CreationCommand::SelectGender(1)),
                _ => {}
            }
        }
        // A finish's answer here: what the session refused itself.
        if s.finish_sent.is_some_and(|sent| sent != current) {
            s.finish_sent = None;
            let refusal = snap.last_local_refusal;
            if refusal.no_name {
                s.card = StudioCard::NoName;
            } else if refusal.attribute_credits_unspent {
                s.card = StudioCard::Unspent(snap.remaining_attribute_credits.max(0));
            }
        }
        // The server's refusal.
        match &snap.last_rejection {
            Some(r) if s.refusal_seen.as_ref() != Some(r) => {
                s.refusal_seen = Some(r.clone());
                let words = self
                    .content
                    .strings()
                    .resolve(
                        ac_ui::pregame::SCREEN_STRING_TABLE,
                        compute_hash(ac_ui::pregame::chargen::rejection_key(r.raw_code)),
                        0,
                    )
                    .unwrap_or_else(|| {
                        "That name is not available. Please choose another.".to_owned()
                    });
                let near = if r.raw_code == ac_proto::character::chargen_code::NAME_IN_USE {
                    near_names(&r.attempted_name)
                } else {
                    Vec::new()
                };
                if let Some(s) = self.studio() {
                    s.card = StudioCard::Refused(words.trim().to_owned(), near);
                }
            }
            None => {
                if let Some(s) = self.studio() {
                    s.refusal_seen = None;
                }
            }
            _ => {}
        }
        let Some(s) = self.studio() else {
            return;
        };
        if snap.verification_pending {
            s.card = StudioCard::Creating(snap.name.trim().to_owned());
        } else if matches!(s.card, StudioCard::Creating(_)) {
            s.card = StudioCard::None;
        }
        let (step, group, part) = (s.step, s.group, s.part);
        let strings = std::rc::Rc::clone(&s.strings);
        let captions = self.studio_captions(snap.heritage_id);
        let world = selection
            .as_ref()
            .map(|v| v.world_name.trim().to_owned())
            .unwrap_or_default();
        let slots = selection.as_ref().map_or((1, 1), |v| {
            (
                v.entries.len() + 1,
                usize::try_from(v.slot_count.max(0)).unwrap_or(0),
            )
        });
        let words =
            |k: &str| strings.resolve(ac_ui::pregame::SCREEN_STRING_TABLE, compute_hash(k), 0);
        let asked = Asked {
            step,
            skill: self
                .host
                .root
                .widget::<StudioView>(view)
                .and_then(|v| v.skill),
            group,
            part,
            world: &world,
            slots,
            captions: captions.as_ref(),
        };
        let screen = studio_screen(&snap, &options, &words, &asked);
        let picked_part = screen.part.unwrap_or(Part::Hair);
        let (swatches, stops) = self.studio_colors(picked_part, &snap, &options);
        // What the view asked.
        let asks = self
            .host
            .root
            .widget_mut::<StudioView>(view)
            .map(|v| std::mem::take(&mut v.asks))
            .unwrap_or_default();
        let typed = kit::field_text(&self.host.root, field);
        let gender = ac_ui::pregame::chargen::appearance::gender_of(&snap, &options).cloned();
        let level = |id: u32| {
            snap.skills
                .get(id as usize)
                .copied()
                .unwrap_or(Class::Inactive)
        };
        let mut set_field: Option<String> = None;
        let mut turns = Vec::new();
        let Some(s) = self.studio() else {
            return;
        };
        let steps = screen.steps.clone();
        let at = steps.iter().position(|x| *x == screen.step).unwrap_or(0);
        let go = |s: &mut StudioUi, to: Step, commands: &mut Vec<CreationCommand>| {
            // Leaving the face puts the headgear back on.
            if (to != Step::Appearance || s.group != Group::Face)
                && let Some(held) = s.held_headgear.take()
            {
                commands.push(CreationCommand::SetAppearance(
                    AppearanceSlot::HeadgearStyle,
                    held,
                ));
            }
            if to == Step::Appearance
                && s.group == Group::Face
                && screen.has_clothes
                && s.held_headgear.is_none()
            {
                s.held_headgear = Some(snap.appearance.headgear_style);
                commands.push(CreationCommand::SetAppearance(
                    AppearanceSlot::HeadgearStyle,
                    UNSET,
                ));
            }
            s.step = to;
        };
        for ask in asks {
            match ask {
                StudioAsk::Go(to) => {
                    if steps.contains(&to) {
                        s.card = StudioCard::None;
                        go(s, to, &mut commands);
                    }
                }
                StudioAsk::Back => {
                    if at == 0 {
                        s.card = StudioCard::Exit;
                    } else {
                        go(s, steps[at - 1], &mut commands);
                    }
                }
                StudioAsk::Next => {
                    if let Some(&to) = steps.get(at + 1) {
                        go(s, to, &mut commands);
                    }
                }
                StudioAsk::Finish => {
                    let name: String = typed.trim().to_owned();
                    if name != snap.name {
                        s.name_sent = Some(name.clone());
                        commands.push(CreationCommand::SetName(name));
                    }
                    s.finish_sent = Some(current);
                    commands.push(CreationCommand::Finish {
                        confirm_unspent_credits: false,
                    });
                }
                StudioAsk::FinishAnyway => {
                    s.card = StudioCard::None;
                    s.finish_sent = Some(current);
                    commands.push(CreationCommand::Finish {
                        confirm_unspent_credits: true,
                    });
                }
                StudioAsk::Heritage(id) => commands.push(CreationCommand::SelectHeritage(id)),
                StudioAsk::Gender(k) => commands.push(CreationCommand::SelectGender(k)),
                StudioAsk::Template(i) => commands.push(CreationCommand::SelectTemplate(i)),
                StudioAsk::Attribute(a, v) => {
                    let now = screen
                        .attributes
                        .iter()
                        .find(|r| r.attribute == a)
                        .map(|r| r.value);
                    // A slider drag asks once per move: only the last
                    // value this frame for that attribute goes out.
                    commands
                        .retain(|c| !matches!(c, CreationCommand::SetAttribute(b, _) if *b == a));
                    if now != Some(v) {
                        commands.push(CreationCommand::SetAttribute(a, v.clamp(10, 100)));
                    }
                }
                StudioAsk::Lock(a, locked) => {
                    commands.push(CreationCommand::SetAttributeLock(a, locked));
                }
                StudioAsk::PickSkill(_) => {}
                StudioAsk::Raise(id) => commands.extend(advance(id, level(id))),
                StudioAsk::Lower(id) => commands.extend(retreat(id, level(id))),
                StudioAsk::Group(g) => {
                    s.group = g;
                    s.part = if g == Group::Face {
                        Part::Hair
                    } else {
                        Part::Headgear
                    };
                    go(s, Step::Appearance, &mut commands);
                }
                StudioAsk::Part(p) => {
                    s.part = p;
                    s.last_shade = None;
                    if let Some(g) = &gender {
                        commands.extend(pick_command(p, &snap.appearance, g));
                    }
                }
                StudioAsk::StepStyle(step) => {
                    if let Some(g) = &gender {
                        commands.extend(step_command(picked_part, step, &snap.appearance, g));
                    }
                }
                StudioAsk::Style(v) => {
                    if let Some(slot) = style_slot(picked_part) {
                        commands.push(CreationCommand::SetAppearance(slot, v));
                        if picked_part == Part::Headgear {
                            s.held_headgear = None;
                        }
                    }
                }
                StudioAsk::Color(i) => {
                    if let Some(slot) = color_slot(picked_part) {
                        commands.push(CreationCommand::SetAppearance(
                            slot,
                            u32::try_from(i).unwrap_or(0),
                        ));
                    }
                }
                StudioAsk::Shade(v) => {
                    if s.last_shade.is_none_or(|l| (l - v).abs() > 0.002)
                        && let Some(slot) = shade_slot(picked_part)
                    {
                        s.last_shade = Some(v);
                        commands.push(CreationCommand::SetShade(slot, f64::from(v)));
                    }
                }
                StudioAsk::Town(i) => commands.push(CreationCommand::SelectStartArea(i)),
                StudioAsk::Name(n) => {
                    set_field = Some(n.clone());
                    commands.push(CreationCommand::SetName(n));
                }
                StudioAsk::TryName(n) => {
                    s.card = StudioCard::None;
                    set_field = Some(n.clone());
                    commands.push(CreationCommand::AcknowledgeRejection);
                    commands.push(CreationCommand::SetName(n));
                    s.finish_sent = Some(current);
                    commands.push(CreationCommand::Finish {
                        confirm_unspent_credits: false,
                    });
                }
                StudioAsk::RandomStep => {
                    let text = &words as &dyn Fn(&str) -> Option<String>;
                    let ctx = PageContext {
                        snapshot: &snap,
                        options: &options,
                        text,
                    };
                    let random = &mut s.random;
                    let mut draw = |max: u32| random.next(max);
                    match screen.step {
                        Step::Heritage => commands.extend(HeritagePage::randomize(&ctx, &mut draw)),
                        Step::Profession => {
                            commands.extend(ProfessionPage::randomize(&ctx, &mut draw));
                        }
                        Step::Appearance => commands.push(match s.group {
                            Group::Face => CreationCommand::RandomizeAppearance,
                            Group::Clothes => CreationCommand::RandomizeClothing,
                        }),
                        Step::Town => commands.extend(TownPage::randomize(&ctx, &mut draw)),
                        Step::Name => {
                            let n = u32::try_from(screen.suggestions.len()).unwrap_or(0);
                            if n > 0 {
                                let pick = screen.suggestions[draw(n) as usize].clone();
                                set_field = Some(pick.clone());
                                commands.push(CreationCommand::SetName(pick));
                            }
                        }
                        Step::Skills => {}
                    }
                }
                StudioAsk::AskRandomAll => s.card = StudioCard::RandomAll,
                StudioAsk::RandomAll => {
                    s.card = StudioCard::None;
                    s.held_headgear = None;
                    commands.push(CreationCommand::RandomizeCharacter);
                    // The name stays.
                    if !snap.name.trim().is_empty() {
                        commands.push(CreationCommand::SetName(snap.name.clone()));
                    }
                }
                StudioAsk::AskExit => s.card = StudioCard::Exit,
                StudioAsk::Exit => {
                    s.card = StudioCard::None;
                    s.open = false;
                }
                StudioAsk::CloseCard => {
                    if matches!(s.card, StudioCard::Refused(..)) {
                        commands.push(CreationCommand::AcknowledgeRejection);
                        go(s, Step::Name, &mut commands);
                    }
                    s.card = StudioCard::None;
                }
                StudioAsk::Preview(r) => turns.push(r),
            }
        }
        s.turns.extend(turns);
        // The name as typed: the characters a name may have, at most 32.
        let clean: String = typed
            .chars()
            .filter(|c| name_filter(*c))
            .take(NAME_LIMIT)
            .collect();
        if clean != typed && set_field.is_none() {
            set_field = Some(clean.clone());
        }
        // The name as typed goes to the session once per change (the
        // session's copy comes back a frame or more later).
        let name = clean.trim();
        if set_field.is_none()
            && name != snap.name.trim()
            && screen.step == Step::Name
            && s.name_sent.as_deref() != Some(name)
        {
            s.name_sent = Some(name.to_owned());
            commands.push(CreationCommand::SetName(name.to_owned()));
        }
        let show_card = s.card.clone();
        let still_open = s.open;
        let face = screen.step == Step::Appearance && screen.group == Group::Face;
        s.figure_look = None;
        let backdrop = self
            .modern
            .as_ref()
            .and_then(|m| m.pregame.as_ref())
            .map(|p| p.backdrop.clone())
            .unwrap_or_default();
        let ui = &mut self.host.root;
        for c in commands {
            create(ui, c);
        }
        if let Some(n) = set_field {
            kit::set_field_text(ui, field, &n);
        }
        let (name_rect, figure_rect) = match ui.widget_mut::<StudioView>(view) {
            Some(v) => {
                v.size = size;
                v.clock = clock;
                v.screen = screen.clone();
                v.backdrop = backdrop;
                v.card = show_card.clone();
                v.swatches = swatches;
                v.shade_stops = stops;
                (v.name_rect(), v.figure_rect())
            }
            None => return,
        };
        if let Some(e) = ui.element_mut(view) {
            e.width = size.0;
            e.height = size.1;
        }
        let field_shows = still_open && screen.step == Step::Name && show_card == StudioCard::None;
        if ui.element(field).is_some_and(|e| e.visible) != field_shows {
            ui.set_visible(field, field_shows);
            if field_shows {
                if kit::field_text(ui, field).is_empty() && !snap.name.is_empty() {
                    kit::set_field_text(ui, field, &snap.name);
                }
                ui.set_keyboard_focus(Some(field));
            }
        }
        if let Some(e) = ui.element_mut(field) {
            e.left = name_rect.0 + 8.0;
            e.top = name_rect.1 + (name_rect.3 - 30.0) * 0.5;
            e.width = name_rect.2 - 16.0;
            e.height = 30.0;
        }
        if let Some(e) = ui.element_mut(figure) {
            e.left = figure_rect.0;
            e.top = figure_rect.1;
            e.width = figure_rect.2;
            e.height = figure_rect.3;
        }
        let figure_shows = still_open && show_card == StudioCard::None;
        if ui.element(figure).is_some_and(|e| e.visible) != figure_shows {
            ui.set_visible(figure, figure_shows);
        }
        if !still_open {
            ui.set_visible(view, false);
            ui.set_visible(field, false);
            ui.set_visible(figure, false);
            return;
        }
        // The figure: the look composed from the choices.
        let preview = (snap.heritage_id != 0 && snap.gender_key != 0).then_some(
            ac_ui::pregame::chargen::ChargenPreview {
                viewport: figure,
                summary: !face,
                heritage: snap.heritage_id,
                gender: snap.gender_key,
                appearance: snap.appearance,
            },
        );
        let look = preview.and_then(|p| self.viewports.look_for(&options, &p));
        if let Some(s) = self.studio() {
            s.figure_look = look.map(|l| (l, face));
        }
    }

    /// A key while the Studio shows: Escape closes the card or asks to
    /// leave; Enter answers the card, finishes on the last step, or goes
    /// on.
    pub(in crate::ui) fn modern_studio_key(&mut self, key: ac_ui::event::Key) -> bool {
        use ac_ui::event::Key;
        let Some(s) = self.studio() else {
            return false;
        };
        let (view, step, card) = (s.view, s.step, s.card.clone());
        let ask = match key {
            Key::Escape => Some(match card {
                StudioCard::None => StudioAsk::AskExit,
                StudioCard::Creating(_) => return true,
                _ => StudioAsk::CloseCard,
            }),
            Key::Enter => Some(match card {
                StudioCard::None if step == Step::Name => StudioAsk::Finish,
                StudioCard::None => StudioAsk::Next,
                StudioCard::Exit => StudioAsk::Exit,
                StudioCard::RandomAll => StudioAsk::RandomAll,
                StudioCard::Unspent(_) => StudioAsk::Go(Step::Profession),
                StudioCard::NoName | StudioCard::Refused(..) => StudioAsk::CloseCard,
                StudioCard::Creating(_) => return true,
            }),
            _ => None,
        };
        let Some(ask) = ask else {
            return false;
        };
        if let Some(v) = self.host.root.widget_mut::<StudioView>(view) {
            v.asks.push(ask);
        }
        true
    }
}
