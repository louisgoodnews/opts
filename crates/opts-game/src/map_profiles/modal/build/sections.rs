//! The sections of the map configuration modal.

use super::*;

mod profile_create;
pub(super) use profile_create::{choice_list, create_section, profile_section};

/// Title, map folder, Chrono entries and the status.
pub(super) fn map_header(
    p: &mut ChildSpawnerCommands,
    loc: &Localization,
    view: &ProfileView,
    status: String,
) {
    text(p, loc.t("map-config-title"), 22.0, ColorRole::Primary);
    text(p, view.map_dir_name.clone(), 15.0, ColorRole::OnSurface);
    let chrono = if view.chrono_entries.is_empty() {
        loc.t("map-config-chrono-none")
    } else {
        let newest = view.chrono_entries.last().cloned().unwrap_or_default();
        let others = view.chrono_entries.len() - 1;
        if others == 0 {
            loc.t_args("map-config-chrono-one", &[arg("newest", newest)])
        } else {
            loc.t_args(
                "map-config-chrono",
                &[arg("newest", newest), arg("others", others)],
            )
        }
    };
    text(p, chrono, 13.0, ColorRole::OnSurfaceVariant);
    if !status.is_empty() {
        text(p, status, 13.0, ColorRole::Primary);
    }
}

/// The load profile of the map.
pub(super) fn load_section(p: &mut ChildSpawnerCommands, loc: &Localization, view: &ProfileView) {
    text(p, loc.t("map-config-load"), 16.0, ColorRole::Primary);
    if view.load.tile_count == 0 {
        text(
            p,
            loc.t("map-config-counting"),
            13.0,
            ColorRole::OnSurfaceVariant,
        );
    } else {
        let l = &view.load;
        let mut line = loc.t_args(
            "map-config-load-counts",
            &[
                arg("objects", l.object_count),
                arg("splines", l.spline_count),
                arg("tiles", l.tile_count),
                arg("area", format!("{:.1}", l.map_size_km2)),
                arg("pool", l.ai_pool_vehicles),
            ],
        );
        if view.counting {
            line.push(' ');
            line.push_str(&loc.t("map-config-load-estimate"));
        }
        text(p, line, 13.0, ColorRole::OnSurface);
        let last = match (l.avg_frame_time_ms, l.p95_frame_time_ms) {
            (Some(a), Some(b)) => loc.t_args(
                "map-config-last",
                &[arg("avg", format!("{a:.0}")), arg("p95", format!("{b:.0}"))],
            ),
            _ => loc.t("map-config-never"),
        };
        text(p, last, 13.0, ColorRole::OnSurfaceVariant);
    }
}

/// What the profile suggests.
pub(super) fn advice_section(p: &mut ChildSpawnerCommands, loc: &Localization, view: &ProfileView) {
    text(p, loc.t("map-config-advice"), 16.0, ColorRole::Primary);
    if view.advice.is_empty() {
        text(
            p,
            loc.t("map-config-advice-none"),
            13.0,
            ColorRole::OnSurfaceVariant,
        );
    }
    for a in &view.advice {
        let role = if a.severity == super::super::super::advice::Severity::Warning {
            ColorRole::Warning
        } else {
            ColorRole::OnSurface
        };
        text(p, loc.t(advice_key(a.id)), 13.0, role);
        if let Some(preset) = a.preset {
            let name = view
                .presets
                .iter()
                .find(|x| x.based_on.as_deref() == Some(preset))
                .map_or(preset.to_string(), |x| x.name.clone());
            text(
                p,
                loc.t_args("map-config-preset-hint", &[arg("preset", name)]),
                12.0,
                ColorRole::OnSurfaceVariant,
            );
        }
    }
}


/// *Apply*, *Apply and keep*, *Delete* and *Cancel*.
pub(super) fn apply_buttons(p: &mut ChildSpawnerCommands, loc: &Localization) {
    p.spawn(Node {
        flex_wrap: FlexWrap::Wrap,
        column_gap: Val::Px(8.0),
        row_gap: Val::Px(6.0),
        ..default()
    })
    .with_children(|row| {
        button(
            row,
            &loc.t("map-config-apply"),
            true,
            Action::Apply { keep: false },
            "map-config.apply".to_string(),
        );
        button(
            row,
            &loc.t("map-config-apply-keep"),
            false,
            Action::Apply { keep: true },
            "map-config.apply-keep".to_string(),
        );
        button(
            row,
            &loc.t("map-config-delete"),
            false,
            Action::Delete,
            "map-config.delete".to_string(),
        );
        button(
            row,
            &loc.t("map-config-cancel"),
            false,
            Action::Close,
            "map-config.cancel".to_string(),
        );
    });
}
