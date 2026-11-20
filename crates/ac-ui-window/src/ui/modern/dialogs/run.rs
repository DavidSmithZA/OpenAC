//! The window's questions on the modern card: what the session asks (the
//! server's questions and the client's own), what the windows ask (a
//! training, leaving a shop with lists, forgetting a spell), and the
//! key-profile questions, one at a time in the order asked. Invitations go
//! to the cards under the target instead, and play goes on.

use std::collections::VecDeque;

use ac_core::chat::LogTextType;
use ac_proto::social::confirmation_type;
use ac_runtime::character::CharacterCommand;
use ac_runtime::communication::ClientConfirmation;
use ac_runtime::inventory::InteractionMode;
use ac_runtime::session::SessionCommand;
use ac_ui::bundled_font::Weight;
use ac_ui::modern::Horizon;
use ac_ui::panels::PanelOutput;
use ac_ui::panels::character::CharacterStatController;
use ac_ui::panels::dialog_factory::{DialogData, DialogType, DialogValue, property};
use ac_ui::panels::items::emit;
use ac_ui::root::ElementMessage;
use ac_ui::tree::ElementId;

use super::card::{CardFonts, Form, Part, QuestionCard, Shown};
use super::invites::{Invite, InviteCards, InviteFonts};
use super::look::{self, Look};
use crate::ui::keyboard_window::KeyboardDialog;
use crate::ui::window::{CHARACTER_WINDOW, DialogAction, WindowUi};

/// What an answer goes back to.
#[derive(Debug, Clone)]
pub(in crate::ui) enum Then {
    /// The window's question (the session's, a window's, a training).
    Window(DialogAction),
    /// A key-profile question.
    Keyboard(KeyboardDialog),
}

/// A question asked.
#[derive(Debug, Clone)]
struct Asked {
    context: u32,
    look: Look,
    text: String,
    form: Form,
    parts: Vec<Part>,
    /// The typed answer's first words.
    typed: String,
    /// The choice picked first.
    chosen: usize,
    then: Then,
}

/// An invitation waiting under the target.
#[derive(Debug, Clone)]
struct Invitation {
    context: u32,
    question: ClientConfirmation,
    look: Look,
    text: String,
}

/// The modern questions' elements and queue.
#[derive(Debug)]
pub struct QuestionsUi {
    /// The card.
    pub card: ElementId,
    /// The invitation cards.
    pub invites: ElementId,
    /// The questions, the one shown first.
    queue: VecDeque<Asked>,
    /// The question the card shows (0: none).
    shown: u32,
    /// The invitations, oldest first.
    invitations: Vec<Invitation>,
    /// The last number given.
    last: u32,
    /// The tool waiting to be used, and what it was last used on (the
    /// crafting question shows both).
    armed: Option<u32>,
    combined: Option<(u32, u32)>,
}

impl QuestionsUi {
    fn next(&mut self) -> u32 {
        self.last = self.last.wrapping_add(1).max(1);
        self.last
    }

    fn is_asked(&self, question: &ClientConfirmation) -> bool {
        self.queue
            .iter()
            .any(|a| matches!(&a.then, Then::Window(DialogAction::Session(c)) if c == question))
            || self.invitations.iter().any(|i| i.question == *question)
    }
}

/// The form of a factory question `data`, its first typed words and its
/// first choice.
fn form_of(data: &DialogData) -> (Form, usize) {
    match data.kind() {
        Some(DialogType::Wait) => (Form::Wait, 0),
        Some(DialogType::Message) => (Form::Ok, 0),
        Some(DialogType::ConfirmationTextInput | DialogType::TextInput) => (Form::Typed, 0),
        Some(DialogType::ConfirmationMenu | DialogType::Menu) => {
            let items = data
                .get_list(property::MENU_ITEMS)
                .unwrap_or_default()
                .to_vec();
            let chosen = usize::try_from(data.get_i32(property::MENU_SELECTION, 0)).unwrap_or(0);
            (Form::Choice(items), chosen)
        }
        _ => (Form::YesNo, 0),
    }
}

