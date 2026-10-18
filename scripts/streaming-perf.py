#!/usr/bin/env python3
"""Tile-streaming frame-time benchmark (M-0089).

Drives the free camera along a straight line over tile borders through the control protocol, so a
run is reproducible, and analyses the per-frame trace the `stream-trace` build writes
(`OPTS_STREAM_TRACE=<file>`, one JSON line per frame: frame period and span timers).

  scripts/streaming-perf.py run  <label> <map-dir-name> [--entry NAME] [--vehicle REL]
                                 [--speed 25] [--legs 2] [--leg-s 40] [--dir auto|E|N|W|S]
                                 [--setting KEY=VALUE ...]
  scripts/streaming-perf.py analyse <trace.jsonl> [--phases 5,45,85]

Environment: OMSI (install), OPTS_BIN (target/release/opts-game), OUT (default
~/.local/share/opts/benchmark/streaming-perf/<label>), DECODER_CACHE (directory that holds a COPY
of the `opts/descramble` cache; XDG_CACHE_HOME of the game), PORT (44261), EXTRA_ENV (k=v,k=v),
EXTRA_ARGS (extra game arguments). The game runs with throw-away XDG config/data directories, no
traffic, no pedestrians, the simulation paused and hot reload off, so the only moving thing is the
camera, the observer of the streamer. A 'leg' flies `--leg-s` seconds at `--speed` m/s in the chosen
direction, the next leg flies back (second visit: caches warm). Results: <OUT>/trace.jsonl,
<OUT>/summary.json, <OUT>/game.log.
"""
import argparse
import json
import math
import os
import re
import shutil
import socket
import statistics
import subprocess
import sys
import tempfile
import time
from pathlib import Path

HOME = Path.home()
DEFAULT_OMSI = "/home/louisgoodnews/nvme2n1/steamapps/common/OMSI 2"


class Client:
    def __init__(self, port):
        self.sock = socket.create_connection(("127.0.0.1", port), timeout=130)
        self.f = self.sock.makefile("rwb")
        self.n = 0

    def call(self, method, **params):
        self.n += 1
        self.f.write((json.dumps({"jsonrpc": "2.0", "id": self.n, "method": method, "params": params}) + "\n").encode())
        self.f.flush()
        line = self.f.readline()
        if not line:
            raise RuntimeError("connection closed")
        return json.loads(line)


def read_tiles(cfg: Path):
    raw = cfg.read_bytes()
    if raw[:2] in (b"\xff\xfe", b"\xfe\xff"):
        text = raw.decode("utf-16")
    else:
        text = raw.decode("latin-1")
    lines = [l.strip() for l in text.splitlines()]
    tiles = set()
    for i, l in enumerate(lines):
        if l == "[map]" and i + 3 < len(lines):
            try:
                tiles.add((int(lines[i + 1]), int(lines[i + 2])))
            except ValueError:
                pass
    return tiles


def pct(xs, p):
    if not xs:
        return 0.0
    xs = sorted(xs)
    return xs[min(len(xs) - 1, int(round(p / 100 * (len(xs) - 1))))]


def analyse(path, phases=None, top=12, frame_bounds=None):
    frames = [json.loads(l) for l in Path(path).read_text().splitlines() if l.strip()]
    frames = [f for f in frames if f["dt"] > 0]
    dts = [f["dt"] for f in frames]

    def stats(sel):
        d = [f["dt"] for f in sel]
        if not d:
            return {}
        return {
            "frames": len(d),
            "mean_ms": round(statistics.fmean(d), 2),
            "p50_ms": round(pct(d, 50), 2),
            "p95_ms": round(pct(d, 95), 2),
            "p99_ms": round(pct(d, 99), 2),
            "max_ms": round(max(d), 2),
            "over_33ms": sum(1 for x in d if x > 33.4),
            "over_50ms": sum(1 for x in d if x > 50),
            "over_100ms": sum(1 for x in d if x > 100),
            "hitch_ms_over_33": round(sum(x for x in d if x > 33.4), 1),
            # stalls that no tile build explains (the span `tile.build` is absent from the frame)
            "hitch_ms_without_tile_build": round(
                sum(f["dt"] for f in sel if f["dt"] > 33.4 and "tile.build" not in f["s"]), 1),
        }

    out = {"all": stats(frames)}
    if frame_bounds:
        # phases by the trace's own frame number: before the flight, one per leg, after it
        bounds = [-1] + list(frame_bounds) + [1 << 60]
        names = ["pre"] + [f"leg{i + 1}" for i in range(len(frame_bounds) - 1)] + ["post"]
        for i, name in enumerate(names):
            out[name] = stats([f for f in frames if bounds[i] < f["f"] <= bounds[i + 1]])
    elif phases:
        # phases: elapsed-time boundaries in seconds (relative to the first frame)
        t = 0.0
        for f in frames:
            t += f["dt"] / 1000
            f["_t"] = t
        bounds = [0.0] + list(phases) + [1e9]
        for i in range(len(bounds) - 1):
            out[f"phase{i}"] = stats([f for f in frames if bounds[i] <= f["_t"] < bounds[i + 1]])
    # span totals over the whole run
    tot = {}
    for f in frames:
        for k, (c, ms, mx) in f["s"].items():
            a = tot.setdefault(k, [0, 0.0, 0.0])
            a[0] += c
            a[1] += ms
            a[2] = max(a[2], mx)
    out["spans"] = {k: {"count": v[0], "total_ms": round(v[1], 1), "max_ms": round(v[2], 2)} for k, v in sorted(tot.items(), key=lambda kv: -kv[1][1])}
    worst = sorted(frames, key=lambda f: -f["dt"])[:top]
    out["worst"] = [
        {
            "frame": f["f"],
            "dt_ms": round(f["dt"], 1),
            "spans": {k: v[1] for k, v in sorted(f["s"].items(), key=lambda kv: -kv[1][1]) if v[1] >= 0.5},
        }
        for f in worst
    ]
    # how much of the frame the (main-thread) tile build explains
    big = [f for f in frames if f["dt"] > 33.4]
    cov = []
    for f in big:
        b = f["s"].get("tile.build")
        cov.append(b[1] / f["dt"] if b else 0.0)
    out["hitch_frames_explained_by_tile_build"] = round(statistics.fmean(cov), 3) if cov else None
    return out


