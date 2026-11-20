//! Pictures of the modern interface without a window: the window's UI
//! built over the retail dats in Modern, fed a made-up player in the
//! world, driven by the pointer and keys, and its draw list rasterised on
//! the CPU into PNGs (the web client's test rasteriser). For looking at a
//! modern screen when no window can be shown. Ignored by default; run with
//! `OPENAC_SNAPSHOT_DIR=<dir> cargo test -p ac-render --test modern_snapshot
//! -- --ignored`.

#![allow(dead_code)]

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use ac_core::items::{ClientObject, ItemType, ShortcutEntry, ShortcutStore};
use ac_core::spells::Spellbook;
use ac_dat::DatCollection;
use ac_proto::{ModelData, Position, Quaternion, Vector3};
use ac_render::scene::Scene;
use ac_render::ui::batch::UiTexture;
use ac_render::ui::input::{self as ui_input, Routed};
use ac_render::ui::{UiBatch, UiTextureSource, WindowUi};
use ac_runtime::session::PresentationEvent;
use ac_runtime::session::interface::{InterfaceEvent, InterfaceState, ItemsUpdate, PlayerFacts};
use ac_runtime::world::{
    ObjectKey, ObjectView, PlayerView, RadarFacts, VitalReading, WorldSnapshot,
};
use ac_ui::draw::TextureRef;
use ac_ui::event::{Key, Modifiers};

#[path = "../../ac-ui-web/tests/harness/raster.rs"]
mod raster;

/// A texture as the rasteriser holds it.
#[derive(Debug, Clone)]
pub struct Held {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
    pub levels: u32,
}

const PLAYER: u32 = 0x5000_0001;
const W: u32 = 1440;
const H: u32 = 900;

fn dats() -> Arc<DatCollection> {
    let dir = std::env::var_os("OPENAC_DAT_DIR")
        .map(PathBuf::from)
        .or_else(ac_dat::default_dat_dir)
        .expect("no dat directory: set OPENAC_DAT_DIR");
    Arc::new(DatCollection::open(dir).expect("open the retail dats"))
}

fn out_dir() -> PathBuf {
    std::env::var_os("OPENAC_SNAPSHOT_DIR").map_or_else(
        || std::env::temp_dir().join("openac-snapshots"),
        PathBuf::from,
    )
}

fn player_object() -> ObjectView {
    ObjectView {
        key: ObjectKey {
            local_id: 1,
            incarnation: 1,
        },
        guid: PLAYER,
        name: Some("Aerin Vale".to_owned()),
        setup_id: None,
        position: Some(Position {
            cell_id: 0xA9B4_0031,
            origin: Vector3 {
                x: 100.0,
                y: 100.0,
                z: 0.0,
            },
            orientation: Quaternion::IDENTITY,
        }),
        parent: None,
        placement: None,
        rest_placement: 0,
        scale: None,
        translucency: None,
        state: 0,
        velocity: None,
        model: ModelData::default(),
        motion_table_id: None,
        sound_table_id: None,
        radar: RadarFacts {
            item_type: 0x10,
            public_weenie_bitfield: 0x8,
            radar_behavior: Some(4),
            radar_blip_color: 0,
            monarch_id: 0,
        },
        motion: None,
        effects: ac_runtime::world::EffectFacts::default(),
        step_blend: None,
    }
}

/// The player's things: (guid, name, item type, icon, stack).
const ITEMS: &[(u32, &str, ItemType, u32, i32)] = &[
    (
        0x8000_0101,
        "Health Potion",
        ItemType::FOOD,
        0x0600_1A8B,
        12,
    ),
    (0x8000_0102, "Mana Potion", ItemType::FOOD, 0x0600_1A8D, 7),
    (
        0x8000_0103,
        "Stamina Elixir",
        ItemType::FOOD,
        0x0600_1A8C,
        1,
    ),
    (0x8000_0104, "Healing Kit", ItemType::MISC, 0x0600_1E3B, 1),
    (0x8000_0105, "Sword", ItemType::MELEE_WEAPON, 0x0600_11CB, 1),
];

/// A scene with the player in the world, its things, its spells (the
/// first `spells` of the dats' table, the first eight its favourites) and
/// `mode`.
fn scene(dats: &DatCollection, mode: ac_core::combat::CombatMode) -> Scene {
    let mut scene = Scene::default();
    scene.apply(PresentationEvent::Snapshot(WorldSnapshot {
        player_guid: Some(PLAYER),
        objects: vec![player_object()],
    }));
    let vital = |current, max| VitalReading {
        current: Some(current),
        max: Some(max),
        percent: Some(current as f32 / max as f32),
    };
    scene.apply(PresentationEvent::Player(PlayerView {
        health: vital(310, 400),
        stamina: vital(380, 400),
        mana: vital(290, 420),
        ..PlayerView::default()
    }));
    let mut player = ClientObject::new(PLAYER);
    player.name = "Aerin Vale".to_owned();
    player.item_type = ItemType::CREATURE;
    let mut objects = vec![player];
    let mut shortcuts = ShortcutStore::new();
    for (i, &(guid, name, item_type, icon, stack)) in ITEMS.iter().enumerate() {
        let mut o = ClientObject::new(guid);
        o.name = name.to_owned();
        o.item_type = item_type;
        o.icon_id = icon;
        o.stack_size = stack;
        o.container_id = PLAYER;
        objects.push(o);
        shortcuts.set(ShortcutEntry {
            index: i as i32,
            object_id: guid,
            spell_id: 0,
        });
    }
    let guids: Vec<u32> = ITEMS.iter().map(|i| i.0).collect();
    scene.apply(PresentationEvent::Interface(InterfaceEvent::Items(
        ItemsUpdate {
            reset: true,
            objects,
            removed: Vec::new(),
            contents: vec![(PLAYER, Some(guids))],
        },
    )));
    let tables = ac_runtime::gameplay::GameTables::load(dats).expect("the game tables");
    let mut book = Spellbook::new(Some(Arc::clone(tables.spells())));
    let ids: Vec<u32> = tables.spells().spell_ids().take(40).collect();
    for (i, &id) in ids.iter().enumerate() {
        book.on_spell_learned(id, 1.0);
        if i < 8 {
            book.set_favorite(0, i, id);
        } else if i < 14 {
            book.set_favorite(2, i - 8, id);
        }
    }
    scene.apply(PresentationEvent::Interface(InterfaceEvent::Player(
        Box::new(PlayerFacts {
            revision: 2,
            numbers: ac_core::player::LocalPlayerState::new(Some(book)),
            session_seconds: 0.0,
        }),
    )));
    scene.apply(PresentationEvent::Interface(InterfaceEvent::State(
        Box::new(InterfaceState {
            player: PLAYER,
            can_make_request: true,
            combat_mode: mode,
            shortcuts,
            ..InterfaceState::default()
        }),
    )));
    scene
}

struct Shot {
    ui: WindowUi,
    scene: Scene,
    now: Instant,
    textures: HashMap<(u32, u32), Held>,
    /// What the window sent the session.
    sent: Vec<ac_runtime::session::SessionCommand>,
}

impl Shot {
    fn new(mode: ac_core::combat::CombatMode) -> Self {
        Self::with_settings(mode, None)
    }

    /// The same, saving to the settings file at `settings`.
    fn with_settings(mode: ac_core::combat::CombatMode, settings: Option<PathBuf>) -> Self {
        use ac_render::ui::SettingsStores;
        let dats = dats();
        let mut ui = WindowUi::build(Arc::clone(&dats), (W, H)).expect("the UI builds");
        ui.set_layout_store(settings.map(ac_host::settings::SettingsStore::new));
        ui.set_interface(ac_host::Interface::Modern);
        let scene = scene(&dats, mode);
        let mut shot = Self {
            ui,
            scene,
            now: Instant::now(),
            textures: HashMap::new(),
            sent: Vec::new(),
        };
        // Long enough for the fonts, the layout and the first bars.
        for _ in 0..10 {
            shot.step();
        }
        shot
    }

    fn step(&mut self) {
        self.ui.feed(&mut self.scene);
        self.now += Duration::from_millis(50);
        let _ = self.ui.frame(self.now, (W, H));
        let sent = self.ui.drain();
        self.sent.extend(sent);
    }

    fn set_mode(&mut self, mode: ac_core::combat::CombatMode) {
        let shortcuts = {
            let mut s = ShortcutStore::new();
            for (i, item) in ITEMS.iter().enumerate() {
                s.set(ShortcutEntry {
                    index: i as i32,
                    object_id: item.0,
                    spell_id: 0,
                });
            }
            s
        };
        self.scene
            .apply(PresentationEvent::Interface(InterfaceEvent::State(
                Box::new(InterfaceState {
                    player: PLAYER,
                    can_make_request: true,
                    combat_mode: mode,
                    shortcuts,
                    ..InterfaceState::default()
                }),
            )));
        self.step();
        self.step();
    }

    fn moved(&mut self, x: i32, y: i32) {
        let r: Routed<u32> = ui_input::moved(self.ui.host_mut(), x, y, false);
        let _ = self.ui.handle(r.outputs);
        self.step();
    }

    fn click(&mut self, button: u16, x: i32, y: i32, modifiers: Modifiers) {
        self.moved(x, y);
        let flags = ui_input::mouse_flags(&[button], modifiers);
        let r: Routed<u32> = ui_input::button(self.ui.host_mut(), button, true, x, y, flags, false);
        let _ = self.ui.handle(r.outputs);
        let flags = ui_input::mouse_flags(&[], modifiers);
        let r: Routed<u32> =
            ui_input::button(self.ui.host_mut(), button, false, x, y, flags, false);
        let _ = self.ui.handle(r.outputs);
        self.step();
    }

    /// Presses the left button at `from`, moves to `to` in steps and lets
    /// go there.
    fn drag(&mut self, from: (i32, i32), to: (i32, i32)) {
        self.moved(from.0, from.1);
        let held = ui_input::mouse_flags(&[0], Modifiers::NONE);
        let r: Routed<u32> =
            ui_input::button(self.ui.host_mut(), 0, true, from.0, from.1, held, false);
        let _ = self.ui.handle(r.outputs);
        self.step();
        for i in 1..=8 {
            let x = from.0 + (to.0 - from.0) * i / 8;
            let y = from.1 + (to.1 - from.1) * i / 8;
            self.moved(x, y);
        }
        let none = ui_input::mouse_flags(&[], Modifiers::NONE);
        let r: Routed<u32> =
            ui_input::button(self.ui.host_mut(), 0, false, to.0, to.1, none, false);
        let _ = self.ui.handle(r.outputs);
        self.step();
        self.step();
    }

    fn key(&mut self, key: Key, modifiers: Modifiers, pressed: bool) {
        let r: Routed<u32> = self.ui.key(0, key, pressed, false, modifiers, None);
        let _ = self.ui.handle(r.outputs);
        self.step();
    }

    fn save(&mut self, name: &str) {
        self.now += Duration::from_millis(50);
        let list = self.ui.frame(self.now, (W, H));
        let batch = UiBatch::build(&list);
        let mut surfaces = self.ui.surfaces();
        for run in &batch.runs {
            let UiTexture::Named(texture) = run.key.texture else {
                continue;
            };
            let key = match texture {
                TextureRef::Surface(id) => (1, id),
                TextureRef::Runtime(h) => (2, h),
            };
            if self.textures.contains_key(&key) {
                continue;
            }
            let Some(pixels) = surfaces.decode(texture) else {
                continue;
            };
            let (width, height) = pixels.size();
            let rgba = match pixels {
                ac_render::ui::gpu::UiPixels::Rgba(t) => t.rgba,
                ac_render::ui::gpu::UiPixels::Coverage { alpha, .. } => {
                    alpha.iter().flat_map(|&a| [255, 255, 255, a]).collect()
                }
                ac_render::ui::gpu::UiPixels::Plugin(levels) => levels
                    .into_iter()
                    .next()
                    .map(|t| t.rgba)
                    .unwrap_or_default(),
            };
            self.textures.insert(
                key,
                Held {
                    width,
                    height,
                    rgba,
                    levels: 1,
                },
            );
        }
        let corners: Vec<raster::Corner> = batch
            .vertices
            .iter()
            .map(|v| raster::Corner {
                position: v.position,
                uv: v.uv,
                color: v.color,
            })
            .collect();
        let runs: Vec<[u32; 5]> = batch
            .runs
            .iter()
            .map(|r| {
                let (kind, id) = match r.key.texture {
                    UiTexture::White => (0, 0),
                    UiTexture::Named(TextureRef::Surface(id)) => (1, id),
                    UiTexture::Named(TextureRef::Runtime(h)) => (2, h),
                };
                let filter = u32::from(matches!(r.key.filter, ac_ui::draw::Filter::Nearest));
                [kind, id, filter, r.first, r.count]
            })
            .collect();
        let rgba = raster::render(W, H, &corners, &runs, &self.textures);
        let dir = out_dir();
        std::fs::create_dir_all(&dir).expect("the snapshot folder");
        let path = dir.join(format!("{name}.png"));
        std::fs::write(&path, ac_bits::png::encode_rgba(W, H, &rgba)).expect("the PNG");
        println!("saved {}", path.display());
        for notice in self.ui.take_notices() {
            println!("notice: {notice}");
        }
    }
}

