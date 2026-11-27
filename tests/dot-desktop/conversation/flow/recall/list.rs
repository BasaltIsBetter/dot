use super::*;

fn recall_of(prompts: &[&str]) -> Recall {
    let mut recall = Recall::default();
    for prompt in prompts {
        recall.add(prompt);
    }
    recall
}

#[test]
fn keeps_each_prompt_once_the_latest_last() {
    let recall = recall_of(&["a", "b", "a", "c", "c"]);

    assert_eq!(recall.prompts(), ["b", "a", "c"]);
    assert_eq!(recall.changes(), 4);
}

#[test]
fn keeps_no_blank_prompt_nor_one_too_large() {
    let largest = "x".repeat(LARGEST_PROMPT);
    let larger = "x".repeat(LARGEST_PROMPT + 1);

    let recall = recall_of(&["", "  \n ", &larger, &largest]);

    assert_eq!(recall.prompts(), [largest]);
    assert_eq!(recall.changes(), 1);
}

#[test]
fn forgets_the_oldest_beyond_its_count() {
    let mut recall = Recall::default();
    for number in 0..=MOST_PROMPTS {
        recall.add(&number.to_string());
    }

    assert_eq!(recall.prompts().len(), MOST_PROMPTS);
    assert_eq!(recall.prompts()[0], "1");
    assert_eq!(recall.prompts()[MOST_PROMPTS - 1], MOST_PROMPTS.to_string());
}

#[test]
fn forgets_the_oldest_beyond_its_size() {
    let mut recall = Recall::default();
    let count = LARGEST_HISTORY / LARGEST_PROMPT;
    let padding = "x".repeat(LARGEST_PROMPT - 4);
    for number in 0..=count {
        recall.add(&format!("{number:04}{padding}"));
    }

    assert_eq!(recall.prompts().len(), count);
    assert!(recall.prompts()[0].starts_with("0001"));
}

#[test]
fn puts_the_kept_prompts_before_those_of_this_launch() {
    let mut recall = recall_of(&["new", "both"]);

    recall.add_older(vec![
        "old".to_owned(),
        "both".to_owned(),
        String::new(),
        "old".to_owned(),
    ]);

    assert_eq!(recall.prompts(), ["old", "new", "both"]);
    assert_eq!(recall.changes(), 2);
}

#[test]
fn trims_the_kept_prompts_beyond_its_count() {
    let mut recall = recall_of(&["new"]);
    let older: Vec<String> = (0..MOST_PROMPTS).map(|number| number.to_string()).collect();

    recall.add_older(older);

    assert_eq!(recall.prompts().len(), MOST_PROMPTS);
    assert_eq!(recall.prompts().last().map(String::as_str), Some("new"));
    assert_eq!(recall.prompts()[0], "1");
}

#[test]
fn walks_from_the_latest_to_the_oldest_and_back() {
    let recall = recall_of(&["a", "b", "c"]);

    assert_eq!(recall.older(None), Some((2, "c")));
    assert_eq!(recall.older(Some(2)), Some((1, "b")));
    assert_eq!(recall.older(Some(0)), None);
    assert_eq!(recall.older(Some(9)), Some((2, "c")));
    assert_eq!(recall.newer(0), Some((1, "b")));
    assert_eq!(recall.newer(2), None);
    assert_eq!(recall.newer(usize::MAX), None);
    assert_eq!(Recall::default().older(None), None);
}

const SIXTY_FOUR_KIBIBYTES: usize = 64 * 1_024;

fn large(number: usize) -> String {
    format!("{number:04}{}", "x".repeat(SIXTY_FOUR_KIBIBYTES - 4))
}

#[test]
fn keeps_a_mebibyte_of_prompts_and_counts_each_once() {
    let mut recall = Recall::default();
    for number in 0..16 {
        recall.add(&large(number));
    }
    assert_eq!(recall.prompts().len(), 16);

    recall.add(&large(0));
    assert_eq!(recall.prompts().len(), 16);
    recall.add(&large(16));
    recall.add(&large(17));

    assert_eq!(recall.prompts().len(), 16);
    assert_eq!(recall.prompts()[0], large(3));
}

#[test]
fn counts_each_kept_prompt_once_when_they_come_back() {
    let mut recall = Recall::default();
    let older: Vec<String> = (0..16).chain([0, 1]).map(large).collect();

    recall.add_older(older);
    assert_eq!(recall.prompts().len(), 16);
    recall.add(&large(16));
    recall.add(&large(17));

    assert_eq!(recall.prompts().len(), 16);
    assert_eq!(recall.prompts().last(), Some(&large(17)));
}
