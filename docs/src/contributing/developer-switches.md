# Developer Switches

Environment variables and flags that change behaviour for development, tests and diagnostics. None is needed for normal play. Library crates do not read the environment; the binaries that do are named below.

## Game (`opts-game`)

| Variable | Effect |
| :--- | :--- |
| `OMSI_PATH` | OMSI 2 installation root when `--omsi-path` is not given; also used by tests that need real files (they skip without it). |
| `OPTS_DEV_ENTRY` | Entry-point name to start at (with `--autostart`), e.g. `Salthammer 595`. |
| `OPTS_DEV_CAM_HEIGHT` | Metres: start the free camera this high above the spawn, looking down (overview screenshots). |
| `OPTS_HIDE_MESH` | Comma-separated substrings; vehicle meshes whose name matches are not spawned (bisecting rendering problems). |
| `OPTS_NO_DESCRAMBLE` | Disables the scrambled-mesh decoder (meshes without a plain twin are skipped). |
| `OPTS_DECODE_SCRAMBLED` | Experimental: runs the older heuristic descrambler instead of the confident one (`o3d.md`, Addendum 4). |
| `OPTS_REPORTS_DIR` | Where F1 reports and screenshots are saved (default `~/.local/share/opts/reports`). |
| `XDG_CACHE_HOME` | Base of the descrambler cache (`<base>/opts/descramble`, capped at 256 MiB). |

## Decoder tuning (read by the game and by `confident_eval`; see `o3d.md`, Addendum 6)

`BP_W`, `BP_T`, `BP_I` (belief propagation weight, truncation, sweeps), `ANNEAL_T0`, `ANNEAL_R` (annealing start temperature, rounds), `OPTS_DESCRAMBLE_RESTARTS` (restarts of the plain solver), `OPTS_CONF_AGREE` (agreement two runs need, default `0.98`). They are applied through `DecodeParams::with_overrides`; every distinct set has its own cache entries.

## Logging (`opts-logging`)

`OPTS_LOG_LEVEL` or `RUST_LOG` (global level), `OPTS_LOG_CONSOLE` (`true`/`false`), `OPTS_LOG_DIR` (log files), `OPTS_LOG_FORMAT` (`text`/`json`). Command-line logging options are in the `opts-game --help` output.

## Control protocol

`--cli-port <n>` starts the control server (loopback only); `--cli-allow-dir <dir>` permits screenshots and reads there. Clients and examples read `OPTS_CLI_PORT` (default 44201) and `OPTS_CLI_TOKEN`. See [control protocol](../architecture/control-protocol.md).

## Tests and examples

| Variable | Effect |
| :--- | :--- |
| `OPTS_X_FIXTURES` | Folder with the `.x`/`.o3d` mutation fixtures (`Sceneryobjects/OPTS_Test/model`); tests skip without fixtures. |
| `OPTS_EXAMPLE_TEXT`, `OPTS_EXAMPLE_SCREENSHOT`, `OPTS_EXAMPLE_MODAL_SHOT` | Example inputs/outputs of the GUI and reporting examples. |
| `CENSUS_ALL`, `LIMIT`, `FILTER`, `COV_FILTER` | Scope limits of the corpus-census examples. |

Tests that need an installation never hardcode a path: set `OMSI_PATH`.
