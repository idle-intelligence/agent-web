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

## Thinking off, clear prompt

Same harness and parameters as above, with `--thinking off`: the prompt
ends with Qwen3's empty thinking block, so the model answers or calls tools
without reasoning first. Clear system prompt, greedy, `max_new_tokens` 1024,
`max_steps` 6, `max_ctx` 4096, backend `webgpu` (native, Metal) in every
run. The binary was built from llm-web `browser-perf` at cc92176 (no
tracked edits at build time) through the same local `cargo --config`
patch; the thinking-on columns are the runs above, built at 572925e. The
report now also lists the calls made and the final answer of each failed
case.

| run | load at start | load at end |
|---|---|---|
| Sonos clear, Qwen3-1.7B, thinking off | 1.61 2.22 2.99 | 2.52 2.21 2.60 |
| two-step clear, Qwen3-1.7B, thinking off | 2.52 2.21 2.60 | 2.35 2.20 2.58 |
| Sonos clear, Qwen3-1.7B, thinking on, 9 cases | 2.35 2.20 2.58 | 3.45 3.07 2.82 |
| Sonos clear, Qwen3-0.6B, thinking off | 2.76 2.94 2.78 | 2.67 2.44 2.55 |
| two-step clear, Qwen3-0.6B, thinking off | 2.67 2.44 2.55 | 3.69 2.66 2.63 |

### Pass rate

Sonos set, 33 scored cases:

| family (cases) | 1.7B clear, thinking on | 1.7B clear, thinking off | 0.6B clear, thinking off |
|---|---|---|---|
| single (14) | 9/14 | 4/14 | 1/14 |
| multi (17) | 11/17 | 0/17 | 0/17 |
| multi-fr (2) | 1/2 | 0/2 | 0/2 |
| all (33) | 21/33 = 63.6% | 4/33 = 12.1% | 1/33 = 3.0% |

Two-step set, 8 cases:

| run | 1.7B, thinking on | 1.7B, thinking off | 0.6B, thinking off |
|---|---|---|---|
| clear | 6/8 = 75.0% | 2/8 = 25.0% | 2/8 = 25.0% |

### Per case

| id | 1.7B clear, thinking on | 1.7B clear, thinking off | 0.6B clear, thinking off |
|---|---|---|---|
| s01 | fail | fail | fail |
| s02 | pass | pass | fail |
| s03 | fail | fail | fail |
| s06 | pass | fail | fail |
| s07 | pass | fail | fail |
| s08 | pass | fail | fail |
| s09 | pass | fail | fail |
| m01 | pass | fail | fail |
| m02 | pass | fail | fail |
| m03 | pass | fail | fail |
| m04 | pass | fail | fail |
| m05 | pass | fail | fail |
| m07 | pass | fail | fail |
| m08 | fail | fail | fail |
| m09 | pass | fail | fail |
| m11 | pass | fail | fail |
| s11 | pass | pass | fail |
| s13 | pass | fail | fail |
| s14 | fail | pass | fail |
| s15 | fail | fail | fail |
| s16 | fail | fail | fail |
| s17 | pass | fail | fail |
| m13 | pass | fail | fail |
| m20 | fail | fail | fail |
| m21 | fail | fail | fail |
| m22 | fail | fail | fail |
| m23 | fail | fail | fail |
| m24 | pass | fail | fail |
| m25 | pass | fail | fail |
| m26 | fail | fail | fail |
| m27 | fail | fail | fail |
| m28 | pass | fail | fail |
| s18 | pass | pass | pass |
| t01 | fail | pass | pass |
| t02 | pass | pass | pass |
| t03 | pass | fail | fail |
| t04 | pass | fail | fail |
| t05 | pass | fail | fail |
| t06 | fail | fail | fail |
| t07 | pass | fail | fail |
| t08 | pass | fail | fail |

### Tokens and time per step

Means over all steps of the scored cases; the thinking-on rows are the
clear rows of the table above.

