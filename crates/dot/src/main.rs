//! Dot binary: assembles the components and starts the interface.

mod log;

use std::backtrace::Backtrace;
use std::env;
use std::hash::{BuildHasher, RandomState};
use std::path::PathBuf;
use std::process::ExitCode;
use std::thread;

use dot_api::{
    CaretBlink, Column, Effort, Hue, Instructions, InterfaceSize, Language, Model,
    OutputCompaction, PermissionMode, Preferences, Reading, ReadingLine, SessionsColumn, Settings,
    Summaries, Toggle, Tuning,
};
use dot_auth::{anthropic_connector, locate_ant, locate_claude_code};
use dot_compact::ShapeCompactor;
use dot_desktop::Choices;
use dot_i18n::{FluentTranslator, negotiate};
use dot_model::{ClaudeChat, summaries_for};
use dot_process::LoginPath;
use dot_session::{FilePreferences, LocalSession, launch_folder};
use dot_tool::{LocalToolbox, LocalWorkspace};
use objc2_foundation::NSLocale;

const WORKSPACE: &str = "Library/Application Support/Dot";
const SHELL_STAND_IN: &str = "dot-shell";
const TUNING: Tuning = Tuning {
    model: Model::OPUS_5_5,
    effort: Effort::High,
    summaries: Summaries::WhenTheyPay,
    compaction: OutputCompaction::FULL,
};
const PERMISSIONS: PermissionMode = PermissionMode::Bypass;
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
    let language = negotiate(preferred_languages.iter().map(|tag| tag.to_string()));
    let defaults = defaults(language);
    let workspace = home.as_ref().map(|home| home.join(WORKSPACE));
    let preferences = FilePreferences::new(workspace.clone());
    let settings = preferences.load(&defaults);
    let translator = FluentTranslator::new(settings.language);

    let path = env::var_os("PATH");
    let claude_code = locate_claude_code(home.as_deref(), path.as_deref());
    let ant = locate_ant(home.as_deref(), path.as_deref());
    let connector = anthropic_connector(claude_code.clone(), ant, home.as_deref());

    let login_path = LoginPath::new(env::var_os("SHELL").map(PathBuf::from));
    let folder = launch_folder(env::args_os().skip(1), env::current_dir().ok());
    let settings_roots: Vec<_> = home.iter().chain(&folder).map(PathBuf::as_path).collect();
    let summaries_allowed = summaries_for(Summaries::WhenTheyPay, &settings_roots);
    let files = LocalWorkspace::new(folder.clone());
    let shell = shell_stand_in();
    let launched_in = folder.clone();
    let open = {
        let login_path = login_path.clone();
        move |folder: Option<PathBuf>| {
            let toolbox = LocalToolbox::new(folder.clone()).login_path(login_path.clone());
            let chat = ClaudeChat::new(
                TUNING,
                claude_code.clone(),
                workspace.clone(),
                shell.clone(),
                toolbox,
                ShapeCompactor::default(),
            )
            .summaries_allowed(summaries_allowed)
            .reading(READING)
            .instructions(INSTRUCTIONS)
            .login_path(login_path.clone());
            (LocalSession::new(folder), chat)
        }
    };

    let ran = thread::scope(|scope| {
        scope.spawn(|| login_path.read());
        let choices = Choices {
            preferences,
            settings,
            defaults,
            folder: launched_in,
            seed: RandomState::new().hash_one(()),
        };
        dot_desktop::run(&translator, connector, open, (files, choices))
    });
    let Err(error) = ran else {
        return ExitCode::SUCCESS;
    };

    let described = error.describe(&translator);
    tracing::error!("Dot stopped: {described}");
    eprintln!("{described}");
    ExitCode::FAILURE
}

const fn defaults(language: Language) -> Settings {
    Settings {
        tuning: TUNING,
        permissions: PERMISSIONS,
        color: Hue::Violet,
        state_colors: Toggle::On,
        tool_colors: Toggle::On,
        language,
        size: InterfaceSize::Large,
        column: Column::Wide,
        reading_line: ReadingLine::Middle,
        caret: CaretBlink::Blinking,
        sessions_column: SessionsColumn::Auto,
    }
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
        let backtrace = Backtrace::force_capture();
        tracing::error!("{panic}\n{backtrace}");
        report(panic);
    }));
}

#[cfg(test)]
#[path = "../../../tests/memory_ceiling.rs"]
mod memory_ceiling;
