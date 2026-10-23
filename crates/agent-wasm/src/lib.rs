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
//! - `{"type": "needTools", "calls": [{"callId", "name", "arguments"}], "timing": T}`
//! - `{"type": "final", "text": "...", "timing": T}`
//! - `{"type": "contextOverflow", "needed": n, "available": n}`
//! - `{"type": "aborted"}`
//! - `{"type": "error", "message": "..."}`
//!
//! `T` is `{"promptTokens", "thinkTokens", "answerTokens"}`, the token
//! counts of the step.
//!
//! Run it in a Web Worker. While `start`, `provideToolResults`, `prefill`
//! or `savePrefix` is running, the agent is busy: any other call that
//! needs it (a second `start`, `provideToolResults`, `setOnText`, `reset`,
//! the prefix calls) throws an "agent is busy" error. `backend` and
//! `abortHandle()` always work; stop a running step through the handle.
//!
//! The system prompt and the tools, the part of the prompt every turn
//! starts with, can be prefilled before the first utterance (`prefill`)
//! and their KV cache saved and restored as a lean prefix image
//! (`prefixKey`, `savePrefix`, `restorePrefix`), so a later session skips
//! that prefill. Where the image is kept is the page's choice.

use std::cell::RefCell;
use std::io::{Read, Write};

use wasm_bindgen::prelude::*;

use agent::agent::{
    tools_from_mcp, AbortFlag, Agent, PendingToolCall, Step, StepOutcome, Thinking, Tool,
};
use agent::lean_generator::LeanGenerator;
use anyhow::Result;
use lean::{Backend, GgufSource, LeanError, LoadOptions, PrefixImageError, TokenizerSource};
use serde_json::Value;

/// Token counts of a step, as the `timing` field of `needTools` and
/// `final` outcomes. Times are not reported: wasm32 has no monotonic clock
/// in the agent, so the page times the step itself.
fn timing_json(step: &Step) -> Value {
    serde_json::json!({
        "promptTokens": step.timing.prompt_tokens,
        "thinkTokens": step.timing.think_tokens,
        "answerTokens": step.timing.answer_tokens,
    })
}

