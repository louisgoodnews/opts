//! Crash acceptance: a worker that crashes on an injected fault is recorded as `F-CRASH`, and the
//! run continues with the other objects.

use std::fs;
use std::process::Command;

use super::{build_root, verdicts};

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
