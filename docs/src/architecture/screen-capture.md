# Screen Capture and Recording

| Metadata | Value |
| :--- | :--- |
| **Status** | 🟡 Design proposal (Phase A), not implemented |
| **Date** | 2026-10-14 |
| **Branch** | `agent/screen-capture` |
| **Decision record** | [ADR-0012](../adr/0012-screen-capture-encoding.md) |
| **Related** | [Control protocol](control-protocol.md), [F1 reporting](../meta/f1-reporting-2026-10-10.md), [Session save](session-save.md) (naming of session ids) |

This page designs capture commands for the [control protocol](control-protocol.md) so that CLI tools and AI agents can observe what the running application does: refined screenshots, burst capture and bounded recordings of the application window. Nothing here is built yet. Statements about Bevy or driver behaviour that were not verified are marked **(verify in B)**; they become measured facts in the Phase C report.

## 0. What exists today

* `screenshot {path?}` (protocol 1.0, `crates/opts-bevy/src/control/handlers/output.rs`) spawns Bevy's `Screenshot::window(window)`, waits at most 5 s for the `ScreenshotCaptured` observer, encodes a PNG on the control thread and writes it. The reply is only `{path}`. `report {text}` and the F1 flow use the same mechanism (`reporting/capture.rs`).
* `PathPolicy` canonicalises a path and checks it against `write_roots` (the reports directory plus `--allow-dir` directories), so `..` and symlinks cannot escape.
* `ControlState` has the render `frame` counter and the smoothed `fps`; `Time<Virtual>` is the simulation clock (paused by `sim.pause`); `CameraState` has the camera mode; `ReportContext` has version, git hash, map, camera and vehicle.
* `image` 0.25 is already compiled in because of Bevy (feature `avif`, so `ravif` and `rav1e` 0.8 are in `Cargo.lock`). No H.264 encoder and no MP4 or WebM muxer is in the tree. `cc` is present as a build dependency of other crates.

## A1. Capture surface

**What is captured:** the render target of the primary window, i.e. exactly what `Screenshot::window` returns: the 3D scene, the UI and every overlay that is part of the Bevy scene (HUD, toasts, trace overlay, egui-style panels). The size is the window's physical resolution (surface size), not the logical size, so a 1.5x scaled desktop gives the real pixel size.

**What is not captured:** the desktop, other windows, the mouse cursor (the OS draws it, it is not in the render target), window decorations, and anything drawn by the compositor. This is a property of the mechanism, not a filter: Bevy reads back the texture the application rendered, it never asks the OS for pixels. There is no code path that could include another window.

| Situation | Behaviour |
| :--- | :--- |
| Windowed | Capture the window's render target. |
| Borderless or exclusive fullscreen | Identical: the render target is still the application's own frame. The result is the monitor resolution but contains only application pixels; anything else on the desktop (other windows, notification popups) is not included because it was never rendered by OPTS. The distinction "window versus desktop" does not exist at the readback level. |
| Multiple monitors | Only the window's surface; a monitor spanning several windows cannot be captured. |
| Minimized | Fail with `NOT_READY` ("the window is minimized, nothing is rendered"). A blank image is never returned. Detection: physical size 0 or no new frame within the capture timeout **(verify in B)**. |
| Occluded or on another workspace | The swapchain may keep rendering (X11) or stop presenting (Wayland compositors withhold frame callbacks) **(verify in B)**. If no frame arrives in time the command fails with `TIMEOUT` and the message names occlusion as the likely cause; a frame that does arrive is valid. |
| Paused simulation (`sim.pause`) | Works: rendering continues, only virtual time stands still. The metadata shows the frozen `sim_time`. |
| Loading screen | Captured like any other frame (`snapshot` already does this). |

## A2. Command surface

All new capture commands are in a new group `capture` (protocol minor bump, additive; the protocol is 1.4.0 at the time of writing, so this would be 1.5.0). `screenshot` stays and is extended compatibly: the old call `{path?}` keeps working and its result only gains fields.

### Paths

* `screenshot.path` keeps today's meaning (any path inside the existing write roots) so existing scripts do not break. It is documented as legacy.
* New parameter `name` (string, optional): a **relative** file name or subpath below the captures directory (§A6), e.g. `run1/shot.png`. Absolute paths, drive letters, `..` and empty segments are rejected with `FORBIDDEN_PATH` before any filesystem access. `path` and `name` are mutually exclusive.
* `capture.*` commands accept only the relative form (`name` for files, `dir` for a burst or frame sequence). They never accept `--allow-dir` directories: captures live in one directory, nothing else.
* Omitted: the server generates `{datetime}_{session_id}_{counter}.{ext}` (§A6).

