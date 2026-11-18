# Modern UI: design notes

A second interface for the client, beside the retail one. It must do
everything the retail interface does; how it looks and works is open.
This folder holds the design only. There is no code yet.

- Live canvas (all boards, clickable where marked):
  https://claude.ai/artifact/3c5NpLimjKtfusjYEbWfpQ
- `canvas/`: a copy of the canvas source (one `.dc.html` per board, plus
  `canvas.json` for the layout), so the design survives without the link.

## Direction

- **Switch:** Options › Interface › Interface: Modern / Retail. It
  switches at once, with a short "Switch back" notice. Each interface
  keeps its own layout.
- **Look:** "Horizon", bronze trim. The world comes first and the chrome
  stays thin: slim vitals top centre, target under them, round radar top
  right, chat with no panel behind it bottom left, and a small strip of
  panel buttons bottom right. Fonts are Inter.
- **Numbers:** levels can run into the thousands and XP into the hundreds
  of billions. No level or XP display may assume a few digits.

## Shortcut bars

- Players make as many bars as they like, put each anywhere on screen,
  horizontal or vertical, and set a key for every slot.
- Items and spells go on the same bars. Retail's separate spell bars are
  gone, and the spellbook has no bar strip.
- Outside Magic mode a spell slot is greyed out and does nothing.
- A slot can have a different key in Magic mode. For example, the Main
  bar uses 1 to = normally and Shift+1 to Shift+= in Magic mode, while
  the Spells bar takes 1 to =.
- Clicking an item slot always works, in any mode.
- Edit layout mode: drag a bar by its tab, turn it, add or remove slots,
  click a slot to set its keys (clashes are caught), make a new bar,
  reset the layout.
- A layout can be per character or shared, and copied from another
  character.
- Dragging a spell out of the spellbook fades the book to about 12% so
  bars behind it can take the drop. Letting go anywhere else cancels.

## Inventory

- One window: equipment (figure plus all 24 retail slots) | pack column
  and grid | item details.
- Pack column: All packs, Main pack, side packs, each showing how full it
  is. All packs groups everything by pack, by category, or not at all.
  Search covers every pack.
- Sort by name, type, value, burden, workmanship, newest, or your own
  order. Tidy: merge partial stacks, close gaps. Automatic rules such as
  sorting items into packs belong in plugins, not the client.
- Item details: fixed header, scrolling sections that fold, fixed action
  buttons. The main button says Equip for things you wear or wield and
  Use for things you use. The small buttons are Give, Drop, Salvage, and
  More (split, inscribe, add to bar, link in chat).
- Small screens: with nothing selected the window narrows to 792 px. The
  details then pop up as a card beside the item, which can be pinned back
  into the window. Docked, Pop-up or Auto is an option.
- Drag to equip (the slots an item covers light up), split a stack, and
  an over-burdened state.

## Spellbook

- Filters: school (with counts), level I to VIII, and a "highest level
  only" switch.
- The list is grouped by school and shows which bar each spell is on.
- Details: mana, difficulty against your skill, components with how many
  you carry, which bar it is on. Buttons: Cast, Add to bar, Forget
  (which asks first).
- Components tab: what you carry and a restock amount for each, which
  vendors fill.

## Options

- Sections: Interface, Gameplay, Social, Chat, Controls, Graphics, Sound,
  Camera, plus a search across all of them. Character select and Exit
  game sit under the section list.
- Settings retail does not have are tagged OPENAC. Groups saved on the
  character (on the server) say so.
- Chat tab filters are one grid: message types against chat tabs.
- Changes show live. Apply keeps them; Revert and Defaults as in retail.

## Character sheet

- Header: a level plate that grows with the number, name, title,
  heritage, PK status, an XP bar, unassigned XP, skill credits, luminance
  and vitae.
- Attributes and vitals, then skills grouped as specialized, trained,
  untrained and unusable, each with its raise cost.
- Details for the selected row: how the value adds up (attributes,
  raises, specialization, spells, vitae), then +1, +10 and Max. An
  untrained skill shows Train with its credit cost.
- Titles tab: pick the title others see. Information tab: life,
  allegiance, fellowship, housing, pursuits, and what you carry.

## Chat

- Bottom left, as on the HUD. Idle, it has no panel or tabs: the last
  few lines fade out, with a small "Enter to chat". Enter (or a click)
  brings back the tabs, the message box and the entry.
- Tabs (Main, Combat, Fellowship, Allegiance, Tells, and any you make)
  show unread counts. A new tab gets a name, the message types it
  shows, the channel it talks on, and whether it opens as its own
  window. What every tab shows is also the grid in Options › Chat.
- Retail's four floating chat windows become popped-out tabs: drag a
  tab off, or use the pop-out button. A window has its own filter and
  background opacity and can be put back as a tab. It talks on its
  tab's channel; a window with no channel (Combat) has no entry. There
  is no limit of four.
- Lines: optional time, a channel tag (TELL, FELLOW, ALLEG, GENERAL,
  TRADE, LFG, PLUGIN), the speaker's name, then the text, in the
  channel's colour. Combat, magic, experience and system lines have no
  tag. A line that mentions your name is tinted gold. Coordinates in a
  plugin's line are links.
- The entry's channel button shows where a line goes. Its menu lists
  every retail talk channel (Say; Fellowship, Allegiance, Monarch,
  Patron, Vassals; General, Trade, LFG, Roleplay, Society, Olthoi) and
  recent tells, each with its @ command. Channels you cannot use now
  are greyed with the reason ("no vassals", "Olthoi only").
- Clicking a tab switches the channel to that tab's own; Main keeps the
  last one used.
- Typing @ suggests commands (@reply, @retell, @cg, …), and the channel
  button shows where the line will really go, such as "Reply · Rook".
- Clicking a name: Tell, Select (greyed when not nearby), Invite to
  fellowship, Add friend, Copy name, Squelch.
- Reading back holds the chat still; new lines wait behind a "14 new
  messages" button.
- Search finds text in the chat history and marks the matches.
- Retail's yellow status messages ("out of range") become one short
  line under the target, which fades.
- Not in retail, all kept: search, highlighting your name, more than
  four windows, per-window opacity. In Options they carry the OPENAC
  tag.

## Still to design

Vendor, trade, salvage, appraising other players and creatures, social
(allegiance, fellowship, friends), map, journal and quests, pop-up
dialogs, login and character creation. After that, a check
of every retail panel in `crates/ac-ui/src/panels` against this design.

Open question: at a server's level cap, does the XP bar say "Max level"
or keep counting total XP?
