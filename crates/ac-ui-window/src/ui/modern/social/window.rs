//! The Social window's frame: the header, the four tabs, and what each
//! tab shows (the pages are drawn in `pages.rs` and `people.rs`), the
//! Manage allegiance page over the allegiance, and the menu of what to
//! hide from someone. What can be clicked is noted as it is drawn, so a
//! click always lands on what was on screen.

use std::sync::Arc;

use ac_ui::draw::RenderContext;
use ac_ui::event::{UiEvent, kind};
use ac_ui::modern::Horizon;
use ac_ui::tree::{Element, ElementId, Widget};
use ac_ui::{UiFont, UiRoot};

use super::model::{SocialScreen, Tab};
use crate::ui::modern::kit::{self, Rect, Scroll, inside};

/// The window's size.
pub const WIDTH: f32 = 792.0;
pub const HEIGHT: f32 = 600.0;
pub(super) const HEADER: f32 = 52.0;
pub(super) const TABS: f32 = 40.0;
pub(super) const PAD: f32 = 20.0;
pub(super) const FOOT: f32 = 64.0;
/// Where a page's content starts.
pub(super) const BODY: f32 = HEADER + TABS;

/// Two people side by side, the window's glyph.
const PEOPLE: &[&[(f32, f32)]] = &[
    &[
        (2.0, 21.0),
        (3.0, 17.5),
        (5.5, 15.0),
        (9.0, 14.0),
        (12.5, 15.0),
        (15.0, 17.5),
        (16.0, 21.0),
    ],
    &[
        (16.0, 3.5),
        (18.5, 5.0),
        (19.5, 7.0),
        (18.5, 9.5),
        (16.0, 11.0),
    ],
    &[(18.0, 14.5), (20.5, 16.0), (22.0, 21.0)],
];

/// The fonts the window draws with.
#[derive(Debug, Clone, Default)]
pub struct SocialFonts {
    /// The title.
    pub title: Option<Arc<UiFont>>,
    /// A page's heading (the allegiance's name).
    pub heading: Option<Arc<UiFont>>,
    /// A card's name.
    pub name: Option<Arc<UiFont>>,
    /// Names, buttons.
    pub strong: Option<Arc<UiFont>>,
    /// Words.
    pub body: Option<Arc<UiFont>>,
    /// Quiet lines.
    pub small: Option<Arc<UiFont>>,
    /// Spaced capitals, tags.
    pub label: Option<Arc<UiFont>>,
}

/// The fields over the window, each shown on its own page.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Slot {
    /// A new fellowship's name.
    FellowshipName,
    /// A friend to add.
    FriendName,
    /// Someone to ignore.
    IgnoreName,
    /// The allegiance's new name.
    AllegianceName,
    /// The message of the day.
    Motd,
    /// A member to appoint (or an office's new title).
    Officer,
    /// The one person approved to swear into a locked allegiance.
    Approved,
    /// A character to ban, gag or put out of the chat.
    Banned,
}

impl Slot {
    /// Every field.
    pub const ALL: [Self; 8] = [
        Self::FellowshipName,
        Self::FriendName,
        Self::IgnoreName,
        Self::AllegianceName,
        Self::Motd,
        Self::Officer,
        Self::Approved,
        Self::Banned,
    ];

    /// The field holds more than one line.
    #[must_use]
    pub const fn lines(self) -> bool {
        matches!(self, Self::Motd)
    }
}

/// The fellowship switches.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FellowSwitch {
    /// Share my loot.
    ShareLoot,
    /// Ignore fellowship requests.
    IgnoreRequests,
    /// Accept requests automatically.
    AutoAccept,
    /// Show fellows on screen (OPENAC).
    OnScreen,
    /// Share experience (a new fellowship).
    ShareXp,
}

