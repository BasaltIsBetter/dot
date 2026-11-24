use dot_api::{Text, Translator};
use dot_transcript::{Ink, Line, Span, columns};

use crate::canvas::TextSize;
use crate::settings::Waiting;
use crate::settings::catalog::Setting;
use crate::settings::view::draw::{ROW, pane, width_of};
use crate::settings::view::{Drawn, Hit, Look, Row};

const KEY_DOT: &str = "•";
const CARET: &str = "_";
const ELLIPSIS: char = '…';
const LABEL_GAP: usize = 2;
const ENTER: &str = "↵";
const ENTER_GAP: &str = "  ";
const ENTER_INK: Ink = Ink::Receding;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Actions {
    pub(crate) folder: String,
    pub(crate) account: String,
    pub(crate) key: String,
}

pub(super) fn action<T: Translator>(
    drawn: &mut Drawn,
    (setting, focused): (Setting, bool),
    look: &Look<'_, T>,
    row: &Row<'_>,
) {
    let (text, ink) = match (setting, look.waiting) {
        (Setting::SignOut, Waiting::SignOut) => (
            look.translator.translate(Text::SignOutConfirm).into_owned(),
            Ink::Live,
        ),
        (Setting::ApiKey, Waiting::Key(typed)) => {
            (format!("{}{CARET}", KEY_DOT.repeat(typed)), Ink::Live)
        }
        (Setting::ApiKey, _) => (look.actions.key.clone(), row.ink),
        (Setting::Folder, _) => (look.actions.folder.clone(), row.ink),
        (Setting::Account, _) => (look.actions.account.clone(), row.ink),
        (Setting::SignOut, _) => (String::new(), row.ink),
        _ => return,
    };
    let arrow = enter_arrow(&text, focused && ink != Ink::Live);
    let layout = row.layout;
    let advance = layout.advance_of(TextSize::Regular);
    let label = look.translator.translate(setting.label());
    let room = whole(row.frame.width / advance)
        .saturating_sub(columns(&label) + LABEL_GAP + columns(&arrow));
    let shown = ending(&text, room);
    if shown.is_empty() && arrow.is_empty() {
        return;
    }
    let width = (width_of(&shown) + width_of(&arrow)) * advance;
    let left = row.frame.right() - width;
    let height = layout.scaled(ROW);
    drawn
        .hits
        .push(([left, row.top, width, height], Hit::Choose(setting, 0)));
    let text_top = layout.text_top(TextSize::Regular, row.top, height);
    let line =
        Line::new(0, vec![Span::new(shown, ink), Span::new(arrow, ENTER_INK)]).faded(row.alpha);
    drawn
        .panes
        .push(pane(left, TextSize::Regular, text_top, line));
}

fn enter_arrow(text: &str, shown: bool) -> String {
    match (shown, text.is_empty()) {
        (false, _) => String::new(),
        (true, true) => ENTER.to_owned(),
        (true, false) => format!("{ENTER_GAP}{ENTER}"),
    }
}

pub(super) fn ending(text: &str, room: usize) -> String {
    let count = text.chars().count();
    if count <= room {
        return text.to_owned();
    }
    let Some(kept) = room.checked_sub(1) else {
        return String::new();
    };
    let mut shown = String::from(ELLIPSIS);
    shown.extend(text.chars().skip(count - kept));
    shown
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "a count of columns is floored and non-negative, far below usize::MAX"
)]
fn whole(columns: f32) -> usize {
    columns.max(0.0).floor() as usize
}

#[cfg(test)]
#[path = "../../../../../tests/dot-desktop/settings/view/action.rs"]
mod tests;
