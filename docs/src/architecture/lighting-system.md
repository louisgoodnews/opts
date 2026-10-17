# Lighting System (System 6: sun, moon, light rooms, god rays)

| Metadata | Value |
| :--- | :--- |
| **Status** | 🟡 Draft: design from measurements (Phase B, 2026-10-17). **Implemented: the quality-mode settings, the ladder state and the governor (section 9)**; the lighting effects themselves are not |
| **Branch** | `agent/lighting-content` (docs only; content probes were throwaway scripts) |
| **Code today** | `opts_bevy::environment` (sun position, `SessionEnvironment`), `opts_game::in_game::{sky,world}` (`SunLight`, discs, fallback light), `opts_bevy::scenery_runtime::{lights,maplights,effects,spawn}`, `opts_bevy::{vehicle,vehicle_glow,vehicle_lights}`, `opts_game::look` |
| **Related** | [Session environment](session-environment.md), [Scenery runtime](scenery-runtime.md), [Systems report §9](../meta/berlin-spandau-systems-2026-10-17.md#9-round-3-lighting-cones-and-volumetrics-2026-10-17-branch-fixmap-and-lighting-systems), [`envir.cfg`](../formats/text/environment.md), [`.sco`](../formats/text/sco.md), [`model.cfg`](../formats/text/vehicles/model_cfg.md), [Reflections research](reflections-research.md) |
| **Last Updated** | 2026-10-17 |

Evidence tags: ✅ measured on the installed content or read in the Bevy 0.20.0 source (file and line given), ❓ open, 📚 community wiki (see the format pages). Counts come from throwaway scans of the installed OMSI 2 files (text and config only, ADR-0002); the scripts are not kept.

## 1. Goals and non-goals

Goals:

1. A lighting model in which every light the content declares has one owner, one Bevy entity kind, one cost class and one switch (section 3).
2. Night that reads like OMSI's: dark ambient (the installation's `envir.cfg` is near-black at night), street lamps, vehicle lights, and in OPTS only, moonlight with a real phase and direction.
3. Sun (and optionally moon and player headlight) light shafts through Bevy's `VolumetricFog`, within a fixed GPU budget and with a fallback ladder (section 9).
4. Interior versus exterior light handled by the cheapest mechanism that the measurements justify (section 4). The measurements say that most of what the brief calls "light rooms" is not needed, and that Bevy's `RenderLayers` cannot do the rest.

Non-goals: lantern and AI-vehicle cones (a separate cone technique, see [Systems report §9](../meta/berlin-spandau-systems-2026-10-17.md)), weather rendering (the weather agent: fog, clouds, precipitation), Bevy's physical `Atmosphere` (the sky stays the installation's textures).

## 2. What the content says (measurements)

### 2.1 Vehicle lights (3,568 `model*.cfg`, 2026-10-17)

| Keyword | Uses | cfgs | Measured |
| :--- | ---: | ---: | :--- |
| `[interiorlight]` | 6,174 | 829 | range min 0, **median 1.14 m**, max 8.1; 1.0 m ×1,521, 2.0 m ×1,305, 1.5 m ×575, 0.85 m ×464; only 211 above 2.5 m and 82 at 5 m or more (all one pack's under-deck lights). Colour `205 200 175` ×2,657 (43 %), white ×860. Height z median 2.75 m, x median 0; 6,019 of 6,022 (files with an exterior reference) have \|x\| ≤ 1.4 m. 124 distinct variables, led by `lights_beleuchtung_unterdeck` 1,634, `…_oberdeck` 1,182, `lights_fahrerlicht` 333, `…_untenrechts` 259, `door_light_1` 224; 5,375 (87 %) have interior-style names, 737 are driver/cockpit lights |
| `[spotlight]` | 2,206 | 662 | range 500 m ×411, 100 m ×348, 450 m ×325, 200 m ×198; cones 30/70 ×1,228, 30/80 ×451, 20/80 ×175 |
| `[light_enh_2]` | 32,218 | 1,557 | 1,192 (3.7 %) are bound to interior-style variables (`licht_innenraum_1` 308, `lights_fahrerlicht` 135, `door_light_*`); the rest are `lights_stand` 6,302, indicators 4,765, `ai_light` 2,313 and similar exterior lamps |
| `[light_enh]` | 1,091 | 233 | legacy indicators |
| `[matl_lightmap]` / `[matl_nightmap]` | 124,782 / 76,003 | 937 / 973 | emissive saloon and dashboard textures; `lights_beleuchtung_unterdeck` alone drives 4,398 light-map uses |

Consequences:

* An interior light is a **small, local** light: half of them reach 1 to 2 m, only 296 (4.8 %) are long enough to reach the ground (range at least their height), and 4,227 (68 %) reach through an assumed 1.27 m half-width side wall (most are door and wall lights near the side). The leak outside is therefore at most about 1 m past the hull. The docs' earlier "corpus values 0.25–1" is corrected ([model.cfg](../formats/text/vehicles/model_cfg.md)).
* The vehicle's own interior is lit twice in OMSI terms: by these lights and by emissive light maps driven by the same variables. OPTS can show the saloon lit from afar through the light maps alone.
* Docs 📚: only the player vehicle's interior lights count, at most four of them light moving passengers. Per-cfg counts are higher (median 6, maximum 24; 639 cfgs have more than four), so "four" is a per-passenger limit, not a per-vehicle one ❓.

### 2.2 Scenery (55,721 `.sco`, and the 29,450 `[object]` records of the Berlin-Spandau tiles)

| Keyword | `.sco` files | Placed in Berlin-Spandau |
| :--- | ---: | ---: |
| `[maplight]` | 917 | 1,607 |
| `[nomaplighting]` | **688** | **1,588** |
| both | **654** (95 % of `[nomaplighting]`, 71 % of `[maplight]`) | **1,587** (99 % of `[maplight]` placements) |
| `[LightMapMapping]` | 9,413 (7,164 with `[surface]`) | 246 |
| `[matl_lightmap]` / `[matl_nightmap]` | 456 / 291 | 73 / 267 |
| `[NightMapMode]` | 13,582 (values 0: 6,036, 2: 4,966, 1: 1,693, 3: 771, 4: 88) | 3,148 |
| `[light_enh_2]` | 1,595 | 1,938 |

* **`[nomaplighting]` marks objects that carry their own `[maplight]`: street lamps, neon and sodium lamps, tunnel lamps, bus shelters with their own light.** 654 of 688 files have a `[maplight]`, 594 have `[light_enh_2]`, 558 have `[shadow]`; by name 381 are lamps, 170 road furniture, 24 buildings, 9 bus stops, 3 tunnel objects. Berlin-Spandau's top placements are `neonlight_M_whip_S` 197, `neonlight_L_whip_L` 184, `strlt_gwg_snowb_single` 148, `neonlight_L_bend_L` 105, `neonlight_S_break_S` 94, and the bus shelter `hst_wartehaus_alt01` 73 (its own maplight sits inside it: `0 0 2`, warm `0.5 0.4 0.3`, radius 2 m). Reading: **an object is not lit by map lights (so a lamp does not light its own housing)**, not "a room that is shielded from outdoor lights". It does not correlate with interiors, tunnels or buildings (14 of 688 have `[LightMapMapping]`).
* **`[LightMapMapping]` is the tile's baked terrain light map, not a room**: each tile has a 256 × 256 `*.map.LM.bmp` (10,546 files, [tiles.md](../formats/map/tiles.md)); 76 % of the objects that use it are `[surface]` (road ribbons that take the tile's shading). 16 files combine it with `[matl_lightmap]`.
* **`[matl_lightmap]` / `[matl_nightmap]` are lit windows**: by name 106 / 53 are bus stops and shelters, 38 / 38 buildings, 137 `[matl_lightmap]` are signs and signals.
* `[maplight]` values (1,436 lights): radius at full intensity median 3.5 m (0 to 30), z median 6.0 m, colours `1.0 0.8 0.5` ×350, `0.9 0.9 1.0` ×267, `0.8 0.8 0.9` ×186.
* No scenery object in the installation declares an "inside" volume. Tunnels (198 by name) have 7 `[maplight]`, 8 `[light_enh_2]` and 19 `[LightMapMapping]`; stations, shelters and halls behave like other buildings.

### 2.3 What OMSI's own documentation says (docs search)