/// The Main bar's first slot's centre on a 1440 x 900 screen.
const MAIN_SLOT: (i32, i32) = (484, 820);

#[test]
#[ignore = "needs the retail dat files; writes pictures"]
fn the_modern_bars() {
    use ac_core::combat::CombatMode;
    let mut s = Shot::new(CombatMode::NON_COMBAT);
    s.save("bars-peace");
    s.set_mode(CombatMode::MAGIC);
    s.save("bars-magic");
    s.set_mode(CombatMode::NON_COMBAT);
    // Hover a slot: its label.
    s.moved(MAIN_SLOT.0, MAIN_SLOT.1);
    for _ in 0..14 {
        s.step();
    }
    s.save("bars-hover");
    // Quick move's menu on the Main bar: Ctrl + right-click.
    s.ui.host_mut().set_modifiers(Modifiers::CTRL);
    s.click(1, MAIN_SLOT.0, MAIN_SLOT.1, Modifiers::CTRL);
    s.ui.host_mut().set_modifiers(Modifiers::NONE);
    s.save("bars-quick-menu");
    // Its last row opens Edit layout (the menu is kept on the screen, so
    // it sits above the click: its foot at 900 - 6).
    s.click(0, MAIN_SLOT.0 + 40, 900 - 6 - 20, Modifiers::NONE);
    s.save("bars-edit");
    // A slot clicked in Edit layout: its keys.
    s.click(0, MAIN_SLOT.0, MAIN_SLOT.1, Modifiers::NONE);
    s.save("bars-keys");
    // The Magic key box, then Shift+Q.
    s.click(0, 570, 674, Modifiers::NONE);
    s.save("bars-keys-listening");
    s.key(Key::Q, Modifiers::SHIFT, true);
    s.key(Key::Q, Modifiers::SHIFT, false);
    s.save("bars-keys-set");
    s.key(Key::Escape, Modifiers::NONE, true);
    s.key(Key::Escape, Modifiers::NONE, false);
    s.key(Key::Escape, Modifiers::NONE, true);
    s.key(Key::Escape, Modifiers::NONE, false);
    s.save("bars-after-edit");
    // Drag the potion from slot 1 to slot 8 (43 units a slot); then the
    // elixir off the bar into the world.
    s.drag(MAIN_SLOT, (MAIN_SLOT.0 + 43 * 7, MAIN_SLOT.1));
    s.drag((MAIN_SLOT.0 + 43 * 2, MAIN_SLOT.1), (700, 400));
    s.save("bars-dragged");
    s.set_mode(CombatMode::MELEE);
    s.save("bars-melee");
    s.set_mode(CombatMode::NON_COMBAT);
    // The spellbook open; its first spell carried to the Main bar's tenth
    // slot (empty), a picture on the way.
    use ac_render::ui::window::{Hotkey, SPELLBOOK_WINDOW};
    use ac_ui::panels::magic::{SpellbookPage, SpellbookWindowController};
    assert!(s.ui.run_hotkey(Hotkey::Spellbook(SpellbookPage::Spells)));
    s.step();
    s.step();
    let list =
        s.ui.host_mut()
            .with_controller::<SpellbookWindowController, _>(SPELLBOOK_WINDOW, |c, _| c.list())
            .expect("the spellbook");
    let row = {
        let root = &s.ui.host().root;
        let cell = root
            .widget::<ac_ui::widgets::ItemList>(list)
            .and_then(|l| l.item(0))
            .expect("a spell row");
        let (x, y) = root.screen_position(cell);
        let e = root.element(cell).unwrap();
        ((x + e.width * 0.3) as i32, (y + e.height * 0.5) as i32)
    };
    println!("spell row at {row:?}");
    s.save("spell-book-open");
    let to = (MAIN_SLOT.0 + 43 * 9, MAIN_SLOT.1);
    s.moved(row.0, row.1);
    let held = ui_input::mouse_flags(&[0], Modifiers::NONE);
    let r: Routed<u32> = ui_input::button(s.ui.host_mut(), 0, true, row.0, row.1, held, false);
    let _ = s.ui.handle(r.outputs);
    s.step();
    for i in 1..=8 {
        s.moved(
            row.0 + (to.0 - row.0) * i / 8,
            row.1 + (to.1 - row.1) * i / 8,
        );
    }
    s.save("spell-dragging");
    let none = ui_input::mouse_flags(&[], Modifiers::NONE);
    let r: Routed<u32> = ui_input::button(s.ui.host_mut(), 0, false, to.0, to.1, none, false);
    let _ = s.ui.handle(r.outputs);
    s.step();
    s.step();
    s.save("spell-dropped");
}

#[test]
#[ignore = "needs the retail dat files; writes pictures"]
fn the_modern_bars_move_and_turn() {
    use ac_core::combat::CombatMode;
    let settings = out_dir().join("settings.json");
    let _ = std::fs::remove_file(&settings);
    let mut s = Shot::with_settings(CombatMode::NON_COMBAT, Some(settings.clone()));
    // Edit layout from the empty slot's menu (right-click on slot 12).
    let last = (MAIN_SLOT.0 + 43 * 11, MAIN_SLOT.1);
    s.click(1, last.0, last.1, Modifiers::NONE);
    s.save("move-empty-menu");
    s.click(0, last.0 + 40, 900 - 6 - 20, Modifiers::NONE);
    // The Main bar's tab is under it: drag it by its grip up and left.
    s.drag((MAIN_SLOT.0 - 10, 862), (300, 500));
    s.save("move-dragged");
    // Turn it with the tab's turn button (the tab is now above the bar,
    // its buttons at its right end).
    s.save("move-before-turn");
    // Its tab sits over it now (286, 405), 347 wide: the turn button is
    // the fourth from its right end.
    s.click(0, 286 + 347 - 4 - 22 * 4 + 11, 418, Modifiers::NONE);
    s.save("move-turned");
    s.click(0, 286 + 347 - 4 - 22 * 4 + 11, 418, Modifiers::NONE);
    // A new bar; the layout menu.
    s.click(0, 913, 115, Modifiers::NONE);
    s.save("move-new-bar");
    s.click(0, 778, 115, Modifiers::NONE);
    s.save("move-layout-menu");
    s.key(Key::Escape, Modifiers::NONE, true);
    s.key(Key::Escape, Modifiers::NONE, false);
    // Saved (Edit layout was left with Escape). A new window for the
    // same character finds the bar where it was left.
    for _ in 0..12 {
        s.step();
    }
    let saved = std::fs::read_to_string(&settings).expect("the settings file");
    assert!(saved.contains("modernBars"), "{saved}");
    assert!(saved.contains("/Aerin Vale"), "{saved}");
    let main = |ui: &WindowUi| {
        let root = &ui.host().root;
        root.children(root.root())
            .into_iter()
            .filter_map(|c| root.element(c))
            .find(|e| e.name.as_deref() == Some("Main bar"))
            .map(|e| (e.left, e.top))
    };
    let before = main(&s.ui);
    assert_ne!(before, Some((460.0, 796.0)));
    drop(s);
    let mut again = Shot::with_settings(CombatMode::NON_COMBAT, Some(settings));
    assert_eq!(main(&again.ui), before);
    again.save("move-reloaded");
}

#[test]
#[ignore = "needs the retail dat files; writes pictures"]
fn the_m2_gaps() {
    use ac_core::combat::CombatMode;
    let settings = out_dir().join("gaps-settings.json");
    let _ = std::fs::remove_file(&settings);
    let mut s = Shot::with_settings(CombatMode::NON_COMBAT, Some(settings.clone()));
    // The potion stack selected: the target line shows its split.
    let potion = ITEMS[0].0;
    s.scene.apply(PresentationEvent::Player(PlayerView {
        selected: Some(potion),
        ..PlayerView::default()
    }));
    s.step();
    s.step();
    s.save("gaps-split");
    assert_eq!(s.ui.drag_amount(potion), 12);
    // Drag the knob back along the track (the line is 420 wide, top
    // centre at 46; its track runs from 40 to 256 at 84).
    let left = (1440 - 420) / 2;
    s.drag((left + 256, 84), (left + 120, 84));
    s.save("gaps-split-dragged");
    let amount = s.ui.drag_amount(potion);
    assert!((4..=6).contains(&amount), "{amount}");
    // Quick move's menu on the radar.
    s.ui.host_mut().set_modifiers(Modifiers::CTRL);
    s.click(1, 1440 - 80, 80, Modifiers::CTRL);
    s.ui.host_mut().set_modifiers(Modifiers::NONE);
    s.save("gaps-radar-menu");
    // Hide it (the menu's second entry).
    s.click(0, 1210, 163, Modifiers::NONE);
    s.save("gaps-radar-hidden");
    // Lock the vitals: their menu's first entry.
    s.ui.host_mut().set_modifiers(Modifiers::CTRL);
    s.click(1, 720, 22, Modifiers::CTRL);
    s.ui.host_mut().set_modifiers(Modifiers::NONE);
    s.save("gaps-vitals-menu");
    s.click(0, 760, 72, Modifiers::NONE);
    // Locked: Ctrl + right-click on it opens nothing now.
    s.ui.host_mut().set_modifiers(Modifiers::CTRL);
    s.click(1, 720, 22, Modifiers::CTRL);
    s.ui.host_mut().set_modifiers(Modifiers::NONE);
    s.save("gaps-vitals-locked");
    // Edit layout from the Main bar's menu; its toolbar.
    s.ui.host_mut().set_modifiers(Modifiers::CTRL);
    s.click(1, MAIN_SLOT.0, MAIN_SLOT.1, Modifiers::CTRL);
    s.ui.host_mut().set_modifiers(Modifiers::NONE);
    s.click(0, MAIN_SLOT.0 + 40, 900 - 6 - 20, Modifiers::NONE);
    s.save("gaps-edit");
    s.click(0, 960, 115, Modifiers::NONE);
    s.save("gaps-pieces-menu");
    // Show Radar; out of Edit layout.
    s.click(0, 770, 188, Modifiers::NONE);
    s.key(Key::Escape, Modifiers::NONE, true);
    s.key(Key::Escape, Modifiers::NONE, false);
    s.save("gaps-radar-back");
    // The Recall list from the button on the radar's rim.
    s.click(0, 1239, 227, Modifiers::NONE);
    s.save("gaps-recall-list");
    // Lifestone carried to the Main bar's seventh slot (empty).
    s.drag((1030, 140), (MAIN_SLOT.0 + 43 * 6, MAIN_SLOT.1));
    s.moved(MAIN_SLOT.0 + 43 * 6, MAIN_SLOT.1);
    for _ in 0..14 {
        s.step();
    }
    s.save("gaps-recall-on-bar");
    // The bars are saved half a second (of the window's own clock) after
    // a change.
    std::thread::sleep(Duration::from_millis(600));
    s.step();
    let saved = std::fs::read_to_string(&settings).expect("the settings file");
    assert!(saved.contains("modernPieces"), "{saved}");
    assert!(saved.contains("\"@ls\""), "{saved}");
}

/// A chat line of `class` from `sender` saying `words` (`text` as the game
/// words the whole line).
fn chat_line(
    n: u64,
    kind: ac_core::chat::ChatKind,
    class: ac_core::chat::LogTextType,
    sender: &str,
    words: &str,
    text: &str,
) -> ac_runtime::communication::ChatLine {
    ac_runtime::communication::ChatLine {
        sequence: n,
        kind,
        log_text_type: class as u32,
        combat_kind: None,
        received_unix_ms: 0,
        text: text.to_owned(),
        channel_name: String::new(),
        sender: sender.to_owned(),
        sender_guid: 0x5000_0100 + n as u32,
        words: words.to_owned(),
        display_text: text.to_owned(),
        channel_id: 0,
        spans: None,
    }
}

