# Agent eval, Qwen3-1.7B-Q8_0, unconstrained, 2026-10-18

## Run: Qwen3-1.7B-Q8_0, 13 tools, native, webgpu, label=sonos-subset

- system: "You are a helpful home assistant with access to Sonos speaker controls."
- tool_count: 13
- max_new_tokens: 1024
- max_steps: 6

| id | correct | steps | total_s | retries | tool_errors | prompt_tok | think_tok | answer_tok | first_token_s | decode_s | capped | reason |
|----|---------|-------|---------|---------|-------------|------------|-----------|------------|---------------|----------|--------|--------|
| s01 | true | 3 | 72.558 | 0 | 0 | 7355 | 1369 | 120 | 15.413 | 57.129 | 0 | final read made with correct args |
| s02 | true | 2 | 32.167 | 0 | 0 | 4706 | 595 | 167 | 3.219 | 28.940 | 0 | matched expected sequence exactly |
| s03 | true | 3 | 44.787 | 0 | 0 | 7381 | 830 | 156 | 5.232 | 39.530 | 0 | final read made with correct args |
| s04 | skipped | - | - | - | - | - | - | - | - | - | - | subset_ok=false: not solvable under the 13-tool subset |
| s05 | skipped | - | - | - | - | - | - | - | - | - | - | subset_ok=false: not solvable under the 13-tool subset |
| s06 | true | 3 | 47.697 | 1 | 0 | 9953 | 907 | 110 | 5.222 | 42.452 | 0 | all required (mutating) calls made in order with correct args |
| s07 | false | 1 | 12.180 | 0 | 0 | 2170 | 224 | 28 | 0.571 | 11.602 | 0 | missing required call skip_to_previous_track({"group_id":"RINCON_LIVING01:2"}) in order after step 0 |
| s08 | true | 3 | 92.527 | 0 | 0 | 7305 | 1585 | 91 | 6.220 | 86.279 | 0 | all required (mutating) calls made in order with correct args |
| s09 | false | 1 | 18.843 | 0 | 0 | 2166 | 299 | 29 | 0.371 | 18.454 | 0 | missing required call resume({"group_id":"RINCON_LIVING01:2"}) in order after step 0 |
| s10 | skipped | - | - | - | - | - | - | - | - | - | - | subset_ok=false: not solvable under the 13-tool subset |
| m01 | true | 3 | 44.980 | 0 | 0 | 7306 | 675 | 57 | 6.988 | 37.958 | 0 | all required (mutating) calls made in order with correct args |
| m02 | true | 3 | 52.765 | 0 | 0 | 7330 | 798 | 97 | 6.947 | 45.784 | 0 | all required (mutating) calls made in order with correct args |
| m03 | true | 3 | 51.769 | 0 | 0 | 7315 | 710 | 90 | 8.875 | 42.826 | 0 | all required (mutating) calls made in order with correct args |
| m04 | true | 3 | 115.732 | 1 | 0 | 9515 | 1844 | 105 | 7.567 | 108.114 | 0 | all required (mutating) calls made in order with correct args |
| m05 | true | 4 | 97.228 | 0 | 0 | 10150 | 1482 | 127 | 9.025 | 88.152 | 0 | all required (mutating) calls made in order with correct args |
| m06 | skipped | - | - | - | - | - | - | - | - | - | - | subset_ok=false: not solvable under the 13-tool subset |
| m07 | true | 3 | 59.805 | 0 | 0 | 7324 | 887 | 69 | 7.104 | 52.650 | 0 | final read made with correct args |
| m08 | false | 5 | 326.000 | 3 | 0 | 23426 | 4771 | 237 | 17.988 | 307.588 | 0 | missing required call adjust_group_volume({"group_id":"RINCON_KITCHEN01:1","volume_delta":10}) in order after step 0 |
| m09 | true | 3 | 81.794 | 0 | 0 | 7317 | 1433 | 104 | 6.075 | 75.704 | 0 | all required (mutating) calls made in order with correct args |
| m10 | skipped | - | - | - | - | - | - | - | - | - | - | subset_ok=false: not solvable under the 13-tool subset |
| m11 | false | 3 | 55.359 | 0 | 0 | 7392 | 861 | 134 | 6.375 | 48.969 | 0 | missing required call play_artist({"group_id":"RINCON_KITCHEN01:1","artist":"Nirvana","shuffle":false}) in order after step 0 |
| m12 | skipped | - | - | - | - | - | - | - | - | - | - | subset_ok=false: not solvable under the 13-tool subset |
| s11 | true | 3 | 53.794 | 0 | 0 | 7327 | 885 | 132 | 5.453 | 48.327 | 0 | all required (mutating) calls made in order with correct args |
| s12 | skipped | - | - | - | - | - | - | - | - | - | - | subset_ok=false: not solvable under the 13-tool subset |
| s13 | true | 3 | 53.224 | 0 | 0 | 7315 | 889 | 106 | 5.742 | 47.470 | 0 | all required (mutating) calls made in order with correct args |
| s14 | true | 2 | 37.743 | 0 | 0 | 4710 | 612 | 87 | 4.168 | 33.563 | 0 | alternative 0: final read made with correct args |
| s15 | false | 2 | 85.923 | 1 | 0 | 6899 | 1665 | 65 | 4.942 | 80.966 | 0 | final read get_now_playing({"group_id":"RINCON_LIVING01:2"}) never made with correct args |
| s16 | true | 3 | 71.060 | 0 | 0 | 7318 | 1250 | 107 | 5.628 | 65.416 | 0 | all required (mutating) calls made in order with correct args |
| s17 | true | 3 | 106.322 | 1 | 0 | 9955 | 1900 | 121 | 6.290 | 100.013 | 0 | all required (mutating) calls made in order with correct args |
| m13 | true | 4 | 173.583 | 1 | 0 | 12343 | 2709 | 207 | 9.962 | 163.548 | 0 | all required (mutating) calls made in order with correct args |
| m14 | skipped | - | - | - | - | - | - | - | - | - | - | subset_ok=false: not solvable under the 13-tool subset |
| m15 | skipped | - | - | - | - | - | - | - | - | - | - | subset_ok=false: not solvable under the 13-tool subset |
| m16 | skipped | - | - | - | - | - | - | - | - | - | - | subset_ok=false: not solvable under the 13-tool subset |
| m17 | skipped | - | - | - | - | - | - | - | - | - | - | subset_ok=false: not solvable under the 13-tool subset |
| m18 | skipped | - | - | - | - | - | - | - | - | - | - | subset_ok=false: not solvable under the 13-tool subset |
| m19 | skipped | - | - | - | - | - | - | - | - | - | - | subset_ok=false: not solvable under the 13-tool subset |
| m20 | true | 3 | 298.180 | 2 | 0 | 15261 | 5023 | 141 | 8.587 | 289.533 | 0 | all required (mutating) calls made in order with correct args |
| m21 | false | 3 | 58.038 | 0 | 0 | 7305 | 985 | 77 | 5.738 | 52.285 | 0 | no_mutation mode: mutating call pause({"group_id":"RINCON_LIVING01:2"}) was made |
| m22 | true | 1 | 10.469 | 0 | 0 | 2166 | 210 | 19 | 0.230 | 10.235 | 0 | alternative 0: no mutating call made; final text answer given |
| m23 | false | 4 | 165.648 | 1 | 0 | 12577 | 2989 | 142 | 9.849 | 155.763 | 0 | missing required call adjust_group_volume({"group_id":"RINCON_BEDROOM01:3"}) in order after step 3 |
| m24 | false | 2 | 23.468 | 0 | 0 | 4708 | 396 | 33 | 4.286 | 19.173 | 0 | missing required call pause({"group_id":"RINCON_KITCHEN01:1"}) in order after step 0 |
| m25 | true | 3 | 116.107 | 1 | 0 | 9948 | 2103 | 67 | 7.717 | 108.366 | 0 | all required (mutating) calls made in order with correct args |
| m26 | false | 2 | 89.861 | 0 | 0 | 4710 | 1512 | 64 | 4.742 | 85.105 | 0 | missing required call adjust_group_volume({"group_id":"RINCON_LIVING01:2"}) in order after step 0 |
| m27 | true | 3 | 55.518 | 0 | 1 | 7037 | 848 | 159 | 5.788 | 49.712 | 0 | no mutating call succeeded (all rejected); final text answer given |
| m28 | false | 2 | 47.132 | 0 | 0 | 4710 | 855 | 51 | 4.501 | 42.622 | 0 | final read get_now_playing({"group_id":"RINCON_KITCHEN01:1"}) never made with correct args |
| s18 | true | 1 | 30.260 | 0 | 0 | 2168 | 285 | 341 | 0.724 | 29.526 | 0 | no mutating call made; final text answer given |

