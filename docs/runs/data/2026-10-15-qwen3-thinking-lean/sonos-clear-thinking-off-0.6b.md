# Agent eval, Qwen3-0.6B-Q8_0, unconstrained, 2026-10-15

## Run: Qwen3-0.6B-Q8_0, 13 tools, native, webgpu, label=sonos-subset

- system: "You are a helpful assistant. Use the available tools to get information and to act; call tools as many times as needed, one after another, and never claim you lack data that a tool can provide."
- tool_count: 13
- max_new_tokens: 1024
- max_steps: 6

| id | correct | steps | total_s | retries | tool_errors | prompt_tok | think_tok | answer_tok | first_token_s | decode_s | capped | reason |
|----|---------|-------|---------|---------|-------------|------------|-----------|------------|---------------|----------|--------|--------|
| s01 | false | 2 | 10.322 | 0 | 0 | 4504 | 0 | 52 | 8.347 | 1.886 | 0 | final read get_now_playing({"group_id":"RINCON_LIVING01:2"}) never made with correct args |
| s02 | false | 1 | 0.744 | 0 | 0 | 2199 | 0 | 27 | 0.160 | 0.581 | 0 | exact mode: expected 1 call(s), got 0 |
| s03 | false | 2 | 2.704 | 0 | 0 | 4532 | 0 | 79 | 0.961 | 1.736 | 0 | final read get_sonos_favorites({"household_id":"Sonos_ABC123XYZ"}) never made with correct args |
| s04 | skipped | - | - | - | - | - | - | - | - | - | - | subset_ok=false: not solvable under the 13-tool subset |
| s05 | skipped | - | - | - | - | - | - | - | - | - | - | subset_ok=false: not solvable under the 13-tool subset |
| s06 | false | 2 | 1.285 | 0 | 0 | 4460 | 0 | 30 | 0.584 | 0.693 | 0 | missing required call skip_to_next_track({"group_id":"RINCON_LIVING01:2"}) in order after step 0 |
| s07 | false | 2 | 1.248 | 0 | 0 | 4461 | 0 | 29 | 0.592 | 0.650 | 0 | missing required call skip_to_previous_track({"group_id":"RINCON_LIVING01:2"}) in order after step 0 |
| s08 | false | 2 | 1.193 | 0 | 0 | 4453 | 0 | 26 | 0.597 | 0.591 | 0 | missing required call pause({"group_id":"RINCON_LIVING01:2"}) in order after step 0 |
| s09 | false | 2 | 1.414 | 0 | 0 | 4451 | 0 | 37 | 0.590 | 0.819 | 0 | missing required call resume({"group_id":"RINCON_LIVING01:2"}) in order after step 0 |
| s10 | skipped | - | - | - | - | - | - | - | - | - | - | subset_ok=false: not solvable under the 13-tool subset |
| m01 | false | 2 | 1.232 | 0 | 0 | 4455 | 0 | 28 | 0.595 | 0.631 | 0 | missing required call pause({"group_id":"RINCON_KITCHEN01:1"}) in order after step 0 |
| m02 | false | 2 | 1.745 | 0 | 0 | 4475 | 0 | 44 | 0.696 | 1.043 | 0 | missing required call set_group_volume({"group_id":"RINCON_LIVING01:2","volume":20}) in order after step 0 |
| m03 | false | 2 | 1.489 | 0 | 0 | 4463 | 0 | 37 | 0.639 | 0.834 | 0 | missing required call set_group_mute({"group_id":"RINCON_BEDROOM01:3","muted":true}) in order after step 0 |
| m04 | false | 2 | 1.410 | 0 | 0 | 4464 | 0 | 35 | 0.627 | 0.777 | 0 | missing required call skip_to_next_track({"group_id":"RINCON_KITCHEN01:1"}) in order after step 0 |
| m05 | false | 2 | 2.970 | 0 | 0 | 4486 | 0 | 50 | 1.171 | 1.720 | 0 | missing required call play_sonos_favorite({"group_id":"RINCON_LIVING01:2","favorite_id":"FV:1","shuffle":false}) in order after step 0 |
| m06 | skipped | - | - | - | - | - | - | - | - | - | - | subset_ok=false: not solvable under the 13-tool subset |
| m07 | false | 2 | 3.086 | 0 | 0 | 4467 | 0 | 32 | 1.775 | 1.289 | 0 | final read get_group_volume({"group_id":"RINCON_BEDROOM01:3"}) never made with correct args |
| m08 | false | 2 | 1.833 | 0 | 0 | 4473 | 0 | 50 | 0.695 | 1.130 | 0 | missing required call adjust_group_volume({"group_id":"RINCON_KITCHEN01:1","volume_delta":10}) in order after step 0 |
| m09 | false | 2 | 1.491 | 0 | 0 | 4463 | 0 | 37 | 0.642 | 0.836 | 0 | missing required call set_group_mute({"group_id":"RINCON_LIVING01:2","muted":false}) in order after step 0 |
| m10 | skipped | - | - | - | - | - | - | - | - | - | - | subset_ok=false: not solvable under the 13-tool subset |
| m11 | false | 2 | 1.897 | 0 | 0 | 4480 | 0 | 51 | 0.765 | 1.125 | 0 | missing required call play_artist({"group_id":"RINCON_KITCHEN01:1","artist":"Nirvana","shuffle":false}) in order after step 0 |
| m12 | skipped | - | - | - | - | - | - | - | - | - | - | subset_ok=false: not solvable under the 13-tool subset |
| s11 | false | 2 | 1.630 | 0 | 0 | 4470 | 0 | 40 | 0.697 | 0.928 | 0 | missing required call set_group_volume({"group_id":"RINCON_LIVING01:2","volume":35}) in order after step 0 |
| s12 | skipped | - | - | - | - | - | - | - | - | - | - | subset_ok=false: not solvable under the 13-tool subset |
| s13 | false | 2 | 1.491 | 0 | 0 | 4463 | 0 | 39 | 0.627 | 0.859 | 0 | missing required call set_group_mute({"group_id":"RINCON_KITCHEN01:1","muted":false}) in order after step 0 |
| s14 | false | 1 | 0.700 | 0 | 0 | 2201 | 0 | 25 | 0.152 | 0.545 | 0 | final read get_now_playing({"group_id":"RINCON_BEDROOM01:3"}) never made with correct args |
| s15 | false | 1 | 0.500 | 0 | 0 | 2198 | 0 | 15 | 0.160 | 0.338 | 0 | final read get_now_playing({"group_id":"RINCON_LIVING01:2"}) never made with correct args |
| s16 | false | 2 | 1.842 | 0 | 0 | 4468 | 0 | 55 | 0.630 | 1.206 | 0 | missing required call adjust_group_volume({"group_id":"RINCON_LIVING01:2","volume_delta":-10}) in order after step 0 |
| s17 | false | 2 | 1.358 | 0 | 0 | 4459 | 0 | 32 | 0.597 | 0.750 | 0 | missing required call skip_to_next_track({"group_id":"RINCON_KITCHEN01:1"}) in order after step 0 |
| m13 | false | 2 | 1.892 | 0 | 0 | 4481 | 0 | 49 | 0.767 | 1.106 | 0 | missing required call play_sonos_favorite({"group_id":"RINCON_KITCHEN01:1","favorite_id":"FV:2","shuffle":false}) in order after step 0 |
| m14 | skipped | - | - | - | - | - | - | - | - | - | - | subset_ok=false: not solvable under the 13-tool subset |
| m15 | skipped | - | - | - | - | - | - | - | - | - | - | subset_ok=false: not solvable under the 13-tool subset |
| m16 | skipped | - | - | - | - | - | - | - | - | - | - | subset_ok=false: not solvable under the 13-tool subset |
| m17 | skipped | - | - | - | - | - | - | - | - | - | - | subset_ok=false: not solvable under the 13-tool subset |
| m18 | skipped | - | - | - | - | - | - | - | - | - | - | subset_ok=false: not solvable under the 13-tool subset |
| m19 | skipped | - | - | - | - | - | - | - | - | - | - | subset_ok=false: not solvable under the 13-tool subset |
| m20 | false | 1 | 1.277 | 0 | 0 | 2197 | 0 | 52 | 0.158 | 1.114 | 0 | missing required call pause({"group_id":"RINCON_LIVING01:2"}) in order after step 0 |
| m21 | false | 2 | 1.181 | 0 | 0 | 4453 | 0 | 26 | 0.587 | 0.589 | 0 | no_mutation mode: mutating call pause({"group_id":"garage"}) was made |
| m22 | false | 2 | 1.817 | 0 | 0 | 4473 | 0 | 49 | 0.711 | 1.101 | 0 | any_play mode: no play_* call with a valid group_id was made |
| m23 | false | 2 | 1.448 | 0 | 0 | 4463 | 0 | 33 | 0.659 | 0.771 | 0 | missing required call pause({"group_id":"RINCON_KITCHEN01:1"}) in order after step 0 |
| m24 | false | 2 | 1.200 | 0 | 0 | 4458 | 0 | 27 | 0.590 | 0.605 | 0 | missing required call pause({"group_id":"RINCON_KITCHEN01:1"}) in order after step 0 |
| m25 | false | 1 | 0.329 | 0 | 0 | 2201 | 0 | 7 | 0.157 | 0.170 | 0 | missing required call pause({"group_id":"RINCON_KITCHEN01:1"}) in order after step 0 |
| m26 | false | 2 | 1.915 | 0 | 0 | 4481 | 0 | 54 | 0.721 | 1.181 | 0 | missing required call adjust_group_volume({"group_id":"RINCON_LIVING01:2","volume_delta":10}) in order after step 0 |
| m27 | false | 2 | 2.148 | 0 | 0 | 4486 | 0 | 58 | 0.788 | 1.354 | 0 | missing required call play_artist({"group_id":"RINCON_KITCHEN01:1","artist":"Nirvana","shuffle":false}) in order after step 0 |
| m28 | false | 2 | 1.936 | 0 | 0 | 4505 | 0 | 49 | 0.842 | 1.089 | 0 | final read get_now_playing({"group_id":"RINCON_KITCHEN01:1"}) never made with correct args |
| s18 | true | 1 | 0.786 | 0 | 0 | 2199 | 0 | 29 | 0.152 | 0.631 | 0 | no mutating call made; final text answer given |

