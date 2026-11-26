use super::*;

#[test]
fn keeps_its_files_where_mac_apps_do() {
    let places = places(Path::new("/Users/me"));

    assert_eq!(
        places.workspace,
        Path::new("/Users/me/Library/Application Support/Dot")
    );
    assert_eq!(places.logs, Path::new("/Users/me/Library/Logs/Dot"));
}
