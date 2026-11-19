# Modern UI: build plan

Draft, 2026-11-19. The design is in `README.md` and on the canvas; this
file is how we build it. M0 was built on 2026-11-19 (see "Progress").

## What the code gives us

Read from the tree on 2026-11-19 (paths relative to the repo root).

- **Game model and actions are already free of the retail UI.** The
  session streams `PresentationEvent` / `InterfaceEvent`
  (`crates/ac-runtime/src/session/commands.rs`, `session/interface.rs`)
  and takes `SessionCommand` back. Panels never change state; they emit
  commands. A second interface reads the same stream and sends the same
  commands.
- **The toolkit under the retail panels is not tied to dats.** `UiRoot`,
  `Element`, `Widget`, the window manager, focus, popups, modals, drag
  and drop (to and from the world), layout persistence and the
  `DrawList` → `UiBatch` → GPU path all work without a `LayoutDesc`.
  The plugin dock is already a hand-built native UI on it, drawn with
  `crates/ac-ui/src/plugin/style.rs` (rounded surfaces, gradients, soft
  shadows, switches, sliders). The modern UI is built the same way.
- **What is tied to retail:** the panel controllers (bound to about 374
  dat element ids), the view-model builders that live inside them
  (`ItemsView`, `MagicView`, `CharacterSheet`, `EffectsView`,
  `ChatFeed`, inventory drop rules, the confirm queue, the split amount
  held by the toolbar), and `WindowUi`, which the hosts
  (`crates/ac-render/src/surface.rs`, `crates/ac-ui-web/src/client.rs`)
  call through about 55 concrete methods.
- **The web client runs the same `WindowUi`**, so anything built inside
  it works in the browser too.

## Progress

**M0 built 2026-11-19** on branch `feat/modern-ui-m0` (off upstream-main
0.2.15), one commit per piece:

1. Launcher: Settings › Interface (Retail / Modern), saved in
   `launcher-interface.json` (its own file, so an older launcher still
   reads the profiles); every client started gets `OPENAC_INTERFACE`.
2. Client: reads it once; Modern mounts `ui/modern` (`ModernLayer`): the
   replaced-windows list, kept hidden before each frame is drawn, and
   left alone by the layout persistence (`set_left_alone`), so the retail
   layout survives; modern default places (`modern/places.rs`).
3. `ac_ui::modern`: Horizon palette, sizes and text sizes; `Tween`;
   `numbers::compact`.
4. Drawing: `stroke_arc` (soft sides and ends), `textured_triangles`,
   `fill_textured_disc` (`DiscUv`: centre, scale, turn), `ShapeCache`.
5. Fonts: Inter 4.1 Regular, Medium, SemiBold (`assets/fonts/Inter`),
   kerned (whole pixels) with even digits; Noto unchanged. Bakes up to
   48 px (1024 pages above 32) so modern text is sharp at 2x; the plugin
   themes keep their 32 px sharp limit. `WindowUi::modern_font(weight,
   em)` bakes on first use and queues the sharp bake.
6. `ac_ui::modern::flex::ElementFlex` (FlexTree over elements, `Measure`:
   `TextLine`, `Fixed`) and `hover::HoverPane`.
7. Quick move in `UiRoot`: `Element::quick_move`, `set_quick_move_key`
   (Ctrl for now), drag whatever the lock, snap 8 units to screen and
   piece edges (Shift: free), `UiOutput::QuickMoveMenu` on right-click
   (the menu itself comes with Edit layout in M2); dashed outlines and
   names drawn by `modern::quick_move::QuickMoveOutlines`.

Mac text keys went on their own branch `feat/mac-text-keys` (Cmd with
A/C/X/V/Z, Shift+Z or Y redo, undo history, Option/Ctrl word moves and
deletes, Cmd line moves), as a small PR of its own.

Toolkit pieces that know nothing of the game went into `ac-ui`
(`ac_ui::modern`), not `ac-ui-window`; the screens still go in
`ac-ui-window/src/ui/modern/`.

