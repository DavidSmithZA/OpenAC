//! How each question looks on the card: its icon and colour, its title,
//! the words on its two buttons (named for what they do), whether its yes
//! cannot be undone (a red button), and a line under the question when the
//! game's own words leave something out.

use ac_proto::social::confirmation_type as kind;
use ac_runtime::communication::{ClientConfirmation, ItemUseConfirmation};
use ac_ui::draw::Color;
use ac_ui::modern::theme::hex;
use ac_ui::modern::tone;

/// The card's icon, drawn as lines.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Glyph {
    /// Crafting.
    Hammer,
    /// Allegiance.
    Shield,
    /// A fellowship.
    Users,
    /// A raise or a change to the character.
    Up,
    /// An augmentation.
    Star,
    /// A plain question.
    Question,
    /// A message.
    Info,
    /// Something that cannot be undone.
    Warn,
}

/// Blue: the character changes, messages.
pub const BLUE: Color = hex(0x8F_B8FF, 1.0);
/// Green: people asking to join up.
pub const GREEN: Color = hex(0x7F_D08A, 1.0);
/// Red: what cannot be undone.
pub const RED: Color = hex(0xFF_8A8E, 1.0);
/// Violet: augmentations.
pub const VIOLET: Color = hex(0xC9_A2FF, 1.0);

/// How a question looks.
#[derive(Debug, Clone, PartialEq)]
pub struct Look {
    /// The title beside the icon.
    pub title: String,
    /// The icon.
    pub glyph: Glyph,
    /// The icon's colour.
    pub tint: Color,
    /// The yes button's words.
    pub yes: String,
    /// The no button's words.
    pub no: String,
    /// The yes cannot be undone: it is red.
    pub danger: bool,
    /// A line under the question.
    pub note: Option<String>,
    /// That line is a warning (red).
    pub note_red: bool,
    /// The label over a typed answer's field.
    pub field: Option<String>,
}

impl Look {
    /// A question titled `title` with `glyph` in `tint`, answered `yes` or
    /// `no`.
    #[must_use]
    pub fn new(title: &str, glyph: Glyph, tint: Color, yes: &str, no: &str) -> Self {
        Self {
            title: title.to_owned(),
            glyph,
            tint,
            yes: yes.to_owned(),
            no: no.to_owned(),
            danger: false,
            note: None,
            note_red: false,
            field: None,
        }
    }

    /// The plain yes or no question.
    #[must_use]
    pub fn question() -> Self {
        Self::new("Question", Glyph::Question, tone::gold(1.0), "Yes", "No")
    }

    /// A message read and closed with OK.
    #[must_use]
    pub fn message() -> Self {
        Self::new("Message", Glyph::Info, BLUE, "OK", "")
    }

    /// The yes is red: it cannot be undone.
    #[must_use]
    pub fn dangerous(mut self) -> Self {
        self.danger = true;
        self
    }

    /// With `note` under the question (`red`: a warning).
    #[must_use]
    pub fn noted(mut self, note: &str, red: bool) -> Self {
        self.note = Some(note.to_owned());
        self.note_red = red;
        self
    }

    /// A typed answer, its field labelled `label`.
    #[must_use]
    pub fn typed(mut self, label: &str) -> Self {
        self.field = Some(label.to_owned());
        self
    }
}

/// Whether `question` is someone asking to join up with the player (an
/// allegiance or a fellowship): it waits under the target while play goes
/// on, rather than stopping play in the middle of the screen.
#[must_use]
pub fn is_invitation(question: &ClientConfirmation) -> bool {
    matches!(
        question,
        ClientConfirmation::Server {
            confirmation_type: kind::SWEAR_ALLEGIANCE | kind::FELLOWSHIP,
            ..
        }
    )
}

/// The yes of a change to a skill or an attribute, named from the
/// server's question.
fn change_word(message: &str) -> &'static str {
    let m = message.to_ascii_lowercase();
    if m.contains("untrain") {
        "Untrain"
    } else if m.contains("unspecial") {
        "Unspecialize"
    } else if m.contains("speciali") {
        "Specialize"
    } else if m.contains("lower") || m.contains("reduce") {
        "Lower"
    } else if m.contains("raise") || m.contains("increase") {
        "Raise"
    } else if m.contains("train") {
        "Train"
    } else {
        "Continue"
    }
}

