use std::sync::Arc;

use dot_api::{Cancellation, Connection, Connector, Translator};

use crate::canvas::Line;
use crate::sign_in::screen::{Command, Input, Method, Report, Screen};
use crate::sign_in::view;
use crate::worker::{Mailbox, Workers};

pub(crate) struct SignIn<K, M> {
    connector: Arc<K>,
    workers: Workers<M>,
    screen: Screen,
    claude_code: Option<Cancellation>,
}

impl<K: Connector + 'static, M: Mailbox<Report>> SignIn<K, M> {
    pub(crate) fn new(connector: K, mailbox: M) -> Self {
        Self {
            connector: Arc::new(connector),
            workers: Workers::new(mailbox),
            screen: Screen::Checking,
            claude_code: None,
        }
    }

    pub(crate) fn start(&mut self) {
        let connector = Arc::clone(&self.connector);
        self.workers
            .spawn(move |_| Report::Checked(connector.current()));
    }

    pub(crate) fn input(&mut self, input: Input) {
        let command = self.screen.input(input);
        self.run(command);
    }

    pub(crate) fn report(&mut self, report: Report) {
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
                self.workers
                    .spawn(move |_| Report::KeyChecked(connector.connect_with_api_key(&key)));
            }
            Command::SignInWithClaudeCode => {
                let connector = Arc::clone(&self.connector);
                let cancellation = self.workers.spawn(move |cancellation| {
                    Report::ClaudeCodeSignedIn(connector.connect_with_claude_code(cancellation))
                });
                self.claude_code = Some(cancellation);
            }
            Command::CancelClaudeCode => {
                if let Some(cancellation) = self.claude_code.take() {
                    cancellation.cancel();
                }
            }
        }
    }
}

#[cfg(test)]
#[path = "../../../../tests/dot-desktop/sign_in/flow.rs"]
pub(crate) mod tests;
