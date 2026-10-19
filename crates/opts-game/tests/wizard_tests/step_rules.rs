//! Rules every step obeys: CI rules, keyboard, layout and sources.

use super::*;

#[test]
fn every_step_meets_the_ci_rules() {
    let mut app = wizard("rules");
    for id in [
        "wizard.stepper.where",
        "wizard.stepper.what",
        "wizard.stepper.when",
        "wizard.stepper.review",
    ] {
        press(&mut app, id);
        for _ in 0..4 {
            app.update();
        }
        let small = rules::small_targets(app.world_mut());
        assert!(small.is_empty(), "{id}: {}", small.join("\n"));
        let text = rules::small_text(app.world_mut());
        assert!(text.is_empty(), "{id}: {}", text.join("\n"));
        let dis = rules::disabled_without_reason(app.world_mut());
        assert!(dis.is_empty(), "{id}: {}", dis.join("\n"));
    }
    cleanup("rules");
}

#[test]
fn the_tree_names_the_controls_of_each_step() {
    let mut app = wizard("tree");
    let snap = snapshot(app.world_mut());
    for id in [
        "wizard.where.search",
        "wizard.where.list",
        "wizard.footer.next",
        "wizard.footer.back",
        "wizard.stepper.review",
    ] {
        assert!(snap.find(id).is_some(), "{id} missing");
    }
    let _ = FooterButton::Drive;
    cleanup("tree");
}

#[test]
fn every_step_is_fully_keyboard_operable() {
    let mut app = wizard("keys");
    for id in [
        "wizard.stepper.where",
        "wizard.stepper.what",
        "wizard.stepper.when",
        "wizard.stepper.review",
    ] {
        press(&mut app, id);
        let report = rules::keyboard_traversal(&mut app);
        assert!(
            report.missed.is_empty(),
            "{id}: never focused {:?}",
            report.missed
        );
        // Confirm on a control that changes the step can hide later controls of the walk
        // (the review's Drive); every other control must react.
        let inert: Vec<_> = report
            .inert
            .iter()
            .filter(|i| !i.contains("footer.drive"))
            .collect();
        assert!(inert.is_empty(), "{id}: Confirm did nothing on {inert:?}");
        // The step jumps (stepper buttons) change the screen; return to the step under test.
        press(&mut app, id);
    }
    cleanup("keys");
}

#[test]
fn the_date_and_time_block_fits_a_1024_window_at_100_and_150_percent() {
    for scale in [1.0_f32, 1.5] {
        let mut app = wizard("fit");
        {
            let mut q = app.world_mut().query::<&mut Window>();
            for mut w in q.iter_mut(app.world_mut()) {
                w.resolution.set(1024.0, 640.0);
            }
            app.world_mut().insert_resource(UiScale(scale));
        }
        // The time block sits at the left of step When and must fit the window; the date block
        // sits in the map step's detail pane and must fit that pane.
        for (stepper, name, in_window) in [
            ("wizard.stepper.where", "DateBlock", false),
            ("wizard.stepper.when", "TimeBlock", true),
        ] {
            press(&mut app, stepper);
            for _ in 0..4 {
                app.update();
            }
            let mut q = app.world_mut().query::<(
                Entity,
                &Name,
                &ComputedNode,
                &UiGlobalTransform,
                Option<&ChildOf>,
            )>();
            let rect = |c: &ComputedNode, t: &UiGlobalTransform| {
                (
                    c.size() * c.inverse_scale_factor,
                    t.translation * c.inverse_scale_factor,
                )
            };
            let (block, parent) = q
                .iter(app.world())
                .find(|(_, n, ..)| n.as_str() == name)
                .map(|(_, _, c, t, p)| (rect(c, t), p.map(ChildOf::parent)))
                .expect("the picker block");
            let limit = if in_window {
                1024.0 / scale
            } else {
                let parent = parent.expect("a parent");
                let (_, _, c, t, _) = q.get(app.world(), parent).expect("parent node");
                let (size, centre) = rect(c, t);
                centre.x + size.x / 2.0
            };
            let right = block.1.x + block.0.x / 2.0;
            assert!(
                right <= limit + 1.0,
                "{name} at scale {scale}: right edge {right} beyond {limit}"
            );
            assert!(block.0.x <= 560.0 + 1.0);
        }
        cleanup("fit");
    }
}

/// Source text without its tests.
fn source_of(path: &str) -> String {
    let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(path);
    let text = std::fs::read_to_string(p).unwrap_or_default();
    text.split("#[cfg(test)]").next().unwrap_or("").to_string()
}

#[test]
fn the_wizard_and_loading_modules_hold_no_hard_coded_colours_or_visible_literals() {
    let mut files: Vec<String> = std::fs::read_dir(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/new_game_screen"),
    )
    .expect("module")
    .flatten()
    .map(|e| format!("src/new_game_screen/{}", e.file_name().to_string_lossy()))
    .filter(|f| f.ends_with(".rs"))
    .collect();
    files.extend(
        [
            "src/in_game/loading_ui.rs",
            "src/in_game/loading_pipeline.rs",
            "src/in_game/load_report.rs",
        ]
        .map(String::from),
    );
    let mut bad = Vec::new();
    for f in &files {
        for (n, line) in source_of(f).lines().enumerate() {
            let code = line.split("//").next().unwrap_or("");
            for banned in [
                "Color::srgb",
                "Color::rgb",
                "Color::hsl",
                "Color::BLACK",
                "Color::WHITE",
                "Srgba::",
                "LinearRgba::",
            ] {
                if code.contains(banned) {
                    bad.push(format!("{f}:{}: {}", n + 1, code.trim()));
                }
            }
            if let Some(i) = code.find("Text::new(\"") {
                let lit = &code[i + 11..];
                if lit
                    .chars()
                    .take_while(|c| *c != '"')
                    .filter(|c| c.is_alphabetic())
                    .count()
                    >= 3
                {
                    bad.push(format!("{f}:{}: literal text {}", n + 1, code.trim()));
                }
            }
        }
    }
    assert!(bad.is_empty(), "{}", bad.join("\n"));
}
