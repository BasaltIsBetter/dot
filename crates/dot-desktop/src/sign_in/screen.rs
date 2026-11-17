use std::error::Error;

use dot_api::{ApiKey, ConnectError, Connection, Text};

const LONGEST_DRAFT: usize = 256;
const CAUSE_SEPARATOR: &str = ": ";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Method {
    ClaudeCode,
    ApiKey,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Problem {
    pub(crate) text: Text,
    pub(crate) cause: Option<String>,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Screen {
    Checking,
    Choosing {
        selected: Method,
        problem: Option<Problem>,
    },
    EnteringKey {
        draft: String,
        problem: Option<Problem>,
    },
    CheckingKey {
        key: ApiKey,
    },
    WaitingForClaudeCode,
    Connected,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Input {
    Previous,
    Next,
    Confirm,
    Back,
    Erase,
    Type(String),
    Paste(String),
    Point(Method),
    Press(Method),
}

#[derive(Debug)]
pub(crate) enum Report {
    Checked(Result<Option<Connection>, ConnectError>),
    KeyChecked(Result<(), ConnectError>),
    ClaudeCodeSignedIn(Result<(), ConnectError>),
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Command {
    Nothing,
    CheckKey(ApiKey),
    SignInWithClaudeCode,
    CancelClaudeCode,
}

impl Method {
    const fn other(self) -> Self {
        match self {
            Self::ClaudeCode => Self::ApiKey,
            Self::ApiKey => Self::ClaudeCode,
        }
    }

    fn numbered(text: &str) -> Option<Self> {
        match text {
            "1" => Some(Self::ClaudeCode),
            "2" => Some(Self::ApiKey),
            _ => None,
        }
    }
}

impl Problem {
    fn from_error(error: &ConnectError) -> Self {
        let causes: Vec<String> = std::iter::successors(error.source(), |&cause| cause.source())
            .map(ToString::to_string)
            .collect();
        Self {
            text: error.failure().text(),
            cause: (!causes.is_empty()).then(|| causes.join(CAUSE_SEPARATOR)),
        }
    }

    const fn plain(text: Text) -> Self {
        Self { text, cause: None }
    }
}

impl Screen {
    pub(crate) const fn choosing(selected: Method) -> Self {
        Self::Choosing {
            selected,
            problem: None,
        }
    }

    pub(crate) fn input(&mut self, input: Input) -> Command {
        match self {
            Self::Choosing { selected, .. } => match input {
                Input::Previous | Input::Next => {
                    *selected = selected.other();
                    Command::Nothing
                }
                Input::Point(method) => {
                    *selected = method;
                    Command::Nothing
                }
                Input::Confirm => {
                    let method = *selected;
                    self.choose(method)
                }
                Input::Press(method) => self.choose(method),
                Input::Type(text) => {
                    Method::numbered(&text).map_or(Command::Nothing, |method| self.choose(method))
                }
                _ => Command::Nothing,
            },
            Self::EnteringKey { draft, problem } => match input {
                Input::Type(text) | Input::Paste(text) => {
                    let room = LONGEST_DRAFT.saturating_sub(draft.len());
                    draft.extend(text.chars().filter(char::is_ascii_graphic).take(room));
                    *problem = None;
                    Command::Nothing
                }
                Input::Erase => {
                    draft.pop();
                    *problem = None;
                    Command::Nothing
                }
                Input::Confirm => {
                    let Ok(key) = ApiKey::parse(draft) else {
                        *problem = Some(Problem::plain(Text::SignInKeyMalformed));
                        return Command::Nothing;
                    };
                    *self = Self::CheckingKey { key: key.clone() };
                    Command::CheckKey(key)
                }
                Input::Back => {
                    *self = Self::choosing(Method::ApiKey);
                    Command::Nothing
                }
                _ => Command::Nothing,
            },
            Self::WaitingForClaudeCode if input == Input::Back => {
                *self = Self::choosing(Method::ClaudeCode);
                Command::CancelClaudeCode
            }
            _ => Command::Nothing,
        }
    }

    pub(crate) fn report(&mut self, report: Report) {
        let next = match (&*self, report) {
            (Self::Checking, Report::Checked(Ok(Some(_))))
            | (Self::CheckingKey { .. }, Report::KeyChecked(Ok(())))
            | (Self::WaitingForClaudeCode, Report::ClaudeCodeSignedIn(Ok(()))) => Self::Connected,
            (Self::Checking, Report::Checked(Ok(None))) => Self::choosing(Method::ClaudeCode),
            (Self::Checking, Report::Checked(Err(error)))
            | (Self::WaitingForClaudeCode, Report::ClaudeCodeSignedIn(Err(error))) => {
                Self::Choosing {
                    selected: Method::ClaudeCode,
                    problem: Some(Problem::from_error(&error)),
                }
            }
            (Self::CheckingKey { key }, Report::KeyChecked(Err(error))) => Self::EnteringKey {
                draft: key.expose().to_owned(),
                problem: Some(Problem::from_error(&error)),
            },
            _ => return,
        };
        *self = next;
    }

    fn choose(&mut self, method: Method) -> Command {
        match method {
            Method::ClaudeCode => {
                *self = Self::WaitingForClaudeCode;
                Command::SignInWithClaudeCode
            }
            Method::ApiKey => {
                *self = Self::EnteringKey {
                    draft: String::new(),
                    problem: None,
                };
                Command::Nothing
            }
        }
    }
}

#[cfg(test)]
#[path = "../../../../tests/dot-desktop/sign_in/screen.rs"]
mod tests;
