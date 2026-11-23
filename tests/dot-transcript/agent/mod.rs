use super::*;

fn after(start: Instant, milliseconds: u64) -> Instant {
    start + Duration::from_millis(milliseconds)
}

fn tool(glyph: Glyph, label: &str) -> Tool {
    Tool {
        glyph,
        label: label.to_owned(),
    }
}

fn one_agent(start: Instant) -> Agents {
    let mut agents = Agents::default();
    agents.start("toolu_1".to_owned(), "find the grid".to_owned(), start);
    agents
}

#[test]
fn follows_an_agent_from_its_start() {
    let start = Instant::now();
    let mut agents = one_agent(start);

    agents.name("toolu_1", "Explore".to_owned());
    agents.said("toolu_1", "Looking.".to_owned());

    let agent = agents.get("toolu_1").unwrap();
    assert_eq!(agent.kind, "Explore");
    assert_eq!(agent.mission, "find the grid");
    assert_eq!(agent.said, ["Looking."]);
    assert_eq!(agent.started, start);
    assert!(agents.get("toolu_2").is_none());
}

#[test]
fn shows_the_tool_that_runs_or_else_the_last_one() {
    let start = Instant::now();
    let mut agents = one_agent(start);
    let before = agents.get("toolu_1").unwrap().current().cloned();
    agents.start_action("toolu_1", "a".to_owned(), tool(Glyph::Read, "layout.rs"));
    agents.start_action("toolu_1", "b".to_owned(), tool(Glyph::Search, "« grid »"));
    agents.end_action("toolu_1", "b", "×3".to_owned());

    let running = agents.get("toolu_1").unwrap().current().unwrap().id.clone();
    agents.end_action("toolu_1", "a", "212 l.".to_owned());
    let last = agents.get("toolu_1").unwrap().current().unwrap().clone();

    assert_eq!(before, None);
    assert_eq!(running, "a");
    assert_eq!(last.id, "b");
    assert_eq!(last.result.as_deref(), Some("×3"));
}

#[test]
fn counts_what_an_agent_writes_once_even_when_its_report_counts_it_again() {
    let start = Instant::now();
    let mut agents = one_agent(start);

    let first = agents.wrote("toolu_1", 17, after(start, 100));
    let rest = agents.report("toolu_1", "Two files.".to_owned(), 243, after(start, 900));
    let over = agents.report("toolu_1", "Two files.".to_owned(), 200, after(start, 950));

    assert_eq!((first, rest, over), (17, 226, 0));
    let agent = agents.get("toolu_1").unwrap();
    assert_eq!(agent.report.as_deref(), Some("Two files."));
    assert_eq!(agent.measures(after(start, 950)).tokens, 243);
}

#[test]
fn counts_nothing_for_an_agent_it_does_not_follow() {
    let mut agents = Agents::default();

    assert_eq!(agents.wrote("toolu_9", 10, Instant::now()), 0);
    assert_eq!(
        agents.report("toolu_9", String::new(), 10, Instant::now()),
        0
    );
}

#[test]
fn lists_the_agents_at_work_until_their_call_ends() {
    let start = Instant::now();
    let mut agents = one_agent(start);
    agents.start(
        "toolu_2".to_owned(),
        "run the tests".to_owned(),
        after(start, 10),
    );

    agents.end("toolu_1", after(start, 7_800));

    let working: Vec<&str> = agents.working().map(|(id, _)| id).collect();
    assert_eq!(working, ["toolu_2"]);
    let ended = agents.get("toolu_1").unwrap();
    assert_eq!(
        ended.worked(after(start, 60_000)),
        Duration::from_millis(7_800)
    );
    assert_eq!(
        agents.get("toolu_2").unwrap().worked(after(start, 1_010)),
        Duration::from_secs(1)
    );
}

#[test]
fn an_agent_ends_once() {
    let start = Instant::now();
    let mut agents = one_agent(start);

    agents.end("toolu_1", after(start, 1_000));
    agents.end("toolu_1", after(start, 2_000));

    assert_eq!(
        agents.get("toolu_1").unwrap().ended,
        Some(after(start, 1_000))
    );
}

#[test]
fn tells_what_an_agent_did_most_the_first_kind_on_a_tie() {
    let start = Instant::now();
    let mut agents = one_agent(start);
    let empty = agents.get("toolu_1").unwrap().mostly();
    for (id, glyph) in [
        ("a", Glyph::Search),
        ("b", Glyph::Read),
        ("c", Glyph::Read),
        ("d", Glyph::Search),
    ] {
        agents.start_action("toolu_1", id.to_owned(), tool(glyph, "x"));
    }
    let tie = agents.get("toolu_1").unwrap().mostly();
    agents.start_action("toolu_1", "e".to_owned(), tool(Glyph::Read, "x"));

    assert_eq!(empty, None);
    assert_eq!(tie, Some((Glyph::Search, 2)));
    assert_eq!(
        agents.get("toolu_1").unwrap().mostly(),
        Some((Glyph::Read, 3))
    );
}

#[test]
fn forgets_every_agent() {
    let mut agents = one_agent(Instant::now());

    agents.clear();

    assert!(agents.get("toolu_1").is_none());
    assert_eq!(agents.working().count(), 0);
}

#[test]
fn ends_the_agents_still_at_work_when_the_reply_ends() {
    let start = Instant::now();
    let mut agents = one_agent(start);
    agents.start("toolu_2".to_owned(), "other".to_owned(), start);
    agents.end("toolu_2", after(start, 500));

    agents.end_working(after(start, 2_000));

    assert_eq!(agents.working().count(), 0);
    assert_eq!(
        agents.get("toolu_1").unwrap().ended,
        Some(after(start, 2_000))
    );
    assert_eq!(
        agents.get("toolu_2").unwrap().ended,
        Some(after(start, 500))
    );
}
