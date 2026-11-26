use std::time::Duration;

use super::*;

fn after(start: Instant, milliseconds: u64) -> Instant {
    start + Duration::from_millis(milliseconds)
}

fn start(agents: &mut Agents, id: &str, kind: &str, now: Instant) {
    agents.start(id.to_owned(), "a task".to_owned(), now);
    agents.name(id, kind.to_owned());
}

fn kinds(agents: &Agents) -> Vec<&str> {
    agents.wave().map(|agent| agent.kind.as_str()).collect()
}

#[test]
fn holds_nothing_without_any_agent() {
    let agents = Agents::default();

    assert_eq!(agents.wave().len(), 0);
    assert_eq!(agents.wave_ended(), None);
}

#[test]
fn gathers_the_agents_started_while_one_of_them_works() {
    let now = Instant::now();
    let mut agents = Agents::default();
    start(&mut agents, "a", "Explore", now);
    start(&mut agents, "b", "tests", after(now, 10));
    agents.end("a", after(now, 20));

    start(&mut agents, "c", "web", after(now, 30));

    assert_eq!(kinds(&agents), ["Explore", "tests", "web"]);
}

#[test]
fn starts_a_new_wave_once_every_agent_ended() {
    let now = Instant::now();
    let mut agents = Agents::default();
    start(&mut agents, "a", "Explore", now);
    start(&mut agents, "b", "tests", now);
    agents.end("a", after(now, 10));
    agents.end("b", after(now, 20));

    start(&mut agents, "c", "web", after(now, 30));
    start(&mut agents, "d", "plan", after(now, 40));

    assert_eq!(kinds(&agents), ["web", "plan"]);
}

#[test]
fn starts_a_new_wave_after_the_reply_ended_its_agents() {
    let now = Instant::now();
    let mut agents = Agents::default();
    start(&mut agents, "a", "Explore", now);
    agents.end_working(after(now, 10));

    start(&mut agents, "b", "tests", after(now, 20));

    assert_eq!(kinds(&agents), ["tests"]);
}

#[test]
fn starts_afresh_once_cleared() {
    let now = Instant::now();
    let mut agents = Agents::default();
    for (id, kind) in [("a", "Explore"), ("b", "tests")] {
        start(&mut agents, id, kind, now);
        agents.end(id, after(now, 10));
    }
    agents.clear();

    start(&mut agents, "c", "web", after(now, 20));

    assert_eq!(kinds(&agents), ["web"]);
}

#[test]
fn ends_with_its_last_agent() {
    let now = Instant::now();
    let mut agents = Agents::default();
    start(&mut agents, "a", "Explore", now);
    start(&mut agents, "b", "tests", now);
    start(&mut agents, "c", "web", now);
    agents.end("b", after(now, 300));
    agents.end("a", after(now, 100));
    let working = agents.wave_ended();

    agents.end("c", after(now, 200));

    assert_eq!(working, None);
    assert_eq!(agents.wave_ended(), Some(after(now, 300)));
}
