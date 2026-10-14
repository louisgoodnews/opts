#!/usr/bin/env bash
# Simulates an AI-agent workflow against a running OPTS (start it with `--cli-port <port>`):
#   1. start a 5 s recording        2. drive with key presses       3. wait for the recording
#   4. burst of 10 frames 200 ms apart       5. one final screenshot
# Then checks that every output is usable by a downstream tool: the video probes as AV1 with the
# frame count of its sidecar, every JSON file parses, every burst frame decodes.
#
#   scripts/capture-agent-demo.sh [port]
#
# Environment: OPTS_CLI (path of the opts-cli binary, default `opts-cli`), KEYS (space-separated
# keys to press while recording, default `W`; use `Tab` to exercise a menu without driving),
# KEY_MS (duration of each key press, default 700).
set -euo pipefail
port=${1:-44201}
cli=${OPTS_CLI:-opts-cli}
keys=${KEYS:-W}
key_ms=${KEY_MS:-700}
c() { "$cli" --port "$port" "$@"; }
field() { awk -v k="$1:" '$1==k {print $2; exit}'; }
run=agent-demo-$(date +%H%M%S)

echo "1. recording 5 s"
rec=$(c capture record start --name "$run/rec" --duration-ms 5000 --fps 30)
id=$(echo "$rec" | field recording_id)
video=$(echo "$rec" | field path)

echo "2. driving: $keys"
for k in $keys; do c input.key "$k" --duration "$key_ms" > /dev/null; sleep 0.3; done

echo "3. waiting for the recording to finish"
for _ in $(seq 1 120); do
  state=$(c capture record status --id "$id" | field state)
  [ "$state" = finished ] || [ "$state" = failed ] && break
  sleep 0.5
done
status=$(c capture record status --id "$id")
echo "   state=$state frames=$(echo "$status" | field frame_count) dropped=$(echo "$status" | field dropped) stopped_by=$(echo "$status" | field stopped_by)"
[ "$state" = finished ] || { echo "recording did not finish"; exit 1; }

echo "4. burst of 10 frames"
burst=$(c capture burst 10 200 --dir "$run/burst")
dir=$(echo "$burst" | field dir)

echo "5. final screenshot"
shot=$(c screenshot --name "$run/final.png")
png=$(echo "$shot" | field path)

echo "verifying outputs"
python3 - "$video" "$dir" "$png" <<'PY'
import json, subprocess, sys, pathlib
video, burst_dir, png = sys.argv[1:4]
side = json.load(open(video + ".json"))
probe = subprocess.run(["ffprobe", "-v", "error", "-count_packets", "-select_streams", "v:0",
                        "-show_entries", "stream=codec_name,nb_read_packets", "-of", "csv=p=0", video],
                       capture_output=True, text=True, check=True).stdout.strip()
assert probe == f"av1,{side['frame_count']}", f"ffprobe says {probe}, sidecar says {side['frame_count']} frames"
assert all("sim_time" in f and "frame" in f for f in side["frames"]), "frame entries need sim_time and frame"
manifest = json.load(open(pathlib.Path(burst_dir) / "manifest.json"))
assert len(manifest["frames"]) == 10, "burst frames"
for f in manifest["frames"]:
    assert pathlib.Path(f["path"]).stat().st_size > 0
meta = json.load(open(png + ".json"))
assert "sim_time" in meta and "frame" in meta and meta["width"] > 0
print(f"ok: video {side['frame_count']} frames ({side['width']}x{side['height']}), burst 10 frames, screenshot {meta['width']}x{meta['height']} at sim_time {meta['sim_time']}")
PY
