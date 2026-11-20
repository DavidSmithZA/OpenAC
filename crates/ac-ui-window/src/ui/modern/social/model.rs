//! What the Social window shows, worked out from the session's social
//! facts: the allegiance (its line of fealty, the vassals and a card for
//! anyone in it), the fellowship (each fellow's share and vitals, who may
//! still rejoin), the friends with what else they are to the player, and
//! who is ignored with what is hidden from each.

use std::collections::HashSet;

use ac_core::social::SquelchInfo;
use ac_proto::allegiance::{AllegianceMember, AllegianceProfile};
use ac_runtime::gameplay::{AllegianceCommand, FellowshipView};
use ac_runtime::inventory::ObjectRef;
use ac_runtime::session::interface::SocialFacts;
use ac_ui::modern::numbers::{compact, group_thousands};
use ac_ui::panels::character::heritage_name;
use ac_ui::panels::social::allegiance::full_name;
use ac_ui::panels::social::fellowship::experience_share;

/// The most friends, and the most people ignored, the server keeps.
pub const MOST_LISTED: usize = 50;

/// The most people in a fellowship.
pub const MOST_FELLOWS: usize = 9;

/// How long someone who left a locked fellowship may rejoin, seconds.
const REJOIN_SECONDS: i64 = ac_runtime::gameplay::fellowship::DEPARTED_GRACE_SECONDS;

/// The window's pages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Tab {
    /// The allegiance.
    #[default]
    Allegiance,
    /// The fellowship.
    Fellowship,
    /// The friends list.
    Friends,
    /// The ignored list.
    Ignored,
}

impl Tab {
    /// Every page, in the tab row's order.
    pub const ALL: [Self; 4] = [
        Self::Allegiance,
        Self::Fellowship,
        Self::Friends,
        Self::Ignored,
    ];

    /// The tab's words.
    #[must_use]
    pub const fn words(self) -> &'static str {
        match self {
            Self::Allegiance => "Allegiance",
            Self::Fellowship => "Fellowship",
            Self::Friends => "Friends",
            Self::Ignored => "Ignored",
        }
    }
}

/// A card in the line of fealty (monarch, patron, the player).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChainCard {
    /// Who.
    pub id: u32,
    /// "MONARCH", "YOUR PATRON", "PATRON AND MONARCH", "YOU" (or "YOU ·
    /// MONARCH").
    pub role: &'static str,
    /// Title and name.
    pub full: String,
    /// Online.
    pub online: bool,
    /// The quiet words on the right.
    pub side: String,
}

/// A vassal's row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VassalRow {
    /// Who.
    pub id: u32,
    /// Title and name.
    pub full: String,
    /// Level, rank and heritage.
    pub sub: String,
    /// The experience they passed up, short ("812K").
    pub passed: String,
    /// Online.
    pub online: bool,
}

/// What breaking with someone in the allegiance asks first, and does.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Parting {
    /// The button's words ("Release Hask").
    pub button: String,
    /// The question's title and words.
    pub title: String,
    pub text: String,
    /// The yes button's words.
    pub yes: String,
    /// What a yes sends.
    pub command: AllegianceCommand,
}

/// The card for anyone in the allegiance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemberCard {
    /// Who.
    pub id: u32,
    /// Their plain name (for a tell or a friend request).
    pub name: String,
    /// Title and name.
    pub full: String,
    /// "Online", "Offline" or "You".
    pub status: &'static str,
    /// Online.
    pub online: bool,
    /// Label and value.
    pub rows: Vec<(String, String)>,
    /// Someone else: Tell, Invite and Add friend show.
    pub other: bool,
    /// They are already a friend.
    pub friend: bool,
    /// Release (a vassal) or break (the patron).
    pub parting: Option<Parting>,
}

/// The allegiance as the window shows it.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AllegianceScreen {
    /// Its name.
    pub name: String,
    /// How many are in it and follow the player.
    pub sub: String,
    /// The message of the day and who set it.
    pub motd: Option<(String, String)>,
    /// Monarch, patron, the player.
    pub chain: Vec<ChainCard>,
    /// The player's direct vassals.
    pub vassals: Vec<VassalRow>,
    /// A card for each person shown.
    pub cards: Vec<MemberCard>,
    /// The player is the monarch (Manage allegiance shows).
    pub monarch: bool,
    /// Nobody new may swear in.
    pub locked: bool,
}