#[test]
#[ignore = "needs the retail dat files; writes pictures"]
fn the_modern_chat() {
    use ac_core::chat::{ChatKind as K, LogTextType as T};
    use ac_core::combat::CombatMode;
    let mut s = Shot::new(CombatMode::NON_COMBAT);
    let lines = vec![
        chat_line(1, K::System, T::System, "", "", "Rook is now online."),
        chat_line(
            2,
            K::Tell,
            T::Tell,
            "Rook",
            "portal gem ready when you are",
            "Rook tells you, \"portal gem ready when you are\"",
        ),
        chat_line(
            3,
            K::Channel,
            T::Fellowship,
            "Mira",
            "pulling the next room, stay on the ramp",
            "",
        ),
        chat_line(
            4,
            K::Channel,
            T::TurbineGeneral,
            "Osk",
            "anyone selling Major Coordination?",
            "",
        ),
        chat_line(
            5,
            K::System,
            T::Magic,
            "",
            "",
            "You cast Incantation of Flame Bolt on Drudge Ravener.",
        ),
        chat_line(
            6,
            K::Channel,
            T::Allegiance,
            "Thessaly",
            "Aerin, grats on 87! That was a long night of drudges and we all saw it.",
            "",
        ),
        chat_line(
            7,
            K::LocalSpeech,
            T::Speech,
            "Bram",
            "anyone heading to the Marketplace?",
            "",
        ),
    ];
    s.scene.apply(PresentationEvent::Chat(lines));
    s.step();
    s.step();
    s.save("chat-idle");
    // Enter: the panel comes up with the entry focused.
    s.ui.chat_entry_key(ac_ui::input::InputAction::EnterChatMode, None);
    s.step();
    for c in "hello there".chars() {
        let r: Routed<u32> = s.ui.key(
            0,
            Key::A,
            true,
            false,
            Modifiers::NONE,
            Some(c.to_string().as_str()),
        );
        let _ = s.ui.handle(r.outputs);
    }
    s.step();
    s.save("chat-active");
    // An NPC with a long name selected: the menu offers a tell to it,
    // its name cut short before the note.
    let npc = 0x8000_0A01;
    let mut guard = player_object();
    guard.guid = npc;
    guard.key.local_id = 2;
    guard.name = Some("Royal Guard Adrana of the Northern Watch".to_owned());
    s.scene.apply(PresentationEvent::Snapshot(WorldSnapshot {
        player_guid: Some(PLAYER),
        objects: vec![player_object(), guard],
    }));
    s.scene.apply(PresentationEvent::Player(PlayerView {
        selected: Some(npc),
        ..PlayerView::default()
    }));
    s.step();
    // The channel button (bottom left: the panel's entry at 900 - 14 - 32).
    s.click(0, 16 + 30, 900 - 14 - 16, Modifiers::NONE);
    s.save("chat-channels");
    s.key(Key::Escape, Modifiers::NONE, true);
    s.key(Key::Escape, Modifiers::NONE, false);
    // The Fellowship tab: its unread count clears.
    s.save("chat-before-tab");
    // Enter sends the typed line through the session's chat entry.
    s.ui.chat_entry_key(ac_ui::input::InputAction::EnterChatMode, None);
    s.step();
    let r: Routed<u32> =
        s.ui.key(0, Key::Enter, true, false, Modifiers::NONE, Some("\r"));
    let mut sent = format!("{:?}", r.outputs);
    let _ = s.ui.handle(r.outputs);
    s.now += Duration::from_millis(50);
    let _ = s.ui.frame(s.now, (W, H));
    sent.push_str(&format!("{:?}", s.ui.drain()));
    assert!(sent.contains("Submit(Some(\"hello there\"))"), "{sent}");
    // Typing @r suggests @reply first.
    s.ui.chat_entry_key(ac_ui::input::InputAction::EnterChatMode, None);
    s.step();
    for c in ["@", "r"] {
        let r: Routed<u32> = s.ui.key(0, Key::A, true, false, Modifiers::NONE, Some(c));
        let _ = s.ui.handle(r.outputs);
    }
    s.step();
    s.save("chat-commands");
    for _ in 0..2 {
        let r: Routed<u32> =
            s.ui.key(0, Key::Backspace, true, false, Modifiers::NONE, None);
        let _ = s.ui.handle(r.outputs);
    }
    s.step();
    // Osk's name: the menu.
    s.click(0, 98, 758, Modifiers::NONE);
    s.save("chat-name");
    s.click(0, 700, 300, Modifiers::NONE);
    // Search, for "drudge".
    s.ui.chat_entry_key(ac_ui::input::InputAction::EnterChatMode, None);
    s.step();
    s.click(0, 16 + 440 - 12, 900 - 14 - 300 + 12, Modifiers::NONE);
    for c in "drudge".chars() {
        let r: Routed<u32> = s.ui.key(
            0,
            Key::A,
            true,
            false,
            Modifiers::NONE,
            Some(c.to_string().as_str()),
        );
        let _ = s.ui.handle(r.outputs);
    }
    s.step();
    s.save("chat-search");
    // Close search; a new tab from the + after the tabs.
    s.click(0, 16 + 440 - 4 - 22 + 11 - 22, 598, Modifiers::NONE);
    s.click(0, 16 + 440 - 4 - 11, 598, Modifiers::NONE);
    s.ui.chat_entry_key(ac_ui::input::InputAction::EnterChatMode, None);
    s.step();
    s.click(0, 386, 598, Modifiers::NONE);
    for c in "Trade watch".chars() {
        let r: Routed<u32> = s.ui.key(
            0,
            Key::A,
            true,
            false,
            Modifiers::NONE,
            Some(c.to_string().as_str()),
        );
        let _ = s.ui.handle(r.outputs);
    }
    // The dialog sits in the middle: 380 x 400 at (530, 250).
    let (dx, dy) = (530, 250);
    s.click(0, dx + 16 + 116 + 20, dy + 130 + 22 + 11, Modifiers::NONE); // Trade
    s.click(0, dx + 16 + 116 + 20, dy + 130 + 44 + 11, Modifiers::NONE); // LFG
    s.click(0, dx + 16 + 20, dy + 130 + 22 + 11, Modifiers::NONE); // Tells
    for _ in 0..8 {
        s.click(0, dx + 16 + 90 + 20, dy + 252 + 13, Modifiers::NONE); // Talks on
    }
    s.click(0, dx + 16 + 15, dy + 286 + 11, Modifiers::NONE); // Own window
    s.save("chat-new-tab");
    s.click(0, dx + 380 - 16 - 30, dy + 356 + 14, Modifiers::NONE); // Make tab
    s.scene.apply(PresentationEvent::Chat(vec![
        chat_line(
            20,
            K::Channel,
            T::TurbineTrade,
            "Osk",
            "WTS Major Coordination, 40k",
            "",
        ),
        chat_line(
            21,
            K::Channel,
            T::TurbineLfg,
            "Varn",
            "LF2M Lugian quest, meet at the Holtburg lifestone",
            "",
        ),
        chat_line(
            22,
            K::Channel,
            T::TurbineTrade,
            "Corvan",
            "WTB Mana Scarabs, paying well",
            "",
        ),
    ]));
    for _ in 0..12 {
        s.step();
    }
    s.save("chat-window");
    // Its close button (440 wide at (500, 300); the X is the last of the
    // two 24 px buttons on its title row): the tab goes back into the
    // panel.
    s.click(0, 500 + 440 - 4 - 12, 300 + 2 + 12, Modifiers::NONE);
    s.ui.chat_entry_key(ac_ui::input::InputAction::EnterChatMode, None);
    for _ in 0..4 {
        s.step();
    }
    s.save("chat-window-closed");
}

/// More of the player's things for the inventory: a side pack with salvage
/// in it, a worn coat and a wielded wand, and value and burden on all.
fn more_things(s: &mut Shot) {
    use ac_core::items::EquipMask;
    let pack = 0x8000_0201;
    let mut objects = Vec::new();
    let mut p = ClientObject::new(PLAYER);
    p.name = "Aerin Vale".to_owned();
    p.item_type = ItemType::CREATURE;
    p.items_capacity = 102;
    p.containers_capacity = 7;
    objects.push(p);
    let mut main = Vec::new();
    for &(guid, name, item_type, icon, stack) in ITEMS {
        let mut o = ClientObject::new(guid);
        o.name = name.to_owned();
        o.item_type = item_type;
        o.icon_id = icon;
        o.stack_size = stack;
        o.stack_size_max = if stack > 1 { 100 } else { 1 };
        o.container_id = PLAYER;
        o.value = 120 * stack.max(1);
        o.burden = 15 * stack.max(1);
        if item_type == ItemType::MELEE_WEAPON {
            o.valid_locations = EquipMask::MELEE_WEAPON;
            o.value = 18_400;
            o.burden = 450;
            o.workmanship = 8.0;
        }
        objects.push(o);
        main.push(guid);
    }
    let mut bag = ClientObject::new(pack);
    bag.name = "Salvage Sack".to_owned();
    bag.item_type = ItemType::CONTAINER;
    bag.items_capacity = 24;
    bag.icon_id = 0x0600_127E;
    bag.container_id = PLAYER;
    objects.push(bag);
    main.push(pack);
    let mut in_pack = Vec::new();
    for (i, name) in ["Salvaged Iron", "Salvaged Granite", "Salvaged Steel"]
        .iter()
        .enumerate()
    {
        let guid = 0x8000_0301 + i as u32;
        let mut o = ClientObject::new(guid);
        o.name = (*name).to_owned();
        o.item_type = ItemType::TINKERING_MATERIAL;
        o.icon_id = 0x0600_1E3B;
        o.container_id = pack;
        o.burden = 100;
        objects.push(o);
        in_pack.push(guid);
    }
    let mut coat = ClientObject::new(0x8000_0401);
    coat.name = "Olthoi Amuli Coat".to_owned();
    coat.item_type = ItemType::ARMOR;
    coat.icon_id = 0x0600_11CB;
    coat.valid_locations = EquipMask::CHEST_ARMOR | EquipMask::UPPER_ARM_ARMOR;
    coat.currently_equipped_location = coat.valid_locations;
    coat.wielder_id = PLAYER;
    coat.burden = 1240;
    objects.push(coat);
    s.scene
        .apply(PresentationEvent::Interface(InterfaceEvent::Items(
            ItemsUpdate {
                reset: true,
                objects,
                removed: Vec::new(),
                contents: vec![(PLAYER, Some(main)), (pack, Some(in_pack))],
            },
        )));
    s.step();
    s.step();
}

/// The inventory window's top left on a 1440 x 900 screen (centred).
const INV: (i32, i32) = (180, 130);

