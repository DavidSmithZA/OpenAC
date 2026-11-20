//! The @ commands the entry suggests while one is being typed, and where a
//! typed line will really go (for the channel button).

use ac_runtime::communication::{ChatChannelKind, parse_chat_input};

use super::channels::channel_name;

/// A chat command the entry can suggest.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Command {
    /// The command as typed.
    pub verb: &'static str,
    /// Its short form, if it has one.
    pub alias: &'static str,
    /// What it does.
    pub about: &'static str,
    /// It answers the last tell (its target is the last teller).
    pub reply: bool,
}

const fn c(verb: &'static str, alias: &'static str, about: &'static str) -> Command {
    Command {
        verb,
        alias,
        about,
        reply: false,
    }
}

/// The chat commands, in the order they are suggested.
pub const COMMANDS: [Command; 18] = [
    Command {
        verb: "@reply",
        alias: "@r",
        about: "Reply to your last tell",
        reply: true,
    },
    c("@retell", "@rt", "Tell the last person you told"),
    c("@tell", "@t", "Tell someone: @tell name, words"),
    c("@say", "@s", "Say something to people nearby"),
    c("@fellowship", "@f", "Message your fellowship"),
    c("@allegiance", "@a", "Message your allegiance"),
    c("@monarch", "@m", "Message your monarch"),
    c("@patron", "@p", "Message your patron"),
    c("@vassals", "@v", "Message your vassals"),
    c("@covassals", "@c", "Message your fellow vassals"),
    c("@general", "@cg", "Message General chat"),
    c("@trade", "@ct", "Message Trade chat"),
    c("@lfg", "@clfg", "Message Looking for Group"),
    c("@roleplay", "@crp", "Message Roleplay chat"),
    c("@society", "@soc", "Message your society"),
    c("@olthoi", "@o", "Message Olthoi chat"),
    c("@help", "", "List every command"),
    c("@emote", "@e", "Act something out"),
];

/// The commands that start with what is typed (`@` or `/` and part of a
/// command, nothing after it); at most `most`.
#[must_use]
pub fn suggestions(typed: &str, most: usize) -> Vec<Command> {
    let Some(rest) = typed.strip_prefix(['@', '/']) else {
        return Vec::new();
    };
    if rest.contains(char::is_whitespace) {
        return Vec::new();
    }
    let rest = rest.to_lowercase();
    let starts = |v: &str| !v.is_empty() && v[1..].starts_with(rest.as_str());
    let mut out: Vec<Command> = COMMANDS
        .iter()
        .filter(|cmd| starts(cmd.alias) && cmd.alias[1..] == *rest)
        .copied()
        .collect();
    for cmd in COMMANDS {
        if (starts(cmd.verb) || starts(cmd.alias)) && !out.contains(&cmd) {
            out.push(cmd);
        }
    }
    out.truncate(most);
    out
}

/// Where a typed line goes when its command sends it somewhere other than
/// the entry's channel: "Reply · Rook", "Fellowship". None for a plain
/// line or a command that is not a chat command.
#[must_use]
pub fn going_to(
    typed: &str,
    channel: ChatChannelKind,
    last_teller: Option<&str>,
    tell_target: Option<&str>,
) -> Option<String> {
    let trimmed = typed.trim_start();
    if !trimmed.starts_with(['@', '/']) || !trimmed.contains(char::is_whitespace) {
        return None;
    }
    let verb = trimmed
        .split_whitespace()
        .next()
        .unwrap_or("")
        .to_lowercase();
    let verb = verb.trim_end_matches(',');
    let parsed = parse_chat_input(trimmed, channel, last_teller, None, tell_target, 0)?;
    let reply = matches!(&verb[1..], "reply" | "r" | "rp");
    Some(match (parsed.channel, parsed.target_name) {
        (ChatChannelKind::Tell, Some(who)) if reply => format!("Reply \u{b7} {who}"),
        (ChatChannelKind::Tell, Some(who)) => format!("Tell {who}"),
        (ChatChannelKind::Tell, None) => return None,
        (kind, _) if kind == channel => return None,
        (kind, _) => channel_name(kind).to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_short_form_typed_whole_comes_first() {
        let found = suggestions("@r", 3);
        assert_eq!(found[0].verb, "@reply");
        assert!(found.iter().any(|c| c.verb == "@retell"));
        assert_eq!(suggestions("/rol", 5)[0].verb, "@roleplay");
        assert!(suggestions("@reply hi", 5).is_empty());
        assert!(suggestions("hello", 5).is_empty());
        assert_eq!(suggestions("@", 4).len(), 4);
    }

    #[test]
    fn the_button_says_where_a_command_sends_the_line() {
        assert_eq!(
            going_to("@r thanks", ChatChannelKind::Say, Some("Rook"), None).as_deref(),
            Some("Reply \u{b7} Rook")
        );
        assert_eq!(
            going_to("@f pulling", ChatChannelKind::Say, None, None).as_deref(),
            Some("Fellowship")
        );
        assert_eq!(going_to("hello", ChatChannelKind::Say, None, None), None);
        assert_eq!(going_to("@s hi", ChatChannelKind::Say, None, None), None);
    }
}