| run | steps | prompt_tok | think_tok | answer_tok | first_token_s | decode_s | total_s | decode tok/s |
|---|---|---|---|---|---|---|---|---|
| Sonos clear, 1.7B, thinking on | 91 | 2659.8 | 411.7 | 37.8 | 1.860 | 21.663 | 23.523 | 20.8 |
| Sonos clear, 1.7B, thinking off | 68 | 2322.7 | 0.0 | 33.0 | 0.869 | 1.226 | 2.095 | 26.9 |
| Sonos clear, 0.6B, thinking off | 60 | 2232.4 | 0.0 | 21.4 | 0.474 | 0.511 | 0.985 | 41.9 |
| two-step clear, 1.7B, thinking on | 20 | 582.2 | 210.8 | 26.9 | 0.648 | 9.069 | 9.717 | 26.2 |
| two-step clear, 1.7B, thinking off | 12 | 531.7 | 0.0 | 25.7 | 0.403 | 0.743 | 1.146 | 34.6 |
| two-step clear, 0.6B, thinking off | 15 | 528.3 | 0.0 | 20.7 | 0.427 | 0.527 | 0.954 | 39.3 |

Per case:

| run | steps per case | mean_total_s | retries | tool errors |
|---|---|---|---|---|
| Sonos clear, 1.7B, thinking on | 2.76 | 69.81 | 7 | 0 |
| Sonos clear, 1.7B, thinking off | 2.09 | 4.32 | 0 | 0 |
| Sonos clear, 0.6B, thinking off | 1.82 | 1.80 | 0 | 0 |
| two-step clear, 1.7B, thinking on | 2.50 | 24.30 | 1 | 1 |
| two-step clear, 1.7B, thinking off | 1.50 | 1.72 | 0 | 1 |
| two-step clear, 0.6B, thinking off | 1.88 | 1.81 | 0 | 3 |

### The nine Sonos cases that fail in every configuration

m08, m21, m22, m23, m26, m27, s14, s15 and s16 fail in all four thinking-on
configurations above (10-08 and now, default and clear prompts, and the
capped run). They were run again with Qwen3-1.7B, clear prompt, thinking
on, on the cc92176 build, to record the calls and the answers; 8 of the 9
reproduce the earlier clear run's token counts exactly, and m08 differs
(4 steps and 5203 thinking tokens against 5 steps and 7447). The fixture
returns one canned result per tool whatever the arguments (`pause` always
reports the kitchen group as paused, `set_group_volume` always reports
volume 35, `get_now_playing` always reports the kitchen group playing
Radiohead). Data: `sonos-clear-nine-failing-rerun.md`.

| id | utterance | required by the case | calls made (thinking on, clear) | final answer, shortened |
|---|---|---|---|---|
| m08 | Turn up the kitchen volume by 10. | `adjust_group_volume(KITCHEN, +10)` | households, `set_group_volume(KITCHEN, 10)`, `set_group_volume(KITCHEN, 45)` | "The kitchen volume has been set to 45." |
| s14 | What's on in the bedroom? | `get_now_playing(BEDROOM)` | households | "The bedroom is currently idle, meaning no music is playing" |
| s15 | Is anything playing? | `get_now_playing(LIVING)` | `get_now_playing("default")` | "Yes, something is playing on RINCON_KITCHEN01:1 ... Radiohead" |
| s16 | Turn it down a bit. | `adjust_group_volume(LIVING, -10)` | households, `adjust_group_volume(LIVING, -5)` | "The volume for the Kitchen group ... adjusted down by 5" |
| m21 | Pause the garage. | no mutating call, text answer | `pause("garage")` | "The garage group (identified as RINCON_KITCHEN01:1) has been paused." |
| m22 | Play something. | any `play_*` call with a valid group id | none | asks for an artist, a favorite or a station |
| m23 | Pause the kitchen and turn up the bedroom. | `pause(KITCHEN)`, then `adjust_group_volume(BEDROOM, +10)` | households, `pause(KITCHEN)`, `set_group_volume(BEDROOM, 50)`, `set_group_volume(BEDROOM, 60)` | "the bedroom's volume has been adjusted to 60%" |
| m26 | Monte le son du salon. | `adjust_group_volume(LIVING, +10)` | households | "The living room ... is currently playing music. Would you like to adjust the volume ...?" |
| m27 | Play nirvana on BLOUPBLIP. | `play_artist(KITCHEN, "Nirvana", false)` | `play_artist(BLOUPBLIP, "Nirvana", false)` | "The group RINCON_KITCHEN01:1 is now playing Nirvana." |