[`sco.md`](../formats/text/sco.md): `[maplight]` "illuminates the **map**" (inverse-square falloff after the radius at full intensity); `[nomaplighting]` "Object is not lit by map lights"; `[LightMapMapping]` "the tile's light map affects the object's textures and illuminates the object". [`model_cfg.md`](../formats/text/vehicles/model_cfg.md): interior lights "shine through objects (no shadows)", "only the player vehicle's interior lights count". Whether OMSI's map lights also light vehicles ❓ is not documented and not measurable from content (needs an OMSI experiment, [pending verification](pending-omsi-verification.md)).

## 3. The six lights

| # | Light | Data | Bevy entity | Shadows | Layer | Count / cost class | Owner file (today → change) |
| ---: | :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| 1 | **Sun** | `envir.cfg` `[lightcolor_A]` (direct), elevation from `SessionEnvironment` | 1 `DirectionalLight` (`SunLight`), shadow cascades 4 × 2048² (Bevy defaults: first bound 10 m, maximum 150 m) | yes while it dominates | 0 | 1; the one expensive light | `in_game/sky.rs` (`update_sky`); hidden below about −2° |
| 2 | **Moon** | date and time (section 5), phase | 1 `DirectionalLight` (`MoonLight`), new | **only when it is the dominant light** (section 5.4) | 0 | 1 | new `in_game/moon.rs`; `opts_bevy::moon` for the model |
| 3 | **Map lights** | `[maplight]` | pool of 48 shadowless `PointLight` | no | 0 | 48 pooled, nearest 200 m | `scenery_runtime/maplights.rs` |
| 4 | **Object lights and glows** | `[light_enh]`, `[light_enh_2]`, light maps | glow sprites, **no light source** | no | 0 | thousands, sprite cost | `scenery_runtime/{lights,effects}.rs` |
| 5 | **Vehicle lights** | `[spotlight]`, `[light_enh_2]` (headlights, brake, indicators) | player: `SpotLight` per `[spotlight]`, shadow maps on; AI/parked: glow sprites plus a pool of 6 shadowless `SpotLight` | player yes, pool no | 0 | ≤ 2 shadowed + 6 pooled | `vehicle.rs`, `vehicle_glow.rs`, `vehicle_lights.rs` |
| 6 | **Interior lights** | `[interiorlight]` | player vehicle only: `PointLight` per entry (up to 24), no shadows. AI and parked vehicles: none (emissive light maps only) | no | **1** | ≤ 24, only for the player vehicle | `vehicle.rs::spawn_interior_light_child` |

Light units are the game's, not physical: sun peak 20,000 lx (`SUN_ILLUMINANCE`, about one fifth of real sunlight), map-light "full" 10,000 lx, Bevy's default `Exposure` is EV100 9.7 (`bevy_camera-0.20.0/src/camera.rs:264`). Physical night values (a full moon is 0.27 lx, a street lamp 15 lx) are invisible at this exposure, so moon and lamps use compressed game values (section 5.3).

## 4. Light rooms and the layer convention

### 4.1 What Bevy 0.20 does with `RenderLayers` on lights (verified)

The brief assumed that a light only affects meshes that share a layer with it. That is **not** what Bevy does:

| Question | Answer | Source |
| :--- | :--- | :--- |
| Does a light's layer filter which meshes it **shades**? | **No.** The forward shader loops over the clustered lights of the fragment's cluster and tests only the mesh's shadow-receiver bit and the light's shadow flags, never a layer. There is no per-mesh light mask in the PBR shader. | `bevy_pbr-0.20.0/src/render/pbr_functions.wesl:463-482` (loop), `render/mesh_types.wesl:74-80` (the only mesh flags) |
| Does it filter which **views** the light is in? | Yes: a point or spot light enters a view's clusters only if the view's layers intersect the light's layers. | `bevy_light-0.20.0/src/cluster/assign.rs:491` |
| Does it filter which meshes **cast shadows** for it? | Yes: for directional (`lib.rs:406,436-437`), point (`lib.rs:588,610-611`) and spot lights (`lib.rs:679,702-703`) a mesh is a caster only if its layers intersect the light's. | `bevy_light-0.20.0/src/lib.rs` |
| Do layers change the **number of shadow cascades** allocated? | Yes: per view, only directional lights with `shadow_maps_enabled` whose layers intersect the camera's count. | `bevy_pbr-0.20.0/src/render/light.rs:1456-1463` |
| Does `RenderLayers` propagate to children? | **No** (unlike `Visibility`). `bevy_app::Propagate<C>` can do it, but `HierarchyPropagatePlugin::<RenderLayers>` is not registered by any Bevy plugin; the game must add it. | `bevy_app-0.20.0/src/propagate.rs:44,77`; grep of `bevy_camera`, `bevy_render`, `bevy_internal` finds no registration |
| An entity without `RenderLayers`? | Layer 0; a camera without it renders layer 0 only. An empty `RenderLayers` is invisible. | `bevy_camera-0.20.0/src/visibility/render_layers.rs:5,45-50,115-135` |

So layers are **a per-view light selector and a per-light shadow-caster selector**, not a per-mesh light mask. Interior and exterior meshes in the same view are lit by every light of that view.

### 4.2 Why that is acceptable for the content

* **Vehicles.** An interior point light is small (median 1.14 m) and sits inside a hull whose outer faces point away from it, so even without shadows it lights the outer skin not at all (the dot product of the outward normal and the direction to the light is negative) and only things within about 1 m of the side. 95 % cannot reach the ground. OMSI itself uses no shadows for them. No layer separation is needed for correctness.
* **Map lights.** OMSI map lights are unshadowed too, and the content has no interior volumes (section 2.2), so a lamp in a shelter and a lamp over a bus are the same kind of light. The one visible failure is a street lamp directly above the player's vehicle lighting the saloon through the roof. That is handled on the CPU (4.5), not with layers.
* **Buildings.** OMSI buildings have no modelled interiors (no `[interiorlight]` for scenery exists; windows are `[matl_lightmap]` / `[matl_nightmap]` emissive). Nothing to separate.

### 4.3 The convention

| Layer | Name | Holds | Camera includes it |
| ---: | :--- | :--- | :--- |
| **0** | `WORLD` (Bevy default; no component needed) | everything: terrain, scenery, vehicle meshes (outside and inside), sky dome, sun and moon discs, sun and moon lights, the map-light pool, glows, headlight pools, AI lights | always |
| **1** | `CABIN` | **lights only**: the `PointLight`s of the player vehicle's `[interiorlight]` (and later a cab dome light). No meshes. | the main camera **while a vehicle interior view is active** (driver, passenger or cockpit view) or while it is within 12 m of the player vehicle (15 m to leave, hysteresis); never mirror, minimap, benchmark-capture or reflection cameras |
| 2 | reserved | a second camera pass for the interior ("viewmodel" pattern), only if section 4.4 option C is ever needed | – |
| 3 and up | free | tools (editor previews) | – |

Rules:

1. A light's layers are chosen by its **owner kind**, never by its position: interior lights `RenderLayers::layer(1)`, everything else default. `ScriptLight` gets a `cabin: bool` (or the spawn function inserts the component).
2. The camera's layers are set by one system `camera_light_layers_system` (new, in `opts_game::in_game::camera` or `opts_bevy::camera`): `{0}` or `{0,1}` from the active `VehicleCameraMode` and the distance to the player vehicle. A camera without the component renders `{0}`.
3. Shadow casting is untouched: interior lights have `shadow_maps_enabled: false`.
4. The player vehicle's lights and meshes live under the vehicle root; the interior lights are the only entities that need the layer, so **no propagation plugin is needed** for v1 (set the component at spawn). If meshes ever need it, register `HierarchyPropagatePlugin::<RenderLayers>` in `PostUpdate`.
5. AI, parked and static vehicles get **no interior lights** (✅ already so: only `opts_bevy::spawn_vehicle_model` in `load_player_vehicle` creates them; `omsi_world/tile.rs` and `traffic.rs` spawn glows only). Their saloon lighting is the emissive light map.

**What the layer buys (measured by reading the source, to be confirmed by the benchmark):** with the camera outside the vehicle, the up-to-24 interior point lights are not in the view's clusters (no per-fragment loop cost, no cluster assignment); mirror and minimap cameras never pay for them; the interior and the exterior look is unchanged. It costs nothing to change, because it is one component on the lights and one component on the camera.

