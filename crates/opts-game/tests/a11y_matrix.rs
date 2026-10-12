//! M-0065: the automatic accessibility rules over every shell screen, in every theme and accent,
//! at three window sizes, and in the pseudo-locale and German (text expansion).

mod common;

use bevy::prelude::*;
use common::screens::{build, set_locale, set_theme, Screen};
use opts_gui::testing::rules;
use opts_gui::theme::mode::ThemeKind;
use opts_gui::theme::{AccentPreset, MaterialTheme};

const KINDS: [ThemeKind; 3] = [ThemeKind::Dark, ThemeKind::Light, ThemeKind::HighContrast];

#[test]
fn every_screen_has_contrast_in_every_theme_and_accent() {
    let mut measured = 0;
    for screen in Screen::ALL {
        let mut app = build(screen, 1280, 720, ThemeKind::Dark, AccentPreset::default());
        for kind in KINDS {
            for accent in AccentPreset::ALL {
                set_theme(&mut app, kind, accent);
                let scheme = MaterialTheme::new(kind, accent).scheme;
                // the theme really in force is the one being measured
                let live = app.world().resource::<MaterialTheme>();
                assert_eq!(live.kind(), kind);
                assert_eq!(live.scheme, scheme, "{kind:?}/{accent:?} not applied");
                let r = rules::text_contrast(app.world_mut(), &scheme);
                measured += r.checked;
                assert!(
                    r.violations.is_empty(),
                    "{} / {kind:?} / {accent:?}:\n{}",
                    screen.name(),
                    r.violations.join("\n")
                );
            }
        }
    }
    assert!(measured > 1500, "only {measured} pairs measured");
}

#[test]
fn every_screen_holds_the_size_rules_at_three_window_sizes() {
    for (w, h) in [(1024, 640), (1280, 720), (1920, 1080)] {
        for screen in Screen::ALL {
            let mut app = build(screen, w, h, ThemeKind::Dark, AccentPreset::default());
            let what = format!("{} at {w}x{h}", screen.name());
            let small = rules::small_targets(app.world_mut());
            assert!(small.is_empty(), "{what}: {}", small.join("\n"));
            let text = rules::small_text(app.world_mut());
            assert!(text.is_empty(), "{what}: {}", text.join("\n"));
            let why = rules::disabled_without_reason(app.world_mut());
            assert!(why.is_empty(), "{what}: {}", why.join("\n"));
        }
    }
}

#[test]
fn text_survives_the_pseudo_locale_and_german_at_three_window_sizes() {
    let mut findings = Vec::new();
    for (w, h) in [(1024, 640), (1280, 720), (1920, 1080)] {
        for screen in Screen::ALL {
            let mut app = build(screen, w, h, ThemeKind::Dark, AccentPreset::default());
            for locale in ["en", "de", "en-XA"] {
                set_locale(&mut app, locale);
                for f in rules::text_overflow(app.world_mut(), Vec2::new(w as f32, h as f32)) {
                    findings.push(format!("{} {w}x{h} {locale}: {f}", screen.name()));
                }
            }
        }
    }
    assert!(findings.is_empty(), "{} findings:\n{}", findings.len(), findings.join("\n"));
}

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
    assert!(findings.is_empty(), "not reachable by directions alone:\n{}", findings.join("\n"));
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
    for screen in [
        Screen::FirstRunFound,
        Screen::FirstRunChoose,
        Screen::Welcome,
    ] {
        let mut app = build(screen, 1280, 720, ThemeKind::Dark, AccentPreset::default());
        let modals = app
            .world_mut()
            .query::<&opts_gui::components::modal::ModalRoot>()
            .iter(app.world())
            .count();
        assert!(modals >= 1, "{} shows no dialog", screen.name());
    }
    // the session summary is a card on Home, not a dialog
    let mut app = build(Screen::Summary, 1280, 720, ThemeKind::Dark, AccentPreset::default());
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
        assert!(focusable >= 2, "{}: {focusable} focusable elements", screen.name());
    }
}

#[test]
fn ui_scale_from_75_to_200_percent_keeps_text_inside_at_three_window_sizes() {
    let mut findings = Vec::new();
    for (w, h) in [(1024, 640), (1280, 720), (1920, 1080)] {
        for scale in [0.75_f32, 1.0, 1.5, 2.0] {
            for screen in Screen::ALL {
                let mut app = build(screen, w, h, ThemeKind::Dark, AccentPreset::default());
                app.insert_resource(bevy::ui::UiScale(scale));
                for _ in 0..6 {
                    app.update();
                }
                let logical = Vec2::new(w as f32 / scale, h as f32 / scale);
                let what = format!("{} {w}x{h} at {}%", screen.name(), (scale * 100.0) as u32);
                for f in rules::text_overflow(app.world_mut(), logical) {
                    findings.push(format!("{what}: {f}"));
                }
                for f in rules::small_targets(app.world_mut()) {
                    findings.push(format!("{what}: {f}"));
                }
            }
        }
    }
    assert!(findings.is_empty(), "{} findings:\n{}", findings.len(), findings.join("\n"));
}
