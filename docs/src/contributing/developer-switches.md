# Developer Switches

Environment variables and flags that change behaviour for development, tests and diagnostics. None is needed for normal play. Library crates do not read the environment; the binaries that do are named below.

## Game (`opts-game`)

| Variable | Effect |
| :--- | :--- |
| `OMSI_PATH` | OMSI 2 installation root when `--omsi-path` is not given (before the path remembered in the settings); also used by tests that need real files (they skip without it). |
| `OPTS_DEV_ENTRY` | Entry-point name to start at (with `--autostart`), e.g. `Salthammer 595`. |
| `OPTS_DEV_HOUR` | Hour `0`–`23` the session starts at (with `--autostart`; checks of night lighting). |
| `OPTS_DEV_CAM_HEIGHT` | Metres: start the free camera this high above the spawn, looking down (overview screenshots). |
| `OPTS_HIDE_MESH` | Comma-separated substrings; vehicle meshes whose name matches are not spawned (bisecting rendering problems). |
| `OPTS_REDUCED_MOTION` | `1` or `true` forces *reduced motion* (as if the desktop asked for it; the OS hint is otherwise read once from `gsettings` on Linux/GNOME). |
| `OPTS_SHAFT_TUNE` | Look of the god rays, read once at startup and logged at INFO (`light shaft style`): comma-separated `key=value`, any subset of `haze` (baseline `density_factor` per metre, default 0.0005), `asym` (Henyey-Greenstein g, 0.55), `gain` (factor of `light_intensity`, 0.5), `ambient` (0), `absorption` (0.3), `scatter` (0.3), `moon_gain` (extra gain of moon shafts, 4). Example: `OPTS_SHAFT_TUNE="haze=0.001,asym=0.6,gain=1"`. Unknown keys and bad values are ignored. Derivation: [lighting system](../architecture/lighting-system.md) 6.7. |
| `OPTS_REPORTS_DIR` | Where F1 reports and screenshots are saved (default `~/.local/share/opts/reports`). |

## Game flags

`--pedestrians <n>` caps simultaneously walking pedestrians (`0` = none, default 40). `--traffic <n>` is the ceiling on simultaneously driving AI vehicles (`0` = none, default 100); the count itself comes from the map's density model (lane metres within 320 m × each group's `trafficdensity` rule × its `unsched_trafficdens.txt` curve at the session's hour and day, one vehicle per 80 m; [AI road traffic](../architecture/ai-traffic.md#density-model-2026-10-17)), and all groups are scaled down in proportion when it exceeds the ceiling. Debug lines of the traffic module: `OPTS_LOG_LEVEL=debug` (lane graph rebuilds, spawns).

## Logging (`opts-logging`)

`OPTS_LOG_LEVEL` or `RUST_LOG` (global level), `OPTS_LOG_CONSOLE` (`true`/`false`), `OPTS_LOG_DIR` (log files), `OPTS_LOG_FORMAT` (`text`/`json`). Command-line logging options are in the `opts-game --help` output.

## Control protocol

`--cli-port <n>` starts the control server (loopback only); `--cli-allow-dir <dir>` permits screenshots and reads there. Clients and examples read `OPTS_CLI_PORT` (default 44201) and `OPTS_CLI_TOKEN`. See [control protocol](../architecture/control-protocol.md).

## Tests and examples

| Variable | Effect |
| :--- | :--- |
| `OPTS_X_FIXTURES` | Folder with the `.x`/`.o3d` mutation fixtures (`Sceneryobjects/OPTS_Test/model`); tests skip without fixtures. |
| `OPTS_EXAMPLE_TEXT`, `OPTS_EXAMPLE_SCREENSHOT`, `OPTS_EXAMPLE_MODAL_SHOT` | Example inputs/outputs of the GUI and reporting examples. |
| `OPTS_FONT_COMPARE_OUT`, `OPTS_FONT_COMPARE_SIZE` | Output PNG and base size of `cargo run -p opts-gui --example font_compare` (rendered typeface comparison; reads candidate fonts from system locations, none of them bundled). `cargo run -p opts-gui --example theme_dump` prints the colour tables of the design-system page. |
| `CENSUS_ALL`, `LIMIT`, `FILTER`, `COV_FILTER` | Scope limits of the corpus-census examples. |

Tests that need an installation never hardcode a path: set `OMSI_PATH`.
