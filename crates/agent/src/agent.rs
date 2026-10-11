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

    /// Reads one MCP `tools/list` entry. `name` is required; a missing or
    /// `null` `description` reads as `""` and a missing `inputSchema` as
    /// an object schema with no properties (both are optional in MCP).
    pub fn try_from_mcp(entry: &Value) -> Result<Self> {
        let name = entry
            .get("name")
            .and_then(Value::as_str)
            .with_context(|| format!("MCP tool entry has no string `name`: {entry}"))?;
        let description = match entry.get("description") {
            None | Some(Value::Null) => "",
            Some(Value::String(s)) => s,
            Some(other) => bail!("MCP tool `{name}`: `description` is not a string: {other}"),
        };
        let input_schema = match entry.get("inputSchema") {
            None | Some(Value::Null) => serde_json::json!({"type": "object", "properties": {}}),
            Some(schema @ Value::Object(_)) => schema.clone(),
            Some(other) => bail!("MCP tool `{name}`: `inputSchema` is not an object: {other}"),
        };
        Ok(Tool::from_mcp(name, description, input_schema))
    }
}

/// Reads an MCP `tools/list` array with [`Tool::try_from_mcp`]; the first
/// invalid entry is an error naming it.
pub fn tools_from_mcp(entries: &[Value]) -> Result<Vec<Tool>> {
    entries.iter().map(Tool::try_from_mcp).collect()
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
    #[serde(default)]
    arguments: Option<Value>,
}

/// A call's `arguments` as a JSON object: missing or `null` is `{}`, and a
/// string holding a JSON object (double-encoded) is that object.
fn call_arguments(name: &str, arguments: Option<Value>) -> Result<Value, String> {
    match arguments {
        None | Some(Value::Null) => Ok(Value::Object(Default::default())),
        Some(args @ Value::Object(_)) => Ok(args),
        Some(Value::String(s)) => match serde_json::from_str::<Value>(&s) {
            Ok(args @ Value::Object(_)) => Ok(args),
            _ => Err(format!(
                "arguments of `{name}` are not a JSON object: {s:?}"
            )),
        },
        Some(other) => Err(format!(
            "arguments of `{name}` are not a JSON object: {other}"
        )),
    }
}

/// Parse one turn of raw (decoded) model output.
///
/// - A trailing `<|im_end|>` (and whitespace before it) is stripped first.
/// - Every `<tool_call>...</tool_call>` block found is parsed as one call;
///   a block whose contents don't parse as `{"name", "arguments"}` is an
///   error, not a panic. A call with no `arguments` has empty arguments.
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
        let arguments = call_arguments(&raw_call.name, raw_call.arguments)?;
        calls.push(ToolCall {
            name: raw_call.name,
            arguments,
        });
        rest = &after_open[end + CLOSE.len()..];
    }
    Ok(ParsedOutput::ToolCalls(calls))
}

