use super::*;

#[test]
fn ui_scale_from_75_to_200_percent_keeps_text_inside_at_three_window_sizes() {
    let mut findings = Vec::new();
    for (w, h) in [(1024, 640), (1280, 720), (1920, 1080)] {
        for scale in [0.75_f32, 1.0, 1.25, 1.5, 2.0] {
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
    assert!(
        findings.is_empty(),
        "{} findings:\n{}",
        findings.len(),
        findings.join("\n")
    );
}

#[test]
fn the_base_text_size_from_12_to_24_px_keeps_text_inside_and_never_below_12_px() {
    use opts_gui::fonts::UiTypography;
    let mut findings = Vec::new();
    for (w, h) in [(1024, 640), (1920, 1080)] {
        for base in [12.0_f32, 14.0, 20.0, 24.0] {
            for screen in Screen::ALL {
                let mut app = build(screen, w, h, ThemeKind::Dark, AccentPreset::default());
                app.insert_resource(UiTypography {
                    family: String::new(),
                    base_px: base,
                });
                for _ in 0..6 {
                    app.update();
                }
                let what = format!("{} {w}x{h} at {base} px", screen.name());
                for f in rules::text_overflow(app.world_mut(), Vec2::new(w as f32, h as f32)) {
                    findings.push(format!("{what}: {f}"));
                }
                for f in rules::small_text(app.world_mut()) {
                    findings.push(format!("{what}: {f}"));
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

#[test]
fn the_language_can_be_chosen_on_every_first_run_step_and_back() {
    use opts_gui::interaction::Activated;
    use opts_gui::l10n::Localization;
    for screen in [Screen::FirstRunChoose, Screen::Welcome] {
        let mut app = build(screen, 1280, 720, ThemeKind::Dark, AccentPreset::default());
        let id = opts_gui::ui_tree::snapshot(app.world_mut())
            .flatten()
            .iter()
            .find(|n| n.id.ends_with("language"))
            .map(|n| n.id.clone())
            .unwrap_or_else(|| panic!("{}: no language button", screen.name()));
        let before = app.world().resource::<Localization>().locale().to_string();
        assert_eq!(before, "en");
        for expected in ["de", "en"] {
            let e = opts_gui::ui_tree::find_entity(app.world_mut(), &id).expect("button");
            app.world_mut().write_message(Activated(e));
            for _ in 0..4 {
                app.update();
            }
            assert_eq!(
                app.world().resource::<Localization>().locale(),
                expected,
                "{}",
                screen.name()
            );
            // the dialog is still open (the choice keeps it open)
            assert!(
                opts_gui::ui_tree::find_entity(app.world_mut(), &id).is_some(),
                "{}: the dialog closed",
                screen.name()
            );
        }
    }
}
