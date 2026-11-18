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
- Item details: fixed header, then the same four tabs as the inspect
  sheet (Overview, Spells, Needs, Details; empty tabs are left out), a
  scrolling body and fixed action buttons. The main button says Equip
  for things you wear or wield and Use for things you use. The small
  buttons are Give, Drop, Salvage, and More (split, inscribe, add to
  bar, link in chat).
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

## Vendor

- One window with two tabs, Buy and Sell, in place of retail's three
  pages. Each tab shows how many things wait on its list. The purse
  sits in the header.
- Buy: the categories this shop stocks (with counts) | the goods as
  rows, each with its price, stock left ("Unlimited", "1 left") and how
  many are already on your list | the selected item (quantity, Max,
  cost, Buy now, Add to list) above the To buy list (total, your
  pyreals now and after, Buy all).
- Sell: your own items are listed in the window, so the inventory need
  not be open (dragging from it still works). Each shows what the
  vendor pays. Items the vendor will not take stay listed, greyed, with
  retail's reason ("too valuable to sell here", "has no value"). The To
  sell list shows what you receive. Selling part of a stack splits it
  off first.
- Dropping a refused item on the sell list shows the reason and the
  vendor's limit in place.
- Spell components: when you carry less than your restock amounts
  (Spellbook › Components), a banner offers to add what is missing to
  the buy list. It only fills the list; you still press Buy.
- A shop that takes another currency prices everything in it and shows
  that currency in the purse, with your pyreals beside it.
- Leaving with anything on either list asks first, and says nothing is
  bought or sold if you leave.
- Small screens: the window narrows to 792 px. The category or pack
  column becomes a drop-down above the list, and what the vendor buys
  moves behind an info button.

## Trade

- Starting a trade works as in retail: double-click a player, or drag
  an item onto them when "Drag item on player opens secure trade" is on
  (otherwise the drag gives the item, if they accept gifts). The window
  opens straight away for both of you, with no prompt.
- Two columns, You offer | the other player offers. Each has an
  Accepted / Not yet chip, its items (name, type, stack, value) and a
  total. A side's border turns green when it has accepted. The header
  carries a status: Offering, Osk accepted, You accepted, Both
  accepted, Did not go through.
- Accept trade; once you have accepted, Withdraw acceptance while you
  wait. When the other side has already accepted, a hint says your
  accept completes the trade.
- The game cannot take back a single item, so there is no remove
  button on an item: Clear both sides empties both, and you offer again.
  Any change takes back both acceptances.
- Changed after you accepted: a banner says your acceptance was taken
  back, and items new since then are marked NEW.
- Add from packs picks items without opening the inventory. Stacks ask
  how many; worn and wielded things are not listed; attuned items are
  greyed ("cannot be given away"). Dragging from the inventory works too.
- Click (or right-click) any item on either side to inspect it: value,
  workmanship, burden, spells, and a link to the full appraisal. Value
  is the vendor value, not what players ask.
- Value totals are not in retail and are kept, tagged OPENAC: exact for
  pyreals and trade notes, vendor value for items.
- A failed trade shows the server's reason in a banner.
- Small screens: the window narrows to 792 px with the same two
  columns; the hint lines go.

## Salvage

- Using a salvaging tool (an Ust) opens the window; the header shows
  your Salvaging skill.
- To salvage: each item with its material and workmanship. Take one out
  with its x, by double-clicking, or by dragging it away. Drop items or
  a whole pack (which adds everything in it that can be salvaged), or
  use Add from packs: only salvageable things are listed, with material
  chips to narrow it; retained items are greyed.
- One bag per material: each bag shows how many items go into it and
  their average workmanship. No amounts are shown in advance; they
  depend on your skill and each item's workmanship.
- The bottom bar says whether your options allow several materials at
  once, with Clear and Salvage N items.
- Nothing is asked before salvaging (no extra warning for items with
  spells or high value), as in retail.
- A refused drop shows the reason where it was dropped.
- After salvaging, the window shows each bag you got, with its units
  and workmanship, and Salvage more. The same lines go to chat.

## Inspect

