//! Errors name the problem.

use super::*;

#[test]
fn errors_name_the_problem() {
    let (mut app, port, _) = app();
    let (missing, disabled, bad_action, unfocusable_ok) = with_client(&mut app, port, |c| {
        (
            c.call("ui.click", json!({"id": "nope"}))
                .expect_err("missing"),
            c.call("ui.click", json!({"id": "menu.load"}))
                .expect_err("disabled"),
            c.call("ui.action", json!({"action": "dance"}))
                .expect_err("bad action"),
            c.call("ui.focus", json!({"id": "menu.load"})).is_ok(),
        )
    });
    assert_eq!(rpc_code(missing), code::NOT_FOUND);
    let ClientError::Rpc(d) = disabled else {
        panic!("rpc error")
    };
    assert_eq!(d.code, code::NOT_READY);
    assert_eq!(d.data.expect("data")["reason"], "Needs a saved game");
    assert_eq!(rpc_code(bad_action), code::INVALID_PARAMS);
    assert!(
        unfocusable_ok,
        "a disabled element with a reason can be focused to read it"
    );
}
