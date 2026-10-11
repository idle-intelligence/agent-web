//! Native CLI: runs one utterance through the agent loop against a GGUF
//! model, printing each step.
//!
//! ```sh
//! cargo run -p agent --example agent_cli --release -- \
//!   --model /path/to/model.gguf \
//!   --tokenizer /path/to/tokenizer_dir \
//!   --tools fixtures/sonos/tools-subset.json \
//!   --utterance "pause the kitchen"
//! ```
//!
//! Tool calls are answered from `<tools-dir>/results/<tool>.json` fixture
//! files next to the tools file (see `fixtures/sonos/results/`), same
//! shape the eval harness uses; there is no live MCP connection here.
//!
//! The model runs on lean's engine, which picks its backend by capability
//! (GPU when a wgpu adapter is available, else the CPU forward pass on
//! every core). The model's raw output is streamed to stderr as it is
//! generated.

use std::io::Write;

use agent::agent::{Agent, FixtureCaller, StepOutcome, Tool, ToolCaller};
use agent::lean_generator::LeanGenerator;
use anyhow::{Context, Result};
use clap::Parser;
use lean::{Backend, GgufSource, LoadOptions, TokenizerSource};
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
    /// KV cache size in tokens: each step's prompt plus `max_new_tokens`
    /// must fit.
    #[arg(long, default_value_t = 4096)]
    max_ctx: u32,
    #[arg(
        long,
        default_value = "You are a helpful assistant with access to tools."
    )]
    system_prompt: String,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let template =
        lean_chat::read_chat_template(&format!("{}/tokenizer_config.json", args.tokenizer))?;

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

    let generator = pollster::block_on(LeanGenerator::load(LoadOptions {
        gguf: GgufSource::Path(args.model.clone()),
        tokenizer: TokenizerSource::Path(format!("{}/tokenizer.json", args.tokenizer)),
        max_ctx: args.max_ctx,
        backend: Backend::Auto,
    }))?;
    eprintln!("backend: {}", generator.backend());
    let mut agent = Agent::new(
        template,
        generator,
        FixtureCaller::new(results_dir),
        args.system_prompt,
        args.max_new_tokens,
    );
    agent.set_max_steps(args.max_steps);
    agent.set_on_text(|text| {
        eprint!("{text}");
        let _ = std::io::stderr().flush();
    });

    let mut outcome = pollster::block_on(agent.start(&args.utterance, &tools));
    loop {
        match outcome {
            StepOutcome::Final { text, .. } => {
                eprintln!();
                println!("step: final answer");
                println!("  text: {text}");
                break;
            }
            StepOutcome::NeedTools { calls, .. } => {
                eprintln!();
                let mut results = Vec::with_capacity(calls.len());
                for call in &calls {
                    println!("step: tool call {}({})", call.name, call.arguments);
                    let result = tool_runner
                        .call(&call.name, &call.arguments)
                        .unwrap_or(Value::Null);
                    println!("  result: {result}");
                    results.push((call.call_id.clone(), result));
                }
                outcome = pollster::block_on(agent.provide_tool_results(results));
            }
            StepOutcome::ContextOverflow { needed, available } => {
                anyhow::bail!("context overflow: needed {needed} tokens, {available} available (raise --max-ctx)");
            }
            StepOutcome::Aborted => anyhow::bail!("aborted"),
            StepOutcome::Error { message, .. } => {
                anyhow::bail!("agent error: {message}");
            }
        }
    }
    Ok(())
}
