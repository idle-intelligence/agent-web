//! [`Generator`] on lean's engine: [`lean::Lean`] picks its backend by
//! capability at load (WebGPU, then the CPU forward pass on a thread pool,
//! then on one thread) and reuses the KV cache of the previous call for the
//! longest prefix the next prompt shares with it, so each agent step only
//! prefills what the previous one did not already hold (the system prompt
//! and tools are prefilled once per turn). One agent per engine.
//!
//! lean checks for an abort once per decoded token, not during prefill;
//! the agent's flag reaches lean's own flag each time a piece of text is
//! streamed.
//!
//! An agent on lean can also prefill the part of the prompt every turn
//! shares (the system prompt and the tools, [`Agent::prompt_prefix`])
//! before the first utterance, and save or restore its KV cache as a lean
//! prefix image, so a later session skips that prefill.

use std::io::{Read, Write};

use lean::{GenParams, Lean, LeanError, LoadOptions};

use crate::agent::{Agent, Tool};
use crate::generator::{AbortFlag, GenerateError, Generator};

/// Greedy decoding on a loaded [`Lean`].
pub struct LeanGenerator {
    lean: Lean,
}

impl LeanGenerator {
    pub async fn load(opts: LoadOptions) -> Result<Self, LeanError> {
        Ok(Self::new(Lean::load(opts).await?))
    }

    pub fn new(lean: Lean) -> Self {
        Self { lean }
    }

    /// The backend lean picked: `"webgpu"`, `"threads"` or `"single"`.
    pub fn backend(&self) -> &'static str {
        self.lean.backend()
    }
}

/// The prompt prefix of [`Agent::prompt_prefix`], with a render failure as
/// a [`LeanError`].
fn prefix_of(agent: &Agent<LeanGenerator>, tools: &[Tool]) -> Result<String, LeanError> {
    agent
        .prompt_prefix(tools)
        .map_err(|e| LeanError::Other(e.context("rendering the prompt prefix")))
}

/// Prefix KV images of the agent's prompt prefix. Each call renders the
/// prefix of [`Agent::prompt_prefix`] for `tools` (the same text the first
/// step of [`Agent::start`] begins with) and passes it to lean's call of
/// the same name.
impl Agent<LeanGenerator> {
    /// Prefills the system prompt and `tools` without generating, so the
    /// next [`Agent::start`] with these tools prefills only the user's
    /// words. Returns the prefix's token count.
    pub async fn prefill(&mut self, tools: &[Tool]) -> Result<usize, LeanError> {
        let prefix = prefix_of(self, tools)?;
        self.generator_mut().lean.prefill(&prefix).await
    }

    /// The storage key of the prefix image for `tools` on this engine (see
    /// [`Lean::prefix_key`]): it changes with the system prompt, the tools,
    /// the chat template, the model, the engine build and the backend.
    pub fn prefix_key(&self, tools: &[Tool]) -> Result<String, LeanError> {
        let prefix = prefix_of(self, tools)?;
        self.generator().lean.prefix_key(&prefix)
    }

    /// Writes the prefix image for `tools` to `out` (see
    /// [`Lean::save_prefix`]), prefilling first if the prefix is not
    /// resident. Returns the number of bytes written.
    pub async fn save_prefix(
        &mut self,
        tools: &[Tool],
        out: &mut impl Write,
    ) -> Result<u64, LeanError> {
        let prefix = prefix_of(self, tools)?;
        self.generator_mut().lean.save_prefix(&prefix, out).await
    }

    /// Restores a prefix image written by [`Agent::save_prefix`] for the
    /// same `tools` (see [`Lean::restore_prefix`]). A refused image is
    /// [`LeanError::PrefixImage`]: prefill again and overwrite it.
    pub fn restore_prefix(
        &mut self,
        tools: &[Tool],
        input: &mut impl Read,
    ) -> Result<(), LeanError> {
        let prefix = prefix_of(self, tools)?;
        self.generator_mut().lean.restore_prefix(&prefix, input)
    }
}

impl Generator for LeanGenerator {
    async fn generate(
        &mut self,
        prompt: &str,
        max_new_tokens: usize,
        on_text: &mut dyn FnMut(&str),
        abort: &AbortFlag,
    ) -> Result<String, GenerateError> {
        if abort.is_aborted() {
            return Err(GenerateError::Aborted);
        }
        let params = GenParams {
            max_new_tokens: u32::try_from(max_new_tokens).unwrap_or(u32::MAX),
            temperature: 0.0,
            ..GenParams::default()
        };
        // lean checks its own flag once per token; this call's caller
        // holds the agent's flag, so it is forwarded after each piece
        // of text.
        let stop = lean::AbortFlag::new();
        let reply = self
            .lean
            .generate(
                prompt,
                &params,
                |text| {
                    on_text(text);
                    if abort.is_aborted() {
                        stop.abort();
                    }
                },
                Some(&stop),
            )
            .await;
        match reply {
            Ok(_) if abort.is_aborted() => Err(GenerateError::Aborted),
            Ok(text) => Ok(text),
            Err(LeanError::ContextOverflow { needed, available }) => {
                Err(GenerateError::ContextOverflow { needed, available })
            }
            Err(e @ LeanError::PrefixImage(_)) => Err(GenerateError::Other(e.into())),
            Err(LeanError::Other(e)) => Err(GenerateError::Other(e)),
        }
    }

    fn count_tokens(&self, text: &str) -> Option<usize> {
        self.lean.token_count(text).ok()
    }
}
