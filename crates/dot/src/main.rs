//! Dot binary: assembles the components and starts the interface.

mod log;

use std::env;
use std::path::PathBuf;
use std::process::ExitCode;
use std::thread;

use dot_api::{Effort, Instructions, Model, PermissionMode, Reading, Summaries};
use dot_auth::{anthropic_connector, locate_ant, locate_claude_code};
use dot_compact::ShapeCompactor;
use dot_i18n::{FluentTranslator, Language};
use dot_model::{ClaudeChat, summaries_for};
use dot_process::LoginPath;
use dot_session::{LocalSession, launch_folder};
use dot_tool::{LocalToolbox, LocalWorkspace};
use objc2_foundation::NSLocale;

const WORKSPACE: &str = "Library/Application Support/Dot";
const SHELL_STAND_IN: &str = "dot-shell";
const MODEL: Model = Model::OPUS_5_5;
const EFFORT: Effort = Effort::High;
const PERMISSIONS: PermissionMode = PermissionMode::Bypass;
const SUMMARIES: Summaries = Summaries::WhenTheyPay;
const READING: Reading = Reading::ByClaude;
const INSTRUCTIONS: Instructions = Instructions::HandedByDot;

fn main() -> ExitCode {
    let home = env::var_os("HOME").map(PathBuf::from);
    if let Some(home) = home.as_deref() {
        let started = log::Log::open(&log::log_folder(home))
            .map_err(Into::into)
            .and_then(log::start_logging);
        if let Err(error) = started {
            eprintln!("Dot keeps no log: {error}");
        }
    }
    log_panics();

    let preferred_languages = NSLocale::preferredLanguages();
    let language = Language::negotiate(preferred_languages.iter().map(|tag| tag.to_string()));
    let translator = FluentTranslator::new(language);

    let path = env::var_os("PATH");
    let claude_code = locate_claude_code(home.as_deref(), path.as_deref());
    let ant = locate_ant(home.as_deref(), path.as_deref());
    let connector = anthropic_connector(claude_code.clone(), ant, home.as_deref());

    let login_path = LoginPath::new(env::var_os("SHELL").map(PathBuf::from));
    let folder = launch_folder(env::args_os().skip(1), env::current_dir().ok());
    let settings_roots: Vec<_> = home.iter().chain(&folder).map(PathBuf::as_path).collect();
    let summaries = summaries_for(SUMMARIES, &settings_roots);
    let workspace = home.map(|home| home.join(WORKSPACE));
    let files = LocalWorkspace::new(folder.clone());
    let shell = shell_stand_in();
    let open = {
        let login_path = login_path.clone();
        move || {
            let toolbox = LocalToolbox::new(folder.clone()).login_path(login_path.clone());
            let chat = ClaudeChat::new(
                MODEL,
                EFFORT,
                claude_code.clone(),
                workspace.clone(),
                shell.clone(),
                toolbox,
                ShapeCompactor,
            )
            .summarizing(summaries)
            .reading(READING)
            .instructions(INSTRUCTIONS)
            .login_path(login_path.clone());
            (LocalSession::new(MODEL, EFFORT, folder.clone()), chat)
        }
    };

    let ran = thread::scope(|scope| {
        scope.spawn(|| login_path.read());
        dot_desktop::run(&translator, connector, open, (PERMISSIONS, files))
    });
    let Err(error) = ran else {
        return ExitCode::SUCCESS;
    };

    let described = error.describe(&translator);
    tracing::error!("Dot stopped: {described}");
    eprintln!("{described}");
    ExitCode::FAILURE
}

fn shell_stand_in() -> Option<PathBuf> {
    let beside_dot = env::current_exe().ok()?.parent()?.join(SHELL_STAND_IN);
    if !beside_dot.is_file() {
        tracing::warn!("command outputs are not shortened: dot-shell is not beside dot");
        return None;
    }
    Some(beside_dot)
}

fn log_panics() {
    let report = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic| {
        tracing::error!("{panic}");
        report(panic);
    }));
}

#[cfg(test)]
#[path = "../../../tests/memory_ceiling.rs"]
mod memory_ceiling;
