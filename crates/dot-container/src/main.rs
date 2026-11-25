//! `dot-agents`: the agents that live in containers, from a terminal or for Claude.

use std::env;
use std::error::Error;
use std::io::{self, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::thread;

use dot_api::Cancellation;
use dot_container::{
    AgentName, AgentSpec, Call, CallEvent, Fleet, Project, Roster, agents_folder, locate_engine,
};
use signal_hook::consts::{SIGHUP, SIGINT, SIGTERM};
use signal_hook::iterator::Signals;

const USAGE: &str = "dot-agents: agents that live in containers

  dot-agents new NAME                          describe a new agent
  dot-agents build                             build the image of the agents
  dot-agents list                              the agents, and their containers
  dot-agents login NAME                        sign the agent in to Claude
  dot-agents call NAME [--continue] [PROMPT]   call it; the prompt is read from the input if left out
  dot-agents start NAME | stop NAME            wake it, put it to sleep
  dot-agents remove NAME                       remove its container; its home is kept
  dot-agents mcp                               serve the calls to Claude

Options, before the command:
  --folder PATH   the project it works in; the current folder if left out
  --agents PATH   the folder where the agents are described";
const FOLDER_OPTION: &str = "--folder";
const AGENTS_OPTION: &str = "--agents";
const CONTINUE_OPTION: &str = "--continue";
const LONGEST_PROMPT: u64 = 4 * 1_024 * 1_024;

type Failure = Box<dyn Error + Send + Sync>;

struct Place {
    home: Option<PathBuf>,
    folder: Option<PathBuf>,
    agents: Option<PathBuf>,
}

fn main() -> ExitCode {
    let cancellation = Cancellation::new();
    if let Err(error) = cancel_on_signals(&cancellation) {
        eprintln!("dot-agents: {error}");
        return ExitCode::FAILURE;
    }
    match run(&cancellation) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("dot-agents: {error}");
            let mut cause = error.source();
            while let Some(told) = cause {
                eprintln!("  {told}");
                cause = told.source();
            }
            ExitCode::FAILURE
        }
    }
}

fn run(cancellation: &Cancellation) -> Result<(), Failure> {
    let mut arguments = env::args().skip(1).peekable();
    let mut place = Place {
        home: env::var_os("HOME").map(PathBuf::from),
        folder: None,
        agents: None,
    };
    while let Some(option) =
        arguments.next_if(|argument| [FOLDER_OPTION, AGENTS_OPTION].contains(&argument.as_str()))
    {
        let value = arguments.next().map(PathBuf::from).ok_or(USAGE)?;
        if option == FOLDER_OPTION {
            place.folder = Some(value);
        } else {
            place.agents = Some(value);
        }
    }
    let command = arguments.next().ok_or(USAGE)?;
    let rest: Vec<String> = arguments.collect();
    let name =
        || -> Result<AgentName, Failure> { Ok(AgentName::parse(rest.first().ok_or(USAGE)?)?) };
    match command.as_str() {
        "new" => {
            let path = Roster::describe(&place.agents()?, &name()?)?;
            println!("{}", path.display());
            Ok(())
        }
        "build" => {
            eprintln!("building {}…", AgentSpec::default_image());
            fleet_engine()?.build(AgentSpec::default_image(), cancellation)?;
            Ok(())
        }
        "list" => list(&place, cancellation),
        "start" => {
            let (name, spec) = place.agent(name()?)?;
            let container = fleet()?.start((&name, &spec), &place.project()?, cancellation)?;
            println!("{container}");
            Ok(())
        }
        "stop" => Ok(fleet()?.stop(&name()?, &place.project()?, cancellation)?),
        "remove" => Ok(fleet()?.remove(&name()?, &place.project()?, cancellation)?),
        "login" => {
            let (name, spec) = place.agent(name()?)?;
            fleet()?.sign_in((&name, &spec), &place.project()?, cancellation)?;
            Ok(())
        }
        "call" => call(&place, name()?, &rest[1..], cancellation),
        "mcp" => {
            let exchange = (BufReader::new(io::stdin()), io::stdout());
            let project = place.project()?;
            dot_container::serve(
                &fleet()?,
                &place.agents()?,
                &project,
                exchange,
                cancellation,
            )?;
            Ok(())
        }
        _ => Err(USAGE.into()),
    }
}

fn call(
    place: &Place,
    name: AgentName,
    rest: &[String],
    cancellation: &Cancellation,
) -> Result<(), Failure> {
    let goes_on = rest.first().is_some_and(|first| first == CONTINUE_OPTION);
    let words = &rest[usize::from(goes_on)..];
    let prompt = if words.is_empty() {
        let mut prompt = String::new();
        io::stdin()
            .take(LONGEST_PROMPT)
            .read_to_string(&mut prompt)?;
        prompt
    } else {
        words.join(" ")
    };
    let (name, spec) = place.agent(name)?;
    let asked = Call {
        prompt: &prompt,
        goes_on,
    };
    let answer = fleet()?.call(
        (&name, &spec),
        &place.project()?,
        &asked,
        cancellation,
        &mut |event| match event {
            CallEvent::Tool(tool) => eprintln!("· {tool}"),
            CallEvent::Text(text) => eprintln!("{text}"),
        },
    )?;
    writeln!(io::stdout(), "{}", answer.text)?;
    Ok(())
}

fn list(place: &Place, cancellation: &Cancellation) -> Result<(), Failure> {
    let roster = Roster::read(&place.agents()?)?;
    for (name, spec) in roster.agents() {
        println!("{name}  {}", spec.description);
    }
    for refused in roster.refused() {
        let cause = refused.source().map_or_default(ToString::to_string);
        eprintln!("unusable: {refused}: {cause}");
    }
    for listed in fleet()?.list(cancellation)? {
        println!("  {} {} {}", listed.agent, listed.state, listed.folder);
    }
    Ok(())
}

impl Place {
    fn agents(&self) -> Result<PathBuf, Failure> {
        if let Some(agents) = &self.agents {
            return Ok(agents.clone());
        }
        let home = self.home.as_deref().ok_or("HOME is not set")?;
        Ok(agents_folder(home, |variable| env::var_os(variable)))
    }

    fn project(&self) -> Result<Project, Failure> {
        let folder = self.folder.as_deref().unwrap_or_else(|| Path::new("."));
        Ok(Project::new(folder, self.home.as_deref())?)
    }

    fn agent(&self, name: AgentName) -> Result<(AgentName, AgentSpec), Failure> {
        let roster = Roster::read(&self.agents()?)?;
        let spec = roster
            .get(&name)
            .cloned()
            .ok_or_else(|| format!("no agent is called {name}: `dot-agents new {name}`"))?;
        Ok((name, spec))
    }
}

fn fleet_engine() -> Result<dot_container::Engine, Failure> {
    locate_engine(env::var_os("PATH").as_deref())
        .ok_or_else(|| "neither Docker nor Podman is installed".into())
}

fn fleet() -> Result<Fleet, Failure> {
    Ok(Fleet::new(fleet_engine()?))
}

/// Ends the calls going on before the program itself: without it, an agent would go on alone in
/// its container.
fn cancel_on_signals(cancellation: &Cancellation) -> io::Result<()> {
    let mut signals = Signals::new([SIGHUP, SIGINT, SIGTERM])?;
    let cancellation = cancellation.clone();
    thread::spawn(move || {
        for _ in signals.forever() {
            cancellation.cancel();
        }
    });
    Ok(())
}