#[test]
#[ignore = "needs the retail dat files; writes pictures"]
fn the_modern_inventory() {
    use ac_core::combat::CombatMode;
    use ac_render::ui::window::Hotkey;
    let mut s = Shot::new(CombatMode::NON_COMBAT);
    more_things(&mut s);
    assert!(s.ui.run_hotkey(Hotkey::Inventory));
    s.step();
    s.step();
    s.save("inventory-open");
    // The figure is asked for, turned by a drag across it.
    let figure = |s: &mut Shot| {
        let (requests, _) = s.ui.viewport_requests(&s.scene, 1.0);
        requests.into_iter().find_map(|r| match r.subject {
            ac_render::viewport::ViewportSubject::Paperdoll { turn, .. } => Some(turn),
            _ => None,
        })
    };
    assert_eq!(figure(&mut s), Some(0.0));
    s.drag((INV.0 + 100, INV.1 + 200), (INV.0 + 180, INV.1 + 200));
    let turned = figure(&mut s).expect("the figure");
    println!("figure turned {turned}");
    assert!(turned > 0.5);
    // The sword: the first cell of the grid.
    let first = (INV.0 + 416 + 20, INV.1 + 158 + 20);
    s.click(0, first.0, first.1, Modifiers::NONE);
    // No server here to answer the selection: it is made as it would be.
    s.scene.apply(PresentationEvent::Player(PlayerView {
        selected: Some(0x8000_0101),
        ..PlayerView::default()
    }));
    s.step();
    s.step();
    s.save("inventory-selected");
    // Its appraisal answer.
    let mut a = ac_proto::appraise::IdentifyObjectResponse {
        guid: 0x8000_0101,
        success: true,
        ..Default::default()
    };
    a.properties.ints.insert(19, 18_400);
    a.properties.ints.insert(5, 450);
    a.properties.ints.insert(105, 8);
    a.properties
        .strings
        .insert(16, "A plain sword, well kept.".to_owned());
    s.scene
        .apply(PresentationEvent::Interface(InterfaceEvent::Appraisal(
            Box::new(ac_runtime::inventory::AppraisalView {
                name: Some("Sword".to_owned()),
                response: a,
                present_in_ui: true,
                opens_window: false,
            }),
        )));
    s.step();
    s.step();
    s.save("inventory-details");
    // Its menu.
    s.click(1, first.0, first.1, Modifiers::NONE);
    s.save("inventory-item-menu");
    // A click away closes it.
    let away = (INV.0 + 200, INV.1 + 560);
    s.click(0, away.0, away.1, Modifiers::NONE);
    // The sort menu.
    s.click(0, INV.0 + 416 + 30, INV.1 + 94 + 13, Modifiers::NONE);
    s.save("inventory-sort-menu");
    s.click(0, away.0, away.1, Modifiers::NONE);
    // All packs, as a list.
    s.click(0, INV.0 + 348 + 28, INV.1 + 68 + 20, Modifiers::NONE);
    s.save("inventory-all");
    s.click(
        0,
        INV.0 + 416 + 352 - 12 - 13,
        INV.1 + 94 + 13,
        Modifiers::NONE,
    );
    s.save("inventory-all-list");
    // The side pack.
    s.click(
        0,
        INV.0 + 348 + 28,
        INV.1 + 68 + 40 + 12 + 50 + 6 + 20,
        Modifiers::NONE,
    );
    s.save("inventory-side-pack");
}

/// The visible root child holding a `T`.
fn shown<T: ac_ui::tree::Widget>(s: &Shot) -> Option<ac_ui::tree::ElementId> {
    let root = &s.ui.host().root;
    root.children(root.root())
        .into_iter()
        .find(|&c| root.widget::<T>(c).is_some() && root.element(c).is_some_and(|e| e.visible))
}

#[test]
#[ignore = "needs the retail dat files; writes pictures"]
fn the_modern_split() {
    use ac_core::combat::CombatMode;
    use ac_render::ui::window::Hotkey;
    use ac_ui_window::ui::modern::items::menu::PopMenu;
    use ac_ui_window::ui::modern::items::split::SplitPopover;
    let mut s = Shot::new(CombatMode::NON_COMBAT);
    more_things(&mut s);
    assert!(s.ui.run_hotkey(Hotkey::Inventory));
    s.step();
    s.step();
    // The potions: the first cell of the grid, a stack of 12.
    let first = (INV.0 + 416 + 20, INV.1 + 158 + 20);
    s.click(1, first.0, first.1, Modifiers::NONE);
    let menu = shown::<PopMenu>(&s).expect("the menu");
    let (mx, my, mw, _) = rect_of(&s, menu);
    let (top, h) =
        s.ui.host()
            .root
            .widget::<PopMenu>(menu)
            .and_then(|m| m.entry_rows("Split the stack\u{2026}"))
            .expect("a split entry");
    s.click(0, mx + mw / 2, my + (top + h / 2.0) as i32, Modifiers::NONE);
    let pop = shown::<SplitPopover>(&s).expect("the split pop-up");
    let (stack, amount) = {
        let p = s.ui.host().root.widget::<SplitPopover>(pop).unwrap();
        (p.stack, p.amount)
    };
    assert_eq!((stack, amount), (12, 6));
    s.save("split-open");
    // The count typed over the half chosen, then Enter.
    let out = ui_input::typed(s.ui.host_mut(), "5", Modifiers::NONE);
    let _ = s.ui.handle(out);
    s.step();
    s.save("split-typed");
    s.key(Key::Enter, Modifiers::NONE, true);
    s.key(Key::Enter, Modifiers::NONE, false);
    assert!(shown::<SplitPopover>(&s).is_none(), "the pop-up closed");
    let moves: Vec<String> = s
        .sent
        .iter()
        .map(|c| format!("{c:?}"))
        .filter(|c| c.contains("MoveToContainer"))
        .collect();
    println!("{moves:?}");
    assert_eq!(moves.len(), 1);
    assert!(moves[0].contains("amount: 5"));
    assert!(moves[0].contains("item: Guid(2147483905)"));
}

/// An answer about `guid` the player asked for.
fn answer(s: &mut Shot, a: ac_proto::appraise::IdentifyObjectResponse, opens: bool) {
    s.scene
        .apply(PresentationEvent::Interface(InterfaceEvent::Appraisal(
            Box::new(ac_runtime::inventory::AppraisalView {
                name: None,
                response: a,
                present_in_ui: true,
                opens_window: opens,
            }),
        )));
    s.step();
    s.step();
}

fn creature(guid: u32, success: bool) -> ac_proto::appraise::IdentifyObjectResponse {
    use ac_proto::appraise::{CreatureAttributes, CreatureProfile, IdentifyObjectResponse};
    let mut a = IdentifyObjectResponse {
        guid,
        success,
        creature_profile: Some(CreatureProfile {
            flags: 0,
            health: 306,
            health_max: 450,
            attributes: Some(CreatureAttributes {
                strength: 170,
                endurance: 200,
                quickness: 150,
                coordination: 160,
                focus: 90,
                self_: 90,
                stamina: 400,
                mana: 0,
                stamina_max: 400,
                mana_max: 0,
            }),
            attribute_highlights: Some((0b101, 0b100)),
        }),
        ..Default::default()
    };
    a.properties.ints.insert(25, 87);
    a.properties.ints.insert(2, 3);
    a.properties.ints.insert(307, 10);
    a.properties.strings.insert(1, "Drudge Ravener".to_owned());
    a
}

#[test]
#[ignore = "needs the retail dat files; writes pictures"]
fn the_modern_inspect() {
    use ac_core::combat::CombatMode;
    let mut s = Shot::new(CombatMode::NON_COMBAT);
    more_things(&mut s);
    // A creature the player assessed.
    answer(&mut s, creature(0x8000_0901, true), true);
    s.save("inspect-creature");
    // A failed assessment.
    answer(&mut s, creature(0x8000_0902, false), true);
    s.save("inspect-failed");
    // A player.
    let mut p = creature(0x5000_0002, true);
    p.properties.strings.insert(5, "Knight".to_owned());
    p.properties.ints.insert(0x71, 1);
    p.properties.ints.insert(0xBC, 3);
    p.armor_levels = Some(ac_proto::appraise::ArmorLevels {
        head: 812,
        chest: 846,
        abdomen: 846,
        upper_arm: 790,
        lower_arm: 790,
        hand: 744,
        upper_leg: 802,
        lower_leg: 802,
        foot: 760,
    });
    answer(&mut s, p, true);
    s.save("inspect-player");
    // The sword, with value and a description.
    let mut a = ac_proto::appraise::IdentifyObjectResponse {
        guid: 0x8000_0105,
        success: true,
        ..Default::default()
    };
    a.properties.ints.insert(19, 18_400);
    a.properties.ints.insert(5, 450);
    a.properties.ints.insert(105, 8);
    a.properties.strings.insert(
        16,
        "A curved blade of mithril, cold to the touch.".to_owned(),
    );
    answer(&mut s, a, true);
    s.save("inspect-item");
    // A spell examined from a bar.
    let spell =
        s.ui.facts()
            .numbers()
            .and_then(|n| n.spellbook())
            .and_then(|b| b.learned_spells().next())
            .expect("a spell");
    let _ = s.ui.handle(vec![ac_ui::root::UiOutput::Panel(
        ac_ui::panels::PanelOutput::ExamineSpell(spell),
    )]);
    s.step();
    s.save("inspect-spell");
}

/// The screen rectangle of element `id`.
fn rect_of(s: &Shot, id: ac_ui::tree::ElementId) -> (i32, i32, i32, i32) {
    let root = &s.ui.host().root;
    let (x, y) = root.screen_position(id);
    let e = root.element(id).expect("the element");
    (x as i32, y as i32, e.width as i32, e.height as i32)
}

#[test]
#[ignore = "needs the retail dat files; writes pictures"]
fn the_modern_inscription() {
    use ac_core::combat::CombatMode;
    use ac_core::items::PublicWeenieFlags;
    use ac_ui::widgets::Field;
    use ac_ui_window::ui::modern::items::sheet_view::SheetView;
    let mut s = Shot::new(CombatMode::NON_COMBAT);
    more_things(&mut s);
    // The sword can be written on.
    let sword = 0x8000_0105;
    let mut o = s.ui.facts().objects.get(sword).cloned().expect("the sword");
    o.public_weenie_bitfield = Some(PublicWeenieFlags::INSCRIBABLE);
    s.scene
        .apply(PresentationEvent::Interface(InterfaceEvent::Items(
            ItemsUpdate {
                reset: false,
                objects: vec![o],
                removed: Vec::new(),
                contents: Vec::new(),
            },
        )));
    s.step();
    let mut a = ac_proto::appraise::IdentifyObjectResponse {
        guid: sword,
        success: true,
        ..Default::default()
    };
    a.properties.ints.insert(19, 18_400);
    a.properties.ints.insert(5, 450);
    answer(&mut s, a, true);
    // Its Details tab.
    let frame =
        s.ui.host()
            .root
            .window("modern-inspect")
            .expect("Inspect")
            .frame;
    let (fx, fy, fw, _) = rect_of(&s, frame);
    let tab =
        s.ui.host()
            .root
            .widget::<SheetView>(frame)
            .and_then(|v| v.tab_rect("Details", fw as f32))
            .expect("a Details tab");
    s.click(
        0,
        fx + (tab.0 + tab.2 / 2.0) as i32,
        fy + (tab.1 + tab.3 / 2.0) as i32,
        Modifiers::NONE,
    );
    s.step();
    // The field over the box.
    let field =
        s.ui.host()
            .root
            .children(frame)
            .into_iter()
            .find(|&c| s.ui.host().root.widget::<Field>(c).is_some())
            .expect("the inscription field");
    assert!(
        s.ui.host().root.element(field).is_some_and(|e| e.visible),
        "the field shows"
    );
    s.save("inscription-empty");
    let (x, y, w, h) = rect_of(&s, field);
    s.click(0, x + w / 2, y + h / 2, Modifiers::NONE);
    let out = ui_input::typed(s.ui.host_mut(), "For Dereth", Modifiers::NONE);
    let _ = s.ui.handle(out);
    s.step();
    s.save("inscription-writing");
    assert!(!s.sent.iter().any(|c| format!("{c:?}").contains("Inscribe")));
    // A click elsewhere on the sheet signs it.
    s.click(0, fx + 30, fy + 30, Modifiers::NONE);
    let sent: Vec<String> = s
        .sent
        .iter()
        .map(|c| format!("{c:?}"))
        .filter(|c| c.contains("Inscribe"))
        .collect();
    println!("{sent:?}");
    assert_eq!(sent.len(), 1);
    assert!(sent[0].contains("For Dereth"));
    s.save("inscription-signed");
}

