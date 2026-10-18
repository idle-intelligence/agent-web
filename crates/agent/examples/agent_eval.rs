//! Native CLI: runs an eval case set against a GGUF model and writes a
//! markdown results table to stdout.
//!
//! Two fixture sets, selected by `--fixture-set`:
//! - `sonos` (default): `eval/utterances.json` against the Sonos MCP
//!   subset of tools (`fixtures/sonos/`).
//! - `two-step`: `fixtures/two-step/cases.json` against that set's own four
//!   tools, a small test of whether a model chains a discovery call into
//!   the action call it was for.
//!
//! ```sh
//! cargo run -p agent --example agent_eval --release -- \
//!   --model /path/to/Qwen3-1.7B-Q8_0.gguf \
//!   --tokenizer-dir /path/to/Qwen3-1.7B \
//!   --model-label "Qwen3-1.7B-Q8_0.gguf" \
//!   --thinking template --max-new-tokens 1024
//! ```
//!
//! The model runs on lean's engine, which picks its backend by capability.

use agent::agent::{Agent, Thinking};
use agent::eval::{
    load_all_tools, load_cases, load_two_step_tools, render_markdown, run_all, select_tools,
    FixtureCaller, ToolSet, TwoStepCaller,
};
use agent::lean_generator::LeanGenerator;
use clap::{Parser, ValueEnum};
use lean::{Backend, GgufSource, LoadOptions, TokenizerSource};

#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
enum FixtureSet {
    Sonos,
    TwoStep,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
enum SystemPromptVariant {
    /// The fixture set's own short system prompt.
    Default,
    /// Spells out repeated, chained tool calls and forbids claiming that
    /// data is missing when a tool can provide it.
    Clear,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
enum ThinkingArg {
    /// Render as the model's template does by default (Qwen3 thinks).
    Template,
    /// End the prompt with Qwen3's empty thinking block.
    Off,
}

#[derive(Parser)]
struct Args {
    /// Path to a GGUF model file.
    #[arg(long, env = "LEAN_GGUF")]
    model: String,
    /// Directory with tokenizer.json and tokenizer_config.json.
    #[arg(long, env = "LEAN_TOKENIZER_DIR")]
    tokenizer_dir: String,
    /// Model name for the report.
    #[arg(long)]
    model_label: String,
    #[arg(long, value_enum, default_value = "sonos")]
    fixture_set: FixtureSet,
    #[arg(long, value_enum, default_value = "default")]
    system_prompt: SystemPromptVariant,
    #[arg(long, value_enum, default_value = "template")]
    thinking: ThinkingArg,
    /// Caps a thinking block at this many tokens (see `Agent::set_think_cap`).
    #[arg(long)]
    think_cap: Option<usize>,
    #[arg(long, default_value_t = 256)]
    max_new_tokens: usize,
    #[arg(long, default_value_t = 6)]
    max_steps: usize,
    /// KV cache size in tokens.
    #[arg(long, default_value_t = 4096)]
    max_ctx: u32,
    #[arg(long, default_value = "2026-10-15")]
    date: String,
    /// Run only the cases with these ids (comma separated).
    #[arg(long, value_delimiter = ',')]
    only: Vec<String>,
    /// Writes the report as markdown to this path as well as stdout.
    #[arg(long)]
    out: Option<String>,
}

const CLEAR_SYSTEM_PROMPT: &str = "You are a helpful assistant. Use the available tools to get information and to act; call tools as many times as needed, one after another, and never claim you lack data that a tool can provide.";

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let repo_root = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
    let template =
        lean_chat::read_chat_template(&format!("{}/tokenizer_config.json", args.tokenizer_dir))?;

    let system_prompt = match (args.system_prompt, args.fixture_set) {
        (SystemPromptVariant::Clear, _) => CLEAR_SYSTEM_PROMPT,
        (SystemPromptVariant::Default, FixtureSet::Sonos) => {
            "You are a helpful home assistant with access to Sonos speaker controls."
        }
        (SystemPromptVariant::Default, FixtureSet::TwoStep) => {
            "You are a helpful assistant with access to tools."
        }
    };

    let generator = pollster::block_on(LeanGenerator::load(LoadOptions {
        gguf: GgufSource::Path(args.model.clone()),
        tokenizer: TokenizerSource::Path(format!("{}/tokenizer.json", args.tokenizer_dir)),
        max_ctx: args.max_ctx,
        backend: Backend::Auto,
    }))?;
    let backend = generator.backend();
    eprintln!("backend: {backend}");

    let mut agent = Agent::new(template, generator, system_prompt, args.max_new_tokens);
    agent.set_max_steps(args.max_steps);
    agent.set_thinking(match args.thinking {
        ThinkingArg::Template => Thinking::TemplateDefault,
        ThinkingArg::Off => Thinking::Off,
    });
    agent.set_think_cap(args.think_cap);

    let (label, mut report) = match args.fixture_set {
        FixtureSet::Sonos => {
            let fixtures_dir = format!("{repo_root}/crates/agent/fixtures/sonos");
            let results_dir = format!("{fixtures_dir}/results");
            let all_tools = load_all_tools(&fixtures_dir)?;
            let tools = select_tools(&all_tools, ToolSet::Subset, &fixtures_dir)?;
            let cases = load_cases(format!("{repo_root}/eval/utterances.json"))?;
            let cases = filter_cases(cases, &args.only);
            let report = pollster::block_on(run_all(
                &mut agent,
                &tools,
                &cases,
                ToolSet::Subset,
                || FixtureCaller::new(results_dir.clone()),
                args.model_label.clone(),
                args.date.clone(),
            ));
            ("sonos-subset", report)
        }
        FixtureSet::TwoStep => {
            let fixtures_dir = format!("{repo_root}/crates/agent/fixtures/two-step");
            let tools = load_two_step_tools(&fixtures_dir)?;
            let cases = load_cases(format!("{fixtures_dir}/cases.json"))?;
            let cases = filter_cases(cases, &args.only);
            let report = pollster::block_on(run_all(
                &mut agent,
                &tools,
                &cases,
                ToolSet::TwoStep,
                TwoStepCaller::new,
                args.model_label.clone(),
                args.date.clone(),
            ));
            ("two-step", report)
        }
    };
    report.backend = format!("native, {backend}");
    report.label = label.to_string();
    report.system_prompt = system_prompt.to_string();
    report.max_new_tokens = args.max_new_tokens;
    report.max_steps = args.max_steps;

    let markdown = render_markdown(&report);
    if let Some(path) = &args.out {
        std::fs::write(path, &markdown)?;
    }
    print!("{markdown}");
    Ok(())
}

fn filter_cases(cases: Vec<agent::eval::EvalCase>, only: &[String]) -> Vec<agent::eval::EvalCase> {
    if only.is_empty() {
        cases
    } else {
        cases.into_iter().filter(|c| only.contains(&c.id)).collect()
    }
}
