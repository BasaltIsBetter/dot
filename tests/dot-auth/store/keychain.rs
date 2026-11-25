use super::*;

#[test]
fn an_entry_never_stored_means_nothing_remembered() {
    let store = KeychainStore {
        service: SERVICE,
        account: "dot-tests-never-stored",
    };

    assert_eq!(store.load().unwrap(), None);
}

#[test]
fn forgetting_an_entry_never_stored_is_no_error() {
    let store = KeychainStore {
        service: SERVICE,
        account: "dot-tests-never-stored",
    };

    assert!(store.forget().is_ok());
}