KITCHEN, LIVING and BEDROOM stand for `RINCON_KITCHEN01:1`,
`RINCON_LIVING01:2` and `RINCON_BEDROOM01:3`; "households" is
`get_households_and_groups_and_players({})`.

### Observations

- Thinking off cuts the Sonos pass rate of Qwen3-1.7B from 21/33 to 4/33
  and the two-step one from 6/8 to 2/8 on the clear prompt. No multi-step
  Sonos case passes without thinking (0/17 and 0/2 French).
- Without thinking the model mostly skips the discovery call: 18 of the
  29 failed Sonos cases (1.7B) use a group id that is not in the household
  (`"kitchen"`, `"your_group_id"`, `"living room"`), and only 3 of the 29
  call `get_households_and_groups_and_players`. On the two-step set it
  calls `get_weather({"city":"here"})` after `get_location` (t03, t05) and
  `set_volume` with a room name instead of the listed id (t07). For 0.6B
  the figures are 26 of 32 Sonos failures with an invented id and none
  with the lookup.
- 7 of the 29 failed Sonos cases (1.7B, thinking off) end in text with no
  call, several of them announcing the call instead of making it (s15
  "Let me call the `get_now_playing` function", s17, m07). t04, t06 and t08
  end the same way on the two-step set.
- The two cases that pass only with thinking off, s14 and t01, are cases
  where thinking on answers without a tool (s14 from the household listing,
  t01 by declining arithmetic); without thinking, s14 calls
  `get_now_playing(BEDROOM)` and t01 answers with 4.
- A thinking-off step costs 2.10 s against 23.52 s with thinking on
  (Sonos, 1.7B), and a case 4.32 s against 69.81 s. Time to the first piece
  drops from 1.86 s to 0.87 s per step and decode from 21.66 s to 1.23 s,
  since the step generates 33.0 tokens instead of 449.5. The thinking-on
  and thinking-off runs used different engine builds (572925e and
  cc92176) under similar load averages.
- Qwen3-0.6B with thinking off takes 0.99 s per step and 1.80 s per case on
  the Sonos set and passes 1/33 (s18, the case that needs no call) and 2/8
  on the two-step set.
- Of the nine cases that fail everywhere with thinking on, the calls show
  the following. m08 and m23 use `set_group_volume` for a relative
  change, with values not read from any tool. s16 makes the expected call
  with a delta of -5 where the case requires -10. m21, s15 and m27 use a
  group id absent from the household (`"garage"`, `"default"`,
  `"BLOUPBLIP"`), and the canned fixture result reports success on the
  kitchen group, which the answers repeat. s14 answers from the household
  listing ("idle") without `get_now_playing`, m22 asks what to play, and
  m26 looks the room up and then asks what to do instead of raising the
  volume.

## Server-like fixture and corrected scoring

The Sonos fixture and five cases' scoring were changed, and the three
thinking-on Sonos configurations were run again. These scores are not
comparable to the earlier runs in this log: the fixture now answers
differently (the model sees different tool results), and five cases are
scored differently. The "before" columns below are the runs above, on the
old fixture and scoring; they are listed to show which cases changed, not
as a baseline.

Fixture changes (`crates/agent/fixtures/sonos/README.md`):