/// A corpse beside the player with a few things in it, open (or out of
/// reach when `in_range` is false).
fn open_corpse(s: &mut Shot, in_range: bool) {
    let corpse = 0x8000_0A01;
    let mut objects = Vec::new();
    let mut c = ClientObject::new(corpse);
    c.name = "Corpse of Drudge Ravener".to_owned();
    c.item_type = ItemType::CONTAINER;
    c.items_capacity = 120;
    c.icon_id = 0x0600_127E;
    objects.push(c);
    let things: &[(u32, &str, ItemType, u32, i32, i32)] = &[
        (0x8000_0A11, "Pyreal", ItemType::MONEY, 0x0600_1A8B, 212, 0),
        (
            0x8000_0A12,
            "Drudge Raider Totem",
            ItemType::MISC,
            0x0600_1E3B,
            1,
            50,
        ),
        (
            0x8000_0A13,
            "Short Sword",
            ItemType::MELEE_WEAPON,
            0x0600_11CB,
            1,
            180,
        ),
        (
            0x8000_0A14,
            "Healing Kit",
            ItemType::MISC,
            0x0600_1E3B,
            1,
            40,
        ),
    ];
    let mut inside = Vec::new();
    for &(guid, name, t, icon, stack, burden) in things {
        let mut o = ClientObject::new(guid);
        o.name = name.to_owned();
        o.plural_name = format!("{name}s");
        o.item_type = t;
        o.icon_id = icon;
        o.stack_size = stack;
        o.container_id = corpse;
        o.burden = burden;
        o.value = if t == ItemType::MONEY { stack } else { 860 };
        o.attuned = name.contains("Totem");
        objects.push(o);
        inside.push(guid);
    }
    s.scene
        .apply(PresentationEvent::Interface(InterfaceEvent::Items(
            ItemsUpdate {
                reset: false,
                objects,
                removed: Vec::new(),
                contents: vec![(corpse, Some(inside))],
            },
        )));
    s.scene
        .apply(PresentationEvent::Interface(InterfaceEvent::State(
            Box::new(InterfaceState {
                player: PLAYER,
                can_make_request: true,
                external_container: corpse,
                external_container_in_range: in_range,
                ..InterfaceState::default()
            }),
        )));
    s.step();
    s.step();
}

/// The Loot window's top left on a 1440 x 900 screen.
const LOOT: (i32, i32) = (1440 - 60 - 400, 250);

#[test]
#[ignore = "needs the retail dat files; writes pictures"]
fn the_modern_loot() {
    use ac_core::combat::CombatMode;
    use ac_runtime::inventory::{ItemCommand, ObjectRef};
    use ac_runtime::session::SessionCommand;
    let mut s = Shot::new(CombatMode::NON_COMBAT);
    more_things(&mut s);
    open_corpse(&mut s, true);
    s.save("loot-corpse");
    // The sword selected: its row's buttons.
    s.scene.apply(PresentationEvent::Player(PlayerView {
        selected: Some(0x8000_0A13),
        ..PlayerView::default()
    }));
    s.step();
    s.step();
    s.save("loot-selected");
    // Its Loot button (the third row, 42 a row, the list 64 down).
    s.sent.clear();
    let row = (LOOT.0 + 8, LOOT.1 + 64 + 2 * 44);
    s.click(0, row.0 + 384 - 12 - 6 - 29, row.1 + 20, Modifiers::NONE);
    println!("loot: {:?}", s.sent);
    assert!(s.sent.iter().any(|c| matches!(
        c,
        SessionCommand::Item(ItemCommand::DropInto {
            item: ObjectRef::Guid(0x8000_0A13),
            pickup: true,
            ..
        })
    )));
    // The right-click menu on the totem.
    s.click(1, row.0 + 100, LOOT.1 + 64 + 44 + 20, Modifiers::NONE);
    s.save("loot-menu");
    s.click(0, 300, 300, Modifiers::NONE);
    // Walking away closes it.
    s.sent.clear();
    open_corpse(&mut s, false);
    s.save("loot-gone");
    assert!(s.sent.iter().any(|c| matches!(
        c,
        SessionCommand::Item(ItemCommand::CloseContainer(ObjectRef::Guid(0x8000_0A01)))
    )));
}

/// A plugin's notes on the potions (kept, with a card and a menu entry),
/// the sword and a corpse's sword (to salvage), and its part of the
/// inventory's and loot window's feet, as the session hands them over.
fn plugin_notes(s: &mut Shot) {
    use ac_plugin_ui::{
        FootButton, ItemMenuEntry, ItemNote, ItemWindow, NoteTone, PluginUi, UiOwner, WindowFoot,
    };
    let owner = UiOwner::new("example.looter", "Looter");
    let mut state = PluginUi::new();
    let note = |object_id, mark: &str, tone| ItemNote {
        object_id,
        mark: mark.to_owned(),
        tone,
        lines: Vec::new(),
        menu: Vec::new(),
    };
    let mut potions = note(0x8000_0101, "Keep", NoteTone::Good);
    potions.lines = vec![
        "Rule: Healing kept up to 20".to_owned(),
        "Carrying 12 of 20.".to_owned(),
    ];
    potions.menu = vec![ItemMenuEntry {
        id: "never".to_owned(),
        label: "Never loot this".to_owned(),
    }];
    assert!(state.set_item_notes(
        &owner,
        vec![
            potions,
            note(0x8000_0105, "Salvage", NoteTone::Warning),
            note(0x8000_0A13, "Salvage", NoteTone::Warning),
            note(0x8000_0A12, "Sell", NoteTone::Bad),
        ]
    ));
    let foot = |summary: &str, label: &str| WindowFoot {
        summary: summary.to_owned(),
        buttons: vec![FootButton {
            id: "go".to_owned(),
            label: label.to_owned(),
            enabled: true,
        }],
    };
    assert!(state.set_window_foot(
        &owner,
        ItemWindow::Inventory,
        foot("1 to salvage", "Salvage 1")
    ));
    assert!(state.set_window_foot(
        &owner,
        ItemWindow::Loot,
        foot("1 to salvage, 1 to sell", "Loot 2")
    ));
    s.scene
        .apply(PresentationEvent::Interface(InterfaceEvent::PluginUi(
            state.snapshot(),
        )));
    s.step();
    s.step();
}

/// The first visible element under `root`'s children holding a `T`.
fn shown_in<T: ac_ui::tree::Widget>(
    s: &Shot,
    parent: ac_ui::tree::ElementId,
) -> Option<ac_ui::tree::ElementId> {
    let root = &s.ui.host().root;
    root.children(parent)
        .into_iter()
        .find(|&c| root.widget::<T>(c).is_some() && root.element(c).is_some_and(|e| e.visible))
}

#[test]
#[ignore = "needs the retail dat files; writes pictures"]
fn the_modern_plugin_notes() {
    use ac_core::combat::CombatMode;
    use ac_plugin_ui::{ItemWindow, UiInput};
    use ac_render::ui::window::Hotkey;
    use ac_runtime::session::SessionCommand;
    use ac_ui_window::ui::modern::items::menu::PopMenu;
    use ac_ui_window::ui::modern::items::plugin_notes::FootBand;
    let mut s = Shot::new(CombatMode::NON_COMBAT);
    more_things(&mut s);
    plugin_notes(&mut s);
    assert!(s.ui.run_hotkey(Hotkey::Inventory));
    s.step();
    s.step();
    s.save("plugin-notes-inventory");
    // The potions selected: the details carry the plugin's card.
    let first = (INV.0 + 416 + 20, INV.1 + 158 + 20);
    s.click(0, first.0, first.1, Modifiers::NONE);
    s.scene.apply(PresentationEvent::Player(PlayerView {
        selected: Some(0x8000_0101),
        ..PlayerView::default()
    }));
    let a = ac_proto::appraise::IdentifyObjectResponse {
        guid: 0x8000_0101,
        success: true,
        ..Default::default()
    };
    answer(&mut s, a, false);
    s.save("plugin-notes-details");
    // Its menu carries the plugin's entry; choosing it goes to the plugin.
    s.sent.clear();
    s.click(1, first.0, first.1, Modifiers::NONE);
    s.save("plugin-notes-menu");
    let menu = shown::<PopMenu>(&s).expect("the menu");
    let (mx, my, mw, _) = rect_of(&s, menu);
    let (top, h) =
        s.ui.host()
            .root
            .widget::<PopMenu>(menu)
            .and_then(|m| m.entry_rows("Never loot this"))
            .expect("the plugin's entry");
    s.click(0, mx + mw / 2, my + (top + h / 2.0) as i32, Modifiers::NONE);
    assert!(
        s.sent.iter().any(|c| matches!(
            c,
            SessionCommand::PluginUi(UiInput::ItemMenu { owner, id, object })
                if owner == "example.looter" && id == "never" && *object == 0x8000_0101
        )),
        "{:?}",
        s.sent
    );
    // The band's button, at the right of its row.
    let frame =
        s.ui.host()
            .root
            .window("modern-inventory")
            .expect("inventory")
            .frame;
    let band = shown_in::<FootBand>(&s, frame).expect("the inventory's band");
    let (bx, by, bw, _) = rect_of(&s, band);
    s.sent.clear();
    s.click(0, bx + bw - 12 - 20, by + 2 + 4 + 12, Modifiers::NONE);
    assert!(
        s.sent.iter().any(|c| matches!(
            c,
            SessionCommand::PluginUi(UiInput::FootButton { owner, window: ItemWindow::Inventory, id })
                if owner == "example.looter" && id == "go"
        )),
        "{:?}",
        s.sent
    );
    // The loot window: marks on its rows, its band over the foot.
    open_corpse(&mut s, true);
    s.save("plugin-notes-loot");
    let frame = s.ui.host().root.window("modern-loot").expect("loot").frame;
    assert!(
        shown_in::<FootBand>(&s, frame).is_some(),
        "the loot window's band"
    );
}