- One sheet for everything you can assess, as retail's one examination
  window: items (anything not alive, so loot, gear, portals, chests,
  doors), creatures, players, and spells examined from a bar. It sits
  on the right edge under the radar, 360 px wide.
- Tabs appear only when there is enough to split:
  - Players: Stats, Allegiance, Armor, About.
  - Items: Overview (value, burden, weapon or armor numbers, ratings,
    set, a magic summary), Spells (mana, spellcraft, every spell and
    cantrip), Needs (each requirement against your own value, ticked or
    crossed), Details (tinkering, crafter, inscription, description).
    Empty tabs are left out. The inventory's item details use the same
    tabs, so an item reads the same in a pack or on the ground.
  - Creatures, portals, chests and doors, and spells: one column.
- Head: an icon for things; level and a health bar for creatures and
  players.
- Values raised or lowered by a spell keep retail's green and red and
  add an arrow; unknown values show ???. A failed assessment says so
  and shows only health.
- Portals show their use text, level range and retail's restriction
  lines. Chests and doors show the lock, retail's "looks hard to pick
  (Resistance N)" line, your Lockpick and the capacity. Spells show
  mana, difficulty against your skill, the description and components
  with what you carry.
- Inscriptions: a signed one shows read-only with who signed it; on
  your own unsigned item it is a field you write in.
- No action buttons: the chat name menu and right-clicking a player
  cover Tell, Trade and the rest.

## Social

- One window, 792 x 600, opened with O. Tabs: Allegiance, Fellowship,
  Friends, Ignored (retail's four pages, squelch included).
- Allegiance: the allegiance's name and member count, how many follow
  you, Allegiance chat and Hometown buttons, and the message of the day
  with who set it. A Monarch › Patron › You strip, then your vassals with
  the experience each passed up. Clicking anyone shows level, rank,
  heritage, loyalty and leadership, experience passed up, and Tell,
  Invite, Add friend. Release (a vassal) and Break allegiance (your
  patron) ask first and say that the person's own followers go with
  them. "Ignore allegiance requests" sits in the bottom bar.
- With no allegiance: a short explanation and "Swear allegiance to X",
  which uses your selection; they are asked to accept.
- Manage allegiance (OPENAC), for the monarch only: rename, message of
  the day, appoint or remove officers (Speaker, Seneschal, Castellan)
  and rename the offices, lock with one approved name, ban or lift a
  ban, gag or remove someone from allegiance chat. Retail has these as
  @ commands only. The server answers officer and ban lists in chat, so
  the view links to "List them in chat" rather than keeping lists.
  Officers keep using @ commands. Both may change at implementation
  (the client cannot tell who is an officer today).
- Fellowship: name, who leads, members of 9, how experience is shared,
  Open/Closed and Locked chips. Each fellow's level, share, health,
  stamina, mana, and whether they share loot. People who left a locked
  fellowship and can rejoin are listed. Your settings: share my loot,
  ignore requests, accept automatically, show fellows on screen
  (OPENAC). Members see Recruit (the selected player) and Leave; the
  leader also gets Make leader, Dismiss, Open or Close recruiting, and
  Disband.
- With no fellowship: Start a fellowship (name, share experience, which
  cannot change later), and how joining works.
- Fellows on screen (OPENAC, on by default): small frames under the
  buffs with name, level and the three vitals; a fellow low on health is
  edged red; clicking one selects them (for healing). Retail only streams
  fellows' vitals while the Fellowship page is open, so this keeps that
  stream on.
- Friends: online first, then offline, with tags such as Patron,
  Fellowship, Monarch, Vassal. The selected friend gets Tell, Invite,
  Remove. Add by name (50 at most) and Appear offline.
- Ignored: ignore a character or a whole account (50 at most). Each row
  shows what is hidden and a menu to change it (everything, speech,
  tells, emotes, spell words, allegiance and fellowship chat, combat
  messages). An Everyone row covers the server's global squelch.

## Still to design

Map, journal and quests,
pop-up dialogs, login and character creation. After that, a check of
every retail panel in `crates/ac-ui/src/panels` against this design.

Open question: at a server's level cap, does the XP bar say "Max level"
or keep counting total XP?
