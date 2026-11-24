use super::*;

fn lines(texts: &[&str]) -> Vec<String> {
    texts.iter().map(|&text| text.to_owned()).collect()
}

#[test]
fn rates_two_lines_by_what_they_share_at_both_ends_past_their_indent() {
    assert_eq!(likeness("let x = 1;", "let x = 1;"), 1_000);
    assert_eq!(likeness("    x();", "x();"), 1_000);
    assert_eq!(likeness("", "   "), 1_000);
    assert_eq!(
        likeness("self.grid = grid;", "self.grid = Grid::of(size);"),
        481
    );
    assert_eq!(likeness("abcd", "abXd"), 750);
    assert_eq!(likeness("aaa", "aa"), 666);
    assert_eq!(likeness("x", "y"), 0);
}

#[test]
fn pairs_an_old_line_with_the_new_line_most_like_it() {
    let old = lines(&["  shown = shown === null ? target : shown + step;"]);
    let new = lines(&[
        "  let reduced = motion();",
        "  shown = shown === null || reduced ? target : shown + step;",
    ]);

    assert_eq!(pairs(&old, &new), [(0, 1)]);
}

#[test]
fn keeps_the_pairs_in_order_and_takes_as_much_likeness_as_it_can() {
    let old = lines(&[
        "self.strip.compose(&self.grid);",
        "self.grid = Grid::of(size, ZOOM);",
    ]);
    let new = lines(&[
        "let grid = Grid::of(size, ZOOM);",
        "self.strip.compose(&grid);",
        "self.grid = grid;",
    ]);

    assert_eq!(pairs(&old, &new), [(0, 1), (1, 2)]);
}

#[test]
fn pairs_lines_alike_in_nothing_in_order_between_the_alike_ones() {
    let old = lines(&["alpha", "beta();", "gamma"]);
    let new = lines(&["one", "two", "beta(x);", "three"]);

    assert_eq!(pairs(&old, &new), [(0, 0), (1, 2), (2, 3)]);
    assert_eq!(pairs(&lines(&["a", "b"]), &lines(&["c"])), [(0, 0)]);
    assert_eq!(pairs(&[], &lines(&["c"])), Vec::<(usize, usize)>::new());
}

#[test]
fn pairs_in_order_without_comparing_runs_too_long_to_compare() {
    let shared = "let shared = across(the, runs);";
    let mut new = vec!["z".to_owned(); 100];
    new.push(shared.to_owned());
    let old_of = |count: usize| {
        let mut old = vec![shared.to_owned()];
        old.extend(vec!["q".to_owned(); count - 1]);
        old
    };

    let compared = pairs(&old_of(99), &new);
    let at_the_most = pairs(&old_of(100), &new[1..]);
    let too_long = pairs(&old_of(101), &new);

    assert_eq!(compared, [(0, 100)]);
    assert_eq!(at_the_most, [(0, 99)]);
    assert_eq!(too_long.len(), 101);
    assert_eq!(too_long[0], (0, 0));
}
