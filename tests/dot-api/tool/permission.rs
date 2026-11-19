use proptest::prelude::*;

use super::*;

fn owned(text: &str) -> String {
    text.to_owned()
}

#[test]
fn a_command_is_granted_on_its_own() {
    let action = Action::Run {
        command: owned("cargo test"),
    };

    assert_eq!(action.grant(), Grant::Command(owned("cargo test")));
    assert_eq!(action.text(), Text::PermissionRun);
    assert_eq!(action.subject(), "cargo test");
    assert_eq!(action.grant().text(), Text::GrantCommand);
}

#[test]
fn a_change_inside_the_project_grants_every_change_inside_it() {
    let one = Action::Change {
        path: owned("src/a.rs"),
    };
    let other = Action::Change {
        path: owned("src/b.rs"),
    };

    assert_eq!(one.grant(), Grant::Changes);
    assert_eq!(one.grant(), other.grant());
    assert_eq!(one.text(), Text::PermissionChange);
    assert_eq!(one.grant().text(), Text::GrantChanges);
}

#[test]
fn reaching_outside_the_project_is_granted_path_by_path() {
    let change = Action::ChangeOutside {
        path: owned("/etc/hosts"),
    };
    let read = Action::ReadOutside {
        path: owned("/etc/hosts"),
    };

    assert_eq!(change.grant(), Grant::ChangeOutside(owned("/etc/hosts")));
    assert_eq!(read.grant(), Grant::ReadOutside(owned("/etc/hosts")));
    assert_ne!(change.grant(), read.grant());
    assert_eq!(change.text(), Text::PermissionChangeOutside);
    assert_eq!(read.text(), Text::PermissionReadOutside);
    assert_eq!(change.subject(), "/etc/hosts");
    assert_eq!(read.subject(), "/etc/hosts");
    assert_eq!(change.grant().text(), Text::GrantFile);
    assert_eq!(read.grant().text(), Text::GrantFile);
}

#[test]
fn a_page_grants_its_whole_host() {
    let action = Action::Fetch {
        url: owned("https://Docs.RS:443/ureq/latest?x=1#top"),
    };

    assert_eq!(action.grant(), Grant::Host(owned("docs.rs")));
    assert_eq!(action.text(), Text::PermissionFetch);
    assert_eq!(action.subject(), "https://Docs.RS:443/ureq/latest?x=1#top");
    assert_eq!(action.grant().text(), Text::GrantHost);
}

#[test]
fn finds_the_host_behind_user_names_brackets_and_missing_schemes() {
    let hosts = [
        ("https://user:secret@example.com/page", "example.com"),
        ("http://[::1]:8080/", "[::1]"),
        ("example.org/path", "example.org"),
        ("https://example.net?query", "example.net"),
        ("https://example.io#part", "example.io"),
        ("", ""),
    ];

    for (url, host) in hosts {
        assert_eq!(
            Action::Fetch { url: owned(url) }.grant(),
            Grant::Host(owned(host)),
            "{url}"
        );
    }
}

#[test]
fn every_web_search_shares_one_grant() {
    let action = Action::SearchWeb {
        query: owned("rust ureq"),
    };

    assert_eq!(action.grant(), Grant::SearchWeb);
    assert_eq!(action.text(), Text::PermissionSearchWeb);
    assert_eq!(action.subject(), "rust ureq");
    assert_eq!(action.grant().text(), Text::GrantSearchWeb);
}

#[test]
fn another_tool_is_granted_by_its_name() {
    let action = Action::Use {
        tool: owned("NotebookEdit"),
        detail: owned("a.ipynb"),
    };

    assert_eq!(action.grant(), Grant::Tool(owned("NotebookEdit")));
    assert_eq!(action.text(), Text::PermissionUse);
    assert_eq!(action.subject(), "NotebookEdit");
    assert_eq!(action.grant().text(), Text::GrantTool);
}

proptest! {
    #[test]
    fn a_host_never_keeps_a_path_a_port_or_capitals(url in "\\PC{0,80}") {
        let Grant::Host(host) = (Action::Fetch { url }).grant() else {
            unreachable!("a page always grants a host");
        };

        prop_assert!(!host.contains(['/', '?', '#']));
        prop_assert_eq!(host.to_ascii_lowercase(), host);
    }
}
