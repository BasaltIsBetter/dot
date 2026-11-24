use std::fs;
use std::os::unix::fs::PermissionsExt;

use super::*;

const TOKEN: &str = "sk-ant-oat01-0123456789abcdef";

struct FakeAnt {
    folder: PathBuf,
}

impl FakeAnt {
    fn printing(name: &str, script: &str) -> Self {
        let folder = std::env::temp_dir().join(format!("dot-ant-{}-{name}", std::process::id()));
        fs::create_dir_all(&folder).unwrap();
        let program = folder.join(PROGRAM);
        fs::write(&program, format!("#!/bin/sh\n{script}\n")).unwrap();
        fs::set_permissions(&program, fs::Permissions::from_mode(0o755)).unwrap();
        Self { folder }
    }

    fn cli(&self) -> AntCli {
        AntCli::new(self.folder.join(PROGRAM))
    }
}

impl Drop for FakeAnt {
    fn drop(&mut self) {
        if let Err(error) = fs::remove_dir_all(&self.folder) {
            eprintln!("could not remove {}: {error}", self.folder.display());
        }
    }
}

#[test]
fn finds_an_installed_ant_on_disk() {
    let ant = FakeAnt::printing("locate", "exit 0");

    let found = locate_ant(None, Some(ant.folder.as_os_str()));

    assert_eq!(found, Some(ant.folder.join(PROGRAM)));
}

#[test]
fn finds_ant_where_go_installs_it() {
    let found = locate(
        PROGRAM,
        &HOME_DIRECTORIES,
        Some(Path::new("/Users/someone")),
        None,
        |candidate| candidate == Path::new("/Users/someone/go/bin/ant"),
    );

    assert_eq!(found, Some(PathBuf::from("/Users/someone/go/bin/ant")));
}

#[test]
fn reads_the_access_token_ant_prints() {
    let ant = FakeAnt::printing(
        "token",
        &format!("[ \"$*\" = 'auth print-credentials --access-token' ] && echo {TOKEN}"),
    );

    let token = ant.cli().access_token(&Cancellation::new()).unwrap();

    assert_eq!(token.expose(), TOKEN);
}

#[test]
fn reports_an_account_ant_cannot_hand_a_token_for() {
    let ant = FakeAnt::printing("signed-out", "echo 'not logged in' >&2; exit 1");

    let error = ant.cli().access_token(&Cancellation::new()).unwrap_err();

    assert_eq!(error.failure(), ConnectFailure::AccountFailed);
}

#[test]
fn reports_something_that_is_not_a_token() {
    let ant = FakeAnt::printing("garbage", "echo 'Apache Ant(TM) version 1.10'");

    let error = ant.cli().access_token(&Cancellation::new()).unwrap_err();

    assert_eq!(error.failure(), ConnectFailure::AccountFailed);
}

#[test]
fn signs_in_through_ant_auth_login() {
    let ant = FakeAnt::printing("login", "[ \"$*\" = 'auth login' ]");

    assert!(ant.cli().sign_in(&Cancellation::new()).is_ok());
}

#[test]
fn reports_a_sign_in_that_fails_or_is_cancelled() {
    let failing = FakeAnt::printing("login-fails", "exit 1");
    let waiting = FakeAnt::printing("login-waits", "sleep 30");
    let cancellation = Cancellation::new();
    cancellation.cancel();

    let failed = failing.cli().sign_in(&Cancellation::new()).unwrap_err();
    let cancelled = waiting.cli().sign_in(&cancellation).unwrap_err();

    assert_eq!(failed.failure(), ConnectFailure::AccountFailed);
    assert_eq!(cancelled.failure(), ConnectFailure::AccountFailed);
}
