use dot_api::{Detail, ReplyEvent, ToolEnd};

use super::*;
use crate::canvas::Scene;
use crate::conversation::flow::running::tests::with_agents;
use crate::conversation::flow::tests::{IdTranslator, after, layout, scene_at, text_of};

const LABEL: &str = "instrument-agents";

fn texts(scene: &Scene) -> Vec<String> {
    scene
        .panes
        .iter()
        .flat_map(|pane| &pane.lines)
        .map(text_of)
        .collect()
}

fn end(conversation: &mut Conversation, id: &str, now: Instant) {
    let ended = ReplyEvent::ToolEnded {
        id: id.to_owned(),
        end: ToolEnd::Done(Detail::Nothing),
    };
    conversation.hear(ended, now, &IdTranslator);
}

#[test]
fn counts_the_agents_at_work_beside_the_reading_line() {
    let now = Instant::now();
    let mut conversation = with_agents(now);

    let working = texts(&scene_at(&mut conversation, after(now, 200)));
    end(&mut conversation, "toolu_1", after(now, 300));
    let one_ended = texts(&scene_at(&mut conversation, after(now, 400)));

    assert!(working.iter().any(|text| text == LABEL), "{working:?}");
    assert!(working.iter().any(|text| text == "2/2"));
    assert!(one_ended.iter().any(|text| text == "1/2"));
}

#[test]
fn keeps_the_row_a_second_and_a_fifth_after_the_last_agent_ended() {
    let now = Instant::now();
    let mut conversation = with_agents(now);
    end(&mut conversation, "toolu_1", after(now, 100));
    end(&mut conversation, "toolu_2", after(now, 200));

    let cooling = scene_at(&mut conversation, after(now, 1_000));
    let gone = scene_at(&mut conversation, after(now, 1_400));

    assert!(texts(&cooling).iter().any(|text| text == "0/2"));
    assert!(conversation.crew_is_cooling(after(now, 1_000)));
    assert!(cooling.animating);
    assert!(!texts(&gone).iter().any(|text| text == LABEL));
    assert!(!conversation.crew_is_cooling(after(now, 1_400)));
}

#[test]
fn shows_the_row_in_the_view_of_an_agent_too() {
    let now = Instant::now();
    let mut conversation = with_agents(now);
    conversation.open_agent("toolu_1", now);

    let scene = scene_at(&mut conversation, after(now, 1_000));

    assert!(
        texts(&scene)
            .iter()
            .any(|text| text.contains("agent-started-by-claude"))
    );
    assert!(texts(&scene).iter().any(|text| text == LABEL));
}

#[test]
fn wakes_for_the_pulse_only_in_front_while_agents_work() {
    let now = Instant::now();
    let mut conversation = with_agents(now);

    conversation.prepare(&layout(), after(now, 100), false);
    let behind = conversation.pulse.next_change(after(now, 100));
    conversation.prepare(&layout(), after(now, 200), true);
    let in_front = conversation.next_change(after(now, 200));
    conversation.end_reply(None, None, after(now, 300));
    conversation.prepare(&layout(), after(now, 300), true);
    let ended = conversation.pulse.next_change(after(now, 300));

    assert_eq!(behind, None);
    assert!(
        in_front.is_some_and(|wake| wake <= after(now, 250)),
        "{in_front:?}"
    );
    assert_eq!(ended, None);
}

#[test]
fn rasterizes_the_silhouette_once_per_scale() {
    let now = Instant::now();
    let mut conversation = with_agents(now);
    let layout = layout();

    let unprepared = matches!(conversation.silhouette_for(&layout), Cow::Owned(_));
    conversation.prepare(&layout, now, true);
    let kept = conversation.silhouette.clone();
    conversation.prepare(&layout, after(now, 16), true);

    assert!(unprepared);
    assert!(kept.is_at(layout.scale()));
    assert_eq!(conversation.silhouette, kept);
    assert!(matches!(
        conversation.silhouette_for(&layout),
        Cow::Borrowed(_)
    ));
    assert_eq!(
        *conversation.silhouette_for(&layout),
        Silhouette::at(layout.scale())
    );
}

#[test]
fn the_view_of_an_agent_keeps_painting_while_the_row_cools() {
    let now = Instant::now();
    let mut conversation = with_agents(now);
    conversation.open_agent("toolu_1", now);
    end(&mut conversation, "toolu_2", after(now, 2_000));

    let cooling = scene_at(&mut conversation, after(now, 2_500));
    let cooled = scene_at(&mut conversation, after(now, 3_300));

    assert!(cooling.animating);
    assert!(!cooled.animating);
    assert_ne!(cooling.veils.len(), 0);
}
