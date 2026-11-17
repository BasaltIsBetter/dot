use dot_api::{Text, Translator};

use crate::canvas::{Ink, Layout, Line, Span};
use crate::sign_in::screen::{Method, Problem, Screen};

const ROWS: usize = Layout::ROWS;
const TITLE_ROW: usize = 0;
const PROMPT_ROW: usize = 1;
const FIELD_ROW: usize = 3;
const OPTION_ROWS: [(usize, Method); 2] = [(3, Method::ClaudeCode), (4, Method::ApiKey)];
const HINT_ROW: usize = 6;
const SECOND_HINT_ROW: usize = 7;
const PROBLEM_ROW: usize = 9;
const CAUSE_ROW: usize = 10;

const SELECTED: &str = "› ";
const UNSELECTED: &str = "  ";
const MARKER_WIDTH: usize = UNSELECTED.len();
const CARET: &str = "_";
const LABEL_GAP: usize = 3;
const KEY_PREFIX: &str = "sk-ant-";
const MASK: char = '•';
const LONGEST_MASK: usize = 32;
const SHOWN_TAIL: usize = 4;
const FULL_KEY: usize = 20;

pub(crate) fn lines(screen: &Screen, translator: &impl Translator) -> Vec<Line> {
    let mut rows = vec![Line::default(); ROWS];
    let span = |text: Text, ink: Ink| Span::new(translator.translate(text), ink);

    match screen {
        Screen::Checking | Screen::Connected(_) => return Vec::new(),
        Screen::Choosing { selected, problem } => {
            rows[PROMPT_ROW] = centred(vec![span(Text::SignInIntro, Ink::Secondary)]);
            show_options(&mut rows, translator, *selected, false);
            rows[HINT_ROW] = centred(vec![span(Text::SignInChooseHint, Ink::Receding)]);
            show_problem(&mut rows, translator, problem.as_ref());
        }
        Screen::WaitingForClaudeCode => {
            rows[PROMPT_ROW] = centred(vec![span(Text::SignInIntro, Ink::Secondary)]);
            show_options(&mut rows, translator, Method::ClaudeCode, true);
            rows[HINT_ROW] = centred(vec![span(Text::SignInWaitingForClaudeCode, Ink::Live)]);
            rows[SECOND_HINT_ROW] = centred(vec![span(Text::SignInCancelHint, Ink::Receding)]);
        }
        Screen::EnteringKey { draft, problem } => {
            rows[PROMPT_ROW] = centred(vec![span(Text::SignInKeyPrompt, Ink::Secondary)]);
            rows[FIELD_ROW] = field(vec![
                Span::new(format!("{SELECTED}{}", masked(draft)), Ink::Full),
                Span::new(CARET, Ink::Full),
            ]);
            rows[HINT_ROW] = centred(vec![span(Text::SignInKeyHint, Ink::Receding)]);
            show_problem(&mut rows, translator, problem.as_ref());
        }
        Screen::CheckingKey { key } => {
            rows[PROMPT_ROW] = centred(vec![span(Text::SignInKeyPrompt, Ink::Secondary)]);
            rows[FIELD_ROW] = field(vec![Span::new(
                format!("{SELECTED}{}", masked(key.expose())),
                Ink::Secondary,
            )]);
            rows[HINT_ROW] = centred(vec![span(Text::SignInCheckingKey, Ink::Live)]);
        }
    }

    rows[TITLE_ROW] = centred(vec![span(Text::SignInTitle, Ink::Full)]);
    rows
}

pub(crate) fn method_at(screen: &Screen, row: usize) -> Option<Method> {
    if !matches!(screen, Screen::Choosing { .. }) {
        return None;
    }
    OPTION_ROWS
        .iter()
        .find_map(|&(option_row, method)| (option_row == row).then_some(method))
}

fn show_options(rows: &mut [Line], translator: &impl Translator, selected: Method, waiting: bool) {
    let widest = |text_of: fn((Text, Text)) -> Text| {
        OPTION_ROWS
            .iter()
            .map(|&(_, method)| {
                translator
                    .translate(text_of(texts_of(method)))
                    .chars()
                    .count()
            })
            .max()
            .unwrap_or_default()
    };
    let width = widest(|(label, _)| label) + LABEL_GAP;
    let indent = indent_for(MARKER_WIDTH + width + widest(|(_, detail)| detail));

    for (row, method) in OPTION_ROWS {
        let (label, detail) = texts_of(method);
        let (marker, label_ink, detail_ink) = match (method == selected, waiting) {
            (true, _) => (SELECTED, Ink::Full, Ink::Secondary),
            (false, false) => (UNSELECTED, Ink::Receding, Ink::Faint),
            (false, true) => (UNSELECTED, Ink::Faint, Ink::Faint),
        };
        let label = translator.translate(label);
        rows[row] = Line::new(
            indent,
            vec![
                Span::new(format!("{marker}{label:<width$}"), label_ink),
                Span::new(translator.translate(detail), detail_ink),
            ],
        );
    }
}

fn show_problem(rows: &mut [Line], translator: &impl Translator, problem: Option<&Problem>) {
    let Some(problem) = problem else {
        return;
    };
    rows[PROBLEM_ROW] = centred(vec![Span::new(
        translator.translate(problem.text),
        Ink::Full,
    )]);
    if let Some(cause) = &problem.cause {
        rows[CAUSE_ROW] = centred(vec![Span::new(cause.as_str(), Ink::Receding)]);
    }
}

fn centred(spans: Vec<Span>) -> Line {
    let width = spans.iter().map(|span| span.text.chars().count()).sum();
    Line::new(indent_for(width), spans)
}

fn field(spans: Vec<Span>) -> Line {
    let widest = MARKER_WIDTH + KEY_PREFIX.len() + LONGEST_MASK + SHOWN_TAIL + CARET.len();
    Line::new(indent_for(widest), spans)
}

fn indent_for(width: usize) -> u16 {
    let width = u16::try_from(width).unwrap_or(u16::MAX);
    Layout::COLUMNS.saturating_sub(width) / 2
}

const fn texts_of(method: Method) -> (Text, Text) {
    match method {
        Method::ClaudeCode => (Text::SignInClaudeAccount, Text::SignInClaudeAccountDetail),
        Method::ApiKey => (Text::SignInApiKey, Text::SignInApiKeyDetail),
    }
}

fn masked(draft: &str) -> String {
    let (prefix, secret) = draft
        .strip_prefix(KEY_PREFIX)
        .map_or(("", draft), |secret| (KEY_PREFIX, secret));
    let secret_length = secret.chars().count();
    let tail_length = if draft.chars().count() >= FULL_KEY {
        SHOWN_TAIL.min(secret_length)
    } else {
        0
    };
    let hidden = secret_length - tail_length;
    let tail: String = secret.chars().skip(hidden).collect();
    let mask: String = std::iter::repeat_n(MASK, hidden.min(LONGEST_MASK)).collect();
    format!("{prefix}{mask}{tail}")
}

#[cfg(test)]
#[path = "../../../../tests/dot-desktop/sign_in/view.rs"]
mod tests;
