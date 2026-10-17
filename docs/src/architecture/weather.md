# Weather

Status: implemented on branch `agent/weather` (2026-10-17). Before this, weather was five hard-coded
presets that changed the sky attenuation, the fallback light and the temperature; fog,
clouds, precipitation and wet ground were not rendered
([session environment](session-environment.md) §3, [environment formats](../formats/text/environment.md)).

The data comes from OMSI's own preset files (`Weather/*.owt`, `Weather/clouds.cfg`, localised
`Weather/<Name>_<LANG>.dsc`, scenario `Situations/*.osn.owt`). Everything is evidence-based on the
installed corpus (9 presets, 84 `.owt` files in all); what the files do not tell stays marked ❓.

## 1. Data flow

```text
install Weather/*.owt, clouds.cfg, *.dsc          builtin presets (no install / old ids)
                    │                                        │
          opts_core::formats::weather ── WeatherFile, CloudCatalogue, WeatherText
                    └──────────────────┬──────────────────────┘
                              opts_bevy::weather::WeatherCatalogue  (resource, list + aliases + names)
                                       │ resolve(id)
        EnvironmentChange{weather:id} ─▶ SessionEnvironment ─ conditions: WeatherConditions (derived)
                                       │
   ┌────────────┬───────────────┬──────┴───────┬───────────────┬────────────────────┐
 light_scales  sky veil      fog params     cloud layer     precipitation       ground wetness / snow
 (one fn)     (sky.rs)    (DistanceFog)    (plane+drift)  (budgeted quads)   (GroundMaterial.wet, roads)
                                       │
                         SceneryClock / vehicle scripts: Weather_Temperature, Weather_AbsHum,
                         PrecipType, PrecipRate
```

`SessionEnvironment` stays the only holder of the session weather and `EnvironmentChange` its only
writer. It stores the preset id string (saves, history, `sim.set_time` unchanged) and the
**resolved** `WeatherConditions`. Conditions are re-derived when the weather id changes and when
the `WeatherCatalogue` resource changes (the install is found after the first frame). A weather
change is a jump (bumps `revision`), so the sky rebuilds at once and every effect below follows in
the next frame. No effect keeps its own copy of the weather.

## 2. Presets, ids and aliases

* **Stable id = file stem** of the `.owt` (`Sommerlich`, `#CAVOK`, `Ueberfrierende Naesse`). The
  display name is the `[name]` of `<stem>_<LANG>.dsc` for the UI language (`de` → `DEU`,
  `en` → `ENG`, `fr` → `FRA`, `hu` → `HUN`, `pl` → `POL`), else the English one, else the
  `[name]` of the `.owt` (angle brackets stripped).
* With an install the wizard and the pause page list its presets (sorted by file name, `#CAVOK` first). Without one they list the five built-in presets.
* The five **built-in ids** (`Clear Summer Day`, `Overcast Autumn`, `Rainy Afternoon`,
  `Winter Snowfall`, `Autumn Morning Fog`) keep working everywhere (old saves and drive history):

  | Old id | With the install | Without |
  | :--- | :--- | :--- |
  | Clear Summer Day | `Sommerlich` | built-in clear (15 °C, no fog, no clouds) |
  | Overcast Autumn | built-in overcast (no install preset is overcast and dry) | same |
  | Rainy Afternoon | `Schmuddelwetter` | built-in overcast rain |
  | Winter Snowfall | `Starker Schneefall` | built-in overcast snow, −3 °C |
  | Autumn Morning Fog | `Bodennebel` | built-in fog (300 m) |

  An alias resolves at read time; the stored string is never rewritten, so a save keeps what it
  had. An unknown id (an install was removed) falls back to the first listed preset.

## 3. What each `.owt` field drives

