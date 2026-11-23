use proptest::prelude::*;

use super::*;

const TABLE: &str = " id |  name   |       email\n----+---------+-------------------\n  1 | Alice   | alice@example.com\n  2 | Bob     | bob@example.com\n(2 rows)\n\n";

fn query(command: &str, text: &str) -> Option<String> {
    compact(&CommandLine::read(command), text)
}

#[test]
fn writes_a_table_without_its_padding() {
    assert_eq!(
        query("psql -c 'select * from users'", TABLE).as_deref(),
        Some("id|name|email\n1|Alice|alice@example.com\n2|Bob|bob@example.com\n(2 rows)\n\n")
    );
}

#[test]
fn writes_every_table_of_the_output() {
    let two = format!("SET\n{TABLE} n | m\n---+---\n 1 | 2\n(1 row)");

    assert_eq!(
        query("docker exec db psql -f report.sql", &two).as_deref(),
        Some(
            "SET\nid|name|email\n1|Alice|alice@example.com\n2|Bob|bob@example.com\n(2 rows)\n\nn|m\n1|2\n(1 row)"
        )
    );
}

#[test]
fn writes_a_table_that_has_no_row() {
    assert_eq!(
        query("psql", " id | name\n----+------\n(0 rows)\n").as_deref(),
        Some("id|name\n(0 rows)\n")
    );
}

#[test]
fn keeps_a_table_it_cannot_read_for_sure() {
    let one_column = " count\n-------\n    42\n(1 row)\n";
    let shifted = TABLE.replace("  2 | Bob     ", " 2 | Bob     ");
    let cut_short = TABLE.replace("  2 | Bob     | bob@example.com", "  2 | Bob");
    let bar_in_a_cell = TABLE.replace(" Bob     ", " B|b     ");
    let expanded = "-[ RECORD 1 ]-+------\nid            | 1\nname          | Alice\n";

    for table in [
        one_column,
        &shifted,
        &cut_short,
        &bar_in_a_cell,
        expanded,
        "",
    ] {
        assert_eq!(query("psql", table), None, "{table}");
    }
}

#[test]
fn only_reads_what_psql_prints() {
    assert_eq!(query("mysql -e 'select 1'", TABLE), None);
    assert_eq!(query("cat table.txt", TABLE), None);
}

#[test]
fn a_rule_says_where_the_cells_break() {
    assert_eq!(breaks_of("----+---------+----"), Some(vec![4, 14]));
    assert_eq!(breaks_of("-+-"), Some(vec![1]));
    assert_eq!(breaks_of("-------"), None);
    assert_eq!(breaks_of("+---+---+"), None);
    assert_eq!(breaks_of("----+---- "), None);
    assert_eq!(breaks_of(""), None);
}

fn drawn(rows: &[Vec<String>]) -> String {
    let widths: Vec<usize> = (0..rows[0].len())
        .map(|column| rows.iter().map(|row| row[column].len()).max().unwrap())
        .collect();
    let line = |row: &Vec<String>| {
        row.iter()
            .zip(&widths)
            .map(|(cell, width)| format!(" {cell:<width$} "))
            .collect::<Vec<String>>()
            .join("|")
    };
    let rule = widths
        .iter()
        .map(|width| "-".repeat(width + 2))
        .collect::<Vec<String>>()
        .join("+");

    let mut lines = vec![line(&rows[0]), rule];
    lines.extend(rows[1..].iter().map(line));
    lines.push("(some rows)\n".to_owned());
    lines.join("\n")
}

fn rows() -> impl Strategy<Value = Vec<Vec<String>>> {
    (2_usize..6).prop_flat_map(|columns| {
        prop::collection::vec(prop::collection::vec("[a-z0-9é]{0,8}", columns), 1..8)
    })
}

proptest! {
    #[test]
    fn tells_the_same_cells(rows in rows()) {
        let shorter = query("psql", &drawn(&rows)).unwrap();

        let told: Vec<&str> = shorter.lines().collect();
        let mut expected: Vec<String> = rows.iter().map(|row| row.join("|")).collect();
        expected.push("(some rows)".to_owned());
        prop_assert_eq!(told, expected);
    }

    #[test]
    fn any_text_is_read_without_panicking(text in "[-+| a-zé\n]{0,200}") {
        if let Some(shorter) = query("psql", &text) {
            prop_assert!(shorter.len() <= text.len());
        }
    }
}