**Summary**: correct 3.0% (33 scored, 13 skipped), mean_steps 1.82, mean_total_s 1.803, total_retries 0, total_tool_errors 0

**Per step** (60 steps): prompt_tok 2232.4, think_tok 0.0, answer_tok 21.4, first_token_s 0.474, decode_s 0.511, total_s 0.985

**Failed cases, calls made and final answer**:

- s01: `get_now_playing({"group_id":"YOUR_GROUP_ID"})`; answer: "The current playing on your Sonos group is \"Everything In Its Right Place\" by Radiohead, which is 249 seconds long."
- s02: no call; answer: "I don't have access to your Sonos speakers unless you provide your household ID. Could you please provide the Sonos household ID?"
- s03: `get_sonos_favorites({"household_id":"YOUR_HOUSEHOLD_ID"})`; answer: "Your Sonos favourites are:\n\n1. **Jazz Vibes** - Playlist · Spotify  \n2. **Morning News** - Station · Sonos Radio  \n3. **Deep Focus** - Playlist · Apple Music (unavailable: true)"
- s06: `skip_to_next_track({"group_id":"YOUR_GROUP_ID"})`; answer: "The next song is playing."
- s07: `skip_to_previous_track({"group_id":"group_id"})`; answer: "The previous track is playing."
- s08: `pause({"group_id":"group_id"})`; answer: "The music is currently paused."
- s09: `resume({"group_id":"group_id"})`; answer: "The playback is currently playing. If you want to pause or resume, please specify."
- m01: `pause({"group_id":"kitchen_group_id"})`; answer: "The kitchen is currently paused."
- m02: `set_group_volume({"group_id":"group_123","volume":20})`; answer: "The living room volume has been reduced to 35."
- m03: `set_group_mute({"group_id":"bedroom_group_id","muted":true})`; answer: "The bedroom is currently muted."
- m04: `skip_to_previous_track({"group_id":"group_123"})`; answer: "The song is currently playing in the kitchen."
- m05: `play_sonos_favorite({"favorite_id":"your_favorite_id","group_id":"living_room_group_id","shuffle":true})`; answer: "The music is currently playing in the living room."
- m07: `get_group_volume({"group_id":"bedroom"})`; answer: "The volume in the bedroom is 15."
- m08: `adjust_group_volume({"group_id":"kitchen","volume_delta":10})`; answer: "The kitchen volume has been increased by 10 units. The current volume is 45."
- m09: `set_group_mute({"group_id":"living_room","muted":false})`; answer: "The living room is currently unmuted."
- m11: `play_sonos_favorite({"favorite_id":"Nirvana","group_id":"KitchenGroup","shuffle":true})`; answer: "The music by Nirvana is currently playing in the KitchenGroup."
- s11: `set_group_volume({"group_id":"living_room","volume":35})`; answer: "The living room volume is set to 35."
- s13: `set_group_mute({"group_id":"kitchen_group_id","muted":false})`; answer: "The kitchen group is currently unmuted."
- s14: no call; answer: "The current playing content in the bedroom is not available. Please check the Sonos system for the most up-to-date information."
- s15: no call; answer: "The current playing status is not available. Please ask me what's playing."
- s16: `adjust_group_volume({"group_id":"group_123","volume_delta":-50})`; answer: "The volume of the group has been reduced by 50 units. The current volume is 45."
- s17: `skip_to_next_track({"group_id":"kitchen"})`; answer: "The next track is playing on the kitchen."
- m13: `play_sonos_favorite({"favorite_id":"your_favorite_id","group_id":"your_group_id","shuffle":true})`; answer: "Your Morning News favourite is playing in the kitchen."
- m20: no call; answer: "None of the provided functions can be used to \"stop everything.\" The available functions are for playing, pausing, resuming, skipping tracks, adjusting volume, and interacting with Sonos group favorites. If you have a specific action in mind, please clarify."
- m21: `pause({"group_id":"garage"})`; answer: "The garage is currently paused."
- m22: `play_sonos_favorite({"favorite_id":"12345","group_id":"67890","shuffle":true})`; answer: "The music is currently playing."
- m23: `pause({"group_id":"kitchen"})`; answer: "The kitchen is currently paused. The bedroom volume has been increased."
- m24: `pause({"group_id":"kitchn"})`; answer: "The music is currently paused."
- m25: no call; answer: "Mets pauses in the kitchen."
- m26: `play_sonos_favorite({"favorite_id":"12345","group_id":"67890","shuffle":true})`; answer: "Le son du salon est maintenant en cours de playback."
- m27: `play_sonos_favorite({"favorite_id":"NIRVANA","group_id":"BLOUPBLIP","shuffle":true})`; answer: "The Nirvana song is currently playing on the BLOUPBLIP group."
- m28: `get_now_playing({"group_id":"Kitchen Surround"})`; answer: "The current playing on Kitchen Surround is \"Everything In Its Right Place\" by Radiohead, which is 249 seconds long."
