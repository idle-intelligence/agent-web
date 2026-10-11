# Sonos MCP agent eval, Qwen2.5-3B-Instruct Q4_0, CPU, unconstrained, 2026-10-07

46 cases from `eval/utterances.json`: 33 scored with the 13-tool set, 13 skipped because they need a tool outside it.

## Run: Qwen2.5-3B-Instruct-q4_0.gguf (lean, CPU, unconstrained), 13 tools, native, label=sonos-subset

- system: "You are a helpful home assistant with access to Sonos speaker controls."
- tool_count: 13
- max_new_tokens: 256
- max_steps: 6

| id | correct | steps | total_s | retries | tool_errors | reason |
|----|---------|-------|---------|---------|-------------|--------|
| s01 | false | 2 | 463.213 | 0 | 0 | final read get_now_playing({"group_id":"RINCON_LIVING01:2"}) never made with correct args |
| s02 | true | 2 | 540.176 | 0 | 0 | matched expected sequence exactly |
| s03 | false | 2 | 493.883 | 0 | 0 | final read get_sonos_favorites({"household_id":"Sonos_ABC123XYZ"}) never made with correct args |
| s04 | skipped | - | - | - | - | subset_ok=false: not solvable under the 13-tool subset |
| s05 | skipped | - | - | - | - | subset_ok=false: not solvable under the 13-tool subset |
| s06 | false | 2 | 445.383 | 0 | 0 | missing required call skip_to_next_track({"group_id":"RINCON_LIVING01:2"}) in order after step 0 |
| s07 | false | 1 | 217.598 | 0 | 0 | missing required call skip_to_previous_track({"group_id":"RINCON_LIVING01:2"}) in order after step 0 |
| s08 | false | 2 | 443.362 | 0 | 0 | missing required call pause({"group_id":"RINCON_LIVING01:2"}) in order after step 0 |
| s09 | false | 2 | 439.755 | 0 | 0 | missing required call resume({"group_id":"RINCON_LIVING01:2"}) in order after step 0 |
| s10 | skipped | - | - | - | - | subset_ok=false: not solvable under the 13-tool subset |
| m01 | false | 2 | 443.202 | 0 | 0 | missing required call pause({"group_id":"RINCON_KITCHEN01:1"}) in order after step 0 |
| m02 | false | 2 | 458.214 | 0 | 0 | missing required call set_group_volume({"group_id":"RINCON_LIVING01:2","volume":20}) in order after step 0 |
| m03 | false | 2 | 446.979 | 0 | 0 | missing required call set_group_mute({"group_id":"RINCON_BEDROOM01:3","muted":true}) in order after step 0 |
| m04 | false | 3 | 840.498 | 0 | 0 | missing required call skip_to_next_track({"group_id":"RINCON_KITCHEN01:1"}) in order after step 0 |
| m05 | false | 2 | 566.183 | 0 | 0 | missing required call play_sonos_favorite({"group_id":"RINCON_LIVING01:2","favorite_id":"FV:1","shuffle":false}) in order after step 0 |
| m06 | skipped | - | - | - | - | subset_ok=false: not solvable under the 13-tool subset |
| m07 | false | 2 | 447.899 | 0 | 0 | final read get_group_volume({"group_id":"RINCON_BEDROOM01:3"}) never made with correct args |
| m08 | false | 2 | 455.013 | 0 | 0 | missing required call adjust_group_volume({"group_id":"RINCON_KITCHEN01:1","volume_delta":10}) in order after step 0 |
| m09 | false | 2 | 542.481 | 0 | 0 | missing required call set_group_mute({"group_id":"RINCON_LIVING01:2","muted":false}) in order after step 0 |
| m10 | skipped | - | - | - | - | subset_ok=false: not solvable under the 13-tool subset |
| m11 | false | 3 | 875.686 | 0 | 0 | missing required call play_artist({"group_id":"RINCON_KITCHEN01:1","artist":"Nirvana","shuffle":false}) in order after step 0 |
| m12 | skipped | - | - | - | - | subset_ok=false: not solvable under the 13-tool subset |
| s11 | false | 2 | 449.862 | 0 | 0 | missing required call set_group_volume({"group_id":"RINCON_LIVING01:2","volume":35}) in order after step 0 |
| s12 | skipped | - | - | - | - | subset_ok=false: not solvable under the 13-tool subset |
| s13 | false | 3 | 1181.724 | 1 | 0 | missing required call set_group_mute({"group_id":"RINCON_KITCHEN01:1","muted":false}) in order after step 0 |
| s14 | false | 2 | 455.865 | 0 | 0 | final read get_now_playing({"group_id":"RINCON_BEDROOM01:3"}) never made with correct args |
| s15 | false | 2 | 440.646 | 0 | 0 | final read get_now_playing({"group_id":"RINCON_LIVING01:2"}) never made with correct args |
| s16 | false | 2 | 435.274 | 0 | 0 | missing required call adjust_group_volume({"group_id":"RINCON_LIVING01:2","volume_delta":-10}) in order after step 0 |
| s17 | false | 2 | 518.446 | 0 | 0 | missing required call skip_to_next_track({"group_id":"RINCON_KITCHEN01:1"}) in order after step 0 |
| m13 | true | 3 | 915.273 | 0 | 0 | all required (mutating) calls made in order with correct args |
| m14 | skipped | - | - | - | - | subset_ok=false: not solvable under the 13-tool subset |
| m15 | skipped | - | - | - | - | subset_ok=false: not solvable under the 13-tool subset |
| m16 | skipped | - | - | - | - | subset_ok=false: not solvable under the 13-tool subset |
| m17 | skipped | - | - | - | - | subset_ok=false: not solvable under the 13-tool subset |
| m18 | skipped | - | - | - | - | subset_ok=false: not solvable under the 13-tool subset |
| m19 | skipped | - | - | - | - | subset_ok=false: not solvable under the 13-tool subset |
| m20 | false | 2 | 441.131 | 0 | 0 | missing required call pause({"group_id":"RINCON_LIVING01:2"}) in order after step 0 |
| m21 | false | 2 | 421.905 | 0 | 0 | no_mutation mode: mutating call pause({"group_id":"garage"}) was made |
| m22 | false | 1 | 231.110 | 0 | 0 | any_play mode: no play_* call with a valid group_id was made |
| m23 | false | 2 | 512.525 | 0 | 0 | missing required call pause({"group_id":"RINCON_KITCHEN01:1"}) in order after step 0 |
| m24 | false | 2 | 422.575 | 0 | 0 | missing required call pause({"group_id":"RINCON_KITCHEN01:1"}) in order after step 0 |
| m25 | false | 2 | 511.158 | 0 | 0 | missing required call pause({"group_id":"RINCON_KITCHEN01:1"}) in order after step 0 |
| m26 | false | 2 | 531.815 | 0 | 0 | missing required call adjust_group_volume({"group_id":"RINCON_LIVING01:2","volume_delta":10}) in order after step 0 |
| m27 | false | 2 | 505.882 | 0 | 0 | missing required call play_artist({"group_id":"RINCON_KITCHEN01:1","artist":"Nirvana","shuffle":false}) in order after step 0 |
| m28 | false | 2 | 441.762 | 0 | 0 | final read get_now_playing({"group_id":"RINCON_KITCHEN01:1"}) never made with correct args |
| s18 | true | 1 | 214.497 | 0 | 0 | no mutating call made; final text answer given |

**Summary**: correct 9.1% (33 scored, 13 skipped), mean_steps 2.03, mean_total_s 507.605, total_retries 1, total_tool_errors 0
