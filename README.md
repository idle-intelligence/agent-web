# agent-web

A tool-calling agent for Qwen2.5-Instruct models, running on the [lean](https://github.com/idle-intelligence/llm-web) inference engine.

lean is inference only: text in, text out, on a backend it picks by capability (WebGPU, then CPU threads, then a single CPU thread). Its chat layer, lean-chat, renders messages and tools into prompt text with the model's own chat template. Everything an agent needs on top of that, output parsing, the step-wise loop, and an eval harness, lives here instead. The loop reaches the model through one small trait (prompt text in, streamed text out, abort, a typed context-overflow error), so any engine that completes text can drive it.

## What it does

- Renders each step's prompt with the model's own chat template and the tools through lean-chat (for Qwen2.5-Instruct, `<tools>...</tools>` in the system turn and `<tool_call>{"name":..., "arguments":...}</tool_call>` in the model's output), byte-exact against `transformers.apply_chat_template(..., tools=...)`.
- Parses the model's tool calls out of its raw output, with a retry policy for malformed or repeated calls.
- Drives a step-wise loop: render prompt, generate, parse, hand back pending tool calls, accept results, repeat until a final text answer.
- Scores transcripts against a fixed set of utterance cases (a Sonos speaker-control tool set) under several acceptance modes (exact call sequence, required subset, no mutation, any valid play call).

## Crates

- `crates/agent`: the parser, loop, and eval library, plus `LeanGenerator`, the loop's generator on lean's engine. Depends on lean and lean-chat through a pinned git revision.
- `crates/agent-wasm`: wasm-bindgen bindings for the agent on lean's engine (WebGPU when the browser grants an adapter, otherwise the CPU).
- `examples/agent_cli.rs`: a native CLI that runs one utterance through the loop, answering tool calls from fixture JSON, and prints each step.
- `examples/agent_eval.rs`: a native CLI that runs the full eval set and prints a markdown results table.

## CLI examples

```sh
cargo run -p agent --example agent_cli --release -- \
  --model /path/to/qwen2.5-3b-instruct-q4_0.gguf \
  --tokenizer /path/to/Qwen2.5-3B-Instruct \
  --tools crates/agent/fixtures/sonos/tools-12.json \
  --utterance "pause the kitchen"
```

```sh
LEAN_GGUF_QWEN25_3B=/path/to/qwen2.5-3b-instruct-q4_0.gguf \
LEAN_TOKENIZER_DIR_QWEN25_3B=/path/to/Qwen2.5-3B-Instruct \
cargo run -p agent --example agent_eval --release
```

Both load the model with lean's capability ladder: the GPU when a wgpu adapter is available, otherwise the CPU forward pass on every core. Each step reuses the KV cache of the previous one for the prompt prefix they share, so the system prompt and tools are prefilled once per turn.

## Demo

A browser demo on the repo's GitHub Pages is planned; link to follow once it is built.

## Eval

43 utterance cases, a 12-tool Sonos speaker-control set, Qwen2.5-3B-Instruct (Q4_0 GGUF), lean's CPU engine, unconstrained decoding (no grammar-constrained output): 9.1% correct, run on 2026-10-07. See `docs/runs/2026-10-07-mcp-agent-lean.md` for the full run log, including a 0.5B-model follow-up and the baseline this was compared against.

## License

Not yet decided; no license file is included.
