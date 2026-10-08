//! wasm-bindgen surface for the tool-calling agent, built on lean's public
//! native CPU engine (`lean::cpu`). A browser can't call an MCP tool
//! synchronously (it's an async round trip through the page), so
//! `start`/`provideToolResults` each return one JSON "outcome" object the
//! page inspects to decide whether to call tools or show the final
//! answer:
//!
//! - `{"type": "needTools", "calls": [{"callId", "name", "arguments"}]}`
//! - `{"type": "final", "text": "..."}`
//! - `{"type": "error", "message": "..."}`
//!
//! Only the CPU engine is wired here.

use std::io::Cursor;
use std::rc::Rc;

use wasm_bindgen::prelude::*;

use agent::agent::{Agent, AgentChatTemplate, Generator, PendingToolCall, StepOutcome, ToolCaller};
use anyhow::Result;
use serde_json::Value;

/// Greedy CPU generator over a shared (`Rc`) `CpuModel`. `Rc`, not a
/// borrow, so the `Agent` that owns one of these can live inside
/// `AgentCpu` directly and persist across `start`/`provideToolResults`
/// calls.
struct RcCpuGenerator {
    model: Rc<lean::cpu::CpuModel>,
}

impl Generator for RcCpuGenerator {
    fn generate(&mut self, prompt_ids: &[u32], max_new_tokens: usize) -> Result<Vec<u32>> {
        let max_ctx = prompt_ids.len() + max_new_tokens;
        let mut cache = lean::cpu::CpuKvCache::new(&self.model.config, max_ctx);
        let logits = lean::cpu::forward_prefill(&self.model, &mut cache, prompt_ids);
        let mut next_id = lean::model::argmax(&logits);
        let mut generated = Vec::with_capacity(max_new_tokens);
        for _ in 0..max_new_tokens {
            if self.model.config.eos_token_ids.contains(&next_id) {
                break;
            }
            generated.push(next_id);
            next_id = lean::cpu::forward_decode_step_argmax(&self.model, &mut cache, next_id);
        }
        Ok(generated)
    }
}

/// No-op `ToolCaller`: the step-wise wasm API never calls tools itself,
/// `start`/`provide_tool_results` only ever reach `StepOutcome::NeedTools`
/// or `StepOutcome::Final`, never `Agent::run`.
struct NoCaller;
impl ToolCaller for NoCaller {
    fn call(&mut self, name: &str, _args: &Value) -> Result<Value> {
        anyhow::bail!("AgentCpu never calls tools itself (tool: {name}); the page does, via provideToolResults")
    }
}

fn outcome_to_json(outcome: StepOutcome) -> String {
    match outcome {
        StepOutcome::NeedTools { calls, .. } => {
            let calls: Vec<Value> = calls.iter().map(|c: &PendingToolCall| serde_json::json!({"callId": c.call_id, "name": c.name, "arguments": c.arguments})).collect();
            serde_json::json!({"type": "needTools", "calls": calls}).to_string()
        }
        StepOutcome::Final { text, .. } => {
            serde_json::json!({"type": "final", "text": text}).to_string()
        }
        StepOutcome::Error { message, .. } => {
            serde_json::json!({"type": "error", "message": message}).to_string()
        }
    }
}

#[wasm_bindgen]
pub struct AgentCpu {
    agent: Option<Agent<RcCpuGenerator, NoCaller>>,
}

#[wasm_bindgen]
impl AgentCpu {
    #[wasm_bindgen(js_name = create)]
    pub fn create() -> AgentCpu {
        console_error_panic_hook::set_once();
        AgentCpu { agent: None }
    }

    /// `gguf_bytes` is copied out of the `Uint8Array` into a `Vec<u8>` and
    /// wrapped in a `std::io::Cursor`, matching `lean::cpu::CpuModel`'s own
    /// doc comment on how its wasm callers are expected to load a GGUF
    /// (`load_from_reader` is generic over any `Read + Seek`, no
    /// lean-internal wasm-byte-reader type is needed here).
    #[wasm_bindgen(js_name = load)]
    pub fn load(
        &mut self,
        gguf_bytes: js_sys::Uint8Array,
        tokenizer_json: String,
        tokenizer_config_json: String,
        system_prompt: String,
        max_new_tokens: u32,
    ) -> Result<(), JsError> {
        let bytes = gguf_bytes.to_vec();
        let model = lean::cpu::CpuModel::load_from_reader(Cursor::new(bytes))
            .map_err(|e| JsError::new(&format!("failed to load model: {e}")))?;
        let tokenizer = tokenizers::Tokenizer::from_bytes(tokenizer_json.as_bytes())
            .map_err(|e| JsError::new(&format!("failed to load tokenizer.json: {e}")))?;
        let template = AgentChatTemplate::from_tokenizer_config_json(&tokenizer_config_json)
            .map_err(|e| JsError::new(&format!("{e}")))?;
        let generator = RcCpuGenerator {
            model: Rc::new(model),
        };
        self.agent = Some(Agent::new(
            template,
            tokenizer,
            generator,
            NoCaller,
            system_prompt,
            max_new_tokens as usize,
        ));
        Ok(())
    }

    /// Begin a new turn. `tools_json` is a JSON array of MCP `tools/list`
    /// entries (`{"name", "description", "inputSchema"}`); pass `"[]"` for
    /// no tools. Returns one outcome JSON object, see the module doc.
    #[wasm_bindgen(js_name = start)]
    pub fn start(&mut self, utterance: String, tools_json: String) -> Result<String, JsError> {
        let raw_tools: Vec<Value> = serde_json::from_str(&tools_json)
            .map_err(|e| JsError::new(&format!("invalid tools JSON: {e}")))?;
        let agent = self
            .agent
            .as_mut()
            .ok_or_else(|| JsError::new("load() must be called first"))?;
        Ok(outcome_to_json(
            agent.start_from_mcp(&utterance, &raw_tools),
        ))
    }

    /// Continue the turn with tool results. `results_json` is a JSON array
    /// of `[callId, result]` pairs, `result` being whatever JSON value the
    /// tool call returned (an MCP-shaped `{"isError": true, ...}` is read
    /// as an error). Returns one outcome JSON object, same shape as
    /// `start`.
    #[wasm_bindgen(js_name = provideToolResults)]
    pub fn provide_tool_results(&mut self, results_json: String) -> Result<String, JsError> {
        let results: Vec<(String, Value)> = serde_json::from_str(&results_json)
            .map_err(|e| JsError::new(&format!("invalid results JSON: {e}")))?;
        let agent = self
            .agent
            .as_mut()
            .ok_or_else(|| JsError::new("load() must be called first"))?;
        Ok(outcome_to_json(agent.provide_tool_results(results)))
    }

    /// Abandon the current turn (e.g. after an `error` outcome) without
    /// reloading the model.
    #[wasm_bindgen(js_name = reset)]
    pub fn reset(&mut self) {
        if let Some(agent) = self.agent.as_mut() {
            agent.reset();
        }
    }
}