/// How a key-profile question looks.
fn keyboard_look(tag: &KeyboardDialog) -> Look {
    use look::{BLUE, GOLD, Glyph, RED};
    match tag {
        KeyboardDialog::Instructions => Look::new("Press a key", Glyph::Info, BLUE, "", ""),
        KeyboardDialog::Message => Look::message(),
        KeyboardDialog::Overwrite => Look::new(
            "Key already used",
            Glyph::Question,
            GOLD,
            "Use it here",
            "Keep",
        ),
        KeyboardDialog::Load(_) => Look::new(
            "Load a key profile",
            Glyph::Question,
            GOLD,
            "Load",
            "Cancel",
        ),
        KeyboardDialog::SaveAs => {
            Look::new("Save key profile", Glyph::Question, GOLD, "Save", "Cancel")
                .typed("Profile name")
        }
        KeyboardDialog::ProfileOverwrite(_) => {
            Look::new("Replace a profile", Glyph::Warn, RED, "Replace", "Cancel").dangerous()
        }
    }
}

impl WindowUi {
    /// The question card and the invitation cards, hidden.
    pub(in crate::ui) fn mount_modern_questions(&mut self) {
        let theme = Horizon::BRONZE;
        let fonts = CardFonts {
            title: self.modern_font(Weight::SemiBold, 15.0),
            body: self.modern_font(Weight::Regular, 13.0),
            small: self.modern_font(Weight::Regular, 11.5),
            strong: self.modern_font(Weight::SemiBold, 12.5),
            hint: self.modern_font(Weight::Regular, 10.0),
        };
        let invite_fonts = InviteFonts {
            strong: self.modern_font(Weight::SemiBold, 12.5),
            body: self.modern_font(Weight::Regular, 11.5),
        };
        let ui = &mut self.host.root;
        let invites = InviteCards::mount(ui, theme, invite_fonts);
        let card = QuestionCard::mount(ui, theme, fonts);
        let Some(modern) = self.modern.as_mut() else {
            return;
        };
        modern.questions = Some(QuestionsUi {
            card,
            invites,
            queue: VecDeque::new(),
            shown: 0,
            invitations: Vec::new(),
            last: 0,
            armed: None,
            combined: None,
        });
    }

    fn questions_mut(&mut self) -> Option<&mut QuestionsUi> {
        self.modern.as_mut().and_then(|m| m.questions.as_mut())
    }

    /// Whether the modern card asks the window's questions.
    #[must_use]
    pub(in crate::ui) fn modern_questions_on(&self) -> bool {
        self.modern.as_ref().is_some_and(|m| m.questions.is_some())
    }

    /// Asks `text` on the card, answered as `then` says; `priority` puts it
    /// before every waiting question (it shows at once).
    #[allow(clippy::too_many_arguments)]
    fn modern_question(
        &mut self,
        look: Look,
        text: String,
        form: Form,
        chosen: usize,
        parts: Vec<Part>,
        then: Then,
        priority: bool,
    ) -> u32 {
        let Some(q) = self.questions_mut() else {
            return 0;
        };
        let context = q.next();
        let asked = Asked {
            context,
            look,
            text,
            form,
            parts,
            typed: String::new(),
            chosen,
            then,
        };
        if priority {
            q.queue.push_front(asked);
        } else {
            q.queue.push_back(asked);
        }
        context
    }

    /// A window's question (`action` says what an answer does), on the
    /// card.
    pub(in crate::ui) fn modern_ask(&mut self, text: String, action: DialogAction) {
        let look = match &action {
            DialogAction::Session(c) => look::of_confirmation(c),
            DialogAction::Train { .. } => look::of_training(),
            DialogAction::Panel(_) | DialogAction::Either(..) => Look::question(),
        };
        self.modern_question(
            look,
            text,
            Form::YesNo,
            0,
            Vec::new(),
            Then::Window(action),
            false,
        );
    }

