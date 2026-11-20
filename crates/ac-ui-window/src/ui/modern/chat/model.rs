//! What the modern chat shows of each line, and its tabs.
//!
//! A line is shown as an optional time, a channel tag (TELL, FELLOW,
//! ALLEG, GENERAL, TRADE, LFG, ...), the speaker's name and the words, in
//! the channel's colour; lines with no speaker (system, combat, magic,
//! experience, emotes) show their text as the game words it. Each tab
//! shows the text classes its filter lets through (the same masks
//! retail's chat windows use) and talks on its own channel, if it has
//! one.

use ac_core::chat::{ChatKind, ChatSpanRole};
use ac_core::chat::{CombatLineKind, LogTextType, MAIN_WINDOW_DEFAULT_FILTER};
use ac_runtime::communication::{ChatChannelKind, ChatLine};
use ac_ui::draw::Color;
use ac_ui::modern::theme::hex;

/// What a line is, for its tag and colour.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LineKind {
    /// Speech nearby (or the player's own).
    Say,
    /// A tell to the player.
    Tell,
    /// A tell the player sent.
    TellOut,
    /// The fellowship.
    Fellow,
    /// The allegiance, monarch, patron or vassals.
    Alleg,
    /// The General room.
    General,
    /// The Trade room.
    Trade,
    /// Looking for group.
    Lfg,
    /// The Roleplay room.
    Roleplay,
    /// The society room.
    Society,
    /// Any other channel.
    Channel,
    /// An emote.
    Emote,
    /// Spells.
    Magic,
    /// The player's own hits (and evades).
    Hit,
    /// Hits the player took, attacks that failed.
    Hurt,
    /// Experience, levels, skills.
    Xp,
    /// Anything else the game says.
    System,
}

/// The line's text colour: the canvas's table.
const TEXT: Color = hex(0xF4_EBDC, 1.0);
const TELL: Color = hex(0xC9_A2FF, 1.0);
const FELLOW: Color = hex(0x7F_D08A, 1.0);
const ALLEG: Color = hex(0xF0_A04B, 1.0);
const ROOM: Color = hex(0x8C_C8F0, 1.0);
const SYSTEM: Color = hex(0xB8_C2CC, 1.0);
const EMOTE: Color = hex(0xD2_D2C8, 1.0);
const MAGIC: Color = hex(0x7F_B2FF, 1.0);
const HIT: Color = hex(0xFF_9A6B, 1.0);
const HURT: Color = hex(0xFF_8A8E, 1.0);
const XP: Color = hex(0xE0_B068, 1.0);

impl LineKind {
    /// The kind of `line`.
    #[must_use]
    pub fn of(line: &ChatLine) -> Self {
        let class = LogTextType::from_u32(line.log_text_type);
        if line.kind == ChatKind::Combat || line.combat_kind.is_some() {
            return match line.combat_kind {
                Some(CombatLineKind::Info) | None => Self::Hit,
                Some(CombatLineKind::Warning | CombatLineKind::Error) => Self::Hurt,
            };
        }
        match class {
            Some(LogTextType::Tell) => Self::Tell,
            Some(LogTextType::SpeechDirectSend) => Self::TellOut,
            Some(LogTextType::Fellowship) => Self::Fellow,
            Some(LogTextType::Allegiance) => Self::Alleg,
            Some(LogTextType::TurbineGeneral) => Self::General,
            Some(LogTextType::TurbineTrade) => Self::Trade,
            Some(LogTextType::TurbineLfg) => Self::Lfg,
            Some(LogTextType::TurbineRoleplay) => Self::Roleplay,
            Some(LogTextType::TurbineSociety) => Self::Society,
            Some(LogTextType::Emote) => Self::Emote,
            Some(LogTextType::Magic | LogTextType::Spellcasting) => Self::Magic,
            Some(LogTextType::Combat | LogTextType::CombatSelf) => Self::Hit,
            Some(LogTextType::CombatEnemy) => Self::Hurt,
            Some(LogTextType::Advancement) => Self::Xp,
            Some(LogTextType::Speech) => Self::Say,
            Some(LogTextType::Channel | LogTextType::ChannelSend) => Self::Channel,
            _ => match line.kind {
                ChatKind::Tell => Self::Tell,
                ChatKind::LocalSpeech | ChatKind::RangedSpeech => Self::Say,
                ChatKind::Emote | ChatKind::SoulEmote => Self::Emote,
                ChatKind::Channel => Self::Channel,
                _ => Self::System,
            },
        }
    }

