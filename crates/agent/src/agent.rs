//! Tool-calling chat template, output parser and step-wise agent loop for
//! Qwen2.5-Instruct's own tools format: `<tools>...</tools>` in the system
//! turn, `<tool_call>{"name":..., "arguments":...}</tool_call>` in the
//! model's output.
//!
//! This module has no dependency on any particular inference engine: the
//! loop is driven by the [`Generator`] trait, implemented against lean's
//! public CPU and GPU APIs in `crate::backend`.

use anyhow::{bail, Context, Result};
use minijinja::value::{Kwargs, Value as MjValue};
use minijinja::{context, Environment};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::io;

// ---------------------------------------------------------------------
// Message / tool-call wire types
// ---------------------------------------------------------------------

/// A single chat message, matching the fields Qwen2.5-Instruct's tools
/// template inspects (`role`, `content`, `tool_calls`, `name`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCallEntry>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

impl Message {
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: "system".into(),
            content: Some(Value::String(content.into())),
            tool_calls: None,
            name: None,
        }
    }

    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: "user".into(),
            content: Some(Value::String(content.into())),
            tool_calls: None,
            name: None,
        }
    }

    pub fn assistant_text(content: impl Into<String>) -> Self {
        Self {
            role: "assistant".into(),
            content: Some(Value::String(content.into())),
            tool_calls: None,
            name: None,
        }
    }

    pub fn assistant_tool_calls(tool_calls: Vec<ToolCallEntry>) -> Self {
        Self {
            role: "assistant".into(),
            content: None,
            tool_calls: Some(tool_calls),
            name: None,
        }
    }

    /// A `tool`-role message carrying one call's result (Qwen2.5's
    /// template wraps every `tool`-role message's content in
    /// `<tool_response>...</tool_response>`).
    pub fn tool_result(name: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            role: "tool".into(),
            content: Some(Value::String(content.into())),
            tool_calls: None,
            name: Some(name.into()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolCallEntry {
    #[serde(rename = "type", default = "default_function_kind")]
    pub kind: String,
    pub function: ToolCallFunction,
}

fn default_function_kind() -> String {
    "function".to_string()
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolCallFunction {
    pub name: String,
    pub arguments: Value,
}

/// One entry of the `tools` list, transformers/OpenAI shape:
/// `{"type": "function", "function": {"name", "description", "parameters"}}`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Tool {
    #[serde(rename = "type", default = "default_function_kind")]
    pub kind: String,
    pub function: ToolFunction,
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
// py_tojson: byte-exact `tojson` against Python's `json.dumps` (minijinja's
// built-in `tojson` HTML-escapes and uses different compact-mode separator
// spacing, which breaks parity against `transformers.apply_chat_template`).
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

fn json_error(err: impl std::fmt::Display) -> minijinja::Error {
    minijinja::Error::new(
        minijinja::ErrorKind::InvalidOperation,
        format!("cannot serialize to JSON: {err}"),
    )
}

fn py_tojson(value: MjValue, kwargs: Kwargs) -> std::result::Result<MjValue, minijinja::Error> {
    let indent: Option<usize> = kwargs.get("indent").ok();
    kwargs.assert_all_used()?;

    let rendered = match indent {
        Some(indent) => {
            let mut out = Vec::new();
            let indentation = " ".repeat(indent);
            let formatter = serde_json::ser::PrettyFormatter::with_indent(indentation.as_bytes());
            let mut ser = serde_json::Serializer::with_formatter(&mut out, formatter);
            serde::Serialize::serialize(&value, &mut ser).map_err(json_error)?;
            String::from_utf8(out).expect("serde_json only emits valid UTF-8")
        }
        None => to_json_compact_py(&value).map_err(json_error)?,
    };

    Ok(MjValue::from_safe_string(rendered))
}

// ---------------------------------------------------------------------
// Template: Qwen2.5-Instruct's own `chat_template`, tools-enabled.
// ---------------------------------------------------------------------

pub struct AgentChatTemplate {
    env: Environment<'static>,
}

impl AgentChatTemplate {
    pub fn new(template_source: impl Into<String>) -> Result<Self> {
        let mut env = Environment::new();
        env.set_unknown_method_callback(minijinja_contrib::pycompat::unknown_method_callback);
        env.add_filter("tojson", py_tojson);
        env.add_template_owned("chat", template_source.into())
            .context("failed to compile chat_template")?;
        Ok(Self { env })
    }

    pub fn from_tokenizer_config_json(tokenizer_config_json: &str) -> Result<Self> {
        let value: Value = serde_json::from_str(tokenizer_config_json)?;
        let template_source = value
            .get("chat_template")
            .and_then(Value::as_str)
            .context("tokenizer_config.json has no string 'chat_template' field")?;
        Self::new(template_source)
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn from_tokenizer_config_path(path: &str) -> Result<Self> {
        let text = std::fs::read_to_string(path).with_context(|| format!("reading {path}"))?;
        Self::from_tokenizer_config_json(&text)
    }

    /// `tools` renders as Jinja `none` when empty, matching
    /// `transformers.apply_chat_template([...], tools=[])`'s behavior of
    /// skipping the tool preamble.
    pub fn render(
        &self,
        messages: &[Message],
        tools: &[Tool],
        add_generation_prompt: bool,
    ) -> Result<String> {
        let tmpl = self.env.get_template("chat").unwrap();
        let tools_ctx = if tools.is_empty() { None } else { Some(tools) };
        let rendered = tmpl.render(context! { messages => messages, tools => tools_ctx, add_generation_prompt => add_generation_prompt })?;
        Ok(rendered)
    }
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
    Message::tool_result(name, content)
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
    Message::tool_result(name, content)
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

/// Runs the model forward from `prompt_ids`, sampling up to
/// `max_new_tokens` tokens (fewer if an eos id is hit). Greedy only.
///
/// Implementations live in `crate::backend`, built on lean's public CPU
/// and GPU forward-pass APIs; this trait itself has no engine dependency.
pub trait Generator {
    fn generate(&mut self, prompt_ids: &[u32], max_new_tokens: usize) -> Result<Vec<u32>>;
}

/// A `Generator` that replays a scripted sequence of outputs for tests.
pub struct FixtureGenerator {
    script: Vec<Vec<u32>>,
    next: usize,
}

impl FixtureGenerator {
    pub fn new(script: Vec<Vec<u32>>) -> Self {
        Self { script, next: 0 }
    }
}

impl Generator for FixtureGenerator {
    fn generate(&mut self, _prompt_ids: &[u32], _max_new_tokens: usize) -> Result<Vec<u32>> {
        let out = self.script.get(self.next).cloned().ok_or_else(|| {
            anyhow::anyhow!(
                "FixtureGenerator script exhausted after {} call(s)",
                self.next
            )
        })?;
        self.next += 1;
        Ok(out)
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
    pub prompt_tokens: Vec<u32>,
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
    Error {
        message: String,
        step: Option<Step>,
    },
}

struct AttemptOutput {
    prompt_tokens: Vec<u32>,
    generated_text: String,
    parsed: std::result::Result<ParsedOutput, String>,
}

pub struct Agent<G: Generator, C: ToolCaller> {
    template: AgentChatTemplate,
    tokenizer: tokenizers::Tokenizer,
    generator: G,
    caller: C,
    system_prompt: String,
    max_new_tokens: usize,
    max_steps: usize,
    max_retries: usize,

    messages: Vec<Message>,
    tools: Vec<Tool>,
    step_index: usize,
    pending_calls: Vec<PendingToolCall>,
    consecutive_tool_errors: usize,
    calls_made_this_turn: Vec<ToolCall>,
}

impl<G: Generator, C: ToolCaller> Agent<G, C> {
    pub fn new(
        template: AgentChatTemplate,
        tokenizer: tokenizers::Tokenizer,
        generator: G,
        caller: C,
        system_prompt: impl Into<String>,
        max_new_tokens: usize,
    ) -> Self {
        Self {
            template,
            tokenizer,
            generator,
            caller,
            system_prompt: system_prompt.into(),
            max_new_tokens,
            max_steps: DEFAULT_MAX_STEPS,
            max_retries: DEFAULT_MAX_RETRIES,
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

    pub fn set_max_steps(&mut self, max_steps: usize) {
        self.max_steps = max_steps;
    }

    pub fn set_max_retries(&mut self, max_retries: usize) {
        self.max_retries = max_retries;
    }

    pub fn reset(&mut self) {
        self.messages.clear();
        self.tools.clear();
        self.step_index = 0;
        self.pending_calls.clear();
        self.consecutive_tool_errors = 0;
        self.calls_made_this_turn.clear();
    }

    pub fn start(&mut self, utterance: &str, tools: &[Tool]) -> StepOutcome {
        self.tools = tools.to_vec();
        self.messages = vec![
            Message::system(&self.system_prompt),
            Message::user(utterance),
        ];
        self.step_index = 0;
        self.pending_calls.clear();
        self.consecutive_tool_errors = 0;
        self.calls_made_this_turn.clear();

        self.step_inner()
    }

    pub fn start_from_mcp(&mut self, utterance: &str, raw_tools: &[Value]) -> StepOutcome {
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
        self.start(utterance, &tools)
    }

    /// Continue the current turn with the results of the tool calls from
    /// the most recent `NeedTools` outcome, keyed by `call_id`.
    pub fn provide_tool_results(&mut self, results: Vec<(String, Value)>) -> StepOutcome {
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
        self.step_inner()
    }

    fn generate_attempt(&mut self) -> std::result::Result<AttemptOutput, String> {
        let rendered = self
            .template
            .render(&self.messages, &self.tools, true)
            .map_err(|e| format!("rendering prompt: {e:#}"))?;
        let encoding = self
            .tokenizer
            .encode(rendered, false)
            .map_err(|e| format!("tokenizer encode failed: {e}"))?;
        let prompt_tokens = encoding.get_ids().to_vec();
        let out_ids = self
            .generator
            .generate(&prompt_tokens, self.max_new_tokens)
            .map_err(|e| format!("generation failed: {e:#}"))?;
        let generated_text = self
            .tokenizer
            .decode(&out_ids, true)
            .map_err(|e| format!("tokenizer decode failed: {e}"))?;
        let parsed = parse_output(&generated_text);
        Ok(AttemptOutput {
            prompt_tokens,
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
    fn step_inner(&mut self) -> StepOutcome {
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
            let attempt = match self.generate_attempt() {
                Ok(a) => a,
                Err(message) => {
                    return StepOutcome::Error {
                        message,
                        step: None,
                    }
                }
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
                    return self.force_final_answer(repeat_guard);
                }
                return StepOutcome::Error {
                    message: "model produced no valid call".to_string(),
                    step: None,
                };
            }

            let AttemptOutput {
                prompt_tokens,
                generated_text,
                parsed,
            } = attempt;
            let parsed = parsed.expect("checked valid above");
            let tool_errors = self.consecutive_tool_errors;

            return match parsed {
                ParsedOutput::ToolCalls(calls) => {
                    let pending = pending_calls_from(&calls);
                    let entries = tool_call_entries(&pending);
                    self.messages.push(Message::assistant_tool_calls(entries));
                    self.pending_calls = pending.clone();
                    self.calls_made_this_turn.extend(calls.iter().cloned());

                    let step = Step {
                        prompt_tokens,
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
                    self.messages.push(Message::assistant_text(&text));
                    let step = Step {
                        prompt_tokens,
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
    fn force_final_answer(&mut self, repeat_guard: bool) -> StepOutcome {
        self.messages.push(Message::user(
            "Give your final answer as plain text now, with no tool call.",
        ));
        let attempt = match self.generate_attempt() {
            Ok(a) => a,
            Err(message) => {
                return StepOutcome::Error {
                    message,
                    step: None,
                }
            }
        };
        let AttemptOutput {
            prompt_tokens,
            generated_text,
            parsed,
        } = attempt;
        let text = match parsed {
            Ok(ParsedOutput::Text(t)) => t,
            _ => generated_text.clone(),
        };
        self.messages.push(Message::assistant_text(&text));
        let step = Step {
            prompt_tokens,
            generated_text,
            parsed: ParsedOutput::Text(text.clone()),
            retries: self.max_retries,
            tool_errors: self.consecutive_tool_errors,
            repeat_guard,
            forced_text_answer: true,
        };
        StepOutcome::Final { text, step }
    }

    /// Synchronous convenience for native callers (tests, eval, a CLI)
    /// with a synchronous `ToolCaller`: runs the whole turn to completion.
    pub fn run(&mut self, utterance: &str, tools: &[Tool], max_steps: usize) -> Result<Transcript> {
        self.max_steps = max_steps;
        let mut steps = Vec::with_capacity(max_steps);
        let mut outcome = self.start(utterance, tools);

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
                    outcome = self.provide_tool_results(results);
                }
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

fn tool_call_entries(pending: &[PendingToolCall]) -> Vec<ToolCallEntry> {
    pending
        .iter()
        .map(|c| ToolCallEntry {
            kind: "function".to_string(),
            function: ToolCallFunction {
                name: c.name.clone(),
                arguments: c.arguments.clone(),
            },
        })
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
        let tmpl = AgentChatTemplate::new(QWEN25_TOOLS_TEMPLATE).unwrap();
        let messages = vec![Message::user("What's the weather in Paris?")];
        let out = tmpl.render(&messages, &[weather_tool()], true).unwrap();
        assert!(out.starts_with("<|im_start|>system\n"));
        assert!(out.contains(
            "<tools>\n{\"type\": \"function\", \"function\": {\"name\": \"get_weather\""
        ));
        assert!(out.contains("\"required\": [\"city\"]}}}"));
        assert!(out.ends_with("<|im_start|>assistant\n"));
    }

    #[test]
    fn renders_multiturn_with_tool_call_and_result() {
        let tmpl = AgentChatTemplate::new(QWEN25_TOOLS_TEMPLATE).unwrap();
        let messages = vec![
            Message::user("What's the weather in Paris?"),
            Message::assistant_tool_calls(vec![ToolCallEntry {
                kind: "function".into(),
                function: ToolCallFunction {
                    name: "get_weather".into(),
                    arguments: serde_json::json!({"city": "Paris"}),
                },
            }]),
            format_tool_result(
                "get_weather",
                &serde_json::json!({"temp_c": 14, "condition": "cloudy"}),
            ),
        ];
        let out = tmpl.render(&messages, &[weather_tool()], true).unwrap();
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
        assert_eq!(
            msg.content,
            Some(Value::String(
                "{\"body\": \"{\\\"temp_c\\\": 14}\"}".to_string()
            ))
        );
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

    fn id(tokenizer: &tokenizers::Tokenizer, s: &str) -> Vec<u32> {
        tokenizer.encode(s, false).unwrap().get_ids().to_vec()
    }

    /// End-to-end step-wise loop against a scripted `FixtureGenerator`: the
    /// model calls a tool, gets a result, then answers in text.
    #[test]
    fn agent_loop_calls_tool_then_answers() {
        // No pre-tokenizer: each whole turn is one atomic vocab entry, so
        // encode/decode round-trips it exactly. This test only exercises
        // the loop's control flow, not real subword tokenization.
        let tool_call_text = "<tool_call>\n{\"name\": \"get_weather\", \"arguments\": {\"city\": \"Paris\"}}\n</tool_call>";
        let answer_text = "It is cloudy.";
        let vocab = [(tool_call_text, 0u32), (answer_text, 1), ("<unk>", 2)];
        let model = tokenizers::models::wordlevel::WordLevel::builder()
            .vocab(vocab.iter().map(|(s, i)| (s.to_string(), *i)).collect())
            .unk_token("<unk>".into())
            .build()
            .unwrap();
        let tokenizer = tokenizers::Tokenizer::new(model);

        let tool_call_ids = id(&tokenizer, tool_call_text);
        let answer_ids = id(&tokenizer, answer_text);

        let generator = FixtureGenerator::new(vec![tool_call_ids, answer_ids]);
        let template = AgentChatTemplate::new(QWEN25_TOOLS_TEMPLATE).unwrap();
        let mut agent = Agent::new(
            template,
            tokenizer,
            generator,
            EchoCaller,
            "You are a helpful assistant.",
            64,
        );

        let transcript = agent
            .run("What's the weather in Paris?", &[weather_tool()], 4)
            .unwrap();
        assert_eq!(transcript.steps.len(), 2);
        assert!(
            matches!(&transcript.steps[0].parsed, ParsedOutput::ToolCalls(calls) if calls.len() == 1 && calls[0].name == "get_weather")
        );
        assert!(matches!(&transcript.steps[1].parsed, ParsedOutput::Text(_)));
    }
}
