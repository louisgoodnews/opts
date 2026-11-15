use super::*;

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
    assert!(
        findings.is_empty(),
        "{} findings:\n{}",
        findings.len(),
        findings.join("\n")
    );
}