/// What the frame asks of the window.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SocialAsk {
    /// Close the window.
    Close,
    /// The page shown changed.
    Tab(Tab),
    /// Allegiance chat; recall to the allegiance's hometown.
    AllegianceChat,
    Hometown,
    /// Swear to the player selected in the world.
    Swear(u32, String),
    /// Release a vassal or break with the patron (asks first).
    Part(u32),
    /// Ignore allegiance requests, on or off.
    IgnoreAllegiance(bool),
    /// Start a tell to someone.
    Tell(String),
    /// Invite someone into the fellowship.
    Invite(u32),
    /// Ask someone to be a friend; stop being friends.
    Befriend(String),
    Unfriend(String),
    /// Manage allegiance (OPENAC): what the monarch asked for.
    Manage(ManageAsk),
    /// A fellow clicked: they are selected in the world too.
    SelectFellow(u32),
    /// Recruit the player selected in the world.
    Recruit(u32),
    /// Hand the lead on; dismiss a fellow.
    MakeLeader(u32),
    Dismiss(u32),
    /// Open or close recruiting.
    SetOpen(bool),
    /// Leave; disband (asks first).
    Leave,
    Disband,
    /// A fellowship switch flipped to this.
    Switch(FellowSwitch, bool),
    /// Start a fellowship with the name typed.
    Create,
    /// Add the friend typed.
    AddFriend,
    /// Appear offline, on or off.
    AppearOffline(bool),
    /// Ignore the name typed (a whole account when true).
    Ignore(bool),
    /// Stop ignoring someone (an account when true).
    Unignore(String, bool),
    /// Hide (or show again) a kind of message from someone (none: from
    /// everyone).
    Hide(Option<String>, u32, bool),
}

/// What Manage allegiance asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManageAsk {
    /// Name the allegiance what is typed.
    Rename,
    /// Post (or clear) the message of the day.
    PostMotd,
    ClearMotd,
    /// List the officers in chat; appoint the member typed to an office;
    /// take an office from them; give an office the title typed.
    ListOfficers,
    Appoint(u32),
    RemoveOffice,
    TitleOffice(u32),
    /// Lock or open the allegiance; approve the name typed.
    Lock(bool),
    Approve,
    /// The bans and allegiance chat.
    ListBans,
    Ban,
    LiftBan,
    Gag,
    PutOutOfChat,
}

/// Something that can be clicked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Hit {
    /// Something only the window does.
    Ask(SocialAsk),
    /// Show a page.
    Tab(Tab),
    /// Pick someone in the allegiance.
    Member(u32),
    /// Show or leave Manage allegiance.
    Manage(bool),
    /// Pick an office.
    Office(u32),
    /// Pick a fellow.
    Fellow(u32),
    /// Pick a friend.
    Friend(u32),
    /// Open the menu of what is hidden from an ignored row (none: from
    /// everyone).
    WhatMenu(Option<usize>),
    /// Nothing, but the click stays here (a card, a menu's edge).
    Swallow,
}

/// What is under the pointer, noted while drawing.
#[derive(Debug, Default)]
pub(super) struct Hits {
    pub list: Vec<(Rect, Hit)>,
    pub hover: Option<(f32, f32)>,
}

impl Hits {
    /// Notes `hit` at `r`; whether the pointer is over it.
    pub fn add(&mut self, r: Rect, hit: Hit) -> bool {
        self.list.push((r, hit));
        kit::hot(self.hover, r)
    }
}

