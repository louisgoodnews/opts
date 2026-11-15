//! One saved configuration as a row of the Configurations list, and what its check says.

use super::*;

/// What the screen shows of a configuration's check, as a message key and its variable.
pub(super) fn problem_text(p: &Problem) -> (&'static str, String) {
    match p {
        Problem::VehicleMissing { .. } => ("wiz-configs-vehicle-missing", String::new()),
        Problem::RepaintMissing { repaint } => ("wiz-configs-repaint-missing", repaint.clone()),
        Problem::SetvarUnknown { name } | Problem::SetvarRejected { name, .. } => {
            ("wiz-configs-option-left-out", name.clone())
        }
    }
}

pub(super) fn config_row(
    commands: &mut Commands,
    library: &ConfigLibrary,
    catalogue: &GameCatalogue,
    c: &BusConfiguration,
    shown: (&VehicleItem, Option<&VehicleRepaints>),
    index: usize,
) -> Entity {
    let line = commands
        .spawn(Node {
            flex_wrap: FlexWrap::Wrap,
            ..row(8.0)
        })
        .id();
    let vehicle = catalogue.vehicles.iter().find(|v| v.id == c.vehicle);
    // The check needs the repaints of the configuration's vehicle; only the shown vehicle's are
    // at hand. For another vehicle, or while they are being read, only a missing vehicle is
    // shown here (the full check runs when the configuration is loaded).
    let known = shown.1.filter(|_| shown.0.id == c.vehicle);
    let mut checked = c.check(catalogue, |_| known.cloned().unwrap_or_default());
    if known.is_none() {
        checked.problems.retain(Problem::is_fatal);
    }
    let load = spawn_choice(
        commands,
        PlannerAction::LoadConfig(c.id.clone()),
        format!("wizard.what.configs.{index}.load"),
        Node {
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::FlexStart,
            padding: UiRect::axes(Val::Px(10.0), Val::Px(6.0)),
            ..default()
        },
    );
    let mut title = c.name.clone();
    if c.favourite {
        title.insert_str(0, "★ ");
    }
    let sub = match vehicle {
        Some(v) if c.repaint.is_empty() => v.full_name.clone(),
        Some(v) => format!("{} · {}", v.full_name, c.repaint),
        None => c.vehicle.clone(),
    };
    let name = commands
        .spawn(plain(title, TypeScale::BodyLarge, ColorRole::OnSurface))
        .id();
    let subtitle = commands
        .spawn(plain(
            sub,
            TypeScale::BodyMedium,
            ColorRole::OnSurfaceVariant,
        ))
        .id();
    let mut content = vec![name, subtitle];
    content.extend(problem_lines(commands, &checked.problems));
    commands
        .entity(load)
        .insert(ChoiceStep(PlannerStep::What))
        .insert_children(1, &content);
    let [favourite, duplicate, delete] = row_buttons(commands, library, c, index);
    let thumb = super::super::thumbnails::spawn_config_thumb(
        commands,
        &c.vehicle,
        vehicle.map_or("", |v| v.class.as_str()),
    );
    commands
        .entity(line)
        .add_children(&[thumb, load, favourite, duplicate, delete]);
    line
}
