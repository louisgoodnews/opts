#!/usr/bin/env python3
"""Frame-rate cost of screen recording on a loaded scene (design criterion: >= 80 % of the
frame rate before recording).

Start the game first, load a map with an active vehicle and AI traffic, then:

    scripts/capture-fps-test.py --port 44219 [--window 30] [--step 0.25]

The script talks to the control server directly. It alternates baseline phases (no recording) and
recording phases, samples the frame counter (`state.frame`) every `--step` seconds, and reports
p5 / p50 / p95 of the frame rate per phase and the ratio of each recording phase to the mean of
the baselines around it. p5 is the slow end (the 95th percentile of frame *time*); p95 the fast
end. A phase list can be given with `--phases`, for example
`baseline,default,baseline,low,baseline`; recording phases are the names `default` (protocol
defaults: quality 70, scale 0.5), `low` (quality 20), `full` (scale 1.0), `full60` (scale 1.0 at 60 fps, the worst case) and `q:<n>` (quality n).
Results go to stdout as a table and to `--json` as machine-readable data.
"""
import argparse
import json
import socket
import statistics
import sys
import time


class Client:
    def __init__(self, host, port, token=None):
        self.sock = socket.create_connection((host, port), timeout=60)
        self.file = self.sock.makefile("rwb")
        self.next_id = 1
        if token:
            self.call("auth", {"token": token})

    def call(self, method, params=None):
        msg = {"jsonrpc": "2.0", "id": self.next_id, "method": method, "params": params or {}}
        self.next_id += 1
        self.file.write((json.dumps(msg) + "\n").encode())
        self.file.flush()
        while True:
            line = self.file.readline()
            if not line:
                raise RuntimeError("control connection closed")
            reply = json.loads(line)
            if reply.get("id") == msg["id"]:
                if "error" in reply:
                    raise RuntimeError(f"{method}: {reply['error']}")
                return reply["result"]


def percentile(sorted_values, p):
    if not sorted_values:
        return float("nan")
    k = (len(sorted_values) - 1) * p / 100.0
    lo, hi = int(k), min(int(k) + 1, len(sorted_values) - 1)
    return sorted_values[lo] + (sorted_values[hi] - sorted_values[lo]) * (k - lo)


def sample(client, seconds, step):
    """Frame rates over consecutive `step`-second windows."""
    rates = []
    start = time.perf_counter()
    prev_t, prev_f = start, client.call("state")["frame"]
    while time.perf_counter() - start < seconds:
        time.sleep(max(0.0, step - 0.002))
        t = time.perf_counter()
        f = client.call("state")["frame"]
        if t > prev_t and f >= prev_f:
            rates.append((f - prev_f) / (t - prev_t))
        prev_t, prev_f = t, f
    return rates


def summarise(rates):
    s = sorted(rates)
    return {
        "n": len(s),
        "mean": statistics.fmean(s) if s else float("nan"),
        "p5": percentile(s, 5),
        "p50": percentile(s, 50),
        "p95": percentile(s, 95),
        "min": s[0] if s else float("nan"),
    }


def record_params(kind, window):
    params = {"duration_ms": int((window + 4) * 1000), "name": f"fps-test/{kind.replace(':', '_')}"}
    if kind == "low":
        params["quality"] = 20
    elif kind == "full":
        params["scale"] = 1.0
    elif kind == "full60":
        params.update(scale=1.0, fps=60)
    elif kind.startswith("q:"):
        params["quality"] = int(kind[2:])
    return params


def wait_finished(client, recording_id, timeout=60):
    end = time.time() + timeout
    while time.time() < end:
        status = client.call("capture.record.status", {"recording_id": recording_id})
        if status.get("state") in ("finished", "failed"):
            return status
        time.sleep(0.3)
    raise RuntimeError("recording did not finish")


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--host", default="127.0.0.1")
    ap.add_argument("--port", type=int, default=44201)
    ap.add_argument("--token")
    ap.add_argument("--window", type=float, default=30.0, help="seconds per phase")
    ap.add_argument("--step", type=float, default=0.25, help="sampling window in seconds")
    ap.add_argument("--settle", type=float, default=4.0, help="seconds between phases")
    ap.add_argument("--phases", default="baseline,default,baseline,low,baseline")
    ap.add_argument("--json", help="write the results here")
    args = ap.parse_args()

    client = Client(args.host, args.port, args.token)
    state = client.call("state")
    print(f"app_state={state.get('app_state')} map={state.get('map')!r} vehicle={state.get('vehicle')!r} "
          f"fps(smoothed)={state.get('fps')}", file=sys.stderr)
    results = []
    for kind in args.phases.split(","):
        time.sleep(args.settle)
        info = {}
        if kind != "baseline":
            started = client.call("capture.record.start", record_params(kind, args.window))
            rid = started["recording_id"]
            time.sleep(1.0)  # the recorder starts; its first second is not part of the phase
        rates = sample(client, args.window, args.step)
        phase = {"phase": kind, **summarise(rates), "worst_frame_ms_10s": client.call("state").get("frame_ms_max")}
        if kind != "baseline":
            client.call("capture.record.stop", {"recording_id": rid})
            status = wait_finished(client, rid)
            phase["recording"] = {k: status.get(k) for k in
                                  ("frame_count", "dropped", "bytes", "stopped_by", "timing_ms", "state")}
        results.append(phase)
        print(f"{kind:>10}: p50 {phase['p50']:.1f}  p5 {phase['p5']:.1f}  p95 {phase['p95']:.1f}  "
              f"mean {phase['mean']:.1f} fps", file=sys.stderr)

    print("| Phase | p5 fps | p50 fps | p95 fps | mean | ratio p50 | ratio p5 | frames recorded / dropped |")
    print("| :--- | ---: | ---: | ---: | ---: | ---: | ---: | :--- |")
    for i, r in enumerate(results):
        ratio50 = ratio5 = ""
        extra = ""
        if r["phase"] != "baseline":
            around = [x for x in (results[i - 1] if i > 0 else None, results[i + 1] if i + 1 < len(results) else None)
                      if x and x["phase"] == "baseline"]
            if around:
                b50 = statistics.fmean(x["p50"] for x in around)
                b5 = statistics.fmean(x["p5"] for x in around)
                ratio50, ratio5 = f"{r['p50'] / b50:.2f}", f"{r['p5'] / b5:.2f}"
            rec = r.get("recording", {})
            extra = f"{rec.get('frame_count')} / {rec.get('dropped')}"
        print(f"| {r['phase']} | {r['p5']:.1f} | {r['p50']:.1f} | {r['p95']:.1f} | {r['mean']:.1f} | {ratio50} | {ratio5} | {extra} |")
    if args.json:
        with open(args.json, "w") as f:
            json.dump({"state": state, "window_s": args.window, "step_s": args.step, "phases": results}, f, indent=2)


if __name__ == "__main__":
    main()
