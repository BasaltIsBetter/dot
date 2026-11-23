use super::*;

fn compacted(output: &str) -> Option<String> {
    compact(&CommandLine::read("ls -la"), output)
}

#[test]
fn writes_a_long_listing_without_what_every_line_repeats() {
    let listing = "total 16\ndrwxr-xr-x@  5 jules  staff   160 Nov 19 18:18 .\ndrwx------   3 jules  staff    96 Nov 19 18:18 ..\n-rw-r--r--   1 jules  staff   716 Nov 19 18:18 hook.py\n-rwxr-xr-x   1 jules  staff  9300 Jan  3  2025 my script.sh\nlrwxr-xr-x   1 jules  staff    11 Nov 19 18:18 link -> hook.py\n";

    assert_eq!(
        compacted(listing).as_deref(),
        Some(
            "total 16\n[ls -l as: mode size date name; all owned by jules:staff]\nd755@ 160 Nov 19 18:18 .\nd700 96 Nov 19 18:18 ..\n644 716 Nov 19 18:18 hook.py\n755 9300 Jan 3 2025 my script.sh\nl755 11 Nov 19 18:18 link -> hook.py\n"
        )
    );
}

#[test]
fn names_the_owners_on_each_line_when_they_differ() {
    let listing = "-rw-r--r--  1 jules  staff  1 Nov 19 18:18 a\n-rw-r--r--  1 root  wheel  2 Nov 19 18:18 b\n-rw-r--r--  1 jules  staff  3 Nov 19 18:18 c\n-rw-r--r--  1 jules  staff  4 Nov 19 18:18 d\n";

    assert_eq!(
        compacted(listing).as_deref(),
        Some(
            "[ls -l as: mode owner:group size date name]\n644 jules:staff 1 Nov 19 18:18 a\n644 root:wheel 2 Nov 19 18:18 b\n644 jules:staff 3 Nov 19 18:18 c\n644 jules:staff 4 Nov 19 18:18 d\n"
        )
    );
}

#[test]
fn reads_the_special_permission_bits() {
    assert_eq!(mode_of("rwsr-xr-x"), Some(0o4755));
    assert_eq!(mode_of("rw-r-Sr--"), Some(0o2644));
    assert_eq!(mode_of("rwxrwxrwt"), Some(0o1777));
    assert_eq!(mode_of("rwxrwxrwT"), Some(0o1776));
    assert_eq!(mode_of("---------"), Some(0));
    assert_eq!(mode_of("rw-r--r-?"), None);
}

#[test]
fn keeps_human_sizes_and_lines_that_are_not_entries() {
    let listing = "src:\n-rw-r--r--  1 me  staff  9.3K Nov 19 18:18 a\n-rw-r--r--  1 me  staff   45B Nov 19 18:18 b\n-rw-r--r--  1 me  staff  1.2M Nov 19 18:18 c\ncrw-rw-rw-  1 me  staff  3, 2 Nov 19 18:18 null\n-rw-r--r--  1 me  staff    0B Nov 19 18:18 d\n";

    assert_eq!(
        compacted(listing).as_deref(),
        Some(
            "src:\n[ls -l as: mode size date name; all owned by me:staff]\n644 9.3K Nov 19 18:18 a\n644 45B Nov 19 18:18 b\n644 1.2M Nov 19 18:18 c\ncrw-rw-rw-  1 me  staff  3, 2 Nov 19 18:18 null\n644 0B Nov 19 18:18 d\n"
        )
    );
}

#[test]
fn leaves_alone_short_listings_other_commands_and_other_text() {
    let three = "-rw-r--r--  1 me  staff  1 Nov 19 18:18 a\n-rw-r--r--  1 me  staff  2 Nov 19 18:18 b\n-rw-r--r--  1 me  staff  3 Nov 19 18:18 c\n";
    let four = format!("{three}-rw-r--r--  1 me  staff  4 Nov 19 18:18 d\n");

    assert_eq!(compacted(three), None);
    assert!(compacted(&four).is_some());
    assert_eq!(compact(&CommandLine::read("stat -l *"), &four), None);
    assert_eq!(compacted("a.rs\nb.rs\nc.rs\nd.rs\ne.rs\n"), None);
    assert_eq!(compacted(""), None);
}

#[test]
fn a_size_is_a_number_with_one_unit_at_most() {
    for size in ["0", "716", "9.3K", "45B", "1.2M"] {
        assert!(is_size(size), "{size}");
    }
    for size in ["3,", "", "K", "1.2MB", "x12", "1/2"] {
        assert!(!is_size(size), "{size}");
    }
}

#[test]
fn does_not_take_a_sentence_for_an_entry() {
    assert!(entry("-rw-r--r-- is how a file that everyone reads is written down here").is_none());
    assert!(entry("drwxr-xr-x  5 jules  staff  160 Nov 19 18:18").is_none());
    assert!(entry("xrwxr-xr-x  5 jules  staff  160 Nov 19 18:18 name").is_none());
    assert!(entry("total 16").is_none());
}

#[test]
fn keeps_the_marks_of_extended_attributes_and_access_lists() {
    let listing = "-rw-r--r--@ 1 me  staff  1 Nov 19 18:18 a\n-rw-r--r--+ 1 me  staff  2 Nov 19 18:18 b\n-rw-r--r--  1 me  staff  3 Nov 19 18:18 c\ndrwxr-xr-x@ 2 me  wheel  4 Nov 19 18:18 d\n";

    assert_eq!(
        compacted(listing).as_deref(),
        Some(
            "[ls -l as: mode owner:group size date name]\n644@ me:staff 1 Nov 19 18:18 a\n644+ me:staff 2 Nov 19 18:18 b\n644 me:staff 3 Nov 19 18:18 c\nd755@ me:wheel 4 Nov 19 18:18 d\n"
        )
    );
}
