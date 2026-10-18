# Two-step tool-chaining fixture set

A small fixture set that is not about Sonos: 4 tools and 8 cases that test
whether a model can chain a discovery call (`get_location`, `list_rooms`)
into the action call it was for (`get_weather`, `set_volume`), instead of
guessing the action call's argument.

## Files

- `tools.json`: the 4 tools, MCP `tools/list` shaped
  (`[{"name", "description", "inputSchema"}, ...]`).
- `cases.json`: the 8 cases, shaped like `EvalCase` in `crates/agent/src/eval.rs`:
  `id`, `utterance`, `expected` (the ordered call sequence), `accept`
  (always `exact_args` here), and `answer_contains_any` /
  `answer_contains_all` (substrings the final text answer must contain,
  case-insensitive).
- `results/get_location.json`, `results/list_rooms.json`: the mock results
  of the two tools without arguments, for reading. The eval serves all four
  tools with `TwoStepCaller`, because the results of `get_weather` and
  `set_volume` depend on their arguments.

## Mocks

- `get_location()` returns `{"city": "Lyon", "country": "FR"}`.
- `get_weather(city, unit="celsius")` returns `{"city": <city>, "temp": 17
  (63 if unit is "fahrenheit"), "condition": "light rain"}` for any city.
- `list_rooms()` returns `[{"id": "RINCON_K1", "name": "Kitchen"}, {"id":
  "RINCON_L2", "name": "Living room"}]`.
- `set_volume(room_id, level)` returns `{"ok": true, "room_id": <room_id>,
  "level": <level>}` for `RINCON_K1` and `RINCON_L2`, else `{"error":
  "unknown room_id"}`.

## Scoring

`exact_args` requires exactly the expected number of calls, in order, each
with the expected name and arguments (extra keys the model also passed are
ignored; string argument values are compared case-insensitively).
`answer_contains_any` and `answer_contains_all` are checked against the
final text answer on top of that, when present.

Case `t08` ("Set the bathroom to 20.") expects exactly one call
(`list_rooms`): a `set_volume` call for a nonexistent room, even one that
gets back `{"error": "unknown room_id"}`, is a failure, because
`list_rooms` already rules the room out.
