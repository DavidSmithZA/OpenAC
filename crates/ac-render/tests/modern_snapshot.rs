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
        let _ = self.ui.drain();
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