impl AllegianceScreen {
    /// The card for `id`.
    #[must_use]
    pub fn card(&self, id: u32) -> Option<&MemberCard> {
        self.cards.iter().find(|c| c.id == id)
    }
}

/// One fellow's row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FellowRow {
    /// Who.
    pub id: u32,
    /// Their name.
    pub name: String,
    /// Their level.
    pub level: u32,
    /// Their share of the experience ("22%").
    pub share: String,
    /// The player.
    pub you: bool,
    /// The leader.
    pub leader: bool,
    /// Health, stamina and mana, now over most.
    pub vitals: [(u32, u32); 3],
    /// They share their loot.
    pub loot: bool,
}

/// The fellowship as the window shows it.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FellowshipScreen {
    /// Its name.
    pub name: String,
    /// Who leads, how many, how experience is shared.
    pub sub: String,
    /// Anyone in it may recruit.
    pub open: bool,
    /// Nobody new may join.
    pub locked: bool,
    /// The fellows, in the fellowship's order.
    pub rows: Vec<FellowRow>,
    /// Who left the locked fellowship and may still rejoin, worded.
    pub departed: Vec<(String, String)>,
    /// The player leads it.
    pub lead: bool,
    /// The leader's name.
    pub leader: String,
    /// It is full.
    pub full: bool,
}

/// A friend's row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FriendRow {
    /// Who.
    pub id: u32,
    /// Their name.
    pub name: String,
    /// Online.
    pub online: bool,
    /// What else they are to the player (Patron, Fellowship...).
    pub tags: Vec<&'static str>,
}

/// One kind of message that can be hidden.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Hideable {
    /// The game's number for it.
    pub kind: u32,
    /// What the player calls it.
    pub words: &'static str,
    /// Its name in the `@squelch` and `@filter` commands.
    pub command: &'static str,
}

/// What can be hidden from someone, in the menu's order ("Everything"
/// first).
pub const HIDEABLE: [Hideable; 8] = [
    Hideable {
        kind: 1,
        words: "Everything",
        command: "All",
    },
    Hideable {
        kind: 2,
        words: "Speech nearby",
        command: "Speech",
    },
    Hideable {
        kind: 3,
        words: "Tells",
        command: "Tell",
    },
    Hideable {
        kind: 12,
        words: "Emotes",
        command: "Emote",
    },
    Hideable {
        kind: 17,
        words: "Spell words",
        command: "Spellcasting",
    },
    Hideable {
        kind: 18,
        words: "Allegiance chat",
        command: "Allegiance",
    },
    Hideable {
        kind: 19,
        words: "Fellowship chat",
        command: "Fellowship",
    },
    Hideable {
        kind: 6,
        words: "Combat messages",
        command: "Combat",
    },
];

/// The other kinds the server may hold, by their command names.
const OTHER_KINDS: [(u32, &str); 7] = [
    (7, "Magic"),
    (16, "Appraisal"),
    (21, "Enemy combat"),
    (22, "Own combat"),
    (23, "Recall"),
    (24, "Craft"),
    (25, "Salvaging"),
];

/// Someone ignored.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IgnoredRow {
    /// Their name.
    pub name: String,
    /// The whole account (every character on it).
    pub account: bool,
    /// "Character" or "Whole account: every character on it".
    pub scope: &'static str,
    /// What is hidden, worded.
    pub what: String,
    /// The kinds hidden.
    pub kinds: HashSet<u32>,
}

/// The friends and ignored lists, and the player's own switches.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PeopleScreen {
    /// Friends online, then offline, each by name.
    pub online: Vec<FriendRow>,
    pub offline: Vec<FriendRow>,
    /// Characters then accounts.
    pub ignored: Vec<IgnoredRow>,
    /// What is hidden from everyone, worded, and its kinds.
    pub everyone: String,
    pub everyone_kinds: HashSet<u32>,
}

