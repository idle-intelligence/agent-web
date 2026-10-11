//! A scripted [`Generator`] for the unit tests.

use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;

use crate::generator::{AbortFlag, GenerateError, Generator};

type Reply = Box<dyn FnMut(&str) -> Result<String, GenerateError>>;

/// Answers each prompt with `reply(prompt)`, streams the reply to
/// `on_text`, and records every prompt it was given. Like an engine, it
/// returns [`GenerateError::Aborted`] when the flag is set before the call
/// or while the reply streams.
pub(crate) struct FakeGenerator {
    reply: Reply,
    prompts: Rc<RefCell<Vec<String>>>,
}

impl FakeGenerator {
    pub(crate) fn new(reply: impl FnMut(&str) -> Result<String, GenerateError> + 'static) -> Self {
        Self {
            reply: Box::new(reply),
            prompts: Rc::default(),
        }
    }

    /// Replies with `replies` in order, then fails.
    pub(crate) fn script(replies: &[&str]) -> Self {
        let mut replies: VecDeque<String> = replies.iter().map(|r| r.to_string()).collect();
        Self::new(move |_| {
            replies.pop_front().ok_or_else(|| {
                GenerateError::Other(anyhow::anyhow!("FakeGenerator script exhausted"))
            })
        })
    }

    /// Every prompt generated from so far, shared with the generator.
    pub(crate) fn prompts(&self) -> Rc<RefCell<Vec<String>>> {
        self.prompts.clone()
    }
}

impl Generator for FakeGenerator {
    fn generate(
        &mut self,
        prompt: &str,
        _max_new_tokens: usize,
        on_text: &mut dyn FnMut(&str),
        abort: &AbortFlag,
    ) -> impl std::future::Future<Output = Result<String, GenerateError>> {
        let reply = if abort.is_aborted() {
            Err(GenerateError::Aborted)
        } else {
            self.prompts.borrow_mut().push(prompt.to_string());
            match (self.reply)(prompt) {
                Ok(text) => {
                    on_text(&text);
                    if abort.is_aborted() {
                        Err(GenerateError::Aborted)
                    } else {
                        Ok(text)
                    }
                }
                Err(e) => Err(e),
            }
        };
        std::future::ready(reply)
    }
}
