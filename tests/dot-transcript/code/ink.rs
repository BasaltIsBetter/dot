use super::*;

fn pieces(text: &str, language: Language) -> Vec<(&str, Ink)> {
    inked(text, language)
        .into_iter()
        .map(|(range, ink)| (&text[range], ink))
        .collect()
}

#[test]
fn knows_a_language_by_the_extension_of_its_file() {
    assert_eq!(Language::of("src/main.rs"), Language::Rust);
    assert_eq!(Language::of("web/App.TSX"), Language::Script);
    assert_eq!(Language::of("tool.py"), Language::Python);
    assert_eq!(Language::of("main.go"), Language::Go);
    assert_eq!(Language::of("Game.java"), Language::CFamily);
    assert_eq!(Language::of("build.sh"), Language::Shell);
    assert_eq!(Language::of("Cargo.toml"), Language::Config);
    assert_eq!(Language::of("notes.md"), Language::Plain);
    assert_eq!(Language::of("Makefile"), Language::Plain);
    assert_eq!(Language::of("my.dir/Makefile"), Language::Plain);
}

#[test]
fn cuts_words_spaces_and_each_sign_apart() {
    let text = "a_b  1é(::)";

    let cut: Vec<&str> = tokens(text).into_iter().map(|range| &text[range]).collect();

    assert_eq!(cut, ["a_b", "  ", "1é", "(", ":", ":", ")"]);
    assert_eq!(tokens(""), []);
}

#[test]
fn inks_names_keywords_and_punctuation_apart() {
    assert_eq!(
        pieces("pub fn go(x: u8) {", Language::Rust),
        [
            ("pub fn ", Ink::Secondary),
            ("go", Ink::Full),
            ("(", Ink::Receding),
            ("x", Ink::Full),
            (": ", Ink::Receding),
            ("u8", Ink::Full),
            (") {", Ink::Receding),
        ]
    );
    assert_eq!(pieces("  x", Language::Rust), [("  x", Ink::Full)]);
}

#[test]
fn recedes_a_comment_to_the_end_of_its_line() {
    assert_eq!(
        pieces("let a = 1; // one", Language::Rust),
        [
            ("let ", Ink::Secondary),
            ("a ", Ink::Full),
            ("= ", Ink::Receding),
            ("1", Ink::Full),
            ("; // one", Ink::Receding),
        ]
    );
    assert_eq!(
        pieces("# note", Language::Python),
        [("# note", Ink::Receding)]
    );
    assert_eq!(
        pieces("x = 1 # note", Language::Config)[3],
        ("# note", Ink::Receding)
    );
}

#[test]
fn reads_no_comment_inside_a_string() {
    let shown = pieces(r#"let u = "https://a"; // b"#, Language::Rust);

    assert!(shown.contains(&("https", Ink::Full)));
    assert_eq!(shown.last(), Some(&("\"; // b", Ink::Receding)));
}

#[test]
fn reads_an_escaped_quote_as_part_of_its_string() {
    let shown = pieces(r#"let s = "a\"//b"; // c"#, Language::Rust);

    assert!(shown.contains(&("b", Ink::Full)));
    assert_eq!(shown.last(), Some(&("\"; // c", Ink::Receding)));
}

#[test]
fn opens_strings_with_the_quotes_of_the_language() {
    let script = pieces("x = 'http://a' // c", Language::Script);
    let rust = pieces("x = 'a' // c", Language::Rust);

    assert!(script.contains(&("http", Ink::Full)));
    assert_eq!(script.last(), Some(&("' // c", Ink::Receding)));
    assert_eq!(rust.last(), Some(&("' // c", Ink::Receding)));
}

#[test]
fn keeps_keywords_as_names_inside_a_string_and_in_plain_text() {
    assert!(pieces(r#""fn""#, Language::Rust).contains(&("fn", Ink::Full)));
    assert_eq!(
        pieces("fn x // y", Language::Plain),
        [
            ("fn x ", Ink::Full),
            ("// ", Ink::Receding),
            ("y", Ink::Full)
        ]
    );
}

#[test]
fn colours_every_language_by_its_own_words() {
    let keyword = |word: &str, language| pieces(word, language) == [(word, Ink::Secondary)];

    assert!(keyword("function", Language::Script));
    assert!(keyword("def", Language::Python));
    assert!(keyword("func", Language::Go));
    assert!(keyword("void", Language::CFamily));
    assert!(keyword("fi", Language::Shell));
    assert!(keyword("true", Language::Config));
    assert!(!keyword("def", Language::Rust));
}