/// Everything the window shows.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SocialScreen {
    /// The tabs' counts.
    pub counts: [Option<String>; 4],
    /// The allegiance (none: the player has none).
    pub allegiance: Option<AllegianceScreen>,
    /// The fellowship (none: the player is in none).
    pub fellowship: Option<FellowshipScreen>,
    /// Friends and the ignored.
    pub people: PeopleScreen,
    /// The player selected in the world (id, name), not the player.
    pub selected: Option<(u32, String)>,
    /// Ignore allegiance requests; the fellowship switches (share loot,
    /// ignore requests, accept automatically); appear offline; share
    /// experience in a new fellowship.
    pub ignore_allegiance: bool,
    pub share_loot: bool,
    pub ignore_fellowship: bool,
    pub auto_accept: bool,
    pub appear_offline: bool,
    pub share_xp: bool,
}

/// What the screen is worked out from.
pub struct SocialInput<'a> {
    /// The player's id.
    pub player: u32,
    /// The social facts.
    pub facts: &'a SocialFacts,
    /// The selected object, when it is another player (id, name).
    pub selected: Option<(u32, String)>,
    /// The experience a level needs to raise (the fellowship's uneven
    /// split).
    pub to_raise: Option<&'a dyn Fn(u32) -> i64>,
    /// Now, Unix seconds.
    pub now_unix_s: i64,
}

impl std::fmt::Debug for SocialInput<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SocialInput")
            .field("player", &self.player)
            .field("selected", &self.selected)
            .finish_non_exhaustive()
    }
}

/// `n` with its thousands grouped.
fn grouped(n: u64) -> String {
    group_thousands(i64::try_from(n).unwrap_or(i64::MAX))
}

/// The screen for `input`.
#[must_use]
pub fn screen(input: &SocialInput<'_>) -> SocialScreen {
    use ac_runtime::character::option_id as o;
    let facts = input.facts;
    let options = &facts.options;
    let friends: HashSet<u32> = facts.friends.entries().iter().map(|f| f.id).collect();
    let allegiance = facts
        .allegiance
        .as_ref()
        .filter(|p| p.find_data(input.player).is_some())
        .map(|p| allegiance_screen(p, input.player, &friends));
    let fellowship = facts
        .fellowship
        .in_fellowship
        .then(|| fellowship_screen(&facts.fellowship, input));
    let people = people_screen(facts, input.player);
    let online = people.online.len();
    let ignored = people.ignored.len();
    let counts = [
        None,
        fellowship
            .as_ref()
            .map(|f| format!("{} of {MOST_FELLOWS}", f.rows.len())),
        (online > 0).then(|| format!("{online} online")),
        (ignored > 0).then(|| ignored.to_string()),
    ];
    SocialScreen {
        counts,
        allegiance,
        fellowship,
        people,
        selected: input.selected.clone(),
        ignore_allegiance: options.get(o::IGNORE_ALLEGIANCE_REQUESTS),
        share_loot: options.get(o::FELLOWSHIP_SHARE_LOOT),
        ignore_fellowship: options.get(o::IGNORE_FELLOWSHIP_REQUESTS),
        auto_accept: options.get(o::FELLOWSHIP_AUTO_ACCEPT_REQUESTS),
        appear_offline: options.get(o::APPEAR_OFFLINE),
        share_xp: options.get(o::FELLOWSHIP_SHARE_XP),
    }
}

fn heritage(member: &AllegianceMember) -> String {
    heritage_name(i32::from(member.heritage_group))
        .unwrap_or("")
        .to_owned()
}

fn level_rank(member: &AllegianceMember) -> String {
    let mut parts = Vec::new();
    if member.level > 0 {
        parts.push(format!("Level {}", member.level));
    }
    parts.push(format!("rank {}", member.rank));
    let h = heritage(member);
    if !h.is_empty() {
        parts.push(h);
    }
    parts.join(" · ")
}

/// The rows every card starts with: level, rank, heritage, loyalty and
/// leadership.
fn common_rows(m: &AllegianceMember) -> Vec<(String, String)> {
    let mut rows = Vec::new();
    if m.level > 0 {
        rows.push(("Level".to_owned(), m.level.to_string()));
    }
    rows.push(("Rank".to_owned(), m.rank.to_string()));
    let h = heritage(m);
    if !h.is_empty() {
        rows.push(("Heritage".to_owned(), h));
    }
    rows.push((
        "Loyalty · leadership".to_owned(),
        format!("{} · {}", m.loyalty, m.leadership),
    ));
    rows
}

