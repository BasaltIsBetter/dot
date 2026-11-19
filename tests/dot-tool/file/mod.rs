use super::*;

#[test]
fn a_nul_byte_makes_a_file_binary() {
    assert!(is_binary(b"abc\0def"));
    assert!(!is_binary(b"abc def\n"));
    assert!(!is_binary(b""));
}

#[test]
fn sniffs_the_first_eight_kibibytes_of_a_file() {
    assert_eq!(SNIFFED, 8_192);
}

#[test]
fn recognizes_windows_line_endings_by_the_first_line() {
    assert!(uses_crlf(b"a\r\nb\n"));
    assert!(!uses_crlf(b"a\nb\r\n"));
    assert!(!uses_crlf(b"\nb"));
    assert!(!uses_crlf(b"no break"));
    assert!(!uses_crlf(b""));
}
