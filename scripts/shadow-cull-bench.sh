#!/usr/bin/env bash
# Setting A/B benchmark (M-0029, docs/src/architecture/lod-culling.md §4): one game run on a real
# map; at each fixed camera pose the setting $SETTING cycles through $VALUES, ROUNDS times, so every
# value sees the same machine load. By default it compares `graphics.shadow_detail` 0 (every part
# casts) with $K (the shadow culling). Per sample: frame times, process CPU time per frame, shadow
# casters, the shadow phase and the GPU time of every pass (OPTS_RENDER_DIAG).
#
#   scripts/shadow-cull-bench.sh <label> <map-dir-name> <entry-substring|-> <vehicle-rel>
#
# Environment: OMSI, OPTS_BIN, OPTS_CLI, OUT (default ~/.local/share/opts/benchmark/shadow-cull/<label>),
#   SETTING (graphics.shadow_detail), VALUES ("0 $K"), K (100), PRE ("key=value ..." set once before
#   the poses: the mirror and quality controls only act with PRE="graphics.light_quality_mode=custom"), ROUNDS (3), HOUR (12), WARM (20), SETTLE (3), SAMPLE (8), PORT (44253), POSES (as in
#   instancing-bench.sh: "name forward_m right_m up_m yaw_offset_deg pitch_deg" per line).
# Results: $OUT/samples.jsonl (one line per pose, round and setting), $OUT/game.log.
set -euo pipefail
label=$1; map=$2; entry=$3; veh=$4
OMSI=${OMSI:-"/home/louisgoodnews/nvme2n1/steamapps/common/OMSI 2"}
BIN=${OPTS_BIN:-target/release/opts-game}
CLI=${OPTS_CLI:-target/release/opts-cli}
OUT=${OUT:-$HOME/.local/share/opts/benchmark/shadow-cull/$label}
K=${K:-100}; ROUNDS=${ROUNDS:-3}; PORT=${PORT:-44253}
SETTING=${SETTING:-graphics.shadow_detail}; VALUES=${VALUES:-"0 $K"}
POSES=${POSES:-"p1_driver 4.4 0.6 2.1 0 0
p2_street 20 0 3.0 0 -6
p3_left 10 -6 3.0 -70 -3
p4_far 40 10 14.0 20 -10
p5_high 0 0 60.0 0 -35"}
mkdir -p "$OUT"; : > "$OUT/samples.jsonl"
cfg=$(mktemp -d -p "$OUT"); data=$(mktemp -d -p "$OUT")
cli() { "$CLI" --port "$PORT" --json --quiet "$@"; }
cleanup() { cli shutdown >/dev/null 2>&1 || true; kill "$pid" 2>/dev/null || true; rm -rf "$cfg" "$data"; }
envs=(XDG_CONFIG_HOME="$cfg" XDG_DATA_HOME="$data" OPTS_DEV_HOUR="${HOUR:-12}" OPTS_RENDER_DIAG=1)
[ "$entry" != "-" ] && envs+=(OPTS_DEV_ENTRY="$entry")
env "${envs[@]}" "$BIN" --omsi-path "$OMSI" --map-path "$OMSI/maps/$map" --vehicle-path "$OMSI/Vehicles/$veh" \
  --autostart --windowed --camera ego --traffic 0 --pedestrians 0 --no-hot-reload \
  --cli-port "$PORT" > "$OUT/game.log" 2>&1 &
pid=$!
trap cleanup EXIT
ready=0
for _ in $(seq 1 3600); do
  if cli state 2>/dev/null | jq -e '.result.loading.ready == true' >/dev/null 2>&1; then ready=1; break; fi
  sleep 0.25
done
[ "$ready" = 1 ] || { echo "session did not start" >&2; exit 1; }
sleep "${WARM:-20}"
cli sim.pause >/dev/null
same=0; last=-1
for _ in $(seq 1 40); do
  n=$(cli render.stats --parts true | jq -r '.result.parts.mesh_entities')
  if [ "$n" = "$last" ]; then same=$((same+1)); else same=0; fi
  last=$n; [ "$same" -ge 3 ] && break
  sleep 5
done
for kv in ${PRE:-}; do cli settings.set --key "${kv%%=*}" --value "${kv#*=}" >/dev/null; done
v=$(cli vehicle.get)
x=$(jq -r '.result.x' <<<"$v"); y=$(jq -r '.result.y' <<<"$v"); z=$(jq -r '.result.z' <<<"$v")
h=$(jq -r '.result.heading_deg' <<<"$v")
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
  for round in $(seq 1 "$ROUNDS"); do
    for k in $VALUES; do
      cli settings.set --key "$SETTING" --value "$k" >/dev/null
      sleep "${SETTLE:-3}"
      cli render.stats --reset true >/dev/null
      cpu0=$(awk '{print $14+$15}' "/proc/$pid/stat")
      sleep "${SAMPLE:-8}"
      stats=$(cli render.stats --parts true | jq -c '.result')
      cpu1=$(awk '{print $14+$15}' "/proc/$pid/stat")
      jq -c --arg pose "$pose" --argjson round "$round" --argjson k "$k" --argjson c "$((cpu1-cpu0))" \
        --arg load "$(cut -d' ' -f1 /proc/loadavg)" \
        '{pose: $pose, round: $round, k: $k, loadavg: $load, frames: .frame_times,
          cpu_ms_per_frame: (if .frame_times.frames > 0 then ($c * 10 / .frame_times.frames) else null end),
          shadow_casters: .parts.shadow_casters, mesh_entities: .parts.mesh_entities,
          active_cameras: ([.parts.cameras[]? | select(.active)] | length),
          shadow_phase: .phases.shadow,
          gpu_shadow_ms: ([.gpu_diagnostics_ms | to_entries[] | select(.key | test("shadow.*gpu")) | .value] | add),
          gpu: .gpu_diagnostics_ms}' <<<"$stats" >> "$OUT/samples.jsonl"
    done
  done
  echo "$label $pose done"
done <<<"$POSES"