- A call naming a group or player id that the household listing does not
  have returns an error result (`group "garage" not found in household
  Sonos_ABC123XYZ`) instead of the canned success on the kitchen group.
- `get_now_playing` answers per group, matching each group's
  `playbackState` in the listing: Kitchen paused on the Radiohead track,
  Living Room playing, Bedroom idle with no current item. It used to report
  the kitchen group playing Radiohead for any argument.

Scoring changes (`eval/utterances.json`):

| id | before | now |
|---|---|---|
| m27 | `play_artist(KITCHEN, "Nirvana", false)` required | no mutating call and a text answer, as m21 |
| s14 | `get_now_playing(BEDROOM)` required | that, or the household lookup and an answer containing "idle", "nothing", "not playing", "no music" or "isn't playing" |
| s16 | `adjust_group_volume(LIVING, -10)` | `adjust_group_volume(LIVING, d)` with -20 <= d <= -1 |
| m23 | `pause(KITCHEN)`, then `adjust_group_volume(BEDROOM, +10)` | the same with 1 <= d <= 20 |
| m26 | `adjust_group_volume(LIVING, +10)` | `adjust_group_volume(LIVING, d)` with 1 <= d <= 20 |
| m22 | any `play_*` call with a valid group id | that, or no mutating call and an answer containing a question mark |

### Parameters

Same as the thinking-on runs above (Qwen3-1.7B Q8_0, thinking on, greedy,
`max_new_tokens` 1024, `max_steps` 6, `max_ctx` 4096, 33 scored cases, the
13-tool subset), with these differences:

| parameter | value |
|---|---|
| agent-web | bcf446b (fixture and scoring changes) |
| engine source | llm-web `browser-perf` at 1199050 through the same local `cargo --config` patch of `lean`, `lean-core` and `lean-chat`; no tracked edits at build time. 1199050 differs from cc92176 by one run document, so the engine code is that of cc92176 |
| backend | `webgpu` (native, Metal) in all three runs, from the binary's `backend:` line |
| runs | one after the other, each started when the 1-minute load average was below 3 and no rustc or cargo was running, under the GPU lock |

| run | load at start | load at end |
|---|---|---|
| Sonos default | 2.60 3.81 3.49 | 1.17 2.53 2.84 |
| Sonos clear | 1.17 2.53 2.84 | 1.23 1.17 1.32 |
| Sonos clear, cap 512 | 1.23 1.17 1.32 | 1.08 1.07 1.09 |

### Pass rate

| family (cases) | default, before | default, now | clear, before | clear, now | clear cap 512, before | clear cap 512, now |
|---|---|---|---|---|---|---|
| single (14) | 9/14 | 11/14 | 9/14 | 11/14 | 10/14 | 12/14 |
| multi (17) | 9/17 | 10/17 | 11/17 | 12/17 | 12/17 | 13/17 |
| multi-fr (2) | 1/2 | 1/2 | 1/2 | 1/2 | 1/2 | 1/2 |
| all (33) | 19/33 = 57.6% | 22/33 = 66.7% | 21/33 = 63.6% | 24/33 = 72.7% | 23/33 = 69.7% | 26/33 = 78.8% |

### Cases whose outcome changed

The same three cases, fail to pass, in each of the three configurations; no
case went from pass to fail.

| id | default | clear | clear, cap 512 | reason given by the scorer now |
|---|---|---|---|---|
| s14 | fail to pass | fail to pass | fail to pass | lookup made, answer says idle or nothing playing (the alternative) |
| s16 | fail to pass | fail to pass | fail to pass | `adjust_group_volume(LIVING, d)` with d in the accepted range |
| m22 | fail to pass | fail to pass | fail to pass | no mutating call, answer asks what to play (the alternative) |

### Tokens and time per step

