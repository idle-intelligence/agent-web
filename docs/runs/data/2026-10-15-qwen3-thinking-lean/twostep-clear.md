# Agent eval, Qwen3-1.7B-Q8_0, unconstrained, 2026-10-15

## Run: Qwen3-1.7B-Q8_0, 4 tools, native, webgpu, label=two-step

- system: "You are a helpful assistant. Use the available tools to get information and to act; call tools as many times as needed, one after another, and never claim you lack data that a tool can provide."
- tool_count: 4
- max_new_tokens: 1024
- max_steps: 6

| id | correct | steps | total_s | retries | tool_errors | prompt_tok | think_tok | answer_tok | first_token_s | decode_s | capped | reason |
|----|---------|-------|---------|---------|-------------|------------|-----------|------------|---------------|----------|--------|--------|
| t01 | false | 1 | 7.193 | 0 | 0 | 500 | 124 | 35 | 1.914 | 5.276 | 0 | final answer did not contain any of ["4"]: Some("I don't have the capability to perform mathematical calculations. However, I can help you with weather updates, location information, or room settings if that's something you need!") |
| t02 | true | 2 | 11.050 | 0 | 0 | 1050 | 237 | 50 | 0.806 | 10.240 | 0 | matched expected call sequence |
| t03 | true | 3 | 24.526 | 0 | 0 | 1643 | 529 | 80 | 1.506 | 23.012 | 0 | matched expected call sequence |
| t04 | true | 3 | 28.268 | 0 | 0 | 1634 | 611 | 65 | 1.638 | 26.622 | 0 | matched expected call sequence |
| t05 | true | 3 | 22.895 | 0 | 0 | 1643 | 493 | 64 | 1.605 | 21.282 | 0 | matched expected call sequence |
| t06 | false | 3 | 39.680 | 1 | 1 | 2412 | 851 | 108 | 2.539 | 37.130 | 0 | exact_args mode: expected 2 call(s), got 3 |
| t07 | true | 3 | 29.849 | 0 | 0 | 1699 | 652 | 74 | 1.924 | 27.917 | 0 | matched expected call sequence |
| t08 | true | 2 | 30.931 | 0 | 0 | 1063 | 718 | 63 | 1.019 | 29.906 | 0 | matched expected call sequence |

**Summary**: correct 75.0% (8 scored, 0 skipped), mean_steps 2.50, mean_total_s 24.299, total_retries 1, total_tool_errors 1

**Per step** (20 steps): prompt_tok 582.2, think_tok 210.8, answer_tok 26.9, first_token_s 0.648, decode_s 9.069, total_s 9.717
