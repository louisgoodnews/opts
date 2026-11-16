# Real Weather: Research and Decision (M-0116)

Status: research and decision, 2026-11-16 (documents only; no code). Milestone:
[M-0116](../roadmap/phase-11/M-0116.md). Builds on [Weather](weather.md) (implemented),
[session environment](session-environment.md) and [lighting system](lighting-system.md).

**Question.** Should OPTS show real weather, from which providers, under which licence and offline
policy, how observations map onto OMSI's `.owt` presets, where the location comes from, and what
the fallback is?

**Decision in one line.** No-go for network weather now. Keep the offline preset model as the
only source. If the project lead later reopens the question, the candidate is MET Norway (open
data, commercial use allowed with attribution, identification header required), used only after an
explicit user opt-in and with the policy in section 5. Section 6 gives the reasons.

Conventions: a fact is **sourced** when a repo file or an external URL (with the date checked) is
given. **Unsourced** means it could not be verified in this run and is not used for a decision.
Access date for every external source: **2026-11-16**.

---

## 1. Provider survey

Three provider classes, as the milestone requires. Terms were checked from the published pages;
where a page was unreachable or inconsistent this is stated in the row.

### 1a. Public meteorological services with open data

| Provider | Coverage | Licence and attribution | Commercial use | Rate limits | Historic data | Auth | Privacy impact |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **MET Norway** (`api.met.no`, Locationforecast) | Global forecast grid (not observations) | CC BY 4.0; the MET page also names NLOD 2.0 for data. Attribution to MET Norway required. Sources disagree on which licence governs, so the live page must be re-read before any implementation. ([api.met.no terms](https://api.met.no/doc/TermsOfService); [MET FAQ](https://docs.api.met.no/doc/FAQ); searched 2026-11-16) | Yes, under CC BY 4.0 with attribution (FAQ). | Not captured in this run (unsourced). Cache headers should be respected (third-party guidance only, unsourced for the official text). | Limited; not a focus | None; **User-Agent must identify the application with a contact address or URL** | The request sends the coordinates; the User-Agent identifies the client, not the player. |
| **DWD** (Deutscher Wetterdienst, Open Data, `opendata.dwd.de`) | Germany, station observations and model data | CC BY 4.0 for the CDC open-data area; "Source: Deutscher Wetterdienst" credit. Official terms PDF could not be fetched in this run (HTTP 404 on the URL tried); the licence statement rests on search summaries of DWD pages ([DWD copyright and terms](https://dwdbib.dwd.de/wiki/termsuse), searched 2026-11-16). Warnings have extra rules: treat as unverified. | Stated as allowed with attribution (search summary; the official PDF not re-read). | Not captured (unsourced). | Yes, large station archives | None | Bulk files; no user data sent. |
| **NOAA / NWS** (US) | US and some observations; METAR data via aviation services | Public domain for NWS web content unless noted; NWS name and logo are trademarks; third-party images may not be public domain. Current data policy not confirmed (search returned archived copies; [NWS disclaimer](https://www.weather.gov/disclaimer), searched 2026-11-16). | Public-domain material may be used for any lawful purpose with the attribution conditions in the disclaimer. | Not captured (unsourced). | Yes (archives) | None | None. |

### 1b. Commercial APIs

| Provider | Coverage | Licence and attribution | Commercial use | Rate limits | Historic data | Auth | Privacy impact |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **Open-Meteo** (`open-meteo.com`) | Global forecast and archive | Data under CC BY 4.0 ([terms](https://open-meteo.com/en/terms), 2026-11-16). Attribution page not reached in this run. | **Free tier: non-commercial only.** Paid plans for commercial use (price list not captured in this run; [pricing page](https://open-meteo.com/en/pricing) was listed by search, not fetched). | Free tier: fewer than 10,000 calls per day, 5,000 per hour, 600 per minute (terms). | Yes (archive API) | Key for commercial plans (unsourced details) | Coordinates sent per request. |
| **OpenWeatherMap** (`openweathermap.org`) | Global current, forecast, history | Terms not read in this run (unsourced licence). | Commercial plans exist. Price list ([openweathermap.org/price](https://openweathermap.org/price), 2026-11-16): free 60 calls/min, 1,000,000 calls/month; Startup 35 EUR/month (600 calls/min, 10M/month); Developer 160 EUR/month (3,000/min, 100M/month); Professional 410 EUR/month (30,000/min, 1B/month); Expert 1,200 EUR/month. History is in Professional and Expert plans only. | As listed above. | Yes, paid plans (history bulk 9 EUR per location) | API key | Key and coordinates per request. |

### 1c. Community and historical datasets

| Provider | Coverage | Licence and attribution | Commercial use | Rate limits | Historic data | Auth | Privacy impact |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **Meteostat** (`meteostat.net`, `dev.meteostat.net`) | Station-based, global; hourly and daily | **Conflicting sources.** The licence page and main README say CC BY 4.0; the FAQ and support pages say CC BY-NC 4.0 ([license page](https://dev.meteostat.net/license), [terms](https://dev.meteostat.net/terms), searched 2026-11-16). The terms page defers the licence to the license page and does not state a non-commercial limit. Written confirmation from the provider would be needed. | **Unclear.** Not usable commercially until confirmed. | Not stated (terms page). | Yes, long station history | RapidAPI key for the JSON API (per search summary) | Coordinates per request. |
| **NOAA ISD / archive** | Global station history | Public domain for US government archives; not re-read in this run. | Yes (public domain) | Not captured (unsourced). | Yes | None | None. |

**Rows marked "unsourced" or "not captured" are not used in section 6.**

**Where a row is incomplete:** the historic-data and rate-limit cells for MET Norway, DWD and NOAA
were not verified in this run and must be checked on the live pages before implementation.

---

## 2. Observation fields and the preset model

What a provider would deliver, and what the `.owt` side can take.

| Observation field | Provider field (typical) | OPTS side | Status |
| :--- | :--- | :--- | :--- |
| Cloud cover | cloud fraction 0–1 or oktas | `[clouds]` type (`sct`/`ovc` class) with base height, [weather](weather.md) §3 | Mapping by class; the digit reading for `Cumulus 1/2/3` is ❓ (weather.md) |
| Precipitation type | rain / snow / none | `[precip]` type 0, 1, 2 | Sourced ([environment format](../formats/text/environment.md)) |
| Precipitation intensity | mm/h | `[precip]` second value 0–255 | Scale mapping sourced; mm/h to 0–255 **not** sourced (no calibration in repo) |
| Visibility | metres | `[fog]` visibility | Sourced (weather.md §3) |
| Temperature, dew point | °C | `[temp]` | Sourced (env format, Q-ENV02) |
| Wind | direction, speed m/s | `[wind]` | Sourced (direction from, degrees, m/s by plausibility) |
| Pressure | hPa | `[press]` | Stored, no consumer (❓) |

**Not mappable at all:** ground wetness and snow depth. The `[groundwet]` fields are partly
unknown (weather.md §3), so real ground state would stay with the preset.

---

## 3. Mapping: ten sample situations

Presets are the ids in the repo docs and the install's file names. Parameter values are cited only
where the repo states them; anything else is "not sourced" and the mapping does not depend on it.
No preset parameter is invented.

**Preset list (9).** Named in the repo: `#CAVOK`, `Sommerlich`, `Schmuddelwetter`,
`Starker Schneefall`, `Sturm`, `Bodennebel`, `Daemmerungsnebel`, `Ueberfrierende Naesse`
([weather.md](weather.md) §2, [environment format](../formats/text/environment.md)). The ninth
(`Eiseskälte`) is a file of the install's `Weather` folder seen in a directory listing; its
parameters are not used here (unsourced).

**Sourced parameters used** (from [environment format](../formats/text/environment.md) and [weather.md](weather.md)):
`Sommerlich` wind 70°; `Sturm` fog 700 m, wind 209° at 18.9 m/s, rain intensity 132;
`Schmuddelwetter` rain intensity 173; `Starker Schneefall` snow intensity 187 (heavy snow);
`Bodennebel` fog 75 m (ground fog); `Daemmerungsnebel` fog 500 m, intensity 32 (drizzle);
`Ueberfrierende Naesse` type 0 with intensity 192 (ice on ground, no precipitation, ❓).

| # | Situation (observation) | Preset or blend | Parameters used | Status |
| :-: | :--- | :--- | :--- | :--- |
| 1 | Clear, calm summer morning | `#CAVOK` | description "best weather" (sourced) | Sourced id; values not used |
| 2 | Clear summer day, light east wind | `Sommerlich` | wind 70° (sourced) | Sourced |
| 3 | Dusk haze, visibility 500 m | `Daemmerungsnebel` | fog 500 m (sourced) | Sourced |
| 4 | Shallow radiation fog, visibility 75 m | `Bodennebel` | fog 75 m, layer (sourced) | Sourced |
| 5 | Severe storm, visibility 700 m, strong wind | `Sturm` | fog 700 m; wind 209°, 18.9 m/s; rain 132 (sourced) | Sourced |
| 6 | Steady rain, low cloud | `Schmuddelwetter` | rain 173 (sourced) | Sourced |
| 7 | Heavy snowfall | `Starker Schneefall` | snow 187 (sourced) | Sourced |
| 8 | Glaze ice on road, no precipitation falling | `Ueberfrierende Naesse` | ground ice, type 0 (sourced ❓ on meaning) | Partly sourced |
| 9 | Light snow, rate 50 % of heavy | blend: `Starker Schneefall` with `graphics.snow_intensity` 50 % | existing setting (0–200 %, weather.md §9) | Uses an existing setting |
| 10 | Afternoon shower on a warm day | blend: `Sommerlich` plus `Schmuddelwetter` rain at `graphics.rain_intensity` 40 % | existing setting | Uses an existing setting; the blend weight is a design proposal |

**Error bounds.** Intensity from provider mm/h to the 0–255 scale is not calibrated in the repo
(unsourced). Until calibrated, the mapping is ordinal: a mapped intensity class (none, light,
moderate, heavy) selects one of the presets above, and the rate-scale settings give the rest.
Visibility error is the distance band (75, 500, 700 m); 50 km is the clear band.

**Blending and rate of change (design proposal, not sourced).** Weather changes are jumps today
(weather.md §1). For live data, cross-fade between two presets over a fixed time (proposed: 10
minutes of game time), and step the source at most once per provider update (the provider's
update interval, unsourced). A blend is a weighted mix of the two presets' conditions, using the
existing `light_scales` function; no new parameter is introduced.

---

## 4. Location source

| Option | Description | Decision |
| :--- | :--- | :--- |
| Map coordinates | The `[worldcoordinates]` map's centre, from [streaming coordinates](streaming-coordinates.md) | Preferred default, already available, no new data |
| User choice | Place name or coordinates typed in the wizard | Only if the map has no coordinates |
| Device location | OS location service | Rejected: sends the device's position, needs a permission prompt |

Latitude already feeds the moon model ([lighting system](lighting-system.md)). The map coordinates
are the location for weather as well.

---

## 5. Policy and consent (proposal)

Per the milestone, this section defines the policy. It is a proposal for review, not an accepted ADR.

**Defaults.**
- **Offline by default.** The game makes no network request for weather. Preset weather is the only source.
- **Opt-in only.** Network weather is off until the user enables it in a settings page (new
  `graphics.*`-style key or a general setting, to be decided on implementation).
- **Consent UI sketch.** On first enable: a dialog naming the provider, the licence, what is sent
  (the map's coordinates and the time, nothing else), and a button to keep offline. A status line
  shows the last update time and the provider's attribution.
- **What is sent.** Latitude and longitude, rounded to 0.1° (about 10 km, proposal), and the request
  time. No account, no user name, no device identifier. The User-Agent for MET Norway names the
  application (the project name), not the player.
- **Keys.** No provider key is stored in the repository. For a key-based provider, the key is
  entered by the user and stored in the user's config directory, outside the repo and outside save
  files. Keys never appear in logs.
- **Caching.** Cache responses per user, on the user's machine, only. No cache is shared or
  uploaded. Cache lifetime follows the provider's cache headers (MET Norway; required, not yet
  verified against the live page).
- **Licensing.** Only providers whose licence allows commercial use with attribution are
  candidates. Meteostat is excluded until its licence is confirmed in writing. Open-Meteo's free
  tier is excluded (non-commercial). Attribution text is shown in the credits.
- **No redistribution.** No provider data is committed to the repo or shipped in builds.

**Review against ADR-0002 and the privacy stance.** ADR-0002 governs OMSI-derived data; weather
data from providers is outside that policy, but the no-redistribution rule and the no-asset
rule are kept. The privacy stance is applied through the coordinate rounding and the opt-in. The
[hardware probe privacy charter](hardware-probe.md) and ADR-0010 (hardware probe privacy) were not
re-read in this run, so the review against them is pending.

---

## 6. Recommendation and go/no-go

**No-go for network weather now.** Reasons:

1. **Licence evidence is incomplete.** Meteostat is contradictory. Open-Meteo's free tier is
   non-commercial. The DWD and NOAA pages were partly verified; the official DWD PDF was not
   reached. A commercial release cannot rely on these.
2. **Coverage would not change most situations.** The ten sample situations are covered by the
   nine existing presets and existing settings (section 3). Live data adds value mainly for
   matching a real day.
3. **Mapping is coarse.** Intensity calibration is unsourced; mapping is ordinal. The milestone's
   own risk table names this.
4. **Privacy and consent work is non-trivial** and not yet reviewed against the hardware-probe charter.

**Candidate if the decision is reopened.** MET Norway: open data, commercial use with
attribution, global, no key. Blocking items: confirm the licence on the live page, the cache
rule, and the User-Agent format.

**Alternatives.** Open-Meteo (non-commercial free tier; a paid plan would be needed). OpenWeatherMap
(paid for commercial use; terms not read). Meteostat (licence unclear). DWD (national, good for
German maps; not global).

**Follow-up.** If the decision is go later, draft a follow-up milestone for the MET Norway client
(offline cache, opt-in UI, attribution). No such milestone is drafted now.

---

## 7. ADR proposal (draft, not accepted)

| Field | Value |
| :--- | :--- |
| Title | Offline-first weather; network weather opt-in only |
| Context | Section 1 to 4 |
| Decision | Weather comes from the install's presets by default. Network weather is opt-in, from an open-data provider whose licence allows commercial use with attribution, cached per user, with no keys in the repo. |
| Consequences | No network code for weather until a go decision. Map coordinates are the location. |
| Status | Proposed; for the project lead. |

---

## 8. Procedural fallback (design, not built)

**Purpose.** When network weather is off or unavailable, produce plausible weather from season and
time of day with the same presets.

**Inputs.** Seed (a 64-bit integer, stored with the session), date (`CalendarDate`), time of day
(seconds since midnight), latitude (map coordinates), and the previous state (for continuity).

**Outputs.** A preset id, and a blend weight to the next preset (0–1), both from the existing
presets. No new preset parameters.

**Determinism.** A pure function `fallback_weather(seed, date, seconds, lat, prev) -> (PresetId, PresetId, f32)`
with no clock, no RNG state other than the seed. Same seed and same inputs give the same output.
The output is quantised to 1-hour buckets, with blend in the bucket interior.

**Testable form.** A table test: for a fixed seed and 24 hours of a fixed date, the sequence of
ids is fixed (golden list). Changing the seed changes the list. Two runs give identical output.

**Limits.** Season and time only; no synoptic model. Parameters for the season bands are a
design proposal and are not sourced.

---

## 9. Unsourced facts (kept out of the decisions)

- Mm/h to the 0–255 precipitation intensity scale.
- Preset parameters beyond those listed in section 3 (not read in this run; the install's
  `Weather` folder could not be read further, see the milestone log).
- Provider rate limits for MET Norway, DWD, NOAA; the MET cache rule text.
- Ground-state fields `[groundwet]` (weather.md §3 ❓).
- The DWD official terms PDF (not reached).

## Provenance

- Repo files read: [weather.md](weather.md), [environment format](../formats/text/environment.md), [ADR-0002](../adr/0002-clean-room-policy.md), the milestone file.
- Preset ids: repo docs and tests, and the install's `Weather` folder file names as listed in a directory listing.
- External (accessed 2026-11-16): [Open-Meteo terms](https://open-meteo.com/en/terms); [OpenWeatherMap prices](https://openweathermap.org/price); [MET Norway terms](https://api.met.no/doc/TermsOfService); [MET Norway FAQ](https://docs.api.met.no/doc/FAQ); [Meteostat license](https://dev.meteostat.net/license); [Meteostat terms](https://dev.meteostat.net/terms); [DWD terms and attribution summary](https://dwdbib.dwd.de/wiki/termsuse); [NWS disclaimer](https://www.weather.gov/disclaimer).
- Not reached: the DWD terms PDF (HTTP 404 on the tried URL); the Open-Meteo pricing and attribution pages.

## Related Documents

[Weather](weather.md) · [Session environment](session-environment.md) · [Lighting system](lighting-system.md) · [Streaming coordinates](streaming-coordinates.md) · [ADR-0002](../adr/0002-clean-room-policy.md) · [M-0116](../roadmap/phase-11/M-0116.md)