| run | steps | prompt_tok | think_tok | answer_tok | first_token_s | decode_s | total_s | steps per case | mean_total_s | retries | tool errors |
|---|---|---|---|---|---|---|---|---|---|---|---|
| Sonos default | 90 | 2828.5 | 493.2 | 40.4 | 1.863 | 23.561 | 25.424 | 2.73 | 69.35 | 12 | 1 |
| Sonos clear | 93 | 2686.9 | 418.4 | 39.1 | 1.770 | 18.978 | 20.748 | 2.85 | 62.55 | 7 | 3 |
| Sonos clear, cap 512 | 94 | 2730.5 | 351.2 | 41.0 | 1.794 | 15.838 | 17.633 | 2.85 | 50.24 | 8 | 3 |

Tool errors by case: default, m27; clear, s15, m21 and m27; clear with cap
512, m08, m21 and m27. Each is a call with an id outside the household
(`"BLOUPBLIP"`, `"garage"`, `"default"`, `"kitchen"`), which the old
fixture answered with a success.

Data: `sonos-default-server-fixture.md`, `sonos-clear-server-fixture.md`,
`sonos-clear-cap512-server-fixture.md`.

### Observations

- All three outcome changes, in all three configurations, come from the
  scoring changes (s14, s16, m22 pass through the new alternatives and
  range); the fixture change did not turn any case from fail to pass or
  from pass to fail. Totals rise by exactly 3 in each configuration: 19 to
  22, 21 to 24, 23 to 26 of 33.
- m27 still fails in all three runs, now against the no-mutation rule:
  the model calls `play_artist` on `"BLOUPBLIP"` first, gets the
  not-found error, then looks the household up and answers that the speaker
  does not exist and lists the three groups. The scorer counts the rejected
  call as a mutating call. m21 does the same with `pause("garage")` in
  the clear and capped runs; in the default run it pauses the living room
  after the lookup.
- With the error results, the final answers of m21, m27 and s15 no longer
  repeat a success on the kitchen group: m21 and m27 report that the room
  was not found, and s15 (clear) calls `get_now_playing("default")`, gets
  the error, calls the household listing and answers that the living room
  is playing. s15 still fails, since it never calls
  `get_now_playing(LIVING)`.
- m23 and m26 still fail with the 1 to 20 range: m23 uses
  `set_group_volume` on the bedroom (70, 50 then 60, and 50 in the three
  runs), and m26 looks the room up and asks what to do without a volume
  call. m08 still uses `set_group_volume` for "by 10".

## Rejected mutating calls no longer count as mutations