    /// The small capitals before the name; none for nearby speech and for
    /// lines without a speaker.
    #[must_use]
    pub fn tag(self) -> Option<&'static str> {
        Some(match self {
            Self::Tell => "TELL",
            Self::TellOut => "YOU TELL",
            Self::Fellow => "FELLOW",
            Self::Alleg => "ALLEG",
            Self::General => "GENERAL",
            Self::Trade => "TRADE",
            Self::Lfg => "LFG",
            Self::Roleplay => "RP",
            Self::Society => "SOCIETY",
            Self::Channel => "CHANNEL",
            _ => return None,
        })
    }

    /// The line's colour.
    #[must_use]
    pub fn color(self) -> Color {
        match self {
            Self::Say => TEXT,
            Self::Tell | Self::TellOut => TELL,
            Self::Fellow => FELLOW,
            Self::Alleg => ALLEG,
            Self::General
            | Self::Trade
            | Self::Lfg
            | Self::Roleplay
            | Self::Society
            | Self::Channel => ROOM,
            Self::Emote => EMOTE,
            Self::Magic => MAGIC,
            Self::Hit => HIT,
            Self::Hurt => HURT,
            Self::Xp => XP,
            Self::System => SYSTEM,
        }
    }

    /// Lines of this kind show the speaker's name before the words.
    #[must_use]
    pub fn names_speaker(self) -> bool {
        matches!(
            self,
            Self::Say
                | Self::Tell
                | Self::TellOut
                | Self::Fellow
                | Self::Alleg
                | Self::General
                | Self::Trade
                | Self::Lfg
                | Self::Roleplay
                | Self::Society
                | Self::Channel
        )
    }
}

/// A line as the modern chat shows it.
#[derive(Debug, Clone, PartialEq)]
pub struct ShownLine {
    /// The transcript's sequence number.
    pub sequence: u64,
    /// Its text class (for the tabs' filters).
    pub class: u32,
    /// What it is.
    pub kind: LineKind,
    /// Its time as the game words it, when timestamps are on.
    pub time: Option<String>,
    /// The speaker, when the line names one.
    pub who: Option<String>,
    /// The speaker's object id (0: none).
    pub who_guid: u32,
    /// The words (or the whole line when it names no speaker).
    pub words: String,
    /// It mentions the player's name.
    pub mention: bool,
}

impl ShownLine {
    /// `line` as shown to the player named `me`.
    #[must_use]
    pub fn of(line: &ChatLine, me: &str) -> Self {
        let kind = LineKind::of(line);
        let time = line.spans.as_ref().and_then(|spans| {
            spans
                .iter()
                .find(|s| s.role == ChatSpanRole::Timestamp)
                .map(|s| s.text.trim().to_owned())
        });
        let named = kind.names_speaker() && !line.sender.is_empty() && !line.words.is_empty();
        let words = if named {
            line.words.clone()
        } else {
            let mut text = line.text.as_str();
            if let Some(t) = &time {
                text = text.trim_start().strip_prefix(t.as_str()).unwrap_or(text);
            }
            text.trim().to_owned()
        };
        let mention =
            !me.is_empty() && line.sender != me && line.sender != "You" && mentions(&words, me);
        Self {
            sequence: line.sequence,
            class: line.log_text_type,
            kind,
            time,
            who: named.then(|| line.sender.clone()),
            who_guid: line.sender_guid,
            words,
            mention,
        }
    }
}

/// `text` names `name` as a whole word (any case).
#[must_use]
pub fn mentions(text: &str, name: &str) -> bool {
    let first = name.split_whitespace().next().unwrap_or(name);
    if first.len() < 3 {
        return false;
    }
    let low = text.to_lowercase();
    let want = first.to_lowercase();
    low.match_indices(&want).any(|(i, _)| {
        let before = low[..i].chars().next_back();
        let after = low[i + want.len()..].chars().next();
        !before.is_some_and(char::is_alphanumeric) && !after.is_some_and(char::is_alphanumeric)
    })
}

/// The mask bit of text class `class`.
#[must_use]
pub const fn bit(class: LogTextType) -> u64 {
    1 << (class as u32)
}

/// A chat tab.
#[derive(Debug, Clone, PartialEq)]
pub struct ChatTab {
    /// Its stable number.
    pub id: u32,
    /// What it is called.
    pub name: String,
    /// The text classes it shows (bit `n` = class `n`).
    pub filter: u64,
    /// The channel it talks on (none: it keeps the last one used).
    pub talk: Option<ChatChannelKind>,
    /// It is popped out into a window of its own.
    pub window: bool,
    /// That window's background opacity, 0..=1.
    pub opacity: f32,
    /// Where its window was left.
    pub place: Option<(f32, f32)>,
}

