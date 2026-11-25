use super::*;

#[test]
fn an_entry_never_stored_means_nothing_remembered() {
    let store = KeyringStore {
        service: SERVICE,
        account: "dot-tests-never-stored",
    };

    assert_eq!(store.load().unwrap(), None);
}

#[test]
fn forgetting_an_entry_never_stored_is_no_error() {
    let store = KeyringStore {
        service: SERVICE,
        account: "dot-tests-never-stored",
    };

    assert!(store.forget().is_ok());
}

#[test]
fn without_a_keyring_nothing_is_remembered() {
    let unknown = fdo::Error::ServiceUnknown("org.freedesktop.secrets".to_owned());

    for absence in [
        Error::Unavailable,
        Error::Zbus(zbus::Error::Address("none".to_owned())),
        Error::Zbus(zbus::Error::FDO(Box::new(unknown))),
    ] {
        assert_eq!(nothing_without_a_keyring::<()>(absence).unwrap(), []);
    }
}

#[test]
fn a_keyring_that_refuses_is_an_error() {
    for refusal in [Error::Locked, Error::Prompt] {
        let failure = nothing_without_a_keyring::<()>(refusal)
            .unwrap_err()
            .failure();

        assert_eq!(failure, ConnectFailure::StorageFailed);
    }
}

#[test]
fn an_entry_is_found_by_its_service_and_its_account() {
    let attributes = KeyringStore::default().attributes();

    assert_eq!(attributes[SERVICE_ATTRIBUTE], "dev.dot.desktop");
    assert_eq!(attributes[ACCOUNT_ATTRIBUTE], "claude-connection");
}
