#!/usr/bin/env bash
# Instancing prototype benchmark (M-0082): load time, entities, draw counts, frame times on fixed
# camera poses, plus one screenshot per pose, through the control protocol.
#
#   scripts/instancing-bench.sh <label> <mode> <map-dir-name> <entry-substring|-> <vehicle-rel>
#
# <mode> is the value of OPTS_INSTANCING (off | dedupe | instance). The game runs with throw-away
# settings (fresh XDG dirs), a COPY of the decoder cache, no traffic or pedestrians, the simulation
# paused after the warm-up and the control server on PORT (44252, not the default 44201). Every pose
# is shown for SAMPLE seconds after a settle time; `render.stats` is read per pose (frame times of
# the window, draw counts of the last frame) and a screenshot is written.
#
# Environment: OMSI, OPTS_BIN, OPTS_CLI, OUT (default ~/.local/share/opts/benchmark/instancing/<label>),
#   DECODER_CACHE (a copy of ~/.cache/opts/descramble's parent), HOUR (12), WARM (20), SETTLE (4),
#   SAMPLE (10), PORT, POSES (lines "name forward_m right_m up_m yaw_offset_deg pitch_deg").
# Results: $OUT/result.json (one object), $OUT/<pose>.png, $OUT/game.log.
set -euo pipefail
label=$1; mode=$2; map=$3; entry=$4; veh=$5
OMSI=${OMSI:-"/home/louisgoodnews/nvme2n1/steamapps/common/OMSI 2"}
BIN=${OPTS_BIN:-target/release/opts-game}
CLI=${OPTS_CLI:-target/release/opts-cli}
OUT=${OUT:-$HOME/.local/share/opts/benchmark/instancing/$label}
CACHE=${DECODER_CACHE:?set DECODER_CACHE to a directory holding a copy of opts/descramble}
PORT=${PORT:-44252}
POSES=${POSES:-"p1_driver 4.4 0.6 2.1 0 0
p2_street 20 0 3.0 0 -6
p3_left 10 -6 3.0 -70 -3
p4_far 40 10 14.0 20 -10
p5_high 0 0 60.0 0 -35
p6_back -30 5 6.0 180 -5"}
mkdir -p "$OUT"
cfg=$(mktemp -d -p "$HOME/.local/share/opts/benchmark/instancing"); data=$(mktemp -d -p "$HOME/.local/share/opts/benchmark/instancing")
cli() { "$CLI" --port "$PORT" --json --quiet "$@"; }
cleanup() { cli shutdown >/dev/null 2>&1 || true; kill "$pid" 2>/dev/null || true; rm -rf "$cfg" "$data"; }
envs=(XDG_CONFIG_HOME="$cfg" XDG_DATA_HOME="$data" XDG_CACHE_HOME="$CACHE" OPTS_DEV_HOUR="${HOUR:-12}" OPTS_INSTANCING="$mode" OPTS_RENDER_DIAG=1)
[ "$entry" != "-" ] && envs+=(OPTS_DEV_ENTRY="$entry")
load0=$(cut -d" " -f1-3 /proc/loadavg)
t0=$(date +%s.%N)
env "${envs[@]}" "$BIN" --omsi-path "$OMSI" --map-path "$OMSI/maps/$map" --vehicle-path "$OMSI/Vehicles/$veh" \
  --autostart --windowed --camera ego --traffic 0 --pedestrians 0 --no-hot-reload \
  --cli-port "$PORT" --cli-allow-dir "$OUT" > "$OUT/game.log" 2>&1 &
pid=$!
trap cleanup EXIT
ready=0
for _ in $(seq 1 1200); do
  if cli state 2>/dev/null | jq -e '.result.loading.ready == true' >/dev/null 2>&1; then ready=1; break; fi
  sleep 0.25
done
t1=$(date +%s.%N)
[ "$ready" = 1 ] || { echo "session did not start" >&2; exit 1; }
load_s=$(python3 -I -c "print(round($t1-$t0,2))")
sleep "${WARM:-20}"
cli sim.pause >/dev/null
v=$(cli vehicle.get)
x=$(jq -r '.result.x' <<<"$v"); y=$(jq -r '.result.y' <<<"$v"); z=$(jq -r '.result.z' <<<"$v")
h=$(jq -r '.result.heading_deg' <<<"$v")
poses_json="[]"
while read -r pose f r u yo p; do
  read -r px py pz yaw < <(python3 -I - "$x" "$y" "$z" "$h" "$f" "$r" "$u" "$yo" <<'PY'
import math, sys
x, y, z, h, f, r, u, yo = map(float, sys.argv[1:9])
a = math.radians(h)
fx, fy = math.sin(a), math.cos(a)
rx, ry = fy, -fx
print(x + f * fx + r * rx, y + f * fy + r * ry, z + u, (h + yo) % 360.0)
PY
)
  cli camera.teleport "$px" "$py" "$pz" --yaw "$yaw" --pitch "$p" >/dev/null
  sleep "${SETTLE:-4}"
  cli render.stats --reset true >/dev/null
  sleep "${SAMPLE:-10}"
  stats=$(cli render.stats --parts true | jq -c '.result')
  cli screenshot --path "$OUT/$pose.png" >/dev/null
  poses_json=$(jq -c --arg n "$pose" --argjson s "$stats" '. + [{pose: $n, stats: $s}]' <<<"$poses_json")
  echo "$label $pose done"
done <<<"$POSES"
load1=$(cut -d" " -f1-3 /proc/loadavg)
scene=$(cli scene | jq -c '.result')
jq -n --arg label "$label" --arg mode "$mode" --arg map "$map" --argjson load "$load_s" \
  --argjson scene "$scene" --argjson poses "$poses_json" --arg l0 "$load0" --arg l1 "$load1" \
  '{label: $label, mode: $mode, map: $map, load_s: $load, loadavg_start: $l0, loadavg_end: $l1, scene: $scene, poses: $poses}' > "$OUT/result.json"
