use std::sync::Arc;

use dot_api::{Cancellation, ConnectError, ConnectFailure, Connection, Connector, Translator};

use crate::canvas::Line;
use crate::sign_in::screen::{Account, Command, Input, Method, Report, Screen};
use crate::sign_in::view;
use crate::worker::{Mailbox, Workers};

pub(crate) struct SignIn<K, M> {
    connector: Arc<K>,
    workers: Workers<M>,
    screen: Screen,
    signing_in: Option<Cancellation>,
}

impl<K: Connector + 'static, M: Mailbox<Report>> SignIn<K, M> {
    pub(crate) fn new(connector: Arc<K>, mailbox: M) -> Self {
        Self {
            connector,
            workers: Workers::new(mailbox),
            screen: Screen::Checking,
            signing_in: None,
        }
    }

    pub(crate) fn start(&mut self) {
        let connector = Arc::clone(&self.connector);
        self.workers.spawn(
            move |_| Report::Checked(connector.current()),
            || Some(Report::Checked(Err(crashed()))),
        );
    }

    pub(crate) fn input(&mut self, input: Input) {
        let command = self.screen.input(input);
        self.run(command);
    }

    pub(crate) fn report(&mut self, report: Report) {
        if let Some(error) = report.error() {
            tracing::warn!(
                failure = error.failure().text().id(),
                "connecting to Claude failed"
            );
        }
        self.screen.report(report);
    }

    pub(crate) fn lines(&self, translator: &impl Translator) -> Vec<Line> {
        view::lines(&self.screen, translator)
    }

    pub(crate) const fn connection(&self) -> Option<Connection> {
        match self.screen {
            Screen::Connected(connection) => Some(connection),
            _ => None,
        }
    }

    pub(crate) fn method_at(&self, row: usize) -> Option<Method> {
        view::method_at(&self.screen, row)
    }

    pub(crate) fn stop(&mut self) {
        self.workers.stop_all();
    }

    fn run(&mut self, command: Command) {
        match command {
            Command::Nothing => {}
            Command::CheckKey(key) => {
                let connector = Arc::clone(&self.connector);
                self.workers.spawn(
                    move |_| Report::KeyChecked(connector.connect_with_api_key(&key)),
                    || Some(Report::KeyChecked(Err(crashed()))),
                );
            }
            Command::SignIn(account) => {
                let connector = Arc::clone(&self.connector);
                let cancellation = self.workers.spawn(
                    move |cancellation| {
                        Report::SignedIn(match account {
                            Account::ClaudeCode => connector.connect_with_claude_code(cancellation),
                            Account::Anthropic => {
                                connector.connect_with_anthropic_account(cancellation)
                            }
                        })
                    },
                    || Some(Report::SignedIn(Err(crashed()))),
                );
                self.signing_in = Some(cancellation);
            }
            Command::CancelSignIn => {
                if let Some(cancellation) = self.signing_in.take() {
                    cancellation.cancel();
                }
            }
        }
    }
}

fn crashed() -> ConnectError {
    ConnectError::new(ConnectFailure::Crashed)
}

#[cfg(test)]
#[path = "../../../../tests/dot-desktop/sign_in/flow.rs"]
pub(crate) mod tests;
