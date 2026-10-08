//! Native CLI: runs one utterance through the agent loop against a GGUF
//! model, printing each step.
//!
//! ```sh
//! cargo run -p agent --example agent_cli --release -- \
//!   --model /path/to/model.gguf \
//!   --tokenizer /path/to/tokenizer_dir \
//!   --tools fixtures/sonos/tools-12.json \
//!   --utterance "pause the kitchen"
//! ```
//!
//! Tool calls are answered from `<tools-dir>/results/<tool>.json` fixture
//! files next to the tools file (see `fixtures/sonos/results/`), same
//! shape the eval harness uses; there is no live MCP connection here.

use agent::agent::{Agent, AgentChatTemplate, FixtureCaller, StepOutcome, Tool, ToolCaller};
use agent::backend::best_generator;
use anyhow::{Context, Result};
use clap::Parser;
use serde_json::Value;

#[derive(Parser)]
struct Args {
    /// Path to a GGUF model file.
    #[arg(long)]
    model: String,
    /// Directory with tokenizer.json and tokenizer_config.json.
    #[arg(long)]
    tokenizer: String,
    /// Path to an MCP tools/list JSON array fixture.
    #[arg(long)]
    tools: String,
    /// The user's utterance.
    #[arg(long)]
    utterance: String,
    #[arg(long, default_value_t = 256)]
    max_new_tokens: usize,
    #[arg(long, default_value_t = 6)]
    max_steps: usize,
    #[arg(
        long,
        default_value = "You are a helpful assistant with access to tools."
    )]
    system_prompt: String,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let tokenizer = tokenizers::Tokenizer::from_file(format!("{}/tokenizer.json", args.tokenizer))
        .map_err(|e| anyhow::anyhow!("loading tokenizer.json: {e}"))?;
    let cfg_json = std::fs::read_to_string(format!("{}/tokenizer_config.json", args.tokenizer))
        .context("reading tokenizer_config.json")?;
    let template = AgentChatTemplate::from_tokenizer_config_json(&cfg_json)?;

    let raw_tools: Vec<Value> = serde_json::from_str(
        &std::fs::read_to_string(&args.tools).with_context(|| format!("reading {}", args.tools))?,
    )?;
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

    let results_dir = std::path::Path::new(&args.tools)
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."))
        .join("results");
    // One FixtureCaller to satisfy `Agent`'s generic `ToolCaller` type
    // parameter (unused: this CLI drives the loop step-wise and never
    // calls `Agent::run`, so `self.caller` inside `Agent` is never
    // invoked) and a second, actually used, to answer tool calls below.
    let mut tool_runner = FixtureCaller::new(results_dir.clone());

    let generator = best_generator(&args.model)?;
    let mut agent = Agent::new(
        template,
        tokenizer,
        generator,
        FixtureCaller::new(results_dir),
        args.system_prompt,
        args.max_new_tokens,
    );
    agent.set_max_steps(args.max_steps);

    let mut outcome = agent.start(&args.utterance, &tools);
    loop {
        match outcome {
            StepOutcome::Final { text, .. } => {
                println!("step: final answer");
                println!("  text: {text}");
                break;
            }
            StepOutcome::NeedTools { calls, .. } => {
                let mut results = Vec::with_capacity(calls.len());
                for call in &calls {
                    println!("step: tool call {}({})", call.name, call.arguments);
                    let result = tool_runner
                        .call(&call.name, &call.arguments)
                        .unwrap_or(Value::Null);
                    println!("  result: {result}");
                    results.push((call.call_id.clone(), result));
                }
                outcome = agent.provide_tool_results(results);
            }
            StepOutcome::Error { message, .. } => {
                anyhow::bail!("agent error: {message}");
            }
        }
    }
    Ok(())
}
