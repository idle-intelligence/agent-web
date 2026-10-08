# agent-web

A tool-calling agent for Qwen2.5-Instruct models, running on the [lean](https://github.com/idle-intelligence/llm-web) inference engine.

lean is inference only: it loads weights, tokenizes, runs the model forward pass, and picks a backend (WebGPU, then CPU threads, then a single CPU thread) by capability. Everything an agent needs on top of that, the tool-calling chat template, output parsing, the step-wise loop, and an eval harness, lives here instead.

## What it does

- Renders Qwen2.5-Instruct's own tools chat template (`<tools>...</tools>` in the system turn, `<tool_call>{"name":..., "arguments":...}</tool_call>` in the model's output), token-exact against `transformers.apply_chat_template(..., tools=...)`.
- Parses the model's tool calls out of its raw output, with a retry policy for malformed or repeated calls.
- Drives a step-wise loop: render prompt, generate, parse, hand back pending tool calls, accept results, repeat until a final text answer.
- Scores transcripts against a fixed set of utterance cases (a Sonos speaker-control tool set) under several acceptance modes (exact call sequence, required subset, no mutation, any valid play call).

## Crates

- `crates/agent`: the template, parser, loop, and eval library. Depends on lean through a pinned git revision.
- `crates/agent-wasm`: wasm-bindgen bindings for the agent, running on lean's native CPU engine.
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

Both pick lean's GPU engine when a wgpu adapter is available and fall back to the CPU engine otherwise, the same capability-based choice lean's own CLI makes.

## Demo

A browser demo on the repo's GitHub Pages is planned; link to follow once it is built.

## Eval

43 utterance cases, a 12-tool Sonos speaker-control set, Qwen2.5-3B-Instruct (Q4_0 GGUF), lean's CPU engine, unconstrained decoding (no grammar-constrained output): 9.1% correct, run on 2026-10-07. See `docs/runs/2026-10-07-mcp-agent-lean.md` for the full run log, including a 0.5B-model follow-up and the baseline this was compared against.

## License

Not yet decided; no license file is included.
