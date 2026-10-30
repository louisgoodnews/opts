//! The pieces of the configurations section: the name row, the buttons and the problem lines.

use super::*;

/// The name field and the *Save* button.
pub(super) fn name_row(
    commands: &mut Commands,
    loc: &Localization,
    library: &ConfigLibrary,
    setup: &NewGameSetup,
    vehicle: &VehicleItem,
) -> Entity {
    let name_row = commands
        .spawn(Node {
            flex_wrap: FlexWrap::Wrap,
            ..row(8.0)
        })
        .id();
    let text = if library.draft_name.is_empty() {
        default_name(vehicle, &setup.selected_livery)
    } else {
        library.draft_name.clone()
    };
    let field = TextFieldBuilder::outlined(loc.t("wiz-configs-name"))
        .with_text(text)
        .with_max_characters(60)
        .spawn(commands);
    commands
        .entity(field)
        .insert((ConfigNameField, UiId::new("wizard.what.configs.name")));
    // Room for a vehicle and a livery name.
    commands.entity(field).queue(|mut entity: EntityWorldMut| {
        if let Some(mut node) = entity.get_mut::<Node>() {
            node.min_width = Val::Px(360.0);
        }
    });
    let save = action_button(
        commands,
        "wiz-configs-save",
        Emphasis::Tonal,
        PlannerAction::SaveConfig,
        "wizard.what.configs.save".to_owned(),
    );
    commands.entity(name_row).add_children(&[field, save]);
    name_row
}

/// The *Favourite*, *Duplicate* and *Delete* buttons of a configuration (a second press deletes).
pub(super) fn row_buttons(
    commands: &mut Commands,
    library: &ConfigLibrary,
    c: &BusConfiguration,
    index: usize,
) -> [Entity; 3] {
    let favourite = action_button(
        commands,
        if c.favourite {
            "wiz-configs-unfavourite"
        } else {
            "wiz-configs-favourite"
        },
        Emphasis::Quiet,
        PlannerAction::FavouriteConfig(c.id.clone()),
        format!("wizard.what.configs.{index}.favourite"),
    );
    let duplicate = action_button(
        commands,
        "wiz-configs-duplicate",
        Emphasis::Quiet,
        PlannerAction::DuplicateConfig(c.id.clone()),
        format!("wizard.what.configs.{index}.duplicate"),
    );
    let confirming = library.pending_delete.as_deref() == Some(c.id.as_str());
    let delete = action_button(
        commands,
        if confirming {
            "wiz-configs-delete-confirm"
        } else {
            "wiz-configs-delete"
        },
        if confirming {
            Emphasis::Primary
        } else {
            Emphasis::Quiet
        },
        PlannerAction::DeleteConfig(c.id.clone()),
        format!("wizard.what.configs.{index}.delete"),
    );
    [favourite, duplicate, delete]
}

/// The first two problems found with a configuration, in the colour of their severity.
pub(super) fn problem_lines(commands: &mut Commands, problems: &[Problem]) -> Vec<Entity> {
    let mut out = Vec::new();
    for p in problems.iter().take(2) {
        let (key, arg) = problem_text(p);
        let role = if p.is_fatal() {
            ColorRole::Error
        } else {
            ColorRole::Warning
        };
        out.push(
            commands
                .spawn(keyed_args(
                    key,
                    L10nArgs::one("name", arg),
                    TypeScale::Label,
                    role,
                ))
                .id(),
        );
    }
    out
}