### `screenshot` (extended)

| Param | Type | Default | Meaning |
| :--- | :--- | :--- | :--- |
| `path` / `name` | string | generated | see above |
| `format` | `png` or `jpeg` | `png` | `jpeg` is lossy, for size |
| `quality` | 1..=100 | 90 | JPEG only |
| `include_metadata` | bool | `true` | write the sidecar and return the metadata |

Result: `{path, width, height, format, bytes, sim_time, frame, metadata_path?, metadata?}` (`metadata` is the object of §A3, present when `include_metadata`).
Errors: `NOT_READY` (no renderer, window minimized), `TIMEOUT` (frame did not arrive in 5 s), `FORBIDDEN_PATH`, `INVALID_PARAMS`, `FAILED` (encode or write failure, disk full).
Blocking: the reply waits for the frame, as today (a control `Waiter`, never the render loop). A screenshot is allowed while a recording runs (§A5).

### `capture.burst`

| Param | Type | Default | Limit |
| :--- | :--- | :--- | :--- |
| `count` | integer | required | 1..=300 |
| `interval_ms` | integer | required | 16..=60 000 (one frame at 60 Hz at least) |
| `format` | `png` or `jpeg` | `png` | |
| `dir` | relative path | generated `{datetime}_{session_id}_{counter}_burst` | |
| `metadata` | bool | `true` | |

Result: `{dir, paths: [..], frames: [{index, path, sim_time, frame, wall_ms}], dropped}`.
The command is **asynchronous**: it returns the final result when the last frame is written (the control reply is deferred through the existing `Waiter` mechanism; the connection stays usable for other clients because the server is non-blocking). Intervals are measured in wall-clock time; each frame carries its own `sim_time`, so a paused or slowed simulation is visible to the agent. A frame whose readback does not arrive within 2 s is skipped and counted in `dropped`. Total wall time is capped at 5 minutes. Counts toward the single-capture-job limit below.

### `capture.record.start`

| Param | Type | Default | Limit |
| :--- | :--- | :--- | :--- |
| `name` | relative path | generated | stem; the extension follows `format` |
| `format` | `frames` or `av1` (`h264` only if built with the optional feature) | `frames` | see §A4 |
| `fps` | integer | 30 | 1..=60 |
| `duration_ms` | integer | 30 000 | 1..=1 800 000 (30 min) |
| `max_size_mb` | integer | 512 | 1..=2 048 |
| `quality` | 1..=100 | 70 | encoders only |
| `scale` | 0.25..=1.0 | 1.0 | downscale before encoding (cheap way to cut cost) |

A stop condition is **mandatory**: `duration_ms` and `max_size_mb` always have defaults and hard caps, and the recording also stops when free disk space falls below 1 GiB. A request without any of them gets the defaults; a request above a cap is refused (`INVALID_PARAMS`), not silently clamped.
Result: `{recording_id, path, format, fps, effective_limits: {duration_ms, max_size_mb, min_free_disk_mb}}`. The id is returned immediately; the recording runs in the background.
Errors: `BUSY` (a recording or burst is already active), `FORBIDDEN_PATH`, `NOT_READY` (no renderer, minimized), `FAILED` (captures directory not creatable or not writable, or free disk space already below the threshold: refused before the first frame), `INVALID_PARAMS` (unknown or unavailable format).

### `capture.record.stop`

`{recording_id}` → `{path, duration_ms, frame_count, dropped, bytes, stopped_by}` where `stopped_by` is one of `request`, `duration`, `size`, `disk`, `degraded`, `encoder_error`, `window`. Stopping an already finished recording returns its final result (idempotent); an unknown id is `NOT_FOUND`. The reply waits until the encoder thread has flushed and closed the file, bounded by 10 s.

### `capture.record.status`

`{recording_id}` → `{state, frame_count, elapsed_ms, dropped, bytes, fps_measured, stopped_by?}`, `state` ∈ `recording`, `finishing`, `finished`, `failed`. Never blocks. Results of the last 8 recordings are kept in memory for these queries.

### `capture.prune` (maintenance, opt-in)

`{older_than_days, dry_run?}` → `{would_delete|deleted: [..], bytes}`. Only deletes files this subsystem wrote (files with a sidecar or the manifest of a frame directory) inside the captures directory. `dry_run` defaults to **true**. There is no automatic deletion.

