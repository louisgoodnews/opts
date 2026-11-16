//! Accessibility matrix tests: theme switching, gamepad reach, accessible names and screen content.
use super::*;

#[test]
fn building_a_screen_in_a_theme_equals_switching_to_it() {
    // Colours come from the tokens when a node is created and again on a theme change; both
    // paths must give a passing screen (the second alone could hide a creation-time colour).
    for screen in Screen::ALL {
        for kind in KINDS {
            let mut app = build(screen, 1280, 720, kind, AccentPreset::default());
            let scheme = MaterialTheme::new(kind, AccentPreset::default()).scheme;
            assert_eq!(app.world().resource::<MaterialTheme>().scheme, scheme);
            let r = rules::text_contrast(app.world_mut(), &scheme);
            assert!(
                r.violations.is_empty(),
                "{} built in {kind:?}:\n{}",
                screen.name(),
                r.violations.join("\n")
            );
        }
    }
}

#[test]
fn a_gamepad_alone_reaches_every_control_of_every_screen() {
    let mut findings = Vec::new();
    for screen in Screen::ALL {
        let mut app = build(screen, 1280, 720, ThemeKind::Dark, AccentPreset::default());
        let t = rules::directional_traversal(&mut app);
        if std::env::var_os("OPTS_DEBUG_REACH").is_some() {
            eprintln!("{}: reached {:?}", screen.name(), t.reached);
        }
        for id in t.unreachable {
            findings.push(format!("{}: {id}", screen.name()));
        }
    }
    assert!(
        findings.is_empty(),
        "not reachable by directions alone:\n{}",
        findings.join("\n")
    );
}

#[test]
fn every_control_has_an_accessible_name_and_node() {
    let mut findings = Vec::new();
    for screen in Screen::ALL {
        for locale in ["en", "de"] {
            let mut app = build(screen, 1280, 720, ThemeKind::Dark, AccentPreset::default());
            // the accessibility sync is throttled to 10 per second of real time
            app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
                std::time::Duration::from_millis(150),
            ));
            set_locale(&mut app, locale);
            for _ in 0..3 {
                app.update();
            }
            for f in rules::unnamed_controls(app.world_mut())
                .into_iter()
                .chain(rules::missing_accessibility_nodes(app.world_mut()))
            {
                findings.push(format!("{} {locale}: {f}", screen.name()));
            }
        }
    }
    assert!(findings.is_empty(), "{}", findings.join("\n"));
}

#[test]
fn the_dialog_and_summary_screens_really_show_their_content() {
    for screen in [Screen::FirstRunChoose, Screen::Welcome] {
        let mut app = build(screen, 1280, 720, ThemeKind::Dark, AccentPreset::default());
        let modals = app
            .world_mut()
            .query::<&opts_gui::components::modal::ModalRoot>()
            .iter(app.world())
            .count();
        assert!(modals >= 1, "{} shows no dialog", screen.name());
    }
    // the session summary is a card on Home, not a dialog
    let mut app = build(
        Screen::Summary,
        1280,
        720,
        ThemeKind::Dark,
        AccentPreset::default(),
    );
    let cards = app
        .world_mut()
        .query::<&opts_game::session_summary::SummaryCard>()
        .iter(app.world())
        .count();
    assert_eq!(cards, 1, "the summary card is missing");
}

#[test]
fn every_screen_has_something_to_operate() {
    for screen in Screen::ALL {
        let mut app = build(screen, 1280, 720, ThemeKind::Dark, AccentPreset::default());
        let focusable = opts_gui::ui_tree::snapshot(app.world_mut())
            .flatten()
            .iter()
            .filter(|n| n.focusable && n.state.visible)
            .count();
        assert!(
            focusable >= 2,
            "{}: {focusable} focusable elements",
            screen.name()
        );
    }
}