/// The message types a tab can show, as the tab dialog names them, and
/// the text classes each stands for.
pub const GROUPS: [(&str, &[LogTextType]); 15] = {
    use LogTextType as T;
    [
        ("Say", &[T::Speech, T::All, T::Default]),
        ("Tells", &[T::Tell, T::SpeechDirectSend, T::AdminTell]),
        ("Emotes", &[T::Emote]),
        ("Fellowship", &[T::Fellowship]),
        ("Allegiance", &[T::Allegiance, T::Social, T::SocialSend]),
        ("General", &[T::TurbineGeneral]),
        ("Trade", &[T::TurbineTrade]),
        ("LFG", &[T::TurbineLfg]),
        ("Roleplay", &[T::TurbineRoleplay]),
        ("Society", &[T::TurbineSociety]),
        (
            "Other channels",
            &[T::Channel, T::ChannelSend, T::WorldBroadcast],
        ),
        ("Combat", &[T::Combat, T::CombatEnemy, T::CombatSelf]),
        ("Magic", &[T::Magic, T::Spellcasting]),
        ("Experience", &[T::Advancement]),
        (
            "System",
            &[
                T::System,
                T::Help,
                T::Appraisal,
                T::Recall,
                T::Craft,
                T::Salvaging,
                T::Abuse,
            ],
        ),
    ]
};

/// The mask of group `i` of [`GROUPS`].
#[must_use]
pub fn group_mask(i: usize) -> u64 {
    GROUPS
        .get(i)
        .map_or(0, |(_, classes)| classes.iter().fold(0, |m, c| m | bit(*c)))
}

/// The channel a saved tab talks on, from its saved name.
#[must_use]
pub fn talk_from_name(name: &str) -> Option<ChatChannelKind> {
    use ChatChannelKind as K;
    [
        K::Say,
        K::Tell,
        K::Fellowship,
        K::Allegiance,
        K::Monarch,
        K::Patron,
        K::Vassals,
        K::CoVassals,
        K::General,
        K::Trade,
        K::Lfg,
        K::Roleplay,
        K::Society,
        K::Olthoi,
    ]
    .into_iter()
    .find(|k| super::channels::channel_name(*k) == name)
}

impl ChatTab {
    /// The saved form.
    #[must_use]
    pub fn to_data(&self) -> ac_host::settings::ChatTabData {
        ac_host::settings::ChatTabData {
            id: self.id,
            name: self.name.clone(),
            filter: self.filter,
            talk: self
                .talk
                .map(|k| super::channels::channel_name(k).to_owned())
                .unwrap_or_default(),
            window: self.window,
            opacity: self.opacity,
            place: self.place,
        }
    }

    /// The tab a saved form describes.
    #[must_use]
    pub fn from_data(data: &ac_host::settings::ChatTabData) -> Self {
        Self {
            id: data.id,
            name: data.name.clone(),
            filter: data.filter,
            talk: talk_from_name(&data.talk),
            window: data.window,
            opacity: data.opacity.clamp(0.0, 1.0),
            place: data.place,
        }
    }
}

impl ChatTab {
    /// It shows `line`.
    #[must_use]
    pub fn shows(&self, line: &ShownLine) -> bool {
        line.class < 64 && self.filter & (1 << line.class) != 0
    }

    /// The tabs a player starts with: Main (all retail's main window
    /// shows), Combat, Fellowship, Allegiance, Tells.
    #[must_use]
    pub fn defaults() -> Vec<Self> {
        use LogTextType as T;
        let mut id = 0;
        let mut tab = |name: &str, filter: u64, talk| {
            id += 1;
            Self {
                id,
                name: name.to_owned(),
                filter,
                talk,
                window: false,
                opacity: DEFAULT_OPACITY,
                place: None,
            }
        };
        vec![
            tab("Main", MAIN_WINDOW_DEFAULT_FILTER, None),
            tab(
                "Combat",
                bit(T::Combat)
                    | bit(T::CombatEnemy)
                    | bit(T::CombatSelf)
                    | bit(T::Magic)
                    | bit(T::Spellcasting),
                None,
            ),
            tab(
                "Fellowship",
                bit(T::Fellowship),
                Some(ChatChannelKind::Fellowship),
            ),
            tab(
                "Allegiance",
                bit(T::Allegiance),
                Some(ChatChannelKind::Allegiance),
            ),
            tab(
                "Tells",
                bit(T::Tell) | bit(T::SpeechDirectSend) | bit(T::AdminTell),
                Some(ChatChannelKind::Tell),
            ),
        ]
    }
}

/// A popped-out tab's background to start with.
pub const DEFAULT_OPACITY: f32 = 0.62;

/// The lines kept, and how many each tab has not seen.
#[derive(Debug, Clone, Default)]
pub struct ChatLog {
    /// The lines, oldest first.
    pub lines: std::collections::VecDeque<ShownLine>,
    /// Lines that came in for each tab since it was last looked at.
    pub unread: Vec<u32>,
    /// Advances on every change.
    pub revision: u64,
}

/// How many lines are kept.
pub const KEPT: usize = 1000;