/// The Social window's frame.
#[derive(Debug)]
pub struct SocialFrame {
    pub(super) theme: Horizon,
    pub(super) fonts: SocialFonts,
    /// What it shows.
    pub screen: SocialScreen,
    /// The page shown.
    pub tab: Tab,
    /// Manage allegiance shows over the allegiance.
    pub managing: bool,
    /// The person picked in the allegiance (0: the first vassal, else the
    /// player).
    pub member: u32,
    /// The office picked in Manage allegiance (1 to 3).
    pub office: u32,
    /// The fellow picked (0: none).
    pub fellow: u32,
    /// The friend picked (0: none).
    pub friend: u32,
    /// The ignored row whose menu shows (none: no menu; `Some(None)`:
    /// everyone's).
    pub menu: Option<Option<usize>>,
    /// Fellows show on screen.
    pub on_screen: bool,
    /// Which fields hold nothing (their hints show).
    pub empty: Vec<Slot>,
    /// The field with the keyboard.
    pub focused: Option<Slot>,
    /// What it asked for since the window last looked.
    pub asks: Vec<SocialAsk>,
    pub(super) hits: Hits,
    pub(super) scroll: [Scroll; 4],
    pub(super) totals: [(f32, Rect); 4],
}

impl SocialFrame {
    /// A frame in `theme`.
    #[must_use]
    pub fn new(theme: Horizon, fonts: SocialFonts) -> Self {
        Self {
            theme,
            fonts,
            screen: SocialScreen::default(),
            tab: Tab::Allegiance,
            managing: false,
            member: 0,
            office: 1,
            fellow: 0,
            friend: 0,
            menu: None,
            on_screen: true,
            empty: Slot::ALL.to_vec(),
            focused: None,
            asks: Vec::new(),
            hits: Hits::default(),
            scroll: [Scroll::default(); 4],
            totals: [(0.0, (0.0, 0.0, 0.0, 0.0)); 4],
        }
    }

    /// Shows page `tab` (Manage allegiance and any menu close).
    pub fn show(&mut self, tab: Tab) {
        if self.tab != tab {
            self.asks.push(SocialAsk::Tab(tab));
        }
        self.tab = tab;
        self.managing = false;
        self.menu = None;
    }

    /// Where field `slot` goes on the page shown (none: it hides).
    #[must_use]
    pub fn field_rect(&self, slot: Slot) -> Option<Rect> {
        match (self.tab, slot) {
            (Tab::Fellowship, Slot::FellowshipName) if self.screen.fellowship.is_none() => {
                Some(super::pages::fellowship_name_rect())
            }
            (Tab::Friends, Slot::FriendName) => Some(super::people::friend_field_rect()),
            (Tab::Ignored, Slot::IgnoreName) => Some(super::people::ignore_field_rect()),
            (Tab::Allegiance, s) if self.managing && self.monarch() => {
                super::pages::manage_field_rect(s)
            }
            _ => None,
        }
    }

    /// The player leads the allegiance (Manage allegiance is theirs).
    #[must_use]
    pub fn monarch(&self) -> bool {
        self.screen.allegiance.as_ref().is_some_and(|a| a.monarch)
    }

    /// Whether field `slot` holds nothing.
    pub(super) fn is_empty(&self, slot: Slot) -> bool {
        self.empty.contains(&slot)
    }

    fn click(&mut self, x: f32, y: f32) {
        let hit = self
            .hits
            .list
            .iter()
            .rev()
            .find(|(r, _)| inside(*r, x, y))
            .map(|(_, h)| h.clone());
        let menu_was = self.menu.take();
        let Some(hit) = hit else {
            return;
        };
        match hit {
            Hit::Ask(SocialAsk::Hide(who, kind, on)) => {
                // The menu stays open for another pick.
                self.menu = menu_was;
                self.asks.push(SocialAsk::Hide(who, kind, on));
            }
            Hit::Ask(ask) => {
                if let SocialAsk::Switch(FellowSwitch::OnScreen, on) = ask {
                    self.on_screen = on;
                }
                if let SocialAsk::Unfriend(_) = ask {
                    self.friend = 0;
                }
                self.asks.push(ask);
            }
            Hit::Tab(tab) => self.show(tab),
            Hit::Member(id) => self.member = id,
            Hit::Manage(on) => {
                self.managing = on;
            }
            Hit::Office(level) => self.office = level,
            Hit::Fellow(id) => {
                self.fellow = id;
                self.asks.push(SocialAsk::SelectFellow(id));
            }
            Hit::Friend(id) => self.friend = if self.friend == id { 0 } else { id },
            Hit::WhatMenu(row) => {
                self.menu = if menu_was == Some(row) {
                    None
                } else {
                    Some(row)
                };
            }
            Hit::Swallow => self.menu = menu_was,
        }
    }

