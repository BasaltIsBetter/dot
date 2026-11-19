use dot_api::{ApiKey, ConnectError, Connection, Text};

use crate::error::Problem;

const LONGEST_DRAFT: usize = 256;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Method {
    ClaudeCode,
    AnthropicAccount,
    ApiKey,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Account {
    ClaudeCode,
    Anthropic,
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
    Type(String),
    Paste(String),
    Point(Method),
    Press(Method),
}

#[derive(Debug)]
pub(crate) enum Report {
    Checked(Result<Option<Connection>, ConnectError>),
    KeyChecked(Result<(), ConnectError>),
    SignedIn(Result<(), ConnectError>),
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Command {
    Nothing,
    CheckKey(ApiKey),
    SignIn(Account),
    CancelSignIn,
}

impl Method {
    const ORDER: [Self; 3] = [Self::ClaudeCode, Self::AnthropicAccount, Self::ApiKey];

    fn step(self, forward: usize) -> Self {
        let position = Self::ORDER
            .iter()
            .position(|&method| method == self)
            .unwrap_or_default();
        Self::ORDER[(position + forward) % Self::ORDER.len()]
    }

    fn numbered(text: &str) -> Option<Self> {
        let number: usize = text.parse().ok()?;
        Self::ORDER.get(number.checked_sub(1)?).copied()
    }
}

impl Account {
    pub(crate) const fn method(self) -> Method {
        match self {
            Self::ClaudeCode => Method::ClaudeCode,
            Self::Anthropic => Method::AnthropicAccount,
        }
    }

    const fn connection(self) -> Connection {
        match self {
            Self::ClaudeCode => Connection::ClaudeCode,
            Self::Anthropic => Connection::AnthropicAccount,
        }
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
                Input::Previous => {
                    *selected = selected.step(Method::ORDER.len() - 1);
                    Command::Nothing
                }
                Input::Next => {
                    *selected = selected.step(1);
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
            Self::WaitingForSignIn(account) if input == Input::Back => {
                *self = Self::choosing(account.method());
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
            (Self::Checking, Report::Checked(Ok(None))) => Self::choosing(Method::ClaudeCode),
            (Self::Checking, Report::Checked(Err(error))) => Self::Choosing {
                selected: Method::ClaudeCode,
                problem: Some(Problem::new(error.failure().text(), &error)),
            },
            (Self::WaitingForSignIn(account), Report::SignedIn(Err(error))) => Self::Choosing {
                selected: account.method(),
                problem: Some(Problem::new(error.failure().text(), &error)),
            },
            (Self::CheckingKey { key }, Report::KeyChecked(Err(error))) => Self::EnteringKey {
                draft: key.expose().to_owned(),
                problem: Some(Problem::new(error.failure().text(), &error)),
            },
            _ => return,
        };
        *self = next;
    }

    fn choose(&mut self, method: Method) -> Command {
        let account = match method {
            Method::ClaudeCode => Account::ClaudeCode,
            Method::AnthropicAccount => Account::Anthropic,
            Method::ApiKey => {
                *self = Self::EnteringKey {
                    draft: String::new(),
                    problem: None,
                };
                return Command::Nothing;
            }
        };
        *self = Self::WaitingForSignIn(account);
        Command::SignIn(account)
    }
}

#[cfg(test)]
#[path = "../../../../tests/dot-desktop/sign_in/screen.rs"]
mod tests;
