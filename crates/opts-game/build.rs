//! Embeds the short git commit hash (`OPTS_GIT_HASH`) for bug reports.

use std::process::Command;

fn main() {
    let hash = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_owned())
        .filter(|h| !h.is_empty());
    if let Some(h) = hash {
        println!("cargo:rustc-env=OPTS_GIT_HASH={h}");
    }
    println!("cargo:rerun-if-changed=../../.git/HEAD");
}
