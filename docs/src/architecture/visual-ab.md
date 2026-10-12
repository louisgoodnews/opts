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

`--crop` leaves out the HUD (for a 1920×1080 screenshot, `0,0,1920,900` cuts the bottom strip). The absolute Laplacian values differ from the ImageMagick numbers of the [proposal](texture-quality.md) (other kernel and scale); **compare ratios only**.

## 2. One screenshot

`scripts/visual-shot.sh <name> <map> <vehicle> [opts-game args…]` starts the game with a throw-away configuration (`XDG_CONFIG_HOME`/`XDG_DATA_HOME` are fresh temporary directories, so the user's settings are untouched), a **copy** of the decoder cache, no traffic and no hot reload, waits `DELAY` seconds and writes `<name>.png` and `<name>.log` below `OUT`.

```sh
cargo build -p opts-game -p opts-tools
export OPTS_BIN=$PWD/target/debug/opts-game
# the cache directory must be a COPY: cp -r ~/.cache/opts/descramble <dir>/opts/descramble
export DECODER_CACHE=<dir>
HOUR=12 scripts/visual-shot.sh bhd_noon_vivid Bad_Huegelsdorf_2020 MAN_SD200/MAN_SD77.bus
HOUR=12 scripts/visual-shot.sh bhd_noon_bevy  Bad_Huegelsdorf_2020 MAN_SD200/MAN_SD77.bus --look bevy
HOUR=12 scripts/visual-shot.sh bhd_noon_nofilter Bad_Huegelsdorf_2020 MAN_SD200/MAN_SD77.bus --texture-filter off
```

`--look vivid|natural|bevy` and `--texture-filter off|4|8|16` override the stored settings **for this run only** (they are not persisted). The vehicle path is relative to `Vehicles/` of the OMSI installation; `HOUR`, `CAM`, `DELAY`, `OMSI` and `OUT` are described at the top of the script.

## 3. Reference views

Fixed view for the first stage (more are added with [M-0057](../roadmap/phase-8/M-0057.md) phase A):

| View | Map | Vehicle | Camera | Hour |
| :--- | :--- | :--- | :--- | :--- |
| Driver's view, noon | `Bad_Huegelsdorf_2020` | `MAN_SD200/MAN_SD77.bus` | `ego` | 12 |
| Driver's view, night | same | same | `ego` | 21 |
| Open view | `Grundorf` | any | `external` | 12 |

For other poses use the control protocol (`camera.teleport`, `screenshot`) on a running game with a throw-away configuration.

## 4. Comparing

```sh
opts-tools image-metrics --crop 0,0,1920,900 a.png b.png
```

Run the same view twice on the same build: the numbers must agree within 1 % (the harness reproduces itself). Then compare before and after, and **look at both images**: metrics do not see shimmer in motion, soft text or a blown-out sky.
