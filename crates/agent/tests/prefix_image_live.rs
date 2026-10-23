//! Prefix KV images of the agent's prompt prefix (the system prompt and the
//! tools) on the real Qwen3-1.7B model, with the Sonos tools subset: a
//! fresh agent answers an utterance; a second one prefills the prefix,
//! saves its image and answers; a third one restores the image and
//! answers. The three greedy replies must be equal. An image saved for one
//! tools list must be refused for another, and the refused agent must still
//! give the fresh reply. Prints prefill, save and restore times and the
//! time to the first token of each answer.
//!
//! Ignored by default (needs the GGUF + tokenizer files on disk):
//!
//! ```sh
//! LEAN_GGUF_QWEN3_1_7B=/path/to/Qwen3-1.7B-Q8_0.gguf \
//! LEAN_TOKENIZER_DIR_QWEN3_1_7B=/path/to/Qwen3-1.7B \
//! cargo test -p agent --release --test prefix_image_live -- --ignored --nocapture
//! ```

use std::fs::File;
use std::io::{BufReader, BufWriter, Write};
use std::time::Instant;

use agent::agent::{tools_from_mcp, Agent, StepOutcome, Thinking, Tool};
use agent::lean_generator::LeanGenerator;
use lean::{Backend, GgufSource, LeanError, LoadOptions, PrefixImageError, TokenizerSource};
use serde_json::Value;

const SYSTEM_PROMPT: &str = "You are a helpful assistant. Use the available tools to get information and to act; call tools as many times as needed, one after another, and never claim you lack data that a tool can provide.";
const UTTERANCE: &str = "pause the kitchen";

fn sonos_tools() -> Vec<Tool> {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/fixtures/sonos/tools-subset.json"
    );
    let raw: Vec<Value> = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    tools_from_mcp(&raw).unwrap()
}

/// The model's GGUF path and tokenizer directory, or `None` when the
/// environment does not name them.
fn model_files() -> Option<(String, String)> {
    Some((
        std::env::var("LEAN_GGUF_QWEN3_1_7B").ok()?,
        std::env::var("LEAN_TOKENIZER_DIR_QWEN3_1_7B").ok()?,
    ))
}

fn load_agent((gguf, dir): &(String, String)) -> Agent<LeanGenerator> {
    let template = lean_chat::read_chat_template(&format!("{dir}/tokenizer_config.json"))
        .expect("reading the chat template");
    let generator = pollster::block_on(LeanGenerator::load(LoadOptions {
        gguf: GgufSource::Path(gguf.clone()),
        tokenizer: TokenizerSource::Path(format!("{dir}/tokenizer.json")),
        max_ctx: 4096,
        backend: Backend::Auto,
    }))
    .expect("loading the model");
    let mut agent = Agent::new(template, generator, SYSTEM_PROMPT, 128);
    agent.set_thinking(Thinking::Off);
    agent
}

/// The raw reply of the turn's first step and its time to first token.
fn answer(agent: &mut Agent<LeanGenerator>, tools: &[Tool]) -> (String, f64) {
    match pollster::block_on(agent.start(UTTERANCE, tools)) {
        StepOutcome::NeedTools { step, .. } | StepOutcome::Final { step, .. } => {
            (step.generated_text, step.timing.first_token_ms)
        }
        other => panic!("unexpected outcome: {other:?}"),
    }
}

#[test]
#[ignore = "needs LEAN_GGUF_QWEN3_1_7B (GGUF path) and LEAN_TOKENIZER_DIR_QWEN3_1_7B (dir with tokenizer.json + tokenizer_config.json) on disk; never committed to this repo"]
fn restored_prefix_gives_the_fresh_reply() {
    let Some(files) = model_files() else {
        println!("skipped: LEAN_GGUF_QWEN3_1_7B and LEAN_TOKENIZER_DIR_QWEN3_1_7B are not set");
        return;
    };
    let tools = sonos_tools();
    let image_path = std::env::temp_dir().join(format!("agent-prefix-{}.bin", std::process::id()));

    let mut fresh = load_agent(&files);
    println!("backend: {}", fresh.generator().backend());
    let (fresh_reply, cold_ttft) = answer(&mut fresh, &tools);
    drop(fresh);

    let mut writer = load_agent(&files);
    let key = writer.prefix_key(&tools).unwrap();
    let t = Instant::now();
    let prefix_tokens = pollster::block_on(writer.prefill(&tools)).unwrap();
    let prefill_ms = t.elapsed().as_secs_f64() * 1000.0;
    let t = Instant::now();
    let mut out = BufWriter::new(File::create(&image_path).unwrap());
    let bytes = pollster::block_on(writer.save_prefix(&tools, &mut out)).unwrap();
    out.flush().unwrap();
    drop(out);
    let save_ms = t.elapsed().as_secs_f64() * 1000.0;
    let (writer_reply, writer_ttft) = answer(&mut writer, &tools);
    drop(writer);

    let mut reader = load_agent(&files);
    assert_eq!(reader.prefix_key(&tools).unwrap(), key);
    let fewer = &tools[..tools.len() - 1];
    assert_ne!(reader.prefix_key(fewer).unwrap(), key);
    let refused =
        reader.restore_prefix(fewer, &mut BufReader::new(File::open(&image_path).unwrap()));
    assert!(
        matches!(
            refused,
            Err(LeanError::PrefixImage(PrefixImageError::Stale("prefix")))
        ),
        "{refused:?}"
    );
    let t = Instant::now();
    reader
        .restore_prefix(
            &tools,
            &mut BufReader::new(File::open(&image_path).unwrap()),
        )
        .unwrap();
    let restore_ms = t.elapsed().as_secs_f64() * 1000.0;
    let (restored_reply, restored_ttft) = answer(&mut reader, &tools);
    std::fs::remove_file(&image_path).unwrap();

    println!("prefix tokens: {prefix_tokens}, image bytes: {bytes}, key: {key}");
    println!("cold time to first token: {cold_ttft:.0} ms");
    println!("prefill: {prefill_ms:.0} ms, save: {save_ms:.0} ms, time to first token after prefill: {writer_ttft:.0} ms");
    println!(
        "restore: {restore_ms:.0} ms, time to first token after restore: {restored_ttft:.0} ms"
    );
    println!("reply: {fresh_reply:?}");
    assert_eq!(writer_reply, fresh_reply);
    assert_eq!(restored_reply, fresh_reply);

    // A refused image leaves the agent as it was: the refusal above came
    // before the restore, and this one comes after an answer.
    let refused = reader.restore_prefix(fewer, &mut std::io::empty());
    assert!(
        matches!(
            refused,
            Err(LeanError::PrefixImage(PrefixImageError::NotAnImage))
        ),
        "{refused:?}"
    );
    assert_eq!(answer(&mut reader, &tools).0, fresh_reply);
}
