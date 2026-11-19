//! Dot binary: assembles the components and starts the interface.

use std::env;
use std::path::PathBuf;
use std::process::ExitCode;

use dot_api::{Effort, Model, PermissionMode};
use dot_auth::{AnthropicConnector, locate_ant, locate_claude_code};
use dot_i18n::{FluentTranslator, Language};
use dot_model::ClaudeChat;
use dot_session::{LocalSession, launch_folder};
use dot_tool::LocalToolbox;
use objc2_foundation::NSLocale;

const WORKSPACE: &str = "Library/Application Support/Dot";
const MODEL: Model = Model::OPUS_5_5;
const EFFORT: Effort = Effort::High;
const PERMISSIONS: PermissionMode = PermissionMode::Bypass;

fn main() -> ExitCode {
    let preferred_languages = NSLocale::preferredLanguages();
    let language = Language::negotiate(preferred_languages.iter().map(|tag| tag.to_string()));
    let translator = FluentTranslator::new(language);

    let home = env::var_os("HOME").map(PathBuf::from);
    let path = env::var_os("PATH");
    let claude_code = locate_claude_code(home.as_deref(), path.as_deref());
    let ant = locate_ant(home.as_deref(), path.as_deref());
    let connector = AnthropicConnector::new(claude_code.clone(), ant, home.as_deref());

    let folder = launch_folder(env::args_os().skip(1), env::current_dir().ok());
    let toolbox = LocalToolbox::new(folder.clone());
    let workspace = home.map(|home| home.join(WORKSPACE));
    let chat = ClaudeChat::new(MODEL, EFFORT, claude_code, workspace, toolbox);
    let session = LocalSession::new(MODEL, EFFORT, folder);

    let Err(error) = dot_desktop::run(&translator, connector, session, chat, PERMISSIONS) else {
        return ExitCode::SUCCESS;
    };

    eprintln!("{}", error.describe(&translator));
    ExitCode::FAILURE
}
