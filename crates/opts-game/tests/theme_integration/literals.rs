//! The screens rebuilt later must carry no literal colours: they use role tokens.

/// The screens rebuilt later by other milestones must not carry literal colours: they use role
/// tokens (and the instrument token set for the HUD readouts).
#[test]
fn loading_screen_hud_and_shell_screens_carry_no_literal_colours() {
    for (name, source) in [
        (
            "loading_ui.rs",
            include_str!("../../src/in_game/loading_ui.rs"),
        ),
        ("hud.rs", include_str!("../../src/in_game/hud.rs")),
        ("home/mod.rs", include_str!("../../src/home/mod.rs")),
        ("help/mod.rs", include_str!("../../src/help/mod.rs")),
        ("shell/frame.rs", include_str!("../../src/shell/frame.rs")),
        ("shell/dialog.rs", include_str!("../../src/shell/dialog.rs")),
        (
            "instruments.rs",
            include_str!("../../src/in_game/instruments.rs"),
        ),
    ] {
        for needle in [
            "Color::srgb(",
            "Color::srgba(",
            "Color::WHITE",
            "Color::BLACK",
            "Color::hsl",
            "Srgba::",
            "LinearRgba",
        ] {
            // test modules may use literals; only the code before them is audited
            let code = source.split("#[cfg(test)]").next().unwrap_or(source);
            assert!(
                !code.contains(needle),
                "{name} contains the literal colour `{needle}`; use a role or an instrument token"
            );
        }
    }
}