### 4.4 Real light separation: options considered

| Option | What it does | Verdict |
| :--- | :--- | :--- |
| A. **Range and normals** (current) | rely on small ranges and unshadowed hulls, as OMSI does | **chosen**; see 4.2 |
| B. **CPU light selection** | choose and scale lights by where the camera is (4.5) | **chosen** for the lamp-over-the-bus case |
| C. **Second camera for the interior** (layer 2, `ClearColorConfig::None`, depth cleared) | interior meshes with cabin lights drawn over the world; window openings show the world | possible for first-person views only; doubles MSAA targets, cluster assignment and culling for one view, breaks every `With<Camera3d>` query that assumes one camera ([reflections research](reflections-research.md)), needs the sun duplicated on layer 2 (a second shadow map). Not worth it unless B proves insufficient ❓ |
| D. **Forked PBR shading with a light mask** | per-material light mask in a custom `ExtendedMaterial` | rejected: copies `apply_pbr_lighting`, breaks on every Bevy upgrade |

### 4.5 CPU rules that replace "rooms"

1. **Lamp above the player vehicle.** While the camera is in an interior view, `choose_nearest` in `maplights.rs` skips lamps whose position lies inside the player vehicle's footprint prism (horizontal bounding box grown by 1 m, from the roof height up). Lamps beside the vehicle still light through windows, as they should. Setting: `MapLightSettings::exclude_over_vehicle: bool` (default on).
2. **Interior views dim map lights.** `interior_view_scale` (default 0.5 ❓) multiplies the pool intensity while an interior view is active, so a lamp outside does not out-shine the cabin light. Tune on screenshots.
3. **Sun through windows needs nothing**: the sun has shadow maps and the vehicle interior meshes receive them.

### 4.6 `[nomaplighting]` handling

Meaning (measured, 2.2): the object is not lit by map lights: a lamp does not light its own housing. Bevy has no per-mesh light mask (4.1), so the mapping is not a layer. Plan:

1. **v1: store, do not act.** `SceneryObject::no_map_lighting` is already parsed. The spawn code adds a marker `NoMapLighting` to the parts of such objects (`scenery_runtime/spawn.rs`), so tools and tests can count them (1,588 in Berlin-Spandau).
2. **Look first.** The window that is under test (graphics agent) checks whether a lamp's own housing is blown out by its pooled point light: the housing's outer faces point away from the light, only faces facing it (reflector underside) can burn. If not visible, v1 stays as it is.
3. **v2 if blown out: night material swap (recommended).** While `SceneryClock::night`, parts with `NoMapLighting` get a shared cached `StandardMaterial` copy with `unlit: true` and `base_color` scaled by the night surface level (the ambient colour times a tunable factor ❓); day restores the original handle. One shared copy per original material keeps batching ([instancing research](instancing-research.md)). Files: `scenery_runtime/{spawn,effects,maplights}.rs`.
4. **Alternatives**: hemisphere `SpotLight` instead of a `PointLight` for lamps with a downward `[light_enh_2]` (the housing is behind the cone; cost as a point light without shadows); a forked shader (rejected, 4.4 D).
5. Docs: [sco.md](../formats/text/sco.md) row corrected with the census.

### 4.7 Edge cases

| Case | Behaviour |
| :--- | :--- |
| Camera inside the bus, interior lights on, looking through a window | World seen through glass is lit by sun, moon and map lights only; the interior lights add at most 1 m beyond the hull. Interior meshes see all lights. ✅ matches OMSI's "lights shine through objects" |
| Camera in the bus, lamp directly above | Skipped by 4.5 rule 1; other lamps through windows stay |
| Camera outside, night, bus interior lit | Interior point lights are out of the view's clusters beyond 12 m; the saloon still glows through `[matl_lightmap]` / `[matl_nightmap]`; within 12 m the point lights return |
| Mirror and minimap cameras | layers `{0}`: no cabin lights, no cost |
| Reflection capture cameras | must copy the main camera's layers, not default ([reflections.rs](reflections-research.md)) |
| Shelter lamp (`[nomaplighting]` + own `[maplight]`) | lights the road and the shelter's inner faces (unshadowed); its housing is handled by 4.6 |
| Tunnel | no special handling; lamps are map lights; sun shadows darken it |
| Two vehicles close (formation) | only the player vehicle (the one with the camera rig) has cabin lights; trains of several units: all units of the player formation, up to 24 lights each ❓ (count against the budget, section 9) |
| AI vehicle driven by the player after a switch | the new player vehicle is respawned through `load_player_vehicle`, which creates the cabin lights |

## 5. Sun and moon

### 5.1 Conventions (one sun, one moon)

* `opts_bevy::environment::sun_position(hour, day_of_year, latitude)` is the **only** sun formula (declination approximation, `LATITUDE_DEG` 50°, clock time = local solar time). Returned: elevation in degrees, **positive above the horizon, negative below; the horizon is 0**, and azimuth in degrees **clockwise from north** (0 north, 90 east, 180 south). `SessionEnvironment::sun_alt_deg/sun_az_deg` cache it every tick (`derive()`, `environment.rs:538-545`). `sky::sun_position` and `scenery_runtime::solar_altitude_deg` only call it. Direction in Bevy space: `(cos el sin az, sin el, −cos el cos az)` (north is −Z, east +X, `sky.rs::direction`), and the light's transform looks along `−dir`.
* Leftover duplicates to remove: the fallback light's hard-coded table in `world.rs::fallback_lighting` (hour-based pitch and yaw, 900 lx "moonlight") and the moon disc at `−sun_dir` in `update_sky` (an anti-sun is wrong except at full moon). Both read the one environment state in the new design.
* Thresholds: night switch `NIGHT_SUN_ALT_DEG` = 3° (lamps, AI lights, instrument palette); `envir.cfg` twilight −18° to +10° (sky texture blend, light colours).
* Accuracy of the existing sun: within 1.0° elevation of a full solar-position chain (sampled over all months at 6, 12, 18, 22 h; noon values differ by 0.0 to 0.9°). Good enough; the moon is computed relative to a sun from the same chain, so the sun and moon stay consistent with each other.

### 5.2 Moon model (`opts_bevy::moon`, pure functions, no Bevy types)

Input: `CalendarDate`, `seconds_of_day`, latitude. Output `MoonState { elevation_deg, azimuth_deg, elongation_deg, illuminated (0..1), phase_angle_deg, bright_limb_deg }`. The session clock is treated as UT (the map has no longitude; the error is the time-zone offset, at most about 0.5° of lunar motion per hour; documented, not corrected).

