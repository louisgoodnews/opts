# Audio System (M-0027)

| Metadata | Value |
| :--- | :--- |
| **Status** | 🟡 Engine implemented 2026-10-20 (this page, Phase A and the first part of Phase B of [M-0027](../roadmap/phase-4/M-0027.md)); the sounds themselves follow milestone by milestone |
| **Code** | `crates/opts-bevy/src/audio/` (engine), `crates/opts-omsi/src/formats/wav.rs` (decoder), `crates/opts-game/src/audio_content.rs` (clip cache), `control_audio.rs` (protocol), `settings_registry/live.rs` (`sync_audio`) |
| **Related** | [wav](../formats/wav.md), [sound.cfg](../formats/text/vehicles/sound_cfg.md), [control protocol](../reference/control-protocol.md) (`audio.*`), [settings](../reference/settings.md) (Audio page) |

## 1. What exists and what is planned

The engine is the layer every sound goes through. It does not know what a bus engine or a street lamp sounds like; it knows how to **decide what is heard** and how to play it. The sounds are added by later milestones in this order (project lead, 2026-10-20):

1. **The engine** (this page).
2. **Scenery objects and splines** ([M-0139](../roadmap/phase-4/M-0139.md)): the `[sound]`/`[loopsound]` blocks of `.sco` and `.sli`.
3. **AI traffic** ([M-0140](../roadmap/phase-4/M-0140.md)).
4. **The player's vehicle** ([M-0102](../roadmap/phase-4/M-0102.md) sound events; engine, tyres, brakes, doors, cockpit).
5. Weather, announcements and the rest after that.

## 2. Parts

| Part | Where | What it does |
| :--- | :--- | :--- |
| `PcmClip` | `audio/clip.rs` | Decoded audio (interleaved `f32`, 1 or 2 channels) as a Bevy asset with a `Decodable` implementation, so Bevy's output plays it without any optional decoder feature. |
| `SoundEmitter` | `audio/emitter.rs` | The component a game system puts on an entity: clip, bus, gain, speed, looping, an optional distance curve (spatial), mute, despawn-when-done. The game keeps `gain` and `speed` true to what the thing does; the engine does the rest. |
| `Distance` | `audio/curves.rs` | Reference distance (full level inside), maximum distance (silent beyond, costs nothing), rolloff; inverse distance law with a smooth fade over the last 15 % of the range. |
| `AudioSettings` | `audio/engine.rs` | Master and bus volumes (percent) and the voice budget. |
| systems | `audio/engine.rs` | `ensure_listener`, `select_voices`, `sync_voice_levels`, in `PostUpdate` after transform propagation. |
| `AudioStats` | `audio/engine.rs` | What the engine did last frame; `audio.state` reports it. |
| `ClipCache` | `opts-game/audio_content.rs` | One decoded clip per file, shared by all emitters that name it. |

## 3. Voices