| Field | Drives | Status |
| :--- | :--- | :--- |
| `[name]`, `[description]` | UI name and description (`.dsc` first) | ✅ |
| `[fog]` visibility, density | `DistanceFog` (exponential-squared, 2 %-contrast at the visibility; density factor → contrast threshold 2 % … 10 %), the fog colour from the sky's horizon colour, the sky's own veil toward that colour, a share of the sun attenuation | ✅ |
| `[wind]` direction, speed | cloud drift (×3 for height), rain/snow slant and flutter, script/Rust API `wind_*`. Direction is **where the wind comes from**, degrees clockwise from north (`Sommerlich` = 70°, "light east wind"); speed in m/s (storm 18.9) | ✅ unit by plausibility ❓ |
| `[temp]` temperature, dew point | `Weather_Temperature`; `Weather_AbsHum` from the dew point (Magnus), frost flag for snow-on-ground | ✅ |
| `[press]` | stored, exposed in `SessionEnvironment` (no consumer: no script variable is documented) | ❓ |
| `[clouds]` type, base height | cloud layer: texture and size from `clouds.cfg`, base height above the camera, cover class `sct`/`ovc`; `-1` = none | ✅ |
| `[precip]` type, intensity | rain/snow particle system; intensity/255 is `PrecipRate` and scales the number of particles; values 3–5 unknown | ✅ / ❓ |
| `[groundwet]` | wetness of the ground and of the road surfaces (see §6) | 🟡 ❓ |
| `[snow]` | white tint on ground (cover 0.85) | 🟡 |
| `[snowOnRoad]` | the same tint on road (spline) materials | 🟡 |

### ❓ Unknown fields (kept in the type, not used for more than the table says)

* `[groundwet]` three numbers (observed: `0 0 0` dry, `30.6 255 0` rain, `78 255 115.9` soaked,
  `77.9 133.2 20.9` icy wet, `1.7 255 0` snow): we use *the second / 255* as wetness and *the third
  / 120* as puddle coverage. The first is unused. All three meanings are guesses.
* `[precip]` the last three values (`0 -2 0` in all but the `32`-baseline files): kept, unused. The
  second value is an intensity only for type ≠ 0 (`Ueberfrierende Naesse` has type 0 and 192:
  ice on the ground without precipitation ❓).
* `[press]` has no known consumer. `[fog]` has no unit for the second value's "layer height";
  it is used only for the sun attenuation and the contrast threshold.
* Cloud cover of `sct` types: we read the digit of `Cumulus 1/2/3` as 0.25/0.4/0.55 ❓.

## 4. Light: one function

`opts_bevy::weather::light_scales(&WeatherConditions) -> LightScales { sun, ambient }` is the only
place where weather scales the existing sun illuminance and ambient brightness (`sky.rs` calls it
through `SessionEnvironment::attenuation()`/`ambient_attenuation()`; the lighting workstream can
adopt or replace it). Overcast (cover 1) gives sun 0.55 and ambient 0.8 exactly as before.
Effective cover = max(cloud cover, precipitation (0.5 + 0.5·rate), fog share); sun = 1 − 0.45 ·
cover; ambient = 1 − 0.2 · cover. The fallback light (`world.rs`) uses the `WeatherKind`
classification of the conditions.

## 5. Rendering and budget

| Effect | Implementation | Setting (default) | Cost |
| :--- | :--- | :--- | :--- |
| Fog | `DistanceFog` on the in-game camera; none when visibility > 15 km | `graphics.weather_fog` (on) | free (GPU, per-fragment blend) |
| Clouds | one 48×6 vertex disc at the base height (radius 950 m, alpha fade at the rim), unlit, texture scrolls with `uv_transform` | `graphics.weather_clouds` (on) | 1 draw call, ~300 vertices |
| Precipitation | one dynamic mesh of quads rebuilt each frame around the camera (rain: velocity-aligned streaks, snow: billboards); particles are a pure function of (seed, time) wrapped in a 44 m box, no state | `graphics.weather_precipitation` (`off`, `low` 1500, `medium` 3000, `high` 6000 quads; default `medium`) | O(active quads) CPU, ≤ 6000 × 4 vertices written per frame, 1 draw call |
| Wet ground | `GroundMaterial.extension.wet` (existing shader), road materials via a global factor | `graphics.weather_wet_ground` (on) | on change only |
| Snow tint | `wet.z` in the ground shader, road base colour | with wet ground | on change only |

