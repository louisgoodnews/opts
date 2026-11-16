//! The profile section, its choice list and the create-profile section of the map modal.

use super::super::*;

pub(in crate::map_profiles::modal::build) fn profile_section(
    p: &mut ChildSpawnerCommands,
    loc: &Localization,
    view: &ProfileView,
) {
    text(p, loc.t("map-config-profile"), 16.0, ColorRole::Primary);
    let resolved = match (&view.resolved.profile, view.resolved.source) {
        (Some(pr), ProfileSource::Chrono) => {
            loc.t_args("map-config-resolved-chrono", &[arg("name", &pr.name)])
        }
        (Some(pr), _) => loc.t_args("map-config-resolved-map", &[arg("name", &pr.name)]),
        (None, _) => loc.t("map-config-resolved-global"),
    };
    text(p, resolved, 13.0, ColorRole::OnSurfaceVariant);
    match &view.picked {
        Some(Some(n)) => text(
            p,
            loc.t_args("map-config-picked", &[arg("name", n)]),
            13.0,
            ColorRole::Primary,
        ),
        Some(None) => text(
            p,
            loc.t("map-config-picked-global"),
            13.0,
            ColorRole::Primary,
        ),
        None => {}
    }
}

/// The profiles to choose from.
pub(in crate::map_profiles::modal::build) fn choice_list(
    p: &mut ChildSpawnerCommands,
    loc: &Localization,
    view: &ProfileView,
    selected: Option<&Choice>,
) {
    let mut choices = vec![Choice::Global];
    choices.extend(
        view.presets
            .iter()
            .filter_map(|x| x.based_on.clone().map(Choice::Preset)),
    );
    choices.extend(
        view.saved
            .iter()
            .map(|(_, x)| Choice::Saved(x.name.clone())),
    );
    p.spawn(Node {
        flex_direction: FlexDirection::Column,
        row_gap: Val::Px(4.0),
        width: Val::Percent(100.0),
        flex_grow: 1.0,
        min_height: Val::Px(0.0),
        overflow: Overflow::scroll_y(),
        ..default()
    })
    .with_children(|list| {
        for (i, c) in choices.iter().enumerate() {
            let label = choice_label(loc, view, c);
            button(
                list,
                &label,
                selected == Some(c),
                Action::Select(c.clone()),
                format!("map-config.choice.{i}"),
            );
        }
    });
}

/// The name field and the *Create* buttons.
pub(in crate::map_profiles::modal::build) fn create_section(
    p: &mut ChildSpawnerCommands,
    loc: &Localization,
    view: &ProfileView,
    name: String,
) {
    text(
        p,
        loc.t("map-config-name"),
        13.0,
        ColorRole::OnSurfaceVariant,
    );
    let field = TextFieldBuilder::outlined(loc.t("map-config-name"))
        .with_text(name)
        .with_max_characters(60)
        .spawn_child(p);
    p.commands()
        .entity(field)
        .insert((NameField, UiId::new("map-config.name")));
    p.spawn(Node {
        flex_wrap: FlexWrap::Wrap,
        column_gap: Val::Px(8.0),
        row_gap: Val::Px(6.0),
        ..default()
    })
    .with_children(|row| {
        button(
            row,
            &loc.t("map-config-create"),
            false,
            Action::Create { chrono: false },
            "map-config.create".to_string(),
        );
        if view.key.chrono_id != DEFAULT_CHRONO {
            button(
                row,
                &loc.t("map-config-create-chrono"),
                false,
                Action::Create { chrono: true },
                "map-config.create-chrono".to_string(),
            );
        }
    });
}
