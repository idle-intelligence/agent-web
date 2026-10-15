# 2026-10-15: Qwen3-1.7B with thinking on lean (WebGPU), two-step set and Sonos set

Re-run of the 2026-10-08 Qwen3-1.7B thinking measurements (Q8_0, thinking on,
greedy, unconstrained) on the agent loop as it stands on `main` after the
move to lean and lean-chat, with the thinking support added on this branch:
thinking-block parsing, an off switch, a thinking cap, and per-step token and
time accounting. The 2026-10-08 numbers are the comparison column; that run
used the earlier agent loop, on the Linux test machine (RTX 3080).

## Parameters

| parameter | value |
|---|---|
| model | Qwen3-1.7B, Q8_0 GGUF, tokenizer and chat template from the model's own files |
| engine | lean, backend chosen by capability (`Backend::Auto`); every run reports `webgpu` (native, Metal) |
| engine source | llm-web `browser-perf` at 572925e, via a local `cargo --config` patch of `lean`, `lean-core` and `lean-chat` (paths to the three crates of a local checkout; nothing committed). The committed pin is llm-web `main` (6daf322) |
| machine | the M2 laptop, macOS 26.3.1, GPU shared with other jobs through the GPU lock |
| thinking | template default (Qwen3 thinks); the empty think block is not appended |
| decoding | greedy, `max_new_tokens` 1024, `max_steps` 6, `max_ctx` 4096 |
| history | the model's reasoning is not kept in the history; the assistant turn keeps the tool calls or the answer |
| system prompts | default: "You are a helpful home assistant with access to Sonos speaker controls." (Sonos) and "You are a helpful assistant with access to tools." (two-step). clear: "You are a helpful assistant. Use the available tools to get information and to act; call tools as many times as needed, one after another, and never claim you lack data that a tool can provide." |
| Sonos set | `eval/utterances.json`: 46 cases, 33 solvable with the 13-tool subset and scored, 13 skipped; fixture results from `crates/agent/fixtures/sonos/results` (offline, no Sonos calls) |
| two-step set | `crates/agent/fixtures/two-step`: 8 cases, 4 tools, `exact_args` scoring plus answer checks |
| cap | `--think-cap N`: a thinking block still open after N tokens is closed with `\n</think>\n\n` and generation continues with the remaining budget |
| token counts | the engine's tokenizer applied to the decoded text of each step, summed over the step's attempts (retries included) |
| times | `first_token_s`: from the start of each generate call to its first streamed piece (prefill plus the first token); `decode_s`: the rest of the call, thinking included |

Load average (1, 5, 15 min) at the end of each run, from `uptime`; the load
came from other jobs on the machine, so none of these times is a clean
measurement:

| run | load at end |
|---|---|
| two-step default | 5.95 6.29 6.53 |
| two-step clear | 8.24 6.99 6.71 |
| two-step clear, cap 256 | 2.21 2.23 2.47 |
| Sonos default | 0.96 1.35 2.06 |
| Sonos clear | 2.16 2.29 2.56 |
| Sonos clear, cap 512 | 2.31 3.32 3.57 |

The binary was built at 572925e; commits 141bd5a through cc92176 landed on
`browser-perf` while the runs were going and are not in it. Whether the
working tree held uncommitted edits at build time was not checked.

## Pass rate

Sonos set, 33 scored cases, by case family (`kind` in `eval/utterances.json`;
`multi-fr` are the two French multi-step cases):

| family (cases) | 10-08 default | now default | 10-08 clear | now clear | 10-08 clear, cap 512 | now clear, cap 512 |
|---|---|---|---|---|---|---|
| single (14) | 9/14 | 9/14 | 9/14 | 9/14 | 10/14 | 10/14 |
| multi (17) | 8/17 | 9/17 | 11/17 | 11/17 | 11/17 | 12/17 |
| multi-fr (2) | 1/2 | 1/2 | 1/2 | 1/2 | 1/2 | 1/2 |
| all (33) | 18/33 = 54.5% | 19/33 = 57.6% | 21/33 = 63.6% | 21/33 = 63.6% | 22/33 = 66.7% | 23/33 = 69.7% |

