#!/usr/bin/env bash
# A shell session the way an AI agent would drive OPTS through the control protocol.
#
#   opts-game --cli-port 44201 &              # start the game first
#   MAP="/path/to/OMSI 2/maps/BRT Berlin" VEHICLE="/path/to/OMSI 2/Vehicles/MAN_SD200/MAN_SD77.bus" \
#       crates/opts-cli/examples/agent_session.sh
#
# Needs `opts-cli` (cargo build --release -p opts-cli) on PATH or in ./target/release, and `jq`.
set -euo pipefail
CLI="${OPTS_CLI:-$(command -v opts-cli || echo ./target/release/opts-cli)}"
: "${MAP:?set MAP to a map folder}"

$CLI ping > /dev/null || { echo "no OPTS control server on port ${OPTS_CLI_PORT:-44201}" >&2; exit 2; }
$CLI --timeout-ms 150000 scene.load_map "$MAP"
if [ -n "${VEHICLE:-}" ]; then $CLI --timeout-ms 150000 vehicle.set "$VEHICLE"; fi
$CLI camera.set_mode external
$CLI sim.resume
$CLI input.key W --duration 5000
$CLI sim.pause
SHOT="$($CLI --json screenshot | jq -r '.result.path')"
echo "screenshot: $SHOT"
$CLI --json scene | jq '.result | {map, tiles_loaded, entity_count}'
$CLI --json log.filter --level warn --n 5 | jq -r '.result.lines[] | "\(.level) \(.target) \(.message)"'
