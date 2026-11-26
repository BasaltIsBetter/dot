use super::*;

fn among<'a>(set: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<String> + 'a {
    move |name| {
        set.iter()
            .find(|(variable, _)| *variable == name)
            .map(|(_, value)| (*value).to_owned())
    }
}

fn places_with(set: &[(&str, &str)]) -> Places {
    let variable = among(set);
    places_among(Path::new("/home/me"), |name| {
        variable(name).map(OsString::from)
    })
}

#[test]
fn keeps_its_files_where_linux_programs_do() {
    let places = places_with(&[]);

    assert_eq!(places.workspace, Path::new("/home/me/.local/share/dot"));
    assert_eq!(places.logs, Path::new("/home/me/.local/state/dot"));
}

#[test]
fn follows_the_folders_the_user_moved() {
    let places = places_with(&[("XDG_DATA_HOME", "/data"), ("XDG_STATE_HOME", "/state")]);

    assert_eq!(places.workspace, Path::new("/data/dot"));
    assert_eq!(places.logs, Path::new("/state/dot"));
}

#[test]
fn a_relative_folder_counts_as_none() {
    let places = places_with(&[("XDG_DATA_HOME", "data"), ("XDG_STATE_HOME", "")]);

    assert_eq!(places, places_with(&[]));
}

#[test]
fn reads_the_language_of_the_locale() {
    let languages = languages_among(among(&[("LANG", "fr_FR.UTF-8")]));

    assert_eq!(languages, ["fr_FR"]);
}

#[test]
fn the_first_variable_set_decides() {
    let languages = languages_among(among(&[
        ("LC_ALL", ""),
        ("LC_MESSAGES", "en_GB@euro"),
        ("LANG", "fr_FR.UTF-8"),
    ]));

    assert_eq!(languages, ["en_GB"]);
}

#[test]
fn the_listed_languages_come_first() {
    let languages = languages_among(among(&[("LANGUAGE", "de::en"), ("LANG", "fr_FR.UTF-8")]));

    assert_eq!(languages, ["de", "en", "fr_FR"]);
}

#[test]
fn the_c_locale_prefers_no_language() {
    for locale in ["C", "C.UTF-8", "POSIX"] {
        let languages = languages_among(among(&[("LANGUAGE", "en"), ("LANG", locale)]));

        assert!(languages.is_empty(), "{locale}");
    }
}