    /// A key-profile question (`data` says its kind and words), on the
    /// card; answers its number.
    pub(in crate::ui) fn modern_keyboard_question(
        &mut self,
        data: &DialogData,
        tag: KeyboardDialog,
    ) -> u32 {
        let (form, chosen) = form_of(data);
        let text = data
            .get_str(property::MESSAGE)
            .unwrap_or_default()
            .to_owned();
        let text = if text.is_empty() {
            match &tag {
                KeyboardDialog::Load(_) => "Which key profile?".to_owned(),
                KeyboardDialog::SaveAs => "Save the keys as a profile named:".to_owned(),
                _ => text,
            }
        } else {
            text
        };
        let look = keyboard_look(&tag);
        self.modern_question(
            look,
            text,
            form,
            chosen,
            Vec::new(),
            Then::Keyboard(tag),
            data.is_priority(),
        )
    }

    /// Takes question `context` back unanswered (the game is done with
    /// it); true when it was the card's.
    pub(in crate::ui) fn withdraw_modern_question(&mut self, context: u32) -> bool {
        let Some(q) = self.questions_mut() else {
            return false;
        };
        let before = q.queue.len();
        q.queue.retain(|a| a.context != context);
        before != q.queue.len()
    }

    /// A tool is waiting to be used, or was just used on `target` (the
    /// crafting question shows both).
    pub(in crate::ui) fn note_modern_craft_target(&mut self, target: u32) {
        let mode = self.facts.state.mode;
        if let Some(q) = self.questions_mut()
            && let InteractionMode::UseItemOnTarget(tool) = mode
        {
            q.combined = Some((tool, target));
        }
    }

    /// The session's open questions: new ones asked (invitations under the
    /// target, the rest on the card), ones it closed taken back.
    pub(in crate::ui) fn follow_modern_confirmations(&mut self) {
        let open = self.facts.state.confirmations.clone();
        let mode = self.facts.state.mode;
        let Some(q) = self.questions_mut() else {
            return;
        };
        if let InteractionMode::UseItemOnTarget(tool) = mode {
            q.armed = Some(tool);
        }
        q.queue.retain(
            |a| !matches!(&a.then, Then::Window(DialogAction::Session(c)) if !open.contains(c)),
        );
        q.invitations.retain(|i| open.contains(&i.question));
        let fresh: Vec<ClientConfirmation> = open.into_iter().filter(|c| !q.is_asked(c)).collect();
        for question in fresh {
            let text = self.confirmation_words(&question);
            let look = look::of_confirmation(&question);
            if look::is_invitation(&question) {
                emit(
                    &mut self.host.root,
                    PanelOutput::Session(SessionCommand::ShowText {
                        text: format!("{text} (waiting for your answer under your target)"),
                        log_text_type: if matches!(
                            question,
                            ClientConfirmation::Server {
                                confirmation_type: confirmation_type::FELLOWSHIP,
                                ..
                            }
                        ) {
                            LogTextType::Fellowship
                        } else {
                            LogTextType::Allegiance
                        },
                    }),
                );
                if let Some(q) = self.questions_mut() {
                    let context = q.next();
                    q.invitations.push(Invitation {
                        context,
                        question,
                        look,
                        text,
                    });
                }
                continue;
            }
            let parts = self.craft_parts(&question);
            self.modern_question(
                look,
                text,
                Form::YesNo,
                0,
                parts,
                Then::Window(DialogAction::Session(question)),
                false,
            );
        }
    }

