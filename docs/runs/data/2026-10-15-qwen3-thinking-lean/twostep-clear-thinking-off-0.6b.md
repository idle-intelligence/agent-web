# Agent eval, Qwen3-0.6B-Q8_0, unconstrained, 2026-10-15

## Run: Qwen3-0.6B-Q8_0, 4 tools, native, webgpu, label=two-step

- system: "You are a helpful assistant. Use the available tools to get information and to act; call tools as many times as needed, one after another, and never claim you lack data that a tool can provide."
- tool_count: 4
- max_new_tokens: 1024
- max_steps: 6

| id | correct | steps | total_s | retries | tool_errors | prompt_tok | think_tok | answer_tok | first_token_s | decode_s | capped | reason |
|----|---------|-------|---------|---------|-------------|------------|-----------|------------|---------------|----------|--------|--------|
| t01 | true | 1 | 0.998 | 0 | 0 | 504 | 0 | 8 | 0.838 | 0.159 | 0 | matched expected call sequence |
| t02 | true | 2 | 2.563 | 0 | 0 | 1058 | 0 | 27 | 1.327 | 1.175 | 0 | matched expected call sequence |
| t03 | false | 2 | 2.024 | 0 | 0 | 1060 | 0 | 36 | 1.066 | 0.927 | 0 | exact_args mode: expected 2 call(s), got 1 |
| t04 | false | 2 | 0.918 | 0 | 0 | 1061 | 0 | 33 | 0.355 | 0.562 | 0 | exact_args mode: expected 2 call(s), got 1 |
| t05 | false | 2 | 3.471 | 0 | 0 | 1067 | 0 | 38 | 1.572 | 1.864 | 0 | exact_args mode: expected 2 call(s), got 1 |
| t06 | false | 2 | 2.115 | 0 | 1 | 1057 | 0 | 53 | 0.695 | 1.416 | 0 | exact_args mode: expected 2 call(s), got 1 |
| t07 | false | 2 | 1.209 | 0 | 1 | 1061 | 0 | 62 | 0.273 | 0.934 | 0 | exact_args mode: expected 2 call(s), got 1 |
| t08 | false | 2 | 1.160 | 0 | 1 | 1057 | 0 | 53 | 0.284 | 0.869 | 0 | exact_args mode: call 0 mismatch: expected list_rooms({}), got set_volume({"room_id":"bathroom","level":20}) |

**Summary**: correct 25.0% (8 scored, 0 skipped), mean_steps 1.88, mean_total_s 1.807, total_retries 0, total_tool_errors 3

**Per step** (15 steps): prompt_tok 528.3, think_tok 0.0, answer_tok 20.7, first_token_s 0.427, decode_s 0.527, total_s 0.954

**Failed cases, calls made and final answer**:

- t03: `get_weather({"city":"here"})`; answer: "The weather where I am is light rain with a temperature of 17°C."
- t04: `get_weather({"city":"here","unit":"celsius"})`; answer: "It is currently light rain here."
- t05: `get_weather({"city":"here","unit":"fahrenheit"})`; answer: "The temperature here is 63°F with light rain."
- t06: `set_volume({"room_id":"kitchen","level":30})`; answer: "I'm sorry, but I don't know the room ID for the kitchen. Could you please specify which room it is?"
- t07: `set_volume({"room_id":"living_room","level":10})`; answer: "I'm sorry, but I don't see a \"living_room\" room ID in the list of rooms. Could you please provide the room ID for the living room?"
- t08: `set_volume({"room_id":"bathroom","level":20})`; answer: "I'm sorry, but I don't know the room ID for the bathroom. Could you please specify which room it is?"
