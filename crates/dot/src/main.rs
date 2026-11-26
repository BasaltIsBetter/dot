//! Dot binary: assembles the components and starts the interface.

mod log;
mod machine;

use std::backtrace::Backtrace;
use std::env;
use std::hash::{BuildHasher, RandomState};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::thread;

use dot_api::{
    CaretBlink, Column, Effort, Hue, Instructions, InterfaceSize, Language, Model,
    OutputCompaction, PermissionMode, Preferences, Reading, ReadingLine, SessionsColumn, Settings,
    Summaries, Toggle, Tuning,
};
use dot_auth::{anthropic_connector, locate_ant, locate_claude_code};
use dot_compact::ShapeCompactor;
use dot_container::{agents_folder, has_agents};
use dot_desktop::Choices;
use dot_i18n::{FluentTranslator, negotiate};
use dot_model::{ClaudeChat, summaries_for};
use dot_process::LoginPath;
use dot_session::{FilePreferences, LocalSession, launch_folder};
use dot_tool::{LocalToolbox, LocalWorkspace};

const SHELL_STAND_IN: &str = "dot-shell";
const AGENTS: &str = "dot-agents";
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
    let places = home.as_deref().map(machine::places);
    if let Some(places) = &places {
        let started = log::Log::open(&places.logs)
            .map_err(Into::into)
            .and_then(log::start_logging);
        if let Err(error) = started {
            eprintln!("Dot keeps no log: {error}");
        }
    }
    log_panics();

    let language = negotiate(machine::preferred_languages());
    let defaults = defaults(language);
    let workspace = places.map(|places| places.workspace);
    let preferences = FilePreferences::new(workspace.clone());
    let settings = preferences.load(&defaults);
    let translator = FluentTranslator::new(settings.language);

    let path = env::var_os("PATH");
    let claude_code = locate_claude_code(home.as_deref(), path.as_deref());
    let ant = locate_ant(home.as_deref(), path.as_deref());
    let connector = anthropic_connector(claude_code.clone(), ant, workspace.as_deref());

    let login_path = LoginPath::new(env::var_os("SHELL").map(PathBuf::from));
    let folder = launch_folder(env::args_os().skip(1), env::current_dir().ok());
    let settings_roots: Vec<_> = home.iter().chain(&folder).map(PathBuf::as_path).collect();
    let summaries_allowed = summaries_for(Summaries::WhenTheyPay, &settings_roots);
    let files = LocalWorkspace::new(folder.clone());
    let shell = shell_stand_in();
    let agents = home.as_deref().and_then(agents);
    let launched_in = folder.clone();
    let open = {
        let login_path = login_path.clone();
        move |folder: Option<PathBuf>| {
            let toolbox = LocalToolbox::new(folder.clone()).login_path(login_path.clone());
            let mut chat = ClaudeChat::new(
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
            if let Some((program, described_in)) = agents.clone() {
                chat = chat.agents(program, described_in);
            }
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
    let stand_in = beside_dot(SHELL_STAND_IN);
    if stand_in.is_none() {
        tracing::warn!("command outputs are not shortened: dot-shell is not beside dot");
    }
    stand_in
}

/// The program that calls the agents of containers and the folder that describes them, when the
/// user described some.
fn agents(home: &Path) -> Option<(PathBuf, PathBuf)> {
    let described_in = agents_folder(home, |variable| env::var_os(variable));
    if !has_agents(&described_in) {
        return None;
    }
    let program = beside_dot(AGENTS);
    if program.is_none() {
        tracing::warn!("Claude cannot call the agents: dot-agents is not beside dot");
    }
    Some((program?, described_in))
}

fn beside_dot(program: &str) -> Option<PathBuf> {
    let path = env::current_exe().ok()?.parent()?.join(program);
    path.is_file().then_some(path)
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