In `no_mutation` cases (m21, m27, and m22's alternative), a mutating call
counted as a mutation even when the fixture rejected it with an error
result (unknown group or player id). The scorer now counts only mutating
calls whose result was not an error: such a case passes when no mutating
call succeeded, the run ended in a text answer, and the answer checks (if
any) pass. A rejected call changed nothing on the household, so trying an
id, getting the not-found error and then answering correctly is a pass. The
rule is the same for every case; the run loop records which calls returned
an error (`tool_error_message`, as the agent itself uses) and passes that
to the scorer. The other acceptance modes are unchanged.

### Parameters

Same as the server-like fixture runs above (Qwen3-1.7B Q8_0, thinking on,
greedy, `max_new_tokens` 1024, `max_steps` 6, `max_ctx` 4096, 33 scored
cases, the 13-tool subset, same fixture and case file apart from the notes
of m21 and m27), with these differences:

| parameter | value |
|---|---|
| agent-web | c8c6e63 (scoring change) |
| engine source | llm-web `rep-penalty-hf` at e7bf205 through the same local `cargo --config` patch of `lean`, `lean-core` and `lean-chat`; no tracked edits at build time. The branch's change over cc92176 is the repetition penalty, which greedy decoding with no penalty does not use |
| backend | `webgpu` (native, Metal) in all three runs, from the binary's `backend:` line |
| runs | one after the other, each started when the 1-minute load average was below 3 and no rustc or cargo was running, under the GPU lock |

| run | load at start | load at end |
|---|---|---|
| Sonos default | 2.42 2.56 2.82 | 2.07 2.62 3.10 |
| Sonos clear | 2.07 2.62 3.10 | 2.09 1.86 1.82 |
| Sonos clear, cap 512 | 2.09 1.86 1.82 | 1.48 1.74 1.99 |

### Pass rate

"before" is the server-like fixture run above (bcf446b, cc92176 engine
code); "now" is this run.

| family (cases) | default, before | default, now | clear, before | clear, now | clear cap 512, before | clear cap 512, now |
|---|---|---|---|---|---|---|
| single (14) | 11/14 | 11/14 | 11/14 | 11/14 | 12/14 | 12/14 |
| multi (17) | 10/17 | 11/17 | 12/17 | 14/17 | 13/17 | 15/17 |
| multi-fr (2) | 1/2 | 1/2 | 1/2 | 1/2 | 1/2 | 1/2 |
| all (33) | 22/33 = 66.7% | 23/33 = 69.7% | 24/33 = 72.7% | 26/33 = 78.8% | 26/33 = 78.8% | 28/33 = 84.8% |

### Cases whose outcome changed

| id | default | clear | clear, cap 512 | reason given by the scorer now |
|---|---|---|---|---|
| m27 | fail to pass | fail to pass | fail to pass | no mutating call succeeded (all rejected); final text answer given |
| m21 | fail (unchanged) | fail to pass | fail to pass | clear and capped: as m27. default: `pause(LIVING)` was made, after the lookup, and succeeded |

### Tokens and time per step

| run | steps | prompt_tok | think_tok | answer_tok | first_token_s | decode_s | total_s | steps per case | mean_total_s | retries | tool errors |
|---|---|---|---|---|---|---|---|---|---|---|---|
| Sonos default | 90 | 2828.5 | 493.2 | 40.4 | 2.306 | 27.486 | 29.792 | 2.73 | 81.29 | 12 | 1 |
| Sonos clear | 93 | 2686.9 | 418.4 | 39.1 | 2.126 | 22.565 | 24.691 | 2.85 | 74.60 | 7 | 3 |
| Sonos clear, cap 512 | 94 | 2730.5 | 351.2 | 41.0 | 2.092 | 17.600 | 19.692 | 2.85 | 56.11 | 8 | 3 |

Data: `sonos-default-rejected-calls.md`, `sonos-clear-rejected-calls.md`,
`sonos-clear-cap512-rejected-calls.md`.

### Observations

- Every case has the same steps, retries, tool errors, capped steps and
  prompt, thinking and answer token counts as the server-like fixture run
  of the same configuration (per-case diff of those columns of the two
  reports is empty), so the model produced the same output on the e7bf205
  engine as on cc92176. All outcome changes come from the scoring change.
- m27 passes in all three configurations: as in the runs above, the model calls `play_artist`
  on `"BLOUPBLIP"`, gets the not-found error, looks the household up and
  answers that the speaker does not exist. m21 passes with the clear
  prompt, with and without the cap, on the same pattern with
  `pause("garage")`. With the default prompt m21 still fails: after the
  lookup the model pauses the living room, a mutation that succeeded.
- Totals rise by 1 (default, 22 to 23), 2 (clear, 24 to 26) and 2 (clear
  with cap 512, 26 to 28) of 33. No case went from pass to fail.
- m22 is unaffected: it passes through its alternative in all three runs
  with no mutating call at all. s15 still fails in all three runs: its
  rejected `get_now_playing("default")` is a read, not a mutation, and the
  case requires `get_now_playing(LIVING)`, which is never called.
- The times per step are 1.12 to 1.19 times those of the server-like
  fixture runs for the same tokens (total_s 29.79 against 25.42, 24.69
  against 20.75, 19.69 against 17.63), under similar load averages; the two
  sets of runs used different engine builds and were not interleaved, so
  this is not a measured engine difference.