/// How the session's `question` looks. `asker` is the name an
/// invitation's text carries (the server sends the asker's name).
#[must_use]
pub fn of_confirmation(question: &ClientConfirmation) -> Look {
    match question {
        ClientConfirmation::Suicide => {
            Look::new("End your life", Glyph::Warn, RED, "Yes", "No").dangerous()
        }
        ClientConfirmation::AbandonHouse | ClientConfirmation::AbandonHouseCertain => {
            Look::new("Abandon your house", Glyph::Warn, RED, "Abandon", "Keep it").dangerous()
        }
        ClientConfirmation::ItemUse { kind, .. } => match kind {
            ItemUseConfirmation::PlayerKillerSwitch
            | ItemUseConfirmation::NonPlayerKillerSwitch => {
                Look::new("Use the altar", Glyph::Warn, tone::gold(1.0), "Use it", "Cancel")
            }
            ItemUseConfirmation::VolatileRare => {
                Look::new("Use a rare", Glyph::Star, VIOLET, "Use it", "Cancel")
            }
        },
        ClientConfirmation::ManaStoneDrain { .. } => {
            Look::new("Drain into the stone", Glyph::Warn, RED, "Drain", "Cancel")
                .dangerous()
                .noted("The item is destroyed.", true)
        }
        ClientConfirmation::Server {
            confirmation_type,
            message,
            ..
        } => match *confirmation_type {
            kind::SWEAR_ALLEGIANCE => Look::new(
                "Allegiance request",
                Glyph::Shield,
                tone::gold(1.0),
                "Accept",
                "Decline",
            )
            .noted(
                &format!(
                    "{message} becomes your vassal, and part of the experience they earn passes up to you."
                ),
                false,
            ),
            kind::FELLOWSHIP => Look::new(
                "Fellowship invitation",
                Glyph::Users,
                GREEN,
                "Join",
                "Decline",
            ),
            kind::ALTER_SKILL => {
                Look::new("Change a skill", Glyph::Up, BLUE, change_word(message), "Cancel")
            }
            kind::ALTER_ATTRIBUTE => Look::new(
                "Change an attribute",
                Glyph::Up,
                BLUE,
                change_word(message),
                "Cancel",
            ),
            kind::CRAFT_INTERACTION => {
                Look::new("Crafting", Glyph::Hammer, tone::gold(1.0), "Apply", "Cancel").dangerous()
            }
            kind::AUGMENTATION => Look::new("Augmentation", Glyph::Star, VIOLET, "Augment", "Cancel")
                .dangerous()
                .noted("It cannot be undone.", true),
            _ => Look::question(),
        },
    }
}

/// The training question's look.
#[must_use]
pub fn of_training() -> Look {
    Look::new("Train a skill", Glyph::Up, BLUE, "Train", "Cancel")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn server(confirmation_type: u32, message: &str) -> ClientConfirmation {
        ClientConfirmation::Server {
            confirmation_type,
            context_id: 9,
            message: message.to_owned(),
        }
    }

    #[test]
    fn invitations_are_the_allegiance_and_fellowship_questions() {
        assert!(is_invitation(&server(kind::SWEAR_ALLEGIANCE, "Lysse")));
        assert!(is_invitation(&server(kind::FELLOWSHIP, "Mira")));
        assert!(!is_invitation(&server(kind::CRAFT_INTERACTION, "x")));
        assert!(!is_invitation(&ClientConfirmation::Suicide));
    }

    #[test]
    fn buttons_are_named_for_what_they_do() {
        let craft = of_confirmation(&server(kind::CRAFT_INTERACTION, "33% ..."));
        assert_eq!((craft.yes.as_str(), craft.danger), ("Apply", true));
        let fellow = of_confirmation(&server(kind::FELLOWSHIP, "Mira"));
        assert_eq!(
            (fellow.yes.as_str(), fellow.no.as_str()),
            ("Join", "Decline")
        );
        let aug = of_confirmation(&server(kind::AUGMENTATION, "Costs a lot"));
        assert!(aug.danger && aug.note_red);
        let plain = of_confirmation(&server(kind::YES_NO, "Enter the arena?"));
        assert_eq!((plain.yes.as_str(), plain.danger), ("Yes", false));
    }

    #[test]
    fn a_change_is_named_from_the_question() {
        let untrain = server(kind::ALTER_SKILL, "Untraining Lockpick returns 6 credits.");
        assert_eq!(of_confirmation(&untrain).yes, "Untrain");
        let lower = server(kind::ALTER_ATTRIBUTE, "Lowering Strength by 10 ...");
        assert_eq!(of_confirmation(&lower).yes, "Lower");
        let other = server(kind::ALTER_SKILL, "Something else.");
        assert_eq!(of_confirmation(&other).yes, "Continue");
    }
}