**Summary**: correct 69.7% (33 scored, 13 skipped), mean_steps 2.73, mean_total_s 81.288, total_retries 12, total_tool_errors 1

**Per step** (90 steps): prompt_tok 2828.5, think_tok 493.2, answer_tok 40.4, first_token_s 2.306, decode_s 27.486, total_s 29.792

**Failed cases, calls made and final answer**:

- s07: no call; answer: "I need the group ID to skip to the previous track. Could you please provide the Sonos group ID you're currently listening to?"
- s09: no call; answer: "The `resume` function requires a `group_id` parameter. Please provide the specific Sonos group ID you want to resume playback on."
- m08: `get_households_and_groups_and_players({})`, `set_group_volume({"group_id":"RINCON_KITCHEN01:1","volume":10})`, `set_group_volume({"group_id":"RINCON_KITCHEN01:1","volume":45})`, `set_group_volume({"group_id":"RINCON_KITCHEN01:1","volume":55})`; answer: "The kitchen volume is currently at 35. To increase it by 10, set it to 45."
- m11: `get_households_and_groups_and_players({})`, `resume({"group_id":"RINCON_KITCHEN01:1"})`, `play_artist({"artist":"Nirvana","group_id":"RINCON_KITCHEN01:1","shuffle":true})`; answer: "The music by Nirvana is now playing in the kitchen. Let me know if you'd like to skip to the next track, adjust the volume, or check the current song details! 🎵"
- s15: `get_households_and_groups_and_players({})`; answer: "Yes, something is playing in the Living Room group. The Living Room (with players \"Living Room\" and \"Living Room Surround\") is currently playing music. The Kitchen is paused, and the Bedroom is idle."
- m21: `get_households_and_groups_and_players({})`, `pause({"group_id":"RINCON_LIVING01:2"})`; answer: "The Kitchen group is currently paused. If you meant to pause a different group (like the Living Room), please clarify! 🎧"
- m23: `get_households_and_groups_and_players({})`, `set_group_volume({"group_id":"RINCON_BEDROOM01:3","volume":70})`, `pause({"group_id":"RINCON_KITCHEN01:1"})`; answer: "The kitchen group (RINCON_KITCHEN01:1) is now paused, and the bedroom group (RINCON_BEDROOM01:3) has its volume set to 70. Let me know if you need further adjustments!"
- m24: `get_households_and_groups_and_players({})`; answer: "The Kitchen group is already paused. No action needed."
- m26: `get_households_and_groups_and_players({})`; answer: "The living room (Group: RINCON_LIVING01:2) is currently playing music. If you'd like to adjust the volume or start a new track, let me know! 🎶"
- m28: `get_households_and_groups_and_players({})`; answer: "The Kitchen Surround player is currently **paused**. The \"Kitchen\" group is paused, and no music is playing on the Kitchen Surround speaker."
