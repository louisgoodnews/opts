# Visual A/B Procedure and Image Metrics

| Metadata | Value |
| :--- | :--- |
| **Status** | 🟡 In use since 2026-10-12 ([M-0057](../roadmap/phase-8/M-0057.md) phase A, minimal) |
| **Tools** | `opts-tools image-metrics`, `scripts/visual-shot.sh` |
| **Results** | outside the repository: `~/.local/share/opts/benchmark/dev-visual/` (screenshots show OMSI content and must never be committed) |

Visual changes of [Phase 8](../roadmap/phase-8/README.md) (filtering, mipmaps, looks) are judged by **numbers and by looking**. The numbers come from the same fixed views before and after a change.

## 1. `opts-tools image-metrics`

```text
opts-tools image-metrics [--crop x,y,w,h] [--json] <a.png> [<b.png>]
```

All values use the sRGB-encoded pixel values (`0..1`), as in a screenshot:

| Metric | Meaning |
| :--- | :--- |
| `saturation` | mean HSL saturation |
| `luma mean / std / min` | Rec. 709 luma: brightness, **contrast** (standard deviation), darkest pixel |
| `laplacian` | standard deviation of the 4-neighbour Laplacian of the luma: **high-frequency energy** (real detail *and* aliasing noise; after filtering a lower value means less noise) |
| `difference` | for two images of equal size: mean absolute error, RMSE, PSNR |

`--crop` leaves out the HUD (the screenshots of `visual-ab.sh` are 1920×1013; `0,0,1920,860` cuts the bottom HUD strip). The absolute Laplacian values differ from the ImageMagick numbers of the [proposal](texture-quality.md) (other kernel and scale); **compare ratios only**.

## 2. Deterministic screenshots: `scripts/visual-ab.sh`

`scripts/visual-ab.sh <config> <map> <vehicle> [opts-game args…]` starts the game with a throw-away configuration (`XDG_CONFIG_HOME`/`XDG_DATA_HOME` are fresh temporary directories, so the user's settings are untouched), a **copy** of the decoder cache, no traffic and no hot reload, and the control server on port 44251 (not the default 44201, so a game you are running is not disturbed). It waits for the session, **pauses the simulation**, flies the free camera to fixed poses relative to the player vehicle (`driver`, `road_grazing`, `foliage`, `far`, `sign_close`; override with `POSES`) and writes `<OUT>/<config>_<pose>.png`. Identical poses on a paused simulation give identical frames, so differences are caused by the setting under test only (the plain `--screenshot` flag is *not* deterministic: the ego camera sometimes starts inside the cockpit).

With `LOOKS="vivid natural bevy"` every pose is shot once per look **inside one run** (`render.look` is switched live), which compares looks on exactly the same frame:

```sh
cargo build -p opts-game -p opts-cli -p opts-tools
export OPTS_BIN=$PWD/target/debug/opts-game OPTS_CLI=$PWD/target/debug/opts-cli
# the cache directory must be a COPY: cp -r ~/.cache/opts/descramble <dir>/opts/descramble
export DECODER_CACHE=<dir>
HOUR=12 LOOKS="vivid natural bevy" scripts/visual-ab.sh bhd12_f16 Bad_Huegelsdorf_2020 MAN_SD200/MAN_SD77.bus --texture-filter 16
HOUR=12 LOOKS="bevy"               scripts/visual-ab.sh bhd12_off Bad_Huegelsdorf_2020 MAN_SD200/MAN_SD77.bus --texture-filter off
HOUR=21 LOOKS="vivid bevy"         scripts/visual-ab.sh bhd21_f16 Bad_Huegelsdorf_2020 MAN_SD200/MAN_SD77.bus --texture-filter 16
```

Filtering has no live switch, so it needs one run per setting (`bhd12_f16_bevy_*` against `bhd12_off_bevy_*`). `--look vivid|natural|bevy` and `--texture-filter off|4|8|16` override the stored settings **for that run only** (not persisted). The vehicle path is relative to `Vehicles/` of the OMSI installation; `HOUR`, `WARM`, `OMSI`, `OUT`, `PORT` are described at the top of the script. `scripts/visual-shot.sh` is the simpler variant with the built-in `--screenshot` (one frame per run, no pose control).

## 3. Reference views

The poses are relative to the vehicle, so the same set works on every map. The first stage uses:

| View | Map | Vehicle | Hour |
| :--- | :--- | :--- | :--- |
| Driver's view, noon | `Bad_Huegelsdorf_2020` | `MAN_SD200/MAN_SD77.bus` | 12 |
| Driver's view, dusk (19) and night (21) | same | same | 19, 21 |
| Open field and trees | `Grundorf` | same | 12 |

The sweep over 6-8 views on three maps with a data file (M-0057 phase A) is still open.

## 4. Comparing

```sh
opts-tools image-metrics --crop 0,0,1920,860 a.png b.png
```

Run the same view twice on the same build: the numbers must agree within 1 % (the harness reproduces itself). Then compare before and after, and **look at both images**: metrics do not see shimmer in motion, soft text or a blown-out sky.
