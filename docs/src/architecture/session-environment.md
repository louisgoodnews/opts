# Session Environment (date, time, weather)

Status: implemented (C6 of the Berlin-Spandau systems work, 2026-10-17; Chrono switch C4a;
[findings §5 System 5](../meta/berlin-spandau-systems-2026-10-17.md#system-5-environment-controls)).

Date, time of day and weather of a drive are one resource, `opts_bevy::SessionEnvironment`
(`crates/opts-bevy/src/environment.rs`). Before C6 they were frozen at spawn in three places and
re-derived from the start values in six more (traffic used an unwrapped hour and always
"weekday").

## 1. The resource and its only writer

| Field | Meaning |
| :--- | :--- |
| `date` | the current calendar date; rolls over at midnight |
| `seconds_of_day` | `0 ≤ s < 86400` |
| `weather` | preset id (`Clear Summer Day`, `Rainy Afternoon`, …) |
| `rate` | clock seconds per `Time<Virtual>` second (default 1) |
| `revision` | counts **jumps** (a change of date, time or weather, a session start); the clock running is not a jump |
| `map_date` | the date the map content is built for (Chrono) |
| derived | weekday and `[set_day_of_week]` bit, wrapped hour, sun elevation/azimuth, `night` (sun < 3°), light attenuation (0.55 under any covered sky), temperature (−3 °C with snow, else 15 °C ❓) |

The fields are private; the resource changes only through the `EnvironmentChange` message
(`date?`, `seconds_of_day?`, `weather?`, `rate?`, `map_date` policy) and its own clock:

* `tick_environment` advances by `Time<Virtual>.delta × rate` (stands while paused);
* `apply_environment_changes` applies the messages in order.

Both run in `EnvironmentSet` (`Update`), which the game limits to `AppState::InGame`; consumers
run `.after(EnvironmentSet)`. Writers: the session start (`EnvironmentChange::reset` from the plan
in `setup_in_game_session`), the pause page, `sim.set_time`, and save restore.

There is **one sun formula**, `environment::sun_position` (declination approximation, latitude
50°, clock time = local solar time). `scenery_runtime::solar_altitude_deg` and `sky::sun_position`
call it; a test pins the scenery clock and the environment to the same elevation and night switch.

## 2. Consumers

| Consumer | Reads |
| :--- | :--- |
| sky (`in_game/sky.rs`) | sun elevation/azimuth, attenuation; rebuilds the texture every 0.5 clock seconds **or at once on a new revision** |
| fallback light (`in_game/world.rs`) | a system (`update_fallback_light`) sets the light from hour and weather kind whenever they change |
| scenery clock (`sync_scenery_clock`) | date, time, sun, night, temperature → `SceneryClock` (`NightlightA`, `Time`, `DayOfYear`, `Weather_Temperature`) |
| traffic density (`spawn_traffic`) | wrapped hour (0–24) and the day bit (weekday, Saturday, Sunday) |
| traffic light programs (`move_traffic`) | `seconds_of_day` |
| AI lights (`update_ai_variables`) | `night` |
| instrument palette | `night` |
| HUD clock | `HH:MM:SS` and the **localised** weather name |
| saves (`time` section) | date, time, map date, weather, rate (see §4) |

`InGameSessionInfo::sim_elapsed_seconds` stays the play time of the session; it is no longer a
clock.

## 3. Pause menu: Environment

*Environment* in the pause overlay opens a page (`pause/environment.rs`; steppers and chips of
`shell::calendar`, shared with the wizard):

* **Time and weather preview live**: each step sends an `EnvironmentChange`, the sky and light
  change behind the overlay while the game stays paused.
* **The date is a draft** that applies with *Apply* (`MapDatePolicy::Follow`).
* *Cancel* and **Esc restore** the time and weather the page opened with.
* Closing the page returns to the pause overlay.
* Weather stays sky, light and temperature: no fog or precipitation is rendered.

## 4. Saves

`TimeData` keeps the start (`year`, `month`, `day`, `start_hour`, `start_minute`) and the play
time (`sim_elapsed_s`). C6 adds `date`, `seconds_of_day`, `map_date`, `weather`, `rate`, all
`#[serde(default)]`. *Prepare* loads the map for `map_date` (else the start date) with the saved
weather; *apply* (stage `Settled`) sends the saved clock as an `EnvironmentChange` with the
`Keep` map date policy. An older save has none of them: its clock is start + play time, with the
date rolled over (`TimeData::clock`). The `weather` section stores the session's current weather.
The drive history keeps recording the plan (start values).

## 5. Chrono live switch

A change with `MapDatePolicy::Follow` whose date differs from `map_date` moves `map_date` and
sends `MapDateChanged { old, new }`. Midnight rollover and restored clocks do not.

The map is loaded for the planned date (`begin_world_load_at`, `stage_init`): every tile is read
through `WorldLoadJob::read_tile`, which applies the overlays of the Chrono entries active on the
map date ([chrono](../formats/map/chrono.md)); the map-wide spline index goes through it too.
Data files come from `WorldLoadJob::data_file` / `data_bytes` (the last active entry's
replacement, `ailists.cfg` with its group updates). `ChronoState` (resource) exposes the active
entries, deactivated lines, ticket pack and money system; nothing consumes it yet.

`omsi_world::apply_map_date_changes` handles `MapDateChanged` (`in_game/omsi_world/chrono.rs`):

1. the active sets of the old and new date are compared; **equal sets only move the map date**;
2. the changed tiles are those whose overlay list differs, plus the 3 × 3 ring around every
   `.chronoterrrelevant` tile of a toggled entry (terrain carve); a changed `parklist_p*.txt`
   adds every resident tile (parked cars);
3. the neighbour caches of those tiles (splines and objects for the carve) and the spline index
   are dropped; tiles that only an entry adds join or leave the tile set;
4. the **resident** ones among them are unloaded with `WorldLoadJob::unload_tile` (the same full
   unregister the streamer and hot reload use) and queued at the front, **the player's tile first**;
   the player's tile keeps its ground and collision until the rebuilt tile registers its own;
5. the traffic pool is rebuilt when `ailists_#upd.cfg` or `unsched_trafficdens.txt` changed (cars
   already driving stay).

`ChronoRebuild { pending_tiles, total_tiles }` is set to the number of tiles queued;
`track_chrono_rebuild` counts `pending_tiles` down as they become resident again (or drop out of
the queue). The page shows `env-rebuilding` while `pending_tiles > 0`, and *Resume* (or Esc on the
overlay) closes the overlay but keeps the simulation paused until the rebuild is done
(`pause::resume_after_rebuild`). Tiles keep streaming while paused.

## 6. Wizard

The date is chosen in one place, the map step (`wiz-where-date` with the era hint). Choosing a map
sets `planner::default_map_date` unless the date was chosen or remembered: today +
`[realyearoffset]` years, clamped into `[years]` `a-01-01 ..= b-12-31`; 29 February becomes
28 February. See [wizard and loading](../roadmap/phase-9/wizard-and-loading.md) §3.

## 7. Control protocol

`sim.set_time {seconds, date?, weather?, rate?}` sends an `EnvironmentChange` (date with
`Follow`) and answers two frames later with `{clock, seconds_of_day, date, map_date, weather,
rate, revision}`; `sim.get` adds `clock`, `seconds_of_day`, `date`, `map_date`, `weather` and
`clock_rate`. See [control protocol](control-protocol.md) §4.4.

## 8. Tests

`opts-bevy`: calendar arithmetic, weekdays, rollover, revisions, map date policy, systems with
virtual time, sun parity with the scenery clock, traffic density on the wrapped hour and the
weekend. `opts-game`: pause page (open, live preview, Esc restores, Apply and `MapDateChanged`,
Resume waiting for a rebuild), save round trip and an old save, fallback light system, wizard
default date (Spandau-like era, a map without `[years]`, 29 February), date steppers on the map
step, `sim.set_time` parameters, and the generated control reference.
