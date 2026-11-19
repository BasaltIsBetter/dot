use std::fmt::{self, Write};

pub(crate) const NEW_LINE: u8 = b'\n';
pub(crate) const CARRIAGE_RETURN: u8 = b'\r';
const A_STRING_ALWAYS_TAKES_TEXT: &str = "writing into a String never fails";

pub(crate) fn append(text: &mut String, arguments: fmt::Arguments<'_>) {
    text.write_fmt(arguments).expect(A_STRING_ALWAYS_TAKES_TEXT);
}
