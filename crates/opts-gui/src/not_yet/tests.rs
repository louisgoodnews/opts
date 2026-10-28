//! Unit tests for `super` (`not_yet.rs`).

use super::*;
use std::path::PathBuf;

fn docs() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../docs/src")
}

#[test]
fn reference_page_is_up_to_date() {
    let path = docs().join("reference/not-available-yet.md");
    let want = reference_markdown();
    if std::env::var_os("UPDATE_NOT_YET").is_some() {
        std::fs::write(&path, &want).expect("write reference page");
    }
    // A Windows checkout (`core.autocrlf`) has CRLF line ends; the page is compared by content.
    let have = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("read the reference page: {e}"))
        .replace("\r\n", "\n");
    assert_eq!(
        have, want,
        "docs/src/reference/not-available-yet.md is stale: run `UPDATE_NOT_YET=1 cargo test -p opts-gui not_yet`"
    );
}

#[test]
fn places_have_unique_ids_and_known_open_milestones() {
    let mut ids = std::collections::HashSet::new();
    for p in PLACES {
        assert!(ids.insert(p.id), "duplicate place id {}", p.id);
        let file = std::fs::read_dir(docs().join("roadmap"))
            .expect("roadmap dir")
            .flatten()
            .map(|d| d.path().join(format!("{}.md", p.milestone)))
            .find(|f| f.exists())
            .unwrap_or_else(|| panic!("{} names unknown milestone {}", p.id, p.milestone));
        let text = std::fs::read_to_string(file).expect("milestone page");
        let status = text
            .lines()
            .find(|l| l.contains("**Status**"))
            .unwrap_or_default();
        assert!(
            !status.contains("🟢 Complete"),
            "{} waits for {} which is marked complete: remove the marker, the feature exists",
            p.id,
            p.milestone
        );
    }
}

#[test]
fn availability_applies_and_clears() {
    let mut app = crate::testing::headless_app();
    let e = app.world_mut().spawn_empty().id();
    {
        let mut commands = app.world_mut().commands();
        Availability::NotYet(&NAV_LIBRARY).apply(&mut commands.entity(e));
    }
    app.update();
    app.update();
    let w = app.world();
    assert!(w.get::<Disabled>(e).is_some());
    assert_eq!(
        w.get::<DisabledReason>(e).map(|r| r.0.to_string()),
        Some("Feature not available yet! (M-0062)".to_string())
    );
    {
        let mut commands = app.world_mut().commands();
        Availability::Enabled.apply(&mut commands.entity(e));
    }
    app.update();
    assert!(app.world().get::<Disabled>(e).is_none());
    assert!(app.world().get::<NotAvailableYet>(e).is_none());
}

#[test]
fn the_label_is_red_text_with_the_exact_wording() {
    let mut app = crate::testing::headless_app();
    let button = {
        let mut commands = app.world_mut().commands();
        crate::components::button::ButtonBuilder::tonal("Load").spawn(&mut commands)
    };
    app.update();
    app.world_mut()
        .entity_mut(button)
        .insert(NotAvailableYet::new(&NAV_LIBRARY));
    app.update();
    app.update();
    let labels: Vec<String> = app
        .world_mut()
        .query_filtered::<(&Text, &ForegroundRole), With<NotYetLabel>>()
        .iter(app.world())
        .map(|(t, r)| {
            assert_eq!(r.0, ColorRole::Error);
            t.0.clone()
        })
        .collect();
    assert_eq!(labels, vec!["Feature not available yet!".to_string()]);
}