### Concurrency

* One **capture job** at a time: a recording or a burst. A second `capture.record.start` or `capture.burst` gets `BUSY` with the active job's id in `data`.
* `screenshot` may run during a job, but never faster than one per 100 ms during a recording (otherwise `BUSY`), so it cannot starve the recording's readback queue.
* Recording is **off by default**: no setting, command or startup path starts one, and nothing is written until `capture.record.start` is called.

## A3. Metadata

Every capture has a metadata object. It is **returned** in the control reply and **written as a sidecar** next to the capture; it is not embedded in the image or video (PNG text chunks would survive moves but cannot be added to video; one mechanism for everything is simpler, and a sidecar is greppable by an agent).

* Sidecar name: the capture's name with `.json` appended (`shot.png` → `shot.png.json`). For a burst or a frame sequence: one `manifest.json` in the directory with the per-frame array, no per-frame sidecars. For a video: `<name>.json` with the recording summary plus a per-frame array of `{index, sim_time, frame, wall_ms}` (about 40 bytes per frame; 30 min at 30 fps stays under 3 MB).
* The sidecar is written after the capture file is complete (temporary file then rename), so a sidecar's presence means the capture is usable.

```json
{
  "schema": 1,
  "kind": "screenshot",
  "file": "20261014T101530_a1b2c3_0007.png",
  "created": "2026-10-14T10:15:30.512+02:00",
  "width": 1920, "height": 1080, "format": "png",
  "sim_time": 1832.46,
  "sim_paused": false,
  "frame": 220981,
  "fixed_tick": 109930,
  "fps": 58.7,
  "app_state": "InGame",
  "map": "Berlin-Spandau",
  "tile": [12, 7],
  "camera_mode": "3 External",
  "camera": {"translation": [..], "fov": 62.0},
  "vehicle": "MAN_SD202",
  "window": {"mode": "windowed", "scale_factor": 1.0},
  "app_version": "0.1.0",
  "git": "a16a02c",
  "session_id": "a1b2c3",
  "markers": {"report": null, "trace": false}
}
```

Sources: `sim_time` is `Time<Virtual>` elapsed seconds (so it excludes paused time and starts at 0 with the session); `frame` is `ControlState.frame` (render frame, the same number `state` returns); `fixed_tick` is the `Time<Fixed>` tick count. `map`, `tile`, `vehicle` and `camera_mode` come from `ReportContext`, the camera state and the world stats (absent keys are omitted, not null, except the explicit `markers`). `markers.report` is the id of an open F1 report, `markers.trace` is whether scene trace is on. The session id is the 6-hex id of the running process, also used in file names. Hardware information is **never** included (it is opt-in privacy data, [ADR-0010](../adr/0010-hardware-probe-privacy.md)). Absolute paths of the user's OMSI installation are never included.

## A4. Formats

| Use | Format | Notes |
| :--- | :--- | :--- |
| Screenshot | PNG (default), JPEG | PNG drops alpha like the F1 report does. |
| Frame sequence | `frames`: `dir/000001.png`, … + `manifest.json` | Default recording format. Zero new dependencies, each frame is independently decodable, best for agents that inspect individual frames, the largest on disk (about 1.5-3 MB per 1080p frame; the size cap protects the disk). |
| Video | `av1`: AV1 in an IVF container (`.ivf`) | Encoded by `rav1e`, **already in the dependency tree** through `image`'s AVIF support. IVF is a 32-byte header plus 12 bytes per frame and is written by hand (about 80 lines); ffmpeg, mpv, VLC and browsers via `ffmpeg` play it. No system dependency, no patent-encumbered codec. |
| Video (optional) | `h264`: H.264 in MP4 via `openh264` | **Not built by default.** Behind a cargo feature `capture-h264`. See ADR-0012 for why this is not the default. |

**When to use what.** Frame sequence for an agent that wants to look at frames (cheapest on CPU: PNG encode only, no inter-frame work). AV1/IVF when a human wants to watch a minute of footage or an agent wants a compact artefact. H.264/MP4 only where a viewer cannot play IVF and the build includes the feature.

**This differs from the task text's recommendation** (H.264 MP4 as the default video, PNG sequence for agents). The reasons are in [ADR-0012](../adr/0012-screen-capture-encoding.md): the `openh264` crate either downloads a Cisco binary at build time or compiles C++ sources (a system toolchain dependency and H.264 licensing questions), and there is no pure-Rust MP4 plus H.264 combination in the tree. This is the main point to approve or overrule at the Phase A checkpoint.