    fn draw_frame(&self, rc: &mut RenderContext<'_>, h: &mut Hits) {
        let theme = &Horizon::current();
        let f = &self.fonts;
        kit::window_back(rc, theme, WIDTH, HEIGHT, HEADER, false);
        let right = kit::title(rc, theme, f.title.as_ref(), PEOPLE, "Social", HEADER);
        if let Some(s) = &f.small {
            ac_ui::modern::text::draw(
                rc,
                s,
                "O",
                right + 10.0,
                ((HEADER - s.line_height) * 0.5).round(),
                ac_ui::modern::text::TextStyle::plain(Horizon::current().palette.muted),
            );
        }
        let close = kit::close_rect(WIDTH, HEADER);
        let lit = h.add(close, Hit::Ask(SocialAsk::Close));
        kit::close(rc, theme, close, lit);
        // The tabs.
        rc.fill(0.0, BODY - 1.0, WIDTH, 1.0, kit::bronze(0.35));
        let mut x = 16.0;
        for (i, tab) in Tab::ALL.into_iter().enumerate() {
            let t = kit::Tab {
                words: tab.words(),
                count: self.screen.counts[i].clone(),
                on: self.tab == tab,
                red: false,
            };
            let w = kit::tab_width(&t, f.strong.as_ref(), f.label.as_ref());
            let r = (x, HEADER, w, TABS);
            let lit = h.add(r, Hit::Tab(tab));
            kit::draw_tab(rc, theme, &t, r, f.strong.as_ref(), f.label.as_ref(), lit);
            x += w + 4.0;
        }
    }
}

impl Widget for SocialFrame {
    fn hit_test(&self, _element: &Element, _x: f32, _y: f32) -> bool {
        true
    }

    fn handles_click(&self, _element: &Element) -> bool {
        true
    }

    fn receives_hover_mouse_move(&self) -> bool {
        true
    }

    fn on_event(&mut self, _ui: &mut UiRoot, me: ElementId, e: &UiEvent) -> bool {
        if e.target != Some(me) {
            return false;
        }
        let (x, y) = (e.data1 as f32, e.data2 as f32);
        match e.kind {
            kind::MOUSE_MOVE | kind::HOVER_ENTER => {
                self.hits.hover = Some((x, y));
                false
            }
            kind::HOVER_LEAVE => {
                self.hits.hover = None;
                false
            }
            kind::CLICK => {
                self.click(x, y);
                true
            }
            kind::RIGHT_CLICK => true,
            kind::SCROLL => {
                let Some((hx, hy)) = self.hits.hover else {
                    return false;
                };
                for (i, (total, room)) in self.totals.iter().enumerate() {
                    if inside(*room, hx, hy) {
                        self.scroll[i].wheel(e.data0, 46.0, *total, room.3);
                        return true;
                    }
                }
                false
            }
            _ => false,
        }
    }

    fn draw(&mut self, _ui: &mut UiRoot, _me: ElementId, rc: &mut RenderContext<'_>) {
        let mut h = Hits {
            list: Vec::new(),
            hover: self.hits.hover,
        };
        self.draw_frame(rc, &mut h);
        match self.tab {
            Tab::Allegiance if self.managing && self.monarch() => self.draw_manage(rc, &mut h),
            Tab::Allegiance => self.draw_allegiance(rc, &mut h),
            Tab::Fellowship => self.draw_fellowship(rc, &mut h),
            Tab::Friends => self.draw_friends(rc, &mut h),
            Tab::Ignored => self.draw_ignored(rc, &mut h),
        }
        self.hits = h;
    }
}
