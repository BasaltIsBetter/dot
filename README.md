# Dot

**English** · [Français](README.fr.md) · [Español](README.es.md) · [Português](README.pt.md)

A coding agent for macOS and Linux, in the spirit of Claude Code, built to be faster, leaner and
more thoughtful: a black window drawn on the GPU, in the style of
[Spaceship](https://chuanqisun.github.io/spaceship/), where you write to Claude, who answers as a
stream and works in the project's folder with its tools (read, write, run commands, search, the
web).

Dot speaks English, French, Spanish and Brazilian Portuguese: it follows the system's languages,
and the “Language” setting changes it at once.

## Install

Download the latest version from the [releases](https://github.com/BasaltIsBetter/dot/releases):

- **Mac (Apple silicon)**: unzip `Dot-<version>-macos-arm64.zip` and move `Dot.app` to
  Applications. These pre-releases are not signed by Apple yet: the first time, allow it in System
  Settings › Privacy & Security › “Open Anyway”, or run
  `xattr -dr com.apple.quarantine /Applications/Dot.app`.
- **Linux (x86_64 or arm64, glibc 2.35 or newer)**: unpack the archive of your processor, then run
  its `install.sh`. Dot goes to `~/.local/share/dot-app`, the `dot` command to `~/.local/bin`,
  with an entry in the application launcher.

From source, `./crates/dot/macos/install.sh` builds the app and installs it in `~/Applications`,
with `dot` in `~/.local/bin`.

## Requirements

- macOS on Apple silicon or Intel, with a Metal GPU; or Linux under Wayland or X11, with a Vulkan
  or OpenGL driver (`mesa-vulkan-drivers` is enough, without a graphics card). Under X11,
  `libxkbcommon-x11` is needed too. An API key is kept by the desktop keyring (Secret Service:
  GNOME Keyring, KWallet…); without a keyring, as under WSL, sign in through Claude Code or `ant`.
- Rust: the version is pinned in `rust-toolchain.toml` (rustup installs it by itself).
- To connect: a Claude plan through [Claude Code](https://claude.com/claude-code), an Anthropic
  account through `ant`, or an Anthropic API key.

## Run

```bash
cargo run --release -- ~/my-project          # Dot opened on a folder
./crates/dot/macos/bundle.sh                 # the Mac app: target/release/bundle/macos/Dot.app
./crates/dot/linux/bundle.sh                 # the Linux archive: target/release/bundle/linux/*.tar.gz
```

Shortcuts: return sends, shift+return starts a new line, esc stops the reply, cmd+V pastes text or
an image, cmd+N starts a new conversation, ↑ and ↓ bring back the prompts already sent. Under
Linux, ctrl replaces cmd, and ctrl+Q quits.

The log is written to `~/Library/Logs/Dot/dot.log` on Mac, to `~/.local/state/dot/dot.log` under
Linux (1 MiB at most, the previous one kept in `dot.log.1`); it never holds a key or a token.

## Agents in containers

Agents can live in Docker (or Podman) containers and be called on demand, from a terminal or by
Claude in Dot (`@reviewer look at this file`): each agent keeps its container per project, woken
when needed, and its memory in a volume of its own.

```bash
cargo build --release -p dot-container           # the command: target/release/dot-agents
dot-agents new reviewer                          # describes an agent, to edit afterwards
dot-agents build                                 # builds the image, with Claude Code inside
dot-agents --folder ~/my-project login reviewer  # connects the agent to Claude, once
dot-agents --folder ~/my-project call reviewer "Review the last commit"
dot-agents list                                  # the agents and their containers
dot-agents --folder ~/my-project stop reviewer   # puts the agent to sleep
```

Agents are described in JSON in `~/.config/dot/agents/` (`~/Library/Application
Support/Dot/agents/` on Mac), never in a project: opening an unknown repository starts none.

| Field | Role | Default |
|---|---|---|
| `description` | what Claude reads to pick the agent | empty |
| `workspace` | access to the project's folder: `none`, `read`, `write` | `read` |
| `network` | `bridge`, or `none` for an agent without network (which therefore cannot be Claude) | `bridge` |
| `memory`, `cpus`, `processes` | the container's limits | `2g`, 2, 512 |
| `minutes` | the longest a call may last | 30 |
| `model`, `instructions` | model and instructions of Claude Code in the container | none |
| `command` | program that answers instead of Claude Code (request on its input, answer on its output) | Claude Code |
| `image` | the container's image; it must provide `sh`, `tr`, `grep` and a `/home/agent` everyone can write to | `dot-agent:latest` |
| `env` | names of the variables passed on to the agent | none |

The container has no privilege (`--cap-drop ALL`, `no-new-privileges`), runs as the user and sees
only the project, at the same path as on the machine; the whole home folder and the root of the
disk are refused. Inside, Claude Code asks for no permission: the container bounds the agent. An
agent in `write` can therefore change anything in the project, `.git/hooks` included. Claude can
call the agents in Dot when `dot-agents` sits beside `dot` and the connection goes through Claude
Code; not yet with an API key.

## Check

```bash
cargo fmt --all --check
cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
cargo test --locked --workspace
RUSTDOCFLAGS="-D warnings" cargo doc --locked --workspace --no-deps
cargo deny check
cargo bench -p dot-desktop --features bench    # criterion benches
```

Continuous integration (`.github/workflows/ci.yml`) runs the same checks on every push and pull
request. To contribute, see [`CONTRIBUTING.md`](CONTRIBUTING.md); to report a vulnerability,
[`SECURITY.md`](SECURITY.md).

## Layout

One crate per responsibility in `crates/`, their tests apart in `tests/<crate>/`:

| Crate | Role |
|---|---|
| `dot` | the binary: assembles the components, keeps the log |
| `dot-api` | shared contracts and types (conversation, tools, permissions, session, texts) |
| `dot-auth` | the connection to Claude |
| `dot-compact` | shortens what Claude reads from commands |
| `dot-container` | the agents that live in containers, and their `dot-agents` command |
| `dot-desktop` | the Mac and Linux app: window, conversation, status bar (winit + wgpu) |
| `dot-i18n` | the translations (Fluent, `crates/dot-i18n/locales/`) |
| `dot-model` | Claude's replies: Messages API or Claude Code |
| `dot-process` | the programs Dot starts (environment, group, stop, wait) and the bounded, line by line reading of what it reads from them |
| `dot-session` | a window's session: folder, git branch, usage; and what is kept between two launches |
| `dot-shell` | stands in for the shell of Claude Code's commands, to shorten what they print |
| `dot-tool` | Claude's tools on the machine |
| `dot-transcript` | the conversation as it is read, without window or GPU: history, line wrapping, motion |

## License

Licensed under [Apache 2.0](LICENSE); the credits are in [`NOTICE`](NOTICE).

- Part of the rules of `dot-compact` is adapted from [RTK](https://github.com/rtk-ai/rtk)
  (Apache 2.0): see [`crates/dot-compact/third-party/rtk/`](crates/dot-compact/third-party/rtk/).
- The design of the interface is inspired by [Spaceship](https://github.com/chuanqisun/spaceship),
  by Sun (MIT); no code is taken from it.
- The embedded JetBrains Mono font is licensed under the SIL Open Font License 1.1.