Still open after M0: the quick-move Mac key; Erik and one PR per
milestone; a live look at both interfaces (the first live run's window
was covered, so nothing was drawn).

## Shape of the build

**One window, retail parts swapped out one at a time.** With Modern on,
`WindowUi` mounts a modern layer and hides each retail window the modern
layer has replaced. Screens not built yet keep their retail window. The
game is playable on Modern from the first milestone, every step can be
live-tested, and Retail is untouched.

Rules that keep this from growing the god object:

- Modern code lives in its own module tree (`crates/ac-ui-window/src/modern/`,
  moving to its own crate if compile times ask for it), never inside
  retail panel files.
- When a modern screen needs logic that sits in a retail panel (a view
  model, drop rules, a queue), that logic moves out to a neutral module
  first, the retail panel is switched to it with no change in
  behaviour, and then the modern screen uses it. One commit each.
- The modern layer registers which retail windows it replaces; hiding
  is one list, not checks scattered through retail code.
- Modern windows get their own names, so `LayoutPersistence` keeps each
  interface's layout apart without new code; modern default positions
  are a separate table.

When every retail window is replaced (milestone 10), the modern layer
stops mounting retail panels at all and the host-facing methods are
pulled into a trait both interfaces implement.

**The switch:** one launcher-wide setting (agreed 2026-11-19). The launcher
passes it to the client at start (the process spec already carries
environment, `crates/ac-launcher-core/src/launching/process.rs`); the
client reads it once. No in-game row, no hot swap, Retail by default.

## Milestones

Each ends with a live test on the Mac and is a candidate upstream PR
(see "Upstream" below). The canvas boards named are the spec; read
the `.dc.html` copy before building each.

### M0 Groundwork

- Launcher: the Modern / Retail setting, saved, passed at launch.
- Client: read it; mount an empty modern layer; the replaced-windows
  list; modern default positions table.
