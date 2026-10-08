//! End-to-end smoke test of the agent loop against the real
//! Qwen2.5-3B-Instruct model on lean's native CPU engine: the model must
//! pick the one tool it's given and call it, then answer in text once it
//! has the result. Doesn't assert on the wording of the final answer
//! (that's a model-quality question, not a correctness one); see
//! `docs/runs/2026-10-07-mcp-agent-lean.md` for scored eval runs.
//!
//! Ignored by default (needs the GGUF + tokenizer files on disk):
//!
//! ```sh
//! LEAN_GGUF_QWEN25_3B=/path/to/qwen2.5-3b-instruct-q4_0.gguf \
//! LEAN_TOKENIZER_DIR_QWEN25_3B=/path/to/Qwen2.5-3B-Instruct \
//! cargo test -p agent --release -- --ignored agent_live
//! ```

use agent::agent::{Agent, AgentChatTemplate, ParsedOutput, ToolCaller};
use agent::backend::CpuGenerator;
use serde_json::{json, Value};

struct WeatherCaller;

impl ToolCaller for WeatherCaller {
    fn call(&mut self, name: &str, args: &Value) -> anyhow::Result<Value> {
        assert_eq!(name, "get_weather");
        assert_eq!(args.get("city").and_then(Value::as_str), Some("Paris"));
        Ok(json!({"temp_c": 14, "condition": "cloudy"}))
    }
}

#[test]
#[ignore = "needs LEAN_GGUF_QWEN25_3B (GGUF path) and LEAN_TOKENIZER_DIR_QWEN25_3B (dir with tokenizer.json + tokenizer_config.json) on disk; never committed to this repo"]
fn agent_cpu_live_calls_tool_then_answers() {
    let gguf_path =
        std::env::var("LEAN_GGUF_QWEN25_3B").expect("set LEAN_GGUF_QWEN25_3B to run this test");
    let tokenizer_dir = std::env::var("LEAN_TOKENIZER_DIR_QWEN25_3B")
        .expect("set LEAN_TOKENIZER_DIR_QWEN25_3B to run this test");

    let tokenizer = tokenizers::Tokenizer::from_file(format!("{tokenizer_dir}/tokenizer.json"))
        .expect("loading tokenizer.json");
    let cfg_json = std::fs::read_to_string(format!("{tokenizer_dir}/tokenizer_config.json"))
        .expect("reading tokenizer_config.json");
    let template =
        AgentChatTemplate::from_tokenizer_config_json(&cfg_json).expect("compiling chat_template");

    let tool = agent::agent::Tool::from_mcp(
        "get_weather",
        "Get the current weather for a city.",
        json!({"type": "object", "properties": {"city": {"type": "string"}}, "required": ["city"]}),
    );

    let generator = CpuGenerator::load(&gguf_path).expect("loading GGUF");
    let mut agent = Agent::new(
        template,
        tokenizer,
        generator,
        WeatherCaller,
        "You are a helpful assistant with access to tools. Use them when needed.",
        96,
    );

    let transcript = agent
        .run("What's the weather like in Paris right now?", &[tool], 4)
        .expect("agent run failed");

    assert!(!transcript.steps.is_empty());
    let first = &transcript.steps[0];
    match &first.parsed {
        ParsedOutput::ToolCalls(calls) => {
            assert_eq!(calls.len(), 1);
            assert_eq!(calls[0].name, "get_weather");
        }
        ParsedOutput::Text(text) => {
            panic!("expected a tool call on the first step, got text: {text:?}")
        }
    }
    assert!(
        !transcript.final_text.trim().is_empty(),
        "final answer was empty"
    );
    println!("final answer: {}", transcript.final_text);
}
