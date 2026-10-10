//! Native CLI: runs the Sonos MCP tool-calling eval set
//! (`eval/utterances.json`) against a GGUF model and writes a markdown
//! results table to stdout.
//!
//! ```sh
//! LEAN_GGUF_QWEN25_3B=/path/to/qwen2.5-3b-instruct-q4_0.gguf \
//! LEAN_TOKENIZER_DIR_QWEN25_3B=/path/to/Qwen2.5-3B-Instruct \
//! cargo run -p agent --example agent_eval --release
//! ```
//!
//! The model runs on lean's engine, which picks its backend by capability.
//! `AGENT_EVAL_MAX_CTX` sets the KV cache size in tokens (default 4096).

use agent::agent::{Agent, FixtureCaller};
use agent::eval::{load_all_tools, load_cases, render_markdown, run_all, select_tools, ToolSet};
use agent::lean_generator::LeanGenerator;
use lean::{Backend, GgufSource, LoadOptions, TokenizerSource};

fn main() -> anyhow::Result<()> {
    let gguf_path = std::env::var("LEAN_GGUF_QWEN25_3B").expect("set LEAN_GGUF_QWEN25_3B");
    let tokenizer_dir =
        std::env::var("LEAN_TOKENIZER_DIR_QWEN25_3B").expect("set LEAN_TOKENIZER_DIR_QWEN25_3B");
    let repo_root = std::env::var("AGENT_EVAL_REPO_ROOT")
        .unwrap_or_else(|_| concat!(env!("CARGO_MANIFEST_DIR"), "/..").to_string());

    let max_ctx: u32 = std::env::var("AGENT_EVAL_MAX_CTX")
        .map(|s| s.parse().expect("AGENT_EVAL_MAX_CTX must be a number"))
        .unwrap_or(4096);
    let template =
        lean_chat::read_chat_template(&format!("{tokenizer_dir}/tokenizer_config.json"))?;

    let fixtures_dir = format!("{repo_root}/crates/agent/fixtures/sonos");
    let results_dir = format!("{fixtures_dir}/results");
    let all_tools = load_all_tools(&fixtures_dir)?;
    let tools = select_tools(&all_tools, ToolSet::Twelve, &fixtures_dir)?;
    let cases = load_cases(format!("{repo_root}/eval/utterances.json"))?;

    let system_prompt = "You are a helpful home assistant with access to Sonos speaker controls.";
    let max_new_tokens = 256usize;
    let max_steps = 6usize;

    let generator = pollster::block_on(LeanGenerator::load(LoadOptions {
        gguf: GgufSource::Path(gguf_path),
        tokenizer: TokenizerSource::Path(format!("{tokenizer_dir}/tokenizer.json")),
        max_ctx,
        backend: Backend::Auto,
    }))?;
    let backend = generator.backend();
    eprintln!("backend: {backend}");
    let caller = FixtureCaller::new(results_dir.clone()); // replaced per case by run_all
    let mut agent = Agent::new(template, generator, caller, system_prompt, max_new_tokens);
    agent.set_max_steps(max_steps);

    let date = std::env::var("AGENT_EVAL_DATE").unwrap_or_else(|_| "2026-10-08".to_string());
    let mut report = pollster::block_on(run_all(
        &mut agent,
        &tools,
        &cases,
        ToolSet::Twelve,
        &results_dir,
        "Qwen2.5-3B-Instruct-q4_0.gguf (lean, capability-selected engine)",
        date,
    ));
    report.backend = format!("native, {backend}");
    report.label = "base43".to_string();
    report.system_prompt = system_prompt.to_string();
    report.max_new_tokens = max_new_tokens;
    report.max_steps = max_steps;

    print!("{}", render_markdown(&report));
    Ok(())
}
