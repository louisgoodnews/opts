use super::*;

fn selection() -> RouteSelection {
    RouteSelection {
        route_code: 1201,
        line: 12,
        course: 1,
        terminus_code: 402,
        line_string: "12".into(),
        route_string: "x".into(),
    }
}

/// A stand-in printer: a trigger per digit and the confirm keys; the mode and the login follow
/// the keys the way the real script's do.
fn printer() -> ScriptEngine {
    let mut src = String::new();
    for d in 0..10 {
        src +=
            &format!("{{trigger:drucker_num{d}}}\n(L.L.typed) 10 * {d} + (S.L.typed)\n{{end}}\n");
    }
    src += "{trigger:drucker_anmelden_enter}\n(L.L.enters) 1 + (S.L.enters)\n{end}\n";
    src += "{trigger:drucker_tab}\n1 (S.L.tabbed)\n{end}\n";
    src += "{trigger:drucker_lk_bestaetigen}\n1 (S.L.confirmed)\n{end}\n";
    let (mut e, diags) = ScriptEngine::from_sources(&[("t.osc", &src)]);
    assert!(diags.is_empty(), "{diags:?}");
    e.consts_mut().set_scalar("Drucker_Fahrernummer", 21052.0);
    e.consts_mut().set_scalar("Drucker_Fahrerpin", 15067.0);
    e
}

#[test]
fn a_printer_selection_waits_for_the_boot_logs_in_and_keys_line_and_course() {
    let mut e = printer();
    let mut q = KeyQueue::default();
    q.type_selection(&e, &selection())
        .expect("a printer keypad");
    // The printer is still booting: nothing is pressed, and nothing is lost.
    for _ in 0..5 {
        assert!(!q.press_next(&mut e, 0.1));
    }
    assert_eq!(e.get("typed"), 0.0);
    e.set("drucker_mode", 1.0);
    let mut pressed = 0;
    for _ in 0..40 {
        if q.press_next(&mut e, 0.016) {
            pressed += 1;
        }
        if e.get("enters") >= 2.0 {
            // Logged in: the printer moves on to the line screen.
            e.set("drucker_mode", 2.0);
        }
    }
    assert!(!q.is_typing());
    // 5 + 5 login digits and 2 enters, 2 line digits, tab, 1 course digit, confirm.
    assert_eq!(pressed, 10 + 2 + 2 + 1 + 1 + 1);
    assert_eq!((e.get("tabbed"), e.get("confirmed")), (1.0, 1.0));
}

#[test]
fn a_logged_in_printer_goes_straight_to_the_line() {
    let mut e = printer();
    e.set("drucker_loggedin", 1.0);
    e.set("drucker_mode", 2.0);
    let mut q = KeyQueue::default();
    q.type_selection(&e, &selection()).expect("keypad");
    let mut pressed = 0;
    while q.is_typing() {
        pressed += usize::from(q.press_next(&mut e, 0.016));
    }
    assert_eq!(pressed, 2 + 1 + 1 + 1);
    assert_eq!(e.get("enters"), 0.0);
}

#[test]
fn a_wait_that_never_ends_drops_the_rest() {
    let mut e = printer();
    let mut q = KeyQueue::default();
    q.type_selection(&e, &selection()).expect("keypad");
    for _ in 0..500 {
        q.press_next(&mut e, 0.1);
    }
    assert!(!q.is_typing());
}

#[test]
fn a_vehicle_without_either_keypad_is_refused() {
    let (e, _) = ScriptEngine::from_sources(&[("t.osc", "{frame}\n{end}")]);
    let mut q = KeyQueue::default();
    assert!(matches!(
        q.type_selection(&e, &selection()),
        Err(RouteError::NoKeypad)
    ));
}
