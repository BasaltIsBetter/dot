//! Dot binary: assembles the components and starts the interface.

use std::env;
use std::path::PathBuf;
use std::process::ExitCode;

use dot_auth::{AnthropicConnector, locate_claude_code};
use dot_i18n::{FluentTranslator, Language};
use objc2_foundation::NSLocale;

fn main() -> ExitCode {
    let preferred_languages = NSLocale::preferredLanguages();
    let language = Language::negotiate(preferred_languages.iter().map(|tag| tag.to_string()));
    let translator = FluentTranslator::new(language);

    let home = env::var_os("HOME").map(PathBuf::from);
    let claude_code = locate_claude_code(home.as_deref(), env::var_os("PATH").as_deref());
    let connector = AnthropicConnector::new(claude_code);

    let Err(error) = dot_desktop::run(&translator, connector) else {
        return ExitCode::SUCCESS;
    };

    eprintln!("{}", error.describe(&translator));
    ExitCode::FAILURE
}
