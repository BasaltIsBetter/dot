use serde::Deserialize;

const TEXT: &str = "text";
const TOOL_USE: &str = "tool_use";
const ASSISTANT: &str = "assistant";
const RESULT: &str = "result";
const LONGEST_ANSWER: usize = 256 * 1_024;
const CUT: &str = "\n[the rest of the answer was cut]";

/// What an agent does while it answers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CallEvent {
    /// It wrote this.
    Text(String),
    /// It uses the tool of this name.
    Tool(String),
}

/// What an agent answered.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Answer {
    /// Its answer.
    pub text: String,
    /// What it cost, in dollars, when the agent tells.
    pub cost: Option<f64>,
}

/// How what an agent prints is understood.
#[derive(Debug)]
pub(crate) enum Reader {
    Claude(Option<Ended>),
    Plain(String),
}

#[derive(Debug)]
pub(crate) struct Ended {
    answer: Answer,
    failed: bool,
}

#[derive(Deserialize)]
struct Line {
    #[serde(rename = "type", default)]
    kind: String,
    #[serde(default)]
    message: Message,
    #[serde(default)]
    result: String,
    #[serde(default)]
    is_error: bool,
    #[serde(default)]
    total_cost_usd: Option<f64>,
}

#[derive(Default, Deserialize)]
struct Message {
    #[serde(default)]
    content: Vec<Block>,
}

#[derive(Deserialize)]
struct Block {
    #[serde(rename = "type", default)]
    kind: String,
    #[serde(default)]
    text: String,
    #[serde(default)]
    name: String,
}

impl Reader {
    pub(crate) const fn of(is_claude: bool) -> Self {
        if is_claude {
            Self::Claude(None)
        } else {
            Self::Plain(String::new())
        }
    }

    pub(crate) fn hear(&mut self, line: &[u8], on_event: &mut dyn FnMut(CallEvent)) {
        match self {
            Self::Plain(text) => {
                let line = String::from_utf8_lossy(line);
                on_event(CallEvent::Text(line.clone().into_owned()));
                if text.len() <= LONGEST_ANSWER {
                    text.push_str(&line);
                    text.push('\n');
                }
            }
            Self::Claude(ended) => {
                let Ok(line) = serde_json::from_slice::<Line>(line) else {
                    return;
                };
                match line.kind.as_str() {
                    ASSISTANT => line.message.content.into_iter().for_each(|block| {
                        match block.kind.as_str() {
                            TEXT => on_event(CallEvent::Text(block.text)),
                            TOOL_USE => on_event(CallEvent::Tool(block.name)),
                            _ => {}
                        }
                    }),
                    RESULT => {
                        *ended = Some(Ended {
                            failed: line.is_error,
                            answer: Answer {
                                text: line.result,
                                cost: line.total_cost_usd,
                            },
                        });
                    }
                    _ => {}
                }
            }
        }
    }

    /// The answer, or what the agent said of its failure; `None` when it ended without a word.
    pub(crate) fn answer(self) -> Option<Result<Answer, String>> {
        match self {
            Self::Plain(text) => Some(Ok(Answer {
                text: bounded(text.trim_end().to_owned()),
                cost: None,
            })),
            Self::Claude(None) => None,
            Self::Claude(Some(Ended { answer, failed })) => Some(if failed {
                Err(bounded(answer.text))
            } else {
                Ok(Answer {
                    text: bounded(answer.text),
                    ..answer
                })
            }),
        }
    }
}

fn bounded(mut text: String) -> String {
    if text.len() > LONGEST_ANSWER {
        text.truncate(text.floor_char_boundary(LONGEST_ANSWER));
        text.push_str(CUT);
    }
    text
}

#[cfg(test)]
#[path = "../../../../tests/dot-container/fleet/answer.rs"]
mod tests;
