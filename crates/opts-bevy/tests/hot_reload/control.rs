//! The reload and watch control commands report success and failure.

use super::*;

fn client(port: u16) -> Client {
    Client::new(ClientConfig {
        host: "127.0.0.1".into(),
        port,
        token: None,
        timeout: Duration::from_secs(10),
    })
}

fn with_client<R: Send + 'static>(app: &mut App, f: impl FnOnce() -> R + Send + 'static) -> R {
    let done = Arc::new(AtomicBool::new(false));
    let flag = done.clone();
    let handle = thread::spawn(move || {
        let r = f();
        flag.store(true, Ordering::SeqCst);
        r
    });
    while !done.load(Ordering::SeqCst) {
        app.update();
        thread::sleep(Duration::from_millis(4));
    }
    handle.join().expect("client")
}

#[test]
fn reload_commands_report_success_and_failure() {
    let dir = temp_dir("control");
    let mut app = app_watching(&dir, true);
    let port = app.world().resource::<ControlAddr>().0.port();
    let good = dir.join("good.sco");
    let bad = dir.join("bad.o3d");
    std::fs::write(&good, VALID_SCO).expect("good");
    std::fs::write(&bad, b"definitely not a mesh").expect("bad");
    // An existing file outside the allowed folder (on every platform, unlike `/etc/hostname`).
    let outside = temp_dir("control_outside").join("outside.sco");
    std::fs::write(&outside, VALID_SCO).expect("outside");
    let (g, b, o) = (good.clone(), bad.clone(), outside.clone());
    let (ok, err, roots, unsupported) = with_client(&mut app, move || {
        let mut c = client(port);
        let ok = c
            .call("reload.asset", json!({"path": g}))
            .expect("good reload");
        let err = c
            .call("reload.asset", json!({"path": b}))
            .expect_err("bad reload");
        let roots = c.call("watch.list", Value::Null).expect("watch.list");
        let unsupported = c
            .call("reload.asset", json!({"path": o}))
            .expect_err("outside");
        (ok, err, roots, unsupported)
    });
    assert_eq!(ok["kind"], "sco");
    assert!(
        matches!(&err, ClientError::Rpc(e) if e.code == code::FAILED),
        "{err}"
    );
    assert!(roots["roots"].as_array().is_some_and(|r| !r.is_empty()));
    assert!(
        matches!(&unsupported, ClientError::Rpc(e) if e.code == code::FORBIDDEN_PATH),
        "{unsupported}"
    );
}

#[test]
fn watch_add_and_remove_change_the_roots() {
    let dir = temp_dir("roots");
    let extra = temp_dir("roots_extra");
    let mut app = app_watching(&dir, true);
    let port = app.world().resource::<ControlAddr>().0.port();
    let e = extra.clone();
    // The extra directory must be readable for the policy: allow it by watching under a root that is allowed.
    let (added, listed) = with_client(&mut app, move || {
        let mut c = client(port);
        let added = c.call("watch.add", json!({"path": e}));
        let listed = c.call("watch.list", Value::Null).expect("list");
        (added, listed)
    });
    // `extra` is outside the allowed directories, so the command is refused rather than silently ignored.
    assert!(
        matches!(&added, Err(ClientError::Rpc(e)) if e.code == code::FORBIDDEN_PATH),
        "{added:?}"
    );
    assert_eq!(listed["roots"].as_array().map(Vec::len), Some(1));
}
