//! What the agent loop needs from an inference engine: prompt text in,
//! reply text out, streamed as it is decoded, with a way to stop it and a
//! typed error when the prompt does not fit the engine's context.
//!
//! The loop renders every prompt itself (the model's chat template, with
//! the tools) and parses the reply itself, so any engine that can complete
//! a piece of text can drive it. `crate::lean_generator` implements it on
//! lean's engine.

use std::fmt;
use std::future::Future;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// Why a [`Generator::generate`] call produced no reply.
#[derive(Debug)]
pub enum GenerateError {
    /// The prompt plus `max_new_tokens` needs `needed` tokens and the
    /// engine's context holds `available`. Nothing was generated and the
    /// engine is still usable.
    ContextOverflow { needed: usize, available: usize },
    /// The call was stopped through its [`AbortFlag`].
    Aborted,
    /// Anything else (the engine failed).
    Other(anyhow::Error),
}

impl fmt::Display for GenerateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GenerateError::ContextOverflow { needed, available } => {
                write!(
                    f,
                    "context overflow: needed {needed} tokens, {available} available"
                )
            }
            GenerateError::Aborted => write!(f, "aborted"),
            GenerateError::Other(e) => write!(f, "{e:#}"),
        }
    }
}

impl std::error::Error for GenerateError {}

impl From<anyhow::Error> for GenerateError {
    fn from(e: anyhow::Error) -> Self {
        GenerateError::Other(e)
    }
}

/// Stops a running generation: keep a clone, call [`AbortFlag::abort`]
/// from anywhere (another thread, a message handler), and the call in
/// flight returns [`GenerateError::Aborted`] at its next token.
#[derive(Debug, Clone, Default)]
pub struct AbortFlag(Arc<AtomicBool>);

impl AbortFlag {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn abort(&self) {
        self.0.store(true, Ordering::Relaxed);
    }

    pub fn is_aborted(&self) -> bool {
        self.0.load(Ordering::Relaxed)
    }

    /// Clears the flag, for the next turn.
    pub fn reset(&self) {
        self.0.store(false, Ordering::Relaxed);
    }
}

/// An engine that completes text. The agent loop drives it from one
/// thread (a browser worker, or a blocking executor natively), so the
/// returned future does not need to be `Send`.
pub trait Generator {
    /// Generates the continuation of `prompt`, exactly as given (the
    /// prompt is already rendered with the chat template), up to
    /// `max_new_tokens` tokens, stopping early at the model's
    /// end-of-sequence token. Each piece of the reply goes to `on_text` as
    /// soon as it is decoded; the whole reply is returned. Returns
    /// [`GenerateError::Aborted`] once `abort` is set, at the engine's
    /// next check.
    fn generate(
        &mut self,
        prompt: &str,
        max_new_tokens: usize,
        on_text: &mut dyn FnMut(&str),
        abort: &AbortFlag,
    ) -> impl Future<Output = Result<String, GenerateError>>;
}
