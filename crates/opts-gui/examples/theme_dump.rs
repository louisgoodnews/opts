//! Prints the colour tables of the design-system document (`gui-design-system.md`, section 3.1)
//! straight from the code, so the document cannot drift: `cargo run -p opts-gui --example
//! theme_dump`. Also prints the contrast summary of every theme and accent preset.

use opts_gui::theme::{
    accent::AccentPreset,
    audit::{audit, rules},
    contrast::{contrast_ratio, to_hex},
    ColorRole, ColorScheme, ThemeKind,
};

fn main() {
    let kinds = [ThemeKind::Dark, ThemeKind::Light, ThemeKind::HighContrast];
    let schemes: Vec<ColorScheme> = kinds
        .iter()
        .map(|&k| ColorScheme::for_kind(k, AccentPreset::default().color()))
        .collect();
    println!("| Role | Dark | Light | High contrast |");
    println!("| :--- | :--- | :--- | :--- |");
    for &role in ColorRole::all() {
        let cells: Vec<String> = schemes
            .iter()
            .map(|s| format!("`{}`", to_hex(s.get(role))))
            .collect();
        println!("| `{role:?}` | {} |", cells.join(" | "));
    }
    println!();
    println!(
        "| Theme | Accent | Rules checked | Violations | Lowest text pair | Lowest control pair |"
    );
    println!("| :--- | :--- | ---: | ---: | :--- | :--- |");
    for kind in kinds {
        for preset in AccentPreset::ALL {
            let s = ColorScheme::for_kind(kind, preset.color());
            let mut low_text = (f32::MAX, String::new());
            let mut low_ctl = (f32::MAX, String::new());
            for r in rules() {
                let ratio = contrast_ratio(s.get(r.fg), s.get(r.bg));
                let slot = match r.kind {
                    opts_gui::theme::audit::RuleKind::Text => &mut low_text,
                    opts_gui::theme::audit::RuleKind::Control => &mut low_ctl,
                };
                if ratio < slot.0 {
                    *slot = (ratio, format!("{:?} on {:?}", r.fg, r.bg));
                }
            }
            println!(
                "| {kind:?} | {} | {} | {} | {:.2} ({}) | {:.2} ({}) |",
                preset.key(),
                rules().len(),
                audit(&s).len(),
                low_text.0,
                low_text.1,
                low_ctl.0,
                low_ctl.1
            );
        }
    }
}
