use super::*;
use crate::cut::{CONTENT, OTHER};

fn numbered(lines: std::ops::RangeInclusive<usize>) -> String {
    lines
        .map(|number| format!("line {number:04} of the output\n"))
        .collect::<Vec<String>>()
        .concat()
}

#[test]
fn cuts_the_middle_of_a_long_output_between_two_lines_and_says_which() {
    let text = numbered(1..=300);

    assert_eq!(
        cut(&text, &OTHER),
        Some(format!(
            "{}[+176 lines cut: 63 to 238 of this output]\n{}",
            numbered(1..=62),
            numbered(239..=300)
        ))
    );
}

#[test]
fn leaves_alone_an_output_of_seven_thousand_bytes_and_cuts_one_byte_more() {
    let exactly = format!("{}123456789012345\n", numbered(1..=291));
    let one_more = format!("{}1234567890123456\n", numbered(1..=291));

    assert_eq!(exactly.len(), 7_000);
    assert_eq!(cut(&exactly, &OTHER), None);
    assert!(cut(&one_more, &OTHER).is_some());
    assert_eq!(cut("", &OTHER), None);
}

#[test]
fn keeps_two_thirds_of_a_file_from_its_start_and_only_cuts_above_ten_thousand_bytes() {
    let under = numbered(1..=416);
    let over = numbered(1..=500);

    assert_eq!(under.len(), 9_984);
    assert_eq!(cut(&under, &CONTENT), None);
    assert_eq!(
        cut(&over, &CONTENT),
        Some(format!(
            "{}[+251 lines cut: 167 to 417 of this output]\n{}",
            numbered(1..=166),
            numbered(418..=500)
        ))
    );
}

#[test]
fn starts_the_tail_exactly_on_a_line_when_one_starts_there() {
    let text = format!("{}{}", numbered(1..=250), "tail\n".repeat(300));
    let shorter = cut(&text, &OTHER).unwrap();

    assert_eq!(text.len(), 7_500);
    assert!(shorter.ends_with(&format!(
        "[+188 lines cut: 63 to 250 of this output]\n{}",
        "tail\n".repeat(300)
    )));
}

#[test]
fn keeps_no_tail_when_the_last_line_alone_is_longer() {
    let text = format!("{}{}", numbered(1..=230), "z".repeat(2_000));

    assert_eq!(
        cut(&text, &OTHER),
        Some(format!(
            "{}[+169 lines cut: 63 to 231 of this output]\n",
            numbered(1..=62)
        ))
    );
}

#[test]
fn cuts_nothing_when_the_start_holds_no_whole_line() {
    let text = format!("{}\nend\n", "x".repeat(6_000));

    assert_eq!(cut(&text, &OTHER), None);
}

#[test]
fn lists_the_blocks_opened_in_what_it_cut_from_a_file() {
    let methods = (1..=250)
        .map(|method| {
            format!("    fn method_{method:03}(&self) {{\n        self.run();\n    }}\n\n")
        })
        .collect::<Vec<String>>()
        .concat();
    let file = format!("impl Service {{\n{methods}}}\n");

    let shorter = cut(&file, &CONTENT).unwrap();
    let noted: Vec<&str> = shorter
        .lines()
        .skip_while(|line| !line.starts_with("[+"))
        .take_while(|line| !line.starts_with("[the output goes on"))
        .collect();

    assert_eq!(file.len(), 13_517);
    assert_eq!(
        noted[0],
        "[+559 lines cut: 295 to 853 of this output; among them, these open a block]"
    );
    assert_eq!(noted[1], "298:    fn method_075(&self) {");
    assert_eq!(noted[40], "454:    fn method_114(&self) {");
    assert_eq!(noted.len(), 1 + 40);
    assert!(shorter.contains("\n[the output goes on at line 854]\n    fn method_214(&self) {\n"));
    assert!(shorter.len() < file.len() - 6_000);
}

#[test]
fn gives_the_outline_a_quarter_of_what_it_cuts_at_most() {
    let limits = Limits {
        cut_above: 100,
        kept: 60,
        kept_from_the_start: 30,
        ..CONTENT
    };
    let blocks: String = (1..=20)
        .map(|number| format!("fn block_{number:02}() {{\n    body();\n}}\n"))
        .collect::<Vec<String>>()
        .concat();
    let text = format!(
        "{}{blocks}{}",
        "start of it\n".repeat(3),
        "end of it\n".repeat(4)
    );

    let shorter = cut(&text, &limits).unwrap();
    let outline: Vec<&str> = shorter
        .lines()
        .skip_while(|line| !line.starts_with("[+"))
        .skip(1)
        .take_while(|line| !line.starts_with("[the output goes on"))
        .collect();
    let cut_bytes = text.len() - "start of it\n".len() * 2 - "end of it\n".len() * 3;
    let outline_bytes: usize = outline.iter().map(|line| line.len()).sum();

    assert_eq!(cut_bytes, 622);
    assert_eq!(outline.first(), Some(&"4:fn block_01() {"));
    assert_eq!(outline.len(), 8);
    assert_eq!(outline_bytes, 142);
    assert!(outline_bytes <= cut_bytes / 4);
    assert!(outline_bytes + "25:fn block_08() {".len() > cut_bytes / 4);
}
