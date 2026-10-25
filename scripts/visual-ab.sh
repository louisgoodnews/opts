#!/usr/bin/env bash
# Deterministic screenshots for visual A/B comparisons through the control protocol.
#
#   scripts/visual-ab.sh <config-name> <map-dir-name> <vehicle-rel> [opts-game args...]
#
# Starts the game (throw-away settings, a COPY of the decoder cache, no traffic), waits until the
# session is up, freezes the simulation, flies the free camera to a fixed set of poses relative
# to the player vehicle, writes <OUT>/<config-name>_<pose>.png for each, then shuts the game down.
# The look can also be switched inside one run: set LOOKS="vivid natural bevy" to take every pose
# once per look (files <config-name>_<look>_<pose>.png), which compares looks on identical frames.
#
# Environment: OMSI, OPTS_BIN (opts-game), OPTS_CLI (opts-cli), OUT, DECODER_CACHE, HOUR (12),
#   PORT (44251; not the default 44201, so a running game is not disturbed), WARM (seconds to let
#   the streaming settle after the HUD is up, 12), LOOKS.
# Poses: "name forward_m right_m up_m yaw_offset_deg pitch_deg" relative to the vehicle's heading.
set -euo pipefail
name=$1; map=$2; veh=$3; shift 3
OMSI=${OMSI:-"/home/louisgoodnews/nvme2n1/steamapps/common/OMSI 2"}
BIN=${OPTS_BIN:-target/debug/opts-game}
CLI=${OPTS_CLI:-target/debug/opts-cli}
OUT=${OUT:-$HOME/.local/share/opts/benchmark/dev-visual/shots}
CACHE=${DECODER_CACHE:-$HOME/.local/share/opts/benchmark/dev-textures/cache}
PORT=${PORT:-44251}
POSES=${POSES:-"driver 4.4 0.6 2.1 0 0
road_grazing 12 0 0.9 0 -4
foliage 6 -4 2.0 -55 3
far 6 0 9.0 0 -4
sign_close 14 3.5 1.6 25 0"}
mkdir -p "$OUT"
cfg=$(mktemp -d); data=$(mktemp -d)
cli() { "$CLI" --port "$PORT" --json --quiet "$@"; }
cleanup() { cli shutdown >/dev/null 2>&1 || true; kill "$pid" 2>/dev/null || true; rm -rf "$cfg" "$data"; }
env XDG_CONFIG_HOME="$cfg" XDG_DATA_HOME="$data" XDG_CACHE_HOME="$CACHE" OPTS_DEV_HOUR="${HOUR:-12}" \
  "$BIN" --omsi-path "$OMSI" --map-path "$OMSI/maps/$map" --vehicle-path "$OMSI/Vehicles/$veh" \
  --autostart --windowed --camera ego --traffic 0 --pedestrians 0 --no-hot-reload \
  --cli-port "$PORT" --cli-allow-dir "$OUT" "$@" > "$OUT/$name.log" 2>&1 &
pid=$!
trap cleanup EXIT
for _ in $(seq 1 240); do
  if cli state 2>/dev/null | jq -e '.result.loading.ready == true' >/dev/null 2>&1; then break; fi
  sleep 1
done
sleep "${WARM:-12}"
cli sim.pause >/dev/null
v=$(cli vehicle.get)
x=$(jq -r '.result.x' <<<"$v"); y=$(jq -r '.result.y' <<<"$v"); z=$(jq -r '.result.z' <<<"$v")
h=$(jq -r '.result.heading_deg' <<<"$v")
shoot() { # <file-stem> <fwd> <right> <up> <yaw_off> <pitch>
  read -r px py pz yaw < <(python3 -I - "$x" "$y" "$z" "$h" "$2" "$3" "$4" "$5" <<'PY'
import math, sys
x, y, z, h, f, r, u, yo = map(float, sys.argv[1:9])
a = math.radians(h)               # compass heading: 0 = north (+Y), clockwise (90 = east)
fx, fy = math.sin(a), math.cos(a)  # forward
rx, ry = fy, -fx                   # right
print(x + f * fx + r * rx, y + f * fy + r * ry, z + u, (h + yo) % 360.0)
PY
)
  cli camera.teleport "$px" "$py" "$pz" --yaw "$yaw" --pitch "$6" >/dev/null
  sleep 2
  # The game writes captures only below its own captures directory (here in the throw-away
  # data dir); copy the frame out before cleanup removes it.
  cli screenshot --name "$1.png" >/dev/null
  for _ in $(seq 1 20); do [ -s "$data/opts/captures/$1.png" ] && break; sleep 0.5; done
  cp "$data/opts/captures/$1.png" "$OUT/$1.png"
  echo "$1"
}
for look in ${LOOKS:-""}; do
  [ -n "$look" ] && cli render.look "$look" >/dev/null && sleep 1
  while read -r pose f r u yo p; do
    stem="${name}${look:+_$look}_$pose"
    shoot "$stem" "$f" "$r" "$u" "$yo" "$p"
  done <<<"$POSES"
done
