//! Byte-exact parity of the agent's prompts (`render_prompt`, lean-chat
//! with the model's own chat template) against
//! `transformers.apply_chat_template(..., tools=...)` itself, and
//! token-exact parity of their tokenization (no model forward pass needed,
//! this is a template/tokenizer check).
//!
//! See `reference/gen_fixture_agent_tools.py` for how the fixture was
//! generated.
//!
//! Ignored by default (needs the tokenizer files on disk, never committed):
//!
//! ```sh
//! LEAN_TOKENIZER_DIR_QWEN25_3B=/path/to/Qwen2.5-3B-Instruct \
//! cargo test -p agent --release -- --ignored fixture_parity_agent_tools
//! ```

use agent::agent::{render_prompt, Message, Tool};
use serde::Deserialize;
use serde_json::Value;

#[derive(Deserialize)]
struct Case {
    name: String,
    messages: Vec<Message>,
    tools: Vec<Tool>,
    rendered: String,
    input_ids: Vec<u32>,
}

#[derive(Deserialize)]
struct Fixture {
    cases: Vec<Case>,
}

#[test]
#[ignore = "needs LEAN_TOKENIZER_DIR_QWEN25_3B (dir with tokenizer.json + tokenizer_config.json) on disk; never committed to this repo"]
fn fixture_parity_agent_tools() {
    let tokenizer_dir = std::env::var("LEAN_TOKENIZER_DIR_QWEN25_3B")
        .expect("set LEAN_TOKENIZER_DIR_QWEN25_3B to run this test");

    let tokenizer = tokenizers::Tokenizer::from_file(format!("{tokenizer_dir}/tokenizer.json"))
        .expect("loading tokenizer.json");
    let template = lean_chat::read_chat_template(&format!("{tokenizer_dir}/tokenizer_config.json"))
        .expect("reading the chat template");

    let fixture_json = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/reference/fixture_agent_tools.json"
    ))
    .expect("reading fixture_agent_tools.json");
    let fixture: Fixture =
        serde_json::from_str(&fixture_json).expect("parsing fixture_agent_tools.json");

    for case in &fixture.cases {
        let rendered = render_prompt(&template, &case.messages, &case.tools)
            .unwrap_or_else(|e| panic!("case {}: render failed: {e:#}", case.name));
        assert_eq!(
            rendered, case.rendered,
            "case {}: rendered text mismatch",
            case.name
        );

        let ids = tokenizer
            .encode(rendered, false)
            .unwrap_or_else(|e| panic!("case {}: tokenizer encode failed: {e}", case.name))
            .get_ids()
            .to_vec();
        assert_eq!(
            ids, case.input_ids,
            "case {}: input_ids mismatch",
            case.name
        );
    }
}

/// `Message` reads either `content` shape the fixtures carry: a plain
/// string (every `system`/`user`/`tool` message) or `null` (an `assistant`
/// message that only has `tool_calls`, read as `""`, which the template
/// treats the same way).
#[test]
fn message_deserializes_null_content_and_string_tool_calls_arguments() {
    let json = r#"{"role": "assistant", "content": null, "tool_calls": [{"id": "call_0", "type": "function", "function": {"name": "get_now_playing", "arguments": "{}"}}]}"#;
    let msg: Message = serde_json::from_str(json).unwrap();
    assert_eq!(msg.content, "");
    let tool_calls = msg.tool_calls;
    assert_eq!(
        tool_calls[0].function.arguments,
        Value::String("{}".to_string())
    );
}
