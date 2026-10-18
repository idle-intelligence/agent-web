# Agent eval, Qwen3-1.7B-Q8_0, unconstrained, 2026-10-15

## Run: Qwen3-1.7B-Q8_0, 4 tools, native, webgpu, label=two-step

- system: "You are a helpful assistant. Use the available tools to get information and to act; call tools as many times as needed, one after another, and never claim you lack data that a tool can provide."
- tool_count: 4
- max_new_tokens: 1024
- max_steps: 6

| id | correct | steps | total_s | retries | tool_errors | prompt_tok | think_tok | answer_tok | first_token_s | decode_s | capped | reason |
|----|---------|-------|---------|---------|-------------|------------|-----------|------------|---------------|----------|--------|--------|
| t01 | false | 1 | 7.440 | 0 | 0 | 500 | 124 | 35 | 2.000 | 5.438 | 0 | final answer did not contain any of ["4"]: Some("I don't have the capability to perform mathematical calculations. However, I can help you with weather updates, location information, or room settings if that's something you need!") |
| t02 | true | 2 | 11.502 | 0 | 0 | 1050 | 237 | 50 | 0.891 | 10.609 | 0 | matched expected call sequence |
| t03 | true | 3 | 23.825 | 0 | 0 | 1643 | 529 | 80 | 1.491 | 22.330 | 0 | matched expected call sequence |
| t04 | true | 3 | 20.658 | 0 | 0 | 1634 | 462 | 65 | 1.642 | 19.011 | 1 | matched expected call sequence |
| t05 | true | 3 | 21.491 | 0 | 0 | 1643 | 493 | 64 | 1.452 | 20.035 | 0 | matched expected call sequence |
| t06 | false | 3 | 35.198 | 1 | 1 | 2412 | 801 | 108 | 2.577 | 32.615 | 1 | exact_args mode: expected 2 call(s), got 3 |
| t07 | true | 3 | 38.206 | 1 | 0 | 2292 | 910 | 76 | 2.325 | 35.875 | 1 | matched expected call sequence |
| t08 | true | 2 | 21.304 | 0 | 0 | 1063 | 516 | 39 | 1.255 | 20.046 | 2 | matched expected call sequence |

**Summary**: correct 75.0% (8 scored, 0 skipped), mean_steps 2.50, mean_total_s 22.453, total_retries 2, total_tool_errors 1

**Per step** (20 steps): prompt_tok 611.9, think_tok 203.6, answer_tok 25.9, first_token_s 0.682, decode_s 8.298, total_s 8.979