## A5. Performance

### Capture path (no per-frame blocking readback on the main thread)

```
 main world (Update)                render world                     encoder thread(s)
 ──────────────────                 ────────────                     ─────────────────
 recording tick (wall clock,        copy surface texture →           receive Image (bounded queue,
  every 1/fps s)                     staging buffer (GPU copy)        depth 3), convert/scale,
 spawn Screenshot::window ───────▶   map buffer asynchronously  ───▶ encode PNG | AV1, write file
 (if in-flight < 3, else drop)       ScreenshotCaptured fires         (own I/O; never the main thread)
```

* Bevy's screenshot copies the frame into a mappable buffer and maps it **asynchronously**; the main thread never waits on the GPU, and our capture never calls a synchronous readback (**verify in B** with the per-frame time trace; the task's concern about drain stalls is checked by measurement, not assumed).
* At most **3 captures are in flight** (spawned but not delivered). If the renderer is slower than the capture clock, the tick is skipped and counted as `dropped`. This bounds GPU buffer memory (3 × 8 MB at 1080p) and means a slow GPU degrades the recording, not the game.
* The observer hands the `Image` to the encoder through a bounded `crossbeam`-style channel (std `sync_channel`, depth 3). The CPU cost on the main thread is the observer's clone of the image bytes (about 8 MB at 1080p). If the channel is full the frame is dropped (`try_send`), never waited for.
* The encode runs on **dedicated worker threads** (not the Bevy task pools, so a long encode cannot starve asset streaming or tile jobs): 1 thread for PNG frames, 1 for AV1 with `rav1e` limited to 2 threads, lower thread priority where the platform allows it. Encoder I/O never touches the render loop.

### Budget (targets; Phase C measures them)

