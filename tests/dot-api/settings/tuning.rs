use super::*;

#[test]
fn names_each_model_like_the_claude_api() {
    let ids: Vec<_> = Model::ALL.iter().map(|model| model.id).collect();

    assert_eq!(
        ids,
        [
            "claude-haiku-5-5",
            "claude-sonnet-5-5",
            "claude-opus-5-5",
            "claude-fable-5-2"
        ]
    );
}

#[test]
fn names_each_model_to_the_user() {
    let names: Vec<_> = Model::ALL.iter().map(|model| model.name).collect();

    assert_eq!(names, ["Haiku 5.5", "Sonnet 5.5", "Opus 5.5", "Fable 5.2"]);
}

#[test]
fn every_model_reads_a_million_tokens() {
    for model in Model::ALL {
        assert_eq!(model.context_window, 1_000_000, "{}", model.name);
    }
}

#[test]
fn finds_a_model_by_its_identifier() {
    for model in Model::ALL {
        assert_eq!(Model::with_id(model.id), Some(model));
    }
}

#[test]
fn refuses_an_identifier_dot_does_not_offer() {
    assert_eq!(Model::with_id("claude-opus-4-8"), None);
    assert_eq!(Model::with_id(""), None);
    assert_eq!(Model::with_id("CLAUDE-OPUS-5-5"), None);
}

#[test]
fn names_each_effort_like_the_claude_api() {
    let names = [
        (Effort::Low, "low"),
        (Effort::Medium, "medium"),
        (Effort::High, "high"),
        (Effort::ExtraHigh, "xhigh"),
        (Effort::Max, "max"),
    ];

    for (effort, name) in names {
        assert_eq!(effort.api_name(), name);
        assert_eq!(Effort::with_api_name(name), Some(effort));
    }
}

#[test]
fn lists_every_effort_from_the_least_to_the_most() {
    assert_eq!(
        Effort::ALL,
        [
            Effort::Low,
            Effort::Medium,
            Effort::High,
            Effort::ExtraHigh,
            Effort::Max
        ]
    );
}

#[test]
fn refuses_an_effort_the_api_does_not_name() {
    assert_eq!(Effort::with_api_name("extra-high"), None);
    assert_eq!(Effort::with_api_name("HIGH"), None);
}

#[test]
fn names_each_effort_to_the_user_with_its_own_text() {
    let texts = [
        (Effort::Low, Text::EffortLow),
        (Effort::Medium, Text::EffortMedium),
        (Effort::High, Text::EffortHigh),
        (Effort::ExtraHigh, Text::EffortExtraHigh),
        (Effort::Max, Text::EffortMax),
    ];

    for (effort, text) in texts {
        assert_eq!(effort.text(), text);
    }
}

#[test]
fn lists_dot_s_own_summaries_first() {
    assert_eq!(
        Summaries::ALL,
        [Summaries::WhenTheyPay, Summaries::LeftToClaude]
    );
}

#[test]
fn keeps_the_wanted_summaries_when_dot_may_choose() {
    for wanted in Summaries::ALL {
        assert_eq!(wanted.within(Summaries::WhenTheyPay), wanted);
    }
}

#[test]
fn leaves_summaries_to_claude_when_the_user_set_them_in_claude_code() {
    for wanted in Summaries::ALL {
        assert_eq!(
            wanted.within(Summaries::LeftToClaude),
            Summaries::LeftToClaude
        );
    }
}
