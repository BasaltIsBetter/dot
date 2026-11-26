use super::*;
use crate::engine::tests::Scratch;

#[test]
fn a_missing_folder_describes_no_agent() {
    let scratch = Scratch::new("roster-missing");
    let folder = scratch.path().join("nowhere");

    assert_eq!(Roster::read(&folder).unwrap().agents().count(), 0);
    assert!(!has_agents(&folder));
}

#[test]
fn reads_each_agent_and_sets_aside_what_it_cannot_use() {
    let scratch = Scratch::new("roster-read");
    scratch.write("reviewer.json", r#"{"description": "Reviews"}"#);
    scratch.write("broken.json", "{");
    scratch.write("Not A Name.json", "{}");
    scratch.write("notes.txt", "{}");

    let roster = Roster::read(scratch.path()).unwrap();

    let names: Vec<&str> = roster.agents().map(|(name, _)| name.as_str()).collect();
    assert_eq!(names, ["reviewer"]);
    assert_eq!(roster.refused().len(), 2);
    assert!(has_agents(scratch.path()));
}

#[test]
fn a_file_too_large_is_set_aside() {
    let scratch = Scratch::new("roster-large");
    scratch.write("large.json", &" ".repeat(300 * 1_024));

    let roster = Roster::read(scratch.path()).unwrap();

    assert_eq!(roster.agents().count(), 0);
    assert_eq!(roster.refused().len(), 1);
}

#[test]
fn the_agent_it_describes_can_be_used_and_is_never_written_over() {
    let scratch = Scratch::new("roster-describe");
    let folder = scratch.path().join("agents");
    let name = AgentName::parse("reviewer").unwrap();

    Roster::describe(&folder, &name).unwrap();

    let roster = Roster::read(&folder).unwrap();
    assert!(roster.get(&name).is_some());
    assert!(Roster::describe(&folder, &name).is_err());
}

#[cfg(not(target_os = "macos"))]
#[test]
fn looks_where_the_user_keeps_their_configuration() {
    let home = Path::new("/home/me");

    assert_eq!(
        agents_folder(home, |_| None),
        Path::new("/home/me/.config/dot/agents")
    );
    assert_eq!(
        agents_folder(home, |_| Some("/cfg".into())),
        Path::new("/cfg/dot/agents")
    );
    assert_eq!(
        agents_folder(home, |_| Some("cfg".into())),
        Path::new("/home/me/.config/dot/agents")
    );
}