| Quantity | Target |
| :--- | :--- |
| Main-thread cost of a recording frame (spawn + clone + send) | ≤ 2 ms at 1080p, ≤ 1 ms with `scale` ≤ 0.5 |
| Render-world cost (copy + map) | ≤ 1.5 ms at 1080p |
| Frame rate during recording | ≥ 80 % of the rate before (the task's acceptance bar) |
| Memory | ≤ 3 in-flight GPU buffers + ≤ 3 queued images + encoder state (≤ 100 MB at 1080p) |
| Stop latency after the limit | ≤ 1 s to stop capturing; file finalised ≤ 10 s |

### Graceful degradation

Measured over a sliding 2 s window of the control `fps`:

1. **Below 45 fps:** the effective capture rate is halved (every second tick skipped). Reported as `dropped` plus `degraded: true` in status.
2. **Below 30 fps for 5 s in a row, or the encoder queue stays full for 3 s:** the recording is **stopped** (`stopped_by: "degraded"`) and the finished file is kept. Pausing is rejected as a policy: a paused recording whose timeline silently has a hole is misleading for an agent, a clearly truncated file is not.
3. Recording never makes a frame wait: every degradation step drops *capture* work, never *simulation or render* work.

### Bounded resources

| Resource | Default | Hard cap |
| :--- | :--- | :--- |
| Duration | 30 s (burst: 5 min) | 30 min |
| File or directory size | 512 MB | 2 GB |
| Free disk space | stop below 1 GiB | not configurable below 256 MiB |
| Concurrent jobs | 1 | 1 |
| Frames in a burst | none | 300 |
| Whole captures directory | 20 GB soft | a warning in `capture.record.status` and refusal to start above it; `capture.prune` frees space |

## A6. Storage

* **Directory:** `directories::BaseDirs::data_dir()/opts/captures` (Linux `~/.local/share/opts/captures`, Windows `%APPDATA%\opts\captures`, macOS `~/Library/Application Support/opts/captures`), next to the existing `reports` directory. Created on first use with default permissions of the user; the benchmark and test code can override it through `OPTS_CAPTURE_DIR`, which tests use to keep the real directory clean.
* The directory is added to `PathPolicy.write_roots` **only for the capture handlers** through their own resolver; `--allow-dir` directories do not apply to `capture.*` and `name`.
* **Naming:** `{datetime}_{session_id}_{counter}.{ext}` with `datetime` as `YYYYMMDDTHHMMSS` (local time, same as the F1 report stems), `session_id` the 6-hex process id and `counter` a 4-digit per-session counter. Bursts and frame sequences get a directory `…_{counter}_burst` / `…_{counter}_rec`; recordings get `{stem}.ivf` and `{stem}.ivf.json`.
* **Retention:** nothing is deleted automatically. `capture.prune` with `dry_run` defaulting to true is the only way, and it only removes files recognised as written by this subsystem.

## A7. Security

* **Path restrictions:** new commands take a relative subpath. Validation happens in this order: reject absolute paths, drive prefixes, `..` and empty or `.` segments and control characters; join below the captures root; create parent directories **inside** the root; canonicalise the existing parent and require it to start with the canonical root (symlinks that leave the root are rejected). Tests cover `../../etc/passwd`, `/etc/passwd`, `C:\x`, a symlink inside the directory pointing out, and `a/../../b`.
* **Rate limiting:** one job at a time; a start request while a job runs returns `BUSY`; at most 10 job starts per rolling minute (`BUSY` with `retry_after_ms`); screenshots limited as in §A2. Defaults (30 s, 512 MB) are lower than the caps the task suggests (30 min, 2 GB) because an agent that forgets to set limits should not fill a disk.
* **Window-only enforcement:** there is nothing to enforce in the fullscreen case, because the mechanism reads the application's own texture (§A1). The invariant is covered by a test that renders a known pattern and asserts the captured size equals the window's physical size.
* **Authentication:** the existing control token and bind rules apply; capture commands are not exempt.

## A8. Failure modes

| Failure | Behaviour |
| :--- | :--- |
| Disk full or below the threshold at start | `FAILED` before any frame; message states free and required space. |
| Disk fills mid-recording | Recording stops (`stopped_by: "disk"`); the finished part is kept and valid (frame sequence: complete frames only; IVF: header frame count is patched on finish). |
| Captures directory missing | Created. If creation fails: `FAILED` naming the directory. |
| Encoder failure mid-recording | `stopped_by: "encoder_error"`, state `failed`, the partial output is kept, the error text is in `status`; the game is unaffected. |
| Encoder thread panics | Caught at the thread boundary, treated as encoder failure. |
| Screenshot while paused | Works (§A1). |
| Screenshot while minimized | `NOT_READY` with a clear message; no blank image. |
| App exits during a recording | An exit hook sends stop and joins the encoder with a 3 s bound; otherwise the file is left as is. IVF without a patched count is still playable (the count is advisory). |
| Window resized mid-recording | The first frame fixes the size; a differently sized frame is scaled to it on the encoder thread (frame sequence keeps native sizes and records them in the manifest). |
| Client disconnects | The recording continues to its limit: it belongs to the app, not the connection. `capture.record.status` finds it by id from any connection. |

## A9. ADR

[ADR-0012: Screen capture encoding](../adr/0012-screen-capture-encoding.md) records the encoder choice (the task text called it 0011, which is already used by [ADR-0011](../adr/0011-session-save-format.md)).

## Implementation outline (Phase B)

`crates/opts-bevy/src/capture/` with `mod.rs`, `screenshot.rs`, `burst.rs`, `recording.rs`, `encoder.rs` (trait plus the PNG-sequence and AV1/IVF encoders), `ivf.rs`, `metadata.rs`, `storage.rs` (root, naming, validation, disk checks, prune) and `error.rs`. Handlers in `control/handlers/capture.rs`, CLI subcommands in `opts-cli`, protocol group `capture`, tests as listed in the task (screenshot dimensions and metadata, burst count and spacing, playable IVF header and frame count, disk-threshold refusal, automatic stop at the duration limit, dropped frames under load, path traversal, second recording rejected). Each file stays under 500 lines.

## Open decisions for the Phase A checkpoint

1. **Video format.** Approve AV1/IVF plus the frame sequence as the built-in formats with H.264/MP4 as an optional feature (recommended, [ADR-0012](../adr/0012-screen-capture-encoding.md)), or require H.264/MP4 by default (adds `openh264`, a C++ build step and the licensing questions in the ADR).
2. **Defaults.** 30 s and 512 MB defaults with the 30 min and 2 GB caps (recommended), or the caps as defaults.
3. **`screenshot.path`.** Keep the legacy any-allowed-directory meaning and add `name` (recommended, compatible), or make `path` relative-to-captures and break existing scripts and the `snapshot` documentation.
4. **Cursor.** Not capturable and not planned; say so if an agent workflow needs it drawn.
