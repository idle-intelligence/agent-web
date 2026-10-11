"""Reference fixture for the agent's Qwen2.5-Instruct tools-template prompts.

Renders `tokenizer.apply_chat_template(messages, tools=..., add_generation_prompt=True)`
through HF `transformers` itself (tokenizer only, no model forward pass),
for the input cases in `agent_inputs/*.json` (OpenAI-shaped messages, which
Qwen2.5's own template accepts) plus one multi-turn case generated here,
with object-shaped `tool_calls[].function.arguments` and two tool round
trips, the shape the agent loop produces.

  LEAN_TOKENIZER_DIR_QWEN25_3B  directory with tokenizer.json + tokenizer_config.json

Run it in a venv with `transformers` installed (tokenizer only, no torch
needed).
"""
import json
import os
from pathlib import Path

from transformers import AutoTokenizer

TOKENIZER_DIR = os.environ.get("LEAN_TOKENIZER_DIR_QWEN25_3B")
if not TOKENIZER_DIR:
    raise SystemExit("set LEAN_TOKENIZER_DIR_QWEN25_3B before running gen_fixture_agent_tools.py")

HERE = Path(__file__).parent
INPUTS_DIR = HERE / "agent_inputs"

tok = AutoTokenizer.from_pretrained(TOKENIZER_DIR)

NEW_MULTITURN_CASE = {
    "messages": [
        {"role": "system", "content": "You are a helpful home assistant with access to Sonos speaker controls."},
        {"role": "user", "content": "pause the kitchen, then tell me what's playing in the living room"},
        {
            "role": "assistant",
            "content": None,
            "tool_calls": [{"type": "function", "function": {"name": "get_households_and_groups_and_players", "arguments": {}}}],
        },
        {
            "role": "tool",
            "name": "get_households_and_groups_and_players",
            "content": json.dumps({"households": [{"householdId": "Sonos_ABC123XYZ", "groups": [{"groupId": "RINCON_KITCHEN01:1", "name": "Kitchen"}, {"groupId": "RINCON_LIVING01:2", "name": "Living Room"}]}]}),
        },
        {
            "role": "assistant",
            "content": None,
            "tool_calls": [{"type": "function", "function": {"name": "get_now_playing", "arguments": {"group_id": "RINCON_LIVING01:2"}}}],
        },
        {
            "role": "tool",
            "name": "get_now_playing",
            "content": json.dumps({"track": "Blue in Green", "artist": "Miles Davis"}),
        },
    ],
    "tools": [
        {"type": "function", "function": {"name": "get_households_and_groups_and_players", "description": "Discover Sonos households, groups and players.", "parameters": {"type": "object", "properties": {}}}},
        {"type": "function", "function": {"name": "get_now_playing", "description": "Get what is currently playing in a group.", "parameters": {"type": "object", "properties": {"group_id": {"type": "string"}}, "required": ["group_id"]}}},
    ],
}

cases = []
for path in sorted(INPUTS_DIR.glob("*.json")):
    data = json.loads(path.read_text())
    cases.append((path.stem, data["messages"], data["tools"]))
cases.append(("05_tools_multiturn_object_args", NEW_MULTITURN_CASE["messages"], NEW_MULTITURN_CASE["tools"]))

fixture = {"cases": []}
for name, messages, tools in cases:
    kwargs = {"add_generation_prompt": True, "tokenize": False}
    rendered = tok.apply_chat_template(messages, tools=tools or None, **kwargs)
    input_ids = tok(rendered, add_special_tokens=False)["input_ids"]
    fixture["cases"].append({
        "name": name,
        "messages": messages,
        "tools": tools,
        "rendered": rendered,
        "input_ids": input_ids,
    })

out_path = HERE / "fixture_agent_tools.json"
out_path.write_text(json.dumps(fixture, indent=2))
print(f"wrote {out_path} ({len(fixture['cases'])} cases)")
