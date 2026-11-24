use dot_api::{ApiKey, ConnectError, Connection, Secret, Text};

use crate::error::Problem;

pub(crate) const LONGEST_DRAFT: usize = 256;
const ORDER: [Connection; 3] = [
    Connection::ClaudeCode,
    Connection::AnthropicAccount,
    Connection::ApiKey,
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Account {
    ClaudeCode,
    Anthropic,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Screen {
    Checking,
    Choosing {
        selected: Connection,
        problem: Option<Problem>,
    },
    EnteringKey {
        draft: Secret,
        problem: Option<Problem>,
    },
    CheckingKey {
        key: ApiKey,
    },
    WaitingForSignIn(Account),
    Connected(Connection),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Input {
    Previous,
    Next,
    Confirm,
    Back,
    Erase,
    Type(Secret),
    Paste(Secret),
    Point(Connection),
    Press(Connection),
}

#[derive(Debug)]
pub(crate) enum Report {
    Checked(Result<Option<Connection>, ConnectError>),
    KeyChecked(Result<(), ConnectError>),
    SignedIn(Result<(), ConnectError>),
    KeyReplaced(Result<(), ConnectError>),
    SignedOut(Result<(), ConnectError>),
}

impl Report {
    pub(crate) const fn error(&self) -> Option<&ConnectError> {
        match self {
            Self::Checked(Err(error))
            | Self::KeyChecked(Err(error))
            | Self::SignedIn(Err(error))
            | Self::KeyReplaced(Err(error))
            | Self::SignedOut(Err(error)) => Some(error),
            _ => None,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Command {
    Nothing,
    CheckKey(ApiKey),
    SignIn(Account),
    CancelSignIn,
}

impl Account {
    pub(crate) const fn connection(self) -> Connection {
        match self {
            Self::ClaudeCode => Connection::ClaudeCode,
            Self::Anthropic => Connection::AnthropicAccount,
        }
    }
}

impl Screen {
    pub(crate) const fn choosing(selected: Connection) -> Self {
        Self::Choosing {
            selected,
            problem: None,
        }
    }

    pub(crate) fn input(&mut self, input: Input) -> Command {
        match self {
            Self::Choosing { selected, .. } => match input {
                Input::Previous => {
                    *selected = step(*selected, ORDER.len() - 1);
                    Command::Nothing
                }
                Input::Next => {
                    *selected = step(*selected, 1);
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
                    numbered(text.expose()).map_or(Command::Nothing, |method| self.choose(method))
                }
                _ => Command::Nothing,
            },
            Self::EnteringKey { draft, problem } => match input {
                Input::Type(text) | Input::Paste(text) => {
                    let draft = draft.expose_mut();
                    let room = LONGEST_DRAFT.saturating_sub(draft.len());
                    let typed = text.expose().chars();
                    draft.extend(typed.filter(char::is_ascii_graphic).take(room));
                    *problem = None;
                    Command::Nothing
                }
                Input::Erase => {
                    draft.expose_mut().pop();
                    *problem = None;
                    Command::Nothing
                }
                Input::Confirm => {
                    let Ok(key) = ApiKey::parse(draft.expose()) else {
                        *problem = Some(Problem::plain(Text::SignInKeyMalformed));
                        return Command::Nothing;
                    };
                    *self = Self::CheckingKey { key: key.clone() };
                    Command::CheckKey(key)
                }
                Input::Back => {
                    *self = Self::choosing(Connection::ApiKey);
                    Command::Nothing
                }
                _ => Command::Nothing,
            },
            Self::WaitingForSignIn(account) if input == Input::Back => {
                *self = Self::choosing(account.connection());
                Command::CancelSignIn
            }
            _ => Command::Nothing,
        }
    }

    pub(crate) fn report(&mut self, report: Report) {
        let next = match (&*self, report) {
            (Self::Checking, Report::Checked(Ok(Some(connection)))) => Self::Connected(connection),
            (Self::CheckingKey { .. }, Report::KeyChecked(Ok(()))) => {
                Self::Connected(Connection::ApiKey)
            }
            (Self::WaitingForSignIn(account), Report::SignedIn(Ok(()))) => {
                Self::Connected(account.connection())
            }
            (Self::Checking, Report::Checked(Ok(None))) => Self::choosing(Connection::ClaudeCode),
            (Self::Checking, Report::Checked(Err(error))) => Self::Choosing {
                selected: Connection::ClaudeCode,
                problem: Some(Problem::new(error.failure().text(), &error)),
            },
            (Self::WaitingForSignIn(account), Report::SignedIn(Err(error))) => Self::Choosing {
                selected: account.connection(),
                problem: Some(Problem::new(error.failure().text(), &error)),
            },
            (Self::CheckingKey { key }, Report::KeyChecked(Err(error))) => Self::EnteringKey {
                draft: key_draft(key.expose()),
                problem: Some(Problem::new(error.failure().text(), &error)),
            },
            _ => return,
        };
        *self = next;
    }

    fn choose(&mut self, method: Connection) -> Command {
        let account = match method {
            Connection::ClaudeCode => Account::ClaudeCode,
            Connection::AnthropicAccount => Account::Anthropic,
            Connection::ApiKey => {
                *self = Self::EnteringKey {
                    draft: Secret::with_capacity(LONGEST_DRAFT),
                    problem: None,
                };
                return Command::Nothing;
            }
        };
        *self = Self::WaitingForSignIn(account);
        Command::SignIn(account)
    }
}

fn key_draft(key: &str) -> Secret {
    let mut draft = Secret::with_capacity(LONGEST_DRAFT);
    draft.expose_mut().push_str(key);
    draft
}

fn step(method: Connection, forward: usize) -> Connection {
    let position = ORDER
        .iter()
        .position(|&listed| listed == method)
        .unwrap_or_default();
    ORDER[(position + forward) % ORDER.len()]
}

fn numbered(text: &str) -> Option<Connection> {
    let number: usize = text.parse().ok()?;
    ORDER.get(number.checked_sub(1)?).copied()
}

#[cfg(test)]
#[path = "../../../../tests/dot-desktop/sign_in/screen.rs"]
mod tests;