Two-step set, 8 cases:

| run | 10-08 | now |
|---|---|---|
| default | 6/8 = 75.0% | 6/8 = 75.0% |
| clear | 7/8 = 87.5% | 6/8 = 75.0% |
| clear, cap 256 | 7/8 = 87.5% | 6/8 = 75.0% |

## Per case

Sonos set (pass or fail):

| id | family | 10-08 default | now default | 10-08 clear | now clear | now clear, cap 512 |
|---|---|---|---|---|---|---|
| s01 | single | pass | pass | fail | fail | pass |
| s02 | single | pass | pass | pass | pass | pass |
| s03 | single | pass | pass | fail | fail | fail |
| s06 | single | pass | pass | pass | pass | pass |
| s07 | single | fail | fail | pass | pass | pass |
| s08 | single | pass | pass | pass | pass | pass |
| s09 | single | fail | fail | pass | pass | pass |
| m01 | multi | pass | pass | pass | pass | pass |
| m02 | multi | pass | pass | pass | pass | pass |
| m03 | multi | pass | pass | pass | pass | pass |
| m04 | multi | fail | pass | pass | pass | pass |
| m05 | multi | pass | pass | pass | pass | pass |
| m07 | multi | pass | pass | pass | pass | pass |
| m08 | multi | fail | fail | fail | fail | fail |
| m09 | multi | pass | pass | pass | pass | pass |
| m11 | multi | pass | fail | pass | pass | pass |
| s11 | single | pass | pass | pass | pass | pass |
| s13 | single | pass | pass | pass | pass | pass |
| s14 | single | fail | fail | fail | fail | fail |
| s15 | single | fail | fail | fail | fail | fail |
| s16 | single | fail | fail | fail | fail | fail |
| s17 | single | pass | pass | pass | pass | pass |
| m13 | multi | fail | pass | pass | pass | pass |
| m20 | multi | pass | pass | pass | fail | pass |
| m21 | multi | fail | fail | fail | fail | fail |
| m22 | multi | fail | fail | fail | fail | fail |
| m23 | multi | fail | fail | fail | fail | fail |
| m24 | multi | fail | fail | fail | pass | pass |
| m25 | multi-fr | pass | pass | pass | pass | pass |
| m26 | multi-fr | fail | fail | fail | fail | fail |
| m27 | multi | fail | fail | fail | fail | fail |
| m28 | multi | fail | fail | pass | pass | pass |
| s18 | single | pass | pass | pass | pass | pass |

Two-step set:

| id | 10-08 default | now default | 10-08 clear | now clear | now clear, cap 256 |
|---|---|---|---|---|---|
| t01 | fail | fail | fail | fail | fail |
| t02 | pass | pass | pass | pass | pass |
| t03 | pass | pass | pass | pass | pass |
| t04 | pass | pass | pass | pass | pass |
| t05 | pass | pass | pass | pass | pass |
| t06 | pass | pass | pass | fail | fail |
| t07 | pass | pass | pass | pass | pass |
| t08 | fail | fail | pass | pass | pass |

## Tokens and time per step

Means over all steps of the scored cases (these columns have no 10-08
counterpart; the earlier loop did not count tokens):

| run | steps | prompt_tok | think_tok | answer_tok | first_token_s | decode_s | total_s | decode tok/s | capped steps | retries |
|---|---|---|---|---|---|---|---|---|---|---|
| Sonos default | 89 | 2830.9 | 496.0 | 40.5 | 1.932 | 23.801 | 25.732 | 22.5 | 0 | 12 |
| Sonos clear | 91 | 2659.8 | 411.7 | 37.8 | 1.860 | 21.663 | 23.523 | 20.8 | 0 | 7 |
| Sonos clear, cap 512 | 90 | 2734.0 | 348.3 | 40.5 | 1.905 | 17.700 | 19.604 | 22.0 | 13 | 8 |
| two-step default | 19 | 514.7 | 254.2 | 25.2 | 0.606 | 10.632 | 11.238 | 26.3 | 0 | 0 |
| two-step clear | 20 | 582.2 | 210.8 | 26.9 | 0.648 | 9.069 | 9.717 | 26.2 | 0 | 1 |
| two-step clear, cap 256 | 20 | 611.9 | 203.6 | 25.9 | 0.682 | 8.298 | 8.980 | 27.7 | 5 | 2 |

