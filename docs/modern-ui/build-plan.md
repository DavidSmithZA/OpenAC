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

**M1 built 2026-11-19** on `feat/modern-ui-m1` (off `feat/modern-ui-m0`):
vitals, target line and status messages, effects counts and chips,
frame rate, radar B with the connection line and card, Recall list and
own recalls, combat strip and jump meter, panel strip, Options rows.
The retail indicator bar stays (its end-session button) until M6.

**M2 built 2026-11-20** on `feat/modern-ui-m2` (off `feat/modern-ui-m1`):

1. Settings section `modernBars` (`ac_host::settings::SavedBars`): a
   `shared` layout and per character (`world/name`) `useShared` and
   `own`. Each bar: name, anchor + offset, upright, locked, hidden,
   slots (`item`/`spell` + id + name, `key` and `magicKey` as
   `Shift+Number1`).
2. `ui/modern/bars/model.rs`: the layout. Keys per slot for
   Peace/Melee/Missile and for Magic; a taken key swaps; slots swap
   content and keep keys; a place is held to the screen third the bar
   sits in. First layout from the retail lists: Main (1 to =, Shift+1 to
   Shift+= in Magic) with the toolbar's numbered row, Spells (1 to = in
   Magic) with favourite bar 1, and the toolbar's second row and other
   favourite bars that held anything as upright bars down the right
   edge.
3. `bars/widgets.rs`, `bars/edit.rs`: slot (icon, key with drawn
   modifier marks since Inter's bake has no Shift or Ctrl symbols,
   count, spell dot, cooldown sweep, greyed spells outside Magic, drops
   of items, spells and other slots), band, Edit layout tab, toolbar,
   key pop-up, menu, rename box, spell-drag hint.
