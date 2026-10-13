//! The charter forbids network access: fail when an HTTP, socket, TLS or async-runtime crate is
//! anywhere in the dependency tree of `opts-hardware` (all platforms, normal and build
//! dependencies), or when a Windows-API crate is built with a networking feature.

use std::collections::{BTreeMap, BTreeSet};
use std::process::Command;

use serde_json::Value;

const BANNED: &[&str] = &[
    "reqwest", "hyper", "hyper-util", "h2", "http", "http-body", "httparse", "ureq", "curl", "curl-sys",
    "isahc", "attohttpc", "surf", "tungstenite", "tokio", "async-std", "smol", "async-net", "mio",
    "socket2", "rustls", "native-tls", "openssl", "openssl-sys", "schannel", "security-framework",
    "security-framework-sys", "quinn", "tiny_http", "webpki", "webpki-roots", "tonic", "libp2p",
    "zmq", "nng", "trust-dns-resolver", "hickory-resolver", "ws", "tiny-http", "minreq",
];

fn metadata() -> Value {
    let manifest = concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml");
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    let out = Command::new(cargo)
        .args(["metadata", "--format-version", "1", "--locked", "--manifest-path", manifest])
        .output()
        .expect("cargo metadata starts");
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    serde_json::from_slice(&out.stdout).expect("metadata is JSON")
}

#[test]
fn no_network_dependencies() {
    let m = metadata();
    let names: BTreeMap<&str, &str> = m["packages"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| (p["id"].as_str().unwrap(), p["name"].as_str().unwrap()))
        .collect();
    let nodes: BTreeMap<&str, &Value> = m["resolve"]["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|n| (n["id"].as_str().unwrap(), n))
        .collect();
    let root = names.iter().find(|(_, n)| **n == "opts-hardware").map(|(id, _)| *id).unwrap();

    let mut seen = BTreeSet::new();
    let mut stack = vec![root];
    while let Some(id) = stack.pop() {
        if !seen.insert(id) {
            continue;
        }
        for d in nodes[id]["deps"].as_array().unwrap() {
            let real = d["dep_kinds"].as_array().unwrap().iter().any(|k| k["kind"] != "dev");
            if real {
                stack.push(d["pkg"].as_str().unwrap());
            }
        }
    }
    let tree: BTreeSet<&str> = seen.iter().map(|id| names[id]).collect();
    let found: Vec<_> = tree.iter().filter(|n| BANNED.contains(n)).collect();
    assert!(found.is_empty(), "network crates in the dependency tree of opts-hardware: {found:?}\nfull tree: {tree:?}");

    for id in &seen {
        let features = nodes[id]["features"].as_array().unwrap();
        for f in features.iter().filter_map(Value::as_str) {
            let l = f.to_ascii_lowercase();
            assert!(
                !(l.contains("networking") || l.contains("winsock") || l.contains("http") || l.contains("sockets")),
                "{} enables the networking feature {f}",
                names[id]
            );
        }
    }
    assert!(tree.contains("sysinfo"), "the check walked the tree: {tree:?}");
}
