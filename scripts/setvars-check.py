#!/usr/bin/env python3
"""Screenshot recipe for the equipment options of a vehicle (M-0078).

Starts a drive on the installed OMSI 2 with the control protocol, lists the vehicle's options
(`vehicle.setvars`), changes the ones named on the command line one after the other while the
simulation runs, and after each change records the script variable (`vehicle.script_vars`) and a
screenshot, then reports how many pixels differ from the first screenshot inside a crop around
the bus (the HUD, the toast and the clock are outside it), next to a noise baseline: two
screenshots without a change. A change that the model reacts to shows a difference well above
the noise; one that only takes effect when the scripts start does not (open question Q-SV02 in
docs/src/formats/text/vehicles/setvars.md).

  scripts/setvars-check.py [--bus "Citybus 530 by Kajosoft/01a_o530_e2_2.bus"] [--map Grundorf]
                           [--out DIR] [--settle 3] [--crop 0.15,0.18,0.85,0.65]
                           NAME=VALUE [NAME=VALUE ...]

Environment: OMSI (install, default below), OPTS_BIN (target/debug/opts-game), PORT (44271).
Needs ImageMagick (`compare`). The game opens a window; throw-away XDG directories are used.
Results: <out>/before.png, <out>/after_<name>.png, <out>/options.json, <out>/report.json.
"""
import argparse
import json
import os
import shutil
import socket
import subprocess
import sys
import tempfile
import time
from pathlib import Path

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


def ok(reply):
    if "error" in reply:
        raise RuntimeError(reply["error"])
    return reply["result"]


def cropped(src: Path, frac) -> Path:
    """The part of a screenshot inside `frac` = (x0, y0, x1, y1) as fractions of its size."""
    w, h = map(int, subprocess.run(["identify", "-format", "%w %h", str(src)], capture_output=True, text=True).stdout.split())
    x0, y0, x1, y1 = frac
    dest = src.with_name(src.stem + "_crop.png")
    box = f"{int((x1 - x0) * w)}x{int((y1 - y0) * h)}+{int(x0 * w)}+{int(y0 * h)}"
    subprocess.run(["magick", str(src), "-crop", box, "+repage", str(dest)], check=True)
    return dest


def differing_pixels(a: Path, b: Path, frac) -> int:
    """Pixels that differ inside the crop between two screenshots (`compare -metric AE`,
    with a small fuzz so that dithering and noise of the renderer do not count)."""
    out = subprocess.run(["compare", "-fuzz", "3%", "-metric", "AE", str(cropped(a, frac)), str(cropped(b, frac)), "null:"], capture_output=True, text=True)
    text = (out.stderr or out.stdout).strip().split()[0]
    return int(float(text))


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--bus", default="Citybus 530 by Kajosoft/01a_o530_e2_2.bus", help=".bus below Vehicles")
    ap.add_argument("--map", default="Grundorf")
    ap.add_argument("--out", default=None)
    ap.add_argument("--settle", type=float, default=3.0, help="seconds to wait after a change")
    ap.add_argument("--crop", default="0.15,0.18,0.85,0.65", help="x0,y0,x1,y1 as fractions of the screenshot")
    ap.add_argument("changes", nargs="+", metavar="NAME=VALUE")
    a = ap.parse_args()
    frac = tuple(float(x) for x in a.crop.split(","))
    omsi = os.environ.get("OMSI", DEFAULT_OMSI)
    binp = os.environ.get("OPTS_BIN", "target/debug/opts-game")
    port = int(os.environ.get("PORT", "44271"))
    base = Path.home() / ".local/share/opts/benchmark/setvars-check"
    base.mkdir(parents=True, exist_ok=True)
    out = Path(a.out) if a.out else base / time.strftime("%Y%m%dT%H%M%S")
    out.mkdir(parents=True, exist_ok=True)
    cfgd, datad = tempfile.mkdtemp(dir=base), tempfile.mkdtemp(dir=base)
    env = dict(os.environ, XDG_CONFIG_HOME=cfgd, XDG_DATA_HOME=datad, OPTS_DEV_HOUR="12")
    cmd = [binp, "--omsi-path", omsi, "--map-path", f"{omsi}/maps/{a.map}",
           "--vehicle-path", f"{omsi}/Vehicles/{a.bus}", "--autostart", "--windowed",
           "--traffic", "0", "--pedestrians", "0", "--no-hot-reload", "--cli-port", str(port),
           "--cli-allow-dir", str(out)]
    log = open(out / "game.log", "w")
    proc = subprocess.Popen(cmd, env=env, stdout=log, stderr=subprocess.STDOUT)
    report = {"bus": a.bus, "map": a.map, "changes": []}
    try:
        cl = None
        for _ in range(2400):
            try:
                cl = cl or Client(port)
                if cl.call("state").get("result", {}).get("loading", {}).get("ready") is True:
                    break
            except Exception:
                cl = None
            if proc.poll() is not None:
                sys.exit("game exited early; see game.log")
            time.sleep(0.25)
        else:
            sys.exit("the drive did not start")
        time.sleep(12)  # the load notice (a toast) has gone by then
        ok(cl.call("camera.set_mode", mode="external"))
        time.sleep(2)
        listing = ok(cl.call("vehicle.setvars"))
        (out / "options.json").write_text(json.dumps(listing, indent=1))
        known = {o["name"].lower(): o for o in listing["options"]}
        print(f"{a.bus}: {listing['count']} options, family {listing['family']}, repaint {listing['repaint']!r}")

        def shot(name):
            r = ok(cl.call("screenshot", name=name))
            dest = out / f"{name}.png"
            shutil.copy(r["path"], dest)
            return dest

        before = shot("before")
        time.sleep(a.settle)
        report["noise_pixels"] = differing_pixels(before, shot("before_again"), frac)
        print(f"  no change: {report['noise_pixels']} pixels differ (the noise level)")
        for change in a.changes:
            name, _, value = change.partition("=")
            opt = known.get(name.lower())
            entry = {"name": name, "value": value, "listed": opt is not None}
            if opt:
                entry.update(kind=opt["kind"], default=opt["default"], before=opt["value"], values=opt["values"])
            reply = cl.call("vehicle.setvars", values={name: float(value)}, allow_unlisted=opt is None)
            if "error" in reply:
                entry["refused"] = reply["error"]["message"]
                report["changes"].append(entry)
                print(f"  {change}: refused: {reply['error']['message']}")
                continue
            time.sleep(a.settle)
            variables = ok(cl.call("vehicle.script_vars", contains=name.lower()))["variables"]
            entry["script_var"] = variables.get(name) if name in variables else variables
            after = shot(f"after_{name}")
            entry["pixels_changed"] = differing_pixels(before, after, frac)
            report["changes"].append(entry)
            print(f"  {change}: script variable {entry['script_var']}, {entry['pixels_changed']} pixels differ from before")
            # Put it back so the next change is measured against the same picture.
            if opt:
                cl.call("vehicle.setvars", values={name: opt["value"]})
                time.sleep(a.settle)
        cl.call("shutdown")
    finally:
        try:
            proc.wait(timeout=25)
        except Exception:
            proc.kill()
        shutil.rmtree(cfgd, ignore_errors=True)
        shutil.rmtree(datad, ignore_errors=True)
    (out / "report.json").write_text(json.dumps(report, indent=1))
    print(f"results in {out}")


if __name__ == "__main__":
    main()
