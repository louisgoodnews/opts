//! Every control usable; key rebinding; one descriptor end to end.

use super::content::outcome_text;
use super::*;

mod keyboard_tests;
mod rebind_tests;

/// The acceptance criterion of M-0063: one descriptor, and the setting is in the UI, in the saved
/// file, in the control protocol and in the generated reference page, with no other edit.
#[test]
fn one_descriptor_reaches_ui_persistence_protocol_and_reference() {
    use crate::settings_registry::{Category, SettingDescriptor, SettingsAppExt};
    const FIXTURE: SettingDescriptor = SettingDescriptor::new(
        "fixture.wheel_sensitivity",
        SettingKind::Int {
            min: 1,
            max: 10,
            step: 1,
        },
        "5",
        Category::Controls,
        5,
        "fixture-label",
        "fixture-desc",
    );
    let (mut app, _, db) = page_app("acceptance", "controls");
    app.register_setting(FIXTURE);
    {
        let mut loc = app
            .world_mut()
            .resource_mut::<opts_gui::l10n::Localization>();
        loc.extend(
            "en",
            "fixture-label = Wheel sensitivity\nfixture-desc = How strongly the wheel steers.\n",
        );
        loc.extend(
            "de",
            "fixture-label = Lenkradempfindlichkeit\nfixture-desc = Wie stark das Lenkrad lenkt.\n",
        );
    }
    app.world_mut().resource_mut::<SettingsPageState>().dirty = true;
    for _ in 0..3 {
        app.update();
    }
    // UI: the row exists and its control works.
    let up =
        by_id(&mut app, "settings/controls/fixture.wheel_sensitivity/up").expect("row in the UI");
    click(&mut app, up);
    app.update();
    app.update();
    assert_eq!(text(&app, "fixture.wheel_sensitivity"), "6");
    // Persistence.
    assert_eq!(
        stored(&db, "fixture.wheel_sensitivity").as_deref(),
        Some("6")
    );
    // Protocol.
    let listed = outcome_text(crate::settings_registry::protocol::list(
        app.world_mut(),
        &serde_json::json!({ "category": "controls" }),
    ));
    assert!(listed.contains("fixture.wheel_sensitivity"), "{listed}");
    // Generated reference page.
    let md = crate::settings_registry::reference::markdown_with(
        app.world().resource::<SettingsRegistry>(),
        app.world().resource::<opts_gui::l10n::Localization>(),
    );
    assert!(
        md.contains("`fixture.wheel_sensitivity`") && md.contains("Wheel sensitivity"),
        "{}",
        md.lines()
            .filter(|l| l.contains("fixture"))
            .collect::<Vec<_>>()
            .join("\n")
    );
    // Search.
    let loc = app
        .world()
        .resource::<opts_gui::l10n::Localization>()
        .clone();
    let found = crate::settings_registry::search::search(
        app.world().resource::<SettingsRegistry>(),
        &loc,
        "wheel",
    );
    assert!(found.iter().any(|d| d.key == "fixture.wheel_sensitivity"));
}
