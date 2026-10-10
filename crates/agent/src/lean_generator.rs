//! [`Generator`] on lean's engine: [`lean::Lean`] picks its backend by
//! capability at load (WebGPU, then the CPU forward pass on a thread pool,
//! then on one thread) and reuses the KV cache of the previous call for the
//! longest prefix the next prompt shares with it, so each agent step only
//! prefills what the previous one did not already hold (the system prompt
//! and tools are prefilled once per conversation). One agent per engine.

use lean::{GenParams, Lean, LeanError, LoadOptions};

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
            max_new_tokens: max_new_tokens as u32,
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
            Err(LeanError::Other(e)) => Err(GenerateError::Other(e)),
        }
    }
}
