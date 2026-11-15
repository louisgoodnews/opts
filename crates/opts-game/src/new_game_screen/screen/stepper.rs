//! The stepper of the wizard header: one button per step with its number, check and name.

use super::*;

pub(super) fn step_key(step: PlannerStep) -> &'static str {
    match step {
        PlannerStep::Where => "wiz-step-where",
        PlannerStep::What => "wiz-step-what",
        PlannerStep::When => "wiz-step-when",
        PlannerStep::Trip => "wiz-step-trip",
        PlannerStep::Review => "wiz-step-review",
    }
}

pub(super) fn spawn_stepper_button(commands: &mut Commands, step: PlannerStep) -> Entity {
    let button = ButtonBuilder::text(String::new()).spawn(commands);
    let number = commands
        .spawn(plain(
            (step.index() + 1).to_string(),
            TypeScale::BodyMedium,
            ColorRole::OnSurface,
        ))
        .id();
    let check = commands
        .spawn((
            glyph(glyphs::CHECK, 18.0, ColorRole::OnSurface),
            Node {
                display: Display::None,
                ..default()
            },
        ))
        .id();
    let label = commands
        .spawn(keyed(
            step_key(step),
            TypeScale::BodyMedium,
            ColorRole::OnSurface,
        ))
        .id();
    commands.entity(button).insert((
        Act(PlannerAction::GoStep(step)),
        UiId::new(format!("wizard.stepper.{}", step.id())),
        StepperButton {
            step,
            number,
            check,
        },
    ));
    commands
        .entity(button)
        .add_children(&[number, check, label]);
    button
}

pub(super) fn spawn_header(commands: &mut Commands) -> Entity {
    let header = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                flex_shrink: 0.0,
                flex_wrap: FlexWrap::Wrap,
                justify_content: JustifyContent::SpaceBetween,
                row_gap: Val::Px(8.0),
                padding: UiRect::axes(Val::Px(4.0), Val::Px(4.0)),
                ..row(16.0)
            },
            Name::new("PlannerHeader"),
        ))
        .id();
    let title = commands
        .spawn(keyed(
            "wiz-title",
            TypeScale::Headline,
            ColorRole::OnSurface,
        ))
        .id();
    let stepper = commands.spawn(row(4.0)).id();
    let buttons: Vec<Entity> = PlannerStep::ALL
        .iter()
        .map(|s| spawn_stepper_button(commands, *s))
        .collect();
    commands.entity(stepper).add_children(&buttons);
    commands.entity(header).add_children(&[title, stepper]);
    header
}
