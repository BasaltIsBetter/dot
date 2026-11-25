use proptest::prelude::*;

use super::*;

fn refused(json: &str) -> bool {
    AgentSpec::parse(json.as_bytes()).is_err()
}

#[test]
fn an_agent_left_undescribed_may_only_read() {
    let spec = AgentSpec::parse(b"{}").unwrap();

    assert_eq!(spec, AgentSpec::default());
    assert_eq!(spec.workspace, Access::Read);
    assert!(spec.uses_the_default_image());
}

#[test]
fn reads_what_an_agent_may_do() {
    let spec = AgentSpec::parse(
        br#"{"workspace": "write", "network": "none", "memory": "512m", "cpus": 0.5, "model": "haiku", "env": ["TOKEN_1"]}"#,
    )
    .unwrap();

    assert_eq!(spec.workspace, Access::Write);
    assert_eq!(spec.network, Network::None);
    assert_eq!(spec.model.as_deref(), Some("haiku"));
    assert_eq!(spec.env, ["TOKEN_1"]);
}

#[test]
fn a_field_that_does_not_exist_is_refused() {
    assert!(refused(r#"{"privileged": true}"#));
    assert!(refused(r#"{"memry": "1g"}"#));
    assert!(refused("[]"));
    assert!(refused(""));
}

#[test]
fn a_field_never_becomes_an_option_of_the_engine() {
    for json in [
        r#"{"image": "--privileged"}"#,
        r#"{"image": ""}"#,
        r#"{"image": "a b"}"#,
        r#"{"memory": "--privileged"}"#,
        r#"{"memory": "g"}"#,
        r#"{"memory": "0g"}"#,
        r#"{"memory": "2gg"}"#,
        r#"{"cpus": 0}"#,
        r#"{"cpus": -1}"#,
        r#"{"processes": 0}"#,
        r#"{"minutes": 0}"#,
        r#"{"minutes": 100000}"#,
        r#"{"env": ["KEY=value"]}"#,
        r#"{"env": ["--privileged"]}"#,
        r#"{"model": "--dangerous"}"#,
        r#"{"command": [""]}"#,
    ] {
        assert!(refused(json), "{json}");
    }
}

#[test]
fn a_name_is_one_plain_word() {
    for name in ["a", "reviewer", "reviewer-2", "2"] {
        assert_eq!(AgentName::parse(name).unwrap().as_str(), name);
    }
    for name in [
        "",
        "-a",
        "a-",
        "A",
        "a b",
        "../a",
        "a/b",
        "é",
        &"a".repeat(33),
    ] {
        assert!(AgentName::parse(name).is_err(), "{name:?}");
    }
}

proptest! {
    #[test]
    fn reads_anything_without_panicking(json in proptest::collection::vec(any::<u8>(), 0..200)) {
        drop(AgentSpec::parse(&json));
    }

    #[test]
    fn a_name_is_never_a_path_nor_an_option(name in ".{0,40}") {
        if let Ok(name) = AgentName::parse(&name) {
            prop_assert!(!name.as_str().starts_with('-'));
            prop_assert!(!name.as_str().contains(['/', '.', '\0']));
        }
    }
}