    /// What a crafting question combines, tool first, when the window saw
    /// the tool used.
    fn craft_parts(&mut self, question: &ClientConfirmation) -> Vec<Part> {
        let ClientConfirmation::Server {
            confirmation_type: confirmation_type::CRAFT_INTERACTION,
            ..
        } = question
        else {
            return Vec::new();
        };
        let Some((tool, target)) = self
            .modern
            .as_ref()
            .and_then(|m| m.questions.as_ref())
            .and_then(|q| q.combined)
        else {
            return Vec::new();
        };
        let names = crate::ui::facts::composed_names(&self.status.names);
        let icons = &mut self.icons;
        let mut parts = Vec::new();
        for id in [tool, target] {
            let Some(o) = self.facts.objects.get(id) else {
                return Vec::new();
            };
            let sub = if o.workmanship > 0.0 {
                format!("workmanship {:.0}", o.workmanship)
            } else if o.stack_size > 1 {
                format!("{} of them", o.stack_size)
            } else {
                String::new()
            };
            parts.push(Part {
                icon: ac_ui::icons::IconSource::item_icon(
                    icons,
                    o.item_type,
                    o.icon_id,
                    o.icon_underlay_id,
                    o.icon_overlay_id,
                    o.effects,
                ),
                name: names(o),
                sub,
            });
        }
        parts
    }

    /// Each frame: answers acted on, the card showing the question first in
    /// line, the invitations under the target.
    pub(in crate::ui) fn tick_modern_questions(&mut self) {
        let Some((card, invites)) = self
            .modern
            .as_ref()
            .and_then(|m| m.questions.as_ref())
            .map(|q| (q.card, q.invites))
        else {
            return;
        };
        for answer in QuestionCard::take(&mut self.host.root, card) {
            let typed = QuestionCard::typed(&self.host.root, card);
            self.modern_answered(answer.context, answer.yes, answer.chosen, typed);
        }
        for (context, yes) in InviteCards::take(&mut self.host.root, invites) {
            let question = self.questions_mut().and_then(|q| {
                let i = q.invitations.iter().position(|i| i.context == context)?;
                Some(q.invitations.remove(i).question)
            });
            if let Some(confirmation) = question {
                emit(
                    &mut self.host.root,
                    PanelOutput::Session(SessionCommand::AnswerConfirmation {
                        confirmation,
                        accepted: yes,
                    }),
                );
            }
        }
        self.show_modern_question();
        self.place_modern_invites();
    }

    fn show_modern_question(&mut self) {
        let Some(q) = self.modern.as_ref().and_then(|m| m.questions.as_ref()) else {
            return;
        };
        let card = q.card;
        let first = q.queue.front().cloned();
        let shown = q.shown;
        let of = q.queue.len();
        match first {
            None if shown != 0 => {
                QuestionCard::hide(&mut self.host.root, card);
                if let Some(q) = self.questions_mut() {
                    q.shown = 0;
                }
            }
            None => {}
            Some(a) if a.context != shown => {
                let typed = a.typed.clone();
                let chosen = a.chosen;
                let context = a.context;
                let s = Shown {
                    context,
                    look: a.look,
                    text: a.text,
                    form: a.form,
                    parts: a.parts,
                    of,
                };
                QuestionCard::show(&mut self.host.root, card, s, &typed, chosen);
                if let Some(q) = self.questions_mut() {
                    q.shown = context;
                }
            }
            Some(_) => {
                // The count follows questions arriving behind it.
                let ui = &mut self.host.root;
                if let Some(c) = ui.widget_mut::<QuestionCard>(card)
                    && let Some(s) = c.shown.as_mut()
                {
                    s.of = of;
                }
                if ui.modal.is_none() {
                    ui.modal = Some(card);
                }
                QuestionCard::place_field(ui, card);
                // Escape in the typed answer's field leaves the field: the
                // question is answered no, as Escape does everywhere else.
                let typed = ui
                    .widget::<QuestionCard>(card)
                    .and_then(|c| c.shown.as_ref())
                    .is_some_and(|s| s.form == Form::Typed);
                if typed && ui.keyboard_focus().is_none() {
                    QuestionCard::answer(ui, card, false);
                }
            }
        }
    }

