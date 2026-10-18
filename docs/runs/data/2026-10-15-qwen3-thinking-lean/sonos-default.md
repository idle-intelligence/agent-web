# Agent eval, Qwen3-1.7B-Q8_0, unconstrained, 2026-10-15

## Run: Qwen3-1.7B-Q8_0, 13 tools, native, webgpu, label=sonos-subset

- system: "You are a helpful home assistant with access to Sonos speaker controls."
- tool_count: 13
- max_new_tokens: 1024
- max_steps: 6

| id | correct | steps | total_s | retries | tool_errors | prompt_tok | think_tok | answer_tok | first_token_s | decode_s | capped | reason |
|----|---------|-------|---------|---------|-------------|------------|-----------|------------|---------------|----------|--------|--------|
| s01 | true | 3 | 92.528 | 0 | 0 | 7357 | 1362 | 146 | 19.475 | 73.026 | 0 | final read made with correct args |
| s02 | true | 2 | 40.899 | 0 | 0 | 4706 | 595 | 167 | 4.461 | 36.430 | 0 | matched expected sequence exactly |
| s03 | true | 3 | 53.016 | 0 | 0 | 7381 | 830 | 156 | 6.458 | 46.546 | 0 | final read made with correct args |
| s04 | skipped | - | - | - | - | - | - | - | - | - | - | subset_ok=false: not solvable under the 13-tool subset |
| s05 | skipped | - | - | - | - | - | - | - | - | - | - | subset_ok=false: not solvable under the 13-tool subset |
| s06 | true | 3 | 53.918 | 1 | 0 | 9953 | 907 | 110 | 6.427 | 47.475 | 0 | all required (mutating) calls made in order with correct args |
| s07 | false | 1 | 11.705 | 0 | 0 | 2170 | 224 | 28 | 0.347 | 11.354 | 0 | missing required call skip_to_previous_track({"group_id":"RINCON_LIVING01:2"}) in order after step 0 |
| s08 | true | 3 | 86.588 | 0 | 0 | 7305 | 1585 | 91 | 5.454 | 81.119 | 0 | all required (mutating) calls made in order with correct args |
| s09 | false | 1 | 15.574 | 0 | 0 | 2166 | 299 | 29 | 0.225 | 15.345 | 0 | missing required call resume({"group_id":"RINCON_LIVING01:2"}) in order after step 0 |
| s10 | skipped | - | - | - | - | - | - | - | - | - | - | subset_ok=false: not solvable under the 13-tool subset |
| m01 | true | 3 | 40.269 | 0 | 0 | 7306 | 675 | 57 | 5.684 | 34.574 | 0 | all required (mutating) calls made in order with correct args |
| m02 | true | 3 | 47.427 | 0 | 0 | 7330 | 798 | 97 | 5.682 | 41.733 | 0 | all required (mutating) calls made in order with correct args |
| m03 | true | 3 | 37.767 | 0 | 0 | 7315 | 710 | 90 | 4.649 | 33.102 | 0 | all required (mutating) calls made in order with correct args |
| m04 | true | 3 | 103.647 | 1 | 0 | 9515 | 1844 | 105 | 6.229 | 97.398 | 0 | all required (mutating) calls made in order with correct args |
| m05 | true | 4 | 82.863 | 0 | 0 | 10150 | 1482 | 127 | 7.761 | 75.085 | 0 | all required (mutating) calls made in order with correct args |
| m06 | skipped | - | - | - | - | - | - | - | - | - | - | subset_ok=false: not solvable under the 13-tool subset |
| m07 | true | 3 | 41.498 | 0 | 0 | 7324 | 887 | 69 | 4.570 | 36.917 | 0 | final read made with correct args |
| m08 | false | 5 | 216.381 | 3 | 0 | 23426 | 4771 | 237 | 8.948 | 207.400 | 0 | missing required call adjust_group_volume({"group_id":"RINCON_KITCHEN01:1","volume_delta":10}) in order after step 0 |
| m09 | true | 3 | 67.134 | 0 | 0 | 7317 | 1433 | 104 | 4.669 | 62.454 | 0 | all required (mutating) calls made in order with correct args |
| m10 | skipped | - | - | - | - | - | - | - | - | - | - | subset_ok=false: not solvable under the 13-tool subset |
| m11 | false | 3 | 44.865 | 0 | 0 | 7392 | 861 | 134 | 5.385 | 39.469 | 0 | missing required call play_artist({"group_id":"RINCON_KITCHEN01:1","artist":"Nirvana","shuffle":false}) in order after step 0 |
| m12 | skipped | - | - | - | - | - | - | - | - | - | - | subset_ok=false: not solvable under the 13-tool subset |
| s11 | true | 3 | 45.305 | 0 | 0 | 7327 | 885 | 132 | 4.789 | 40.506 | 0 | all required (mutating) calls made in order with correct args |
| s12 | skipped | - | - | - | - | - | - | - | - | - | - | subset_ok=false: not solvable under the 13-tool subset |
| s13 | true | 3 | 53.754 | 0 | 0 | 7315 | 889 | 106 | 6.110 | 47.622 | 0 | all required (mutating) calls made in order with correct args |
| s14 | false | 2 | 39.802 | 0 | 0 | 4710 | 612 | 87 | 4.694 | 35.097 | 0 | final read get_now_playing({"group_id":"RINCON_BEDROOM01:3"}) never made with correct args |
| s15 | false | 2 | 83.930 | 1 | 0 | 6899 | 1665 | 65 | 4.226 | 79.693 | 0 | final read get_now_playing({"group_id":"RINCON_LIVING01:2"}) never made with correct args |
| s16 | false | 3 | 75.189 | 0 | 0 | 7318 | 1250 | 107 | 5.942 | 69.227 | 0 | missing required call adjust_group_volume({"group_id":"RINCON_LIVING01:2","volume_delta":-10}) in order after step 0 |
| s17 | true | 3 | 102.159 | 1 | 0 | 9955 | 1900 | 121 | 5.842 | 96.293 | 0 | all required (mutating) calls made in order with correct args |
| m13 | true | 4 | 152.154 | 1 | 0 | 12343 | 2709 | 207 | 8.462 | 143.671 | 0 | all required (mutating) calls made in order with correct args |
| m14 | skipped | - | - | - | - | - | - | - | - | - | - | subset_ok=false: not solvable under the 13-tool subset |
| m15 | skipped | - | - | - | - | - | - | - | - | - | - | subset_ok=false: not solvable under the 13-tool subset |
| m16 | skipped | - | - | - | - | - | - | - | - | - | - | subset_ok=false: not solvable under the 13-tool subset |
| m17 | skipped | - | - | - | - | - | - | - | - | - | - | subset_ok=false: not solvable under the 13-tool subset |
| m18 | skipped | - | - | - | - | - | - | - | - | - | - | subset_ok=false: not solvable under the 13-tool subset |
| m19 | skipped | - | - | - | - | - | - | - | - | - | - | subset_ok=false: not solvable under the 13-tool subset |
| m20 | true | 3 | 237.009 | 2 | 0 | 15261 | 5023 | 141 | 6.802 | 230.182 | 0 | all required (mutating) calls made in order with correct args |
| m21 | false | 3 | 46.243 | 0 | 0 | 7305 | 985 | 77 | 4.413 | 41.821 | 0 | no_mutation mode: mutating call pause({"group_id":"RINCON_LIVING01:2"}) was made |
| m22 | false | 1 | 8.992 | 0 | 0 | 2166 | 210 | 19 | 0.189 | 8.801 | 0 | any_play mode: no play_* call with a valid group_id was made |
| m23 | false | 4 | 134.125 | 1 | 0 | 12577 | 2989 | 142 | 6.255 | 127.852 | 0 | missing required call adjust_group_volume({"group_id":"RINCON_BEDROOM01:3","volume_delta":10}) in order after step 3 |
| m24 | false | 2 | 20.061 | 0 | 0 | 4708 | 396 | 33 | 3.625 | 16.430 | 0 | missing required call pause({"group_id":"RINCON_KITCHEN01:1"}) in order after step 0 |
| m25 | true | 3 | 94.319 | 1 | 0 | 9948 | 2103 | 67 | 5.161 | 89.145 | 0 | all required (mutating) calls made in order with correct args |
| m26 | false | 2 | 67.418 | 0 | 0 | 4710 | 1512 | 64 | 3.692 | 63.719 | 0 | missing required call adjust_group_volume({"group_id":"RINCON_LIVING01:2","volume_delta":10}) in order after step 0 |
| m27 | false | 2 | 29.423 | 0 | 0 | 4420 | 615 | 100 | 1.316 | 28.101 | 0 | missing required call play_artist({"group_id":"RINCON_KITCHEN01:1","artist":"Nirvana","shuffle":false}) in order after step 0 |
| m28 | false | 2 | 39.719 | 0 | 0 | 4710 | 855 | 51 | 3.687 | 36.026 | 0 | final read get_now_playing({"group_id":"RINCON_KITCHEN01:1"}) never made with correct args |
| s18 | true | 1 | 24.934 | 0 | 0 | 2168 | 285 | 341 | 0.282 | 24.649 | 0 | no mutating call made; final text answer given |

**Summary**: correct 57.6% (33 scored, 13 skipped), mean_steps 2.70, mean_total_s 69.413, total_retries 12, total_tool_errors 0

**Per step** (89 steps): prompt_tok 2830.9, think_tok 496.0, answer_tok 40.5, first_token_s 1.932, decode_s 23.801, total_s 25.732
