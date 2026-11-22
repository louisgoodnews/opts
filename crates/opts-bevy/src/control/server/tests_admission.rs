//! Tests of the idle read timeout and of the connection cap (ISS-003).

use std::io::{BufRead, BufReader};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::AtomicBool;
use std::time::Duration;

use opts_cli::protocol::code;
use serde_json::Value;

use super::admission::MAX_CONNECTIONS;
use super::*;

const TOKEN: &str = "0123456789abcdef0123456789abcdef";

#[test]
fn a_silent_client_is_dropped_after_the_read_timeout() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let addr = listener.local_addr().expect("addr");
    let _client = TcpStream::connect(addr).expect("connect");
    let (server, _) = listener.accept().expect("accept");
    let (done_tx, done_rx) = mpsc::channel();
    std::thread::spawn(move || {
        let (tx, _rx) = mpsc::sync_channel(1);
        let stop = AtomicBool::new(false);
        let throttle = AuthThrottle::default();
        let idle = Duration::from_millis(200);
        serve_connection(server, &tx, Some(TOKEN), &stop, &throttle, idle);
        let _ = done_tx.send(());
    });
    assert!(
        done_rx.recv_timeout(Duration::from_secs(10)).is_ok(),
        "a silent connection was not closed"
    );
}

#[test]
fn the_ninth_connection_is_refused_with_busy() {
    let (tx, _rx) = mpsc::sync_channel(QUEUE_CAPACITY);
    let cfg = ServerConfig {
        bind: "127.0.0.1".into(),
        port: 0,
        token: Some(TOKEN.into()),
    };
    let handle = start(&cfg, tx).expect("start");
    // Silent clients hold the slots: the listener serves connections in accept order.
    let held: Vec<TcpStream> = (0..MAX_CONNECTIONS)
        .map(|_| TcpStream::connect(handle.addr).expect("connect"))
        .collect();
    let ninth = TcpStream::connect(handle.addr).expect("connect");
    ninth
        .set_read_timeout(Some(Duration::from_secs(5)))
        .expect("timeout");
    let mut line = String::new();
    BufReader::new(ninth)
        .read_line(&mut line)
        .expect("a refusal line");
    let reply: Value = serde_json::from_str(&line).expect("json");
    assert_eq!(reply["error"]["code"], serde_json::json!(code::BUSY));
    drop(held);
}
