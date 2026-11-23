use super::*;

#[test]
fn keeps_the_first_five_lines_found_in_a_file_and_counts_the_others() {
    let found = "src/a.rs\n1:a\n2-b\n3:c\n4:d\n5:e\n6:f\n7:g\n8:h\n\nsrc/b.rs\n1:x\n2:y\n";

    assert_eq!(
        matches_of_a_file(found).as_deref(),
        Some("src/a.rs\n1:a\n2-b\n3:c\n4:d\n5:e\n[+3 lines of this file]\n\nsrc/b.rs\n1:x\n2:y\n")
    );
}

#[test]
fn leaves_alone_seven_lines_of_a_file_and_lines_no_file_heads() {
    let seven = "src/a.rs\n1:a\n2:b\n3:c\n4:d\n5:e\n6:f\n7:g\n";
    let rows = "1:a\n2:b\n3:c\n4:d\n5:e\n6:f\n7:g\n8:h\n";

    assert_eq!(matches_of_a_file(seven), None);
    assert_eq!(matches_of_a_file(rows), None);
    assert_eq!(matches_of_a_file(&format!("some text\n{rows}")), None);
    assert_eq!(matches_of_a_file(&format!("\n{rows}")), None);
    assert!(matches_of_a_file(&format!("Makefile\n{rows}")).is_some());
    assert_eq!(matches_of_a_file(""), None);
}

#[test]
fn a_line_found_starts_with_its_number() {
    assert!(is_row("12:    let x"));
    assert!(is_row("12-    context"));
    assert!(!is_row(":no number"));
    assert!(!is_row("x12:text"));
    assert!(!is_row("12"));
    assert!(!is_row("src/a.rs"));
}

#[test]
fn keeps_four_names_of_a_long_folder_and_counts_the_others() {
    let listed = "src/{a.rs,b.rs,c.rs,d.rs,e.rs,f.rs,g.rs}\nsrc/x/{a,b}\nplain\n";

    assert_eq!(
        names_of_a_folder(listed).as_deref(),
        Some("src/{a.rs,b.rs,c.rs,d.rs,+3 more}\nsrc/x/{a,b}\nplain\n")
    );
}

#[test]
fn leaves_alone_six_names_and_what_is_not_a_folded_folder() {
    assert_eq!(names_of_a_folder("src/{a,b,c,d,e,f}\n"), None);
    assert_eq!(names_of_a_folder("see {a,b,c,d,e,f,g}\n"), None);
    assert_eq!(names_of_a_folder("src/{a,b,c,d,e,f,g\n"), None);
    assert_eq!(names_of_a_folder("src/a,b,c,d,e,f,g}\n"), None);
    assert_eq!(names_of_a_folder(""), None);
    assert_eq!(
        names_of_a_folder("{a,b,c,d,e,f,g}").as_deref(),
        Some("{a,b,c,d,+3 more}")
    );
}

#[test]
fn reads_once_the_matches_no_file_heads_however_many() {
    let rows: String = (1..=200_000)
        .map(|number| format!("{number}:let reply = chat.reply(prompt);\n"))
        .collect::<Vec<String>>()
        .concat();

    assert_eq!(matches_of_a_file(&rows), None);
}

#[test]
fn keeps_the_name_and_the_size_of_what_a_long_listing_shows() {
    let listing = "total 32\n[ls -l as: mode size date name; all owned by jules:staff]\nd755@ 320 Nov 20 11:38 .\nd755@ 256 Nov 19 19:45 ..\nd755@ 128 Nov 19 18:43 command\n644@ 5278 Nov 20 13:20 compactor.rs\nl755 12 Nov 20 2025 current -> release 2\n755 1.3K Jan 3 09:00 run me.sh\n";

    assert_eq!(
        entries_of_a_listing(listing).as_deref(),
        Some(
            "total 32\n[ls -l as: name size; all owned by jules:staff]\ncommand/\ncompactor.rs 5278\ncurrent -> release 2 12\nrun me.sh 1.3K\n"
        )
    );
}

#[test]
fn leaves_alone_what_is_not_an_entry_and_a_listing_whose_owners_vary() {
    let around = "644 12 Nov 20 11:38 before the listing\n[ls -l as: mode size date name; all owned by a:b]\n\ncrates/x:\n644 12 Nov 20 11:38 a.rs\nmode size Nov 20 11:38 not a mode\n644 size Nov 20 11:38 not a size\n644 12 20 Nov 11:38 not a month\n644 12 Nov 20 11:38\n";
    let owners_vary = "[ls -l as: mode owner:group size date name]\n644 a:b 12 Nov 20 11:38 a.rs\n";

    assert_eq!(
        entries_of_a_listing(around).as_deref(),
        Some(
            "644 12 Nov 20 11:38 before the listing\n[ls -l as: name size; all owned by a:b]\n\ncrates/x:\na.rs 12\nmode size Nov 20 11:38 not a mode\n644 size Nov 20 11:38 not a size\n644 12 20 Nov 11:38 not a month\n644 12 Nov 20 11:38\n"
        )
    );
    assert_eq!(entries_of_a_listing(owners_vary), None);
    assert_eq!(entries_of_a_listing("a.rs\nb.rs\n"), None);
}
