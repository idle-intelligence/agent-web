//! Offline Sonos MCP tool-call eval: scores an agent transcript against a
//! fixed set of utterance cases, each with an expected call sequence and
//! an acceptance mode. The agent run here is unconstrained (no
//! schema-constrained decoding), so `CaseResult`/`EvalReport` carry only
//! steps, time, retries and tool_errors.
//!
//! Native only (`std::fs`, `std::time::Instant`); not part of the wasm
//! build.

use crate::agent::{
    call_or_error, tools_from_mcp, Agent, Generator, StepOutcome, StepTiming, Tool, ToolCall,
    ToolCaller,
};
use anyhow::{Context, Result};
use serde::Deserialize;
use serde_json::Value;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Accept {
    Exact,
    Subset,
    #[serde(rename = "no_mutation")]
    NoMutation,
    #[serde(rename = "any_play")]
    AnyPlay,
    /// Exactly `case.expected.len()` calls in order; each call's name must
    /// match and its arguments must match on the expected keys only (extra
    /// keys are ignored), with string values compared case-insensitively.
    #[serde(rename = "exact_args")]
    ExactArgs,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct ExpectedCall {
    pub name: String,
    pub args: Value,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EvalCase {
    pub id: String,
    pub utterance: String,
    pub kind: String,
    pub expected: Vec<ExpectedCall>,
    pub accept: Accept,
    pub subset_ok: bool,
    #[serde(default)]
    pub notes: String,
    #[serde(default)]
    pub lang: Option<String>,
    /// If non-empty, the final text answer must contain at least one of
    /// these substrings (case-insensitive), on top of what `accept`
    /// requires of the calls.
    #[serde(default)]
    pub answer_contains_any: Vec<String>,
    /// If non-empty, the final text answer must contain every one of these
    /// substrings (case-insensitive).
    #[serde(default)]
    pub answer_contains_all: Vec<String>,
}

/// A [`ToolCaller`] that serves canned JSON results from
/// `<results_dir>/<tool>.json` and records every call it saw, in order.
pub struct FixtureCaller {
    results_dir: std::path::PathBuf,
    pub calls_seen: Vec<(String, Value)>,
}

/// A [`ToolCaller`] whose calls so far the eval can read back.
pub trait CallLog {
    fn calls_seen(&self) -> &[(String, Value)];
}

impl CallLog for FixtureCaller {
    fn calls_seen(&self) -> &[(String, Value)] {
        &self.calls_seen
    }
}

/// A [`ToolCaller`] for the two-step fixture set (`fixtures/two-step/`):
/// `get_location` and `list_rooms` return a fixed value; `get_weather` and
/// `set_volume` compute their result from their arguments, which no static
/// per-tool file can do.
#[derive(Default)]
pub struct TwoStepCaller {
    pub calls_seen: Vec<(String, Value)>,
}

impl TwoStepCaller {
    pub fn new() -> Self {
        Self::default()
    }
}

impl ToolCaller for TwoStepCaller {
    fn call(&mut self, name: &str, args: &Value) -> Result<Value> {
        self.calls_seen.push((name.to_string(), args.clone()));
        let result = match name {
            "get_location" => serde_json::json!({ "city": "Lyon", "country": "FR" }),
            "get_weather" => {
                let city = args.get("city").and_then(Value::as_str).unwrap_or("");
                let fahrenheit = args.get("unit").and_then(Value::as_str) == Some("fahrenheit");
                serde_json::json!({
                    "city": city,
                    "temp": if fahrenheit { 63 } else { 17 },
                    "condition": "light rain",
                })
            }
            "list_rooms" => serde_json::json!([
                { "id": "RINCON_K1", "name": "Kitchen" },
                { "id": "RINCON_L2", "name": "Living room" },
            ]),
            "set_volume" => {
                let room_id = args.get("room_id").and_then(Value::as_str).unwrap_or("");
                let level = args.get("level").cloned().unwrap_or(Value::Null);
                match room_id {
                    "RINCON_K1" | "RINCON_L2" => {
                        serde_json::json!({ "ok": true, "room_id": room_id, "level": level })
                    }
                    _ => serde_json::json!({ "error": "unknown room_id" }),
                }
            }
            other => anyhow::bail!("TwoStepCaller: unknown tool {other}"),
        };
        Ok(result)
    }
}

impl CallLog for TwoStepCaller {
    fn calls_seen(&self) -> &[(String, Value)] {
        &self.calls_seen
    }
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

pub fn load_cases(path: impl AsRef<Path>) -> Result<Vec<EvalCase>> {
    let path = path.as_ref();
    let raw = std::fs::read_to_string(path)
        .with_context(|| format!("reading eval cases from {path:?}"))?;
    serde_json::from_str(&raw).with_context(|| format!("parsing eval cases from {path:?}"))
}

/// Which tools the agent is given: every tool in `tools.json`, or the
/// eval subset in `tools-subset.json`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolSet {
    All,
    Subset,
    /// The two-step fixture set's own four tools
    /// (`fixtures/two-step/tools.json`), no subsetting.
    TwoStep,
}

/// The two-step fixture set's fixed tool list.
pub fn load_two_step_tools(fixtures_dir: impl AsRef<Path>) -> Result<Vec<Tool>> {
    load_mcp_tools(fixtures_dir.as_ref().join("tools.json"))
}

fn load_mcp_tools(path: impl AsRef<Path>) -> Result<Vec<Tool>> {
    let path = path.as_ref();
    let raw: Vec<Value> = serde_json::from_str(
        &std::fs::read_to_string(path).with_context(|| format!("reading tools from {path:?}"))?,
    )?;
    tools_from_mcp(&raw).with_context(|| format!("reading tools from {path:?}"))
}

pub fn load_all_tools(fixtures_dir: impl AsRef<Path>) -> Result<Vec<Tool>> {
    load_mcp_tools(fixtures_dir.as_ref().join("tools.json"))
}

/// Select the tools for a `ToolSet`, in `tools-subset.json`'s own order for
/// `Subset` (the listing tool comes first; the model's greedy output
/// depends on tool order, so this order is not arbitrary).
pub fn select_tools(
    all: &[Tool],
    subset: ToolSet,
    fixtures_dir: impl AsRef<Path>,
) -> Result<Vec<Tool>> {
    match subset {
        ToolSet::All => Ok(all.to_vec()),
        ToolSet::TwoStep => {
            anyhow::bail!(
                "ToolSet::TwoStep has no subset of a Sonos tool set; use load_two_step_tools"
            )
        }
        ToolSet::Subset => {
            let subset = load_mcp_tools(fixtures_dir.as_ref().join("tools-subset.json"))?;
            subset
                .iter()
                .map(|t| {
                    all.iter()
                        .find(|a| a.function.name == t.function.name)
                        .cloned()
                        .ok_or_else(|| {
                            anyhow::anyhow!(
                                "tools-subset.json tool `{}` not found in the full tool set",
                                t.function.name
                            )
                        })
                })
                .collect()
        }
    }
}

fn is_mutating(name: &str) -> bool {
    !name.starts_with("get_")
}

fn parse_mcp_result(result: &Value) -> Value {
    result
        .get("content")
        .and_then(Value::as_array)
        .and_then(|items| items.first())
        .and_then(|item| item.get("text"))
        .and_then(Value::as_str)
        .and_then(|text| serde_json::from_str(text).ok())
        .unwrap_or_else(|| result.clone())
}

fn valid_group_ids(households: Option<&Value>) -> std::collections::HashSet<String> {
    let Some(parsed) = households else {
        return Default::default();
    };
    let parsed = parse_mcp_result(parsed);
    parsed["households"]
        .as_array()
        .into_iter()
        .flatten()
        .flat_map(|h| h["groups"].as_array().cloned().unwrap_or_default())
        .filter_map(|g| g["groupId"].as_str().map(|s| s.to_string()))
        .collect()
}

#[derive(Debug, Clone)]
pub struct CaseResult {
    pub id: String,
    pub correct: bool,
    pub skipped: bool,
    pub reason: String,
    pub steps: usize,
    pub calls_made: Vec<ToolCall>,
    /// The run's final text answer, when it ended in one.
    pub final_text: Option<String>,
    pub total_ms: f64,
    pub retries: usize,
    pub tool_errors: usize,
    /// Token and time accounting of each step, in order.
    pub step_timings: Vec<StepTiming>,
}

const MAX_TOOL_CALLS: usize = 6;

fn args_match(expected: &Value, made: &Value) -> bool {
    match (expected, made) {
        (Value::Object(exp), Value::Object(got)) => exp.iter().all(|(k, v)| got.get(k) == Some(v)),
        _ => expected == made,
    }
}

fn values_match_ci(expected: &Value, got: &Value) -> bool {
    match (expected, got) {
        (Value::String(e), Value::String(g)) => e.to_lowercase() == g.to_lowercase(),
        _ => expected == got,
    }
}

fn args_match_ci(expected: &Value, made: &Value) -> bool {
    match (expected, made) {
        (Value::Object(exp), Value::Object(got)) => exp
            .iter()
            .all(|(k, v)| got.get(k).is_some_and(|g| values_match_ci(v, g))),
        _ => values_match_ci(expected, made),
    }
}

/// Why the final text answer fails the case's `answer_contains_*`, if it
/// does.
fn answer_check(case: &EvalCase, final_text: Option<&str>) -> Option<String> {
    let text = final_text.unwrap_or("").to_lowercase();
    let has = |s: &String| text.contains(&s.to_lowercase());
    if !case.answer_contains_any.is_empty() && !case.answer_contains_any.iter().any(has) {
        return Some(format!(
            "final answer did not contain any of {:?}: {:?}",
            case.answer_contains_any, final_text
        ));
    }
    if !case.answer_contains_all.iter().all(has) {
        return Some(format!(
            "final answer did not contain all of {:?}: {:?}",
            case.answer_contains_all, final_text
        ));
    }
    None
}

/// [`score`] without the final answer: for cases with no
/// `answer_contains_*`.
pub fn score(
    case: &EvalCase,
    calls_made: &[ToolCall],
    households: Option<&Value>,
    ended_in_final: bool,
) -> (bool, String) {
    score_with_answer(case, calls_made, households, ended_in_final, None)
}

/// Score `calls_made` (in order) against `case.expected`, then
/// `final_text` against the case's `answer_contains_*`. `is_mutating`
/// falls back to the `get_`-prefix heuristic since this fixture tool set
/// carries no MCP `annotations`.
pub fn score_with_answer(
    case: &EvalCase,
    calls_made: &[ToolCall],
    households: Option<&Value>,
    ended_in_final: bool,
    final_text: Option<&str>,
) -> (bool, String) {
    let (ok, reason) = score_calls(case, calls_made, households, ended_in_final);
    if !ok {
        return (ok, reason);
    }
    match answer_check(case, final_text) {
        Some(why) => (false, why),
        None => (ok, reason),
    }
}

fn score_calls(
    case: &EvalCase,
    calls_made: &[ToolCall],
    households: Option<&Value>,
    ended_in_final: bool,
) -> (bool, String) {
    if calls_made.len() > MAX_TOOL_CALLS {
        return (
            false,
            format!(
                "exceeded max steps: {} tool calls made (limit {MAX_TOOL_CALLS})",
                calls_made.len()
            ),
        );
    }

    match case.accept {
        Accept::Exact => {
            if calls_made.len() != case.expected.len() {
                return (
                    false,
                    format!(
                        "exact mode: expected {} call(s), got {}",
                        case.expected.len(),
                        calls_made.len()
                    ),
                );
            }
            for (i, expected) in case.expected.iter().enumerate() {
                let made = &calls_made[i];
                if made.name != expected.name || made.arguments != expected.args {
                    return (
                        false,
                        format!(
                            "exact mode: call {i} mismatch: expected {}({}), got {}({})",
                            expected.name, expected.args, made.name, made.arguments
                        ),
                    );
                }
            }
            (true, "matched expected sequence exactly".to_string())
        }
        Accept::Subset => {
            let required: Vec<&ExpectedCall> = case
                .expected
                .iter()
                .filter(|c| !c.name.starts_with("get_"))
                .collect();
            if !required.is_empty() {
                let mut cursor = 0usize;
                for expected in &required {
                    match calls_made[cursor..].iter().position(|c| {
                        c.name == expected.name && args_match(&expected.args, &c.arguments)
                    }) {
                        Some(offset) => cursor += offset + 1,
                        None => {
                            return (
                                false,
                                format!(
                                    "missing required call {}({}) in order after step {cursor}",
                                    expected.name, expected.args
                                ),
                            )
                        }
                    }
                }
                (
                    true,
                    "all required (mutating) calls made in order with correct args".to_string(),
                )
            } else {
                let Some(last) = case.expected.last() else {
                    return (false, "subset mode: the case expects no call".to_string());
                };
                if calls_made
                    .iter()
                    .any(|c| c.name == last.name && args_match(&last.args, &c.arguments))
                {
                    (true, "final read made with correct args".to_string())
                } else {
                    (
                        false,
                        format!(
                            "final read {}({}) never made with correct args",
                            last.name, last.args
                        ),
                    )
                }
            }
        }
        Accept::NoMutation => {
            if !ended_in_final {
                return (
                    false,
                    "no_mutation mode: run did not end in a final text answer".to_string(),
                );
            }
            match calls_made.iter().find(|c| is_mutating(&c.name)) {
                None => (
                    true,
                    "no mutating call made; final text answer given".to_string(),
                ),
                Some(c) => (
                    false,
                    format!(
                        "no_mutation mode: mutating call {}({}) was made",
                        c.name, c.arguments
                    ),
                ),
            }
        }
        Accept::AnyPlay => {
            let group_ids = valid_group_ids(households);
            match calls_made.iter().find(|c| {
                c.name.starts_with("play_")
                    && is_mutating(&c.name)
                    && c.arguments
                        .get("group_id")
                        .and_then(|v| v.as_str())
                        .is_some_and(|id| group_ids.contains(id))
            }) {
                Some(c) => (
                    true,
                    format!(
                        "any_play mode: {}({}) made with a valid group_id",
                        c.name, c.arguments
                    ),
                ),
                None => (
                    false,
                    "any_play mode: no play_* call with a valid group_id was made".to_string(),
                ),
            }
        }
        Accept::ExactArgs => {
            if calls_made.len() != case.expected.len() {
                return (
                    false,
                    format!(
                        "exact_args mode: expected {} call(s), got {}",
                        case.expected.len(),
                        calls_made.len()
                    ),
                );
            }
            for (i, expected) in case.expected.iter().enumerate() {
                let made = &calls_made[i];
                if made.name != expected.name || !args_match_ci(&expected.args, &made.arguments) {
                    return (
                        false,
                        format!(
                            "exact_args mode: call {i} mismatch: expected {}({}), got {}({})",
                            expected.name, expected.args, made.name, made.arguments
                        ),
                    );
                }
            }
            (true, "matched expected call sequence".to_string())
        }
    }
}

pub async fn run_case<G: Generator, C: ToolCaller + CallLog>(
    agent: &mut Agent<G>,
    tools: &[Tool],
    case: &EvalCase,
    caller: &mut C,
) -> CaseResult {
    let wall_start = std::time::Instant::now();
    let mut steps = 0usize;
    let mut retries = 0usize;
    let mut tool_errors = 0usize;
    let mut households: Option<Value> = None;
    let mut final_text: Option<String> = None;

    let mut step_timings: Vec<StepTiming> = Vec::new();
    let mut accumulate = |step: &crate::agent::Step| {
        retries += step.retries;
        tool_errors += step.tool_errors;
        step_timings.push(step.timing);
    };

    let mut outcome = agent.start(&case.utterance, tools).await;
    let (correct, reason) = loop {
        steps += 1;
        match outcome {
            StepOutcome::Final { text, step } => {
                accumulate(&step);
                let calls_made = to_tool_calls(caller.calls_seen());
                let scored =
                    score_with_answer(case, &calls_made, households.as_ref(), true, Some(&text));
                final_text = Some(text);
                break scored;
            }
            StepOutcome::NeedTools { calls, step } => {
                accumulate(&step);
                let mut results = Vec::with_capacity(calls.len());
                for call in &calls {
                    let result = call_or_error(caller, call);
                    if call.name == "get_households_and_groups_and_players" {
                        households = Some(result.clone());
                    }
                    results.push((call.call_id.clone(), result));
                }
                if caller.calls_seen().len() > MAX_TOOL_CALLS {
                    let calls_made = to_tool_calls(caller.calls_seen());
                    break score(case, &calls_made, households.as_ref(), false);
                }
                outcome = agent.provide_tool_results(results).await;
            }
            StepOutcome::ContextOverflow { needed, available } => {
                break (
                    false,
                    format!("context overflow: needed {needed} tokens, {available} available"),
                );
            }
            StepOutcome::Aborted => break (false, "aborted".to_string()),
            StepOutcome::Error { message, step } => {
                if let Some(step) = step {
                    accumulate(&step);
                }
                break (false, format!("agent error: {message}"));
            }
        }
    };

    let total_ms = wall_start.elapsed().as_secs_f64() * 1000.0;
    CaseResult {
        id: case.id.clone(),
        correct,
        skipped: false,
        reason,
        steps,
        calls_made: to_tool_calls(caller.calls_seen()),
        final_text,
        total_ms,
        retries,
        tool_errors,
        step_timings,
    }
}

fn to_tool_calls(calls_seen: &[(String, Value)]) -> Vec<ToolCall> {
    calls_seen
        .iter()
        .map(|(name, arguments)| ToolCall {
            name: name.clone(),
            arguments: arguments.clone(),
        })
        .collect()
}

fn skipped_result(case: &EvalCase) -> CaseResult {
    CaseResult {
        id: case.id.clone(),
        correct: false,
        skipped: true,
        reason: "subset_ok=false: not solvable under the 13-tool subset".to_string(),
        steps: 0,
        calls_made: Vec::new(),
        final_text: None,
        total_ms: 0.0,
        retries: 0,
        tool_errors: 0,
        step_timings: Vec::new(),
    }
}

#[derive(Debug, Clone)]
pub struct EvalReport {
    pub model: String,
    pub tool_set: ToolSet,
    pub tool_count: usize,
    pub backend: String,
    pub date: String,
    pub results: Vec<CaseResult>,
    pub correct_pct: f64,
    pub mean_steps: f64,
    pub mean_total_s: f64,
    pub total_retries: usize,
    pub total_tool_errors: usize,
    pub label: String,
    pub system_prompt: String,
    pub max_new_tokens: usize,
    pub max_steps: usize,
}

pub async fn run_all<G: Generator, C: ToolCaller + CallLog>(
    agent: &mut Agent<G>,
    tools: &[Tool],
    cases: &[EvalCase],
    tool_set: ToolSet,
    mut make_caller: impl FnMut() -> C,
    model: impl Into<String>,
    date: impl Into<String>,
) -> EvalReport {
    let mut results = Vec::with_capacity(cases.len());
    for case in cases {
        if tool_set == ToolSet::Subset && !case.subset_ok {
            results.push(skipped_result(case));
            continue;
        }
        let mut caller = make_caller();
        results.push(run_case(agent, tools, case, &mut caller).await);
    }

    let scored: Vec<&CaseResult> = results.iter().filter(|r| !r.skipped).collect();
    let n = scored.len().max(1) as f64;
    let correct_pct = scored.iter().filter(|r| r.correct).count() as f64 / n * 100.0;
    let mean_steps = scored.iter().map(|r| r.steps as f64).sum::<f64>() / n;
    let mean_total_s = scored.iter().map(|r| r.total_ms / 1000.0).sum::<f64>() / n;
    let total_retries = scored.iter().map(|r| r.retries).sum();
    let total_tool_errors = scored.iter().map(|r| r.tool_errors).sum();

    EvalReport {
        model: model.into(),
        tool_set,
        tool_count: tools.len(),
        backend: "native".to_string(),
        date: date.into(),
        results,
        correct_pct,
        mean_steps,
        mean_total_s,
        total_retries,
        total_tool_errors,
        label: String::new(),
        system_prompt: String::new(),
        max_new_tokens: 0,
        max_steps: 0,
    }
}

/// The case's steps summed.
fn case_timing(r: &CaseResult) -> StepTiming {
    let mut sum = StepTiming::default();
    for t in &r.step_timings {
        sum.add(t);
    }
    sum
}

pub fn render_markdown(report: &EvalReport) -> String {
    use std::fmt::Write;
    let mut out = String::new();

    let _ = writeln!(
        out,
        "# Agent eval, {}, unconstrained, {}",
        report.model, report.date
    );
    let _ = writeln!(out);
    let _ = writeln!(
        out,
        "## Run: {}, {} tools, {}, label={}",
        report.model, report.tool_count, report.backend, report.label
    );
    let _ = writeln!(out);
    let _ = writeln!(out, "- system: {:?}", report.system_prompt);
    let _ = writeln!(out, "- tool_count: {}", report.tool_count);
    let _ = writeln!(out, "- max_new_tokens: {}", report.max_new_tokens);
    let _ = writeln!(out, "- max_steps: {}", report.max_steps);
    let _ = writeln!(out);
    let _ = writeln!(
        out,
        "| id | correct | steps | total_s | retries | tool_errors | prompt_tok | think_tok | answer_tok | first_token_s | decode_s | capped | reason |"
    );
    let _ = writeln!(
        out,
        "|----|---------|-------|---------|---------|-------------|------------|-----------|------------|---------------|----------|--------|--------|"
    );
    for r in &report.results {
        if r.skipped {
            let _ = writeln!(
                out,
                "| {} | skipped | - | - | - | - | - | - | - | - | - | - | {} |",
                r.id, r.reason
            );
            continue;
        }
        let sum = case_timing(r);
        let _ = writeln!(
            out,
            "| {} | {} | {} | {:.3} | {} | {} | {} | {} | {} | {:.3} | {:.3} | {} | {} |",
            r.id,
            r.correct,
            r.steps,
            r.total_ms / 1000.0,
            r.retries,
            r.tool_errors,
            sum.prompt_tokens,
            sum.think_tokens,
            sum.answer_tokens,
            sum.first_token_ms / 1000.0,
            sum.decode_ms() / 1000.0,
            r.step_timings.iter().filter(|t| t.think_capped).count(),
            r.reason
        );
    }
    let _ = writeln!(out);
    let scored: Vec<&CaseResult> = report.results.iter().filter(|r| !r.skipped).collect();
    let skipped = report.results.len() - scored.len();
    let _ = writeln!(out, "**Summary**: correct {:.1}% ({} scored, {} skipped), mean_steps {:.2}, mean_total_s {:.3}, total_retries {}, total_tool_errors {}", report.correct_pct, scored.len(), skipped, report.mean_steps, report.mean_total_s, report.total_retries, report.total_tool_errors);
    let step_count: usize = scored.iter().map(|r| r.step_timings.len()).sum();
    if step_count > 0 {
        let mut all = StepTiming::default();
        for r in &scored {
            all.add(&case_timing(r));
        }
        let n = step_count as f64;
        let _ = writeln!(out);
        let _ = writeln!(
            out,
            "**Per step** ({step_count} steps): prompt_tok {:.1}, think_tok {:.1}, answer_tok {:.1}, first_token_s {:.3}, decode_s {:.3}, total_s {:.3}",
            all.prompt_tokens as f64 / n,
            all.think_tokens as f64 / n,
            all.answer_tokens as f64 / n,
            all.first_token_ms / 1000.0 / n,
            all.decode_ms() / 1000.0 / n,
            all.total_ms / 1000.0 / n
        );
    }

    let failed: Vec<&&CaseResult> = scored.iter().filter(|r| !r.correct).collect();
    if !failed.is_empty() {
        let _ = writeln!(out);
        let _ = writeln!(out, "**Failed cases, calls made and final answer**:");
        let _ = writeln!(out);
        for r in failed {
            let calls: Vec<String> = r
                .calls_made
                .iter()
                .map(|c| format!("`{}({})`", c.name, c.arguments))
                .collect();
            let calls = if calls.is_empty() {
                "no call".to_string()
            } else {
                calls.join(", ")
            };
            let answer = match &r.final_text {
                Some(t) => format!("{:?}", t.trim()),
                None => "no final answer".to_string(),
            };
            let _ = writeln!(out, "- {}: {calls}; answer: {answer}", r.id);
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::ChatTemplate;
    use crate::test_support::FakeGenerator;
    use serde_json::json;

    const QWEN25_TOOLS_TEMPLATE: &str =
        include_str!("../tests/fixtures/qwen25_chat_template.jinja");
    const REPO_ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");

    fn fixtures_dir() -> String {
        format!("{REPO_ROOT}/crates/agent/fixtures/sonos")
    }

    fn cases() -> Vec<EvalCase> {
        load_cases(format!("{REPO_ROOT}/eval/utterances.json")).unwrap()
    }

    fn case(id: &str) -> EvalCase {
        cases().into_iter().find(|c| c.id == id).unwrap()
    }

    fn call(name: &str, arguments: Value) -> ToolCall {
        ToolCall {
            name: name.to_string(),
            arguments,
        }
    }

    fn households() -> Value {
        let path = format!(
            "{}/results/get_households_and_groups_and_players.json",
            fixtures_dir()
        );
        serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
    }

    #[test]
    fn eval_set_has_46_cases_33_solvable_with_the_subset() {
        let cases = cases();
        assert_eq!(cases.len(), 46);
        assert_eq!(cases.iter().filter(|c| c.subset_ok).count(), 33);
        let mut ids: Vec<&str> = cases.iter().map(|c| c.id.as_str()).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), 46, "case ids are unique");

        let all = load_all_tools(fixtures_dir()).unwrap();
        let subset = select_tools(&all, ToolSet::Subset, fixtures_dir()).unwrap();
        assert_eq!(all.len(), 34);
        assert_eq!(subset.len(), 13);
        let has = |tools: &[Tool], name: &str| tools.iter().any(|t| t.function.name == name);
        for c in &cases {
            for e in &c.expected {
                assert!(has(&all, &e.name), "{}: unknown tool {}", c.id, e.name);
            }
            let solvable = c.expected.iter().all(|e| has(&subset, &e.name));
            assert_eq!(solvable, c.subset_ok, "{}: subset_ok", c.id);
        }
    }

    #[test]
    fn exact_mode_needs_the_same_calls() {
        let c = case("s02");
        let listing = call("get_households_and_groups_and_players", json!({}));
        assert!(score(&c, std::slice::from_ref(&listing), None, true).0);
        assert!(!score(&c, &[listing.clone(), listing], None, true).0);
        assert!(!score(&c, &[call("get_now_playing", json!({}))], None, true).0);
    }

    #[test]
    fn subset_mode_needs_the_mutating_calls_in_order() {
        let c = case("m01");
        let pause = call(
            "pause",
            json!({"group_id": "RINCON_KITCHEN01:1", "extra": 1}),
        );
        assert!(score(&c, std::slice::from_ref(&pause), None, true).0);
        assert!(
            !score(
                &c,
                &[call("pause", json!({"group_id": "RINCON_LIVING01:2"}))],
                None,
                true
            )
            .0
        );
        assert!(!score(&c, &[], None, true).0);
    }

    #[test]
    fn subset_mode_with_only_reads_needs_the_final_read() {
        let c = case("m07");
        let read = call(
            "get_group_volume",
            json!({"group_id": "RINCON_BEDROOM01:3"}),
        );
        assert!(score(&c, &[read], None, true).0);
        assert!(
            !score(
                &c,
                &[call("get_group_volume", json!({"group_id": "x"}))],
                None,
                true
            )
            .0
        );
    }

    #[test]
    fn no_mutation_mode_needs_a_text_answer_and_no_mutating_call() {
        let c = case("m21");
        let listing = call("get_households_and_groups_and_players", json!({}));
        assert!(score(&c, std::slice::from_ref(&listing), None, true).0);
        assert!(!score(&c, std::slice::from_ref(&listing), None, false).0);
        assert!(
            !score(
                &c,
                &[call("pause", json!({"group_id": "garage"}))],
                None,
                true
            )
            .0
        );
    }

    #[test]
    fn any_play_mode_needs_a_known_group_id() {
        let c = case("m22");
        let households = households();
        let play = |group: &str| {
            call(
                "play_sonos_favorite",
                json!({"group_id": group, "favorite_id": "FV:1", "shuffle": false}),
            )
        };
        assert!(score(&c, &[play("RINCON_KITCHEN01:1")], Some(&households), true).0);
        assert!(!score(&c, &[play("garage")], Some(&households), true).0);
        assert!(!score(&c, &[play("RINCON_KITCHEN01:1")], None, true).0);
        let mcp_shaped = json!({"content": [{"type": "text", "text": households.to_string()}]});
        assert!(score(&c, &[play("RINCON_KITCHEN01:1")], Some(&mcp_shaped), true).0);
    }

    #[test]
    fn too_many_calls_fail_any_mode() {
        let c = case("m01");
        let pause = call("pause", json!({"group_id": "RINCON_KITCHEN01:1"}));
        let calls = vec![pause; MAX_TOOL_CALLS + 1];
        assert!(!score(&c, &calls, None, true).0);
    }

    #[test]
    fn run_case_scores_a_scripted_run() {
        let generator = FakeGenerator::script(&[
            "<tool_call>\n{\"name\": \"get_households_and_groups_and_players\"}\n</tool_call>",
            "<tool_call>\n{\"name\": \"pause\", \"arguments\": {\"group_id\": \"RINCON_KITCHEN01:1\"}}\n</tool_call>",
            "The kitchen is paused.",
        ]);
        let mut agent = Agent::new(
            ChatTemplate::new(QWEN25_TOOLS_TEMPLATE),
            generator,
            "You are a helpful home assistant.",
            64,
        );
        let all = load_all_tools(fixtures_dir()).unwrap();
        let tools = select_tools(&all, ToolSet::Subset, fixtures_dir()).unwrap();
        let mut caller = FixtureCaller::new(format!("{}/results", fixtures_dir()));
        let result = pollster::block_on(run_case(&mut agent, &tools, &case("m01"), &mut caller));
        assert!(result.correct, "{}", result.reason);
        assert_eq!(result.steps, 3);
        assert_eq!(result.calls_made.len(), 2);
        assert_eq!(result.tool_errors, 0);
    }

    fn two_step_cases() -> Vec<EvalCase> {
        load_cases(format!(
            "{REPO_ROOT}/crates/agent/fixtures/two-step/cases.json"
        ))
        .unwrap()
    }

    #[test]
    fn two_step_set_has_8_cases_and_4_tools() {
        let cases = two_step_cases();
        assert_eq!(cases.len(), 8);
        assert!(cases.iter().all(|c| c.accept == Accept::ExactArgs));
        let tools =
            load_two_step_tools(format!("{REPO_ROOT}/crates/agent/fixtures/two-step")).unwrap();
        assert_eq!(tools.len(), 4);
        for c in &cases {
            for e in &c.expected {
                assert!(tools.iter().any(|t| t.function.name == e.name), "{}", c.id);
            }
        }
    }

    #[test]
    fn exact_args_ignores_extra_keys_and_case_in_strings() {
        let mut c = case("s01");
        c.accept = Accept::ExactArgs;
        c.expected = vec![ExpectedCall {
            name: "get_weather".into(),
            args: json!({"city": "Lyon"}),
        }];
        c.answer_contains_any = vec!["17".into(), "rain".into()];
        let made = [call(
            "get_weather",
            json!({"city": "LYON", "unit": "celsius"}),
        )];
        let ok = score_with_answer(&c, &made, None, true, Some("It's 17C and light rain."));
        assert!(ok.0, "{}", ok.1);
        assert!(!score_with_answer(&c, &made, None, true, Some("No idea.")).0);
        let wrong = [call("get_weather", json!({"city": "Paris"}))];
        assert!(!score_with_answer(&c, &wrong, None, true, Some("17C")).0);
    }

    #[test]
    fn answer_contains_all_needs_every_substring() {
        let mut c = case("s01");
        c.accept = Accept::ExactArgs;
        c.expected.clear();
        c.answer_contains_all = vec!["bathroom".into()];
        c.answer_contains_any = vec!["no".into(), "not".into()];
        let none: [ToolCall; 0] = [];
        assert!(score_with_answer(&c, &none, None, true, Some("There is no bathroom.")).0);
        assert!(!score_with_answer(&c, &none, None, true, Some("There is a bathroom.")).0);
        assert!(!score_with_answer(&c, &none, None, true, Some("No such room.")).0);
    }

    #[test]
    fn two_step_caller_answers_from_the_arguments() {
        let mut caller = TwoStepCaller::new();
        assert_eq!(
            caller.call("get_location", &json!({})).unwrap(),
            json!({"city": "Lyon", "country": "FR"})
        );
        let w = caller
            .call(
                "get_weather",
                &json!({"city": "Lyon", "unit": "fahrenheit"}),
            )
            .unwrap();
        assert_eq!(w["temp"], 63);
        let bad = caller
            .call("set_volume", &json!({"room_id": "bathroom", "level": 20}))
            .unwrap();
        assert_eq!(bad["error"], "unknown room_id");
        assert!(caller.call("nope", &json!({})).is_err());
        assert_eq!(caller.calls_seen().len(), 4);
    }

    #[test]
    fn run_case_reports_each_steps_timing() {
        let generator = FakeGenerator::script(&[
            "<think>\nlist first\n</think>\n\n<tool_call>\n{\"name\": \"get_households_and_groups_and_players\"}\n</tool_call>",
            "<think>\ndone\n</think>\n\nAll good.",
        ]);
        let mut agent = Agent::new(
            ChatTemplate::new(QWEN25_TOOLS_TEMPLATE),
            generator,
            "You are a helpful assistant.",
            256,
        );
        let all = load_all_tools(fixtures_dir()).unwrap();
        let tools = select_tools(&all, ToolSet::Subset, fixtures_dir()).unwrap();
        let mut caller = FixtureCaller::new(format!("{}/results", fixtures_dir()));
        let result = pollster::block_on(run_case(&mut agent, &tools, &case("s02"), &mut caller));
        assert!(result.correct, "{}", result.reason);
        assert_eq!(result.step_timings.len(), 2);
        assert!(result.step_timings.iter().all(|t| t.think_tokens > 0));
        let md = render_markdown(&EvalReport {
            model: "m".into(),
            tool_set: ToolSet::Subset,
            tool_count: tools.len(),
            backend: "native".into(),
            date: "d".into(),
            results: vec![result],
            correct_pct: 100.0,
            mean_steps: 2.0,
            mean_total_s: 0.0,
            total_retries: 0,
            total_tool_errors: 0,
            label: String::new(),
            system_prompt: String::new(),
            max_new_tokens: 0,
            max_steps: 0,
        });
        assert!(md.contains("**Per step** (2 steps)"), "{md}");
    }
}
