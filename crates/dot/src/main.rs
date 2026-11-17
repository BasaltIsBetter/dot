//! Dot binary: assembles the components and starts the interface.

use std::env;
use std::path::PathBuf;
use std::process::ExitCode;

use dot_api::Model;
use dot_auth::{AnthropicConnector, locate_claude_code};
use dot_i18n::{FluentTranslator, Language};
use dot_session::{LocalSession, launch_folder};
use objc2_foundation::NSLocale;

fn main() -> ExitCode {
    let preferred_languages = NSLocale::preferredLanguages();
    let language = Language::negotiate(preferred_languages.iter().map(|tag| tag.to_string()));
    let translator = FluentTranslator::new(language);

    let home = env::var_os("HOME").map(PathBuf::from);
    let claude_code = locate_claude_code(home.as_deref(), env::var_os("PATH").as_deref());
    let connector = AnthropicConnector::new(claude_code, home.as_deref());

    let folder = launch_folder(env::args_os().skip(1), env::current_dir().ok());
    let session = LocalSession::new(Model::OPUS_5_5, folder);

    let Err(error) = dot_desktop::run(&translator, connector, session) else {
        return ExitCode::SUCCESS;
    };

    eprintln!("{}", error.describe(&translator));
    ExitCode::FAILURE
}