fn status(online: bool) -> &'static str {
    if online { "Online" } else { "Offline" }
}

#[allow(clippy::too_many_lines)]
fn allegiance_screen(p: &AllegianceProfile, me: u32, friends: &HashSet<u32>) -> AllegianceScreen {
    let own = p.find_data(me);
    let monarch = p.monarch.as_ref();
    let patron = p.find_patron(me);
    let mut vassals: Vec<&AllegianceMember> = p.find_vassals(me).collect();
    // The highest ranks first, as the line of fealty reads.
    vassals.sort_by(|a, b| {
        b.rank
            .cmp(&a.rank)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    let followers = p.total_vassals;
    let i_am_monarch = monarch.is_some_and(|m| m.character_id == me);
    let total = p.total_members;
    let sub = if followers == 0 {
        format!("{} members", grouped(u64::from(total)))
    } else {
        format!(
            "{} members · {} follow you, {} of them sworn to you directly",
            grouped(u64::from(total)),
            grouped(u64::from(followers)),
            vassals.len()
        )
    };
    let motd = (!p.motd.trim().is_empty()).then(|| (p.motd.clone(), p.motd_set_by.clone()));
    let mut chain = Vec::new();
    let mut cards = Vec::new();
    let tithed = own.map_or(0, |o| o.cp_tithed);
    let other_card = |m: &AllegianceMember, rows: Vec<(String, String)>, parting| MemberCard {
        id: m.character_id,
        name: m.name.clone(),
        full: full_name(m),
        status: status(m.is_logged_in),
        online: m.is_logged_in,
        rows,
        other: true,
        friend: friends.contains(&m.character_id),
        parting,
    };
    let breaking = |m: &AllegianceMember| Parting {
        button: format!("Break allegiance with {}", m.name),
        title: format!("Break allegiance with {}?", m.name),
        text: if followers > 0 {
            format!(
                "You leave {}'s allegiance, and your {} followers come with you. Your experience stops passing up to {}.",
                m.name,
                grouped(u64::from(followers)),
                m.name
            )
        } else {
            format!(
                "You leave {}'s allegiance. Your experience stops passing up to {}.",
                m.name, m.name
            )
        },
        yes: "Break allegiance".to_owned(),
        command: AllegianceCommand::Break(ObjectRef::Guid(m.character_id)),
    };
    if let Some(m) = monarch.filter(|m| m.character_id != me) {
        let is_patron = patron.is_some_and(|pa| pa.character_id == m.character_id);
        let mut rows = common_rows(m);
        rows.push((
            "Followers".to_owned(),
            grouped(u64::from(total.saturating_sub(1))),
        ));
        if is_patron {
            rows.push((
                "You passed up".to_owned(),
                format!("{} XP", grouped(u64::from(tithed))),
            ));
        }
        chain.push(ChainCard {
            id: m.character_id,
            role: if is_patron {
                "PATRON AND MONARCH"
            } else {
                "MONARCH"
            },
            full: full_name(m),
            online: m.is_logged_in,
            side: if is_patron && tithed > 0 {
                format!("you pass up {}", compact(i64::from(tithed)))
            } else {
                format!("{} members", grouped(u64::from(total)))
            },
        });
        cards.push(other_card(m, rows, is_patron.then(|| breaking(m))));
    }
    if let Some(pa) = patron.filter(|pa| monarch.is_none_or(|m| m.character_id != pa.character_id))
    {
        let mut rows = common_rows(pa);
        rows.push((
            "You passed up".to_owned(),
            format!("{} XP", grouped(u64::from(tithed))),
        ));
        chain.push(ChainCard {
            id: pa.character_id,
            role: "YOUR PATRON",
            full: full_name(pa),
            online: pa.is_logged_in,
            side: if tithed > 0 {
                format!("you pass up {}", compact(i64::from(tithed)))
            } else {
                String::new()
            },
        });
        cards.push(other_card(pa, rows, Some(breaking(pa))));
    }
    if let Some(o) = own {
        let mut rows = common_rows(o);
        rows.push((
            "Followers".to_owned(),
            format!(
                "{} direct · {} in all",
                vassals.len(),
                grouped(u64::from(followers))
            ),
        ));
        let passed: u64 = vassals.iter().map(|v| u64::from(v.cp_tithed)).sum();
        if passed > 0 {
            rows.push((
                "Passed up to you".to_owned(),
                format!("{} XP", grouped(passed)),
            ));
        }
        chain.push(ChainCard {
            id: me,
            role: if i_am_monarch {
                "YOU · MONARCH"
            } else {
                "YOU"
            },
            full: full_name(o),
            online: true,
            side: format!("rank {}", o.rank),
        });
        cards.push(MemberCard {
            id: me,
            name: o.name.clone(),
            full: full_name(o),
            status: "You",
            online: true,
            rows,
            other: false,
            friend: false,
            parting: None,
        });
    }
    let mut rows = Vec::new();
    for v in &vassals {
        rows.push(VassalRow {
            id: v.character_id,
            full: full_name(v),
            sub: level_rank(v),
            passed: compact(i64::from(v.cp_tithed)),
            online: v.is_logged_in,
        });
        let mut card_rows = common_rows(v);
        card_rows.push((
            "Passed up to you".to_owned(),
            format!("{} XP", grouped(u64::from(v.cp_tithed))),
        ));
        let followers_go = format!(
            "{} leaves your allegiance, along with anyone sworn to {}. The experience {} passes up to you stops.",
            v.name, v.name, v.name
        );
        cards.push(other_card(
            v,
            card_rows,
            Some(Parting {
                button: format!("Release {}", v.name),
                title: format!("Release {}?", v.name),
                text: followers_go,
                yes: format!("Release {}", v.name),
                command: AllegianceCommand::Kick(ObjectRef::Guid(v.character_id)),
            }),
        ));
    }
    AllegianceScreen {
        name: if p.allegiance_name.is_empty() {
            monarch.map_or_else(
                || "Your allegiance".to_owned(),
                |m| format!("{}'s allegiance", m.name),
            )
        } else {
            p.allegiance_name.clone()
        },
        sub,
        motd,
        chain,
        vassals: rows,
        cards,
        monarch: i_am_monarch,
        locked: p.is_locked,
    }
}

fn fellowship_screen(f: &FellowshipView, input: &SocialInput<'_>) -> FellowshipScreen {
    let me = input.player;
    let leader = f
        .members
        .iter()
        .find(|m| m.guid == f.leader_guid)
        .map(|m| m.name.clone())
        .unwrap_or_default();
    let lead = f.leader_guid == me && me != 0;
    let shared = if !f.share_xp {
        "experience not shared"
    } else if f.even_xp_split {
        "experience shared evenly"
    } else {
        "experience shared by level"
    };
    let who = if lead {
        "You lead".to_owned()
    } else if leader.is_empty() {
        "No leader".to_owned()
    } else {
        format!("{leader} leads")
    };
    let rows = f
        .members
        .iter()
        .map(|m| {
            let share = experience_share(m, f, input.to_raise);
            // Truncated, as the game prints it.
            #[allow(clippy::cast_possible_truncation)]
            let percent = (share * 100.0) as i32;
            FellowRow {
                id: m.guid,
                name: m.name.clone(),
                level: m.level,
                share: format!("{percent}%"),
                you: m.guid == me,
                leader: m.guid == f.leader_guid,
                vitals: [m.health, m.stamina, m.mana],
                loot: m.share_loot,
            }
        })
        .collect::<Vec<_>>();
    let departed = f
        .departed
        .iter()
        .filter(|d| input.now_unix_s - d.left_unix_s <= REJOIN_SECONDS)
        .map(|d| {
            let minutes = ((input.now_unix_s - d.left_unix_s).max(0) / 60).max(1);
            let name = if d.name.is_empty() {
                "Someone".to_owned()
            } else {
                d.name.clone()
            };
            let ago = if minutes == 1 {
                "a minute ago".to_owned()
            } else {
                format!("{minutes} minutes ago")
            };
            (
                name,
                format!("left {ago} and can still rejoin while the fellowship is locked."),
            )
        })
        .collect();
    FellowshipScreen {
        name: f.name.clone(),
        sub: format!("{who} · {} of {MOST_FELLOWS} · {shared}", rows.len()),
        open: f.open,
        locked: f.locked,
        full: rows.len() >= MOST_FELLOWS,
        rows,
        departed,
        lead,
        leader,
    }
}

/// What `kinds` hides, worded.
#[must_use]
pub fn hidden_words(kinds: &HashSet<u32>) -> String {
    if kinds.contains(&1) {
        return "Everything".to_owned();
    }
    let mut words: Vec<&str> = HIDEABLE
        .iter()
        .filter(|h| kinds.contains(&h.kind))
        .map(|h| h.words)
        .collect();
    words.extend(
        OTHER_KINDS
            .iter()
            .filter(|(k, _)| kinds.contains(k))
            .map(|(_, w)| *w),
    );
    if words.is_empty() {
        return "Nothing".to_owned();
    }
    let mut out = words.join(", ");
    if let Some(first) = out.get(..1) {
        let rest = out[1..].to_lowercase();
        out = format!("{first}{rest}");
    }
    out
}

fn people_screen(facts: &SocialFacts, me: u32) -> PeopleScreen {
    let allegiance = facts.allegiance.as_ref();
    let patron = allegiance
        .and_then(|p| p.find_patron(me))
        .map(|m| m.character_id);
    let monarch = allegiance
        .and_then(|p| p.monarch.as_ref())
        .map(|m| m.character_id)
        .filter(|m| *m != me);
    let vassals: HashSet<u32> = allegiance
        .map(|p| p.find_vassals(me).map(|v| v.character_id).collect())
        .unwrap_or_default();
    let fellows: HashSet<u32> = facts.fellowship.members.iter().map(|m| m.guid).collect();
    let mut online = Vec::new();
    let mut offline = Vec::new();
    for f in facts.friends.entries() {
        let mut tags = Vec::new();
        if Some(f.id) == monarch && Some(f.id) == patron {
            tags.push("Patron and monarch");
        } else if Some(f.id) == monarch {
            tags.push("Monarch");
        } else if Some(f.id) == patron {
            tags.push("Patron");
        }
        if vassals.contains(&f.id) {
            tags.push("Vassal");
        }
        if fellows.contains(&f.id) {
            tags.push("Fellowship");
        }
        let row = FriendRow {
            id: f.id,
            name: f.name.clone(),
            online: f.online,
            tags,
        };
        if f.online {
            online.push(row);
        } else {
            offline.push(row);
        }
    }
    let by_name = |a: &FriendRow, b: &FriendRow| a.name.to_lowercase().cmp(&b.name.to_lowercase());
    online.sort_by(by_name);
    offline.sort_by(by_name);
    let db = facts.squelch.database();
    let mut characters: Vec<&SquelchInfo> = db
        .characters
        .values()
        .filter(|i| !i.message_types.is_empty())
        .collect();
    characters.sort_by_key(|i| i.name.to_lowercase());
    let mut ignored: Vec<IgnoredRow> = characters
        .into_iter()
        .map(|i| IgnoredRow {
            name: i.name.clone(),
            account: false,
            scope: "Character",
            what: hidden_words(&i.message_types),
            kinds: i.message_types.clone(),
        })
        .collect();
    let mut accounts: Vec<&String> = db.accounts.keys().collect();
    accounts.sort_by_key(|a| a.to_lowercase());
    ignored.extend(accounts.into_iter().map(|name| IgnoredRow {
        name: name.clone(),
        account: true,
        scope: "Whole account: every character on it",
        what: "Everything".to_owned(),
        kinds: HashSet::from([1]),
    }));
    PeopleScreen {
        online,
        offline,
        ignored,
        everyone: hidden_words(&db.global.message_types),
        everyone_kinds: db.global.message_types.clone(),
    }
}

/// The `@squelch` (or `@unsquelch`) line that hides (or shows again)
/// `kind` from character `name`.
#[must_use]
pub fn squelch_line(name: &str, kind: &Hideable, hide: bool) -> String {
    let verb = if hide { "squelch" } else { "unsquelch" };
    format!("@{verb} -{} {name}", kind.command)
}

/// The `@filter` (or `@unfilter`) line that hides (or shows again) `kind`
/// from everyone.
#[must_use]
pub fn filter_line(kind: &Hideable, hide: bool) -> String {
    let verb = if hide { "filter" } else { "unfilter" };
    format!("@{verb} -{}", kind.command)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ac_core::social::{FriendEntry, FriendsStateParts, SquelchDatabase, SquelchStateParts};
    use ac_runtime::gameplay::{DepartedFellowView, FellowView};

    const ME: u32 = 0x5000_0001;
    const PATRON: u32 = 0x5000_0002;
    const MONARCH: u32 = 0x5000_0003;
    const VASSAL: u32 = 0x5000_0004;

    fn member(id: u32, parent: u32, name: &str, rank: u16) -> AllegianceMember {
        AllegianceMember {
            character_id: id,
            parent_guid: parent,
            is_logged_in: id != VASSAL,
            name: name.to_owned(),
            rank,
            level: 100,
            loyalty: 10,
            leadership: 20,
            cp_cached: 0,
            cp_tithed: 812_400,
            gender: 1,
            heritage_group: 1,
            may_passup_experience: true,
        }
    }

    fn facts() -> SocialFacts {
        let profile = AllegianceProfile {
            total_members: 1284,
            total_vassals: 19,
            motd: "Mana forge run on Friday.".to_owned(),
            motd_set_by: "Rook".to_owned(),
            allegiance_name: "House of Arwic".to_owned(),
            monarch: Some(member(MONARCH, 0, "Rook", 9)),
            records: vec![
                member(PATRON, MONARCH, "Mira", 7),
                member(ME, PATRON, "Aerin", 6),
                member(VASSAL, ME, "Hask", 3),
            ],
            ..AllegianceProfile::default()
        };
        SocialFacts {
            in_world: true,
            options: ac_runtime::character::CharacterOptions::default(),
            friends: FriendsStateParts {
                entries: vec![
                    FriendEntry {
                        id: PATRON,
                        name: "Mira".to_owned(),
                        online: true,
                        appear_offline: false,
                        friends: Vec::new(),
                        friend_of: Vec::new(),
                    },
                    FriendEntry {
                        id: 0x5000_0009,
                        name: "Corvan".to_owned(),
                        online: false,
                        appear_offline: false,
                        friends: Vec::new(),
                        friend_of: Vec::new(),
                    },
                ],
                revision: 1,
            }
            .into(),
            squelch: SquelchStateParts {
                database: SquelchDatabase {
                    characters: std::collections::HashMap::from([(
                        0x5000_0010,
                        SquelchInfo {
                            name: "Loudmouth".to_owned(),
                            account_wide: false,
                            message_types: HashSet::from([3, 12]),
                        },
                    )]),
                    ..SquelchDatabase::default()
                },
                revision: 1,
            }
            .into(),
            fellowship: FellowshipView::default(),
            allegiance: Some(profile),
            allegiance_rank: 6,
            allegiance_revision: 1,
        }
    }

    fn input(facts: &SocialFacts) -> SocialInput<'_> {
        SocialInput {
            player: ME,
            facts,
            selected: None,
            to_raise: None,
            now_unix_s: 1_000_000,
        }
    }

    #[test]
    fn the_line_of_fealty_runs_monarch_patron_you_with_the_vassals_below() {
        let f = facts();
        let s = screen(&input(&f));
        let a = s.allegiance.expect("an allegiance");
        let roles: Vec<&str> = a.chain.iter().map(|c| c.role).collect();
        assert_eq!(roles, ["MONARCH", "YOUR PATRON", "YOU"]);
        assert_eq!(a.name, "House of Arwic");
        assert_eq!(
            a.sub,
            "1,284 members · 19 follow you, 1 of them sworn to you directly"
        );
        assert_eq!(a.vassals.len(), 1);
        assert_eq!(a.vassals[0].passed, "812K");
        assert!(!a.vassals[0].online);
        assert!(!a.monarch);
    }

    #[test]
    fn a_vassal_is_released_and_the_patron_broken_with_after_a_question() {
        let f = facts();
        let a = screen(&input(&f)).allegiance.unwrap();
        let v = a.card(VASSAL).unwrap().parting.clone().unwrap();
        assert_eq!(v.button, "Release Hask");
        assert_eq!(v.command, AllegianceCommand::Kick(ObjectRef::Guid(VASSAL)));
        let p = a.card(PATRON).unwrap();
        assert!(p.friend, "Mira is a friend");
        assert_eq!(
            p.parting.as_ref().unwrap().command,
            AllegianceCommand::Break(ObjectRef::Guid(PATRON))
        );
        assert!(a.card(MONARCH).unwrap().parting.is_none());
        assert!(a.card(ME).unwrap().parting.is_none());
        assert_eq!(a.card(ME).unwrap().status, "You");
    }

    #[test]
    fn without_a_record_of_the_player_there_is_no_allegiance() {
        let mut f = facts();
        f.allegiance
            .as_mut()
            .unwrap()
            .records
            .retain(|r| r.character_id != ME);
        assert!(screen(&input(&f)).allegiance.is_none());
    }

    #[test]
    fn friends_come_online_first_with_what_else_they_are() {
        let f = facts();
        let s = screen(&input(&f));
        assert_eq!(s.people.online.len(), 1);
        assert_eq!(s.people.online[0].tags, ["Patron"]);
        assert_eq!(s.people.offline[0].name, "Corvan");
        assert_eq!(s.counts[2].as_deref(), Some("1 online"));
    }

    #[test]
    fn what_is_hidden_is_worded_everything_first() {
        assert_eq!(hidden_words(&HashSet::from([3, 12])), "Tells, emotes");
        assert_eq!(hidden_words(&HashSet::from([1, 3])), "Everything");
        assert_eq!(hidden_words(&HashSet::new()), "Nothing");
        let f = facts();
        let s = screen(&input(&f));
        assert_eq!(s.people.ignored[0].what, "Tells, emotes");
        assert_eq!(s.counts[3].as_deref(), Some("1"));
    }

    #[test]
    fn hiding_one_kind_is_the_squelch_command_the_chat_box_takes() {
        let tells = HIDEABLE[2];
        assert_eq!(
            squelch_line("Lou Mouth", &tells, true),
            "@squelch -Tell Lou Mouth"
        );
        assert_eq!(squelch_line("Lou", &tells, false), "@unsquelch -Tell Lou");
        assert_eq!(filter_line(&HIDEABLE[4], true), "@filter -Spellcasting");
    }

    #[test]
    fn a_fellowship_shows_shares_leader_and_who_may_rejoin() {
        let mut f = facts();
        let fellow = |guid, name: &str, level| FellowView {
            guid,
            name: name.to_owned(),
            level,
            health: (10, 20),
            stamina: (5, 5),
            mana: (1, 2),
            share_loot: true,
            vitals_age_seconds: None,
        };
        f.fellowship = FellowshipView {
            in_fellowship: true,
            name: "Ramp Runners".to_owned(),
            leader_guid: PATRON,
            share_xp: true,
            even_xp_split: true,
            open: true,
            locked: true,
            members: vec![fellow(PATRON, "Mira", 201), fellow(ME, "Aerin", 126)],
            departed: vec![DepartedFellowView {
                guid: 7,
                name: "Pell".to_owned(),
                left_unix_s: 1_000_000 - 12 * 60,
            }],
            ..FellowshipView::default()
        };
        let s = screen(&input(&f));
        let fel = s.fellowship.unwrap();
        assert_eq!(fel.sub, "Mira leads · 2 of 9 · experience shared evenly");
        assert!(fel.rows[0].leader && !fel.rows[0].you);
        assert!(fel.rows[1].you);
        assert!(!fel.lead);
        assert_eq!(
            fel.departed,
            [(
                "Pell".to_owned(),
                "left 12 minutes ago and can still rejoin while the fellowship is locked."
                    .to_owned()
            )]
        );
        assert_eq!(s.counts[1].as_deref(), Some("2 of 9"));
    }
}