1. Julian day `J` from date and time; `T = (J − 2451545.0)/36525`.
2. Moon's ecliptic longitude and latitude from the 13 + 7 principal terms of the lunar series (mean elements `L'`, `D`, `M`, `M'`, `F` as in Meeus ch. 47 low-precision form): longitude terms `+6.288774 sin M'`, `+1.274027 sin(2D−M')`, `+0.658314 sin 2D`, `+0.213618 sin 2M'`, `−0.185116 sin M`, `−0.114332 sin 2F`, `+0.058793 sin(2D−2M')`, `+0.057066 sin(2D−M−M')`, `+0.053322 sin(2D+M')`, `+0.045758 sin(2D−M)`, `−0.040923 sin(M−M')`, `−0.034720 sin D`, `−0.030383 sin(M+M')`; latitude terms `+5.128122 sin F`, `+0.280602 sin(M'+F)`, `+0.277693 sin(M'−F)`, `+0.173237 sin(2D−F)`, `+0.055413 sin(2D−M'+F)`, `+0.046271 sin(2D−M'−F)`, `+0.032573 sin(2D+F)`. Mean elements: `L' = 218.3164477 + 481267.88123421 T`, `D = 297.8501921 + 445267.1114034 T`, `M = 357.5291092 + 35999.0502909 T`, `M' = 134.9633964 + 477198.8675055 T`, `F = 93.2720950 + 483202.0175233 T`.
3. Sun's longitude: `L0 = 280.46646 + 36000.76983 T`, `Ms = 357.52911 + 35999.05029 T`, `λ☉ = L0 + 1.914602 sin Ms + 0.019993 sin 2Ms`.
4. Elongation `ψ = acos(cos β cos(λ☾ − λ☉))`, illuminated fraction `k = (1 − cos ψ)/2`, phase angle `α = 180° − ψ` (0 at full moon). Waxing when `(λ☾ − λ☉) mod 360` is below 180.
5. Equatorial coordinates with obliquity `ε = 23.439291 − 0.0130042 T`; hour angle `H☾ = 15°(hour − 12) + (RA☉ − RA☾)` (this makes the moon relative to the sun exact and inherits only the app's local-solar-time convention); horizontal coordinates with the same formulas as `sun_position`.
6. Parallax (up to 0.95°) and eclipses are not modelled.

**Accuracy** (✅ measured with a Python implementation of exactly these steps against the published times of 11 principal phases, 2000-2025): the model's time of each phase is within **5 minutes** (maximum +4.9 min), i.e. the elongation is right to about 0.04°; illuminated fraction at the reference instants is 0.0000 or 1.0000 (new, full) and 0.5000 (quarters). Position of the moon on the sky: ecliptic longitude within about 0.3° (truncated series), plus parallax up to 0.95° and the time-zone ambiguity; fine for a disc whose artistic size is 7.7° (the real moon is 0.5°). The mean synodic-month alternative (29.530588853 d from the 2000-01-06 18:14 new moon) would be off by up to 16.6 hours (about 0.7 days, a phase error of about 0.1 in illuminated fraction), so it is **not** used.

Unit-test table (reference times are the published UTC times of the principal phases; the test solves for the phase near the reference and asserts the tolerance):

| Phase | Reference (UTC) | Model | Error |
| :--- | :--- | :--- | ---: |
| new | 2000-01-06 18:14 | 18:14 | +0.6 min |
| full (eclipse) | 2000-01-21 04:40 | 04:41 | +1.7 min |
| new | 2024-01-11 11:57 | 11:59 | +2.8 min |
| first quarter | 2024-01-18 03:53 | 03:53 | +0.3 min |
| full | 2024-01-25 17:54 | 17:57 | +3.8 min |
| new (eclipse) | 2024-04-08 18:21 | 18:21 | +0.3 min |
| full | 2024-06-22 01:08 | 01:12 | +4.9 min |
| full | 2022-11-08 11:02 | 11:04 | +2.5 min |
| full | 2025-03-14 06:55 | 06:57 | +2.8 min |
| new | 2025-09-21 19:54 | 19:54 | +0.2 min |
| last quarter | 2024-01-04 03:30 | 03:29 | −0.2 min |

Tolerance in the test: 10 minutes. Additional tests: `k` is 0 at new and 1 at full, `k(t)` waxes for 14.77 days; at new moon the moon's elevation is within about 6° of the sun's (same direction), at full moon the moon is about opposite; sample (2026-10-17, 22:00, latitude 50°): sun −42.5°, moon −4.9° (az 235°), illuminated 0.43 (waxing, near first quarter on 2026-10-18); at 18:00 the moon stands 13.6° high at azimuth 186°.

### 5.3 Moonlight (`MoonLight`, `DirectionalLight`)

* Direction from the model. Colour constant `MOON_COLOR` (0.72, 0.80, 1.00) (blue-shifted perception; `envir.cfg` has no moon at all, so this is **an OPTS addition, not OMSI behaviour** ❓).
* Illuminance in game units: `E = MOON_PEAK × f(α) × h(el) × w(sun) × weather.moon`
  * `MOON_PEAK` = 900 lx (the value the fallback light already used for night, unverified in the sky path ❓; the physical ratio full moon to sun is 1 : 400,000, which would be invisible).
  * `f(α) = (10^(−0.4 (0.026 α + 4·10⁻⁹ α⁴)))^γ` (Allen's lunar phase function: 1 at full, 0.23 at 60°, 0.09 at 90°, 0.026 at 120°) with `γ = 0.5` as a perceptual compression, so a half moon gives 0.30 of full instead of 0.09 ❓ (tunable `MOON_PHASE_GAMMA`).
  * `h(el) = smoothstep(−1°, 8°, el) × (0.4 + 0.6 sin el)` (below the horizon none; low moon is dimmer by extinction).
  * `w(sun) = 1 − smoothstep(−4°, 6°, sun elevation)`: the moon does not add light in daylight.
* The ambient at night stays `envir.cfg` (B and C are near-black: `5 5 10` and `2 2 5` at the nadir, `10 10 20` and `15 15 25` at −18°). The moon is the only directional light at night because `lightcolor_A` is black from the nadir to −18° (`update_sky` clamps it to 0.001): today **no light** shines at night in the sky path (the 900 lx moonlight exists only in the fallback light).
* Hidden (`Visibility::Hidden`) when `E < 1 lx`: a hidden directional light is not extracted for shadow work (`bevy_light-0.20.0/src/lib.rs:401` clears its visible entities when it has no `ViewVisibility`).

### 5.4 Which light carries shadows

At most one directional light has `shadow_maps_enabled` at a time (a second one would double the shadow pass: 4 cascades × 2048² depth, about 64 MB and four extra caster passes, `light.rs:1440-1470`). Selection (system `shadow_carrier_system`, hysteresis 15 %):

* `score = illuminance × step(elevation > 0)` for sun and moon; the larger score carries the shadows; ties and the first frame prefer the sun.
* Expected hand-over: sun `lightcolor_A` falls to black at −18°; against a full moon at 900 lx the moon wins near −16°; with a new moon nothing carries shadows after −18° (both hidden or near zero).
* The non-carrier keeps lighting without shadows for one second (cross-fade of illuminance over 1 s) so the swap is not a pop.
* Shafts at night: `VolumetricLight` goes on the **carrier only** (the shader requires a shadow-enabled light, `light.rs:1781-1787`), so moon shafts exist only while the moon carries the shadows.

### 5.5 Moon disc and phase

`MoonDisc` stays a billboard quad at `dome − 10 m`. `Texture/mond.bmp` is 128 × 128, 24 bit, a full-moon photograph on a **light background without alpha** (corner pixel 213,213,213, no near-black pixels), while `sky.rs` draws it additively: the quad is probably a bright grey square ❓ (to verify in a window). Replace the material with a small custom `MoonMaterial` (WESL, like the ground material):

1. Uniforms: `phase_angle`, `bright_limb_angle` (rotation of the terminator in the disc plane), `brightness`, `earthshine` (0.03).
2. Fragment: `uv → p ∈ [−1,1]²`, discard outside the circle `|p| ≤ 0.94`, sphere normal `n = (p, √(1−|p|²))`, light direction in the disc frame `L = (sin α cos χ, sin α sin χ, cos α)` with `χ` the bright-limb angle, `lit = smoothstep(−0.03, 0.03, n·L)`, colour `tex × (earthshine + (1 − earthshine) × lit) × brightness`.
3. Additive blending off the sky dome and fog off as today; `NotShadowCaster`/`NotShadowReceiver` stay. No Bevy light is involved (a lit sphere would need the sun lit on a layer, which lights everything, 4.1).
4. Visibility: scale to zero below the horizon as now; fade with daylight (`w(sun)` inverted) so the moon is visible by day faintly.

### 5.6 Interplay with `envir.cfg` and weather

| Quantity | Source | Notes |
| :--- | :--- | :--- |
| sun direct colour and intensity | `lightcolor_A` at the sun elevation × `SUN_ILLUMINANCE` × weather | unchanged; the sun is black below −18° |
| ambient | `lightcolor_C + 0.5 B`, `look` scale | unchanged |
| sky texture blend | `twilight_start_end` (−18 / +10) | unchanged |
| moon | section 5.3 | independent of `envir.cfg`; `w(sun)` uses the same elevation |
| shaft tint | the carrier's colour (orange at sunrise from `lightcolor_A`) | no Bevy `Atmosphere`, so `ATMOSPHERE` shader code stays off (`volumetric_fog.wesl:314`) |

**Weather hook (the one attenuation function).** Today `SessionEnvironment::attenuation()` returns 0.55 for any covered sky and `update_sky` uses it for the sun and the ambient. Replace it by one pure function in `opts_bevy::environment` that the weather agent calls from its own state:

```rust
/// What the sky does to each light, 0..=1 (1 = clear).
pub struct LightAttenuation { pub direct: f32, pub ambient: f32, pub moon: f32, pub shafts: f32 }
pub fn light_attenuation(w: &WeatherSky) -> LightAttenuation
// WeatherSky { cloud_cover: f32 /*0..1*/, visibility_m: f32, precip: PrecipKind, wet: f32 }
```

Rules (initial, tunable): `direct = 1 − 0.45·cover^1.5` (0.55 at full cover, the current value), `ambient = max(direct, 0.8)` (current `att.max(0.8)`), `moon = 1 − 0.65·cover` (cloud blocks moonlight more than it blocks the sun, the moon being the weaker light), `shafts = direct² × clamp(1 − cover, 0.15, 1)` (shafts need gaps; under a solid overcast there is no direct beam). Fog: the weather agent's distance fog and the volumetric haze are tied by the extinction `β = 3.912 / visibility_m` (Koschmieder); the haze used for shafts is `density = max(HAZE_BASE, β × FOG_TO_SHAFT)` with `HAZE_BASE` about 0.002 per metre ❓ (a clear sky has `β` ≈ 10⁻⁴ per metre, far too thin to show a shaft within the 150 m shadow range; the baseline is an artistic haze). `SessionEnvironment::attenuation()` stays as a thin wrapper that returns `direct`, so existing callers do not break. The lighting code only reads `LightAttenuation`; the weather agent only writes `WeatherSky`.

### 5.7 Risks

| Risk | Mitigation |
| :--- | :--- |
| Two `DirectionalLight`s | `MAX_DIRECTIONAL_LIGHTS` is 10 (`light.rs:236`); only one carries shadows (5.4); the shader loop over directional lights costs one extra iteration |
| Shadow memory and passes | only the carrier has shadows; moon off below `E` 1 lx |
| Moon too dark or too bright | `MOON_PEAK`, `MOON_PHASE_GAMMA` tunable in the Environment page debug section; screenshot at three phases |
| `update_sky` refreshes colours every 0.5 s | the moon uses the same cadence, the direction every frame |
| Fallback light path (no `envir.cfg`) | uses the same moon and attenuation functions; the hard-coded table goes |

## 6. Sun god rays with `VolumetricFog`

### 6.1 How Bevy 0.20 does it (read from the source)

| Item | Finding | Source |
| :--- | :--- | :--- |
| Components | `VolumetricFog` on the camera (`step_count` default 64, `jitter` 0, `ambient_color` white, `ambient_intensity` 0.1); `VolumetricLight` on the lights; **`FogVolume`** entities (a transformed unit cube) are what is actually drawn. Without a `FogVolume` nothing is rendered. | `bevy_light-0.20.0/src/volumetric.rs:14-17,25-60,74-76`, `bevy_pbr-0.20.0/src/volumetric_fog/render.rs:239-278,282-` |
| Cleanup | If no entity has `VolumetricLight` the extract step removes `VolumetricFog` and the pipelines from the render view and all `FogVolume`s: **removing `VolumetricLight` from the sun costs nothing**. | `render.rs:247-262` |
| Pass | One full-screen ray-march **per fog volume per view**, after the main pass and before post-processing, blended over the main texture; the cost is `pixels × step_count × (volumetric lights + one clustered-lights loop)`. A camera inside the volume draws a full-screen quad; outside, the cube's back faces. | `volumetric_fog/mod.rs:73-110`, `render.rs:282-420`, `volumetric_fog.wesl:113-445` |
| Step length | `step_size = ray_length / step_count`, where the ray runs from the near plane (or the volume entry) to the depth buffer or the volume's far side. **A huge volume therefore means coarse steps**: 64 steps over 4 km are 62 m apart. | `volumetric_fog.wesl:190` |
| Lights | directional lights **with shadow maps** (`DIRECTIONAL_LIGHT_FLAGS_VOLUMETRIC`, set only if `shadow_maps_enabled`, `light.rs:1781-1787`). **Correction to the systems report §9:** Bevy 0.20 also ray-marches **point and spot lights** that have `VolumetricLight` **and shadow maps** (`light.rs:1326-1332`, shader loop `volumetric_fog.wesl:340-445`, cube/spot shadow fetch `:446-520`); the module docs still say directional only (`volumetric_fog/mod.rs:12-16`). Untested here ❓. | as listed |
| Shadow range | outside the cascades' range the shader assumes **no light** (`light_local.w == 0` gives attenuation 0, `volumetric_fog.wesl:290-302`): with the sun's default cascades (maximum 150 m) there are no shafts beyond 150 m. | `bevy_light-0.20.0/src/cascade.rs:142-155` |
| MSAA | supported: the bind-group layout has a multisampled depth variant. | `render.rs:49-56,380` |
| Depth | needs the depth texture as a shader input; Bevy sets `TEXTURE_BINDING` on every `Camera3d`'s depth usage when any `VolumetricFog` exists. **No prepass required.** | `render.rs:701-715` |
| HDR | not required, but without `Hdr` the PBR shaders tonemap **inside the material shader** (`TONEMAP_IN_SHADER`, `bevy_pbr/src/render/mesh.rs:485-489`) and the fog pass then adds untonemapped linear light to an already tonemapped image; colours clip at 1. | as listed |
| Atmosphere | with Bevy's `Atmosphere` the shafts are tinted by the transmittance LUT; OPTS has none. | `volumetric_fog.wesl:314-325` |
| Alpha-blended meshes | the depth buffer holds opaque depth only, so shafts are drawn over blended window glass; blended materials in the shadow pass are alpha-tested (`light.rs:2585-2590`): whether bus windows cast partial shadows ❓ | |

### 6.2 The existing camera and light

* Cameras: `Camera3d::default()` at `in_game/vehicle_spawner.rs:14` (the tracking camera, with `VehicleCameraRig`, `IsDefaultUiCamera`) and `:34` (the free camera); the look system (`look.rs:173-190`, `apply_look_to_cameras`) later inserts `Tonemapping` and `ColorGrading`. **No `Hdr`, default `Msaa` (4 samples, `bevy_render-0.20.0/src/view/mod.rs:248-251`), no prepass, no SSAO.** Benchmark and test cameras use `Msaa::Off` (`opts-benchmark`).
* Sun: `DirectionalLight { shadow_maps_enabled: true }` with default `CascadeShadowConfig` (4 cascades, 0.1 to 150 m, first far bound 10 m, overlap 0.2) and default `DirectionalLightShadowMap` 2048.
* Mirror cameras (reflections prototype) toggle `Camera3d` per frame; they must never carry `VolumetricFog`.

### 6.3 Integration plan

1. **Camera:** insert `VolumetricFog { step_count: Q.steps, jitter: 0.0, ambient_intensity: 0.0, .. }` on the tracking and free cameras through one system in `look.rs`'s plugin neighbour (`LightShaftPlugin`, new `in_game/light_shafts.rs`), not at spawn, so the quality setting can remove it. `ambient_intensity` 0, because the default 0.1 adds a uniform haze with the volume's extinction over the whole screen.
2. **Fog volume:** one `FogVolume` entity (not `InGameEntity`-leaking; despawned with the session) scaled to a cube of side `2 × shaft_range` (default 200 m, never more than the cascade maximum distance) and **moved to the camera every frame**. The camera is then always inside (cheap quad path). `density_factor` from section 5.6 (`HAZE_BASE` to start), `scattering` 0.3 and `absorption` 0.3 (defaults), `scattering_asymmetry` 0.8 (default; strong forward scattering, shafts show when looking towards the sun), `light_tint` white, `light_intensity` 1.
3. **Light:** `VolumetricLight` on the **shadow carrier** (sun by day, moon by night, 5.4); added and removed by `shadow_carrier_system`, not at spawn.
4. **HDR decision:** requires a test (benchmark item): enable `Hdr` on the main camera so tonemapping moves out of the material shader and the fog is added in scene-linear space. The cost is an `Rgba16Float` target (twice the bytes; with 4× MSAA at 1920 × 1080 about 66 MB instead of 33 MB for the colour target) and a changed look pipeline (`Tonemapping` and `ColorGrading` still apply). If the look cannot be kept identical, ship shafts at LDR with a smaller `light_intensity` and accept the clipping ❓.
5. **Shadow quality:** the shafts reuse the sun's shadow maps. Cascades are fitted to the **view frustum** (`minimum_distance` to `maximum_distance`); if the benchmark shows shafts need a longer reach, raise `maximum_distance` for the sun only while shafts are on (cost: lower shadow resolution near the camera) or add a coarser second cascade set ❓.
6. **Settings:** `GameSettings::light_shafts: LightShaftQuality` (Off, Low, Medium, High) with `settings_registry` entry, a `--light-shafts` override (not persisted, like `--look`), and a row in the pause *Environment* page (section 8). Default **Medium** until the benchmark decides ❓.

### 6.4 What fails (and how it shows)

| Failure | Why |
| :--- | :--- |
| No shafts at all | no `FogVolume`, or no `VolumetricLight` on a light **with** `shadow_maps_enabled` (silently dropped, `light.rs:1781-1787`), or the camera is outside a small volume that does not cover the view |
| Washed-out grey screen | `ambient_intensity` above 0 or too high a density; LDR clipping |
| Shafts end at 150 m | shadow cascades end there |
| Banding or blocky shafts | a volume far larger than the shadow range with few steps (step = length / steps), or `jitter` 0 without temporal anti-aliasing; use more steps or a smaller volume; `jitter` helps only with TAA (OPTS uses MSAA) |
| Mirror cameras show shafts or cost time | never add `VolumetricFog` to them |
| DLSS or SSR switched on later | those need MSAA off (volumetrics work with either); they change the depth requirements ([reflections research](reflections-research.md)) |

### 6.5 What the benchmark must vary (to forward)

Metric: the GPU time of the `volumetric_lighting` span (Bevy's render diagnostics) and the frame time, 1920 × 1080, RTX 3060 12 GB, night and dusk scenes of Berlin-Spandau, after warm-up, per setting:

1. `step_count` 0 (off), 8, 16, 24, 32, 48, 64 (default).
2. Volume side 100, 200, 400, 1000 m (step length changes; image quality compared at the same time).
3. Camera view: looking into the low sun (most pixels lit), looking away, cockpit view through the windscreen, night with moon carrier, nothing lit (worst for nothing, best for the early out `background_alpha < 0.001`).
4. Density 0.0005, 0.002, 0.01 (high density ends rays earlier; shows the early out).
5. Volumetric lights: sun only; sun plus the player vehicle's two headlight `SpotLight`s with `VolumetricLight` (shadows needed); plus 4 spots; plus 8 spots (cost of the second raymarch loop and spot shadow maps).
6. Sun shadows: cascades 1, 2, 4; `DirectionalLightShadowMap` 1024, 2048, 4096; `maximum_distance` 100, 150, 250 m (cost of shafts is linked to cascade resolution).
7. `Hdr` off and on; MSAA off, 2×, 4× (volumetric cost and total cost).
8. Moon as carrier at night versus no carrier (does a shadow-less night save more than shafts cost).
9. Interior lights: 0, 6, 24 cabin `PointLight`s in the view versus on layer 1 with the camera outside (cluster cost).
10. Map-light pool 48 versus 24 with shafts on.
11. Whether alpha-blended window panes cast shadows and show shafts in the cab ❓.

## 7. Weather and the Environment UI

* The weather agent supplies `WeatherSky` (section 5.6); the lighting code never reads `Weather/*.owt` itself. The weather agent's distance fog is Bevy `DistanceFog` and is independent of the volumetric haze; both derive from the same visibility `V`.
* Pause *Environment* page (`pause/environment.rs`): add
  * **Light shafts**: Off / Low / Medium / High (the setting, live), with the frame-time governor state shown ("auto reduced to Low");
  * **Moonlight**: On / Off (an OPTS addition, default On);
  * **Night shafts (moon)**: On / Off (default Off until measured).
  Changes go through the settings registry and a live message, not through `EnvironmentChange` (they are display options, not session state). Saves do not store them.
* Control protocol: `sim.get` gains `moon {elevation, azimuth, illuminated, phase_angle}` and `shadow_carrier`; `sim.set_lighting {shafts?, moonlight?}` for tests ([control protocol](control-protocol.md)).

## 8. Data flow

```text
SessionEnvironment (date, time, weather, WeatherSky)         [environment.rs; writer: EnvironmentChange]
   |-- derive(): sun_alt/az (sun_position) + moon_state(date,time) + light_attenuation(WeatherSky)
   v
update_sky (every frame; colours every 0.5 s or on revision)  [in_game/sky.rs]
   |-- SunLight: colour lightcolor_A, E = SUN * att.direct
   |-- ambient, clear colour, sky texture blend (unchanged)
   |-- SunDisc, MoonDisc (MoonMaterial uniforms: phase, limb, brightness)
   |-- MoonLight: direction, E(f(alpha), h(el), w(sun), att.moon)          [new in_game/moon.rs]
   v
shadow_carrier_system -> exactly one of {Sun, Moon}: shadow_maps_enabled + VolumetricLight
   v
light_shafts_system  -> VolumetricFog on cameras (quality), FogVolume follows camera (density from att.shafts / haze)
camera_light_layers_system -> camera RenderLayers {0} or {0,1}  (interior view / distance to player vehicle)
scenery clock (NightlightA) -> map-light pool (exclude lamps over the vehicle; interior-view scale), NoMapLighting swap (v2)
```

## 9. Performance budget (measured 2026-10-17)

**Rule:** at least 60 fps at all times (frame time ≤ 16.7 ms; 13.3 ms is the stricter target of the 74.9 Hz second display) on the user's machine: **RTX 3060 12 GB, 1920 × 1080, 16 cores**, in the Berlin-Spandau night scene with all of System 6 at its default quality. Full method, tables, screenshots and caveats: [lighting cones and benchmarks](lighting-cones.md). The numbers are GPU frame-time **deltas** in a synthetic street scene made GPU-bound with a 5.1 ms ALU ballast (noise ±0.15 ms); the absolute frame times of that scene are meaningless.

**The real game today** (windowed, vsync as the game default, Rathaus 2402/11279, 22:00 and noon, two viewpoints each): 258-269 fps, 3.7-3.9 ms per frame, p99 5.6-5.7 ms; GPU 1.3-2.0 ms, so the game is **CPU-bound**. Night costs the same as noon in these views. **Existing problem outside lighting:** the 1 % lows are 34-47 fps because of tile-streaming hitches of 25-140 ms; no lighting setting can fix that, but it breaks the "never below 60 fps" rule independently of this work (follow-up).

| Item | Setting | GPU ms (delta) | Notes |
| :--- | :--- | ---: | :--- |
| Sun god rays (Bevy volumetric fog) | 8 / 16 / 32 / 64 steps, day | +0.45 / **+0.95** / +1.95 / +3.9 | 0.06 ms per step; flat across volume size 100-1000 m, density, view direction, sun elevation 3-30°, jitter, HDR, MSAA |
| Sun god rays at night with the game's 54 lights | 16 / 32 steps | +2.7 / +4.9 | Bevy's shader runs the clustered-light loop at every step (about 2.4× the day cost); halving the lamps changed nothing |
| Sun shadow map | 4096² vs 2048² | +1.2 to +1.5 | keep 2048² |
| Cascades | 1 vs 4 | ±0.6 on the fog cost | |
| Cone: additive mesh | 16 / 48 / 100 / 200 | +0.1 / +0.5 / +1.0 / +1.4 | no depth fade without a prepass |
| Cone: ray-marched proxy, 8 steps | 16 / 48 / 100 / 200 | +0.15 / +0.5 / +1.1 / +2.1 | 4 steps at N = 200: +1.45; 16 steps: +3.2; camera inside a cone +0.4 |
| Depth prepass (for cone depth fade/clip) | N = 100 / 200 | +1.6 / +2.2 | unmeasured in the real game; the prepass also cut the synthetic night scene from 8.77 to 7.57 ms |
| Bevy volumetric **spot** lights | 1 / 4 spots, 32 steps | +4.5 / +7.1 | supported in Bevy 0.20 (`light.rs:1321-1332`, `volumetric_fog.wesl:340-445`) but rejected on cost |
| Real `SpotLight` shadow maps | 1 / 6 / 8 | +0.4 / +1.3 / +1.9 | 0.15-0.25 each; the game's headlight pool is shadowless |
| HDR (`Rgba16Float`) | MSAA off / 4×, with and without fog | −0.1 to +0.1 (noise) | colour target 50 → 100 MB at 1080p with 4× MSAA (1440p 89 → 177, 4K 199 → 398) |
| Bloom (`NATURAL`) | – | +0.3 | optional |
| MSAA 4× | – | +0.27 | |
| Mirror camera (render-to-texture) | 1 / 2 mirrors | +0.6 / +1.4-1.5 | about 0.7 ms per view, independent of target size (256×128 = 512×256); 2 mirrors at 512×256 every 2nd frame +0.9 |
| IBL per camera (`GeneratedEnvironmentMapLight`) | main camera | +1.2 | about 1 ms each: share one cube map |

**Decisions drawn from the numbers**

* **HDR on the main camera, always** (cost within noise; geometry pixels identical to LDR, mean difference 0.02/255; only the tonemapped clear colour in the sky region differs: mean 4.7-6.1). A camera with `Hdr` renders its main texture as `Rgba16Float` and the output pass writes the 8-bit target (`bevy_render-0.20.0/src/camera.rs:619-626`), so **mirror cameras get HDR for free with an 8-bit image target**. Bloom is optional (+0.3 ms).
* **Sun rays: default 16 steps** (+1.0 ms by day, about +2.5 ms at night), never Bevy's default of 64; camera-following 200 m `FogVolume`, `ambient_intensity` 0, shadow map 2048². Shafts end at the sun's cascade maximum distance (150 m).
* **Cones: the ray-marched proxy cone** (per-cone angle, range and colour via the model matrix and a per-instance tag), **hard cap 96 simultaneous cones, default 64 (about +0.75 ms)**, nearest by screen coverage with hysteresis; the additive mesh cone is the cheaper fallback. **No Bevy volumetric spots.** Pitfall: `AlphaMode::Add` shares the premultiplied pipeline, so the cone shader must return alpha 0.
* **Mirrors:** at most 2, every 2nd frame, 256×128, switched with `Camera::is_active` (not the `Camera3d` add/remove trick of the research prototype).
* **Volumetric lights on screen:** one directional carrier (sun by day, moon at night, never both) plus no volumetric spots.

**Night budget at 1080p on the 3060:** existing game GPU 2.0 + rays 2.5 + cones 0.75 + HDR 0.1 + bloom 0.3 + mirrors 0.9 = about **6.6 ms**, against 16.7 ms (13.3 ms on the DP-2 display). Ceiling for the lighting additions alone: **4.5 ms**. Limits: the scene is synthetic; the depth-prepass and overdraw costs and the CPU cost of many cone entities, mirrors and light assignment must be re-measured in the real game; cone quality is not tuned.

**Unified fallback ladder** (setting `graphics.light_shafts` for the rays, `graphics.light_cones` for the cones; both can be driven by one governor):

| Level | Name | What runs |
| ---: | :--- | :--- |
| 0 | **Full** | rays 16 steps (day: sun, night: moon if "night shafts" is on), cones 64 (cap 96), mirrors ≤ 2 every 2nd frame, bloom if on |
| 1 | **Fewer cones** | cones 48, then 24; far cones at 4 steps; mirrors every 2nd frame at 256×128 |
| 2 | **Headlights only** | lanterns' cones off, vehicle headlight cones only (cap 16) |
| 3 | **Cones off** | glows and point/spot lights only |
| 4 | **Sun rays only** | 8 steps, no bloom, no mirrors |
| 5 | **All volumetrics off** | `VolumetricLight`, `VolumetricFog` and the `FogVolume` removed (zero GPU cost, `render.rs:247-262`); HDR stays |

**Quality mode (user decision, 2026-10-17: "give the user the choice and ability").** One setting `graphics.light_quality_mode` with two values:

* **Automatic** (default): a governor picks the ladder level from the measured frame time. It is frame-time based, because GPU timestamp queries exist only on Vulkan/DX12: one level down when the 95th-percentile frame time over 120 frames exceeds 15 ms for 2 s (never more than one step per 5 s); one level up when it stays below 11 ms for 15 s ❓ (thresholds tuned in the real game). The current level is shown in the Environment UI and the settings page.
* **Custom:** the governor never touches anything; every effect has its own control, on the settings page and the Environment UI: sun rays (off / low 8 / medium 16 / high 32 / ultra 64 steps), night (moon) shafts, cones for lanterns, cones for vehicle headlights, cone cap (0-96), cone depth fade, mirror count (0-2) and update rate, sky-cubemap reflections on glass, bloom. The ladder levels are presets of these controls ("Apply level n to custom").

Switching from Automatic to Custom keeps the level reached so far as the starting values. Defaults at level 0: sun rays medium (16 steps), **night shafts on**, lantern and headlight cones on (64, cap 96), **cone depth fade on** (needs a depth prepass on the main camera; it is a custom-mode control and the first thing the ladder drops after "fewer cones"), mirrors 2 at 256×128 every 2nd frame, bloom off.

### 9.1 Implemented: settings, ladder state and governor (2026-10-17, `agent/lighting-settings`)

The effects (rays, cones, mirrors, bloom) are not built yet. What exists is everything they will **read** and the controls that drive it.

**State.** `opts_bevy::light_quality::LightQuality` is the resource the effects read: `mode` (`Automatic` or `Custom`), `level` (0-5), `shafts` (`Off`, `Low` 8, `Medium` 16, `High` 32, `Ultra` 64 steps), `night_shafts`, `lantern_cones`, `headlight_cones`, `cone_cap` (0-96), `cone_depth_fade`, `mirror_count` (0-2), `mirror_every` (1, 2 or 4), `glass_reflections`, `bloom`. Order of systems: input (settings) → governor → resolve, all in `LightQualitySet`; effects schedule `.after(LightQualitySet)`. HDR is not part of it.

**The ladder as implemented** (`ladder_preset(level)`; the table is a unit test):

| Level | Name | Rays | Night rays | Lantern cones | Headlight cones | Cone cap | Depth fade | Mirrors | Glass | Bloom |
| ---: | :--- | :--- | :--- | :--- | :--- | ---: | :--- | :--- | :--- | :--- |
| 0 | Full | medium (16) | on | on | on | 64 | on | 2, every 2nd frame | on | off |
| 1 | Fewer cones | medium | on | on | on | 48 | on | 2, every 2nd | on | off |
| 2 | Headlights only | medium | on | off | on | 16 | off | 2, every 2nd | on | off |
| 3 | Cones off | medium | on | off | off | 0 | off | 2, every 2nd | on | off |
| 4 | Sun rays only | low (8) | off | off | off | 0 | off | 0 | on | off |
| 5 | All volumetrics off | off | off | off | off | 0 | off | 0 | on | off |

Decisions: the "cones 48, then 24" of level 1 is **not** a sub-step; level 1 is 48 and the next rung goes straight to the 16 headlight cones (24 against 16 differs by about 0.1 ms in the table above). Depth fade (the prepass, +1.6-2.2 ms) goes at level 2, right after "fewer cones". Night rays go at level 4 (a moon ray costs about 2.4 times the day cost). Bloom is off by default and so in every row, which also satisfies "no bloom" at level 4. Glass reflections stay on (they cost nothing measurable). The "far cones at 4 steps" detail of level 1 belongs to the cone effect, which reads the level.

**Settings** (Graphics page, section *Lighting quality*, all live): `graphics.light_quality_mode` (`automatic` default, `custom`), `graphics.light_shafts`, `graphics.light_night_shafts`, `graphics.light_lantern_cones`, `graphics.light_headlight_cones`, `graphics.light_cone_cap`, `graphics.light_cone_depth_fade`, `graphics.mirror_count`, `graphics.mirror_rate` (1, 2, 4), `graphics.glass_reflections`, `graphics.bloom` ([reference](../reference/settings.md)). In Automatic the ten controls are shown disabled with a note and display the values the governor applies now; the stored custom values stay untouched. In Custom the controls are authoritative and the governor does nothing. Switching **Automatic → Custom** writes the values of the level reached into the controls (so they are the starting values); **Custom → Automatic** restarts the governor at level 0.

**Governor** (`GovernorSettings`, a resource; defaults are the constants of one `Default` impl): window 120 frames of `Time<Real>` deltas; judged on the 95th percentile of a full window; one level down above 15 ms for 2 s, one level up below 11 ms for 15 s; at least 5 s between two steps; level stays in 0..=5; after a step the window starts again, so the new level is judged on its own frames. It counts frames only when `GovernorGate` is open: Automatic mode, app state `InGame`, the simulation not paused, the loading screen gone and loading complete, no map rebuild pending (`update_governor_gate`). A frame longer than 50 ms is a **hitch** (tile streaming, 25-140 ms in the measurements) and is not counted, neither in the window nor in the timers; but a run of more than 3 such frames in a row is a real slowdown and counts. Every change is logged once at INFO (`lighting quality level changed`, with `from`, `to`, `p95_ms`). `sim.get` reports `light_quality {mode, level, level_name, values}`.

**Environment page:** one row *Lighting quality* with a button *Automatic (level n: name)* / *Custom* that toggles the mode.

## 10. Files that change

| File | Change |
| :--- | :--- |
| `crates/opts-bevy/src/moon.rs` (new) | pure moon model, `MoonState`, tests (the table in 5.2) |
| `crates/opts-bevy/src/environment.rs` | `derive()` also stores the moon state; `LightAttenuation` / `light_attenuation()` replace the 0.55 constant; `attenuation()` becomes a wrapper |
| `crates/opts-game/src/in_game/sky.rs` | sun update uses `att.direct`; sun hidden below −2°; moon disc uses `MoonMaterial` and the model position; new sun/moon shadow-carrier system |
| `crates/opts-game/src/in_game/moon.rs` (new) | `MoonLight`, `MoonMaterial` (+ WESL shader), illuminance model, constants |
| `crates/opts-game/src/in_game/world.rs` | fallback light reads the same sun/moon/attenuation (drop the hour table) |
| `crates/opts-game/src/in_game/light_shafts.rs` (new) | `LightShaftPlugin`: camera `VolumetricFog`, `FogVolume` follower, quality ladder, governor |
| `crates/opts-game/src/look.rs` | none required; `Hdr` decision recorded here (6.3 item 4) |
| `crates/opts-game/src/in_game/camera.rs` | `camera_light_layers_system` (layers `{0}` / `{0,1}`) |
| `crates/opts-bevy/src/vehicle.rs` | interior `PointLight`s get `RenderLayers::layer(1)`; spotlights keep layer 0 |
| `crates/opts-bevy/src/scenery_runtime/{spawn,maplights,effects}.rs` | `NoMapLighting` marker; lamp-over-vehicle exclusion and interior-view scale in `choose_nearest`; v2 night material swap |
| `crates/opts-game/src/settings.rs`, `settings_registry/builtin.rs` and the `.ftl` files | `light_shafts`, `light_shafts_auto`, `moonlight`, `night_shafts` |
| `crates/opts-game/src/pause/environment.rs` | the three rows (section 7) |
| `crates/opts-game/src/control_environment.rs`, `docs/src/architecture/control-protocol.md` | `sim.get` moon fields, `sim.set_lighting` |
| `crates/opts-bevy/src/reflections.rs` | mirror and capture cameras keep layers `{0}` and no `VolumetricFog` (check after the layer system exists) |

## 11. Open questions

| ID | Question | Impact |
| :--- | :--- | :--- |
| Q-LS01 | Do OMSI map lights light vehicles and the player's saloon at all? Not decidable from content; needs an OMSI experiment. | 🟡 decides whether 4.5 rules are fidelity or taste |
| Q-LS02 | Is `[nomaplighting]` exactly "not lit by map lights" (read here), or also "does not receive the tile light map"? 14 of 688 files combine it with `[LightMapMapping]`. | 🟢 |
| Q-LS03 | Does a lamp's own housing blow out under its pooled point light in OPTS (4.6)? Needs a night screenshot at a lamp. | 🟡 triggers the v2 swap |
| Q-LS04 | `MOON_PEAK` 900 lx and `MOON_PHASE_GAMMA` 0.5 against the game's exposure; the night ambient is nearly black (`envir.cfg`). | 🟡 look |
| Q-LS05 | Does `mond.bmp` (no alpha, light background) draw as a square today? | 🟢 solved by the `MoonMaterial` mask either way |
| Q-LS06 | Is the session clock UT or local solar time for the moon? Treated as UT (up to 0.5° of lunar motion per hour of offset). | 🟢 |
| Q-LS07 | `Hdr` for the shafts: cost and whether the look can stay identical. | 🟡 benchmark |
| Q-LS08 | Do alpha-blended window panes cast (partial) shadows, and so shafts in the cab? | 🟢 |
| Q-LS09 | Volumetric `SpotLight` / `PointLight` (code present in Bevy 0.20, docs say directional only): works, and what does a shadowed headlight cone cost? | 🟡 headlight cones |
| Q-LS10 | Shafts reach: raising the sun's `maximum_distance` for shafts versus a second coarse cascade set. | 🟡 |
| Q-LS11 | Formations (trains): cabin lights of all player units (24 each) or only the controlled unit. | 🟢 |
| Q-LS12 | `HAZE_BASE` (0.002 per metre) and how it combines with the weather agent's fog density. | 🟡 weather agent |
| Q-LS13 | Per-passenger "four lights" rule versus 639 cfgs with more than four `[interiorlight]` (per cfg, not per LOD?). | 🟢 |
| Q-LS14 | Night-shafts default (moon); a game-design choice. | 🟢 |

## 12. Test plan

| Level | Test |
| :--- | :--- |
| Unit, `opts_bevy::moon` | the 11-phase table (tolerance 10 min), `k` at new/full/quarter, waxing/waning flag, new moon near the sun, full opposite, a 29.53-day periodicity of `k` within 0.01, azimuth in 0..360, determinism; sample 2026-10-17 22:00 values |
| Unit, `light_attenuation` | clear gives 1, full overcast gives the current 0.55 direct and 0.8 ambient floor, monotone in cloud cover, `shafts` is 0 under solid cover |
| Unit, moonlight | illuminance 0 below the horizon and by day, monotone in phase and elevation, 0 at new moon |
| Unit, shadow carrier | sun carries at noon, moon at 23:00 near full moon, nothing at new moon midnight, hand-over with hysteresis (no flicker over 100 steps through −16°), exactly one `shadow_maps_enabled` directional light at every time of a simulated day |
| Unit, layers | `camera_light_layers_system`: exterior chase `{0}`, cockpit `{0,1}`, 12 m / 15 m hysteresis; interior `PointLight`s have layer 1 and spotlights layer 0; AI vehicle spawn has no `PointLight` |
| Unit, `NoMapLighting` | marker count equals the `.sco` flag; night swap restores the original handle by day; shared copy per material |
| Unit, `choose_nearest` | lamp inside the vehicle's footprint prism is skipped in interior view and not in exterior view |
| Unit, ladder | each setting level produces the expected components (`VolumetricFog` step count, `VolumetricLight` presence, `FogVolume` count); Off leaves none; governor steps down at 17.5 ms and up at 13 ms with the stated delays and never above the choice |
| Headless app | `MinimalPlugins` plus `EnvironmentPlugin`: advance the clock through dusk; the sun hides, the moon appears, the carrier swaps once |
| Control protocol | `sim.set_time` to a new moon night and a full moon night; `sim.get` moon fields; `sim.set_lighting` |
| Render (graphics agent) | screenshots: dusk with shafts through trees, full moon night, first quarter, new moon, cockpit with shafts, lamp housing close-up, bus interior at night from inside and outside, mirror view; the quality ladder at each level; compare the moon disc shape to the phase |
| Benchmark | section 6.5 matrix, filling section 9 |

## Provenance

* Content scans (2026-10-17): all `model*.cfg` under `Vehicles/` (3,568), all `.sco` under `Sceneryobjects/` (55,721), the `[object]` records of the 329 Berlin-Spandau tiles, `envir.cfg` and `Texture/mond.bmp` / `sonne.bmp` (header and pixels). Python 3 scripts, not kept.
* Bevy 0.20.0 source from the local cargo registry (`bevy_light`, `bevy_pbr`, `bevy_camera`, `bevy_render`, `bevy_app`); every claim carries its file and line.
* Moon model: low-precision lunar and solar series after Meeus, *Astronomical Algorithms*, ch. 25 and 47 (the formulas, not the code); phase function after Allen's lunar magnitude formula; reference phase times from published tables of lunar phases (principal phases, 2000-2025).
* **ADR-0002 attestation:** no OMSI executable or DLL was inspected. Only OMSI text and configuration files and one texture were read. No OMSI asset is in the repository.
