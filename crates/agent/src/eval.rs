//! Offline Sonos MCP tool-call eval: scores an agent transcript against a
//! fixed set of utterance cases, each with an expected call sequence and
//! an acceptance mode. The agent run here is unconstrained (no
//! schema-constrained decoding), so `CaseResult`/`EvalReport` carry only
//! steps, time, retries and tool_errors.
//!
//! Native only (`std::fs`, `std::time::Instant`); not part of the wasm
//! build.

use crate::agent::{
    call_or_error, tools_from_mcp, Agent, Generator, StepOutcome, Tool, ToolCall, ToolCaller,
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
}

/// A [`ToolCaller`] that serves canned JSON results from
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
    pub total_ms: f64,
    pub retries: usize,
    pub tool_errors: usize,
}

const MAX_TOOL_CALLS: usize = 6;

fn args_match(expected: &Value, made: &Value) -> bool {
    match (expected, made) {
        (Value::Object(exp), Value::Object(got)) => exp.iter().all(|(k, v)| got.get(k) == Some(v)),
        _ => expected == made,
    }
}

/// Score `calls_made` (in order) against `case.expected`. `is_mutating`
/// falls back to the `get_`-prefix heuristic since this fixture tool set
/// carries no MCP `annotations`.
pub fn score(
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
    }
}

pub async fn run_case<G: Generator>(
    agent: &mut Agent<G>,
    tools: &[Tool],
    case: &EvalCase,
    caller: &mut FixtureCaller,
) -> CaseResult {
    let wall_start = std::time::Instant::now();
    let mut steps = 0usize;
    let mut retries = 0usize;
    let mut tool_errors = 0usize;
    let mut households: Option<Value> = None;

    let mut accumulate = |step: &crate::agent::Step| {
        retries += step.retries;
        tool_errors += step.tool_errors;
    };

    let mut outcome = agent.start(&case.utterance, tools).await;
    let (correct, reason) = loop {
        steps += 1;
        match outcome {
            StepOutcome::Final { step, .. } => {
                accumulate(&step);
                let calls_made = to_tool_calls(&caller.calls_seen);
                break score(case, &calls_made, households.as_ref(), true);
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
                if caller.calls_seen.len() > MAX_TOOL_CALLS {
                    let calls_made = to_tool_calls(&caller.calls_seen);
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
        calls_made: to_tool_calls(&caller.calls_seen),
        total_ms,
        retries,
        tool_errors,
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
        total_ms: 0.0,
        retries: 0,
        tool_errors: 0,
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

pub async fn run_all<G: Generator>(
    agent: &mut Agent<G>,
    tools: &[Tool],
    cases: &[EvalCase],
    tool_set: ToolSet,
    results_dir: impl AsRef<Path>,
    model: impl Into<String>,
    date: impl Into<String>,
) -> EvalReport {
    let results_dir = results_dir.as_ref();
    let mut results = Vec::with_capacity(cases.len());
    for case in cases {
        if tool_set == ToolSet::Subset && !case.subset_ok {
            results.push(skipped_result(case));
            continue;
        }
        let mut caller = FixtureCaller::new(results_dir);
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

pub fn render_markdown(report: &EvalReport) -> String {
    use std::fmt::Write;
    let mut out = String::new();

    let _ = writeln!(
        out,
        "# Sonos MCP agent eval, {}, unconstrained, {}",
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
        "| id | correct | steps | total_s | retries | tool_errors | reason |"
    );
    let _ = writeln!(
        out,
        "|----|---------|-------|---------|---------|-------------|--------|"
    );
    for r in &report.results {
        if r.skipped {
            let _ = writeln!(out, "| {} | skipped | - | - | - | - | {} |", r.id, r.reason);
            continue;
        }
        let _ = writeln!(
            out,
            "| {} | {} | {} | {:.3} | {} | {} | {} |",
            r.id,
            r.correct,
            r.steps,
            r.total_ms / 1000.0,
            r.retries,
            r.tool_errors,
            r.reason
        );
    }
    let _ = writeln!(out);
    let scored = report.results.iter().filter(|r| !r.skipped).count();
    let skipped = report.results.len() - scored;
    let _ = writeln!(out, "**Summary**: correct {:.1}% ({} scored, {} skipped), mean_steps {:.2}, mean_total_s {:.3}, total_retries {}, total_tool_errors {}", report.correct_pct, scored, skipped, report.mean_steps, report.mean_total_s, report.total_retries, report.total_tool_errors);

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
}