/// An effect made up for the pictures: spell name, layer, caster, start,
/// duration, stat-mod type, key and value, category.
type MadeEffect = (&'static str, u32, u32, f64, f64, u32, u32, f32, u32);

/// The first spell of the dats' table whose name is `name`.
fn spell_named(tables: &ac_runtime::gameplay::GameTables, name: &str) -> Option<u32> {
    let spells = tables.spells();
    spells
        .spell_ids()
        .find(|id| spells.get(*id).is_some_and(|m| m.name == name))
}

/// Effects on the player: helpful ones of its own (one ending soon, one
/// with a weaker copy underneath), one from someone else, a cantrip from
/// worn gear, two harmful ones and vitae. Returns the spells used.
fn effects_on_player(s: &mut Shot) -> Vec<u32> {
    use ac_core::spells::{ActiveEnchantmentRecord, EnchantmentTypeFlags};
    let dats = dats();
    let tables = ac_runtime::gameplay::GameTables::load(&dats).expect("the game tables");
    let mut book = Spellbook::new(Some(Arc::clone(tables.spells())));
    let attr = EnchantmentTypeFlags::ATTRIBUTE.bits() | EnchantmentTypeFlags::ADDITIVE.bits();
    let skill = EnchantmentTypeFlags::SKILL.bits() | EnchantmentTypeFlags::ADDITIVE.bits();
    let resist = EnchantmentTypeFlags::FLOAT.bits() | EnchantmentTypeFlags::MULTIPLICATIVE.bits();
    let list: &[MadeEffect] = &[
        (
            "Focus Self VI",
            1,
            PLAYER,
            -2500.0,
            2700.0,
            attr,
            5,
            35.0,
            900,
        ),
        (
            "Strength Self VI",
            1,
            PLAYER,
            -900.0,
            2700.0,
            attr,
            1,
            35.0,
            901,
        ),
        (
            "Strength Other V",
            2,
            0x5000_0099,
            -600.0,
            1800.0,
            attr,
            1,
            25.0,
            901,
        ),
        (
            "Invulnerability Self VI",
            1,
            PLAYER,
            -100.0,
            2700.0,
            skill,
            6,
            35.0,
            902,
        ),
        (
            "Fire Protection Self VI",
            1,
            0x5000_0099,
            -50.0,
            2700.0,
            resist,
            67,
            0.5,
            903,
        ),
        (
            "Minor Strength",
            1,
            0x8000_0105,
            0.0,
            -1.0,
            attr,
            1,
            10.0,
            904,
        ),
        (
            "Weakness Other VI",
            1,
            0x5000_00AA,
            -60.0,
            240.0,
            attr,
            1,
            -35.0,
            905,
        ),
        (
            "Clumsiness Other VI",
            1,
            0x5000_00AA,
            -10.0,
            240.0,
            attr,
            4,
            -35.0,
            906,
        ),
    ];
    let mut used = Vec::new();
    for &(name, layer, caster, start, duration, kind, key, value, category) in list {
        let Some(id) = spell_named(&tables, name) else {
            println!("no spell named {name}");
            continue;
        };
        book.on_spell_learned(id, 1.0);
        used.push(id);
        book.on_enchantment_added(ActiveEnchantmentRecord {
            stat_mod_type: Some(kind),
            stat_mod_key: Some(key),
            stat_mod_value: Some(value),
            bucket: if kind & EnchantmentTypeFlags::ADDITIVE.bits() != 0 {
                2
            } else {
                1
            },
            start_time: start,
            spell_category: category,
            power_level: if layer == 1 { 300 } else { 250 },
            ..ActiveEnchantmentRecord::new(id, layer, duration, caster)
        });
    }
    // Vitae.
    book.on_enchantment_added(ActiveEnchantmentRecord {
        stat_mod_type: Some(0),
        stat_mod_value: Some(0.94),
        bucket: 4,
        start_time: -100.0,
        ..ActiveEnchantmentRecord::new(666, 1, -1.0, 0)
    });
    s.scene
        .apply(PresentationEvent::Interface(InterfaceEvent::Player(
            Box::new(PlayerFacts {
                revision: 3,
                numbers: ac_core::player::LocalPlayerState::new(Some(book)),
                session_seconds: 0.0,
            }),
        )));
    s.step();
    s.step();
    used
}

#[test]
#[ignore = "needs the retail dat files; writes pictures"]
fn the_modern_effects() {
    use ac_core::combat::CombatMode;
    use ac_ui_window::ui::modern::magic::effects_window::EffectsWindow;
    let mut s = Shot::new(CombatMode::NON_COMBAT);
    let used = effects_on_player(&mut s);
    assert!(used.len() >= 4, "the dats' spells: {used:?}");
    s.save("effects-hud");
    // The helpful count opens the window on Helpful.
    s.click(0, 16 + 24, 14 + 14, Modifiers::NONE);
    assert!(
        s.ui.host().root.is_window_visible("modern-effects-window"),
        "the window opens from the count"
    );
    s.save("effects-helpful");
    let frame =
        s.ui.host()
            .root
            .window("modern-effects-window")
            .expect("the window")
            .frame;
    let (fx, fy, _, _) = rect_of(&s, frame);
    {
        let w = s.ui.host().root.widget::<EffectsWindow>(frame).expect("it");
        assert!(!w.harmful);
        assert!(w.list.helpful.len() >= 3, "{:?}", w.list.helpful);
        assert!(w.list.vitae.is_some());
    }
    // The Strength row (with the weaker copy underneath) chosen.
    let strength = {
        let w = s.ui.host().root.widget::<EffectsWindow>(frame).expect("it");
        w.list
            .helpful
            .iter()
            .position(|e| e.under.is_some())
            .expect("an effect with a copy underneath")
    };
    let _ = strength;
    // The Timed group's first row: after Ending soon's heading and row.
    let y = fy + 104 + 32 + 8 + 26 + 8 + 26 + 46 + 26 + 20;
    s.click(0, fx + 120, y, Modifiers::NONE);
    s.save("effects-chosen");
    // The Skills chip.
    s.click(
        0,
        fx + 20 + 60 + 4 + 90 + 4 + 30,
        fy + 104 + 40 + 13,
        Modifiers::NONE,
    );
    s.save("effects-skills");
    // The Harmful tab, with vitae.
    s.click(0, fx + 150, fy + 52 + 20, Modifiers::NONE);
    {
        let w = s.ui.host().root.widget::<EffectsWindow>(frame).expect("it");
        assert!(w.harmful, "the Harmful tab shows");
    }
    s.save("effects-harmful");
}

/// The player carries packs of the first six components the dats name
/// (by their weenie classes), a few of each.
fn carry_components(s: &mut Shot) {
    let dats = dats();
    let table: ac_dat::objects::SpellComponentTable = dats
        .get(ac_dat::objects::SPELL_COMPONENT_TABLE_ID)
        .expect("read")
        .expect("the component table");
    let map: ac_dat::objects::EnumIdMap = dats.get(0x2700_0002).expect("read").expect("the map");
    let mut objects = Vec::new();
    let mut guids = Vec::new();
    for (i, (id, base)) in table.components.iter().take(6).enumerate() {
        let Some(class) = map.client_id(*id) else {
            continue;
        };
        let guid = 0x8000_0C00 + i as u32;
        let mut o = ClientObject::new(guid);
        o.name = base.name.clone();
        o.weenie_class_id = class;
        o.item_type = ItemType::SPELL_COMPONENTS;
        o.icon_id = base.icon;
        o.stack_size = [48, 4, 0, 22, 9, 120][i];
        o.container_id = PLAYER;
        if o.stack_size > 0 {
            objects.push(o);
            guids.push(guid);
        }
    }
    s.scene
        .apply(PresentationEvent::Interface(InterfaceEvent::Items(
            ItemsUpdate {
                reset: false,
                objects,
                removed: Vec::new(),
                contents: Vec::new(),
            },
        )));
    s.step();
    s.step();
}

#[test]
#[ignore = "needs the retail dat files; writes pictures"]
fn the_modern_spellbook() {
    use ac_core::combat::CombatMode;
    use ac_render::ui::window::Hotkey;
    use ac_runtime::gameplay::GameplayCommand;
    use ac_runtime::session::SessionCommand;
    use ac_ui::panels::magic::SpellbookPage;
    use ac_ui_window::ui::modern::magic::spellbook_window::{BookFrame, ComponentTable, SpellList};
    let mut s = Shot::new(CombatMode::MAGIC);
    carry_components(&mut s);
    assert!(s.ui.run_hotkey(Hotkey::Spellbook(SpellbookPage::Spells)));
    s.step();
    s.step();
    assert!(
        s.ui.host().root.is_window_visible("modern-spellbook"),
        "the book opens from its key"
    );
    s.save("spellbook");
    let frame =
        s.ui.host()
            .root
            .window("modern-spellbook")
            .expect("book")
            .frame;
    let (fx, fy, _, _) = rect_of(&s, frame);
    let list = shown_in::<SpellList>(&s, frame).expect("the list");
    let (lx, ly, _, _) = rect_of(&s, list);
    {
        let l = s.ui.host().root.widget::<SpellList>(list).expect("it");
        assert!(!l.groups.is_empty(), "spells listed");
    }
    // The first spell chosen: its details.
    s.click(0, lx + 100, ly + 30 + 19, Modifiers::NONE);
    s.step();
    s.step();
    s.save("spellbook-chosen");
    let chosen = {
        let f = s.ui.host().root.widget::<BookFrame>(frame).expect("it");
        f.details.as_ref().map(|d| d.id).expect("details")
    };
    // Cast (in Magic) goes out.
    s.sent.clear();
    s.click(0, fx + 612 + 40, fy + 68 + 556 - 56 + 28, Modifiers::NONE);
    assert!(
        s.sent.iter().any(|c| matches!(
            c,
            SessionCommand::Gameplay(GameplayCommand::Cast { spell, .. }) if *spell == chosen.to_string()
        )),
        "{:?}",
        s.sent
    );
    // Forget asks first.
    s.sent.clear();
    s.click(
        0,
        fx + 612 + 304 - 14 - 16,
        fy + 68 + 556 - 56 + 28,
        Modifiers::NONE,
    );
    s.save("spellbook-forget");
    assert!(s.sent.is_empty());
    s.click(
        0,
        fx + 612 + 304 - 14 - 52,
        fy + 68 + 556 - 56 - 140 + 136 - 14 - 15,
        Modifiers::NONE,
    );
    assert!(
        s.sent.iter().any(|c| matches!(
            c,
            SessionCommand::Gameplay(GameplayCommand::ForgetSpell(spell)) if *spell == chosen.to_string()
        )),
        "{:?}",
        s.sent
    );
    // One school: the filter goes to the server.
    s.sent.clear();
    s.click(0, fx + 24 + 60, fy + 68 + 18 + 32 + 15, Modifiers::NONE);
    assert!(
        s.sent.iter().any(|c| matches!(
            c,
            SessionCommand::Gameplay(GameplayCommand::SetSpellbookFilter(_))
        )),
        "{:?}",
        s.sent
    );
    s.save("spellbook-school");
    // The components page.
    assert!(s.ui.run_hotkey(Hotkey::Spellbook(SpellbookPage::Components)));
    s.step();
    s.step();
    s.save("spellbook-components");
    let table = shown_in::<ComponentTable>(&s, frame).expect("the table shows");
    let (tx, ty, _, _) = rect_of(&s, table);
    // The first row's restock box: type 30 and Enter.
    let (bx, by, bw, bh) = ComponentTable::restock_rect(0.0);
    s.click(
        0,
        tx + (bx + bw / 2.0) as i32,
        ty + (by + bh / 2.0) as i32,
        Modifiers::NONE,
    );
    s.sent.clear();
    for c in "30".chars() {
        let r: Routed<u32> = s.ui.key(
            0,
            Key::Number3,
            true,
            false,
            Modifiers::NONE,
            Some(c.to_string().as_str()),
        );
        let _ = s.ui.handle(r.outputs);
        s.step();
    }
    s.save("spellbook-restock");
    s.key(Key::Enter, Modifiers::NONE, true);
    s.key(Key::Enter, Modifiers::NONE, false);
    s.step();
    assert!(
        s.sent.iter().any(|c| matches!(
            c,
            SessionCommand::Gameplay(GameplayCommand::SetDesiredComponent { amount: 30, .. })
        )),
        "{:?}",
        s.sent
    );
}

/// The player's numbers for the character window: level 126, every
/// attribute raised a way, the vitals, a dozen skills in every group,
/// experience to spend and a Focus buff.
fn character_numbers(s: &mut Shot) {
    use ac_core::spells::{ActiveEnchantmentRecord, EnchantmentTypeFlags};
    let dats = dats();
    let curves: ac_dat::objects::ExperienceTable = dats
        .get(ac_dat::objects::EXPERIENCE_TABLE_ID)
        .expect("read")
        .expect("the experience table");
    let tables = ac_runtime::gameplay::GameTables::load(&dats).expect("the game tables");
    let mut book = Spellbook::new(Some(Arc::clone(tables.spells())));
    if let Some(id) = spell_named(&tables, "Focus Self VI") {
        book.on_spell_learned(id, 1.0);
        book.on_enchantment_added(ActiveEnchantmentRecord {
            stat_mod_type: Some(
                EnchantmentTypeFlags::ATTRIBUTE.bits() | EnchantmentTypeFlags::ADDITIVE.bits(),
            ),
            stat_mod_key: Some(5),
            stat_mod_value: Some(35.0),
            bucket: 2,
            start_time: -100.0,
            spell_category: 900,
            power_level: 300,
            ..ActiveEnchantmentRecord::new(id, 1, 2700.0, PLAYER)
        });
    }
    let mut n = ac_core::player::LocalPlayerState::new(Some(book));
    let a = |r: u32| curves.attributes.get(r as usize).copied().unwrap_or(0);
    for (id, start, ranks) in [
        (1, 60, 40),
        (2, 60, 40),
        (3, 60, 40),
        (4, 60, 40),
        (5, 100, 190),
        (6, 100, 190),
    ] {
        n.on_attribute_update(id, ranks, start, a(ranks));
    }
    let v = |r: u32| curves.vitals.get(r as usize).copied().unwrap_or(0);
    for (id, ranks, current) in [(2, 80, 310), (4, 60, 380), (6, 120, 290)] {
        n.on_vital_update(id, ranks, 0, v(ranks), current);
    }
    let t = |r: u32| curves.trained_skills.get(r as usize).copied().unwrap_or(0);
    let sp = |r: u32| {
        curves
            .specialized_skills
            .get(r as usize)
            .copied()
            .unwrap_or(0)
    };
    // id, status (1 untrained, 2 trained, 3 specialized), ranks
    for (id, status, ranks) in [
        (34, 3, 180),
        (33, 3, 160),
        (16, 2, 150),
        (15, 3, 170),
        (31, 2, 150),
        (32, 2, 140),
        (14, 2, 120),
        (6, 2, 130),
        (24, 2, 100),
        (22, 2, 90),
        (21, 2, 60),
        (23, 1, 0),
        (20, 1, 0),
        (38, 1, 0),
        (37, 1, 0),
    ] {
        let xp = match status {
            3 => sp(ranks),
            2 => t(ranks),
            _ => 0,
        };
        n.on_skill_update(id, ranks, status, xp, 0, 0, 0.0, 0);
    }
    s.scene
        .apply(PresentationEvent::Interface(InterfaceEvent::Player(
            Box::new(PlayerFacts {
                revision: 4,
                numbers: n,
                session_seconds: 0.0,
            }),
        )));
    let mut player = ClientObject::new(PLAYER);
    player.name = "Aerin Vale".to_owned();
    player.item_type = ItemType::CREATURE;
    player.properties.ints.insert(0x19, 126);
    player.properties.ints.insert(0x18, 7);
    player.properties.ints.insert(0x71, 2);
    player.properties.ints.insert(0xBC, 1);
    player.properties.ints.insert(0x2B, 38);
    player.properties.ints.insert(0x7D, 5_270_400);
    let at = |l: usize| i64::try_from(curves.levels.get(l).copied().unwrap_or(0)).unwrap_or(0);
    player
        .properties
        .int64s
        .insert(1, at(126) + (at(127) - at(126)) * 64 / 100);
    player.properties.int64s.insert(2, 4_210_880_000);
    s.scene
        .apply(PresentationEvent::Interface(InterfaceEvent::Items(
            ItemsUpdate {
                reset: false,
                objects: vec![player],
                removed: Vec::new(),
                contents: Vec::new(),
            },
        )));
    s.step();
    s.step();
}

#[test]
#[ignore = "needs the retail dat files; writes pictures"]
fn the_modern_character() {
    use ac_core::combat::CombatMode;
    use ac_render::ui::window::Hotkey;
    use ac_runtime::session::SessionCommand;
    use ac_ui::panels::character::CharacterTab;
    use ac_ui_window::ui::modern::character::window::{CharacterFrame, SkillList};
    let mut s = Shot::new(CombatMode::NON_COMBAT);
    character_numbers(&mut s);
    s.scene
        .apply(PresentationEvent::Interface(InterfaceEvent::Titles(
            Box::new(ac_runtime::session::interface::TitleFacts {
                display_title_id: 2,
                title_ids: (1..=20).collect(),
                revision: 1,
                table_generation: 1,
            }),
        )));
    s.step();
    assert!(s.ui.run_hotkey(Hotkey::Character(CharacterTab::Attributes)));
    s.step();
    s.step();
    assert!(
        s.ui.host().root.is_window_visible("modern-character"),
        "the window opens from its key"
    );
    s.save("character");
    let frame =
        s.ui.host()
            .root
            .window("modern-character")
            .expect("it")
            .frame;
    let (fx, fy, _, _) = rect_of(&s, frame);
    {
        let f =
            s.ui.host()
                .root
                .widget::<CharacterFrame>(frame)
                .expect("it");
        assert_eq!(f.sheet.level, Some(126), "the sheet is read");
    }
    // Focus chosen: how it adds up, with its spell.
    let focus_y = fy + 172 + 26 + 4 * 44 + 21;
    s.click(0, fx + 80, focus_y, Modifiers::NONE);
    s.step();
    s.save("character-focus");
    // Strength's "+" raises it by one.
    s.sent.clear();
    s.click(
        0,
        fx + 24 + 300 - 6 - 13,
        fy + 172 + 26 + 21,
        Modifiers::NONE,
    );
    assert!(
        s.sent
            .iter()
            .any(|c| matches!(c, SessionCommand::Character(_))),
        "{:?}",
        s.sent
    );
    // A skill chosen in the list.
    let list = shown_in::<SkillList>(&s, frame).expect("the skills");
    let (lx, ly, _, _) = rect_of(&s, list);
    s.click(0, lx + 100, ly + 30 + 17, Modifiers::NONE);
    s.step();
    s.save("character-skill");
    // The titles and information pages.
    s.click(0, fx + 300, fy + 26, Modifiers::NONE);
    s.click(0, fx + 24 + 120, fy + 210 + 20, Modifiers::NONE);
    s.save("character-titles");
    assert!(
        shown_in::<ac_ui_window::ui::modern::character::window::TitleGrid>(&s, frame).is_some(),
        "the titles page shows its grid"
    );
    s.click(0, fx + 380, fy + 26, Modifiers::NONE);
    s.save("character-info");
    {
        let f =
            s.ui.host()
                .root
                .widget::<CharacterFrame>(frame)
                .expect("it");
        assert!(!f.extras.info.is_empty(), "the information cards");
    }
}

#[test]
#[ignore = "needs the retail dat files; writes pictures"]
fn the_modern_options() {
    use ac_core::combat::CombatMode;
    use ac_render::ui::window::Hotkey;
    use ac_runtime::session::SessionCommand;
    let mut s = Shot::new(CombatMode::NON_COMBAT);
    s.ui.attach_key_bindings(&ac_ui::input::KeyBindings::retail_defaults());
    assert!(s.ui.run_hotkey(Hotkey::Options));
    s.step();
    s.step();
    assert!(
        s.ui.host().root.is_window_visible("modern-options"),
        "the window opens from its key"
    );
    assert!(
        !s.ui.host().root.is_window_visible("options"),
        "retail's stays hidden"
    );
    s.save("options");
    let frame = s.ui.host().root.window("modern-options").expect("it").frame;
    let (fx, fy, _, _) = rect_of(&s, frame);
    let nav = |k: i32| fy + 52 + 14 + k * 38 + 18;
    // Gameplay; Auto target switched off sends the option.
    s.click(0, fx + 60, nav(1), Modifiers::NONE);
    s.sent.clear();
    s.click(0, fx + 950, fy + 52 + 114 + 26, Modifiers::NONE);
    s.save("options-gameplay");
    let set: Vec<bool> = s
        .sent
        .iter()
        .filter_map(|c| match c {
            SessionCommand::SetCharacterOption { value, .. } => Some(*value),
            _ => None,
        })
        .collect();
    assert_eq!(set.len(), 1, "{:?}", s.sent);
    // The other sections.
    for (k, name) in [
        (3, "options-chat"),
        (4, "options-controls"),
        (5, "options-graphics"),
        (6, "options-sound"),
    ] {
        s.click(0, fx + 60, nav(k), Modifiers::NONE);
        s.save(name);
    }
    // Graphics' resolution menu.
    s.click(0, fx + 60, nav(5), Modifiers::NONE);
    s.click(0, fx + 900, fy + 52 + 114 + 26, Modifiers::NONE);
    s.save("options-menu");
    s.click(0, fx + 500, fy + 300, Modifiers::NONE);
    // The search.
    let field = shown_in::<ac_ui::widgets::Field>(&s, frame).expect("the search field");
    s.ui.host_mut()
        .root
        .with::<ac_ui::widgets::Field, _>(field, |f, _| f.set_text("run"));
    s.step();
    s.save("options-search");
    // Keys: the movement keys, a new key caught in place, then one the
    // game uses for something else.
    s.ui.host_mut()
        .root
        .with::<ac_ui::widgets::Field, _>(field, |f, _| f.set_text("movement keys"));
    s.step();
    let _ = s.ui.take_keyboard_requests();
    s.click(0, fx + 690, fy + 52 + 114 + 20, Modifiers::NONE);
    assert_eq!(
        s.ui.take_keyboard_requests(),
        [ac_render::ui::keyboard_window::KeyboardRequest::BeginCapture]
    );
    s.save("options-key-catch");
    let f13 = ac_ui::input::KeyChord::key(Key::F13, Modifiers::NONE);
    assert!(!s.ui.keyboard_captured(Some(f13)));
    s.step();
    s.step();
    let changed = |s: &Shot| {
        s.ui.host()
            .root
            .widget::<ac_ui_window::ui::modern::options::window::OptionsFrame>(frame)
            .map(|f| (f.keys_changed, f.clash.is_some()))
    };
    assert_eq!(changed(&s), Some((1, false)));
    s.click(0, fx + 690 + 114, fy + 52 + 114 + 20, Modifiers::NONE);
    let _ = s.ui.take_keyboard_requests();
    let w = ac_ui::input::KeyChord::key(Key::S, Modifiers::NONE);
    assert!(!s.ui.keyboard_captured(Some(w)));
    s.step();
    assert_eq!(changed(&s), Some((1, true)), "S is in use: it asks");
    s.save("options-key-clash");
    // Closing without Apply puts the change back.
    s.sent.clear();
    s.click(0, fx + 1000 - 12 - 15, fy + 26, Modifiers::NONE);
    s.step();
    assert!(!s.ui.host().root.is_window_visible("modern-options"));
    let back: Vec<bool> = s
        .sent
        .iter()
        .filter_map(|c| match c {
            SessionCommand::SetCharacterOption { value, .. } => Some(*value),
            _ => None,
        })
        .collect();
    assert_eq!(back, vec![!set[0]], "{:?}", s.sent);
    assert_eq!(changed(&s), Some((0, false)), "the keys are put back too");
}

/// Sets the session's open questions to `questions`.
fn ask(s: &mut Shot, questions: Vec<ac_runtime::communication::ClientConfirmation>) {
    s.scene
        .apply(PresentationEvent::Interface(InterfaceEvent::State(
            Box::new(InterfaceState {
                player: PLAYER,
                can_make_request: true,
                confirmations: questions,
                ..InterfaceState::default()
            }),
        )));
    s.step();
    s.step();
}

#[test]
#[ignore = "needs the retail dat files; writes pictures"]
fn the_modern_questions() {
    use ac_core::combat::CombatMode;
    use ac_runtime::communication::ClientConfirmation;
    use ac_runtime::session::SessionCommand;
    use ac_ui_window::ui::modern::dialogs::card::QuestionCard;
    use ac_ui_window::ui::modern::dialogs::invites::InviteCards;
    let mut s = Shot::new(CombatMode::NON_COMBAT);
    let craft = ClientConfirmation::Server {
        confirmation_type: 5,
        context_id: 11,
        message: "You have a 33% chance of successfully applying Granite to the Mithril Tachi. If you fail, the Mithril Tachi will be destroyed.".to_owned(),
    };
    let arena = ClientConfirmation::Server {
        confirmation_type: 7,
        context_id: 12,
        message: "Do you wish to enter the arena? You cannot leave until the fight is over."
            .to_owned(),
    };
    let fellow = ClientConfirmation::Server {
        confirmation_type: 4,
        context_id: 13,
        message: "Mira".to_owned(),
    };
    ask(&mut s, vec![craft.clone(), arena.clone(), fellow.clone()]);
    let card = shown::<QuestionCard>(&s).expect("the card shows");
    let of =
        s.ui.host()
            .root
            .widget::<QuestionCard>(card)
            .and_then(|c| c.shown.as_ref().map(|v| (v.of, v.look.yes.clone())));
    assert_eq!(
        of,
        Some((2, "Apply".to_owned())),
        "the invitation waits apart"
    );
    assert_eq!(s.ui.host().root.modal, Some(card));
    let invites = shown::<InviteCards>(&s).expect("the invitation shows");
    s.save("questions-craft");
    // Enter applies; the next question shows.
    s.sent.clear();
    s.key(Key::Enter, Modifiers::NONE, true);
    s.key(Key::Enter, Modifiers::NONE, false);
    let answered: Vec<(ClientConfirmation, bool)> = s
        .sent
        .iter()
        .filter_map(|c| match c {
            SessionCommand::AnswerConfirmation {
                confirmation,
                accepted,
            } => Some((confirmation.clone(), *accepted)),
            _ => None,
        })
        .collect();
    assert_eq!(answered, vec![(craft.clone(), true)], "{:?}", s.sent);
    ask(&mut s, vec![arena.clone(), fellow.clone()]);
    s.save("questions-yes-no");
    // Escape says no.
    s.sent.clear();
    s.key(Key::Escape, Modifiers::NONE, true);
    s.key(Key::Escape, Modifiers::NONE, false);
    assert!(
        s.sent.iter().any(|c| matches!(c,
            SessionCommand::AnswerConfirmation { accepted: false, confirmation } if *confirmation == arena)),
        "{:?}",
        s.sent
    );
    ask(&mut s, vec![fellow.clone()]);
    assert!(
        shown::<QuestionCard>(&s).is_none(),
        "nothing left in the middle"
    );
    assert_eq!(s.ui.host().root.modal, None);
    s.save("questions-invite");
    // Join on the invitation.
    let (ix, iy, iw, ih) = rect_of(&s, invites);
    s.sent.clear();
    s.click(0, ix + iw - 14 - 20, iy + (ih - 28) / 2, Modifiers::NONE);
    assert!(
        s.sent.iter().any(|c| matches!(c,
            SessionCommand::AnswerConfirmation { accepted: true, confirmation } if *confirmation == fellow)),
        "{:?}",
        s.sent
    );
    // A training question from a window: its own words on the buttons.
    ask(&mut s, vec![ClientConfirmation::Suicide]);
    s.save("questions-danger");
}

/// A shop open at Ilsa's, with five listings and one row on each list.
fn open_shop(s: &mut Shot, reason: ac_runtime::inventory::VendorViewReason) {
    use ac_core::items::vendor::{NO_LIMIT, UNLIMITED_STOCK};
    use ac_core::items::{VendorShopItem, VendorShopProfile};
    use ac_runtime::inventory::VendorView;
    use ac_runtime::inventory::shop::{StagedRow, VendorListsView};
    let listing =
        |guid: u32, name: &str, ty: ItemType, icon: u32, value: i32, stock: i32| VendorShopItem {
            item_guid: guid,
            stack_size: stock,
            name: Some(name.to_owned()),
            item_type: Some(ty.bits()),
            icon_id: icon,
            value: Some(value),
            ..VendorShopItem::default()
        };
    let view = VendorView {
        vendor_guid: 0x7000_0901,
        name: Some("Ilsa Brenner".to_owned()),
        profile: VendorShopProfile {
            merchandise_item_types: (ItemType::FOOD | ItemType::MISC | ItemType::GEM).bits(),
            merchandise_min_value: NO_LIMIT,
            merchandise_max_value: 10_000,
            deal_magical_items: false,
            buy_price: 0.5,
            sell_price: 1.2,
            ..VendorShopProfile::default()
        },
        items: vec![
            listing(
                0x7000_0911,
                "Adept Healing Kit",
                ItemType::MISC,
                0x0600_1E3B,
                600,
                UNLIMITED_STOCK,
            ),
            listing(
                0x7000_0912,
                "Healing Kit",
                ItemType::MISC,
                0x0600_1E3B,
                95,
                UNLIMITED_STOCK,
            ),
            listing(
                0x7000_0913,
                "Health Potion",
                ItemType::FOOD,
                0x0600_1A8B,
                40,
                UNLIMITED_STOCK,
            ),
            listing(
                0x7000_0914,
                "Mana Potion",
                ItemType::FOOD,
                0x0600_1A8D,
                40,
                UNLIMITED_STOCK,
            ),
            listing(0x7000_0915, "Amethyst", ItemType::GEM, 0x0600_1A8C, 250, 3),
        ],
        lists: VendorListsView {
            buying: vec![StagedRow {
                item_guid: 0x7000_0913,
                name: "Health Potion".to_owned(),
                quantity: 5,
                price: 240,
            }],
            selling: vec![StagedRow {
                item_guid: 0x8000_0104,
                name: "Healing Kit".to_owned(),
                quantity: 1,
                price: 60,
            }],
            money: 52_400,
            ..VendorListsView::default()
        },
        reason,
    };
    s.scene
        .apply(PresentationEvent::Interface(InterfaceEvent::Vendor(Some(
            Box::new(view),
        ))));
    s.step();
    s.step();
}

#[test]
#[ignore = "needs the retail dat files; writes pictures"]
fn the_modern_vendor() {
    use ac_core::combat::CombatMode;
    use ac_runtime::inventory::{ItemCommand, VendorCommand, VendorViewReason};
    use ac_runtime::session::SessionCommand;
    let mut s = Shot::new(CombatMode::NON_COMBAT);
    more_things(&mut s);
    open_shop(&mut s, VendorViewReason::Opened);
    assert!(
        s.ui.host().root.is_window_visible("modern-vendor"),
        "it opens with the shop"
    );
    assert!(
        !s.ui.host().root.is_window_visible("vendor"),
        "retail's stays hidden"
    );
    s.save("vendor-buy");
    let frame = s.ui.host().root.window("modern-vendor").expect("it").frame;
    let (fx, fy, _, _) = rect_of(&s, frame);
    // The second row (Healing Kit), then Buy now.
    s.click(0, fx + 216 + 200, fy + 72 + 36 + 48 + 20, Modifiers::NONE);
    s.sent.clear();
    s.click(
        0,
        fx + 672 + 14 + 40,
        fy + 72 + 214 - 14 - 16,
        Modifiers::NONE,
    );
    assert!(
        s.sent
            .iter()
            .any(|c| matches!(c, SessionCommand::Item(ItemCommand::Buy(rows)) if rows.len() == 1)),
        "{:?}",
        s.sent
    );
    // The Sell tab.
    s.click(0, fx + 262, fy + 30, Modifiers::NONE);
    s.save("vendor-sell");
    // Sell all.
    s.sent.clear();
    s.click(0, fx + 672 + 100, fy + 72 + 552 - 14 - 17, Modifiers::NONE);
    s.save("vendor-sell-after");
    let _ = s.sent.iter().any(|c| {
        matches!(
            c,
            SessionCommand::Item(ItemCommand::Vendor(VendorCommand::SellAll))
        )
    });
    // Close with lists waiting: it asks.
    s.click(0, fx + 1040 - 16 - 15, fy + 28, Modifiers::NONE);
    s.save("vendor-leave");
}

/// A trade with Osk: the player offers two things, Osk one (and a second
/// when `more`), with the acceptances given.
fn trade_with_osk(s: &mut Shot, more: bool, mine: bool, theirs: bool) {
    use ac_runtime::inventory::TradeView;
    let mut partner = vec![0x8000_0301];
    if more {
        partner.push(0x8000_0302);
    }
    let view = TradeView {
        open: true,
        partner_guid: 0x5000_0099,
        partner_name: Some("Osk".to_owned()),
        self_accepted: mine,
        partner_accepted: theirs,
        self_items: vec![0x8000_0101, 0x8000_0104],
        partner_items: partner,
        revision: 1,
        ..TradeView::default()
    };
    s.scene
        .apply(PresentationEvent::Interface(InterfaceEvent::Trade(
            Box::new(view),
        )));
    s.step();
    s.step();
}

#[test]
#[ignore = "needs the retail dat files; writes pictures"]
fn the_modern_trade() {
    use ac_core::combat::CombatMode;
    use ac_runtime::inventory::ItemCommand;
    use ac_runtime::session::SessionCommand;
    let mut s = Shot::new(CombatMode::NON_COMBAT);
    more_things(&mut s);
    trade_with_osk(&mut s, false, false, true);
    assert!(
        s.ui.host().root.is_window_visible("modern-trade"),
        "it opens with the trade"
    );
    assert!(
        !s.ui.host().root.is_window_visible("secure-trade"),
        "retail's stays hidden"
    );
    s.save("trade-they-accepted");
    let frame = s.ui.host().root.window("modern-trade").expect("it").frame;
    let (fx, fy, _, _) = rect_of(&s, frame);
    // Accept.
    s.sent.clear();
    s.click(0, fx + 900 - 20 - 60, fy + 560 - 36, Modifiers::NONE);
    assert!(
        s.sent
            .iter()
            .any(|c| matches!(c, SessionCommand::Item(ItemCommand::AcceptTrade))),
        "{:?}",
        s.sent
    );
    trade_with_osk(&mut s, false, true, false);
    s.save("trade-you-accepted");
    // Osk adds something: the acceptance is taken back and it is new.
    trade_with_osk(&mut s, true, false, false);
    s.save("trade-changed");
    let banner =
        s.ui.host()
            .root
            .widget::<ac_ui_window::ui::modern::dealing::trade_window::TradeFrame>(frame)
            .and_then(|f| f.screen.banner.clone());
    assert!(banner.is_some_and(|b| !b.bad), "the change is told");
    // Add from packs.
    s.click(
        0,
        fx + 20 + 209,
        fy + 120 + 48 + 8 + 2 * 48 + 4 + 64 - 8 - 14,
        Modifiers::NONE,
    );
    s.save("trade-pick");
    // A click on the other side's item shows its card.
    s.click(0, fx + 500, fy + 300, Modifiers::NONE);
    s.click(0, fx + 462 + 200, fy + 120 + 48 + 8 + 20, Modifiers::NONE);
    s.save("trade-card");
}

/// An Ust and four things to salvage in the main pack.
fn salvage_things(s: &mut Shot) -> u32 {
    const UST: u32 = 0x8000_0501;
    let mut objects = Vec::new();
    let mut p = ClientObject::new(PLAYER);
    p.name = "Aerin Vale".to_owned();
    p.item_type = ItemType::CREATURE;
    p.items_capacity = 102;
    p.containers_capacity = 7;
    objects.push(p);
    let mut ust = ClientObject::new(UST);
    ust.name = "Ust".to_owned();
    ust.item_type = ItemType::TINKERING_TOOL;
    ust.icon_id = 0x0600_1E3B;
    ust.container_id = PLAYER;
    objects.push(ust);
    let mut main = vec![UST];
    for (i, (name, material, ws, icon)) in [
        ("Steel Longsword", 64, 6.0, 0x0600_11CB),
        ("Steel Kite Shield", 64, 5.0, 0x0600_11CB),
        ("Granite Mace", 67, 4.0, 0x0600_11CB),
        ("Silk Robe", 6, 5.0, 0x0600_1E3B),
    ]
    .into_iter()
    .enumerate()
    {
        let guid = 0x8000_0510 + i as u32;
        let mut o = ClientObject::new(guid);
        o.name = name.to_owned();
        o.item_type = ItemType::MELEE_WEAPON;
        o.icon_id = icon;
        o.material_type = Some(material);
        o.workmanship = ws;
        o.value = 1_240 - 100 * i as i32;
        o.container_id = PLAYER;
        objects.push(o);
        main.push(guid);
    }
    s.scene
        .apply(PresentationEvent::Interface(InterfaceEvent::Items(
            ItemsUpdate {
                reset: true,
                objects,
                removed: Vec::new(),
                contents: vec![(PLAYER, Some(main))],
            },
        )));
    s.step();
    s.step();
    UST
}

#[test]
#[ignore = "needs the retail dat files; writes pictures"]
fn the_modern_salvage() {
    use ac_core::combat::CombatMode;
    use ac_runtime::inventory::ItemCommand;
    use ac_runtime::session::SessionCommand;
    let mut s = Shot::new(CombatMode::NON_COMBAT);
    let ust = salvage_things(&mut s);
    s.scene
        .apply(PresentationEvent::Interface(InterfaceEvent::OpenSalvage(
            ust,
        )));
    s.step();
    s.step();
    assert!(
        s.ui.host().root.is_window_visible("modern-salvage"),
        "it opens on the tool"
    );
    assert!(
        !s.ui.host().root.is_window_visible("salvage"),
        "retail's stays hidden"
    );
    s.save("salvage-empty");
    let frame = s.ui.host().root.window("modern-salvage").expect("it").frame;
    let (fx, fy, _, _) = rect_of(&s, frame);
    // Add from packs, then the first three.
    s.click(0, fx + 20 + 470 - 70, fy + 68 + 14, Modifiers::NONE);
    s.save("salvage-pick");
    for _ in 0..3 {
        // The first row not yet added sits lower each time.
        let pick =
            s.ui.host()
                .root
                .widget::<ac_ui_window::ui::modern::dealing::salvage_window::SalvageFrame>(frame)
                .map(|f| f.pick.iter().position(|p| !p.off).unwrap_or(0))
                .unwrap_or(0) as i32;
        s.click(
            0,
            fx + 110 + 400 - 12 - 6 - 32,
            fy + 100 + 44 + 24 + 10 + pick * 42 + 20,
            Modifiers::NONE,
        );
    }
    s.click(0, fx + 110 + 400 - 24, fy + 100 + 22, Modifiers::NONE);
    s.save("salvage-staged");
    // Salvage.
    s.sent.clear();
    s.click(0, fx + 820 - 20 - 95, fy + 540 - 36, Modifiers::NONE);
    let salvaged = s.sent.iter().find_map(|c| match c {
        SessionCommand::Item(ItemCommand::Salvage { items, .. }) => Some(items.len()),
        _ => None,
    });
    // One material at a time (the options' default): the mace was refused.
    assert_eq!(salvaged, Some(2), "{:?}", s.sent);
    s.scene
        .apply(PresentationEvent::Interface(InterfaceEvent::Salvaged(
            Box::new(ac_core::items::SalvageOutcome {
                bags: vec![ac_core::items::SalvageBag {
                    material_type: 64,
                    workmanship: 5.5,
                    units: 64,
                }],
                unsuitable: Vec::new(),
            }),
        )));
    s.step();
    s.step();
    s.save("salvage-done");
}