impl ChatLog {
    /// Adds `line`; every tab but `looking` that shows it counts it unseen.
    pub fn push(&mut self, line: ShownLine, tabs: &[ChatTab], looking: Option<usize>) {
        self.unread.resize(tabs.len(), 0);
        for (i, tab) in tabs.iter().enumerate() {
            if Some(i) != looking && tab.shows(&line) {
                self.unread[i] = self.unread[i].saturating_add(1);
            }
        }
        self.lines.push_back(line);
        while self.lines.len() > KEPT {
            self.lines.pop_front();
        }
        self.revision += 1;
    }

    /// Tab `tab` has been looked at.
    pub fn seen(&mut self, tab: usize) {
        if let Some(n) = self.unread.get_mut(tab)
            && *n != 0
        {
            *n = 0;
            self.revision += 1;
        }
    }

    /// Tab `tab`'s unseen lines.
    #[must_use]
    pub fn unread(&self, tab: usize) -> u32 {
        self.unread.get(tab).copied().unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(kind: ChatKind, class: LogTextType, sender: &str, words: &str, text: &str) -> ChatLine {
        ChatLine {
            sequence: 1,
            kind,
            log_text_type: class as u32,
            combat_kind: None,
            received_unix_ms: 0,
            text: text.to_owned(),
            channel_name: String::new(),
            sender: sender.to_owned(),
            sender_guid: 7,
            words: words.to_owned(),
            display_text: text.to_owned(),
            channel_id: 0,
            spans: None,
        }
    }

    #[test]
    fn a_tell_shows_its_tag_name_and_words() {
        let l = line(
            ChatKind::Tell,
            LogTextType::Tell,
            "Rook",
            "portal gem ready",
            "Rook tells you, \"portal gem ready\"",
        );
        let shown = ShownLine::of(&l, "Kestrel");
        assert_eq!(shown.kind, LineKind::Tell);
        assert_eq!(shown.kind.tag(), Some("TELL"));
        assert_eq!(shown.who.as_deref(), Some("Rook"));
        assert_eq!(shown.words, "portal gem ready");
        assert!(!shown.mention);
    }

    #[test]
    fn system_and_combat_lines_keep_their_text() {
        let l = line(
            ChatKind::System,
            LogTextType::System,
            "",
            "",
            "Rook is now online.",
        );
        let shown = ShownLine::of(&l, "Kestrel");
        assert_eq!(shown.kind, LineKind::System);
        assert_eq!(shown.who, None);
        assert_eq!(shown.words, "Rook is now online.");
        let mut hit = line(
            ChatKind::Combat,
            LogTextType::Combat,
            "",
            "",
            "You hit Drudge Ravener for 212 points of fire damage.",
        );
        hit.combat_kind = Some(CombatLineKind::Warning);
        assert_eq!(ShownLine::of(&hit, "Kestrel").kind, LineKind::Hurt);
    }

    #[test]
    fn a_mention_is_a_whole_word_from_someone_else() {
        assert!(mentions("Kestrel, grats on 87!", "Kestrel"));
        assert!(mentions("grats kestrel", "Kestrel Vale"));
        assert!(!mentions("Kestrels fly", "Kestrel"));
        let l = line(
            ChatKind::Channel,
            LogTextType::Allegiance,
            "Thessaly",
            "Kestrel, grats on 87!",
            "",
        );
        assert!(ShownLine::of(&l, "Kestrel").mention);
        let mine = line(
            ChatKind::Channel,
            LogTextType::Allegiance,
            "Kestrel",
            "Kestrel here",
            "",
        );
        assert!(!ShownLine::of(&mine, "Kestrel").mention);
    }

    #[test]
    fn tabs_count_what_they_have_not_seen() {
        let tabs = ChatTab::defaults();
        let mut log = ChatLog::default();
        let fellow = ShownLine::of(
            &line(
                ChatKind::Channel,
                LogTextType::Fellowship,
                "Mira",
                "pulling",
                "",
            ),
            "Kestrel",
        );
        log.push(fellow, &tabs, Some(0));
        assert_eq!(log.unread(0), 0);
        assert_eq!(log.unread(2), 1);
        assert_eq!(log.unread(4), 0);
        log.seen(2);
        assert_eq!(log.unread(2), 0);
        assert!(tabs[0].shows(&log.lines[0]));
        assert!(!tabs[1].shows(&log.lines[0]));
    }

    #[test]
    fn a_tab_reads_back_from_its_saved_form() {
        let mut tab = ChatTab::defaults().remove(2);
        tab.window = true;
        tab.opacity = 0.8;
        assert_eq!(ChatTab::from_data(&tab.to_data()), tab);
        assert_eq!(tab.to_data().talk, "Fellowship");
        assert_ne!(group_mask(1) & bit(LogTextType::Tell), 0);
        assert_eq!(group_mask(99), 0);
    }
}
