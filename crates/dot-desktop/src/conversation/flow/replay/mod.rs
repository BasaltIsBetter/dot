use std::time::Instant;

use dot_api::{Happened, ImageLabel, Opening, Record, ReplyFailure, Text, Translator};

use crate::conversation::flow::{Conversation, size};
use crate::conversation::prompter::label;
use crate::error::Problem;

impl Conversation {
    pub(crate) fn replay(
        &mut self,
        records: Vec<Record<'static>>,
        start: Instant,
        translator: &impl Translator,
    ) {
        let stopped = translator
            .translate(ReplyFailure::Stopped.text())
            .into_owned();
        let mut stopped_at = None;
        let mut now = start;

        for Record { at, happened } in records {
            now = start.checked_add(at).unwrap_or(now);
            if let Some(at) = stopped_at.take()
                && !is_a_redirect(&happened)
            {
                self.end_reply(Some(&stopped), None, at);
            }
            match happened {
                Happened::Asked(asked) => {
                    if self.transcript.is_live() {
                        match asked.opening {
                            Opening::Redirect => self.end_without_a_word(now),
                            Opening::Prompt | Opening::Note => {
                                self.end_reply(Some(&stopped), None, now);
                            }
                        }
                    }
                    let labels: Vec<String> = asked
                        .images
                        .iter()
                        .map(|image| self.image_label(image, translator))
                        .collect();
                    self.show_opening(asked.opening, &asked.text, &labels, now);
                }
                Happened::Heard(event) => self.hear(event.into_owned(), now, translator),
                Happened::Ended(ending) => match ending.failure {
                    None => {
                        self.end_reply(None, Some(ending.spent.output_tokens), now);
                    }
                    Some(failure) if failure.text == ReplyFailure::Stopped.text() => {
                        stopped_at = Some(now);
                    }
                    Some(failure) => {
                        let problem = Problem {
                            text: failure.text,
                            cause: failure.cause,
                        };
                        self.end_reply(Some(&problem.notice(translator)), None, now);
                    }
                },
                Happened::StartedOver => {
                    if self.transcript.is_live() {
                        self.end_reply(Some(&stopped), None, now);
                    }
                    let restarted = translator.translate(Text::ConversationRestarted);
                    self.notice(&restarted, now);
                }
                Happened::Carried(_) => {}
            }
        }

        if self.transcript.is_live() {
            self.end_reply(Some(&stopped), None, stopped_at.unwrap_or(now));
        }
        self.reader.come_back();
    }

    pub(crate) fn has_exchanges(&self) -> bool {
        self.transcript.exchanges() > 0
    }

    pub(crate) fn is_untouched(&self) -> bool {
        !self.has_exchanges()
            && !self.transcript.is_live()
            && self.prompter.is_empty()
            && self.queued.is_none()
            && self.pause.is_none()
    }

    fn image_label(&self, image: &ImageLabel, translator: &impl Translator) -> String {
        let kind = translator.translate(Text::ImageLabel);
        let bytes = usize::try_from(image.bytes).unwrap_or(usize::MAX);
        label(
            &kind,
            &image.name,
            &size(bytes, translator),
            self.prompter.width(),
        )
    }
}

const fn is_a_redirect(happened: &Happened<'_>) -> bool {
    matches!(
        happened,
        Happened::Asked(asked) if matches!(asked.opening, Opening::Redirect)
    )
}

#[cfg(test)]
#[path = "../../../../../../tests/dot-desktop/conversation/flow/replay/mod.rs"]
mod tests;