fn outcome_to_json(outcome: StepOutcome) -> String {
    match outcome {
        StepOutcome::NeedTools { calls, step } => {
            let calls: Vec<Value> = calls.iter().map(|c: &PendingToolCall| serde_json::json!({"callId": c.call_id, "name": c.name, "arguments": c.arguments})).collect();
            serde_json::json!({"type": "needTools", "calls": calls, "timing": timing_json(&step)})
                .to_string()
        }
        StepOutcome::Final { text, step } => {
            serde_json::json!({"type": "final", "text": text, "timing": timing_json(&step)})
                .to_string()
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

/// The tools of an MCP `tools/list` JSON array, as `start` reads them.
fn tools_from_json(tools_json: &str) -> Result<Vec<Tool>, JsValue> {
    let raw: Vec<Value> = serde_json::from_str(tools_json)
        .map_err(|e| JsError::new(&format!("invalid tools JSON: {e}")))?;
    tools_from_mcp(&raw).map_err(|e| JsError::new(&format!("reading tools: {e:#}")).into())
}

fn set(err: &js_sys::Error, key: &str, value: JsValue) {
    let _ = js_sys::Reflect::set(err, &JsValue::from_str(key), &value);
}

/// A `LeanError` as the JS exception lean's own `Lean` calls throw:
/// `name` `"PrefixImage"` with `code` `"STALE"` (and `field`, what
/// differs), `"NOT_AN_IMAGE"` or `"TRUNCATED"`; `name` `"ContextOverflow"`
/// with `code` `"CONTEXT_OVERFLOW"`, `needed` and `available`; otherwise a
/// plain `Error`.
fn lean_error_to_js(e: LeanError) -> JsValue {
    let err = js_sys::Error::new(&e.to_string());
    match e {
        LeanError::ContextOverflow { needed, available } => {
            err.set_name("ContextOverflow");
            set(&err, "code", "CONTEXT_OVERFLOW".into());
            set(&err, "needed", (needed as f64).into());
            set(&err, "available", (available as f64).into());
        }
        LeanError::PrefixImage(kind) => {
            err.set_name("PrefixImage");
            let code = match kind {
                PrefixImageError::NotAnImage => "NOT_AN_IMAGE",
                PrefixImageError::Stale(field) => {
                    set(&err, "field", field.into());
                    "STALE"
                }
                PrefixImageError::Truncated => "TRUNCATED",
            };
            set(&err, "code", code.into());
        }
        LeanError::Other(_) => {}
    }
    err.into()
}

fn js_message(e: &JsValue) -> String {
    e.dyn_ref::<js_sys::Error>()
        .map(|e| String::from(e.message()))
        .or_else(|| e.as_string())
        .unwrap_or_else(|| format!("{e:?}"))
}

/// `Write` over the page's `onChunk(Uint8Array)`; a throw is a write error.
struct ChunkWriter(js_sys::Function);

impl Write for ChunkWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0
            .call1(&JsValue::NULL, &js_sys::Uint8Array::from(buf))
            .map_err(|e| std::io::Error::other(format!("onChunk threw: {}", js_message(&e))))?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// `Read` over a `Uint8Array` holding the whole image, copied into wasm
/// memory one read at a time.
struct BytesReader {
    data: js_sys::Uint8Array,
    pos: u32,
}

impl Read for BytesReader {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let n = (buf.len() as u32).min(self.data.length() - self.pos);
        self.data
            .subarray(self.pos, self.pos + n)
            .copy_to(&mut buf[..n as usize]);
        self.pos += n;
        Ok(n as usize)
    }
}

/// `Read` over the page's `read(n)`, which returns the next at most `n`
/// bytes as a `Uint8Array` (an empty one at the end).
struct FnReader(js_sys::Function);

impl Read for FnReader {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let got = self
            .0
            .call1(&JsValue::NULL, &JsValue::from_f64(buf.len() as f64))
            .map_err(|e| std::io::Error::other(format!("read threw: {}", js_message(&e))))?;
        let bytes: js_sys::Uint8Array = got
            .dyn_into()
            .map_err(|_| std::io::Error::other("read(n) must return a Uint8Array"))?;
        let n = bytes.length() as usize;
        if n > buf.len() {
            return Err(std::io::Error::other("read(n) returned more than n bytes"));
        }
        bytes.copy_to(&mut buf[..n]);
        Ok(n)
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

    /// With `off` true, asks a Qwen3-style model not to think (the prompt
    /// ends with the empty thinking block); false renders as the model's
    /// template does by default.
    #[wasm_bindgen(js_name = setThinkingOff)]
    pub fn set_thinking_off(&self, off: bool) -> Result<(), JsError> {
        let mut agent = self.take()?;
        agent.set_thinking(if off {
            Thinking::Off
        } else {
            Thinking::TemplateDefault
        });
        self.put(agent);
        Ok(())
    }

    /// Caps a thinking block at `cap` tokens, 0 for no cap.
    #[wasm_bindgen(js_name = setThinkCap)]
    pub fn set_think_cap(&self, cap: u32) -> Result<(), JsError> {
        let mut agent = self.take()?;
        agent.set_think_cap((cap > 0).then_some(cap as usize));
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

    /// Prefills the system prompt and the tools (`tools_json` as for
    /// `start`) without generating: the next `start` with the same tools
    /// processes only the user's words. Resolves to the prefix's token
    /// count.
    pub async fn prefill(&self, tools_json: String) -> Result<f64, JsValue> {
        let tools = tools_from_json(&tools_json)?;
        let mut agent = self.take()?;
        let result = agent.prefill(&tools).await;
        self.put(agent);
        result.map(|n| n as f64).map_err(lean_error_to_js)
    }

    /// The key to store the prefix image for `tools_json` under (32 hex
    /// characters). It changes whenever the image would no longer fit:
    /// another system prompt, tools list, chat template, model file,
    /// engine build or backend.
    #[wasm_bindgen(js_name = prefixKey)]
    pub fn prefix_key(&self, tools_json: String) -> Result<String, JsValue> {
        let tools = tools_from_json(&tools_json)?;
        let agent = self.take()?;
        let result = agent.prefix_key(&tools);
        self.put(agent);
        result.map_err(lean_error_to_js)
    }

    /// Writes the prefix image for `tools_json` through `onChunk(bytes)`,
    /// called once for the 128-byte header and then once per K or V tensor
    /// of each layer, in order; write each chunk at the running offset (an
    /// OPFS `FileSystemSyncAccessHandle.write(bytes, {at})`). Each chunk is
    /// a fresh `Uint8Array` the callback may keep. Prefills first if the
    /// prefix is not resident. Resolves to the total byte count. A throw
    /// from `onChunk` rejects the call; the agent stays usable.
    #[wasm_bindgen(js_name = savePrefix)]
    pub async fn save_prefix(
        &self,
        tools_json: String,
        on_chunk: js_sys::Function,
    ) -> Result<f64, JsValue> {
        let tools = tools_from_json(&tools_json)?;
        let mut agent = self.take()?;
        let result = agent.save_prefix(&tools, &mut ChunkWriter(on_chunk)).await;
        self.put(agent);
        result.map(|n| n as f64).map_err(lean_error_to_js)
    }

    /// Restores an image written by `savePrefix` for the same tools, so the
    /// next `start` with them processes only the user's words. `source` is
    /// the whole image as a `Uint8Array`, or a synchronous `read(n)`
    /// returning the next at most `n` bytes as a `Uint8Array` (in a worker,
    /// an OPFS sync access handle). Throws an `Error` named `"PrefixImage"`
    /// (`code` `"STALE"` with `field`, `"NOT_AN_IMAGE"` or `"TRUNCATED"`)
    /// when the image cannot be used: the agent stays usable; call
    /// `savePrefix` and overwrite the image.
    #[wasm_bindgen(js_name = restorePrefix)]
    pub fn restore_prefix(&self, tools_json: String, source: JsValue) -> Result<(), JsValue> {
        let tools = tools_from_json(&tools_json)?;
        let mut agent = self.take()?;
        let result = if let Some(data) = source.dyn_ref::<js_sys::Uint8Array>() {
            agent.restore_prefix(
                &tools,
                &mut BytesReader {
                    data: data.clone(),
                    pos: 0,
                },
            )
        } else if let Some(f) = source.dyn_ref::<js_sys::Function>() {
            agent.restore_prefix(&tools, &mut FnReader(f.clone()))
        } else {
            self.put(agent);
            return Err(JsError::new(
                "restorePrefix: source must be a Uint8Array or a read(n) function",
            )
            .into());
        };
        self.put(agent);
        result.map_err(lean_error_to_js)
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

#[cfg(test)]
mod tests {
    use super::*;
    use agent::agent::{ParsedOutput, StepTiming};

    fn step(timing: StepTiming) -> Step {
        Step {
            prompt: String::new(),
            generated_text: String::new(),
            parsed: ParsedOutput::Text(String::new()),
            retries: 0,
            tool_errors: 0,
            oversized_results: 0,
            repeat_guard: false,
            forced_text_answer: false,
            timing,
        }
    }

    #[test]
    fn need_tools_and_final_carry_token_counts() {
        let timing = StepTiming {
            prompt_tokens: 812,
            think_tokens: 40,
            answer_tokens: 17,
            ..Default::default()
        };
        let expected =
            serde_json::json!({"promptTokens": 812, "thinkTokens": 40, "answerTokens": 17});

        let v: Value = serde_json::from_str(&outcome_to_json(StepOutcome::NeedTools {
            calls: vec![],
            step: step(timing),
        }))
        .unwrap();
        assert_eq!(v["type"], "needTools");
        assert_eq!(v["timing"], expected);

        let v: Value = serde_json::from_str(&outcome_to_json(StepOutcome::Final {
            text: "ok".into(),
            step: step(timing),
        }))
        .unwrap();
        assert_eq!(v["type"], "final");
        assert_eq!(v["timing"], expected);
    }
}
