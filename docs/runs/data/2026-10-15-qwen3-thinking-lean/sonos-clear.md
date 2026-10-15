# Agent eval, Qwen3-1.7B-Q8_0, unconstrained, 2026-10-15

## Run: Qwen3-1.7B-Q8_0, 13 tools, native, webgpu, label=sonos-subset

- system: "You are a helpful assistant. Use the available tools to get information and to act; call tools as many times as needed, one after another, and never claim you lack data that a tool can provide."
- tool_count: 13
- max_new_tokens: 1024
- max_steps: 6

| id | correct | steps | total_s | retries | tool_errors | prompt_tok | think_tok | answer_tok | first_token_s | decode_s | capped | reason |
|----|---------|-------|---------|---------|-------------|------------|-----------|------------|---------------|----------|--------|--------|
| s01 | false | 1 | 51.713 | 0 | 0 | 2196 | 1023 | 1 | 11.265 | 40.445 | 0 | final read get_now_playing({"group_id":"RINCON_LIVING01:2"}) never made with correct args |
| s02 | true | 2 | 33.343 | 0 | 0 | 4760 | 498 | 248 | 3.717 | 29.620 | 0 | matched expected sequence exactly |
| s03 | false | 1 | 6.993 | 0 | 0 | 2197 | 142 | 32 | 0.309 | 6.682 | 0 | final read get_sonos_favorites({"household_id":"Sonos_ABC123XYZ"}) never made with correct args |
| s04 | skipped | - | - | - | - | - | - | - | - | - | - | subset_ok=false: not solvable under the 13-tool subset |
| s05 | skipped | - | - | - | - | - | - | - | - | - | - | subset_ok=false: not solvable under the 13-tool subset |
| s06 | true | 3 | 30.655 | 0 | 0 | 7394 | 576 | 91 | 4.715 | 25.930 | 0 | all required (mutating) calls made in order with correct args |
| s07 | true | 3 | 33.945 | 0 | 0 | 7397 | 653 | 93 | 4.675 | 29.261 | 0 | all required (mutating) calls made in order with correct args |
| s08 | true | 3 | 63.393 | 0 | 0 | 7386 | 1374 | 87 | 4.467 | 58.915 | 0 | all required (mutating) calls made in order with correct args |
| s09 | true | 3 | 31.586 | 0 | 0 | 7383 | 604 | 93 | 4.403 | 27.173 | 0 | all required (mutating) calls made in order with correct args |
| s10 | skipped | - | - | - | - | - | - | - | - | - | - | subset_ok=false: not solvable under the 13-tool subset |
| m01 | true | 3 | 46.111 | 0 | 0 | 7387 | 990 | 57 | 4.698 | 41.403 | 0 | all required (mutating) calls made in order with correct args |
| m02 | true | 3 | 34.583 | 0 | 0 | 7411 | 672 | 85 | 4.772 | 29.801 | 0 | all required (mutating) calls made in order with correct args |
| m03 | true | 3 | 28.399 | 0 | 0 | 7396 | 522 | 91 | 4.704 | 23.686 | 0 | all required (mutating) calls made in order with correct args |
| m04 | true | 3 | 59.859 | 0 | 0 | 7398 | 1020 | 114 | 5.056 | 54.785 | 0 | all required (mutating) calls made in order with correct args |
| m05 | true | 4 | 75.956 | 0 | 0 | 10258 | 1164 | 157 | 8.241 | 67.689 | 0 | all required (mutating) calls made in order with correct args |
| m06 | skipped | - | - | - | - | - | - | - | - | - | - | subset_ok=false: not solvable under the 13-tool subset |
| m07 | true | 3 | 55.910 | 0 | 0 | 7405 | 900 | 69 | 6.184 | 49.706 | 0 | final read made with correct args |
| m08 | false | 5 | 418.834 | 5 | 0 | 28937 | 7447 | 240 | 12.777 | 405.986 | 0 | missing required call adjust_group_volume({"group_id":"RINCON_KITCHEN01:1","volume_delta":10}) in order after step 0 |
| m09 | true | 4 | 140.215 | 0 | 0 | 10104 | 2361 | 123 | 7.367 | 132.819 | 0 | all required (mutating) calls made in order with correct args |
| m10 | skipped | - | - | - | - | - | - | - | - | - | - | subset_ok=false: not solvable under the 13-tool subset |
| m11 | true | 3 | 74.366 | 0 | 0 | 7416 | 1275 | 80 | 6.172 | 68.174 | 0 | all required (mutating) calls made in order with correct args |
| m12 | skipped | - | - | - | - | - | - | - | - | - | - | subset_ok=false: not solvable under the 13-tool subset |
| s11 | true | 3 | 44.419 | 0 | 0 | 7408 | 652 | 99 | 6.134 | 38.264 | 0 | all required (mutating) calls made in order with correct args |
| s12 | skipped | - | - | - | - | - | - | - | - | - | - | subset_ok=false: not solvable under the 13-tool subset |
| s13 | true | 3 | 52.937 | 0 | 0 | 7396 | 824 | 118 | 5.953 | 46.969 | 0 | all required (mutating) calls made in order with correct args |
| s14 | false | 2 | 37.800 | 0 | 0 | 4764 | 614 | 40 | 4.611 | 33.175 | 0 | final read get_now_playing({"group_id":"RINCON_BEDROOM01:3"}) never made with correct args |
| s15 | false | 2 | 38.706 | 0 | 0 | 4490 | 648 | 96 | 1.877 | 36.817 | 0 | final read get_now_playing({"group_id":"RINCON_LIVING01:2"}) never made with correct args |
| s16 | false | 3 | 78.521 | 0 | 0 | 7399 | 1486 | 114 | 4.868 | 73.642 | 0 | missing required call adjust_group_volume({"group_id":"RINCON_LIVING01:2","volume_delta":-10}) in order after step 0 |
| s17 | true | 3 | 46.405 | 0 | 0 | 7395 | 871 | 97 | 4.857 | 41.535 | 0 | all required (mutating) calls made in order with correct args |
| m13 | true | 4 | 69.330 | 0 | 0 | 10251 | 1076 | 167 | 8.074 | 61.238 | 0 | all required (mutating) calls made in order with correct args |
| m14 | skipped | - | - | - | - | - | - | - | - | - | - | subset_ok=false: not solvable under the 13-tool subset |
| m15 | skipped | - | - | - | - | - | - | - | - | - | - | subset_ok=false: not solvable under the 13-tool subset |
| m16 | skipped | - | - | - | - | - | - | - | - | - | - | subset_ok=false: not solvable under the 13-tool subset |
| m17 | skipped | - | - | - | - | - | - | - | - | - | - | subset_ok=false: not solvable under the 13-tool subset |
| m18 | skipped | - | - | - | - | - | - | - | - | - | - | subset_ok=false: not solvable under the 13-tool subset |
| m19 | skipped | - | - | - | - | - | - | - | - | - | - | subset_ok=false: not solvable under the 13-tool subset |
| m20 | false | 3 | 214.955 | 0 | 0 | 4756 | 945 | 49 | 4.401 | 47.875 | 0 | agent error: model produced no valid call after 2 retries: reply ended inside the thinking block (4545 bytes of reasoning, no </think>) |
| m21 | false | 2 | 22.307 | 0 | 0 | 4445 | 385 | 56 | 1.430 | 20.870 | 0 | no_mutation mode: mutating call pause({"group_id":"garage"}) was made |
| m22 | false | 1 | 11.884 | 0 | 0 | 2193 | 189 | 58 | 0.237 | 11.644 | 0 | any_play mode: no play_* call with a valid group_id was made |
| m23 | false | 4 | 239.073 | 2 | 0 | 15458 | 4321 | 192 | 9.079 | 229.966 | 0 | missing required call adjust_group_volume({"group_id":"RINCON_BEDROOM01:3","volume_delta":10}) in order after step 2 |
| m24 | true | 3 | 55.297 | 0 | 0 | 7393 | 950 | 80 | 5.787 | 49.498 | 0 | all required (mutating) calls made in order with correct args |
| m25 | true | 3 | 58.332 | 0 | 0 | 7396 | 1020 | 80 | 5.768 | 52.552 | 0 | all required (mutating) calls made in order with correct args |
| m26 | false | 2 | 41.900 | 0 | 0 | 4764 | 722 | 62 | 4.473 | 37.420 | 0 | missing required call adjust_group_volume({"group_id":"RINCON_LIVING01:2","volume_delta":10}) in order after step 0 |
| m27 | false | 2 | 28.597 | 0 | 0 | 4474 | 488 | 84 | 1.623 | 26.966 | 0 | missing required call play_artist({"group_id":"RINCON_KITCHEN01:1","artist":"Nirvana","shuffle":false}) in order after step 0 |
| m28 | true | 3 | 53.271 | 0 | 0 | 7442 | 824 | 129 | 6.208 | 47.050 | 0 | final read made with correct args |
| s18 | true | 1 | 24.169 | 0 | 0 | 2195 | 231 | 259 | 0.357 | 23.808 | 0 | no mutating call made; final text answer given |

**Summary**: correct 63.6% (33 scored, 13 skipped), mean_steps 2.76, mean_total_s 69.811, total_retries 7, total_tool_errors 0

**Per step** (90 steps): prompt_tok 2689.4, think_tok 416.3, answer_tok 38.2, first_token_s 1.881, decode_s 21.904, total_s 23.785
