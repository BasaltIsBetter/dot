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
    assert_eq!(
        server["args"],
        json!([
            "--folder",
            "/p/a \"b\"",
            "--agents",
            "/home/me/.config/dot/agents",
            "mcp"
        ])
    );
}

#[test]
fn a_folder_whose_path_is_no_text_has_no_agents() {
    let folder = Path::new(OsStr::from_bytes(b"/p/\xff"));

    assert_eq!(server().configuration(folder), None);
}
