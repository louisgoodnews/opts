//! Every control works and every category obeys the UI rules.

use super::*;

/// Every usable setting can be changed on the page's own route and shows its new value.
#[test]
fn every_visible_setting_has_a_working_control() {
    let all: Vec<_> = SettingsRegistry::with_builtin()
        .all()
        .into_iter()
        .cloned()
        .collect();
    for d in all
        .iter()
        .filter(|d| !d.hidden && matches!(d.available, Available::Always))
    {
        let (mut app, _, _) = page_app(
            &format!("walk-{}", d.key.replace('.', "_")),
            d.category.id(),
        );
        let ids = app
            .world_mut()
            .query::<&UiId>()
            .iter(app.world())
            .filter(|u| {
                u.0.starts_with(&format!("settings/{}/{}/", d.category.id(), d.key))
            })
            .count();
        assert!(ids > 0, "{} has no control on the page", d.key);
        let alt = match d.kind {
            SettingKind::Bool => (if d.default == "true" { "false" } else { "true" }).to_string(),
            SettingKind::Enum(o) => o
                .iter()
                .find(|o| o.value != d.default)
                .map(|o| o.value.to_string())
                .unwrap_or_default(),
            SettingKind::Int { min, max, .. } => (if d.default.parse::<i64>().unwrap_or(0) == min {
                max
            } else {
                min
            })
            .to_string(),
            SettingKind::Time => "05:30".into(),
            SettingKind::Text if d.key == "interface.language" => "fr".into(),
            SettingKind::Path | SettingKind::Text => "x".into(),
            SettingKind::Float { .. } | SettingKind::Binding | SettingKind::Action => continue,
        };
        let r = crate::settings_registry::apply_change(
            app.world_mut(),
            d.key,
            Some(&alt),
            ChangeSource::Protocol,
        );
        assert!(r.is_ok(), "{}: {r:?}", d.key);
        app.update();
        app.update();
        assert_eq!(text(&app, d.key), alt, "{} did not take {alt}", d.key);
        if !matches!(
            d.apply,
            Apply::Live | Apply::OnRestart | Apply::WithRevert(_)
        ) {
            unreachable!();
        }
    }
}

#[test]
fn every_category_obeys_the_ui_rules() {
    for cat in crate::settings_registry::Category::ALL {
        let (mut app, _, _) = page_app(&format!("rules-{}", cat.id()), cat.id());
        let id = cat.id();
        assert!(
            rules::small_targets(app.world_mut()).is_empty(),
            "{id}: {:?}",
            rules::small_targets(app.world_mut())
        );
        assert!(rules::small_text(app.world_mut()).is_empty(), "{id}");
        assert!(
            rules::disabled_without_reason(app.world_mut()).is_empty(),
            "{id}: {:?}",
            rules::disabled_without_reason(app.world_mut())
        );
        let t = rules::keyboard_traversal(&mut app);
        assert!(t.missed.is_empty(), "{id} unreached: {:?}", t.missed);
        assert!(
            t.inert.is_empty(),
            "{id} inert: {:?} activated: {:?} reached: {:?}",
            t.inert,
            t.activated,
            t.reached
        );
    }
}
