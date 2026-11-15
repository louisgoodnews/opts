//! Unit tests of `toast_stack`.

use super::*;
use crate::components::toast::{ToastBuilder, ToastSeverity};

#[test]
fn at_most_three_are_visible_and_the_newest_sits_lowest_without_overlap() {
    let s = slots(&[48.0, 60.0, 48.0, 48.0, 48.0]);
    assert_eq!(s[3], None);
    assert_eq!(s[4], None);
    let (a, b, c) = (s[0].unwrap(), s[1].unwrap(), s[2].unwrap());
    assert_eq!(c, MARGIN, "the newest visible toast is at the bottom");
    assert_eq!(b, c + 48.0 + GAP);
    assert_eq!(a, b + 60.0 + GAP, "heights are respected: no overlap");
}

/// The headless app already carries `OptsGuiPlugin`, which registers [`stack_toasts`]; adding
/// it a second time would run the merge twice per frame.
fn app() -> App {
    crate::testing::headless_app()
}

fn spawn(app: &mut App, text: &str, key: Option<&str>) -> Entity {
    let mut commands = app.world_mut().commands();
    let mut b = ToastBuilder::new(text).with_duration(30.0);
    if let Some(k) = key {
        b = b.with_dedupe_key(k);
    }
    b.spawn(&mut commands)
}

#[test]
fn a_burst_shows_three_and_queues_the_rest_until_a_slot_frees() {
    let mut app = app();
    let toasts: Vec<_> = (0..5)
        .map(|i| spawn(&mut app, &format!("t{i}"), None))
        .collect();
    app.update();
    app.update();
    let shown = |app: &App, e| app.world().get::<Node>(e).map(|n| n.display);
    let visible = toasts
        .iter()
        .filter(|e| shown(&app, **e) == Some(Display::Flex))
        .count();
    assert_eq!(visible, 3);
    assert!(app.world().get::<ToastQueued>(toasts[4]).is_some());
    // The first one expires: the next queued toast takes its place.
    app.world_mut().entity_mut(toasts[0]).despawn();
    app.update();
    app.update();
    assert_eq!(shown(&app, toasts[3]), Some(Display::Flex));
    assert!(app.world().get::<ToastQueued>(toasts[3]).is_none());
}

#[test]
fn identical_toasts_are_merged_with_a_counter() {
    let mut app = app();
    let a = spawn(&mut app, "Saved", Some("saved"));
    app.update();
    let _b = spawn(&mut app, "Saved", Some("saved"));
    let _c = spawn(&mut app, "Saved", Some("saved"));
    app.update();
    app.update();
    let count = app
        .world_mut()
        .query::<&M3Toast>()
        .iter(app.world())
        .count();
    assert_eq!(count, 1, "one toast stands for all three");
    assert_eq!(app.world().get::<ToastCount>(a).map(|c| c.0), Some(3));
    let text = app
        .world_mut()
        .query::<&Text>()
        .iter(app.world())
        .map(|t| t.0.clone())
        .find(|t| t.starts_with("Saved"));
    assert_eq!(text.as_deref(), Some("Saved ×3"));
}

#[test]
fn errors_and_warnings_stay_longer_than_information() {
    assert!(ToastSeverity::Error.default_seconds() > ToastSeverity::Info.default_seconds());
    let mut app = app();
    let mut commands = app.world_mut().commands();
    let e = ToastBuilder::new("x")
        .with_severity(ToastSeverity::Error)
        .spawn(&mut commands);
    app.update();
    let timer = app.world().get::<M3Toast>(e).and_then(|t| t.timer.clone());
    assert_eq!(timer.map(|t| t.duration().as_secs_f32()), Some(10.0));
}