`decode tok/s` is (think_tok + answer_tok) / decode_s. `prompt_tok` is the
whole rendered prompt of each attempt, counted before the engine reuses the
KV prefix of the previous call.

Per case (mean over the scored cases), 2026-10-08 on the RTX 3080 against now on the M2 laptop:

| run | steps per case, 10-08 | steps per case, now | mean_total_s, 10-08 | mean_total_s, now |
|---|---|---|---|---|
| Sonos default | 2.36 | 2.70 | 16.745 | 69.41 |
| Sonos clear | 2.55 | 2.76 | 15.819 | 69.81 |
| Sonos clear, cap 512 | 2.73 | 2.73 | 16.196 | 53.49 |
| two-step default | 2.38 | 2.38 | 8.750 | 26.70 |
| two-step clear | 2.50 | 2.50 | 7.185 | 24.30 |
| two-step clear, cap 256 | 2.50 | 2.50 | 6.660 | 22.45 |

Full per-case reports (reasons, retries, tokens, times):
`docs/runs/data/2026-10-15-qwen3-thinking-lean/`.

## Observations

- The Sonos pass rates match the 2026-10-08 ones within one case per
  configuration: default 19/33 against 18/33, clear 21/33 against 21/33,
  clear with a 512 cap 23/33 against 22/33. The case flips are m04 and m13
  (fail to pass) and m11 (pass to fail) with the default prompt, and m20
  (pass to fail) and m24 (fail to pass) with the clear prompt.
- The 13 cases that are skipped need a tool outside the 13-tool subset. The
  repo's set has 46 cases and 33 scored ones; no set of 43 cases exists in
  it, and these are the cases the 2026-10-08 percentages were computed on.
- The two-step set scored 6/8 in all three configurations, against 7/8 for
  clear and clear with cap 256 on 2026-10-08. The difference is t06: with the
  clear prompt it made a third call (1 retry, 1 tool error, expected 2
  calls). t01 (the arithmetic question, which the model answers by saying it
  has no tool for it) and, with the default prompt, t08 fail as before.
- The cap fired on 13 of 90 steps with 512 tokens (Sonos) and on 5 of 20
  steps with 256 tokens (two-step), through the engine's ordinary
  text-in/text-out generate. Capping 512 lowered mean think_tok per step from
  411.7 to 348.3 and mean_total_s per case from 69.81 to 53.49, with 23/33
  against 21/33 on the same prompt. The cap 512 run is one run; the 2 case
  difference is within what the flips between runs above show.
- A step costs about 23.5 s (Sonos clear) or 9.7 s (two-step clear) in total.
  Of the 23.5 s, 1.86 s is time to the first piece and 21.7 s is decode,
  which at 20.8 tokens per second is mostly the 411.7 thinking tokens per
  step (answer tokens: 37.8). A case takes 2.76 steps and 69.81 s on
  average, so each step of a tool-calling turn costs about 24 s on this
  machine under these load averages.
- Prompts are 2660 to 2831 tokens per step on the Sonos set (13 tool
  schemas) and 515 to 612 on the two-step set (4 tools).
- Retries: 12 in the default Sonos run and 7 in the clear one. Tool errors
  were 0 on the Sonos set and 1 on two-step clear, which is the t06 case.
- The times are 3.0 to 4.4 times the RTX 3080 times of 2026-10-08 for the
  same configuration, on a different machine and engine build, with other
  jobs running (load averages above). They show the order of magnitude on the
  M2 laptop, not a comparison of the two GPUs.
