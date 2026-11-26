use std::ffi::OsStr;
use std::os::unix::ffi::OsStrExt;

use super::*;

fn server() -> AgentServer {
    AgentServer {
        program: PathBuf::from("/opt/dot/bin/dot-agents"),
        described_in: PathBuf::from("/home/me/.config/dot/agents"),
    }
}

#[test]
fn has_claude_code_start_the_program_for_the_project() {
    let configuration = server().configuration(Path::new("/p/a \"b\"")).unwrap();

    let read: serde_json::Value = serde_json::from_str(&configuration).unwrap();
    let server = &read["mcpServers"]["dot-agents"];
    assert_eq!(server["command"], "/opt/dot/bin/dot-agents");
    assert_eq!(server["args"], json!(["mcp"]));
    assert_eq!(server["env"]["DOT_AGENTS_PROJECT"], "/p/a \"b\"");
    assert_eq!(
        server["env"]["DOT_AGENTS_FOLDER"],
        "/home/me/.config/dot/agents"
    );
}

#[test]
fn no_path_reaches_the_line_a_shell_reads() {
    let project = "/p/x; touch $(y)";

    let configuration = server().configuration(Path::new(project)).unwrap();

    let read: serde_json::Value = serde_json::from_str(&configuration).unwrap();
    let server = &read["mcpServers"]["dot-agents"];
    assert!(!server["args"].to_string().contains("touch"));
    assert_eq!(server["env"]["DOT_AGENTS_PROJECT"], project);
}

#[test]
fn a_path_claude_code_would_expand_has_no_agents() {
    assert_eq!(server().configuration(Path::new("/p/${HOME}")), None);
}

#[test]
fn a_folder_whose_path_is_no_text_has_no_agents() {
    let folder = Path::new(OsStr::from_bytes(b"/p/\xff"));

    assert_eq!(server().configuration(folder), None);
}
