//! Native CLI: runs the Sonos MCP tool-calling eval set
//! (`eval/utterances.json`) against a GGUF model and writes a markdown
//! results table to stdout.
//!
//! ```sh
//! LEAN_GGUF_QWEN25_3B=/path/to/qwen2.5-3b-instruct-q4_0.gguf \
//! LEAN_TOKENIZER_DIR_QWEN25_3B=/path/to/Qwen2.5-3B-Instruct \
//! cargo run -p agent --example agent_eval --release
//! ```

use agent::agent::{Agent, AgentChatTemplate, FixtureCaller};
use agent::backend::best_generator;
use agent::eval::{load_all_tools, load_cases, render_markdown, run_all, select_tools, ToolSet};

fn main() -> anyhow::Result<()> {
    let gguf_path = std::env::var("LEAN_GGUF_QWEN25_3B").expect("set LEAN_GGUF_QWEN25_3B");
    let tokenizer_dir =
        std::env::var("LEAN_TOKENIZER_DIR_QWEN25_3B").expect("set LEAN_TOKENIZER_DIR_QWEN25_3B");
    let repo_root = std::env::var("AGENT_EVAL_REPO_ROOT")
        .unwrap_or_else(|_| concat!(env!("CARGO_MANIFEST_DIR"), "/..").to_string());

    let tokenizer = tokenizers::Tokenizer::from_file(format!("{tokenizer_dir}/tokenizer.json"))
        .expect("loading tokenizer.json");
    let cfg_json = std::fs::read_to_string(format!("{tokenizer_dir}/tokenizer_config.json"))
        .expect("reading tokenizer_config.json");
    let template =
        AgentChatTemplate::from_tokenizer_config_json(&cfg_json).expect("compiling chat_template");

    let fixtures_dir = format!("{repo_root}/crates/agent/fixtures/sonos");
    let results_dir = format!("{fixtures_dir}/results");
    let all_tools = load_all_tools(&fixtures_dir)?;
    let tools = select_tools(&all_tools, ToolSet::Twelve, &fixtures_dir)?;
    let cases = load_cases(format!("{repo_root}/eval/utterances.json"))?;

    let system_prompt = "You are a helpful home assistant with access to Sonos speaker controls.";
    let max_new_tokens = 256usize;
    let max_steps = 6usize;

    let generator = best_generator(&gguf_path)?;
    let caller = FixtureCaller::new(results_dir.clone()); // replaced per case by run_all
    let mut agent = Agent::new(
        template,
        tokenizer,
        generator,
        caller,
        system_prompt,
        max_new_tokens,
    );
    agent.set_max_steps(max_steps);

    let date = std::env::var("AGENT_EVAL_DATE").unwrap_or_else(|_| "2026-10-08".to_string());
    let mut report = run_all(
        &mut agent,
        &tools,
        &cases,
        ToolSet::Twelve,
        &results_dir,
        "Qwen2.5-3B-Instruct-q4_0.gguf (lean, capability-selected engine)",
        date,
    );
    report.label = "base43".to_string();
    report.system_prompt = system_prompt.to_string();
    report.max_new_tokens = max_new_tokens;
    report.max_steps = max_steps;

    print!("{}", render_markdown(&report));
    Ok(())
}
