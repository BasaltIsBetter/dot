//! Dot's stand-in for the shell of Claude Code's commands: reads what Claude Code hands it and
//! either lets the real shell take its place or shortens what the command prints.

use std::env;
use std::ffi::OsString;
use std::io;
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::{self, Command, ExitCode, Stdio};
use std::time::SystemTime;

use dot_api::WHOLE_OUTPUTS_FOLDER;
use dot_process::{Errors, LIVE_OUTPUT_VARIABLE, LiveOutput, Watched, relay};
use dot_shell::{
    COULD_NOT_RUN, PACE, Plan, Shortening, WholeOutputs, exit_code, in_own_temporary_folder,
    leave_passing_on, plan, share_a_file,
};

const SHELL_VARIABLE: &str = "SHELL";
const TEMPORARY_FOLDER_VARIABLE: &str = "TMPDIR";
const COMMAND_FLAG: &str = "-c";
const SHORTEST_TEMPORARY_FOLDER: &str = "/tmp";

fn main() -> ExitCode {
    let arguments = env::args_os().skip(1).collect();
    match plan(arguments, env::var_os(SHELL_VARIABLE)) {
        Plan::Hand { shell, arguments } => hand(&shell, arguments),
        Plan::Shorten {
            shell,
            script,
            command,
        } => shorten(&shell, &script, command),
    }
}

fn hand(shell: &Path, arguments: Vec<OsString>) -> ExitCode {
    let error = Command::new(shell).args(arguments).exec();
    eprintln!("dot-shell: {}: {error}", shell.display());
    ExitCode::from(COULD_NOT_RUN)
}

fn shorten(shell: &Path, script: &str, command: String) -> ExitCode {
    let mut real = Command::new(shell);
    real.arg(COMMAND_FLAG).arg(script);
    let errors = if share_a_file(&io::stdout(), &io::stderr()) {
        Errors::Joined
    } else {
        Errors::Apart
    };

    let mut whole_outputs = WholeOutputs::new(
        Path::new(SHORTEST_TEMPORARY_FOLDER).join(WHOLE_OUTPUTS_FOLDER),
        SystemTime::now(),
        process::id(),
    );
    if let Some(fallback) = in_own_temporary_folder(env::var_os(TEMPORARY_FOLDER_VARIABLE)) {
        whole_outputs = whole_outputs.or_else_in(fallback);
    }
    let live = env::var_os(LIVE_OUTPUT_VARIABLE)
        .and_then(|address| LiveOutput::connect(Path::new(&address), &command).ok());
    let mut watched = Watched::new(Shortening::of(command, whole_outputs), live);
    let ended = match relay(real, errors, PACE, &mut watched, &mut io::stdout().lock()) {
        Ok(ended) => ended,
        Err(error) => {
            eprintln!("dot-shell: {}: {error}", shell.display());
            return ExitCode::from(COULD_NOT_RUN);
        }
    };
    if let Some(output) = ended.left_open
        && let Err(error) = leave_passing_on(output, Stdio::inherit())
    {
        eprintln!("dot-shell: {error}");
    }
    ExitCode::from(exit_code(ended.status))
}