- Horizon theme: palette and metrics beside `ThemePalette` (neutral
  glass, bronze trim, the canvas's colours and radii).
- Inter (OFL) bundled in Regular, Medium and SemiBold; more baked
  sizes; kerning in `UiFont` (port from `ac-plugin-ui/src/fonts.rs`);
  digits drawn at one fixed advance so numbers do not jitter (fontdue
  has no `tnum`).
- Drawing: arc and ring shapes; textured triangles so the radar and
  map can be clipped to a circle; cache tessellated shapes that do not
  change.
- Mac keys: Cmd for copy, paste, cut, select all and undo in text
  fields on macOS, plus Option/Cmd arrow and backspace moves. Touches
  the shared text field, so retail gets it too; could go up as its own
  small PR first.
- Small helpers: an eased-value tween, compact numbers (1.2B) next to
  `group_thousands`, a native flex adapter (a measure trait over
  `FlexTree`, not tied to markup), a themed hover pane that can hold
  any widget.

- Quick move in `UiRoot`: while the quick-move key is held, any modern
  piece or window can be dragged (outlines, names, snapping) whatever
  the lock says; Ctrl + right-click opens the piece's menu. Built here
  so every later piece gets it for free.

Done when: Retail launches exactly as before; Modern launches into the
retail UI with an empty modern layer.

### M1 The HUD ("Round 2: Horizon", "Combat and vitals", "Effects on you", "Small pieces")

- Vitals top centre with the damage and heal motion, numbers over 5%,
  low-health pulse and screen-edge glow, not-enough-mana shake, stamina
  blink.
- Target line: name, level, health (or an owned item's mana), Use
  (named Talk / Open / Loot) and Inspect, the status line that replaces
  the yellow spew box.
- Effects counts top left and their hover pane; burden and chess chips;
  frame rate.
- Radar B, the bearing ring (page "Radar and recalls"), with the
  connection status line and card, and the Recall button and list (the
  recall requests already exist in the client; custom recalls per
  server saved on this computer).
- Combat strip and the jump meter.
- Panel strip bottom right (opens the retail windows for now), drop on
  Inventory puts in the main pack.
- Options › Interface › Vitals and effects rows.

Replaces: vitals, radar, selected object, indicators, combat bar, jump
bar, spew box, fps, the toolbar's panel buttons.

### M2 Your bars ("Your bars")

- A client-side store for bars (settings.json, per character or
  shared, copy from another character). The server's shortcut list
  stays retail's; the first time a character uses Modern its bars are
  seeded from it.
- Item and spell slots on the same bars, Magic-mode keys, greyed spells
  outside Magic, drawn cooldown sweep.
- Edit layout mode: drag by tab, turn, add or remove slots, per-slot
  keys with clash checks, new bar, reset; also moves the quests panel
  and fellows later.

Replaces: toolbar slots, the eight spell bars.

### M3 Chat ("Chat")

Tabs with unread counts, idle fade, entry with the channel button and
menu, @ suggestions, name menu, read-back hold, search, mentions,
popped-out windows with their own filter and opacity.

Replaces: chat, floating chat, transcript.

### M4 Items ("Inventory", "Inspect", "Loot, barber, chess")

- The modern item cell (drawn, larger icons, cooldown, selection),
  a virtualised grid and list.
- Inventory with equipment figure (viewport looked up by element, not
  dat id; turnable), packs, search, sort, Tidy, item details with the
  four tabs, pop-up card on small screens.
- Inspect sheet for every kind; Loot window.

Replaces: inventory, paperdoll, external container, appraisal.

### M5 Character and magic ("Spellbook", "Character", "Effects on you")

Spellbook and Components; character sheet with Titles and Information;
the Effects window.

Replaces: magic, spell components, character, character info, titles.

### M6 Options ("Options")

The eight sections with search, built from the retail options spec
(`panels/options/spec.rs` already types every row), chat filter grid,
key bindings with clash checks, OPENAC tags, Character select and Exit.

Replaces: options, keyboard config.

### M7 Dealing ("Vendor", "Trade", "Salvage", "Dialogs")

Vendor, Trade, Salvage; the one dialog card with its queue; invitation
cards under the target.

Replaces: vendor, trade, salvage, dialog, dialog factory.

### M8 Social and world ("Social", "Journal", "Map", "House", "Dialogs")

Social window; Journal (quests, notes, quests on screen) and the book
reader; full-screen Map with pins, markers and houses for sale; House
window; house sale window.

Replaces: social, journal, contracts, book, map/house, house purchase.

### M9 The rest of the game ("Loot, barber, chess", "Small pieces")

Barber, Chess, Credits.

### M10 Pre-game ("Login", "Character creation")

Connecting screen (portal ring) and Stage character select, with the
per-computer character memory; Studio character creation.

After this nothing retail is mounted on Modern: pull the host-facing
methods into a trait, check the web client, remove the hiding list.

### Plugin hooks (alongside M4 and M6)

A new plugin API version: the client asks plugins about an item (when
it is inspected, shown in a loot window, inventory, trade or sell list)
and draws what they return (a verdict, a few lines, menu entries, foot
buttons); Options › Plugins hosts each plugin's markup page. The item
hooks land with M4, the Options pages with M6. Retail never calls them;
the plugin API is shared, so retail could show the same answers later.

### Later, not blocking

- Backdrop blur behind glass panels (needs the renderer to copy and blur
  the scene for the UI pass). Translucent fills stand in until then.
- Fewer texture switches between shapes and text in dense windows.

## Upstream

Erik's rule is one PR for work that builds on itself. Here each
milestone stands on its own once M0 is in: it only adds modern screens
behind a switch that is off by default. Plan: ask Erik before M0 whether
he takes the modern UI as one PR per milestone. The usual rules hold
(no client-internals names in comments, `scripts/check.sh` green, the
PR says what retail does).

## Open points

- Quick move's default key on a Mac (Ctrl + click is often right-click
  there); decide during M0.
- Server level cap: XP bar says "Max level" or keeps counting
  (from the design).
- Officers and bans in Manage allegiance: revisit at M8 (from the
  design).