def run(a):
    omsi = os.environ.get("OMSI", DEFAULT_OMSI)
    binp = os.environ.get("OPTS_BIN", "target/release/opts-game")
    port = int(os.environ.get("PORT", "44261"))
    base = HOME / ".local/share/opts/benchmark/streaming-perf"
    out = Path(os.environ.get("OUT", base / a.label))
    out.mkdir(parents=True, exist_ok=True)
    cache = os.environ.get("DECODER_CACHE")
    if not cache:
        sys.exit("set DECODER_CACHE to a directory containing a copy of opts/descramble")
    cfgd = tempfile.mkdtemp(dir=base)
    datad = tempfile.mkdtemp(dir=base)
    env = dict(os.environ, XDG_CONFIG_HOME=cfgd, XDG_DATA_HOME=datad, XDG_CACHE_HOME=cache,
               OPTS_DEV_HOUR="12", OPTS_STREAM_TRACE=str(out / "trace.jsonl"))
    if a.entry:
        env["OPTS_DEV_ENTRY"] = a.entry
    for kv in filter(None, os.environ.get("EXTRA_ENV", "").split(",")):
        k, v = kv.split("=", 1)
        env[k] = v
    cmd = [binp, "--omsi-path", omsi, "--map-path", f"{omsi}/maps/{a.map}",
           "--vehicle-path", f"{omsi}/Vehicles/{a.vehicle}", "--autostart", "--windowed",
           "--camera", "ego", "--traffic", "0", "--pedestrians", "0", "--no-hot-reload",
           "--cli-port", str(port), "--cli-allow-dir", str(out)] + os.environ.get("EXTRA_ARGS", "").split()
    load0 = Path("/proc/loadavg").read_text().split()[:3]
    log = open(out / "game.log", "w")
    t0 = time.time()
    proc = subprocess.Popen(cmd, env=env, stdout=log, stderr=subprocess.STDOUT)
    cl = None
    try:
        for _ in range(7200):
            try:
                cl = Client(port)
                r = cl.call("state")
                if r.get("result", {}).get("loading", {}).get("ready") is True:
                    break
            except Exception:
                cl = None
            if proc.poll() is not None:
                sys.exit("game exited early; see game.log")
            time.sleep(0.25)
        else:
            sys.exit("session did not start")
        load_s = time.time() - t0
        for kv in a.setting:
            key, _, value = kv.partition("=")
            r = cl.call("settings.set", key=key, value=value)
            if "error" in r:
                sys.exit(f"setting {kv}: {r['error']['message']}")
        cl.call("sim.pause")
        # let the initial streaming settle
        time.sleep(float(os.environ.get("WARM", "15")))
        text = re.sub(r"\x1b\[[0-9;]*m", "", (out / "game.log").read_text(errors="replace"))
        # The tile size: the pitch the grid measured from the splines (`tile grid` line), or the
        # older `tile_size_m` field, or the default of 300 m.
        sm = re.search(r"median_step_m=Some\(([0-9.]+)\)", text) or re.search(r"tile_size_m[= ]+([0-9.]+)", text)
        size = float(sm.group(1)) if sm else 300.0
        m = re.search(r"start tile and spawn point selected.*?tile=\((-?\d+), (-?\d+)\)", text, re.S) or \
            re.search(r"tile=\((-?\d+), (-?\d+)\).*?start tile and spawn", text, re.S)
        start = (int(m.group(1)), int(m.group(2))) if m else None
        tiles = read_tiles(Path(omsi) / "maps" / a.map / "global.cfg")
        v = cl.call("vehicle.get")["result"]
        x0, y0, z0 = v["x"], v["y"], v["z"] + 4.0
        dirs = {"E": (1, 0), "W": (-1, 0), "N": (0, 1), "S": (0, -1)}
        want = a.dir
        if want == "auto":
            best = (-1, "E")
            for name, (dx, dy) in dirs.items():
                n, k = 0, start
                while k and (k[0] + dx, k[1] + dy) in tiles and n < 50:
                    k = (k[0] + dx, k[1] + dy)
                    n += 1
                if n > best[0]:
                    best = (n, name)
            want = best[1]
        dx, dy = dirs[want]
        dist = a.speed * a.leg_s
        print(f"{a.label}: tile size {size} m, start tile {start}, tiles {len(tiles)}, direction {want}, "
              f"leg {dist:.0f} m = {dist / size:.1f} tiles, load {load_s:.1f} s", flush=True)
        # Flight: out, back, out, ...; position is a function of wall time (a hitch lets the camera
        # jump ahead, as a vehicle that keeps driving would).
        scene_samples = []
        bounds = []
        frame_marks = [cl.call("state")["result"]["frame"]]
        tstart = time.time()
        for leg in range(a.legs):
            sign = 1 if leg % 2 == 0 else -1
            leg_t0 = time.time()
            bounds.append(round(leg_t0 - tstart, 2))
            while True:
                el = time.time() - leg_t0
                if el >= a.leg_s:
                    break
                s = (el * a.speed) if sign > 0 else (dist - el * a.speed)
                cl.call("camera.teleport", x=x0 + dx * s, y=y0 + dy * s, z=z0, yaw=0, pitch=-5)
                time.sleep(0.02)
            frame_marks.append(cl.call("state")["result"]["frame"])
        bounds.append(round(time.time() - tstart, 2))
        time.sleep(3)
        scene = cl.call("scene").get("result", {})
        warm_end = time.time()
        meta = {"label": a.label, "map": a.map, "entry": a.entry, "tile_size_m": size, "start_tile": start,
                "tiles_in_cfg": len(tiles), "direction": want, "speed_m_s": a.speed, "legs": a.legs,
                "leg_s": a.leg_s, "load_s": round(load_s, 1), "loadavg_start": load0,
                "loadavg_end": Path("/proc/loadavg").read_text().split()[:3],
                "scene_tiles_end": len(scene.get("tiles", [])), "entity_count_end": scene.get("entity_count"),
                "leg_bounds_s": bounds}
        frame_end = cl.call("state")["result"]["frame"]
        cl.call("shutdown")
    finally:
        try:
            proc.wait(timeout=20)
        except Exception:
            proc.kill()
        shutil.rmtree(cfgd, ignore_errors=True)
        shutil.rmtree(datad, ignore_errors=True)
    # The trace starts when the first frame of the game ran; the flight starts later. Find it by the
    # first frame after the warm-up: use cumulative time of the trace relative to its end.
    frames = [json.loads(l) for l in (out / "trace.jsonl").read_text().splitlines() if l.strip()]
    total = sum(f["dt"] for f in frames) / 1000
    # flight began (3 s settle + sum of legs) before the end of the trace
    flight = bounds[-1]
    flight_start = max(0.0, total - 3 - flight)
    phases = [flight_start] + [flight_start + b for b in bounds[1:-1]]
    # The control protocol's frame counter and the trace's differ by a constant: calibrate it on
    # the last state call before shutdown (the few frames after it are the shutdown itself).
    shift = frames[-1]["f"] - frame_end
    fb = [m + shift for m in frame_marks]
    meta["frame_marks"] = frame_marks
    meta["trace_frame_shift"] = shift
    res = analyse(out / "trace.jsonl", phases=phases, frame_bounds=fb)
    res["meta"] = meta
    (out / "summary.json").write_text(json.dumps(res, indent=1))
    for k in ["all", "pre"] + [f"leg{i + 1}" for i in range(len(frame_marks) - 1)] + ["post"]:
        if k in res:
            print(k, res[k], flush=True)
    for w in res["worst"][:5]:
        print("worst", w, flush=True)


if __name__ == "__main__":
    ap = argparse.ArgumentParser()
    sub = ap.add_subparsers(dest="cmd", required=True)
    r = sub.add_parser("run")
    r.add_argument("label")
    r.add_argument("map")
    r.add_argument("--entry")
    r.add_argument("--vehicle", default="MAN_SD200/MAN_SD77.bus")
    r.add_argument("--speed", type=float, default=25.0)
    r.add_argument("--legs", type=int, default=2)
    r.add_argument("--leg-s", type=float, default=40.0)
    r.add_argument("--dir", default="auto")
    r.add_argument("--setting", action="append", default=[], metavar="KEY=VALUE",
                   help="a setting to change through the protocol once the drive is up (repeatable)")
    an = sub.add_parser("analyse")
    an.add_argument("trace")
    an.add_argument("--phases", default="")
    args = ap.parse_args()
    if args.cmd == "run":
        run(args)
    else:
        ph = [float(x) for x in args.phases.split(",") if x]
        print(json.dumps(analyse(args.trace, ph), indent=1))