    fn place_modern_invites(&mut self) {
        let Some(q) = self.modern.as_ref().and_then(|m| m.questions.as_ref()) else {
            return;
        };
        let invites = q.invites;
        let list: Vec<Invite> = q
            .invitations
            .iter()
            .map(|i| Invite {
                context: i.context,
                look: i.look.clone(),
                text: i.text.clone(),
            })
            .collect();
        let target = self.modern.as_ref().and_then(|m| m.target);
        let ui = &mut self.host.root;
        let (sw, _) = ui.size();
        let (centre, top) = match target.and_then(|t| ui.element(t).map(|e| (t, e.width, e.height)))
        {
            Some((t, w, h)) => {
                let (x, y) = ui.screen_position(t);
                (x + w * 0.5, y + h + 10.0)
            }
            None => (sw * 0.5, 110.0),
        };
        InviteCards::show(ui, invites, list, centre, top);
    }

    /// Question `context` was answered (`yes`, the choice `chosen`, the
    /// typed words `typed`).
    fn modern_answered(&mut self, context: u32, yes: bool, chosen: i32, typed: String) {
        let asked = self.questions_mut().and_then(|q| {
            let i = q.queue.iter().position(|a| a.context == context)?;
            q.queue.remove(i)
        });
        let Some(asked) = asked else {
            return;
        };
        match asked.then {
            Then::Window(action) => self.modern_dialog_action(action, yes),
            Then::Keyboard(tag) => {
                let data = DialogData::default()
                    .with(property::CONFIRMATION_RESULT, DialogValue::Bool(yes))
                    .with(
                        property::TEXT_INPUT_RESULT,
                        DialogValue::Str(if yes { typed } else { String::new() }),
                    )
                    .with(property::MENU_SELECTION, DialogValue::I32(chosen));
                self.keyboard_dialog_closed(context, &data, tag);
            }
        }
    }

    /// What an answer to a window's question does, as the retail dialog's
    /// answer does it.
    fn modern_dialog_action(&mut self, action: DialogAction, yes: bool) {
        let ui = &mut self.host.root;
        match action {
            DialogAction::Session(confirmation) => emit(
                ui,
                PanelOutput::Session(SessionCommand::AnswerConfirmation {
                    confirmation,
                    accepted: yes,
                }),
            ),
            DialogAction::Panel(output) if yes => emit(ui, output),
            DialogAction::Panel(_) => {}
            DialogAction::Either(on_yes, on_no) => emit(ui, if yes { on_yes } else { on_no }),
            DialogAction::Train { skill, credits } if yes => emit(
                ui,
                PanelOutput::Session(SessionCommand::Character(CharacterCommand::TrainSkill {
                    skill,
                    credits,
                })),
            ),
            DialogAction::Train { .. } => {
                self.host.with_controller::<CharacterStatController, _>(
                    CHARACTER_WINDOW,
                    |c, ui| {
                        c.release_raise(ui);
                    },
                );
            }
        }
    }

    /// A message from an element: true when it was the card's field
    /// handing over its words (Enter answers yes).
    pub(in crate::ui) fn modern_question_message(
        &mut self,
        element: ElementId,
        message: &ElementMessage,
    ) -> bool {
        let Some((card, shown)) = self
            .modern
            .as_ref()
            .and_then(|m| m.questions.as_ref())
            .map(|q| (q.card, q.shown))
        else {
            return false;
        };
        let field = self.host.root.widget::<QuestionCard>(card).map(|c| c.field);
        if field != Some(element) {
            return false;
        }
        if matches!(message, ElementMessage::Submitted(_)) && shown != 0 {
            QuestionCard::answer(&mut self.host.root, card, true);
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_factory_question_keeps_its_form() {
        assert_eq!(form_of(&DialogData::confirmation("x")).0, Form::YesNo);
        assert_eq!(form_of(&DialogData::message("x")).0, Form::Ok);
        assert_eq!(form_of(&DialogData::wait("x")).0, Form::Wait);
        assert_eq!(
            form_of(&DialogData::confirmation_text_input("")).0,
            Form::Typed
        );
        let names = vec!["a".to_owned(), "b".to_owned()];
        let (form, chosen) = form_of(&DialogData::confirmation_menu(&names, 1));
        assert_eq!(form, Form::Choice(names));
        assert_eq!(chosen, 1);
    }
}
