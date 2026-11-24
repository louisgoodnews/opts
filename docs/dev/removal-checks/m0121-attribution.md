# M-0121: on-screen attribution of the snow refresh (commit 6395d448)

| Field | Value |
| :--- | :--- |
| **Milestone** | [M-0121](../../src/roadmap/phase-12/M-0121.md) (ISS-016 snow load order) |
| **Change under test** | `feat(opts-bevy): scenery textures follow the snow cover on a change` (6395d448) |
| **Date** | 2026-11-24 |
| **Branch / base** | `agent/ingame-checks` from master 95a8a588 |

## Method

* **Arm A (refresh active):** `opts-game` built from master 95a8a588 unchanged. Binary copied to `opts-game-A`.
* **Arm B (refresh removed):** the one registration line removed from `crates/opts-game/src/plugin.rs` (line 80, `.add_plugins(opts_bevy::model::SeasonTexturePlugin)`), built, binary copied to `opts-game-B`. The source was then restored from a copy and checked (below).
* Both arms: `scene.load_map` Berlin-Spandau, vehicle `MAN_NL_NG/MAN_EN92_main.bus`, 2026-07-15 12:00 (summer; screenshots taken), then `sim.set_time` 2026-01-15 12:00 with weather `Starker Schneefall` (winter; cover flips), 20 s wait, screenshots of the same two poses. Poses relative to the vehicle as in `scripts/visual-views/m0122-reference.poses` style: `far` (6 m forward, 9 m up, pitch -4) and `road` (12 m forward, 0.9 m up, pitch -4).
* Game run under `game.lock` (rustrim), one session per arm, display :0.

## Restore check (plugin.rs)

| Step | Result |
| :--- | :--- |
| sha256 before edit | `ef9bb82d8a080122de5f290427faabe7215e982fb97c58e256ea32e459b5ca2e` |
| sha256 after restore | `ef9bb82d8a080122de5f290427faabe7215e982fb97c58e256ea32e459b5ca2e` |
| `cmp` against the saved copy | identical |
| `cmp` against `git show HEAD:crates/opts-game/src/plugin.rs` | identical |
| `git status` after restore | clean |

Binaries differ (`cmp`: differ at byte 25):

* `opts-game-A` sha256 `9d8d2348bd3031189ae18947d02850dc9557dd0352c0a3b6488a735411193557`
* `opts-game-B` sha256 `5f94f3bbb15363d26f1a4f8da8132c5641a0dead9947404b86bf1537f3d2147c`

## Runs

| Arm | Session | Lock held | Screenshots (scratch, `ingame/`) |
| :--- | :--- | :--- | :--- |
| A | m2a | 17:55:36 to 17:57:02 | `m2a-A-summer-far.png`, `m2a-A-summer-road.png`, `m2a-A-winter-far.png`, `m2a-A-winter-road.png` |
| B | m2b | 17:57:04 to 17:58:46 | `m2b-B-summer-far.png`, `m2b-B-summer-road.png`, `m2b-B-winter-far.png`, `m2b-B-winter-road.png` |

## Numbers (ImageMagick MAE, 0 to 65535 scale, normalised in brackets)

| Pair | far | road |
| :--- | :--- | :--- |
| Summer, A vs B (noise floor: no change expected) | 493 (0.0075) | 641 (0.0098) |
| Winter, A vs B | 3312 (0.0505) | 4513 (0.0689) |
| A, summer vs winter | 6950 (0.106) | 13393 (0.204) |
| B, summer vs winter | 7887 (0.120) | 16263 (0.248) |

The summer pair is the noise floor of the method. The winter pair is about six to seven times larger, and it appears only after the cover flip.

## What changed and what did not (by eye, winter views)

Changed between A and B:

* Tree crowns and bushes: snow-covered and whitened in A; plain green in B (the far view's left bushes and the road view's birch and crowns).
* Pavement and sidewalk: white snow cover in A; grey paving in B (road view, sidewalk along the parking).
* Road surface in the far view: lighter and snowier in A than in B.

Not changed (same in A and B by eye): buildings, parked cars and vans, signs and the bus stop, the sky, the far-field skyline.

Reading: the changed items are scenery and road textures that carry a `WinterSnow` variant, which is what the refresh swaps on a cover flip. The unchanged items are not texture-swapped by the refresh. This is one run per arm, so the attribution rests on the size of the difference against the summer noise floor, not on repeated runs.

## Not covered

* Derived copies (`opaque_variant`, composed transmap) are not refreshed by 6395d448 (its own commit message); a remaining snow state on those materials is not attributed here.
* No measurement of the cover ramp time; the wait was a fixed 20 s.
