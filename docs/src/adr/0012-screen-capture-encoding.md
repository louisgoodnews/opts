# ADR-0012: Screen recordings use a frame sequence and AV1 in IVF, H.264 stays optional

| Metadata | Value |
| :--- | :--- |
| **ADR ID** | 0012 |
| **Status** | ✅ Accepted |
| **Date** | 2026-10-14 |
| **Deciders** | Project lead (approved 2026-10-14), Claude (design in [screen capture](../architecture/screen-capture.md)) |
| **Supersedes** | None |
| **Superseded by** | None |

---

## 📌 Context

The control protocol gets bounded recordings of the application window ([design](../architecture/screen-capture.md)). The task asked for H.264 in MP4 as the default video format and a PNG sequence for agents, and forbade a hard dependency on the `ffmpeg` binary, GStreamer or any system codec. What the repository has today:

* `image` 0.25 with its `avif` feature is already compiled through Bevy, so `rav1e` 0.8 (a Rust AV1 encoder) is in `Cargo.lock`.
* No H.264 encoder, no MP4 muxer and no WebM muxer are in the tree.
* The `openh264` crate has two modes: download Cisco's prebuilt binary at build or run time (a network dependency and a binary we do not build), or compile Cisco's C++ sources with the system C++ compiler (`source` feature, a toolchain dependency). H.264 is also patent-encumbered: Cisco's binary comes with a royalty-free licence for the binary form only, so compiling the sources ourselves and shipping them is not covered.
* `ffmpeg-next` links the system FFmpeg libraries, which is exactly the system dependency the guardrails exclude.

## 🎯 Decision

**`capture.record` produces AV1 in an IVF container by default; `capture.burst` produces a PNG sequence; H.264/MP4 exists only behind an optional cargo feature.**

1. **AV1 in IVF** (`format: "av1"`, the default of `capture.record`): encoded with the `rav1e` already in the tree, muxed by a small hand-written IVF writer (32-byte file header, 12-byte frame headers). *Why `rav1e` and not `openh264`:* no new crate, no C++ toolchain, no download, no patent-encumbered codec. *Why IVF:* it needs no muxer dependency, it is trivially appendable while recording (a crash leaves a playable file), and FFmpeg remuxes it without re-encoding.
2. **PNG sequence** (`capture.burst`, and `format: "png_sequence"` for recordings): individual frames plus a manifest. It is a separate mode because it serves a different consumer, an agent looking at single frames, and because it is not a sensible *recording* default: 30 s at 60 fps is 1 800 files of 1.5-3 MB each, far above the 512 MB budget. As a recording format it is opt-in and the size cap ends it early.
3. **H.264 in MP4** (`format: "h264"`): only with the cargo feature `capture-h264`, off by default, using `openh264` with Cisco's downloaded binary. Without the feature the format is rejected with `INVALID_PARAMS` naming the feature.

Encoding runs on dedicated worker threads fed through a bounded queue; frames that do not fit are dropped, never waited for. Users who need MP4, MKV or WebM remux without re-encoding: `ffmpeg -i x.ivf -c copy x.mp4` (verified 2026-10-14 with FFmpeg n9.0.2, see the design).

## ⚖️ Alternatives considered

| Alternative | Why not |
| :--- | :--- |
| **H.264/MP4 as default** (the task's recommendation) | Needs `openh264` (binary download or C++ build) plus an MP4 muxer crate; adds a build-time dependency and patent exposure to every build, for a feature that is off by default. |
| **Pipe raw frames to an `ffmpeg` binary** | Forbidden as a hard dependency; allowed only as optional. It would still work well for users who have it, but it needs process management and a second code path. Left as a possible later optional backend. |
| **AV1 in WebM or MP4** | Needs a muxer; the available crates wrap C++ libraries (`libwebm`) or are young. IVF needs about 80 lines and is understood by FFmpeg, mpv, VLC and browsers' tooling. WebM/MP4 can be added later by remuxing without re-encoding. |
| **MJPEG in AVI** | Easy and universal, but large, and it adds a second lossy path next to PNG and AV1. |
| **No recording, bursts only** | An agent investigating timing needs continuous footage; a burst at 60 Hz is a PNG flood and cannot exceed 300 frames. |
| **GIF or APNG** | Palette or size problems at 1080p; no advantage for agents over the frame sequence. |
| **Platform capture APIs** (X11, PipeWire, Windows Graphics Capture) | Violates the window-only and no-system-codec goals and differs per OS. |

## ✅ Consequences

* No new crate and no system dependency in the default build. The cost is that the most common video container (MP4) is not produced by default; users can remux an IVF with FFmpeg (`ffmpeg -i x.ivf -c copy x.mp4`) and the design documents this command.
* `rav1e` is built here **without its assembly kernels** (the feature set `image` already uses; the assembly needs the `nasm` assembler, a system tool), which makes it noticeably slower than a normal `rav1e` build. `opts-bevy` depends on it directly with exactly that feature set, so nothing new is compiled. Measured on a 1920x1013 window it encodes about 9 fps at full size, 23 fps at half size and 29 fps at a quarter size (design page, "Measurements"), so recordings default to half size; full-size 30 fps would need the assembly build or an external encoder.
* `rav1e` is slow compared to hardware or x264-style encoders. At realtime speed presets it will not keep up with 1080p60 on a busy machine; the degradation policy (drop frames, stop on a persistently full queue) exists for that, and `scale` and `fps` are the knobs. Whether the 80 % frame-rate acceptance bar is met is measured in Phase C; if AV1 cannot meet it at 1080p30, `frames` stays the supported default and the finding goes into the report.
* The optional H.264 feature keeps a path open without making the licensing question everyone's problem. It is not tested in CI.
* Decoding a frame sequence is trivial for agents and tools, which is the main agent workflow.
* If the project lead decides MP4/H.264 must be the default, this ADR is superseded and the dependency and licence review has to be done first.
