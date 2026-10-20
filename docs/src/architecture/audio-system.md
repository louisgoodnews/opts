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

## 8. Open

| Id | Question |
| :--- | :--- |
| Q-AU01 | OMSI's own fall-off: its `[3d]` blocks give a minimum and maximum distance and a volume curve ([sound_cfg](../formats/text/vehicles/sound_cfg.md)); the engine's inverse-distance curve is a stand-in until the scenery milestone maps them. |
| Q-AU02 | Whether virtual loops should keep their play position so that a returning source continues instead of restarting. |
| Q-AU03 | Output device choice and a latency setting (Bevy uses the system default device). |
