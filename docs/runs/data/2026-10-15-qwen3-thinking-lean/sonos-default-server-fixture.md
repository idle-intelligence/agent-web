# Agent eval, Qwen3-1.7B-Q8_0, unconstrained, 2026-10-15

## Run: Qwen3-1.7B-Q8_0, 13 tools, native, webgpu, label=sonos-subset

- system: "You are a helpful home assistant with access to Sonos speaker controls."
- tool_count: 13
- max_new_tokens: 1024
- max_steps: 6

| id | correct | steps | total_s | retries | tool_errors | prompt_tok | think_tok | answer_tok | first_token_s | decode_s | capped | reason |
|----|---------|-------|---------|---------|-------------|------------|-----------|------------|---------------|----------|--------|--------|
| s01 | true | 3 | 70.438 | 0 | 0 | 7355 | 1369 | 120 | 14.344 | 56.081 | 0 | final read made with correct args |
| s02 | true | 2 | 30.141 | 0 | 0 | 4706 | 595 | 167 | 2.988 | 27.147 | 0 | matched expected sequence exactly |
| s03 | true | 3 | 50.621 | 0 | 0 | 7381 | 830 | 156 | 6.161 | 44.450 | 0 | final read made with correct args |
| s04 | skipped | - | - | - | - | - | - | - | - | - | - | subset_ok=false: not solvable under the 13-tool subset |
| s05 | skipped | - | - | - | - | - | - | - | - | - | - | subset_ok=false: not solvable under the 13-tool subset |
| s06 | true | 3 | 53.564 | 1 | 0 | 9953 | 907 | 110 | 6.671 | 46.880 | 0 | all required (mutating) calls made in order with correct args |
| s07 | false | 1 | 10.564 | 0 | 0 | 2170 | 224 | 28 | 0.315 | 10.246 | 0 | missing required call skip_to_previous_track({"group_id":"RINCON_LIVING01:2"}) in order after step 0 |
| s08 | true | 3 | 73.150 | 0 | 0 | 7305 | 1585 | 91 | 4.588 | 68.550 | 0 | all required (mutating) calls made in order with correct args |
| s09 | false | 1 | 13.024 | 0 | 0 | 2166 | 299 | 29 | 0.200 | 12.821 | 0 | missing required call resume({"group_id":"RINCON_LIVING01:2"}) in order after step 0 |
| s10 | skipped | - | - | - | - | - | - | - | - | - | - | subset_ok=false: not solvable under the 13-tool subset |
| m01 | true | 3 | 33.750 | 0 | 0 | 7306 | 675 | 57 | 4.786 | 28.954 | 0 | all required (mutating) calls made in order with correct args |
| m02 | true | 3 | 39.865 | 0 | 0 | 7330 | 798 | 97 | 4.722 | 35.133 | 0 | all required (mutating) calls made in order with correct args |
| m03 | true | 3 | 35.989 | 0 | 0 | 7315 | 710 | 90 | 4.714 | 31.264 | 0 | all required (mutating) calls made in order with correct args |
| m04 | true | 3 | 83.007 | 1 | 0 | 9515 | 1844 | 105 | 5.169 | 77.826 | 0 | all required (mutating) calls made in order with correct args |
| m05 | true | 4 | 70.933 | 0 | 0 | 10150 | 1482 | 127 | 6.555 | 64.363 | 0 | all required (mutating) calls made in order with correct args |
| m06 | skipped | - | - | - | - | - | - | - | - | - | - | subset_ok=false: not solvable under the 13-tool subset |
| m07 | true | 3 | 42.986 | 0 | 0 | 7324 | 887 | 69 | 4.831 | 38.142 | 0 | final read made with correct args |
| m08 | false | 5 | 216.710 | 3 | 0 | 23426 | 4771 | 237 | 9.091 | 207.586 | 0 | missing required call adjust_group_volume({"group_id":"RINCON_KITCHEN01:1","volume_delta":10}) in order after step 0 |
| m09 | true | 3 | 67.197 | 0 | 0 | 7317 | 1433 | 104 | 4.766 | 62.421 | 0 | all required (mutating) calls made in order with correct args |
| m10 | skipped | - | - | - | - | - | - | - | - | - | - | subset_ok=false: not solvable under the 13-tool subset |
| m11 | false | 3 | 44.832 | 0 | 0 | 7392 | 861 | 134 | 5.376 | 39.445 | 0 | missing required call play_artist({"group_id":"RINCON_KITCHEN01:1","artist":"Nirvana","shuffle":false}) in order after step 0 |
| m12 | skipped | - | - | - | - | - | - | - | - | - | - | subset_ok=false: not solvable under the 13-tool subset |
| s11 | true | 3 | 45.103 | 0 | 0 | 7327 | 885 | 132 | 4.719 | 40.374 | 0 | all required (mutating) calls made in order with correct args |
| s12 | skipped | - | - | - | - | - | - | - | - | - | - | subset_ok=false: not solvable under the 13-tool subset |
| s13 | true | 3 | 44.266 | 0 | 0 | 7315 | 889 | 106 | 4.783 | 39.473 | 0 | all required (mutating) calls made in order with correct args |
| s14 | true | 2 | 31.240 | 0 | 0 | 4710 | 612 | 87 | 3.705 | 27.529 | 0 | alternative 0: final read made with correct args |
| s15 | false | 2 | 74.240 | 1 | 0 | 6899 | 1665 | 65 | 4.301 | 69.930 | 0 | final read get_now_playing({"group_id":"RINCON_LIVING01:2"}) never made with correct args |
| s16 | true | 3 | 60.431 | 0 | 0 | 7318 | 1250 | 107 | 4.943 | 55.478 | 0 | all required (mutating) calls made in order with correct args |
| s17 | true | 3 | 88.705 | 1 | 0 | 9955 | 1900 | 121 | 5.517 | 83.175 | 0 | all required (mutating) calls made in order with correct args |
| m13 | true | 4 | 149.256 | 1 | 0 | 12343 | 2709 | 207 | 7.956 | 141.273 | 0 | all required (mutating) calls made in order with correct args |
| m14 | skipped | - | - | - | - | - | - | - | - | - | - | subset_ok=false: not solvable under the 13-tool subset |
| m15 | skipped | - | - | - | - | - | - | - | - | - | - | subset_ok=false: not solvable under the 13-tool subset |
| m16 | skipped | - | - | - | - | - | - | - | - | - | - | subset_ok=false: not solvable under the 13-tool subset |
| m17 | skipped | - | - | - | - | - | - | - | - | - | - | subset_ok=false: not solvable under the 13-tool subset |
| m18 | skipped | - | - | - | - | - | - | - | - | - | - | subset_ok=false: not solvable under the 13-tool subset |
| m19 | skipped | - | - | - | - | - | - | - | - | - | - | subset_ok=false: not solvable under the 13-tool subset |
| m20 | true | 3 | 298.620 | 2 | 0 | 15261 | 5023 | 141 | 8.090 | 290.472 | 0 | all required (mutating) calls made in order with correct args |
| m21 | false | 3 | 61.953 | 0 | 0 | 7305 | 985 | 77 | 5.980 | 55.948 | 0 | no_mutation mode: mutating call pause({"group_id":"RINCON_LIVING01:2"}) was made |
| m22 | true | 1 | 12.391 | 0 | 0 | 2166 | 210 | 19 | 0.535 | 11.850 | 0 | alternative 0: no mutating call made; final text answer given |
| m23 | false | 4 | 181.207 | 1 | 0 | 12577 | 2989 | 142 | 8.250 | 172.912 | 0 | missing required call adjust_group_volume({"group_id":"RINCON_BEDROOM01:3"}) in order after step 3 |
| m24 | false | 2 | 26.587 | 0 | 0 | 4708 | 396 | 33 | 4.795 | 21.778 | 0 | missing required call pause({"group_id":"RINCON_KITCHEN01:1"}) in order after step 0 |
| m25 | true | 3 | 99.273 | 1 | 0 | 9948 | 2103 | 67 | 6.145 | 93.112 | 0 | all required (mutating) calls made in order with correct args |
| m26 | false | 2 | 68.452 | 0 | 0 | 4710 | 1512 | 64 | 3.771 | 64.673 | 0 | missing required call adjust_group_volume({"group_id":"RINCON_LIVING01:2"}) in order after step 0 |
| m27 | false | 3 | 44.919 | 0 | 1 | 7037 | 848 | 159 | 4.864 | 40.046 | 0 | no_mutation mode: mutating call play_artist({"group_id":"BLOUPBLIP","artist":"Nirvana","shuffle":false}) was made |
| m28 | false | 2 | 39.954 | 0 | 0 | 4710 | 855 | 51 | 3.736 | 36.211 | 0 | final read get_now_playing({"group_id":"RINCON_KITCHEN01:1"}) never made with correct args |
| s18 | true | 1 | 25.291 | 0 | 0 | 2168 | 285 | 341 | 0.300 | 24.987 | 0 | no mutating call made; final text answer given |

**Summary**: correct 66.7% (33 scored, 13 skipped), mean_steps 2.73, mean_total_s 69.353, total_retries 12, total_tool_errors 1

**Per step** (90 steps): prompt_tok 2828.5, think_tok 493.2, answer_tok 40.4, first_token_s 1.863, decode_s 23.561, total_s 25.424

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
- m27: `play_artist({"group_id":"BLOUPBLIP","artist":"Nirvana","shuffle":false})`, `get_households_and_groups_and_players({})`; answer: "The group \"BLOUPBLIP\" was not found in your Sonos system. The available groups are:\n1. Kitchen (RINCON_KITCHEN01:1)\n2. Living Room (RINCON_LIVING01:2)\n3. Bedroom (RINCON_BEDROOM01:3)\n\nPlease confirm the correct group name or check for any typos. If you meant a different group, let me know and I'll adjust the playback accordingly."
- m28: `get_households_and_groups_and_players({})`; answer: "The Kitchen Surround player is currently **paused**. The \"Kitchen\" group is paused, and no music is playing on the Kitchen Surround speaker."
