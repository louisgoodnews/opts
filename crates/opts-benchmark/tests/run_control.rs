//! Run-control acceptance test (M-0054): a worker that crashes or hangs on an injected fault
//! costs one restart and is recorded as `F-CRASH` / `F-TIMEOUT`; the other objects still pass,
//! workers are recycled, and a resumed run does not repeat finished objects.
//!
//! Uses a synthetic installation and the real binary; needs a GPU adapter and otherwise passes
//! with a message.

use std::fs;
use std::path::Path;
use std::process::Command;

fn write(path: &Path, text: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

/// A synthetic installation with the anchor and `names` as one-triangle-pair objects.
fn build_root(root: &Path, names: &[&str]) {
    write(
        &root.join("Sceneryobjects/Generic/entrypoint_bus.sco"),
        "[groups]\n1\nUtilities\n\n[friendlyname]\nanchor\n\n[entrypoint]\n\n[onlyeditor]\n",
    );
    let mesh = "xof 0303txt 0032\r\nMesh Q {\r\n4;\r\n-5.0;0.0;-5.0;,\r\n5.0;0.0;-5.0;,\r\n5.0;6.0;-5.0;,\r\n-5.0;6.0;-5.0;;\r\n\
        2;\r\n4;0,1,2,3;,\r\n4;3,2,1,0;;\r\nMeshMaterialList {\r\n1;\r\n2;\r\n0,0;;\r\nMaterial M {\r\n\
        0.5;0.6;0.7;1.0;;\r\n3.2;\r\n0.0;0.0;0.0;;\r\n0.0;0.0;0.0;;\r\n}\r\n}\r\n}\r\n";
    write(&root.join("Sceneryobjects/Test/model/q.x"), mesh);
    for n in names {
        write(
            &root.join(format!("Sceneryobjects/Test/{n}.sco")),
            "[groups]\n1\nTest\n\n[friendlyname]\nq\n\n[mesh]\nq.x\n",
        );
    }
}

fn run(root: &Path, out: &Path, list: &Path, extra: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_opts-benchmark"))
        .args(["scenery", "run", "--no-catalogue", "--omsi-path"])
        .arg(root)
        .arg("--out")
        .arg(out)
        .arg("--list")
        .arg(list)
        .args(extra)
        .env("XDG_CONFIG_HOME", out.join("xdg-config"))
        .env("XDG_DATA_HOME", out.join("xdg-data"))
        .env("OPTS_BENCH_INJECT", "hang:hang")
        .output()
        .unwrap()
}

fn verdicts(run_dir: &Path) -> Vec<(String, String, Vec<String>)> {
    let text = fs::read_to_string(run_dir.join("results.jsonl")).unwrap_or_default();
    text.lines()
        .filter_map(|l| serde_json::from_str::<serde_json::Value>(l).ok())
        .map(|v| {
            let cats = v["categories"]
                .as_array()
                .map(|a| {
                    a.iter()
                        .filter_map(|c| c.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default();
            (
                v["sco"].as_str().unwrap_or_default().to_string(),
                v["verdict"].as_str().unwrap_or_default().to_string(),
                cats,
            )
        })
        .collect()
}

#[test]
fn crash_hang_recycle_and_resume() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("omsi");
    let names = ["a", "b", "hang_x", "c", "d", "e"];
    build_root(&root, &names);
    let list = tmp.path().join("list.txt");
    let lines: Vec<String> = names
        .iter()
        .map(|n| format!("Sceneryobjects/Test/{n}.sco"))
        .collect();
    fs::write(&list, lines.join("\n")).unwrap();
    let out = tmp.path().join("out");
    // The crash fault travels in the worker environment through `OPTS_BENCH_INJECT`; one spec
    // per run, so the crash is injected in a second run on the same run directory below.
    // The hang loops forever, so any limit catches it; 10 s leaves room for a fresh worker's
    // first object on a slow machine (the Windows CI runner timed `a.sco` out at 2 s).
    let first = run(
        &root,
        &out,
        &list,
        &["--workers", "2", "--timeout-s", "10", "--recycle", "2"],
    );
    let stderr = String::from_utf8_lossy(&first.stderr);
    if !first.status.success() && stderr.contains("failed to start") {
        eprintln!("skipped: workers cannot start (no GPU adapter?)\n{stderr}");
        return;
    }
    assert!(first.status.success(), "{stderr}");
    let run_dir = fs::read_dir(&out)
        .unwrap()
        .filter_map(Result::ok)
        .map(|e| e.path())
        .find(|p| p.join("results.jsonl").exists())
        .unwrap();
    let got = verdicts(&run_dir);
    assert_eq!(got.len(), 6, "{got:?}");
    for (sco, verdict, cats) in &got {
        if sco.contains("hang_x") {
            assert_eq!(cats, &["F-TIMEOUT"], "{sco}");
            assert_eq!(verdict, "fail");
        } else {
            assert_eq!(verdict, "pass", "{sco}: {cats:?}");
        }
    }
    let run_id = run_dir.file_name().unwrap().to_str().unwrap().to_string();
    let again = run(&root, &out, &list, &["--workers", "2", "--resume", &run_id]);
    assert!(again.status.success());
    assert_eq!(
        verdicts(&run_dir).len(),
        6,
        "resume must not repeat finished objects"
    );
}

#[test]
fn crash_is_recorded_and_the_run_continues() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("omsi");
    let names = ["a", "boom", "c"];
    build_root(&root, &names);
    let list = tmp.path().join("list.txt");
    let lines: Vec<String> = names
        .iter()
        .map(|n| format!("Sceneryobjects/Test/{n}.sco"))
        .collect();
    fs::write(&list, lines.join("\n")).unwrap();
    let out = tmp.path().join("out");
    let o = Command::new(env!("CARGO_BIN_EXE_opts-benchmark"))
        .args([
            "scenery",
            "run",
            "--no-catalogue",
            "--workers",
            "1",
            "--omsi-path",
        ])
        .arg(&root)
        .arg("--out")
        .arg(&out)
        .arg("--list")
        .arg(&list)
        .env("XDG_CONFIG_HOME", out.join("xdg-config"))
        .env("XDG_DATA_HOME", out.join("xdg-data"))
        .env("OPTS_BENCH_INJECT", "crash:boom")
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&o.stderr);
    if !o.status.success() && stderr.contains("failed to start") {
        eprintln!("skipped: workers cannot start (no GPU adapter?)");
        return;
    }
    assert!(o.status.success(), "{stderr}");
    let run_dir = fs::read_dir(&out)
        .unwrap()
        .filter_map(Result::ok)
        .map(|e| e.path())
        .find(|p| p.join("results.jsonl").exists())
        .unwrap();
    let got = verdicts(&run_dir);
    assert_eq!(got.len(), 3, "{got:?}");
    for (sco, verdict, cats) in &got {
        if sco.contains("boom") {
            assert_eq!(cats, &["F-CRASH"]);
        } else {
            assert_eq!(verdict, "pass", "{sco}: {cats:?}");
        }
    }
}
