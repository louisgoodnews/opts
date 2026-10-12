# opts-cli

Command-line pieces of OPTS:

* the arguments of the game binary (`CliArgs`, shared with `opts-game`),
* the JSON-RPC types of the **control protocol**, and
* the **`opts-cli` client** that drives a running game from a terminal, a script or an AI agent.

## Start the game with the control server

The server is off unless you ask for it, and listens on loopback only:

```text
cargo run --release -p opts-game -- --map-path "/path/to/OMSI 2/maps/BRT Berlin" --autostart --cli-port 44201
```

`--cli-port 0` picks a free port and logs it (`control server listening addr=…`). Other flags:
`--cli-token <T>` (required to bind anything but loopback), `--cli-allow-dir <DIR>` (extra directory for
reads and writes), `--no-hot-reload`, `--hot-reload-root <DIR>`, `--hot-reload-debounce-ms <MS>`.

## A quick tour

```text
opts-cli ping
opts-cli state                         # app state, camera mode, fps, map, vehicle
opts-cli scene                         # tiles, entity and asset counts, loader statistics
opts-cli camera.set_mode free          # ego | cockpit | external | pax | free  (or 1-5)
opts-cli camera.teleport 3910 3580 5 --yaw 90 --pitch -10
opts-cli render.look                   # read the colour look (vivid | natural | bevy)
opts-cli render.look natural           # switch it live (also saved in the settings)
opts-cli input.key W --duration 2000   # press W for two seconds
opts-cli sim.pause && opts-cli sim.step 60
opts-cli screenshot                    # saved in the reports directory
opts-cli report "the junction has no road surface"
opts-cli log.filter --level warn --n 10
opts-cli reload.asset "/path/to/object.sco"
opts-cli scene.load_map "/path/to/OMSI 2/maps/X10 Berlin"
echo '[{"method":"sim.pause"},{"method":"state"}]' | opts-cli batch
opts-cli repl                          # interactive, Tab completion, history
opts-cli list                          # every command with a one-line description
```

Global flags go before the command: `--host`, `--port` (default 44201), `--token`, `--json`, `--quiet`,
`--timeout-ms`. Exit codes: 0 success, 1 command error, 2 no connection, 3 protocol error.

## Examples (as code)

`crates/opts-cli/examples/` — `drive_and_screenshot`, `reload_iterate`, `scene_probe`. Run with
`cargo run -p opts-cli --example scene_probe` (set `OPTS_CLI_PORT` / `OPTS_CLI_TOKEN` if needed).

## More

* Protocol specification: [`docs/src/architecture/control-protocol.md`](../../docs/src/architecture/control-protocol.md)
* Hot reload: [`docs/src/architecture/hot-reload.md`](../../docs/src/architecture/hot-reload.md)
* Crate manual: [`docs/crates/opts-cli.md`](../../docs/crates/opts-cli.md)
