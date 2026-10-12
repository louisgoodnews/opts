//! The generated reference page `docs/src/reference/settings.md`.

use super::{Apply, Available, SettingKind, SettingsRegistry};
use opts_gui::l10n::Localization;

fn kind_text(k: &SettingKind) -> String {
    match k {
        SettingKind::Bool => "on / off".into(),
        SettingKind::Enum(o) => o
            .iter()
            .map(|o| format!("`{}`", o.value))
            .collect::<Vec<_>>()
            .join(", "),
        SettingKind::Int { min, max, .. } => format!("whole number {min} to {max}"),
        SettingKind::Float { min, max, .. } => format!("number {min} to {max}"),
        SettingKind::Text => "text".into(),
        SettingKind::Path => "path".into(),
        SettingKind::Time => "time `HH:MM`".into(),
        SettingKind::Binding => "input binding".into(),
        SettingKind::Action => "action (no stored value)".into(),
    }
}

/// The reference page for `registry`, in English.
#[must_use]
pub fn markdown(registry: &SettingsRegistry) -> String {
    let loc = Localization::new("en");
    let mut out = String::from(
        "<!-- Generated from the settings registry by the test `reference_page_is_up_to_date`; \
         regenerate with `UPDATE_SETTINGS_DOC=1 cargo test -p opts-game settings_registry`. -->\n\n\
         # Settings Reference\n\n\
         Every setting is declared once as a `SettingDescriptor` ([registry](../architecture/settings-registry.md)); \
         this page, the settings screen, the saved file and the control-protocol commands \
         `settings.list`, `settings.get`, `settings.set` and `settings.reset` all come from it. \
         Values are stored as text in `settings.db`; *Applies* says when a change takes effect.\n",
    );
    for cat in registry.categories() {
        out.push_str(&format!("\n## {}\n\n", loc.t(cat.label_key())));
        out.push_str("| Key | Name | Values | Default | Applies | Status | Description |\n| :--- | :--- | :--- | :--- | :--- | :--- | :--- |\n");
        for d in registry.in_category(cat) {
            let applies = match d.apply {
                Apply::Live => "at once".to_string(),
                Apply::OnRestart => "after restart".to_string(),
                Apply::WithRevert(s) => format!("at once, {s} s to confirm"),
            };
            let status = match d.available {
                Available::Always if d.hidden => "internal".to_string(),
                Available::Always => "available".to_string(),
                Available::Capability(c) => format!("needs {c}"),
                Available::NotYet(p) => format!("not available yet ({})", p.milestone),
            };
            let default = if d.default.is_empty() {
                "-".to_string()
            } else {
                format!("`{}`", d.default)
            };
            out.push_str(&format!(
                "| `{}` | {} | {} | {} | {} | {} | {} |\n",
                d.key,
                loc.t(d.label_key),
                kind_text(&d.kind),
                default,
                applies,
                status,
                loc.t(d.description_key)
            ));
        }
    }
    out
}
