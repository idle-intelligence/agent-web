# agent-web

A tool-calling agent loop for Qwen2.5-Instruct models, running on the [lean](https://github.com/idle-intelligence/llm-web) inference engine.

lean runs the model: text in, text out, on a backend it picks by capability (WebGPU, then CPU threads, then a single CPU thread). Its chat layer, lean-chat, renders messages and tools into prompt text with the model's own chat template. This repo holds what an agent adds on top: parsing the model's tool calls, the step-wise loop, and an offline eval harness. The loop reaches the model through one small trait (prompt text in, streamed text out, abort, a typed context-overflow error), so any engine that completes text can drive it.

## What it does

- Renders each step's prompt with the model's own chat template and the tools, through lean-chat. For Qwen2.5-Instruct that is `<tools>...</tools>` in the system turn and `<tool_call>{"name":..., "arguments":...}</tool_call>` in the model's output. The `fixture_parity_agent_tools` test checks the prompts byte for byte against `transformers.apply_chat_template(..., tools=...)`.
- Parses the model's tool calls out of its raw output, and regenerates when a reply is malformed, calls an unknown tool, or repeats a call.
- Drives a step-wise loop: render the prompt, generate, parse, hand back the pending tool calls, take their results, and repeat until a final text answer.
- Scores runs against a fixed set of utterance cases for a Sonos speaker-control tool set, under four acceptance modes: exact call sequence, required calls in order, no mutating call, any valid play call.

## Crates

- `crates/agent`: the parser, the loop and the eval harness, plus `LeanGenerator`, the loop's generator on lean's engine. Its examples are two native CLIs: `agent_cli` runs one utterance through the loop and prints each step, answering tool calls from fixture files; `agent_eval` runs the eval set and prints a markdown results table.
- `crates/agent-wasm`: wasm-bindgen bindings for the agent on lean's engine, for a Web Worker. lean uses WebGPU when the browser grants an adapter, otherwise the CPU.

lean and lean-chat come from a pinned git revision of [idle-intelligence/llm-web](https://github.com/idle-intelligence/llm-web).

## Model

The examples and the model tests use [Qwen2.5-3B-Instruct](https://huggingface.co/Qwen/Qwen2.5-3B-Instruct): the Q4_0 GGUF from [Qwen/Qwen2.5-3B-Instruct-GGUF](https://huggingface.co/Qwen/Qwen2.5-3B-Instruct-GGUF), and the tokenizer files from the model repo.

```sh
hf download Qwen/Qwen2.5-3B-Instruct-GGUF qwen2.5-3b-instruct-q4_0.gguf --local-dir models/Qwen2.5-3B-Instruct-GGUF
hf download Qwen/Qwen2.5-3B-Instruct tokenizer.json tokenizer_config.json --local-dir models/Qwen2.5-3B-Instruct
```

## Build and test

```sh
cargo build --release
cargo test --workspace
```

The tests that need the model files are ignored by default. With the files above:

```sh
LEAN_GGUF_QWEN25_3B=$PWD/models/Qwen2.5-3B-Instruct-GGUF/qwen2.5-3b-instruct-q4_0.gguf \
LEAN_TOKENIZER_DIR_QWEN25_3B=$PWD/models/Qwen2.5-3B-Instruct \
cargo test -p agent --release -- --ignored
```

The wasm package (needs [wasm-pack](https://rustwasm.github.io/wasm-pack/)) is built into `_site/pkg` by:

```sh
scripts/build.sh
```

## Run

One utterance, with the tool calls answered from `crates/agent/fixtures/sonos/results/`:

```sh
cargo run -p agent --example agent_cli --release -- \
  --model models/Qwen2.5-3B-Instruct-GGUF/qwen2.5-3b-instruct-q4_0.gguf \
  --tokenizer models/Qwen2.5-3B-Instruct \
  --tools crates/agent/fixtures/sonos/tools-subset.json \
  --utterance "pause the kitchen"
```

The eval set, as a markdown table on stdout (`AGENT_EVAL_MAX_CTX` sets the context size in tokens, default 4096; `AGENT_EVAL_DATE` the date in the table's title):

```sh
LEAN_GGUF_QWEN25_3B=models/Qwen2.5-3B-Instruct-GGUF/qwen2.5-3b-instruct-q4_0.gguf \
LEAN_TOKENIZER_DIR_QWEN25_3B=models/Qwen2.5-3B-Instruct \
cargo run -p agent --example agent_eval --release
```

Both pick the backend by capability: the GPU when a wgpu adapter is available, otherwise the CPU forward pass on every core. Each step reuses the KV cache of the previous one for the prompt prefix they share, so the system prompt and the tools are prefilled once per turn.

## Eval

`eval/utterances.json` holds 46 cases. With the 13-tool subset (`crates/agent/fixtures/sonos/tools-subset.json`), 33 of them are solvable and scored; the other 13 need a tool outside the subset and are skipped. Qwen2.5-3B-Instruct Q4_0, run on the CPU with unconstrained decoding (no grammar-constrained output) on 2026-10-07, got 3 of the 33 right (9.1%). The per-case results are in `docs/runs/2026-10-07-sonos-eval.md`.

## License

The code is under the MIT license, see `LICENSE`. The model weights are not part of this repo and come under their own license, given on each model's page.
