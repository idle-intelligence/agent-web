//! wasm-bindgen surface for the tool-calling agent, on lean's engine.
//! lean picks its backend by capability when the model loads: WebGPU when
//! the browser grants an adapter and the model loads on it, otherwise the
//! CPU forward pass (SIMD128, one thread).
//!
//! A browser can't call an MCP tool synchronously (it's an async round
//! trip through the page), so `start`/`provideToolResults` each resolve to
//! one JSON "outcome" object the page inspects to decide whether to call
//! tools or show the final answer:
//!
//! - `{"type": "needTools", "calls": [{"callId", "name", "arguments"}]}`
//! - `{"type": "final", "text": "..."}`
//! - `{"type": "contextOverflow", "needed": n, "available": n}`
//! - `{"type": "aborted"}`
//! - `{"type": "error", "message": "..."}`
//!
//! Run it in a Web Worker. While `start` or `provideToolResults` is
//! running, the agent is busy: any other call that needs it (a second
//! `start`, `provideToolResults`, `setOnText`, `reset`) throws an "agent is
//! busy" error. `backend` and `abortHandle()` always work; stop a running
//! step through the handle.

use std::cell::RefCell;

use wasm_bindgen::prelude::*;

use agent::agent::{AbortFlag, Agent, PendingToolCall, StepOutcome};
use agent::lean_generator::LeanGenerator;
use anyhow::Result;
use lean::{Backend, GgufSource, LoadOptions, TokenizerSource};
use serde_json::Value;

fn outcome_to_json(outcome: StepOutcome) -> String {
    match outcome {
        StepOutcome::NeedTools { calls, .. } => {
            let calls: Vec<Value> = calls.iter().map(|c: &PendingToolCall| serde_json::json!({"callId": c.call_id, "name": c.name, "arguments": c.arguments})).collect();
            serde_json::json!({"type": "needTools", "calls": calls}).to_string()
        }
        StepOutcome::Final { text, .. } => {
            serde_json::json!({"type": "final", "text": text}).to_string()
        }
        StepOutcome::ContextOverflow { needed, available } => {
            serde_json::json!({"type": "contextOverflow", "needed": needed, "available": available})
                .to_string()
        }
        StepOutcome::Aborted => serde_json::json!({"type": "aborted"}).to_string(),
        StepOutcome::Error { message, .. } => {
            serde_json::json!({"type": "error", "message": message}).to_string()
        }
    }
}

/// Stops the agent's running step; see the module doc.
#[wasm_bindgen]
pub struct AgentAbort(AbortFlag);

#[wasm_bindgen]
impl AgentAbort {
    pub fn abort(&self) {
        self.0.abort();
    }
}

#[wasm_bindgen(js_name = Agent)]
pub struct AgentJs {
    /// `None` while a step runs.
    agent: RefCell<Option<Agent<LeanGenerator>>>,
    backend: &'static str,
    abort: AbortFlag,
}

#[wasm_bindgen(js_class = Agent)]
impl AgentJs {
    /// `gguf_bytes` is the GGUF file, `tokenizer_json` the text of
    /// `tokenizer.json`, `tokenizer_config_json` the text of
    /// `tokenizer_config.json` (its `chat_template` renders every prompt).
    /// `max_ctx` is the KV cache size in tokens: each step's prompt plus
    /// `max_new_tokens` must fit.
    pub async fn load(
        gguf_bytes: js_sys::Uint8Array,
        tokenizer_json: String,
        tokenizer_config_json: String,
        system_prompt: String,
        max_new_tokens: u32,
        max_ctx: u32,
    ) -> Result<AgentJs, JsError> {
        console_error_panic_hook::set_once();
        let template = lean_chat::template_from_tokenizer_config(&tokenizer_config_json)
            .map_err(|e| JsError::new(&format!("{e:#}")))?;
        let generator = LeanGenerator::load(LoadOptions {
            gguf: GgufSource::Bytes(gguf_bytes.to_vec()),
            tokenizer: TokenizerSource::Json(tokenizer_json),
            max_ctx,
            backend: Backend::Auto,
        })
        .await
        .map_err(|e| JsError::new(&format!("failed to load model: {e}")))?;
        let backend = generator.backend();
        let agent = Agent::new(template, generator, system_prompt, max_new_tokens as usize);
        Ok(AgentJs {
            abort: agent.abort_flag(),
            agent: RefCell::new(Some(agent)),
            backend,
        })
    }

    /// The backend lean picked: `"webgpu"`, `"threads"` or `"single"`.
    #[wasm_bindgen(getter)]
    pub fn backend(&self) -> String {
        self.backend.to_string()
    }

    /// `on_text(text)` receives the model's raw output as it is generated,
    /// every step. It must not call this agent.
    #[wasm_bindgen(js_name = setOnText)]
    pub fn set_on_text(&self, on_text: js_sys::Function) -> Result<(), JsError> {
        let mut agent = self.take()?;
        agent.set_on_text(move |text| {
            let _ = on_text.call1(&JsValue::NULL, &JsValue::from_str(text));
        });
        self.put(agent);
        Ok(())
    }

    /// A handle whose `abort()` stops the running step (it resolves to an
    /// `aborted` outcome). Take it before calling `start`; it stays valid
    /// for every later turn.
    #[wasm_bindgen(js_name = abortHandle)]
    pub fn abort_handle(&self) -> AgentAbort {
        AgentAbort(self.abort.clone())
    }

    /// Begin a new turn. `tools_json` is a JSON array of MCP `tools/list`
    /// entries (`{"name", "description", "inputSchema"}`); pass `"[]"` for
    /// no tools. Resolves to one outcome JSON object, see the module doc.
    pub async fn start(&self, utterance: String, tools_json: String) -> Result<String, JsError> {
        let raw_tools: Vec<Value> = serde_json::from_str(&tools_json)
            .map_err(|e| JsError::new(&format!("invalid tools JSON: {e}")))?;
        let mut agent = self.take()?;
        let outcome = agent.start_from_mcp(&utterance, &raw_tools).await;
        self.put(agent);
        Ok(outcome_to_json(outcome))
    }

    /// Continue the turn with tool results. `results_json` is a JSON array
    /// of `[callId, result]` pairs, `result` being whatever JSON value the
    /// tool call returned (an MCP-shaped `{"isError": true, ...}` is read
    /// as an error). Resolves to one outcome JSON object, same shape as
    /// `start`.
    #[wasm_bindgen(js_name = provideToolResults)]
    pub async fn provide_tool_results(&self, results_json: String) -> Result<String, JsError> {
        let results: Vec<(String, Value)> = serde_json::from_str(&results_json)
            .map_err(|e| JsError::new(&format!("invalid results JSON: {e}")))?;
        let mut agent = self.take()?;
        let outcome = agent.provide_tool_results(results).await;
        self.put(agent);
        Ok(outcome_to_json(outcome))
    }

    /// Abandon the current turn (e.g. after an `error` outcome) without
    /// reloading the model.
    pub fn reset(&self) -> Result<(), JsError> {
        let mut agent = self.take()?;
        agent.reset();
        self.put(agent);
        Ok(())
    }
}

impl AgentJs {
    /// The agent, out of its cell for the duration of one call; an error
    /// while another call holds it.
    fn take(&self) -> Result<Agent<LeanGenerator>, JsError> {
        self.agent
            .borrow_mut()
            .take()
            .ok_or_else(|| JsError::new("agent is busy: a step is still running"))
    }

    fn put(&self, agent: Agent<LeanGenerator>) {
        *self.agent.borrow_mut() = Some(agent);
    }
}
