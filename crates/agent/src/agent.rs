//! Tool-calling agent loop for Qwen2.5-Instruct's own tools format:
//! `<tools>...</tools>` in the system turn (rendered by the model's chat
//! template), `<tool_call>{"name":..., "arguments":...}</tool_call>` in the
//! model's output.
//!
//! Prompts are rendered with lean-chat (the model's own chat template, byte
//! for byte what transformers' `apply_chat_template` renders). The loop has
//! no dependency on any inference engine: it is driven by the
//! [`Generator`] trait, prompt text in and reply text out.

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::io;

pub use crate::generator::{AbortFlag, GenerateError, Generator};
pub use lean_chat::{ChatTemplate, Message};

// ---------------------------------------------------------------------
// Tool definitions
// ---------------------------------------------------------------------

/// One entry of the `tools` list, transformers/OpenAI shape:
/// `{"type": "function", "function": {"name", "description", "parameters"}}`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Tool {
    #[serde(rename = "type", default = "default_function_kind")]
    pub kind: String,
    pub function: ToolFunction,
}

fn default_function_kind() -> String {
    "function".to_string()
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolFunction {
    pub name: String,
    pub description: String,
    pub parameters: Value,
}

impl Tool {
    /// Build a `Tool` from an MCP `tools/list` entry
    /// (`{"name", "description", "inputSchema"}`).
    pub fn from_mcp(
        name: impl Into<String>,
        description: impl Into<String>,
        input_schema: Value,
    ) -> Self {
        Tool {
            kind: "function".to_string(),
            function: ToolFunction {
                name: name.into(),
                description: description.into(),
                parameters: input_schema,
            },
        }
    }
}

// ---------------------------------------------------------------------
// Python `json.dumps` formatting for tool results (the same separators
// the chat template's `tojson` uses for tool calls), so a tool result
// reads like the rest of the prompt.
// ---------------------------------------------------------------------

#[derive(Default)]
struct PySeparatorsFormatter;

impl serde_json::ser::Formatter for PySeparatorsFormatter {
    fn begin_array_value<W: ?Sized + io::Write>(
        &mut self,
        writer: &mut W,
        first: bool,
    ) -> io::Result<()> {
        writer.write_all(if first { b"" } else { b", " })
    }

    fn begin_object_key<W: ?Sized + io::Write>(
        &mut self,
        writer: &mut W,
        first: bool,
    ) -> io::Result<()> {
        writer.write_all(if first { b"" } else { b", " })
    }

    fn begin_object_value<W: ?Sized + io::Write>(&mut self, writer: &mut W) -> io::Result<()> {
        writer.write_all(b": ")
    }
}

/// Serialize `value` the way Python's `json.dumps(value, ensure_ascii=False)`
/// would with no `indent`.
pub(crate) fn to_json_compact_py<T: ?Sized + Serialize>(value: &T) -> Result<String> {
    let mut out = Vec::new();
    let mut ser = serde_json::Serializer::with_formatter(&mut out, PySeparatorsFormatter);
    value.serialize(&mut ser)?;
    Ok(String::from_utf8(out).expect("serde_json only emits valid UTF-8"))
}

// ---------------------------------------------------------------------
// Prompt rendering
// ---------------------------------------------------------------------

/// The prompt for the next model turn: `messages` and `tools` through the
/// model's chat template (lean-chat, byte-exact with transformers'
/// `apply_chat_template(messages, tools=tools, add_generation_prompt=True)`).
/// No tools renders like `tools=None`, which skips the tools preamble.
pub fn render_prompt(
    template: &ChatTemplate,
    messages: &[Message],
    tools: &[Tool],
) -> Result<String> {
    let tools: Vec<Value> = tools
        .iter()
        .map(serde_json::to_value)
        .collect::<std::result::Result<_, _>>()?;
    let tools = if tools.is_empty() {
        None
    } else {
        Some(tools.as_slice())
    };
    lean_chat::render(template, messages, tools, true)
}

// ---------------------------------------------------------------------
// Tool-call output parsing (Qwen2.5 format: one or more
// `<tool_call>\n{"name": ..., "arguments": ...}\n</tool_call>` blocks).
// ---------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolCall {
    pub name: String,
    pub arguments: Value,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ParsedOutput {
    ToolCalls(Vec<ToolCall>),
    Text(String),
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
struct RawToolCall {
    name: String,
    arguments: Value,
}

/// Parse one turn of raw (decoded) model output.
///
/// - A trailing `<|im_end|>` (and whitespace before it) is stripped first.
/// - Every `<tool_call>...</tool_call>` block found is parsed as one call;
///   a block whose contents don't parse as `{"name", "arguments"}` is an
///   error, not a panic.
/// - With no `<tool_call>` block at all, the (trimmed) text is the plain
///   answer.
pub fn parse_output(raw: &str) -> Result<ParsedOutput, String> {
    let trimmed = raw.trim();
    let without_end = trimmed.strip_suffix("<|im_end|>").unwrap_or(trimmed).trim();

    const OPEN: &str = "<tool_call>";
    const CLOSE: &str = "</tool_call>";

    if !without_end.contains(OPEN) {
        return Ok(ParsedOutput::Text(without_end.to_string()));
    }

    let mut calls = Vec::new();
    let mut rest = without_end;
    while let Some(start) = rest.find(OPEN) {
        let after_open = &rest[start + OPEN.len()..];
        let end = after_open
            .find(CLOSE)
            .ok_or_else(|| format!("unterminated {OPEN} block in model output: {without_end:?}"))?;
        let body = after_open[..end].trim();
        let raw_call: RawToolCall = serde_json::from_str(body)
            .map_err(|e| format!("malformed tool_call JSON {body:?}: {e}"))?;
        calls.push(ToolCall {
            name: raw_call.name,
            arguments: raw_call.arguments,
        });
        rest = &after_open[end + CLOSE.len()..];
    }
    Ok(ParsedOutput::ToolCalls(calls))
}

/// Above this many characters, a compacted tool result is kept in full
/// (never truncated) but is worth flagging; see `format_tool_result`.
pub const MAX_RESULT_CHARS: usize = 8000;

/// If `value` is a string that itself parses as JSON, replace it with a
/// compact re-serialization of that parsed JSON; anything else is left
/// verbatim. Recurses into arrays/objects. Fixes MCP servers that return
/// pretty-printed JSON text embedded as a string.
fn compact_embedded_json(value: &Value) -> Value {
    match value {
        Value::String(s) => match serde_json::from_str::<Value>(s) {
            Ok(parsed) => {
                let compacted = compact_embedded_json(&parsed);
                Value::String(to_json_compact_py(&compacted).unwrap_or_else(|_| s.clone()))
            }
            Err(_) => value.clone(),
        },
        Value::Array(items) => Value::Array(items.iter().map(compact_embedded_json).collect()),
        Value::Object(map) => Value::Object(
            map.iter()
                .map(|(k, v)| (k.clone(), compact_embedded_json(v)))
                .collect(),
        ),
        other => other.clone(),
    }
}

/// Build the `tool`-role `Message` carrying a tool's result: `content` is
/// a JSON-encoded string, Python-`json.dumps`-formatted, after first
/// compacting any pretty-printed JSON embedded in `result`.
pub fn format_tool_result(name: &str, result: &Value) -> Message {
    let compacted = compact_embedded_json(result);
    let content = to_json_compact_py(&compacted).unwrap_or_else(|_| result.to_string());
    if content.len() > MAX_RESULT_CHARS {
        eprintln!("warning: tool result for {name} is {} chars, over MAX_RESULT_CHARS ({MAX_RESULT_CHARS}); keeping it uncut", content.len());
    }
    tool_message(name, content)
}

/// A `tool`-role message carrying one call's result (Qwen2.5's template
/// wraps its content in `<tool_response>...</tool_response>`).
fn tool_message(name: &str, content: String) -> Message {
    Message {
        name: Some(name.to_string()),
        ..Message::tool(content)
    }
}

/// If `result` is an MCP-shaped error (`isError: true` or a top-level
/// `error` field), extract a human-readable message; `None` otherwise.
pub fn tool_error_message(result: &Value) -> Option<String> {
    if let Some(error) = result.get("error") {
        let message = match error {
            Value::String(s) => s.clone(),
            Value::Object(o) => o
                .get("message")
                .and_then(Value::as_str)
                .map(str::to_string)
                .unwrap_or_else(|| error.to_string()),
            other => other.to_string(),
        };
        return Some(message);
    }
    if result.get("isError").and_then(Value::as_bool) == Some(true) {
        let message = result
            .get("content")
            .and_then(Value::as_array)
            .and_then(|items| items.first())
            .and_then(|item| item.get("text"))
            .and_then(Value::as_str)
            .map(str::to_string)
            .unwrap_or_else(|| "tool call failed".to_string());
        return Some(message);
    }
    None
}

/// The `tool`-role `Message` fed back for an error result; `content` is
/// the compact JSON string `{"error": "<message>"}`.
pub fn format_tool_error(name: &str, message: &str) -> Message {
    let content = to_json_compact_py(&serde_json::json!({ "error": message }))
        .unwrap_or_else(|_| format!("{{\"error\": {message:?}}}"));
    tool_message(name, content)
}

// ---------------------------------------------------------------------
// Agent loop
// ---------------------------------------------------------------------

const DEFAULT_MAX_STEPS: usize = 8;
const DEFAULT_MAX_RETRIES: usize = 2;
const MAX_CONSECUTIVE_TOOL_ERRORS: usize = 3;

/// A generic, non-committal nudge appended as a `user` message when a
/// model output is malformed/empty/unknown-tool twice in a row.
const RETRY_NOTE: &str = "Respond with a tool call from the list or a final answer.";

/// Executes one tool call and returns its JSON result.
pub trait ToolCaller {
    fn call(&mut self, name: &str, args: &Value) -> Result<Value>;
}

/// A `Generator` that replays a scripted sequence of replies for tests.
pub struct FixtureGenerator {
    script: Vec<String>,
    next: usize,
}

impl FixtureGenerator {
    pub fn new(script: Vec<String>) -> Self {
        Self { script, next: 0 }
    }
}

impl Generator for FixtureGenerator {
    fn generate(
        &mut self,
        _prompt: &str,
        _max_new_tokens: usize,
        on_text: &mut dyn FnMut(&str),
        abort: &AbortFlag,
    ) -> impl std::future::Future<Output = std::result::Result<String, GenerateError>> {
        let reply = if abort.is_aborted() {
            Err(GenerateError::Aborted)
        } else {
            match self.script.get(self.next) {
                Some(text) => {
                    self.next += 1;
                    on_text(text);
                    Ok(text.clone())
                }
                None => Err(GenerateError::Other(anyhow::anyhow!(
                    "FixtureGenerator script exhausted after {} call(s)",
                    self.next
                ))),
            }
        };
        std::future::ready(reply)
    }
}

/// A `ToolCaller` that serves canned JSON results from
/// `<results_dir>/<tool>.json` and records every call it saw, in order.
pub struct FixtureCaller {
    results_dir: std::path::PathBuf,
    pub calls_seen: Vec<(String, Value)>,
}

impl FixtureCaller {
    pub fn new(results_dir: impl Into<std::path::PathBuf>) -> Self {
        Self {
            results_dir: results_dir.into(),
            calls_seen: Vec::new(),
        }
    }
}

impl ToolCaller for FixtureCaller {
    fn call(&mut self, name: &str, args: &Value) -> Result<Value> {
        self.calls_seen.push((name.to_string(), args.clone()));
        let path = self.results_dir.join(format!("{name}.json"));
        let text = std::fs::read_to_string(&path)
            .with_context(|| format!("reading fixture result {path:?}"))?;
        Ok(serde_json::from_str(&text)?)
    }
}

/// One tool call the model asked for, awaiting a result via
/// [`Agent::provide_tool_results`].
#[derive(Debug, Clone, PartialEq)]
pub struct PendingToolCall {
    pub call_id: String,
    pub name: String,
    pub arguments: Value,
}

/// One render -> generate -> parse round of the agent loop.
#[derive(Debug)]
pub struct Step {
    pub prompt: String,
    pub generated_text: String,
    pub parsed: ParsedOutput,
    pub retries: usize,
    pub tool_errors: usize,
    pub repeat_guard: bool,
    pub forced_text_answer: bool,
}

pub struct Transcript {
    pub steps: Vec<Step>,
    pub final_text: String,
}

#[derive(Debug)]
pub enum StepOutcome {
    NeedTools {
        calls: Vec<PendingToolCall>,
        step: Step,
    },
    Final {
        text: String,
        step: Step,
    },
    /// The prompt no longer fits the engine's context (`needed` tokens,
    /// `available` in the context). What to drop is the caller's choice,
    /// e.g. fewer tools or a shorter system prompt.
    ContextOverflow {
        needed: usize,
        available: usize,
    },
    /// The turn was stopped through [`Agent::abort_flag`].
    Aborted,
    Error {
        message: String,
        step: Option<Step>,
    },
}

struct AttemptOutput {
    prompt: String,
    generated_text: String,
    parsed: std::result::Result<ParsedOutput, String>,
}

pub struct Agent<G: Generator, C: ToolCaller> {
    template: ChatTemplate,
    generator: G,
    caller: C,
    system_prompt: String,
    max_new_tokens: usize,
    max_steps: usize,
    max_retries: usize,
    abort: AbortFlag,
    on_text: Box<dyn FnMut(&str)>,

    messages: Vec<Message>,
    tools: Vec<Tool>,
    step_index: usize,
    pending_calls: Vec<PendingToolCall>,
    consecutive_tool_errors: usize,
    calls_made_this_turn: Vec<ToolCall>,
}

impl<G: Generator, C: ToolCaller> Agent<G, C> {
    pub fn new(
        template: ChatTemplate,
        generator: G,
        caller: C,
        system_prompt: impl Into<String>,
        max_new_tokens: usize,
    ) -> Self {
        Self {
            template,
            generator,
            caller,
            system_prompt: system_prompt.into(),
            max_new_tokens,
            max_steps: DEFAULT_MAX_STEPS,
            max_retries: DEFAULT_MAX_RETRIES,
            abort: AbortFlag::new(),
            on_text: Box::new(|_| {}),
            messages: Vec::new(),
            tools: Vec::new(),
            step_index: 0,
            pending_calls: Vec::new(),
            consecutive_tool_errors: 0,
            calls_made_this_turn: Vec::new(),
        }
    }

    pub fn caller(&self) -> &C {
        &self.caller
    }

    pub fn generator(&self) -> &G {
        &self.generator
    }

    pub fn set_max_steps(&mut self, max_steps: usize) {
        self.max_steps = max_steps;
    }

    pub fn set_max_retries(&mut self, max_retries: usize) {
        self.max_retries = max_retries;
    }

    /// Receives the model's raw output as it is generated, every step
    /// (tool calls included, before they are parsed).
    pub fn set_on_text(&mut self, on_text: impl FnMut(&str) + 'static) {
        self.on_text = Box::new(on_text);
    }

    /// A handle that stops the current turn: after `abort()`, the running
    /// step returns [`StepOutcome::Aborted`]. [`Agent::start`] clears it.
    pub fn abort_flag(&self) -> AbortFlag {
        self.abort.clone()
    }

    pub fn reset(&mut self) {
        self.messages.clear();
        self.tools.clear();
        self.step_index = 0;
        self.pending_calls.clear();
        self.consecutive_tool_errors = 0;
        self.calls_made_this_turn.clear();
    }

    pub async fn start(&mut self, utterance: &str, tools: &[Tool]) -> StepOutcome {
        self.abort.reset();
        self.tools = tools.to_vec();
        self.messages = vec![
            Message::system(&self.system_prompt),
            Message::user(utterance),
        ];
        self.step_index = 0;
        self.pending_calls.clear();
        self.consecutive_tool_errors = 0;
        self.calls_made_this_turn.clear();

        self.step_inner().await
    }

    pub async fn start_from_mcp(&mut self, utterance: &str, raw_tools: &[Value]) -> StepOutcome {
        let tools: Vec<Tool> = raw_tools
            .iter()
            .filter_map(|t| {
                Some(Tool::from_mcp(
                    t.get("name")?.as_str()?,
                    t.get("description")?.as_str()?,
                    t.get("inputSchema")?.clone(),
                ))
            })
            .collect();
        self.start(utterance, &tools).await
    }

    /// Continue the current turn with the results of the tool calls from
    /// the most recent `NeedTools` outcome, keyed by `call_id`.
    pub async fn provide_tool_results(&mut self, results: Vec<(String, Value)>) -> StepOutcome {
        for (call_id, result) in results {
            if let Some(pending) = self.pending_calls.iter().find(|c| c.call_id == call_id) {
                if let Some(error_message) = tool_error_message(&result) {
                    self.consecutive_tool_errors += 1;
                    if self.consecutive_tool_errors > MAX_CONSECUTIVE_TOOL_ERRORS {
                        self.pending_calls.clear();
                        return StepOutcome::Error {
                            message: format!(
                                "{} consecutive tool errors (giving up after {}): {error_message}",
                                self.consecutive_tool_errors, MAX_CONSECUTIVE_TOOL_ERRORS
                            ),
                            step: None,
                        };
                    }
                    self.messages
                        .push(format_tool_error(&pending.name, &error_message));
                } else {
                    self.consecutive_tool_errors = 0;
                    self.messages
                        .push(format_tool_result(&pending.name, &result));
                }
            }
        }
        self.pending_calls.clear();
        self.step_inner().await
    }

    /// Renders the conversation, generates and parses one reply. `Err` is
    /// the outcome to return as is (render or engine failure, overflow,
    /// abort).
    async fn generate_attempt(&mut self) -> std::result::Result<AttemptOutput, Box<StepOutcome>> {
        let prompt = render_prompt(&self.template, &self.messages, &self.tools).map_err(|e| {
            StepOutcome::Error {
                message: format!("rendering prompt: {e:#}"),
                step: None,
            }
        })?;
        let generated_text = self
            .generator
            .generate(
                &prompt,
                self.max_new_tokens,
                &mut *self.on_text,
                &self.abort,
            )
            .await
            .map_err(|e| {
                Box::new(match e {
                    GenerateError::ContextOverflow { needed, available } => {
                        StepOutcome::ContextOverflow { needed, available }
                    }
                    GenerateError::Aborted => StepOutcome::Aborted,
                    GenerateError::Other(e) => StepOutcome::Error {
                        message: format!("generation failed: {e:#}"),
                        step: None,
                    },
                })
            })?;
        let parsed = parse_output(&generated_text);
        Ok(AttemptOutput {
            prompt,
            generated_text,
            parsed,
        })
    }

    /// One render -> generate -> parse round using `self.messages`/`self.tools`.
    ///
    /// Malformed-output retry policy: an empty tool-call array, unparsable
    /// output, or a call naming a tool outside `self.tools` does not get
    /// appended to history; the agent regenerates, nudging with
    /// [`RETRY_NOTE`] once `self.max_retries` is exhausted. A call
    /// identical (same name+args) to one already made earlier this turn is
    /// never re-executed (the repeated-call loop guard).
    async fn step_inner(&mut self) -> StepOutcome {
        if self.step_index >= self.max_steps {
            return StepOutcome::Error {
                message: format!(
                    "agent exceeded max_steps ({}) without a final text response",
                    self.max_steps
                ),
                step: None,
            };
        }
        self.step_index += 1;

        let mut retries = 0usize;
        let mut repeat_guard = false;
        loop {
            let attempt = match self.generate_attempt().await {
                Ok(a) => a,
                Err(outcome) => return *outcome,
            };

            let empty_calls =
                matches!(&attempt.parsed, Ok(ParsedOutput::ToolCalls(calls)) if calls.is_empty());
            let invalid = match &attempt.parsed {
                Err(_) => true,
                Ok(ParsedOutput::ToolCalls(calls)) => {
                    calls.is_empty()
                        || calls
                            .iter()
                            .any(|c| !self.tools.iter().any(|t| t.function.name == c.name))
                }
                Ok(ParsedOutput::Text(_)) => false,
            };

            let is_repeat = !invalid
                && matches!(&attempt.parsed, Ok(ParsedOutput::ToolCalls(calls)) if calls.iter().any(|c| self.calls_made_this_turn.contains(c)));
            if is_repeat {
                repeat_guard = true;
            }

            if invalid || is_repeat {
                if retries < self.max_retries {
                    retries += 1;
                    self.messages.push(Message::user(RETRY_NOTE));
                    continue;
                }
                if is_repeat || empty_calls {
                    return self.force_final_answer(repeat_guard).await;
                }
                return StepOutcome::Error {
                    message: "model produced no valid call".to_string(),
                    step: None,
                };
            }

            let AttemptOutput {
                prompt,
                generated_text,
                parsed,
            } = attempt;
            let parsed = parsed.expect("checked valid above");
            let tool_errors = self.consecutive_tool_errors;

            return match parsed {
                ParsedOutput::ToolCalls(calls) => {
                    let pending = pending_calls_from(&calls);
                    self.messages
                        .push(Message::assistant_tool_calls("", chat_tool_calls(&pending)));
                    self.pending_calls = pending.clone();
                    self.calls_made_this_turn.extend(calls.iter().cloned());

                    let step = Step {
                        prompt,
                        generated_text,
                        parsed: ParsedOutput::ToolCalls(calls),
                        retries,
                        tool_errors,
                        repeat_guard,
                        forced_text_answer: false,
                    };
                    StepOutcome::NeedTools {
                        calls: pending,
                        step,
                    }
                }
                ParsedOutput::Text(text) => {
                    self.messages.push(Message::assistant(&text));
                    let step = Step {
                        prompt,
                        generated_text,
                        parsed: ParsedOutput::Text(text.clone()),
                        retries,
                        tool_errors,
                        repeat_guard,
                        forced_text_answer: false,
                    };
                    StepOutcome::Final { text, step }
                }
            };
        }
    }

    /// Last resort when the retry budget is exhausted on a repeated or
    /// empty tool call: nudge once more and accept whatever text comes
    /// back (falling back to the raw output if the model still emits a
    /// tool call, rather than looping forever).
    async fn force_final_answer(&mut self, repeat_guard: bool) -> StepOutcome {
        self.messages.push(Message::user(
            "Give your final answer as plain text now, with no tool call.",
        ));
        let attempt = match self.generate_attempt().await {
            Ok(a) => a,
            Err(outcome) => return *outcome,
        };
        let AttemptOutput {
            prompt,
            generated_text,
            parsed,
        } = attempt;
        let text = match parsed {
            Ok(ParsedOutput::Text(t)) => t,
            _ => generated_text.clone(),
        };
        self.messages.push(Message::assistant(&text));
        let step = Step {
            prompt,
            generated_text,
            parsed: ParsedOutput::Text(text.clone()),
            retries: self.max_retries,
            tool_errors: self.consecutive_tool_errors,
            repeat_guard,
            forced_text_answer: true,
        };
        StepOutcome::Final { text, step }
    }

    /// Convenience for callers with a synchronous `ToolCaller` (tests,
    /// eval, a CLI): runs the whole turn to completion.
    pub async fn run(
        &mut self,
        utterance: &str,
        tools: &[Tool],
        max_steps: usize,
    ) -> Result<Transcript> {
        self.max_steps = max_steps;
        let mut steps = Vec::with_capacity(max_steps);
        let mut outcome = self.start(utterance, tools).await;

        loop {
            match outcome {
                StepOutcome::Final { text, step } => {
                    steps.push(step);
                    return Ok(Transcript {
                        steps,
                        final_text: text,
                    });
                }
                StepOutcome::NeedTools { calls, step } => {
                    steps.push(step);
                    let mut results = Vec::with_capacity(calls.len());
                    for call in &calls {
                        let result = self.caller.call(&call.name, &call.arguments)?;
                        results.push((call.call_id.clone(), result));
                    }
                    outcome = self.provide_tool_results(results).await;
                }
                StepOutcome::ContextOverflow { needed, available } => {
                    bail!("context overflow: needed {needed} tokens, {available} available");
                }
                StepOutcome::Aborted => bail!("aborted"),
                StepOutcome::Error { message, step } => {
                    if let Some(step) = step {
                        steps.push(step);
                    }
                    bail!("{message}");
                }
            }
        }
    }
}

fn pending_calls_from(calls: &[ToolCall]) -> Vec<PendingToolCall> {
    calls
        .iter()
        .enumerate()
        .map(|(i, c)| PendingToolCall {
            call_id: format!("call_{i}"),
            name: c.name.clone(),
            arguments: c.arguments.clone(),
        })
        .collect()
}

fn chat_tool_calls(pending: &[PendingToolCall]) -> Vec<lean_chat::ToolCall> {
    pending
        .iter()
        .map(|c| lean_chat::ToolCall::new(c.name.clone(), c.arguments.clone()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Qwen2.5-Instruct's tools-branch template, copied from the model's
    /// own `tokenizer_config.json`.
    const QWEN25_TOOLS_TEMPLATE: &str =
        include_str!("../tests/fixtures/qwen25_chat_template.jinja");

    fn weather_tool() -> Tool {
        Tool::from_mcp(
            "get_weather",
            "Get the current weather for a city.",
            serde_json::json!({"type": "object", "properties": {"city": {"type": "string"}}, "required": ["city"]}),
        )
    }

    #[test]
    fn renders_tools_preamble_and_single_user_turn() {
        let tmpl = ChatTemplate::new(QWEN25_TOOLS_TEMPLATE);
        let messages = vec![Message::user("What's the weather in Paris?")];
        let out = render_prompt(&tmpl, &messages, &[weather_tool()]).unwrap();
        assert!(out.starts_with("<|im_start|>system\n"));
        assert!(out.contains(
            "<tools>\n{\"type\": \"function\", \"function\": {\"name\": \"get_weather\""
        ));
        assert!(out.contains("\"required\": [\"city\"]}}}"));
        assert!(out.ends_with("<|im_start|>assistant\n"));
    }

    #[test]
    fn renders_multiturn_with_tool_call_and_result() {
        let tmpl = ChatTemplate::new(QWEN25_TOOLS_TEMPLATE);
        let messages = vec![
            Message::user("What's the weather in Paris?"),
            Message::assistant_tool_calls(
                "",
                vec![lean_chat::ToolCall::new(
                    "get_weather",
                    serde_json::json!({"city": "Paris"}),
                )],
            ),
            format_tool_result(
                "get_weather",
                &serde_json::json!({"temp_c": 14, "condition": "cloudy"}),
            ),
        ];
        let out = render_prompt(&tmpl, &messages, &[weather_tool()]).unwrap();
        assert!(out.contains("<tool_call>\n{\"name\": \"get_weather\", \"arguments\": {\"city\": \"Paris\"}}\n</tool_call>"));
        assert!(out.contains(
            "<tool_response>\n{\"temp_c\": 14, \"condition\": \"cloudy\"}\n</tool_response>"
        ));
    }

    #[test]
    fn parses_single_tool_call() {
        let out = parse_output("<tool_call>\n{\"name\": \"get_weather\", \"arguments\": {\"city\": \"Paris\"}}\n</tool_call><|im_end|>").unwrap();
        assert_eq!(
            out,
            ParsedOutput::ToolCalls(vec![ToolCall {
                name: "get_weather".into(),
                arguments: serde_json::json!({"city": "Paris"})
            }])
        );
    }

    #[test]
    fn parses_multiple_tool_calls() {
        let raw = "<tool_call>\n{\"name\": \"a\", \"arguments\": {}}\n</tool_call>\n<tool_call>\n{\"name\": \"b\", \"arguments\": {}}\n</tool_call>";
        let out = parse_output(raw).unwrap();
        assert_eq!(
            out,
            ParsedOutput::ToolCalls(vec![
                ToolCall {
                    name: "a".into(),
                    arguments: serde_json::json!({})
                },
                ToolCall {
                    name: "b".into(),
                    arguments: serde_json::json!({})
                }
            ])
        );
    }

    #[test]
    fn parses_plain_text() {
        let out = parse_output("The capital of France is Paris.<|im_end|>").unwrap();
        assert_eq!(
            out,
            ParsedOutput::Text("The capital of France is Paris.".to_string())
        );
    }

    #[test]
    fn parse_error_on_malformed_tool_call() {
        assert!(parse_output("<tool_call>\nnot json\n</tool_call>").is_err());
    }

    #[test]
    fn format_tool_result_compacts_embedded_json_string() {
        let msg = format_tool_result(
            "get_weather",
            &serde_json::json!({"body": "{\n  \"temp_c\": 14\n}"}),
        );
        assert_eq!(msg.content, "{\"body\": \"{\\\"temp_c\\\": 14}\"}");
    }

    #[test]
    fn tool_error_message_from_mcp_shape() {
        let result = serde_json::json!({"isError": true, "content": [{"type": "text", "text": "bad group_id"}]});
        assert_eq!(
            tool_error_message(&result),
            Some("bad group_id".to_string())
        );
    }

    struct EchoCaller;
    impl ToolCaller for EchoCaller {
        fn call(&mut self, name: &str, args: &Value) -> Result<Value> {
            Ok(serde_json::json!({"called": name, "with": args}))
        }
    }

    /// End-to-end step-wise loop against a scripted `FixtureGenerator`: the
    /// model calls a tool, gets a result, then answers in text.
    #[test]
    fn agent_loop_calls_tool_then_answers() {
        let tool_call_text = "<tool_call>\n{\"name\": \"get_weather\", \"arguments\": {\"city\": \"Paris\"}}\n</tool_call>";
        let answer_text = "It is cloudy.";
        let generator =
            FixtureGenerator::new(vec![tool_call_text.to_string(), answer_text.to_string()]);
        let template = ChatTemplate::new(QWEN25_TOOLS_TEMPLATE);
        let mut agent = Agent::new(
            template,
            generator,
            EchoCaller,
            "You are a helpful assistant.",
            64,
        );

        let transcript =
            pollster::block_on(agent.run("What's the weather in Paris?", &[weather_tool()], 4))
                .unwrap();
        assert_eq!(transcript.steps.len(), 2);
        assert!(
            matches!(&transcript.steps[0].parsed, ParsedOutput::ToolCalls(calls) if calls.len() == 1 && calls[0].name == "get_weather")
        );
        assert!(matches!(&transcript.steps[1].parsed, ParsedOutput::Text(_)));
        assert!(transcript.steps[1].prompt.contains(
            "<tool_response>\n{\"called\": \"get_weather\", \"with\": {\"city\": \"Paris\"}}\n</tool_response>"
        ));
        assert_eq!(transcript.final_text, answer_text);
    }

    #[test]
    fn streams_text_and_abort_stops_the_turn() {
        let generator = FixtureGenerator::new(vec!["Hello.".to_string()]);
        let mut agent = Agent::new(
            ChatTemplate::new(QWEN25_TOOLS_TEMPLATE),
            generator,
            EchoCaller,
            "You are a helpful assistant.",
            64,
        );
        let streamed = std::rc::Rc::new(std::cell::RefCell::new(String::new()));
        let sink = streamed.clone();
        agent.set_on_text(move |t| sink.borrow_mut().push_str(t));
        let first = pollster::block_on(agent.start("hi", &[weather_tool()]));
        assert!(matches!(first, StepOutcome::Final { .. }), "{first:?}");
        assert_eq!(*streamed.borrow(), "Hello.");

        agent.abort_flag().abort();
        let outcome = pollster::block_on(agent.provide_tool_results(Vec::new()));
        assert!(matches!(outcome, StepOutcome::Aborted), "{outcome:?}");
    }
}
