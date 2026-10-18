# Agent eval, Qwen3-1.7B-Q8_0, unconstrained, 2026-10-15

## Run: Qwen3-1.7B-Q8_0, 4 tools, native, webgpu, label=two-step

- system: "You are a helpful assistant. Use the available tools to get information and to act; call tools as many times as needed, one after another, and never claim you lack data that a tool can provide."
- tool_count: 4
- max_new_tokens: 1024
- max_steps: 6

| id | correct | steps | total_s | retries | tool_errors | prompt_tok | think_tok | answer_tok | first_token_s | decode_s | capped | reason |
|----|---------|-------|---------|---------|-------------|------------|-----------|------------|---------------|----------|--------|--------|
| t01 | true | 1 | 2.063 | 0 | 0 | 504 | 0 | 8 | 1.802 | 0.261 | 0 | matched expected call sequence |
| t02 | true | 2 | 1.461 | 0 | 0 | 1058 | 0 | 33 | 0.480 | 0.979 | 0 | matched expected call sequence |
| t03 | false | 2 | 2.272 | 0 | 0 | 1093 | 0 | 56 | 0.651 | 1.619 | 0 | exact_args mode: call 1 mismatch: expected get_weather({"city":"Lyon"}), got get_weather({"city":"here"}) |
| t04 | false | 1 | 0.941 | 0 | 0 | 501 | 0 | 28 | 0.134 | 0.806 | 0 | exact_args mode: expected 2 call(s), got 0 |
| t05 | false | 2 | 2.243 | 0 | 0 | 1100 | 0 | 54 | 0.688 | 1.552 | 0 | exact_args mode: call 1 mismatch: expected get_weather({"city":"Lyon","unit":"fahrenheit"}), got get_weather({"city":"here","unit":"fahrenheit"}) |
| t06 | false | 1 | 0.979 | 0 | 0 | 504 | 0 | 28 | 0.176 | 0.803 | 0 | exact_args mode: expected 2 call(s), got 0 |
| t07 | false | 2 | 2.927 | 0 | 1 | 1116 | 0 | 77 | 0.729 | 2.197 | 0 | exact_args mode: call 1 mismatch: expected set_volume({"room_id":"RINCON_L2","level":10}), got set_volume({"room_id":"living room","level":10}) |
| t08 | false | 1 | 0.874 | 0 | 0 | 504 | 0 | 24 | 0.176 | 0.697 | 0 | exact_args mode: expected 1 call(s), got 0 |

**Summary**: correct 25.0% (8 scored, 0 skipped), mean_steps 1.50, mean_total_s 1.720, total_retries 0, total_tool_errors 1

**Per step** (12 steps): prompt_tok 531.7, think_tok 0.0, answer_tok 25.7, first_token_s 0.403, decode_s 0.743, total_s 1.146

**Failed cases, calls made and final answer**:

- t03: `get_location({})`, `get_weather({"city":"here"})`; answer: "The weather where you are (Lyon, France) is currently 17°C with light rain."
- t04: no call; answer: "I cannot determine if it is raining here without knowing the specific location. Could you please tell me the city or region you are referring to?"
- t05: `get_location({})`, `get_weather({"city":"here","unit":"fahrenheit"})`; answer: "The temperature here is 63°F with light rain."
- t06: no call; answer: "I need to know the room's ID for the kitchen before I can set the volume. Let me list the rooms with a speaker first."
- t07: `list_rooms({})`, `set_volume({"room_id":"living room","level":10})`; answer: "I apologize for the confusion. It seems that the room ID \"living room\" is not recognized. Could you please provide the actual room ID for the living room?"
- t08: no call; answer: "I cannot set the bathroom to 20. Please provide the room ID of the bathroom and the desired volume level."
