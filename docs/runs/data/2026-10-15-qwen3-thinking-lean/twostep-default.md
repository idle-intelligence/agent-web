# Agent eval, Qwen3-1.7B-Q8_0, unconstrained, 2026-10-15

## Run: Qwen3-1.7B-Q8_0, 4 tools, native, webgpu, label=two-step

- system: "You are a helpful assistant with access to tools."
- tool_count: 4
- max_new_tokens: 1024
- max_steps: 6

| id | correct | steps | total_s | retries | tool_errors | prompt_tok | think_tok | answer_tok | first_token_s | decode_s | capped | reason |
|----|---------|-------|---------|---------|-------------|------------|-----------|------------|---------------|----------|--------|--------|
| t01 | false | 1 | 7.728 | 0 | 0 | 469 | 134 | 40 | 1.893 | 5.832 | 0 | final answer did not contain any of ["4"]: Some("I don't have access to mathematical calculation tools. The available functions only handle location, weather, room listings, and volume settings. Would you like me to help with something related to these capabilities?") |
| t02 | true | 2 | 11.548 | 0 | 0 | 995 | 248 | 57 | 0.897 | 10.646 | 0 | matched expected call sequence |
| t03 | true | 3 | 33.016 | 0 | 0 | 1550 | 739 | 77 | 1.652 | 31.356 | 0 | matched expected call sequence |
| t04 | true | 3 | 49.595 | 0 | 0 | 1541 | 1150 | 63 | 1.625 | 47.961 | 0 | matched expected call sequence |
| t05 | true | 3 | 34.108 | 0 | 0 | 1550 | 781 | 68 | 1.579 | 32.521 | 0 | matched expected call sequence |
| t06 | true | 3 | 40.333 | 0 | 0 | 1600 | 945 | 61 | 1.812 | 38.513 | 0 | matched expected call sequence |
| t07 | true | 3 | 29.080 | 0 | 0 | 1606 | 657 | 74 | 1.809 | 27.263 | 0 | matched expected call sequence |
| t08 | false | 1 | 8.169 | 0 | 0 | 469 | 176 | 38 | 0.249 | 7.918 | 0 | exact_args mode: expected 1 call(s), got 0 |

**Summary**: correct 75.0% (8 scored, 0 skipped), mean_steps 2.38, mean_total_s 26.697, total_retries 0, total_tool_errors 0

**Per step** (19 steps): prompt_tok 514.7, think_tok 254.2, answer_tok 25.2, first_token_s 0.606, decode_s 10.632, total_s 11.238