**Colours are scene-linear** and come from one function, `opts_bevy::weather::weather_look` (fog colour, cloud and precipitation brightness); a later HDR camera needs a change there only.

Effects use real time (not `Time<Virtual>`), so a weather preview on the paused Environment page
animates. Disabling a setting takes effect live (`SettingChanged`).

## 6. Wet ground and snow

The weather is the writer of the reflections prototype's `Wetness` resource (`opts_bevy::reflections::Wetness`, [reflections research](reflections-research.md)): `drive_wetness` sets it from `[groundwet]` (0 when `graphics.weather_wet_ground` is off) and the surfaces system reads it, so there is one wet-ground implementation. The prototype's env-switched standard-material path (`OPTS_REFLECT=wet`) stays a research option; its ground-shader write was removed (the weather writes the uniform, with puddles and snow). The wet path is now a normal setting, default on; its cost is one uniform write on change and a 0.5 s scan of the ground materials, the road materials are touched only on change (the research measured wet surfaces as free within noise).

Wetness `w = groundwet[1]/255`; puddle coverage `0.45 · min(groundwet[2]/120, 1)` clamped (0.45 is the
existing prototype value). Ground uses the existing shader path (darker, rougher→smoother,
puddles). Road materials (`StandardMaterial`s of the splines) get the same global factor: base
colour × `(1 − 0.3·w)` and roughness × `(1 − 0.55·w)`, from their original values captured at first
sight. Snow cover tints the ground toward `(0.92, 0.94, 0.97)` by 0.85 and, with `[snowOnRoad]`,
the roads by 0.7. Objects, roofs and vehicles are not snow-covered (deviation).

## Headless check

`cargo run -p opts-game --example weather_probe -- <dir> [preset…]` renders a small scene (layered ground, boxes) off-screen under each preset (with `OMSI_PATH`: the install's presets and cloud textures) and writes PNGs: fog, cloud disc, rain streaks, snow flakes, wet ground and puddles, snow cover. It does not include the sky dome (the fogged sky texture is in `sky.rs`).

## 7. Script variables

The names named by [predefined variables](../formats/text/vehicles/predefined-variables.md):
`Weather_Temperature` (°C), `Weather_AbsHum` (g/m³), `PrecipType` (0 none, 1 rain, 2 snow, the
`.owt` coding ❓: the vehicle-input table assumed 0 rain/1 snow), `PrecipRate` (0…1 = intensity/255,
0 without precipitation). They reach scenery scripts (`SceneryClock` → `HostValues`), player vehicle
scripts (system and local variables) and the AI variable tables. **Wind, pressure and the fog
visibility have no documented variable name**; they are available to Rust code
(`SessionEnvironment::conditions()`, `SceneryClock`) and no script variable is invented.

## 8. Control protocol

`sim.set_time --weather <id>` accepts any listed preset id, slug, case-insensitive name or a word
of it, plus the old built-in ids. `sim.get` in a drive adds `weather_presets` (the ids available).
`sim.set_time` rejects an unknown name and lists the ids in the error.

## 9. Deviations from OMSI (as far as known) and not done

* No height-limited fog layer, no volumetric fog, no light rays (System 6 *Lighting* owns those).
* No cloud shadows, no lightning, no rain on windows (`Rain_Window_*` variables stay unset), no
  splash/ripple effects, no puddle reflections beyond the existing shader, no snow on objects.
* *Live weather* (ICAO lookup) stays the registered not-yet place `wizard.live-weather`.
* Scenario `.osn.owt` files are parsed (they have the same grammar) but not yet used: OPTS does not
  load `.osn` scenarios.
* Wind affects clouds and precipitation only (no vehicle forces, no sound, no vegetation).
