# Agent eval, Qwen3-1.7B-Q8_0, unconstrained, 2026-10-15

## Run: Qwen3-1.7B-Q8_0, 13 tools, native, webgpu, label=sonos-subset

- system: "You are a helpful assistant. Use the available tools to get information and to act; call tools as many times as needed, one after another, and never claim you lack data that a tool can provide."
- tool_count: 13
- max_new_tokens: 1024
- max_steps: 6

| id | correct | steps | total_s | retries | tool_errors | prompt_tok | think_tok | answer_tok | first_token_s | decode_s | capped | reason |
|----|---------|-------|---------|---------|-------------|------------|-----------|------------|---------------|----------|--------|--------|
| s01 | true | 3 | 64.025 | 0 | 0 | 7438 | 1217 | 109 | 14.721 | 49.289 | 1 | final read made with correct args |
| s02 | true | 2 | 41.276 | 0 | 0 | 4760 | 498 | 248 | 4.452 | 36.816 | 0 | matched expected sequence exactly |
| s03 | false | 1 | 9.381 | 0 | 0 | 2197 | 142 | 32 | 0.403 | 8.974 | 0 | final read get_sonos_favorites({"household_id":"Sonos_ABC123XYZ"}) never made with correct args |
| s04 | skipped | - | - | - | - | - | - | - | - | - | - | subset_ok=false: not solvable under the 13-tool subset |
| s05 | skipped | - | - | - | - | - | - | - | - | - | - | subset_ok=false: not solvable under the 13-tool subset |
| s06 | true | 3 | 38.819 | 0 | 0 | 7394 | 576 | 91 | 6.026 | 32.780 | 0 | all required (mutating) calls made in order with correct args |
| s07 | true | 3 | 42.265 | 0 | 0 | 7397 | 653 | 93 | 5.423 | 36.673 | 0 | all required (mutating) calls made in order with correct args |
| s08 | true | 3 | 62.669 | 0 | 0 | 7386 | 1232 | 89 | 5.350 | 57.308 | 1 | all required (mutating) calls made in order with correct args |
| s09 | true | 3 | 31.838 | 0 | 0 | 7383 | 604 | 93 | 4.424 | 27.406 | 0 | all required (mutating) calls made in order with correct args |
| s10 | skipped | - | - | - | - | - | - | - | - | - | - | subset_ok=false: not solvable under the 13-tool subset |
| m01 | true | 3 | 56.179 | 0 | 0 | 7387 | 990 | 57 | 5.288 | 50.775 | 0 | all required (mutating) calls made in order with correct args |
| m02 | true | 3 | 34.439 | 0 | 0 | 7411 | 672 | 85 | 4.664 | 29.766 | 0 | all required (mutating) calls made in order with correct args |
| m03 | true | 3 | 32.735 | 0 | 0 | 7396 | 522 | 91 | 5.044 | 27.679 | 0 | all required (mutating) calls made in order with correct args |
| m04 | true | 3 | 58.949 | 0 | 0 | 7398 | 1020 | 54 | 5.887 | 53.049 | 0 | all required (mutating) calls made in order with correct args |
| m05 | true | 4 | 99.240 | 1 | 0 | 12487 | 1677 | 175 | 8.263 | 90.862 | 1 | all required (mutating) calls made in order with correct args |
| m06 | skipped | - | - | - | - | - | - | - | - | - | - | subset_ok=false: not solvable under the 13-tool subset |
| m07 | true | 3 | 43.197 | 0 | 0 | 7405 | 900 | 69 | 4.763 | 38.416 | 0 | final read made with correct args |
| m08 | false | 2 | 39.978 | 0 | 0 | 4465 | 771 | 74 | 1.772 | 38.197 | 1 | missing required call adjust_group_volume({"group_id":"RINCON_KITCHEN01:1","volume_delta":10}) in order after step 0 |
| m09 | true | 4 | 95.878 | 0 | 0 | 10104 | 1653 | 120 | 9.156 | 86.632 | 2 | all required (mutating) calls made in order with correct args |
| m10 | skipped | - | - | - | - | - | - | - | - | - | - | subset_ok=false: not solvable under the 13-tool subset |
| m11 | true | 3 | 56.123 | 0 | 0 | 7416 | 1207 | 80 | 4.970 | 51.142 | 1 | all required (mutating) calls made in order with correct args |
| m12 | skipped | - | - | - | - | - | - | - | - | - | - | subset_ok=false: not solvable under the 13-tool subset |
| s11 | true | 3 | 44.077 | 0 | 0 | 7408 | 652 | 99 | 5.361 | 38.649 | 0 | all required (mutating) calls made in order with correct args |
| s12 | skipped | - | - | - | - | - | - | - | - | - | - | subset_ok=false: not solvable under the 13-tool subset |
| s13 | true | 3 | 41.747 | 0 | 0 | 7396 | 824 | 118 | 4.676 | 37.061 | 0 | all required (mutating) calls made in order with correct args |
| s14 | false | 2 | 33.834 | 0 | 0 | 4764 | 614 | 40 | 4.346 | 29.480 | 0 | final read get_now_playing({"group_id":"RINCON_BEDROOM01:3"}) never made with correct args |
| s15 | false | 2 | 61.944 | 1 | 0 | 6980 | 1035 | 134 | 5.102 | 56.829 | 0 | final read get_now_playing({"group_id":"RINCON_LIVING01:2"}) never made with correct args |
| s16 | false | 3 | 70.873 | 0 | 0 | 7399 | 1329 | 114 | 5.937 | 64.925 | 1 | missing required call adjust_group_volume({"group_id":"RINCON_LIVING01:2","volume_delta":-10}) in order after step 0 |
| s17 | true | 3 | 52.112 | 0 | 0 | 7395 | 871 | 97 | 5.033 | 47.069 | 0 | all required (mutating) calls made in order with correct args |
| m13 | true | 4 | 69.110 | 0 | 0 | 10251 | 1076 | 167 | 7.998 | 61.094 | 0 | all required (mutating) calls made in order with correct args |
| m14 | skipped | - | - | - | - | - | - | - | - | - | - | subset_ok=false: not solvable under the 13-tool subset |
| m15 | skipped | - | - | - | - | - | - | - | - | - | - | subset_ok=false: not solvable under the 13-tool subset |
| m16 | skipped | - | - | - | - | - | - | - | - | - | - | subset_ok=false: not solvable under the 13-tool subset |
| m17 | skipped | - | - | - | - | - | - | - | - | - | - | subset_ok=false: not solvable under the 13-tool subset |
| m18 | skipped | - | - | - | - | - | - | - | - | - | - | subset_ok=false: not solvable under the 13-tool subset |
| m19 | skipped | - | - | - | - | - | - | - | - | - | - | subset_ok=false: not solvable under the 13-tool subset |
| m20 | true | 4 | 150.751 | 3 | 0 | 20989 | 2807 | 235 | 9.834 | 140.873 | 3 | all required (mutating) calls made in order with correct args |
| m21 | false | 2 | 18.137 | 0 | 0 | 4445 | 385 | 56 | 1.129 | 17.002 | 0 | no_mutation mode: mutating call pause({"group_id":"garage"}) was made |
| m22 | false | 1 | 9.768 | 0 | 0 | 2193 | 189 | 58 | 0.181 | 9.584 | 0 | any_play mode: no play_* call with a valid group_id was made |
| m23 | false | 3 | 168.223 | 3 | 0 | 17748 | 2993 | 284 | 9.637 | 158.547 | 2 | missing required call pause({"group_id":"RINCON_KITCHEN01:1"}) in order after step 0 |
| m24 | true | 3 | 45.633 | 0 | 0 | 7393 | 950 | 80 | 4.705 | 40.919 | 0 | all required (mutating) calls made in order with correct args |
| m25 | true | 3 | 56.518 | 0 | 0 | 7396 | 1020 | 80 | 5.082 | 51.416 | 0 | all required (mutating) calls made in order with correct args |
| m26 | false | 2 | 34.539 | 0 | 0 | 4764 | 722 | 51 | 3.735 | 30.799 | 0 | missing required call adjust_group_volume({"group_id":"RINCON_LIVING01:2","volume_delta":10}) in order after step 0 |
| m27 | false | 2 | 24.297 | 0 | 0 | 4474 | 488 | 84 | 1.317 | 22.973 | 0 | missing required call play_artist({"group_id":"RINCON_KITCHEN01:1","artist":"Nirvana","shuffle":false}) in order after step 0 |
| m28 | true | 3 | 53.472 | 0 | 0 | 7442 | 824 | 129 | 6.371 | 47.088 | 0 | final read made with correct args |
| s18 | true | 1 | 23.279 | 0 | 0 | 2195 | 231 | 259 | 0.362 | 22.915 | 0 | no mutating call made; final text answer given |

**Summary**: correct 69.7% (33 scored, 13 skipped), mean_steps 2.73, mean_total_s 53.494, total_retries 8, total_tool_errors 0

**Per step** (90 steps): prompt_tok 2734.0, think_tok 348.3, answer_tok 40.5, first_token_s 1.905, decode_s 17.700, total_s 19.604