4. `bars/run.rs`: load / seed (waits up to 4 s for the retail lists) /
   save 0.5 s after a change; build, place (a moved bar takes a new
   place), feed; keys before the key map (a slot's key in the mode in
   use); click uses an item (or offers it as a target) and casts a
   spell only in Magic; right-click inspects (an empty slot opens its
   bar's menu); a slot dragged off the bars comes off; New shortcut and
   the spellbook's double-click put things on the first free slot; the
   retail toolbar and combat window are replaced.
5. Edit layout opens from quick move's menu (Ctrl + right-click a piece)
   or an empty slot's right-click. Quick move's menu for other pieces:
   back to the default place, Edit layout.
6. A spell dragged out of the spellbook fades it to 12% and lifts the
   bars over it.
7. Fixed on the way (M1 bug): pieces that do not clip answered every hit
   test, so the panel strip took the pointer anywhere on the screen and
   the combat strip, radar, target and effects did so while quick move
   was held.
8. `crates/ac-render/tests/modern_snapshot.rs` (ignored): pictures of the
   modern UI with no window (real dats, made-up player, real input, the
   web tests' CPU rasteriser). Used to check M2 while the Mac was
   locked.

Gaps closed after M2 (2026-11-20, same branch):

- The split amount moved to the target line: a selected stack of more
  than one shows Split, a track, the typed amount and "of N"; the wheel
  steps it (Shift: ten), the split key types into it. It sets the amount
  the hidden retail split controller keeps, which drags already use.
- Lock in place and Hide in quick move's menu for every HUD piece, saved
  in settings section `modernPieces` (this computer). Edit layout's
  toolbar has Hidden and locked to show or unlock them.
- Recall rows drag onto bars: places and own recalls become `recall`
  slots (name, command, icon; the command is sent as if typed); recall
  spells go on as spells.

Still not done: the Options › Controls row for the quick-move key (M6);
moving the quests panel and fellows in Edit layout (they come in M8).

**M3 built 2026-11-20** on `feat/modern-ui-m3` (off `feat/modern-ui-m2`),
one or more commits per step:

1. Model (`ui/modern/chat/model.rs`): line kind (tag, colour), speaker
   and words, mentions; tabs as text-class filters with a talk channel,
   an id, a window flag, opacity and place; unread counts; the message
   type groups the tab dialog toggles.
2. Panel (`chat/panel.rs`): tabs with unread badges (they give up
   padding so +, pop-out and search fit), wrapped lines, idle fade with
   "Enter to chat", read-back hold with "N new messages"; window mode
   for popped-out tabs.
3. Entry (`chat/run.rs`, `chat/channels.rs`): window `modern-chat` in
   place of retail's chat window; the field is wired to the session's
   chat entry exactly as retail's (a `PanelController` on the window);
   channel button and two-column menu with @ commands and recent tells.
   Retail's port greys every channel but Say, General, Trade and LFG;
   retail itself keeps a mask of usable talk channels that our runtime
   does not track, so the modern menu greys only what the client knows
   (no fellowship, no allegiance; Olthoi always) and leaves the rest to
   the server. The chat keys go to the modern entry.
4. @ suggestions (`chat/commands.rs`; Tab or a click completes; the
   button says where a command sends the line, from the runtime's parser,
   now exported), the name menu (`chat/names.rs`), search.
5. New tab / change tab dialog (`chat/tab_dialog.rs`), pop-out windows
   with their own entry (straight to the session on the tab's channel),
   saved in settings section `modernChat`; they stand in for the four
   floating chat windows, whose keys show or hide the first four.

Checked offscreen (`the_modern_chat` in `modern_snapshot.rs`): idle,
active with typing, Enter sends through the chat entry, channel menu,
suggestions, name menu, search, the dialog, a popped-out window.

Not done in M3: arrows to choose a suggestion (the field keeps Up and
Down for sent lines); dragging a tab off to pop it out (button and
dialog instead); coordinate links in plugin lines; the chat settings
button (Options › Chat is M6); the retail chat font option (the modern
chat uses Inter); allegiance patron and vassals greying (the client
cannot tell them apart from the allegiance's records yet).

**Live check 2026-11-20** (M1 to M3; everything else worked). Fixed on
`feat/modern-ui-m3` (edcb5ecd, 012988ba, 5a964043):

- The combat mode button is the combat toggle (Peace and combat; what is
  wielded picks Melee, Missile or Magic), not a menu of four modes.
- The idle chat comes up when the pointer rests on it (0.2 s).
- Tell to the selected thing (retail's Tell to Selected, by id, so NPCs
  answer) in the channel menu, for anything selected, not only players;
  the name menu's Tell aims at a creature or NPC by id.
- A popped-out tab's close button puts it back in the panel; a full tab
  row cuts the longest names short.
- Bar spells in Peace with a caster in hand are live: a click goes to
  Magic and casts there, through the runtime's `CastInMagic` (bfb3aca0),
  which waits for the server's Magic and the magic stance with no motion
  queued (sent at once, the server refused it as too busy). A spell that
  needs a target is greyed unless the selection can take it (the spell
  bars' cast gate).
- Second round (87f11d7a, 48945f3d, ff24354b): long names in the channel
  menu's tell rows are cut short before their note; a tell sent reads
  "YOU TELL Name: words" (the server's sentence is split); a clicked name
  with no id is looked up among the objects nearby, so the name menu's
  Tell and Select reach an NPC.

**M1, M2 and M3 accepted live 2026-11-20.** Next: M4 (items).

**M4 started 2026-11-20** on `feat/modern-ui-m4` (off `feat/modern-ui-m3`).
Retail windows it replaces: `inventory` (with the paperdoll),
`external-container` (loot) and `examination` (Inspect). Steps, one or
more commits each:

1. Neutral first: the item report in named parts (retail's text,
   unchanged, still one report for retail), so the modern tabs can sort
   the parts into Overview, Spells, Needs and Details; the viewer's
   appraisal names usable outside the retail window.
2. Inventory model (`ui/modern/items/model.rs`): packs and how full they
   are, the items in view (one pack, or all grouped by pack, by kind or
   not at all), filter chips, search over every pack, sort keys, the 24
   equipment slots, burden and pyreals.
3. The item cell and a virtualised grid and list (drawn icons, stack
   count, cooldown, selection; drag and drop with retail's payload).
4. Item details (head, facts, the four tabs, inscription), shared with
   Inspect.
5. The inventory window: equipment slots, pack rail, toolbar, grid,
   details with Equip or Use, Give, Drop, Salvage, More; footer with
   burden and pyreals. The inventory key, the panel strip and an opened
   pack show it.
6. Inspect sheet for every kind; it takes the appraisals before retail's
   examination code and follows the selection.
7. Loot window; it closes the container itself when out of range
   (retail's window does that only while it shows).
8. Sort menu and Tidy, the split popover, drag to equip (slots light
   up), the turning figure (its own viewport request), the pop-up card
   on small screens.
9. Plugin item hooks (a new API version), once the screens stand.

Built 2026-11-20 on `feat/modern-ui-m4` (steps 1 to 8 in part), each
checked offscreen (`the_modern_inventory`, `the_modern_inspect`,
`the_modern_loot` in `modern_snapshot.rs`; drags and loots assert the
commands sent):

- Neutral moves first: `item_report_in_parts` (b51dfc80), the pack drop
  rules out of the retail inventory into `ac_ui::panels::pack_drops`
  (ef565a89, retail's 96 item panel tests unchanged), `MainPanels::stand_in`
  so the inventory key, panel strip, burden chip and a pack used anywhere
  open the modern window (e67c73e3).
- `items/model.rs`, `cell.rs`, `grid.rs` (recycled cells), `sheet.rs` and
  `sheet_view.rs` (retail's report lines sorted into the four tabs),
  `menu.rs`, `inventory.rs` + `run.rs` (the window), `sheets.rs`,
  `inspect.rs`, `loot.rs`.
- Inventory: equipment slots (an item covering several shows in each;
  Aetheria by its quest), the figure (the paperdoll render takes a turn;
  drag turns it, gear dropped on it is worn), pack rail with fill and
  drops, Group/Sort/compact/list/chips, search over every pack, details
  asked about quietly (the answer kept; Inspect does not open),
  Equip/Take off/Use, Give (to the previous selection), Drop, Salvage,
  More (inscribe tab, add to a bar, link in chat), right-click menu,
  Tidy merges partial stacks one request at a time, burden and pyreals.
- Inspect: items (portals, chests, doors in one column), creatures,
  characters (Stats, Allegiance, Armor, About), spells from bars or the
  book; follows the selection; 0.75 s combat refresh; examine key closes.
- Loot: rows with Inspect and Loot, double-click, Loot into a pack,
  chest tabs and drop zone, burden now and with all, Loot all (OPENAC),
  closes the container itself out of reach.

Then (2026-11-20, while the M4 live check was open): writing an
inscription in the details and in Inspect (7e64014e; a field over the
writable box, retail's rule: an inscribable item you carry, unsigned or
signed by you; sent when the field is left), the split pop-up from the
right-click and More menus (ee868c56; track, count box, Enter splits into
a new stack just after it), drawn glyphs in empty equipment slots
(fa583cfd). Offscreen tests `the_modern_inscription`, `the_modern_split`.

**M4 accepted live 2026-11-20** (install 38de12d1: all of M4).

Step 9, plugin item hooks (2026-11-20; user chose plugins push, not the
client asking): API 1.15 (31d4bbe5) adds `ui_item_notes_set` (per item:
a mark in a tone, card lines, menu entries; up to 4,096 notes, replaced
whole) and `ui_window_foot_set` (a summary and up to 3 buttons for the
Inventory, Loot, Trade or Vendor window), with events `ItemMenuChosen`
and `FootButtonPressed`; notes go at logoff and unload. The modern
screens draw them (25e213ad, `items/plugin_notes.rs`): marks on cells and
rows, a card per plugin at the head of the details and Inspect, plugin
entries in the item menus, a band over the inventory grid's foot and the
loot window's foot. Trade and Vendor draw theirs with M7. Offscreen test
`the_modern_plugin_notes`. Not live-tested: needs a plugin built for 1.15.

Not done yet in M4: the details as a pop-up card on small screens
(Docked / Pop-up / Auto belongs with Options, M6); Close gaps (packs in
this game hold no gaps).

**M5 built 2026-11-20** on `feat/modern-ui-m5` (off `feat/modern-ui-m4`;
plugin checks for API 1.15 left for later). Retail windows it replaces:
`effects-positive`, `effects-negative`, `vitae`, `spellbook` (with its
components page), `character` (with its titles page) and
`character-information`. Neutral moves first, one commit each:

- Enchantment stat families gain the whole-number, float and natural
  armor bits; vitae's experience-to-recover in its own helper
  (af910199).
- The bars' cast logic (Magic at once, Peace with a caster through
  `CastInMagic`) and bar-spot lookups as helpers other windows use
  (e78f1dd1, a9eea573).
- The character sheet carries the most each row's unassigned experience
  buys (`max_raise`, for Max); attribute and vital descriptions public;
  the information report in six named parts, retail's text unchanged
  (a875cf6e, 4801f12e).
- `ui/modern/kit.rs`: what the windows draw alike (back, title, close,
  underlined tabs with counts, buttons, chips, search box and field,
  scroll, scrollbar, meter, wrap).

Screens, each checked offscreen (`the_modern_effects`,
`the_modern_spellbook`, `the_modern_character`):

1. Effects window (`magic/effects*.rs`, 184ae0ba): Helpful and Harmful
   tabs, search, sort (time left, name, what it changes), chips
   (Attributes, Skills, Vitals, Protections), groups Ending soon / Timed /
   From your gear, vitae card with the experience to the next 1%; details
   with what it changes (from and to when the value is known), who cast it
   or which item, time left of the whole, the description, a weaker copy
   underneath, Cast again with its bar slot (own spells) or Show item
   (gear). The HUD counts and the effects and vitae keys open it on their
   tab. Glyphs: the bake has no U+2212 or arrows, so minus is an en dash
   and "210 to 255".
2. Spellbook (`magic/spellbook*.rs`, 8db5e7b5): school and level filters
   are the server's bits (shared with retail), highest level only (on by
   default), search past the filters, sort; rows show the first bar a
   spell is on; details with mana, difficulty against your skill,
   components with uses and how many carried, the bar slots and keys;
   Cast (greyed outside Magic or Peace with a caster), Add to bar, Forget
   with its in-window question; drag a row to the bars (the book fades as
   before); double click puts it on a bar, right click examines it in
   Inspect. Components page: type filter, table with carried, restock to
   (click the box, type, Enter) and a stock bar.
3. Character (`character/*.rs`, 7acca51a): level plate that grows with
   the number, name, title, heritage and PK status, experience line and
   bar, chips for unassigned XP, credits, luminance and vitae; attributes
   and vitals with "+" and their cost; skills in four groups with search,
   "+" or Train; details with how the value adds up (starting value,
   raises, each spell on it when they add up, vitae), the game's
   description, +1 / +10 / Max or Train (retail's training question).
   One raise at a time until the sheet changes (released after 3 s).
   Titles page: grid with search, "Others see you as" and Display this
   title. Information page: the report's parts as cards (life,
   resistances, starting attributes, chess and fishing, masteries and
   augmentations, burden) plus allegiance and fellowship.
4. Panel strip: Character, Inventory and Spellbook light for the modern
   windows (49e330b3; the inventory never lit in M4).

Not done in M5: the Settings button in the Effects window opens retail's
options (M6 brings the modern ones); "Specialize" from the window (retail
has none either); Housing and Society cards on Information (the client
has no house facts outside the house panel yet). Not live-tested yet.

**M6 built 2026-11-20** on `feat/modern-ui-m6` (off `feat/modern-ui-m5`).
Retail windows it replaces: `options` and `keyboard-config`. Neutral
moves first:

- One place for an options window's change (a character option to the
  session, a settings section onto the settings in force, the chat
  following) and for the character-select question (91564d19).
- The options pages' string table public (2ef8535f).
- Configure Keyboard: each row's class; conflict, assign, erase, keep
  and key names reachable without the screen's buttons; retail's
  capture and overwrite question use the same calls (59474bcd).

Screens (`options/*.rs`), checked offscreen (`the_modern_options`):

1. Options (9c10e951): the eight sections, search across every setting
   and key (with where each lives), Character select and Exit game under
   the list. Rows are the window's own words over the game's option
   pages: what a row is, its range, choices and default come from
   `panels/options/spec.rs`, and a test checks every row of those pages
   is listed or left out with a reason (side-by-side vitals, the retail
   chat font and shading, the retail chat windows' filters). Groups of
   character options say "Saved on your character"; OpenAC's own carry
   OPENAC. Switches (the game's "Disable ..." shown the positive way),
   sliders, segments or a menu for choices, volumes as a switch and
   slider, buttons (Edit layout opens the bars' edit mode; the game's
   mouse turning settings). Changes are live through the same path as
   retail's window; Apply saves the character's options, Revert puts
   back, Defaults the section; closing without Apply reverts. The chat
   grid edits the modern chat's tabs (saved at once). The Effects
   window's settings button opens it on Interface.
2. Keys in Controls (def4f7a0): every action of the game's key map by
   class, three slots each, keys named the modern way ("Alt+D"). A click
   catches the next key in place ("Press a key"), a right click empties
   a slot; a key something else holds shows on the row ("S is used by
   Stop Moving") with Use it here or Keep, a key the game keeps is
   refused there. Key changes count with the others for Apply, Revert
   and closing. Key profiles: Load and Save as (still retail's dialogs
   until M7). The Configure Keyboard key opens Options on Controls.

Not done in M6: the item details' small-screen card; Trim. Both decided
2026-11-21 (see To build, decided 2026-11-21). Not live-tested yet.

**M7 built 2026-11-20** on `feat/modern-ui-m7` (off `feat/modern-ui-m6`).
Retail windows it replaces: `vendor`, `secure-trade` and `salvage`, and the
dialogs (the window's questions and the key-profile questions no longer
go through the dialog factory on Modern). Neutral moves first, one commit
each:

- Vendor rules: whether the vendor takes an item, what a cleared row
  gives up, selling part of a stack (split, then stage the new stack)
  (fa39a297).
- Salvage staging: the tool, the staged items, the one-material rule,
  pack contents, the send and the session's marks (b36db135).
- A salvage's bags reach the interface as `InterfaceEvent::Salvaged`
  beside the chat line (d3b7038c; stream shape re-pinned, format stays 3).

Screens, each checked offscreen (`the_modern_questions`,
`the_modern_vendor`, `the_modern_trade`, `the_modern_salvage`):

1. Questions (`dialogs/*`, 2892ebd3): one card for every kind (yes or
   no, OK, please wait, a typed answer, a choice), centred over a slight
   dimming, modal, one at a time with "1 of 3" and "2 more waiting";
   icon, colour, title and button words by kind (Apply, Untrain, Augment,
   Abandon...), red yes for what cannot be undone, Enter and Esc shown.
   Crafting shows the tool and target (the window notes the target picked
   while a tool waits; icons f03a267a). Allegiance and fellowship
   invitations are cards under the target (Decline / Accept or Join), noted
   in chat; play goes on. Key profiles (Load, Save as, overwrite) use the
   card too.
2. Vendor (`dealing/vendor_*`, f4909b07): Buy and Sell tabs with how many
   rows wait on each list, the purse (another currency with pyreals
   beside), search; Buy: the shop's categories, goods with price, stock
   and "On list"; Sell: the player's packs, every carried item with what
   the vendor pays or greyed with retail's reason, "Only what they buy",
   what the vendor buys in words; the chosen thing with a quantity (typed,
   -, +, Max / All / To restock), Buy now / Sell now and Add to list; the
   To buy / To sell list with total, now and after, Buy all / Sell all,
   Clear and a cross per row. Restock banner from the Spellbook's amounts
   (the `/fillcomps` planner, all categories, no ceiling). A refused drop
   shows the reason and the vendor's limit; leaving with lists asks in the
   window. Plugin marks and foot band (API 1.15).
3. Trade (`dealing/trade_*`, 3d9a010f): both sides with acceptance,
   items, count and total (money exact, items at vendor value, OPENAC tag);
   status pill; banner when the other player changed the trade after the
   player accepted (new items marked NEW) or the game refused an item
   (its words from the error table); Accept trade / Withdraw acceptance
   with the hint; Clear both sides; drop zone and Add from packs (stacks
   ask how many through the split pop-up, attuned greyed); a card for any
   item with Full details. Plugin marks and foot band.
4. Salvage (`dealing/salvage_*`, 42467ccf): the tool and the player's
   Salvaging skill; items with material chip and workmanship, out with the
   cross or a double click; drop zone that shows why a drop was refused;
   one bag per material with count and average workmanship; Clear, the
   options' material rule, Salvage N items; Add from packs with material
   chips; after the answer, each bag with units and workmanship, Salvage
   more.

Not done in M7: the vendor and trade windows' narrow layouts for small
screens (792 px); dragging a staged salvage item out (the cross and a
double click do it); a tool's target picked in the 3D world is not noted
for the crafting card (inventory targets are). Not live-tested yet.

**M8 built 2026-11-20/21** on `feat/modern-ui-m8` (off `feat/modern-ui-m7`).
Retail windows it replaces: `social-panel`, `journal`, `book`,
`house-purchase` and `map-house` (the house page became its own window).
Neutral moves first, one commit each:

- Fellowship: who left a locked fellowship, with the name they had, reaches
  the interface (326007b4; stream shape re-pinned, format stays 3).
- Fellowship: a member's experience share in its own function (d9774737).
- Settings: the quests each character shows on screen, `modernQuests`
  (39256a10); each character's map pins, `modernPins` (52304ee4).
- House: the house itself (type, place, dates, each maintenance item paid
  of needed), the guest list and the houses for sale reach the interface
  (65991493; stream shape re-pinned).
- Map: the map art (image, size, marker area) and the projection both
  ways in their own functions (db25860a).
- The kit gained a hit registry (what can be clicked is noted while
  drawing), switches, the OPENAC tag, input boxes with their fields, pills.

Screens, each checked offscreen (`the_modern_social`, `the_modern_journal`,
`the_modern_book`, `the_modern_house`, `the_modern_map`):

1. Social (`social/*`, f8d703c0): O, F3, F4 and the friends key open it on
   their tab. Allegiance: the line of fealty, vassals by rank with what
   they passed up, a card for anyone (Tell, Invite, Add friend; Release or
   Break asked on the question card), ignore requests, swear to the
   selected player. Manage allegiance (OPENAC, monarch only): rename, the
   message of the day, appoint, remove or retitle an office, lock and
   approve, ban, lift, gag, put out of chat, the lists in chat; all as
   the chat box's `@allegiance` commands. Fellowship: shares, vitals, loot,
   who may rejoin, the leader's actions, Disband asked first, the four
   switches. Friends online first with what else they are (Patron,
   Vassal, Fellowship). Ignored: each person with what is hidden from them
   and a menu to change it (`@squelch -Kind`), everyone (`@filter`).
   Fellows on screen (OPENAC): a HUD piece that keeps the vitals streaming
   (the window asks for them itself; retail's page never shows).
2. Journal (`journal/*`, 75edc2f4): Quests grouped (in progress, waiting
   to repeat, can be started, done), search, the picked quest's words,
   progress, time, who and where with Map; Abandon (or Remove for a done
   one) asked first. Notes: pages with label and countdown, search, New;
   the page's label, title and notes with counters (sent when a field is
   left, a page turned or the window closed), Record and Map, one
   countdown at a time, Delete asked first. Quests on screen (OPENAC) per
   character, a HUD piece.
3. Book reader (`journal/book.rs`, d81929f2): title (inscription), page
   list with who wrote each, read only or typed on, saved on turn or close.
4. House (`house/*`, 7637ab55): H and the panel strip. Type, place, Show on
   map, dates, maintenance bars, the note, the buying rule; who may come in
   (OPENAC, `@house` commands; the guest list is asked for when the window
   first opens, so its chat lines show once); no house: free houses by
   type. The house sale window in place of the slumlord window: rules
   ticked against the player, a row per price line with Add, what is on
   the rows, Buy or Pay maintenance asked first as the game asks.
5. Map (`world/*`, 40c4eaae): full screen, M, F10, the panel strip, Esc.
   The game's map art zooms and pans; towns, the player's arrow, house,
   pins (right click, per character), houses for sale (`@hslist
   cottage`); a card per town or pin (Copy, Link in chat, Remove pin); the
   side column (where you are, nearest town and facing, Dereth date, Find
   a place, the layers). Underground dims and keeps the last outside place.
   The Journal's and House's Map buttons open it on their place.

Not done in M8 (the map's plugin markers were cut, user 2026-11-21): the
map's drawer and the windows' small-screen layouts
(the side column only folds); officers cannot be told from members (the
client is not told), so Manage stays monarch-only. Not live-tested yet.

**M9 built 2026-11-21** on `feat/modern-ui-m9` (off `feat/modern-ui-m8`).
Retail windows it replaces: `mini-game` (the Game Center) and `barber`.
Neutral moves first, one commit each:

- Barber: what a barber window decides (each part's slots, how many
  styles and colours, the marked colour, the style a pick or a step asks
  for) in `ac_ui::panels::barber_rules`; the retail window uses it
  (7eae3214).
- Chess: the squares a picked piece can move to and a move's usual
  written form (`ChessBoard::targets`, `ChessBoard::notation`,
  5e15ca3d); the moves made reach the interface (`ChessView::moves`,
  8e0ed64b; stream shape re-pinned 60bacf5e217f5cb1, format stays 3).

Screens, each checked offscreen (`the_modern_chess`, `the_modern_barber`):

1. Chess (`chess/*`, 15df31ba): a used board opens it, the chess chip
   brings it back, closing it leaves the game going. The board from the
   player's side (Black's view when watching) in the game's own piece
   images, the last move marked; the two sides with whose move it is; a
   status line (joining, waiting for a seat, your move, check, the piece
   picked up, move sent, game over); Offer or Take back stalemate;
   Resign asked first on a card. OPENAC: a picked piece shows dots where
   it can go and rings what it can take (a click on another own piece
   picks that one, right click lets go); the moves made, two to a line,
   the last in gold.
2. Barber (`barber/*`, 8cbc629d): shown while the barber is open. The
   figure (the barber's own preview slot) with turn buttons and Hold for
   before (OPENAC: the look as it opened while held); part tabs (Hair,
   Eyes, Nose, Mouth, Skin, or a Gearknight's or Olthoi's three, in the
   game's own words); a style stepper and numbered picks that fit the
   room (Aluvian women have 50 hair styles, not the 12 the board drew:
   up to twelve to a row, paged round the picked one if still too
   many); colour swatches for hair and eyes; a shade slider whose track
   runs through the palette set's own colours; the heritage's one option
   as a switch; "Changed: ..." since it opened; Cancel (and the close
   button) and Accept.

Credits moved to M10: they are reached only from character selection,
which the modern layer does not cover until then, so they are built with
the Stage.

Not done in M9: the opponent's name (the game does not send it; the side
says "Your opponent"); dragging a piece (click, then click); turning the
barber figure by dragging it. Not live-tested yet.

**M10 built 2026-11-21** on `feat/modern-ui-m10` (off `feat/modern-ui-m9`).
On Modern the retail pre-game screens are no longer mounted (taken down
when the modern layer mounts). Neutral moves first, one commit each:

- Credits: what they show read without building the retail screen
  (`ac_ui::pregame::credits_resources`, 71b5c9e3).
- When the connecting screen shows (`ConnectionPresence`, ba5f88fb) and
  when character selection shows (`selection_shows`, 068409b3), as rules
  both interfaces use.
- Settings: the remembered characters (`modernCharacters`, a member per
  "world/name": level, heritage, gender, last place, when, the look as
  worn; ec185fc0).
- Viewports: a Stage slot that draws a remembered look as the creation
  previews draw theirs (rest pose and idle, no scenery; 3ee54f3f).
- Creation: the part-pick and style-step rules (`pick_command`,
  `step_command`) and the counts they read (1716c2d8); the keys of the
  game's own words for heritages, professions, towns and name examples
  made public (0f7e7717).

Screens, each checked offscreen (`the_modern_stage`, `the_modern_studio`):

1. Connecting, the Stage and the credits (`pregame/{stage,credits,mod}`,
   19a025f7). Connecting: the portal ring (outer arc the connection,
   inner the data check), the status lines, red with the reason on a
   failure, Cancel. The Stage: the roster (level plate, name, heritage ·
   last place; "Not played on this computer yet"; pending deletions red
   with their countdown, counted from when seen), the picked character
   centre stage in the sky of its last place (desert, Sho blossom, coast,
   dusk; unknown grey; faded while pending), its name large with level ·
   heritage and last in · when, the figure from the remembered look,
   Enter world (Enter key, double click), Delete (the card wants the
   game's word typed, "DELETE"), Restore with the warning, the error card,
   "Entering world…" and "Please wait…"; world, population, account
   across the top; Credits, Exit (asks, the game's words), version in the
   corner; an empty account's welcome and Create your first character;
   arrows walk the roster. The credits: the game's pictures rising on the
   left, its text on the right (starting part way up so the screen is
   never empty), any key or click or Back to your characters. While a
   character plays on Modern, what the Stage shows of it is saved (on a
   change, and every five minutes).
2. The Studio (`pregame/studio/*`, 82f44a93): opened from Create, rolling
   a random character and turning its gender as retail does. The rail,
   the glass panel per step with Random (none on Skills), the So far card
   (the summary with skills on Name), the figure (whole body; the face up
   close on Appearance › Face, headgear off while it shows as retail),
   Turn and Zoom, Back · Step N of M · Next / Finish, Randomize all and
   Exit asking first. Heritage tiles with masteries read from the game's
   bonus words; the professions' and towns' words are the game's; the
   name examples come from the game's naming words (Aluvian,
   Gharu'ndim, Sho, Viamontian only); the server's refusal shows its words
   and, for a name in use, three near names that try at once (OPENAC).

Not done in M10: pulling the host-facing methods into a trait and
removing the hiding list (the last step of "nothing retail mounted";
the retail windows are still built under Modern and hidden), to agree
before starting; Lugian and the Olthoi have no mastery words in the game
data (their tiles say nothing, or Always PK); the README's "a big change
makes you Custom, titled Adventurer" warning (retail has no such rule
that the client knows of); typed attribute values (slider and steps
instead); clothing shade tracks are plain (only hair and skin read their
palettes); the figures do not turn by dragging. Not live-tested yet.

**M10 finished 2026-11-21** on `feat/modern-ui-retail-off` (off
`feat/modern-ui-m10`): nothing retail is mounted in game on Modern.

- Neutral first, one commit each: the selected stack's split quantity
  follows the selection on its own (`ac_ui::panels::items::SelectionSplit`,
  the toolbar's split uses it); the key rows without the Configure Keyboard
  screen (`KeyboardConfigController::without_screen`); `UiHost::remove_window`
  (unregistered, controller dropped, elements freed); the Game Center's
  follow rule (`MiniGameFollow`); a house's guest list goes with the house.
- The retail windows the modern pieces stand in for are taken down once
  when the layer mounts, not hidden before every frame; the hiding list is
  gone. What the modern pieces read from them moved out first: the split
  amount (the target line's own), the character sheet (built by the window
  for either interface), the key rows (screenless on Modern: Configure
  Keyboard is never built there), the book opening the reader, the chess
  window following the games, the panel strip's map check. Layout
  persistence only follows registered windows, so the saved retail layout
  is untouched for when Retail is picked again.
- The trait: not built. With one window type serving both interfaces it
  would have a single implementor, so it would add a layer and remove
  nothing; it makes sense only if Modern ever becomes its own window type.
  The web client never sets an interface, so it is unchanged.

Review of M5 to M10 (two independent reviews; Codex ran out of credits
after one milestone) and the fixes, same branch: vendor leave card clicks,
drops on the goods list, dropped stacks honour the split; Options sliders
save when let go, Defaults while searching only touches the rows found,
settings and options words follow separately; panel keys open the modern
Effects and Character windows; trade and salvage keep nothing asked while
closed and take only pack items; game-decided windows (salvage, trade,
vendor, loot, house sale, barber) are no longer restored at login; a
forgotten spell is unchosen; journal typing survives Record and Stop; the
map's zoom clamp could panic; map and Stage memory reset on a character
switch; barber slider and paging; Studio name and attribute sends, one
text table; the Stage's remembering throttled; retail no longer keeps
salvage results.

Quick move key: Options › Controls › Quick move (Control, Option or
Command on a Mac; Ctrl, Alt or the Windows key elsewhere), default Control,
live at once (settings `modernQuickMoveKey`).

Backdrops (2026-11-21, same branch, 73d5c66f): the Stage (connecting and
character selection) and the Studio show pictures of the world instead of
the drawn skies: fifteen views the user picked from 25 candidates rendered
by the client (gallery artifact 2q8nGyTm3wiikX37v4v21S), finished with a
painterly pass, bundled as WebP in `assets/backdrops` (4 MB; native only).
A random one shows for 30 s with a slow push in, the next fades in over
1.5 s; only the shown picture is held, and all are let go in the world.
For upstream: these are renders of the game's own art, so whether they
may ship in the repository is Erik's call (they could instead be made on
the player's computer from the dats).

Detailed world map (2026-11-21, branch `feat/modern-ui-map-tiles` off
`feat/modern-ui-retail-off`: d32e9733, 681112e3, b5a4911f; not pushed,
not live-tested; to be discussed). The user's choices: use Aethrix's
existing WebP map rather than baking one in the client; offer all four
tiers and let the player pick; manage the download in game, in the
Options window only (not the launcher).

- Source: the public release `atlas-map-v1` on
  `DavidSmithZA/OpenAC-Aethrix`, a manifest and parts of at most 16 MB per
  tier: Standard 11.4 MB (4 px a 24 m cell, towns 8 and 16), HD 35.5 MB
  (8; towns 16 and 32), Ultra 125 MB (16; towns 32), Extreme 885 MB (32,
  no separate town tiles). 256 px WebP tiles, pixel (0, 0) the world's
  north-west corner, map coordinates `x * 240 + 24468` metres.
- Crate `ac-world-map`: manifest, index and part reading for every build;
  with its `net` feature the download (the launcher's HTTPS and redirect
  checks), the store and two threads that read and decode tiles. Parts are
  kept as downloaded and tiles read at their offset (Extreme: 56 files,
  not 86,870); each part is checked for size, SHA-256 and table before it
  takes its name; `manifest.json` is written last as the marker; a stopped
  download keeps the parts it has. Folder: `world-map/<tier>/` in the
  data folder.
- Setting `modernMapDetail` (game, standard, hd, ultra, extreme; Standard
  by default). The map draws the chosen tier when downloaded, else the
  most detailed one that is, else the game's map.
- Options › Graphics › World map: Map detail, and a Detailed map row
  saying what is here and under way, with Download, Update, Cancel and
  Remove and a progress bar.
- The map: the world level as fine as the screen, up to three coarser
  ones under it so nothing shows empty while loading, the towns' finer
  tiles over it past the finest world level; tiles asked nearest the
  middle first, at most 24 uploaded a frame, 288 kept, all let go when
  the map closes; each tile sampled half a pixel in (the first render
  showed seams). The zoom goes on to twice the tier's finest detail (up
  to 192x; 8x on the game's map alone).
- Checked: unit tests; every real tier read and decoded
  (`AC_WORLD_MAP_PACKS`); a real Standard download from GitHub (4.6 s,
  `AC_WORLD_MAP_LIVE=1`); offscreen pictures with the Extreme and HD
  packs (`the_modern_map_detailed`, `AC_WORLD_MAP_PACK`): tiles line up
  with the coordinates and Holtburg's buildings sit around the player at
  42.1N 33.9E. `check.sh` not run.

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

### M7 Dealing ("Vendor", "Trade", "Salvage", "Dialogs") (built 2026-11-20)

Vendor, Trade, Salvage; the one dialog card with its queue; invitation
cards under the target.

Replaces: vendor, trade, salvage, dialog, dialog factory.

### M8 Social and world ("Social", "Journal", "Map", "House", "Dialogs") (built 2026-11-21)

Social window; Journal (quests, notes, quests on screen) and the book
reader; full-screen Map with pins, markers and houses for sale; House
window; house sale window.

Replaces: social, journal, contracts, book, map/house, house purchase.

### M9 The rest of the game ("Loot, barber, chess") (built 2026-11-21)

Barber, Chess. (Credits moved to M10, beside the character selection
they are reached from.)

### M10 Pre-game ("Login", "Character creation", "Small pieces") (built 2026-11-21)

Connecting screen (portal ring) and Stage character select, with the
per-computer character memory; Studio character creation; Credits.

After this nothing retail is mounted on Modern: pull the host-facing
methods into a trait, check the web client, remove the hiding list.

### Plugin hooks (alongside M4 and M6)

A new plugin API version: the client asks plugins about an item (when
it is inspected, shown in a loot window, inventory, trade or sell list)
and draws what they return (a verdict, a few lines, menu entries, foot
buttons). The item hooks land with M4. (Options pages drawn by plugins
were cut, user 2026-11-21: plugin settings stay in the plugin's own dock
windows.) Retail never calls them;
the plugin API is shared, so retail could show the same answers later.

### To build, decided 2026-11-21

- Item details on small screens (Docked / Pop-up / Auto cut): where the
  wide inventory (1080) does not fit, it stays at 792 and the details are
  a card inside the window over the equipment column; Esc, its close
  button or a click on empty space puts it away. No option, no pinning.
- Theme (Trim cut): the modern interface follows `plugin_ui.theme`.
  1. The about 80 places that write the bronze directly take it from the
     palette.
  2. A theme's 7 colours map onto `HorizonPalette` (accent: trim and
     highlights; background: glass tint; text, muted, selected across);
     the vitals, good, bad, warn and enemy stay fixed; Classic is
     `Horizon::BRONZE`; Parchment keeps the dark glass and takes only its
     accent.
  3. Options › Interface › Theme (replacing Trim), a theme button on the
     Stage, and the dock's picker write the same setting; a change
     redraws everything at once.
  4. Snapshot tests: every window in two or three themes, Parchment
     among them.

Built 2026-11-21, each on its own branch stacked on
`feat/modern-ui-small-screens`:

- Quick move key: Command on a Mac at first, Control elsewhere
  (`feat/modern-ui-mac-quick-move`, 3ceba2b9).
- Item details card on small screens (`feat/modern-ui-details-card`,
  83049aff): `inventory::CARD_*`, put away by its close button, Escape
  (after the map's), a click on empty space or an empty slot
  (`CellRequest::Empty`); the narrow foot's hint wraps beside the
  pyreals. Offscreen: `the_modern_small_inventory`.
- Theme (`feat/modern-ui-theme`, c2b347f6, ea54db5d): `Horizon::current`
  (thread-local, set each frame by `follow_modern_theme` from
  `plugin_ui.theme`), `Horizon::themed`, `modern::tone`; about 280
  colours written into windows now come from the look, and 33 widgets no
  longer keep a copy of it. `OptionKey::Theme` (Options › Interface ›
  Style, and the retail options' modern section), the Stage's Theme
  button and list. Offscreen: `the_modern_themes`, end of
  `the_modern_stage`.
- Backdrops at 4K (`feat/modern-ui-backdrops-4k`): all 15 drawn again at
  5120x2880 and shipped at 3840x2160 (9.8 MB of WebP), four framed again,
  a focus point each (`FOCUS`, `cover`); accepted by the user. Masters and
  tools in `~/OpenAC-dev/modern-ui-backdrops` (`masters-2x/`,
  `manifest-v2.json`, `tools/render2.sh`, `finish4k.sh`).
- Known: `the_m2_gaps` fails on these branches and their base (the bars
  are not found in the saved settings file); not looked into yet.

### Upstream (on hold, user 2026-11-21)

How the modern interface goes to Erik (the PR's shape, and whether the
backdrops and the detailed world map ship) waits until the feature is
releasable.

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

- Quick move's key on a Mac: now a choice in Options › Controls (default
  Control everywhere). Whether a Mac should default to Option is still
  yours to say.
- Server level cap: settled by the game itself. At the top of the
  experience table the Character window shows the game's own words for
  it ("Infinity!"), as retail does; no "Max level" text.
- Officers and bans in Manage allegiance: the client is not told who the
  officers are, so Manage stays monarch-only.
- Detailed world map, to discuss: whether it goes upstream at all (the
  client would download renders of the game's art, so Erik's call, as
  with the backdrops); where the tiers are hosted (now a fixed address on
  the Aethrix release; a setting would need a non-Copy settings field);
  the default (Standard, or the game's map until the player picks); the
  Map detail row also showing in the retail options' modern section; a
  live test of a download and of zooming into towns.
