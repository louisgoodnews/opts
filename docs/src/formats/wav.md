# `.wav` — Sound Files

| Metadata | Value |
| :--- | :--- |
| **Format Family** | Binary (RIFF/WAVE audio; standard container) |
| **File Extensions** | `.wav` (42,375); related: `.mp3` (18), `.wavx` (1) |
| **Parser Crate** | `opts-core` (decoding delegated to an audio crate; see [Implementation Notes](#implementation-notes)) |
| **Parser Module** | `opts_core::formats::wav` |
| **Encoding** | Binary, little-endian |
| **Status** | 🟡 Draft — container usage corpus-verified; playback semantics unverified |
| **Last Updated** | 2026-10-04 |

---

## 📖 Overview

`.wav` is the second most numerous asset type after meshes and textures: **42,375 files, 25.9 GiB, about 49.4 hours of audio**. The files are ordinary Microsoft RIFF/WAVE files; OMSI adds no container of its own. This page documents **which variants occur**, what a tolerant reader must survive, and how the files are referenced. It does not restate the public WAVE format.

Evidence tags: **[C]** computed over all 42,375 files (RIFF chunk walk, headers only), **[H]** seen in individual files, 📚 community wiki (restated, CC BY-SA 4.0), ❓ unverified. No behavioural test in OMSI has been run.

### Where the files live

| Location | Files | Size | Mono | Stereo | 32-bit float |
| :--- | ---: | ---: | ---: | ---: | ---: |
| `Vehicles/<vehicle>/…ound` (engines, doors, brakes, IBIS, …) | 22,446 | 13.4 GB | 5,233 | 17,208 | 752 |
| `TicketPacks/…` (passenger voices) | 11,842 | 4.2 GB | 5,208 | 6,634 | 1,290 |
| `Vehicles/Announcements/<map>/…` | 7,566 | 6.6 GB | 4,069 | 3,497 | 499 |
| `Sceneryobjects/…/Sounds` | 275 | 3.6 GB | 60 | 215 | 12 |
| `Sounds/…` (AI vehicles, ambient) | 246 | 0.04 GB | 135 | 111 | 5 |

[C] (the 5 files that are not RIFF are counted in the file totals but have no channel data.)

### How the files are referenced

- **Vehicles and scenery:** by `[sound]` / `[loopsound]` blocks in sound configuration files ([`sound_cfg`](text/vehicles/sound_cfg.md)). **1,394** such `.cfg` files contain **127,122** references (`[sound]` 96,428 blocks, `[loopsound]` 30,694); **124,511 (98.0 %)** resolve to an existing file relative to the cfg's folder, its `Sound` sub-folder or the vehicle's `Sound` folder; 2,611 do not (most of them scenery sound configs whose files sit elsewhere — not analysed ❓). [C]
- **Case:** **4,597 references (3.7 %) match a file only case-insensitively** (e.g. `Gong.wav` for `gong.WAV`). OMSI runs on Windows; a reader must resolve names case-insensitively. [C]
- **Used vs unused:** 17,143 distinct files are named by sound configs; 16,850 of the 22,446 vehicle files (75 %) are among them. The rest are referenced differently: announcements by stop name from the [`.hof`](text/vehicles/hof.md) (📚 file name = stop name, `_#terminus` suffix for the final stop — ✅ 956 such files), passenger voices by folder from the ticket pack / human definition ([misc types](text/misc-types.md)).
- **Engine samples** are often named by the RPM they were recorded at (`D_0826LUH@460.wav`; `@` appears in 1,312 referenced names); the config's reference value drives playback pitch ([`sound_cfg`](text/vehicles/sound_cfg.md#loopsound--continuous-sound)). The file itself carries no pitch information.
- **Positional sound:** **99.9 % of the resolved references (124,434 of 124,514) are followed by a `[3d]` block** (spatial placement; 80 are not), and **84.8 % of those positional sources are stereo files** (105,473 stereo, 18,954 mono). So stereo samples are normal as 3D sources; an implementation must down-mix them before spatialising ❓ (what OMSI does is untested).

---

## 🔤 General Structure

Standard layout: `RIFF` `<size>` `WAVE`, then chunks `<id:4> <size:u32> <data> [pad byte if size is odd]`. The two chunks every file needs are `fmt ` and `data`.

### Format variants [C]

| Variant | Files |
| :--- | ---: |
| PCM, 16-bit (format tag 1) | 39,662 |
| IEEE float, 32-bit (tag 3) | 2,558 |
| PCM, 24-bit | 96 |
| PCM, 8-bit | 54 |
| **MPEG audio data under a `.wav` name** (no RIFF header; starts `FF F2 40 C0`) | **5** |

- **No `WAVE_FORMAT_EXTENSIBLE` (tag `0xFFFE`)** and no format other than tags 1 and 3. Channels: **1 → 14,705, 2 → 27,665; never more than 2.**
- `fmt ` fields are **internally consistent in every RIFF file**: byte rate = rate × block align and block align = channels × bytes per sample (0 violations).
- Sample rates: 44,100 Hz ×26,644; 48,000 Hz ×11,007; 22,050 Hz ×3,171; 32,000 Hz ×981; 96,000 Hz ×361; 11,025 Hz ×88; 16,000 Hz ×91; 22,255 Hz ×20 (and a few 4,000 / 8,000 / 11,127 / 24,000 Hz). **Mixed rates are the norm; resampling is required.**
- The five MPEG files are all copies of `ibisplus_abfahrt.wav` (an IBIS departure sound in five vehicle packs): **the extension does not identify the codec** (cf. [textures](textures.md#extension-does-not-equal-content)). 18 genuine `.mp3` files also exist; no sound config references an `.mp3`, so MP3 reaches OMSI only through such `.wav` names or other mechanisms ❓.

### Chunks present [C]

| Chunk | Files | Meaning / remark |
| :--- | ---: | :--- |
| `fmt `, `data` | 42,370 | required |
| `LIST` | 17,421 | metadata list (`INFO`); includes software tags |
| `id3 ` | 9,630 | ID3 tag stored in a RIFF chunk |
| `bext` | 2,642 | Broadcast Wave extension |
| `PEAK` | 2,295 | peak-envelope data written by some editors |
| `fact` | 2,301 | sample count (accompanies the float files) |
| `CDif` | 1,211 | unknown ❓ |
| `_PMX` | 1,332 | XMP metadata packet (Adobe tools) |
| `DISP` | 820 | display/title data |
| `cart` | 747 | broadcast cart chunk |
| `iXML` | 647 | production metadata (XML) |
| `JUNK` | 146 | padding |
| `cue ` | 73 | cue points |
| `SAUR` 67, `ResU`/`LGWV` 51 each, `umid`/`minf`/`elm1`/`regn` 22 each, `xtra` 10, `ovwf` 10, `SMED` 13, `smpl` 3, `muma` 3, `dspm` 3, `chrp` 2, `plst` 2, `PADD`/`Fake`/`FLLR`/`nmen` 1 each | | tool-specific; unknown ❓ |

**Loops live in the config, not the file:** only **3** files contain a `smpl` chunk with loop points (`Sceneryobjects/ADDON_Bad_Huegelsdorf/Sounds/sound/Konzert_1.wav` and the two voice files `TicketPacks/Bad_Huegelsdorf/Geist/GoodMorning_1.wav` and `…/Italiener/TooBad_C.wav`, all from one add-on). Looping is decided by `[loopsound]` ([`sound_cfg`](text/vehicles/sound_cfg.md)), so loop metadata can be ignored.

Software tags found in `LIST`/`INFO` (non-normative): GoldWave (1,887), Sound Forge Pro 10/11 (≈ 1,800), Adobe Audition (≈ 350), Adobe Premiere, libsndfile (many), ffmpeg `Lavf` (≈ 240).

### Robustness findings [C]

A reader that assumes a "clean" file will fail on the corpus:

| Finding | Files |
| :--- | ---: |
| First chunk after the header is **not** `fmt ` (`JUNK` 52, `bext` 27) | 79 |
| `data` chunk **declares more bytes than the file contains** (truncated) | 152 |
| RIFF size field ≠ file size − 8 | 168 |
| Chunks with an odd size (pad byte required) | 752 chunks |
| Not RIFF at all (MPEG audio) | 5 |

So: walk the chunk list instead of assuming `fmt ` at offset 12, honour the pad byte, clamp `data` to the bytes actually present, and sniff the first bytes before trusting the extension.

### Durations and sizes [C]

Over the 42,370 RIFF files: total **49.4 h**; median **2.25 s**; 90th percentile 7.1 s; 99th 33.9 s; maximum **531 s** (8.9 min). File size: median 285 KiB; **194 files exceed 10 MB**, **6 exceed 100 MB** (the largest is a 160 MB float stereo announcement). Short effect samples dominate; a few long ambient or announcement files make streaming (not preloading) worthwhile ❓.

---

## 🧪 Examples

Hex of the start of the largest file: `RIFF`, size, `WAVE`, `fmt ` chunk of 16 bytes with format tag `03 00` (float), 2 channels, `44 AC 00 00` (44,100 Hz), 32 bits, then a `fact` chunk:

```
52 49 46 46 .. .. .. ..  57 41 56 45 66 6d 74 20  10 00 00 00 03 00 02 00  44 ac 00 00 20 62 05 00  08 00 20 00 66 61 63 74
```

Resolution of a configured name (descriptive): try `<cfg folder>/<name>`, `<cfg folder>/Sound/<name>`, `<vehicle>/Sound/<name>`; compare case-insensitively; treat a missing file as muted (see the error-handling section of [`sound_cfg`](text/vehicles/sound_cfg.md)).

---

## Implementation Notes

- Use an audio decoder that supports PCM 8/16/24, IEEE float 32 and MPEG audio; detect the codec from the first bytes, not the extension.
- Parse chunks tolerantly (see Robustness findings); ignore `LIST`, `id3 `, `bext`, `iXML`, `_PMX`, `PEAK`, `smpl` and unknown chunks.
- Convert to a common engine sample rate; down-mix stereo to mono for 3D sources.
- Resolve names case-insensitively and cache the result.

---

## ❓ Open Questions

| ID | Question | Priority |
| :--- | :--- | :--- |
| Q-WAV01 | Does OMSI decode MPEG data inside a `.wav` (the five `ibisplus_abfahrt.wav` files), and which formats does it accept beyond PCM 8/16/24 and float 32? | 🟡 Medium |
| Q-WAV02 | How does OMSI spatialise **stereo** `[3d]` sources (down-mix, left channel only, no panning)? | 🟡 Medium |
| Q-WAV03 | Behaviour for truncated `data` chunks (152 files) and for RIFF files whose first chunk is not `fmt ` (79). | 🟢 Low |
| Q-WAV04 | Which folder the 2,611 unresolved scenery sound references expect; are they genuinely missing? | 🟢 Low |
| Q-WAV05 | Does OMSI resample on load, and is there a rate or length limit (the corpus has 4 kHz to 96 kHz and up to 9 minutes)? | 🟢 Low |
| Q-WAV06 | Meaning of the `CDif` chunk (1,211 files) and other unknown tool chunks — believed irrelevant. | 🟢 Low |

---

## 📝 Provenance

- **Corpus:** `/…/steamapps/common/OMSI 2/`, entire tree, scanned 2026-10-04: all 42,375 `.wav` files. A scratch script (outside the repository) read the RIFF chunk headers by seeking past audio data (format, chunk ids, sizes, `smpl` loop count, `LIST/INFO` software tag, truncation and size checks). No audio was decoded and no file was copied.
- **Configuration cross-check:** 1,394 sound `.cfg` files parsed for `[sound]`/`[loopsound]` and following `[3d]` blocks; names resolved against the file index with and without case.
- **Examples inspected:** `Vehicles/Announcements/Ruhrgebiet/STR_Movie Park.wav` (largest), `Vehicles/HB76_MAN_LionsCity/sound/misc/IBIS/ibisplus_abfahrt.wav` (smallest; MPEG data).
- **Community wiki (📚):** announcements page (naming, WAV recommendation). Facts restated; nothing copied.

---

**Clean-room attestation (ADR-0002):**
- No OMSI binaries were inspected.
- No GPL source code was read.
- No OMSI assets were committed.

## 🔗 Related Documents

- [`sound_cfg`](text/vehicles/sound_cfg.md), [`.hof`](text/vehicles/hof.md), [Inventory](inventory.md)
- [Textures — extension ≠ content](textures.md#extension-does-not-equal-content), [Community Cross-Check](community-crosscheck.md)
- [Ticket Packs](text/ticketpacks.md)