/// The default limit on one tool result's formatted content, in bytes. The
/// agent feeds a larger result back as a tool error saying it was too
/// large, so one oversized result does not overflow the context; see
/// [`Agent::set_max_result_chars`].
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
/// compacting any pretty-printed JSON embedded in `result`. The content is
/// kept whole, whatever its size.
pub fn format_tool_result(name: &str, result: &Value) -> Message {
    let compacted = compact_embedded_json(result);
    let content = to_json_compact_py(&compacted).unwrap_or_else(|_| result.to_string());
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
/// `error` field that is not `null` or `false`), extract a human-readable
/// message; `None` otherwise.
pub fn tool_error_message(result: &Value) -> Option<String> {
    if let Some(error) = result
        .get("error")
        .filter(|e| !matches!(e, Value::Null | Value::Bool(false)))
    {
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

/// The first sentence of the `user` note added to a retry's prompt after
/// an invalid reply; the note then says what was wrong.
const RETRY_NOTE: &str = "Respond with a tool call from the list or a final answer.";

/// The `user` note added to the prompt when the retries are spent on
/// repeated or empty tool calls.
const FINAL_ANSWER_NOTE: &str = "Give your final answer as plain text now, with no tool call.";

/// Executes one tool call and returns its JSON result.
pub trait ToolCaller {
    fn call(&mut self, name: &str, args: &Value) -> Result<Value>;
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
    /// The prompt of the accepted attempt.
    pub prompt: String,
    /// The model's raw reply to that prompt.
    pub generated_text: String,
    pub parsed: ParsedOutput,
    /// Attempts regenerated because the reply was invalid or repeated.
    pub retries: usize,
    /// Tool error results fed back in this step's prompt.
    pub tool_errors: usize,
    /// Tool results fed back as "too large" errors in this step's prompt.
    pub oversized_results: usize,
    /// A reply repeated a call already made this turn.
    pub repeat_guard: bool,
    /// The step ended with the model asked for a plain-text answer.
    pub forced_text_answer: bool,
}

#[derive(Debug)]
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
    /// `available` in the context). The agent is left as it was before the
    /// call that returned this: after `start`, call `start` again with
    /// fewer tools or a shorter utterance; after `provide_tool_results`,
    /// call it again with smaller results (or errors in their place), or
    /// lower `max_new_tokens` first.
    ContextOverflow {
        needed: usize,
        available: usize,
    },
    /// The turn was stopped through [`Agent::abort_flag`].
    Aborted,
    /// The turn cannot go on, except after a `provide_tool_results` whose
    /// call ids do not match the pending calls: the agent is left as it
    /// was, and a call with the right ids continues the turn.
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

/// Why a reply was not accepted.
enum Rejection {
    /// Unparsable, or a call to a tool that is not in the list.
    Invalid,
    /// A `<tool_call>` block with no calls in it.
    Empty,
    /// Every call in it was already made this turn.
    Repeat,
}

/// Everything a turn changes, saved before a call so a context overflow
/// can put it back.
struct TurnState {
    messages_len: usize,
    step_index: usize,
    pending_calls: Vec<PendingToolCall>,
    consecutive_tool_errors: usize,
    step_tool_errors: usize,
    step_oversized_results: usize,
    calls_made_this_turn: Vec<ToolCall>,
}

/// The step-wise agent loop over a [`Generator`]. One turn answers one
/// utterance: [`Agent::start`] begins it (and a new conversation), each
/// [`StepOutcome::NeedTools`] is answered with
/// [`Agent::provide_tool_results`], until a final text answer.
pub struct Agent<G: Generator> {
    template: ChatTemplate,
    generator: G,
    system_prompt: String,
    max_new_tokens: usize,
    max_steps: usize,
    max_retries: usize,
    max_result_chars: usize,
    abort: AbortFlag,
    on_text: Box<dyn FnMut(&str)>,

    messages: Vec<Message>,
    tools: Vec<Tool>,
    step_index: usize,
    pending_calls: Vec<PendingToolCall>,
    consecutive_tool_errors: usize,
    step_tool_errors: usize,
    step_oversized_results: usize,
    calls_made_this_turn: Vec<ToolCall>,
}

impl<G: Generator> Agent<G> {
    pub fn new(
        template: ChatTemplate,
        generator: G,
        system_prompt: impl Into<String>,
        max_new_tokens: usize,
    ) -> Self {
        Self {
            template,
            generator,
            system_prompt: system_prompt.into(),
            max_new_tokens,
            max_steps: DEFAULT_MAX_STEPS,
            max_retries: DEFAULT_MAX_RETRIES,
            max_result_chars: MAX_RESULT_CHARS,
            abort: AbortFlag::new(),
            on_text: Box::new(|_| {}),
            messages: Vec::new(),
            tools: Vec::new(),
            step_index: 0,
            pending_calls: Vec::new(),
            consecutive_tool_errors: 0,
            step_tool_errors: 0,
            step_oversized_results: 0,
            calls_made_this_turn: Vec::new(),
        }
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

    pub fn set_max_new_tokens(&mut self, max_new_tokens: usize) {
        self.max_new_tokens = max_new_tokens;
    }

    /// A tool result whose formatted content is longer than this (in
    /// bytes) is fed back to the model as a tool error saying it was too
    /// large. Default [`MAX_RESULT_CHARS`].
    pub fn set_max_result_chars(&mut self, max_result_chars: usize) {
        self.max_result_chars = max_result_chars;
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
        self.step_tool_errors = 0;
        self.step_oversized_results = 0;
        self.calls_made_this_turn.clear();
    }

    /// Begins a turn for `utterance` with `tools`. Each turn is a new
    /// conversation: the system prompt and this one user message.
    pub async fn start(&mut self, utterance: &str, tools: &[Tool]) -> StepOutcome {
        self.abort.reset();
        self.reset();
        self.tools = tools.to_vec();
        self.messages = vec![
            Message::system(&self.system_prompt),
            Message::user(utterance),
        ];
        let outcome = self.step_inner().await;
        if matches!(outcome, StepOutcome::ContextOverflow { .. }) {
            self.reset();
        }
        outcome
    }

    /// [`Agent::start`] with the tools of an MCP `tools/list` array, read
    /// with [`tools_from_mcp`]; an invalid entry is an `Error` outcome.
    pub async fn start_from_mcp(&mut self, utterance: &str, raw_tools: &[Value]) -> StepOutcome {
        match tools_from_mcp(raw_tools) {
            Ok(tools) => self.start(utterance, &tools).await,
            Err(e) => StepOutcome::Error {
                message: format!("reading tools: {e:#}"),
                step: None,
            },
        }
    }

    /// Continues the turn with the results of the calls from the most
    /// recent `NeedTools` outcome, one per call, keyed by `call_id`. An
    /// unknown, duplicate or missing id is an `Error` outcome that leaves
    /// the agent as it was.
    pub async fn provide_tool_results(&mut self, results: Vec<(String, Value)>) -> StepOutcome {
        let ordered = match self.results_in_call_order(results) {
            Ok(ordered) => ordered,
            Err(message) => {
                return StepOutcome::Error {
                    message,
                    step: None,
                }
            }
        };
        let saved = self.save_turn();

        let pending = std::mem::take(&mut self.pending_calls);
        self.step_tool_errors = 0;
        self.step_oversized_results = 0;
        for (call, result) in pending.iter().zip(ordered) {
            if let Some(error_message) = tool_error_message(&result) {
                self.consecutive_tool_errors += 1;
                self.step_tool_errors += 1;
                if self.consecutive_tool_errors > MAX_CONSECUTIVE_TOOL_ERRORS {
                    return StepOutcome::Error {
                        message: format!(
                            "{} consecutive tool errors (giving up after {}): {error_message}",
                            self.consecutive_tool_errors, MAX_CONSECUTIVE_TOOL_ERRORS
                        ),
                        step: None,
                    };
                }
                // A failed call may be retried as is.
                let failed = ToolCall {
                    name: call.name.clone(),
                    arguments: call.arguments.clone(),
                };
                self.calls_made_this_turn.retain(|c| *c != failed);
                self.messages
                    .push(format_tool_error(&call.name, &error_message));
            } else {
                self.consecutive_tool_errors = 0;
                let message = format_tool_result(&call.name, &result);
                if message.content.len() > self.max_result_chars {
                    self.step_oversized_results += 1;
                    self.messages.push(format_tool_error(
                        &call.name,
                        &format!(
                            "result too large: {} characters, the limit is {}",
                            message.content.len(),
                            self.max_result_chars
                        ),
                    ));
                } else {
                    self.messages.push(message);
                }
            }
        }

        let outcome = self.step_inner().await;
        if matches!(outcome, StepOutcome::ContextOverflow { .. }) {
            self.restore_turn(saved);
        }
        outcome
    }

    /// `results` in the order of the pending calls, or why they do not
    /// match them one to one.
    fn results_in_call_order(
        &self,
        mut results: Vec<(String, Value)>,
    ) -> std::result::Result<Vec<Value>, String> {
        let mut ordered = Vec::with_capacity(self.pending_calls.len());
        for call in &self.pending_calls {
            let index = results
                .iter()
                .position(|(id, _)| *id == call.call_id)
                .ok_or_else(|| {
                    format!("no result for tool call `{}` ({})", call.call_id, call.name)
                })?;
            ordered.push(results.swap_remove(index).1);
        }
        if let Some((id, _)) = results.first() {
            return Err(format!(
                "no pending tool call has id `{id}`, or it has more than one result"
            ));
        }
        Ok(ordered)
    }

    fn save_turn(&self) -> TurnState {
        TurnState {
            messages_len: self.messages.len(),
            step_index: self.step_index,
            pending_calls: self.pending_calls.clone(),
            consecutive_tool_errors: self.consecutive_tool_errors,
            step_tool_errors: self.step_tool_errors,
            step_oversized_results: self.step_oversized_results,
            calls_made_this_turn: self.calls_made_this_turn.clone(),
        }
    }

    fn restore_turn(&mut self, saved: TurnState) {
        self.messages.truncate(saved.messages_len);
        self.step_index = saved.step_index;
        self.pending_calls = saved.pending_calls;
        self.consecutive_tool_errors = saved.consecutive_tool_errors;
        self.step_tool_errors = saved.step_tool_errors;
        self.step_oversized_results = saved.step_oversized_results;
        self.calls_made_this_turn = saved.calls_made_this_turn;
    }

    /// Renders the conversation plus `notes` (as `user` messages that are
    /// not kept in the history), generates and parses one reply. `Err` is
    /// the outcome to return as is (render or engine failure, overflow,
    /// abort).
    async fn generate_attempt(
        &mut self,
        notes: &[String],
    ) -> std::result::Result<AttemptOutput, Box<StepOutcome>> {
        let rendered = if notes.is_empty() {
            render_prompt(&self.template, &self.messages, &self.tools)
        } else {
            let mut messages = self.messages.clone();
            messages.extend(notes.iter().map(Message::user));
            render_prompt(&self.template, &messages, &self.tools)
        };
        let prompt = rendered.map_err(|e| StepOutcome::Error {
            message: format!("rendering prompt: {e:#}"),
            step: None,
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

    /// `parsed` if it can be accepted; otherwise why not, with a short
    /// reason for the model and the full one for the caller.
    fn check(
        &self,
        parsed: std::result::Result<ParsedOutput, String>,
    ) -> std::result::Result<ParsedOutput, (Rejection, String, String)> {
        let calls = match parsed {
            Err(e) => {
                return Err((
                    Rejection::Invalid,
                    "Your last reply was not a valid tool call.".to_string(),
                    e,
                ))
            }
            Ok(ParsedOutput::ToolCalls(calls)) => calls,
            Ok(text) => return Ok(text),
        };
        if calls.is_empty() {
            return Err((
                Rejection::Empty,
                "Your last reply had no tool call in it.".to_string(),
                "empty tool call list".to_string(),
            ));
        }
        if let Some(unknown) = calls
            .iter()
            .find(|c| !self.tools.iter().any(|t| t.function.name == c.name))
        {
            return Err((
                Rejection::Invalid,
                format!("`{}` is not one of the tools.", unknown.name),
                format!("call to unknown tool `{}`", unknown.name),
            ));
        }
        if let Some(repeat) = calls.iter().find(|c| self.calls_made_this_turn.contains(c)) {
            return Err((
                Rejection::Repeat,
                format!(
                    "`{}` was already called with these arguments; use its result.",
                    repeat.name
                ),
                format!("repeated call {}({})", repeat.name, repeat.arguments),
            ));
        }
        Ok(ParsedOutput::ToolCalls(calls))
    }

    /// One render -> generate -> parse round using `self.messages`/`self.tools`.
    ///
    /// Retry policy: an unparsable reply, an empty tool-call block, a call
    /// to a tool outside `self.tools`, or a call identical (same name and
    /// arguments) to one already made this turn whose result was not an
    /// error is not accepted. The agent regenerates up to
    /// `self.max_retries` times, each time with a `user` note saying what
    /// was wrong; the notes are part of that step's prompts only, never of
    /// the history. When the retries are spent on repeated or empty calls,
    /// the model is asked once more for a plain-text answer; otherwise the
    /// step is an error carrying the last reason.
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

        let mut notes: Vec<String> = Vec::new();
        let mut repeat_guard = false;
        loop {
            let attempt = match self.generate_attempt(&notes).await {
                Ok(a) => a,
                Err(outcome) => return *outcome,
            };

            let AttemptOutput {
                prompt,
                generated_text,
                parsed,
            } = attempt;
            match self.check(parsed) {
                Ok(parsed) => {
                    let step = Step {
                        prompt,
                        generated_text,
                        parsed,
                        retries: notes.len(),
                        tool_errors: self.step_tool_errors,
                        oversized_results: self.step_oversized_results,
                        repeat_guard,
                        forced_text_answer: false,
                    };
                    return self.accept(step);
                }
                Err((rejection, note, detail)) => {
                    if matches!(rejection, Rejection::Repeat) {
                        repeat_guard = true;
                    }
                    if notes.len() < self.max_retries {
                        notes.push(format!("{note} {RETRY_NOTE}"));
                        continue;
                    }
                    if matches!(rejection, Rejection::Repeat | Rejection::Empty) {
                        return self.force_final_answer(notes, repeat_guard).await;
                    }
                    return StepOutcome::Error {
                        message: format!(
                            "model produced no valid call after {} retries: {detail}",
                            notes.len()
                        ),
                        step: None,
                    };
                }
            }
        }
    }

    fn accept(&mut self, step: Step) -> StepOutcome {
        match &step.parsed {
            ParsedOutput::ToolCalls(calls) => {
                let pending = pending_calls_from(calls);
                self.messages
                    .push(Message::assistant_tool_calls("", chat_tool_calls(&pending)));
                self.pending_calls = pending.clone();
                self.calls_made_this_turn.extend(calls.iter().cloned());
                StepOutcome::NeedTools {
                    calls: pending,
                    step,
                }
            }
            ParsedOutput::Text(text) => {
                let text = text.clone();
                self.messages.push(Message::assistant(&text));
                StepOutcome::Final { text, step }
            }
        }
    }

    /// Last resort when the retries are spent on repeated or empty tool
    /// calls: one more generation, in the same step, with
    /// [`FINAL_ANSWER_NOTE`] added. A text reply is the final answer; a
    /// reply that still calls a tool is an error (its markup is never
    /// returned as the answer).
    async fn force_final_answer(
        &mut self,
        mut notes: Vec<String>,
        repeat_guard: bool,
    ) -> StepOutcome {
        let retries = notes.len();
        notes.push(FINAL_ANSWER_NOTE.to_string());
        let attempt = match self.generate_attempt(&notes).await {
            Ok(a) => a,
            Err(outcome) => return *outcome,
        };
        let AttemptOutput {
            prompt,
            generated_text,
            parsed,
        } = attempt;
        let parsed = match parsed {
            Ok(parsed) => parsed,
            Err(e) => {
                return StepOutcome::Error {
                    message: format!("asked for a plain-text answer, the model replied: {e}"),
                    step: None,
                }
            }
        };
        let is_text = matches!(parsed, ParsedOutput::Text(_));
        let step = Step {
            prompt,
            generated_text,
            parsed,
            retries,
            tool_errors: self.step_tool_errors,
            oversized_results: self.step_oversized_results,
            repeat_guard,
            forced_text_answer: true,
        };
        if is_text {
            self.accept(step)
        } else {
            StepOutcome::Error {
                message: "asked for a plain-text answer, the model called a tool again".to_string(),
                step: Some(step),
            }
        }
    }

    /// Runs a whole turn with a synchronous `caller` (tests, eval, a CLI),
    /// up to the agent's `max_steps`. A caller error is fed to the model
    /// as a tool error, like an MCP error result.
    pub async fn run<C: ToolCaller>(
        &mut self,
        utterance: &str,
        tools: &[Tool],
        caller: &mut C,
    ) -> Result<Transcript> {
        let mut steps = Vec::new();
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
                    let results = calls
                        .iter()
                        .map(|call| (call.call_id.clone(), call_or_error(caller, call)))
                        .collect();
                    outcome = self.provide_tool_results(results).await;
                }
                StepOutcome::ContextOverflow { needed, available } => {
                    bail!("context overflow: needed {needed} tokens, {available} available");
                }
                StepOutcome::Aborted => bail!("aborted"),
                StepOutcome::Error { message, .. } => bail!("{message}"),
            }
        }
    }
}

/// `caller`'s result for `call`, or its error as an `{"error": ...}`
/// result.
pub fn call_or_error<C: ToolCaller>(caller: &mut C, call: &PendingToolCall) -> Value {
    caller
        .call(&call.name, &call.arguments)
        .unwrap_or_else(|e| serde_json::json!({ "error": format!("{e:#}") }))
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
    use crate::test_support::FakeGenerator;

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
    fn call_without_arguments_has_empty_arguments() {
        let empty = serde_json::json!({});
        for raw in [
            "<tool_call>\n{\"name\": \"a\"}\n</tool_call>",
            "<tool_call>\n{\"name\": \"a\", \"arguments\": null}\n</tool_call>",
            "<tool_call>\n{\"name\": \"a\", \"arguments\": \"{}\"}\n</tool_call>",
        ] {
            assert_eq!(
                parse_output(raw).unwrap(),
                ParsedOutput::ToolCalls(vec![ToolCall {
                    name: "a".into(),
                    arguments: empty.clone()
                }]),
                "{raw}"
            );
        }
        assert!(
            parse_output("<tool_call>\n{\"name\": \"a\", \"arguments\": 3}\n</tool_call>").is_err()
        );
    }

    #[test]
    fn mcp_tools_without_description_or_schema_are_accepted() {
        let tools = tools_from_mcp(&[
            serde_json::json!({"name": "ping"}),
            serde_json::json!({"name": "echo", "description": null, "inputSchema": {"type": "object"}}),
        ])
        .unwrap();
        assert_eq!(tools[0].function.name, "ping");
        assert_eq!(tools[0].function.description, "");
        assert_eq!(
            tools[0].function.parameters,
            serde_json::json!({"type": "object", "properties": {}})
        );
        assert_eq!(tools[1].function.description, "");
        assert!(tools_from_mcp(&[serde_json::json!({"description": "no name"})]).is_err());
        assert!(
            tools_from_mcp(&[serde_json::json!({"name": "x", "inputSchema": "nope"})]).is_err()
        );
    }

    #[test]
    fn null_or_false_error_field_is_not_an_error() {
        assert_eq!(
            tool_error_message(&serde_json::json!({"error": null, "ok": 1})),
            None
        );
        assert_eq!(
            tool_error_message(&serde_json::json!({"error": false})),
            None
        );
        assert_eq!(
            tool_error_message(&serde_json::json!({"error": "no such group"})),
            Some("no such group".to_string())
        );
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

    struct FailingCaller;
    impl ToolCaller for FailingCaller {
        fn call(&mut self, name: &str, _args: &Value) -> Result<Value> {
            bail!("{name} is offline")
        }
    }

    const PARIS_CALL: &str = "<tool_call>\n{\"name\": \"get_weather\", \"arguments\": {\"city\": \"Paris\"}}\n</tool_call>";
    const LYON_CALL: &str = "<tool_call>\n{\"name\": \"get_weather\", \"arguments\": {\"city\": \"Lyon\"}}\n</tool_call>";

    fn agent_with(generator: FakeGenerator) -> Agent<FakeGenerator> {
        Agent::new(
            ChatTemplate::new(QWEN25_TOOLS_TEMPLATE),
            generator,
            "You are a helpful assistant.",
            64,
        )
    }

    fn start(agent: &mut Agent<FakeGenerator>) -> StepOutcome {
        pollster::block_on(agent.start("What's the weather in Paris?", &[weather_tool()]))
    }

    fn provide(agent: &mut Agent<FakeGenerator>, result: Value) -> StepOutcome {
        pollster::block_on(agent.provide_tool_results(vec![("call_0".to_string(), result)]))
    }

    fn sunny() -> Value {
        serde_json::json!({"temp_c": 21, "condition": "sunny"})
    }

    /// End-to-end step-wise loop: the model calls a tool, gets a result,
    /// then answers in text.
    #[test]
    fn agent_loop_calls_tool_then_answers() {
        let answer_text = "It is cloudy.";
        let mut agent = agent_with(FakeGenerator::script(&[PARIS_CALL, answer_text]));
        agent.set_max_steps(4);

        let transcript = pollster::block_on(agent.run(
            "What's the weather in Paris?",
            &[weather_tool()],
            &mut EchoCaller,
        ))
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
    fn run_feeds_a_caller_error_to_the_model() {
        let mut agent = agent_with(FakeGenerator::script(&[PARIS_CALL, "It is offline."]));
        let transcript = pollster::block_on(agent.run(
            "What's the weather in Paris?",
            &[weather_tool()],
            &mut FailingCaller,
        ))
        .unwrap();
        assert_eq!(transcript.steps[1].tool_errors, 1);
        assert!(transcript.steps[1].prompt.contains(
            "<tool_response>\n{\"error\": \"get_weather is offline\"}\n</tool_response>"
        ));
    }

    #[test]
    fn streams_text_and_abort_stops_the_turn() {
        let mut agent = agent_with(FakeGenerator::script(&["Hello."]));
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

    #[test]
    fn abort_during_a_generation_stops_the_step() {
        let mut agent = agent_with(FakeGenerator::script(&[PARIS_CALL]));
        let flag = agent.abort_flag();
        agent.set_on_text(move |_| flag.abort());
        assert!(matches!(start(&mut agent), StepOutcome::Aborted));
    }

    #[test]
    fn retry_notes_are_not_kept_in_history() {
        let generator = FakeGenerator::script(&[
            "<tool_call>\nnot json\n</tool_call>",
            PARIS_CALL,
            "It is sunny.",
        ]);
        let prompts = generator.prompts();
        let mut agent = agent_with(generator);

        let outcome = start(&mut agent);
        let StepOutcome::NeedTools { step, .. } = outcome else {
            panic!("{outcome:?}");
        };
        assert_eq!(step.retries, 1);
        assert!(prompts.borrow()[1].contains("Your last reply was not a valid tool call."));

        let outcome = provide(&mut agent, sunny());
        assert!(matches!(outcome, StepOutcome::Final { .. }), "{outcome:?}");
        let prompts = prompts.borrow();
        assert!(!prompts[2].contains(RETRY_NOTE));
        // The next step's prompt extends the first one, so an engine can
        // reuse its KV cache for all of it.
        assert!(prompts[2].starts_with(&prompts[0]));
    }

    #[test]
    fn spent_retries_report_the_last_reason() {
        let mut agent = agent_with(FakeGenerator::script(&[
            "<tool_call>\nnot json\n</tool_call>",
            "<tool_call>\n{\"name\": \"get_weather\"\n</tool_call>",
            "<tool_call>\n{\"name\": \"nope\", \"arguments\": {}}\n</tool_call>",
        ]));
        let outcome = start(&mut agent);
        let StepOutcome::Error { message, .. } = outcome else {
            panic!("{outcome:?}");
        };
        assert!(message.contains("after 2 retries"), "{message}");
        assert!(message.contains("unknown tool `nope`"), "{message}");
    }

    #[test]
    fn repeated_call_ends_in_a_forced_text_answer() {
        let generator = FakeGenerator::script(&[
            PARIS_CALL,
            PARIS_CALL,
            PARIS_CALL,
            PARIS_CALL,
            "It is sunny.",
        ]);
        let prompts = generator.prompts();
        let mut agent = agent_with(generator);
        assert!(matches!(start(&mut agent), StepOutcome::NeedTools { .. }));

        let outcome = provide(&mut agent, sunny());
        let StepOutcome::Final { text, step } = outcome else {
            panic!("{outcome:?}");
        };
        assert_eq!(text, "It is sunny.");
        assert!(step.forced_text_answer && step.repeat_guard);
        assert_eq!(step.retries, 2);
        assert!(prompts.borrow()[4].contains(FINAL_ANSWER_NOTE));

        // The note is not kept: the history ends with the answer.
        let next = pollster::block_on(agent.provide_tool_results(Vec::new()));
        assert!(matches!(next, StepOutcome::Error { .. }));
        let last = prompts.borrow().last().cloned().unwrap();
        assert!(!last.contains(FINAL_ANSWER_NOTE));
    }

    #[test]
    fn forced_answer_that_calls_a_tool_is_an_error_not_markup() {
        let mut agent = agent_with(FakeGenerator::script(&[PARIS_CALL; 5]));
        assert!(matches!(start(&mut agent), StepOutcome::NeedTools { .. }));
        let outcome = provide(&mut agent, sunny());
        let StepOutcome::Error { message, step } = outcome else {
            panic!("{outcome:?}");
        };
        assert!(message.contains("called a tool again"), "{message}");
        assert!(step.is_some_and(|s| s.forced_text_answer));
    }

    #[test]
    fn a_failed_call_may_be_retried_as_is() {
        let mut agent = agent_with(FakeGenerator::script(&[PARIS_CALL, PARIS_CALL]));
        assert!(matches!(start(&mut agent), StepOutcome::NeedTools { .. }));
        let outcome = provide(&mut agent, serde_json::json!({"error": "timeout"}));
        let StepOutcome::NeedTools { step, .. } = outcome else {
            panic!("{outcome:?}");
        };
        assert!(!step.repeat_guard);
        assert_eq!(step.retries, 0);
        assert_eq!(step.tool_errors, 1);
    }

    #[test]
    fn tool_errors_count_per_step() {
        let mut agent = agent_with(FakeGenerator::script(&[
            PARIS_CALL,
            LYON_CALL,
            "<tool_call>\n{\"name\": \"get_weather\", \"arguments\": {\"city\": \"Nice\"}}\n</tool_call>",
        ]));
        assert!(matches!(start(&mut agent), StepOutcome::NeedTools { .. }));
        let error = serde_json::json!({"error": "timeout"});
        let StepOutcome::NeedTools { step, .. } = provide(&mut agent, error.clone()) else {
            panic!()
        };
        assert_eq!(step.tool_errors, 1);
        let StepOutcome::NeedTools { step, .. } = provide(&mut agent, error) else {
            panic!()
        };
        assert_eq!(step.tool_errors, 1);
    }

    #[test]
    fn too_many_consecutive_tool_errors_end_the_turn() {
        let mut agent = agent_with(FakeGenerator::script(&[PARIS_CALL; 4]));
        assert!(matches!(start(&mut agent), StepOutcome::NeedTools { .. }));
        let error =
            serde_json::json!({"isError": true, "content": [{"type": "text", "text": "bad city"}]});
        for _ in 0..MAX_CONSECUTIVE_TOOL_ERRORS {
            let outcome = provide(&mut agent, error.clone());
            assert!(
                matches!(outcome, StepOutcome::NeedTools { .. }),
                "{outcome:?}"
            );
        }
        let outcome = provide(&mut agent, error);
        let StepOutcome::Error { message, .. } = outcome else {
            panic!("{outcome:?}");
        };
        assert!(message.contains("4 consecutive tool errors"), "{message}");
    }

    #[test]
    fn max_steps_ends_the_turn() {
        let mut agent = agent_with(FakeGenerator::script(&[PARIS_CALL]));
        agent.set_max_steps(1);
        assert!(matches!(start(&mut agent), StepOutcome::NeedTools { .. }));
        let outcome = provide(&mut agent, sunny());
        assert!(
            matches!(&outcome, StepOutcome::Error { message, .. } if message.contains("max_steps")),
            "{outcome:?}"
        );
    }

    #[test]
    fn generator_failure_is_an_error_outcome() {
        let mut agent = agent_with(FakeGenerator::script(&[]));
        let outcome = start(&mut agent);
        assert!(
            matches!(&outcome, StepOutcome::Error { message, .. } if message.contains("generation failed")),
            "{outcome:?}"
        );
    }

    #[test]
    fn context_overflow_mid_turn_is_recoverable() {
        const LIMIT: usize = 4000;
        let mut replies = vec![PARIS_CALL, "It is sunny."].into_iter();
        let generator = FakeGenerator::new(move |prompt| {
            if prompt.len() > LIMIT {
                return Err(GenerateError::ContextOverflow {
                    needed: prompt.len(),
                    available: LIMIT,
                });
            }
            Ok(replies.next().unwrap().to_string())
        });
        let prompts = generator.prompts();
        let mut agent = agent_with(generator);
        assert!(matches!(start(&mut agent), StepOutcome::NeedTools { .. }));

        let big = serde_json::json!({"report": "x".repeat(LIMIT)});
        let outcome = provide(&mut agent, big);
        assert!(
            matches!(
                outcome,
                StepOutcome::ContextOverflow {
                    available: LIMIT,
                    ..
                }
            ),
            "{outcome:?}"
        );

        // The same call takes a smaller result.
        let outcome = provide(&mut agent, sunny());
        let StepOutcome::Final { text, step } = outcome else {
            panic!("{outcome:?}");
        };
        assert_eq!(text, "It is sunny.");
        assert!(!step.prompt.contains("xxxx"));
        assert_eq!(prompts.borrow().len(), 3);
    }

    #[test]
    fn oversized_result_is_fed_back_as_an_error() {
        let generator = FakeGenerator::script(&[PARIS_CALL, "It is too much."]);
        let mut agent = agent_with(generator);
        assert!(matches!(start(&mut agent), StepOutcome::NeedTools { .. }));
        let big = serde_json::json!({"report": "x".repeat(MAX_RESULT_CHARS)});
        let StepOutcome::Final { step, .. } = provide(&mut agent, big) else {
            panic!()
        };
        assert_eq!(step.oversized_results, 1);
        assert!(step.prompt.contains("result too large"));
        assert!(!step.prompt.contains("xxxx"));
    }

    #[test]
    fn tool_results_must_match_the_pending_calls() {
        let mut agent = agent_with(FakeGenerator::script(&[PARIS_CALL, "It is sunny."]));
        assert!(matches!(start(&mut agent), StepOutcome::NeedTools { .. }));

        for results in [
            vec![],
            vec![("call_7".to_string(), sunny())],
            vec![
                ("call_0".to_string(), sunny()),
                ("call_0".to_string(), sunny()),
            ],
        ] {
            let outcome = pollster::block_on(agent.provide_tool_results(results));
            assert!(matches!(outcome, StepOutcome::Error { .. }), "{outcome:?}");
        }
        // The agent was left as it was: the right id continues the turn.
        assert!(matches!(
            provide(&mut agent, sunny()),
            StepOutcome::Final { .. }
        ));
    }

    #[test]
    fn start_from_mcp_takes_tools_without_a_description() {
        let mut agent = agent_with(FakeGenerator::script(&[
            "<tool_call>\n{\"name\": \"ping\"}\n</tool_call>",
        ]));
        let outcome = pollster::block_on(
            agent.start_from_mcp("ping it", &[serde_json::json!({"name": "ping"})]),
        );
        let StepOutcome::NeedTools { calls, .. } = outcome else {
            panic!("{outcome:?}");
        };
        assert_eq!(calls[0].name, "ping");
        assert_eq!(calls[0].arguments, serde_json::json!({}));
    }
}