An emitter is a *wish* to be heard; a **voice** is the actual playback (Bevy's `AudioPlayer` with a sink). `select_voices` runs every frame:

1. The **level** of every emitter: `gain × distance fade × bus volume × master`, where a volume setting of *p* percent is the gain `(p / 100)²` (the middle of the slider is about a quarter of the power, not half). A muted emitter, a disabled engine, a spatial emitter without a listener or without a position: level 0.
2. Levels below `AUDIBLE_MIN` (0.002) are not worth a voice.
3. The loudest `max_voices` (32) emitters get voices, the rest are **virtual**: they keep existing (a looping engine far away is still running in the game) and cost nothing. When one comes back in range or the budget frees up it gets a voice again (it restarts from the beginning of its loop: a position-keeping virtual voice is a later refinement).
4. A **one-shot** (`despawn_when_done`) that cannot be heard when it is triggered is dropped at once; one that played is despawned when Bevy has removed its finished player.
5. `sync_voice_levels` applies the level and the emitter's `speed` to the sink every frame (volume changes are cheap), and counts the sinks that exist: on a machine without an output device the voices rise but no sink appears (`audio.state.voices.playing` stays 0).

## 4. Space

The listener is the **main camera** (`MainCamera` gets a `SpatialListener` with 0.2 m between the ears). Panning is Bevy's: a stereo balance from the angle to the two ears. Bevy's own distance fall-off (`1/d²`, clamped) would add to ours; each spatial voice is therefore given a **spatial scale of `1 / max_distance`**, which keeps its scaled distance at or under 1 inside the range, where Bevy's attenuation is exactly 1, and the engine's curve alone fades the sound. The ratio of the two ear distances, which pans, is unchanged by the scale. Bevy mixes a spatial source down to mono before panning.

Not modelled yet: the Doppler effect, occlusion by buildings, reverberation in tunnels (see [M-0115](../roadmap/phase-11/M-0115.md)), and the interior/exterior filter of a vehicle cabin (milestone 4 of the list above).

## 5. Buses and settings

| Bus | Setting | Default | For |
| :--- | :--- | ---: | :--- |
| master | `audio.master` | 80 | everything; 0 switches the sound off and frees every voice |
| `Effects` | `audio.effects` | 80 | one-off sounds: doors, switches, horns |
| `Vehicles` | `audio.engine` | 80 | engines, tyres, brakes of vehicles |
| `Ambient` | `audio.ambience` | 60 | scenery objects, splines, weather |
| `Ui` | `audio.interface` | 60 | the interface |

The settings page (category Audio) writes them; `live::sync_audio` copies them into `AudioSettings` the frame they change. The keys of the first four existed since the settings registry (as "not available yet" entries); `audio.effects` is new.

## 6. Content

`opts_core::formats::wav::decode` reads RIFF/WAVE with PCM 8, 16 and 24 bit and IEEE float 32 bit, mono or stereo, any rate; the odd-sized chunk padding, a `data` chunk that claims more than the file holds and a partial last frame are handled. A file that is not RIFF (the five MPEG files named `.wav`), a missing `fmt `/`data` chunk or another sample format is an error with a reason. Name matching is case-insensitive in the installation lookup (3.7 % of references differ in case, [wav](../formats/wav.md)).

Decoding happens where `ClipCache::clip` is called, on the calling thread. A map's sounds can be hundreds of megabytes, so the milestones that place sounds load them on a worker and only hand the finished `Handle` to the frame; a limit on decoded memory (an LRU over the cache) belongs with them.

## 7. Protocol and checks

`audio.state` (volumes, voice counts, listener, the loudest emitters with their computed level and distance), `audio.tone` (a sine test signal, optionally positioned), `audio.play` (a sound file of the installation), `audio.stop`. The unit tests run without a sound card (`opts-bevy/src/audio/tests.rs`: range, budget, loops, one-shots, settings, listener; `curves`, `clip`, the WAV decoder, the clip cache, the settings sync). The output path is checked live: `audio.tone` shows up as a playback stream of the game in the system mixer (`pactl list sink-inputs`) with the volume the engine computed.

## 8. Scenery objects (M-0139)

`scenery_sound.rs` turns the `[sound]` configuration of a placed object into `SceneryVoice` children at the `[3d]` positions (the configuration is parsed once per file, cached in the load job). Three systems: `poll_clip_loader` (finished decodes into the assets), `stream_scenery_voices` (four times a second: a continuous voice gets a looping `SoundEmitter` when the listener is within its range and its clip is decoded, loses it beyond 1.25 × the maximum distance; the level is the product of its `[volcurve]`s over the session clock), `fire_scenery_triggers` (the events an object's script raised become one-shot emitters at the voice's place). Clips are decoded on two worker threads the first time a voice comes near, so no frame waits for a file and the 3.6 GB of scenery sound is never read as a whole. Objects without geometry (`[onlyeditor]` helpers) are built when they carry a sound, so their script and voices exist. Counters: `audio.state.scenery` and `audio.state.clips`.

Since M-0102 the scenery voices use the shared rules of `sound_logic.rs` (§10): a `[conditionSingle]` over the session clock gates a loop or fires a `[noloop]` voice on its edge. Not yet: `[viewpoint]`, `[onlyone]`, `[checkloading]`, script variables in `[volcurve]`, eviction of decoded clips, splines (no spline names a sound; road and rail rolling noise belongs to the vehicles, whose tyre sounds read the surface texture configurations, M-0102).

## 9. AI traffic (M-0140)

`vehicle_sound.rs`. When an AI vehicle type loads (`traffic_variant::load_unit`), `ai_sound_set` picks its `[sound_ai]` configuration (else the file `[sound]` names when it is a shared `Sounds/AI_Cars` file, or when the vehicle file or the configuration is an AI variant by name: `…_KI.bus` naming its own `…_ai.cfg`, as the MAN GN92 and EN92 AI buses do; 2026-10-24, they were silent before) and keeps the **playable** definitions: continuous, no condition, and only variables the traffic model states (`velocity`, `throttle`, `brake`, `engine_n`/`ai_motor`/`antrieb_n_kardanwelle` from the gear model, `wheel_rotationspeed_<axle>_<l|r>`, `n_wheel`, `streetcond`, `ai_engine`, `engine_throttle_injection`), at most 16 per vehicle; the rest is counted as skipped. At spawn the head unit gets `AiVoice` children at the `[3d]` places. `update_ai_variables` publishes the variables every frame into the vehicle's `AiVariables` (speed and acceleration from the agent, wheel speeds from the rolling radius, wet road from the precipitation rate); `stream_ai_voices` (five times a second) gives each voice near the listener a looping emitter on the **Vehicles** bus, with the level from its `[volcurve]`s and, for a loop, the speed `1 + (variable / reference − 1) × scale`. The engine speed is a stand-in gear model (idle 650 rpm, shifts at 2,100 rpm at 12, 25, 40, 60 and 90 km/h) with the right shape: ❓ OMSI's AI computes `AI_Motor` itself (Q-AU05).

Not yet: door sounds, horns (`ev_AI_Horn`), trailers' own sounds (only the head unit sounds), conditions, brakes squeal and the other script variables; trams and trains (the traffic has road vehicles only).

## 10. The player's vehicle (M-0102)

`player_sound.rs` and `sound_logic.rs`. `load_player_vehicle` calls `player_sound::attach`, which turns every definition of the `.bus` `[sound]` configuration into a `PlayerVoice` child at its `[3d]` place (the Citybus 530 has 330; definitions with an unsupported condition mode or a missing file are counted). `sound_logic` holds the rules all three layers share: what a definition does (`playback_of`: loop, once on the edge of its conditions, once per trigger), the condition comparison (modes 1 to 3), level (product of the `[volcurve]`s), pitch (`1 + (variable / reference − 1) × scale`: the reference value is the natural speed, the scale a sensitivity, Q-SND-001) and the `[viewpoint]` split (2 and 6 inside, 5 and 1 outside). The variables are the vehicle script's own (`engine.get(name)`), so every predefined and script variable a volume curve names is there: `engine_n`, `velocity`, `throttle`, the door and brake variables, … The vehicle's scripts' events (`(T.L.name)`, `(T.F.name)`, `(T.S.name)`) are kept in `VehicleScripts::raised` and played by `fire_player_triggers`.

`stream_player_voices` (twenty times a second): a loop within range of the listener gets a looping emitter on the **Vehicles** bus (silenced while its conditions fail or its viewpoint does not match), a `[noloop]` voice fires a one-shot on the edge of its conditions; one-shots are children of the voice and use the **Effects** bus. Whether the listener is inside the vehicle (a box about a solo bus' body) decides the `[viewpoint]`. Counters: `audio.state.player`.

Checked live (Citybus 530, Ahlheim 5): 330 voices, 44 loops with an emitter, 52 of its sound files decoded on demand (60 MB); pressing the battery switch and a door key plays their one-shots 1.7 m from the listener with a sink in the system mixer. Not checked live: the engine loops of a running engine (the engine of this bus does not start through the protocol keys yet, `engine_n` stays 0 and the loops are silent), tested instead over a script engine in the unit tests.

Trailers and the articulated bus' second unit have their own voices since 2026-10-20 (`player_sound::attach_unit`, children of the unit, driven by the head's variables and events; the Bremen MAN NG313's engine loops are in `sound_NG313_T.cfg`, its head has none). Not yet: announcements (the `(T.F)` payload files), the `Snd_*` mixer inputs, force feedback, the horn and the other definitions with an unknown condition mode, a cabin filter for outside sounds heard from inside (the viewpoint rule only switches them).

## 11. Open

| Id | Question |
| :--- | :--- |
| Q-AU01 | OMSI's own fall-off: its `[3d]` blocks give a minimum and maximum distance and a volume curve ([sound_cfg](../formats/text/vehicles/sound_cfg.md)); the engine's inverse-distance curve is a stand-in until the scenery milestone maps them. |
| Q-AU02 | Whether virtual loops should keep their play position so that a returning source continues instead of restarting. |
| Q-AU04 | Which surface a road or rail gives its tyre noise: the texture configurations next to the scenery (`x.bmp.cfg` with `[surface]`, `[moisture]`, `[puddles]`) look like the source; M-0102. |
| Q-AU06 | The comparison modes 0, 4 and 5 of `[conditionSingle]` (the horn, the retarder stages, gear selector): 749 definitions of the installed vehicles are left out until they are known. |
| Q-AU05 | The engine speed OMSI's AI writes to `AI_Motor`/`engine_n` of an AI vehicle (the gear model of M-0140 is a stand-in); it fixes the pitch of every AI engine loop. |
| Q-AU03 | Output device choice and a latency setting (Bevy uses the system default device). |


### 10a. What was wrong with the engine sound (2026-10-20, found by the project lead's ears)

Three readings of `sound.cfg` were wrong and made the stationary Citybus 530 sound like top speed and the Bremen MAN beep:

1. **Loop pitch.** `variable / reference × scale` played a loop recorded at its reference value at `scale` times its speed: the Citybus' idle loop (reference 600, scale 2) an octave up. The pitch is `1 + (variable / reference - 1) × scale` (natural speed at the reference value, the scale a sensitivity; `sound_logic::speed`, Q-SND-001).
2. **A pitch of zero is silence.** The loops without condition and curve, like `brake_warning` and `notheben_initialized`, are switched by their pitch variable: 0 stops them. They played at the lower limit of the speed (0.3, "deep beeping") all the time. A loop whose speed is below 0.1 is silent (`sound_logic::loop_voice`), which also silences the engine loops of a stopped engine.
3. **Rooms.** `[viewpoint]` 2 and 6 are interior sounds, 5 and 1 exterior sounds (Q-SND-004; the first reading had 2 and 5 swapped). Inside is decided by the camera: the driver's and the passengers' views are inside, the external view is outside (its camera hangs a few metres behind the body, which the old box test called inside), the walking and the free camera by their position. So the cabin plays `OM_906hLA_E2_*_v` and the outside `OM_906hLA_E2_ext_*`.

Also: the volume curve variable `-1` is the time the voice has been sounding (fade-in of the starter and retarder loops, Q-SND-005). `audio.state.player.loudest` lists the loudest loops with their file, gain and speed (idle Citybus, cockpit: `OM_906hLA_E2_0_v.wav` at speed 0.99; outside: `OM_906hLA_E2_ext_0_v.wav`; the Bremen MAN NG313 idle, cockpit: `Innen_idle.wav` at 0.96, outside `Motor_ext_leerlauf.wav`).


**Where the AI sounds come from (answer to "are they OMSI's .wav files?"):** the configuration and the `.wav` files are the installation's own (`[sound_ai]` or the light AI `[sound]` of the vehicle, the shared `Sounds/AI_Cars` files for cars); nothing is synthesised. What is simplified is the control: only the continuous definitions that need variables the traffic model states are played (6,319 of the 45,043 definitions of the 499 AI configurations qualify), so horns, indicators, door sounds and everything triggered by a script variable are missing, and the engine speed is a stand-in gear model. Reworking this means giving AI vehicles the variables their configurations read.
