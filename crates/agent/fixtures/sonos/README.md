# Sonos MCP tool schemas

Tool schemas of the Sonos MCP server, 34 tools, with each tool's name,
description and input schema copied verbatim, in MCP `tools/list` shape.
They were not captured from a raw `tools/list` response, so wire-level
details (extra fields, key names) may differ from what a server sends.

## Files

- `tools.json`: all 34 tools, MCP `tools/list`-shaped:
  `[{"name", "description", "inputSchema"}, ...]`.
- `tools-subset.json`: the 13-tool subset the eval runs with
  (`get_households_and_groups_and_players`, `get_now_playing`, `pause`,
  `resume`, `skip_to_next_track`, `skip_to_previous_track`,
  `set_group_volume`, `adjust_group_volume`, `get_group_volume`,
  `set_group_mute`, `get_sonos_favorites`, `play_sonos_favorite`), the
  same entries as in `tools.json`, verbatim, in the order the eval uses.

## Tool name → required parameters

| Tool | Required params |
|---|---|
| add_players_to_group | group_id, player_ids |
| adjust_group_volume | group_id, volume_delta |
| adjust_player_volume | player_id, volume_delta |
| get_group_volume | group_id |
| get_households_and_groups_and_players | (none) |
| get_night_sound_and_speech_enhancement | player_id |
| get_now_playing | group_id |
| get_player_volume | player_id |
| get_registered_music_services | household_id |
| get_shuffle_repeat_crossfade | group_id |
| get_sonos_favorites | household_id |
| get_sonos_playlists | household_id |
| move_audio_to_players | source_group_id, destination_player_ids |
| pause | group_id |
| play_album | group_id, album, shuffle |
| play_artist | group_id, artist, shuffle |
| play_player_line_in | group_id, source_player_id |
| play_playlist | group_id, playlist, shuffle |
| play_radio | group_id |
| play_sonos_favorite | group_id, favorite_id, shuffle |
| play_sonos_playlist | group_id, playlist_id, shuffle |
| play_station | group_id |
| play_track | group_id, track |
| remove_players_from_group | group_id, player_ids |
| resume | group_id |
| seek | group_id |
| set_group_mute | group_id, muted |
| set_group_volume | group_id, volume |
| set_night_sound_and_speech_enhancement | player_id |
| set_player_mute | player_id, muted |
| set_player_volume | player_id, volume |
| set_shuffle_repeat_crossfade | group_id |
| skip_to_next_track | group_id |
| skip_to_previous_track | group_id |

Note: `move_audio_to_players` uses `source_group_id` (not `group_id`), and
several music-service play tools (`play_radio`, `play_station`,
`play_sonos_favorite` via `favorite_id`, etc.) take extra optional params
(`music_service`, `artist`, `album`, `shuffle`, etc.); see `tools.json` for
the full schemas.

## `results/`

One JSON file per tool, `results/<tool>.json`: the bare value that call
returns against the fixture household (one Kitchen, Living Room and
Bedroom group each). `agent::eval::FixtureCaller` serves it the way the
a real server answers:

- A call whose `group_id`, `source_group_id`, `player_id`, `player_ids` or
  `destination_player_ids` names a group or player the household listing
  does not have returns `{"error": "group \"<id>\" not found in household
  Sonos_ABC123XYZ"}` (or `player ...`). The wording is the fixture's own.
- A file of the form `{"by_group_id": {"<groupId>": <result>, ...}}`
  answers with the entry of the call's `group_id`. `get_now_playing.json`
  is one: Kitchen is paused on a Radiohead track, Living Room is playing,
  Bedroom is idle with no current item, matching each group's
  `playbackState` in the listing.
- Every other file is returned as is, the same for any valid arguments.

The agent reads a result with a top-level non-null `error` field, or an
MCP result with `isError: true`, as a tool error.
