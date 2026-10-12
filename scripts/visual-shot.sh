#!/usr/bin/env bash
# One screenshot run for visual A/B comparisons (see docs/src/architecture/visual-ab.md).
#
#   scripts/visual-shot.sh <name> <map-dir-name> <vehicle-rel> [opts-game args...]
#
# Environment: OMSI (installation, default below), OPTS_BIN (opts-game binary), HOUR (default 12),
#   CAM (default ego), DELAY (screenshot delay in seconds, default 12), OUT (default
#   ~/.local/share/opts/benchmark/dev-visual/shots), DECODER_CACHE (a COPY of the descramble cache).
# Never touches the user's settings or caches: a throw-away config/data directory is used.
set -euo pipefail
name=$1; map=$2; veh=$3; shift 3
OMSI=${OMSI:-"/home/louisgoodnews/nvme2n1/steamapps/common/OMSI 2"}
BIN=${OPTS_BIN:-target/debug/opts-game}
OUT=${OUT:-$HOME/.local/share/opts/benchmark/dev-visual/shots}
CACHE=${DECODER_CACHE:-$HOME/.local/share/opts/benchmark/dev-textures/cache}
mkdir -p "$OUT"
cfg=$(mktemp -d); data=$(mktemp -d)
trap 'rm -rf "$cfg" "$data"' EXIT
env XDG_CONFIG_HOME="$cfg" XDG_DATA_HOME="$data" XDG_CACHE_HOME="$CACHE" OPTS_DEV_HOUR="${HOUR:-12}" \
  timeout 300 "$BIN" --omsi-path "$OMSI" --map-path "$OMSI/maps/$map" \
  --vehicle-path "$OMSI/Vehicles/$veh" --autostart --windowed --camera "${CAM:-ego}" \
  --traffic 0 --pedestrians 0 --no-hot-reload \
  --screenshot "$OUT/$name.png" --screenshot-delay "${DELAY:-12}" "$@" > "$OUT/$name.log" 2>&1
echo "$name rc=$?"
