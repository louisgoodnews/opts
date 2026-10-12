//! The frame of the wizard: header with the stepper, the step body, the persistent summary and the
//! fixed footer; plus the systems that keep that frame current.

use bevy::prelude::*;
use opts_gui::interaction::state::{Disabled, DisabledReason, Selected};
use opts_gui::l10n::Localization;
use opts_gui::patterns::glyphs;
use opts_gui::prelude::*;

use super::planner::{validate, Invalid, PlannerState, PlannerStep};
use super::texts::{what_text, when_text, where_text};
use super::types::{GameCatalogue, NewGameScreenRoot, NewGameSetup};
use super::ui::{action_button, column, glyph, keyed, plain, row, Act, Emphasis, PlannerAction};
use crate::shell::{spawn_screen_root, ShellContent};

/// The container the step bodies are built into.
#[derive(Component, Debug, Clone, Copy)]
pub struct PlannerBody;

/// A part of the persistent summary (its text).
#[derive(Component, Debug, Clone, Copy)]
pub struct SummaryPart(pub PlannerStep);

/// One button of the stepper.
#[derive(Component, Debug, Clone, Copy)]
pub struct StepperButton {
    /// The step it goes to.
    pub step: PlannerStep,
    /// Node showing the step number.
    pub number: Entity,
    /// Node showing the check of a reached step.
    pub check: Entity,
}

/// Which footer button an entity is.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub enum FooterButton {
    /// *Back*.
    Back,
    /// *Next*.
    Next,
    /// *Skip to review*.
    Skip,
    /// *Drive*.
    Drive,
}

fn step_key(step: PlannerStep) -> &'static str {
    match step {
        PlannerStep::Where => "wiz-step-where",
        PlannerStep::What => "wiz-step-what",
        PlannerStep::When => "wiz-step-when",
        PlannerStep::Review => "wiz-step-review",
    }
}

fn spawn_stepper_button(commands: &mut Commands, step: PlannerStep) -> Entity {
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

fn spawn_header(commands: &mut Commands) -> Entity {
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

fn spawn_summary(commands: &mut Commands) -> Entity {
    let bar = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                flex_shrink: 0.0,
                padding: UiRect::axes(Val::Px(8.0), Val::Px(4.0)),
                border: UiRect::top(Val::Px(1.0)),
                ..row(4.0)
            },
            BorderRole(ColorRole::OutlineVariant),
            Name::new("PlannerSummary"),
        ))
        .id();
    let mut kids = Vec::new();
    for step in [PlannerStep::Where, PlannerStep::What, PlannerStep::When] {
        if step != PlannerStep::Where {
            kids.push(
                commands
                    .spawn(plain(
                        "·",
                        TypeScale::BodyMedium,
                        ColorRole::OnSurfaceVariant,
                    ))
                    .id(),
            );
        }
        let part = ButtonBuilder::text(String::new()).spawn(commands);
        commands.entity(part).queue(|mut e: EntityWorldMut| {
            if let Some(mut n) = e.get_mut::<Node>() {
                n.min_width = Val::Px(0.0);
                n.flex_shrink = 1.0;
                n.overflow = Overflow::clip();
                n.justify_content = JustifyContent::FlexStart;
            }
        });
        let text = commands
            .spawn((
                plain(String::new(), TypeScale::BodyMedium, ColorRole::OnSurface),
                SummaryPart(step),
                TextLayout::new(Justify::Left, LineBreak::NoWrap),
            ))
            .id();
        commands.entity(part).insert((
            Act(PlannerAction::GoStep(step)),
            UiId::new(format!("wizard.summary.{}", step.id())),
        ));
        commands.entity(part).add_child(text);
        kids.push(part);
    }
    commands.entity(bar).add_children(&kids);
    bar
}

fn footer_button(
    commands: &mut Commands,
    kind: FooterButton,
    key: &'static str,
    emphasis: Emphasis,
    action: PlannerAction,
    id: &str,
) -> Entity {
    let b = action_button(
        commands,
        key,
        emphasis,
        action,
        format!("wizard.footer.{id}"),
    );
    commands.entity(b).insert(kind);
    b
}

fn spawn_footer(commands: &mut Commands) -> Entity {
    let footer = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                flex_shrink: 0.0,
                justify_content: JustifyContent::SpaceBetween,
                padding: UiRect::axes(Val::Px(4.0), Val::Px(8.0)),
                border: UiRect::top(Val::Px(1.0)),
                ..row(12.0)
            },
            BorderRole(ColorRole::OutlineVariant),
            FocusRegion(Landmark::Footer),
            Name::new("PlannerFooter"),
        ))
        .id();
    let skip = footer_button(
        commands,
        FooterButton::Skip,
        "wiz-skip",
        Emphasis::Quiet,
        PlannerAction::Skip,
        "skip",
    );
    let right = commands
        .spawn(Node {
            margin: UiRect::left(Val::Auto),
            ..row(12.0)
        })
        .id();
    let back = footer_button(
        commands,
        FooterButton::Back,
        "wiz-back",
        Emphasis::Tonal,
        PlannerAction::Back,
        "back",
    );
    let next = footer_button(
        commands,
        FooterButton::Next,
        "wiz-next",
        Emphasis::Primary,
        PlannerAction::Next,
        "next",
    );
    let drive = footer_button(
        commands,
        FooterButton::Drive,
        "wiz-drive",
        Emphasis::Primary,
        PlannerAction::Drive,
        "drive",
    );
    commands.entity(right).add_children(&[back, next, drive]);
    commands.entity(footer).add_children(&[skip, right]);
    footer
}

/// Spawns the frame (the body is filled by the caller) and returns `(root, body)`.
pub fn spawn_frame(commands: &mut Commands, shell: &ShellContent) -> (Entity, Entity) {
    let root = spawn_screen_root(commands, shell, "PlanADrive");
    commands.entity(root).insert((
        NewGameScreenRoot,
        BackgroundRole(ColorRole::Surface),
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            padding: UiRect::all(Val::Px(16.0)),
            row_gap: Val::Px(8.0),
            ..column(8.0)
        },
    ));
    let header = spawn_header(commands);
    let body = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                flex_grow: 1.0,
                ..column(0.0)
            },
            PlannerBody,
            Name::new("PlannerBody"),
        ))
        .id();
    let summary = spawn_summary(commands);
    let footer = spawn_footer(commands);
    commands
        .entity(root)
        .add_children(&[header, body, summary, footer]);
    (root, body)
}

/// The reason the current step cannot go on, if any.
pub fn step_invalid(
    step: PlannerStep,
    setup: &NewGameSetup,
    catalogue: &GameCatalogue,
) -> Option<Invalid> {
    let all = validate(setup, catalogue).err()?;
    match (step, all) {
        (PlannerStep::Where, Invalid::NoMap | Invalid::NoStartPoint) => Some(all),
        (PlannerStep::What, Invalid::NoVehicle) => Some(all),
        _ => None,
    }
}

/// Keeps summary, stepper and footer current.
#[allow(clippy::too_many_arguments)]
pub fn refresh_chrome(
    state: Res<PlannerState>,
    setup: Res<NewGameSetup>,
    catalogue: Res<GameCatalogue>,
    loc: Res<Localization>,
    mut summary: Query<(&SummaryPart, &mut Text)>,
    steppers: Query<(Entity, &StepperButton, Has<Selected>)>,
    footers: Query<(Entity, &FooterButton, Has<Disabled>)>,
    mut nodes: Query<&mut Node>,
    mut commands: Commands,
    added: Query<(), Added<StepperButton>>,
) {
    if !state.is_changed() && !setup.is_changed() && !catalogue.is_changed() && added.is_empty() {
        return;
    }
    for (part, mut text) in &mut summary {
        let new = match part.0 {
            PlannerStep::Where => where_text(&setup),
            PlannerStep::What => what_text(&catalogue, &setup),
            _ => when_text(&loc, &setup),
        };
        if text.0 != new {
            text.0 = new;
        }
    }
    for (entity, s, selected) in &steppers {
        let now = s.step == state.step;
        if now != selected || !added.is_empty() {
            let mut e = commands.entity(entity);
            if now {
                e.insert((Selected, BackgroundRole(ColorRole::SecondaryContainer)));
            } else {
                e.remove::<Selected>();
                e.insert(BackgroundRole(ColorRole::Surface));
            }
        }
        let reached = s.step.index() < state.step.index();
        if let Ok(mut n) = nodes.get_mut(s.check) {
            n.display = if reached {
                Display::Flex
            } else {
                Display::None
            };
        }
        if let Ok(mut n) = nodes.get_mut(s.number) {
            n.display = if reached {
                Display::None
            } else {
                Display::Flex
            };
        }
    }
    refresh_footer(
        &state,
        &setup,
        &catalogue,
        &loc,
        &footers,
        &mut nodes,
        &mut commands,
    );
}

fn reason_for(loc: &Localization, reason: Invalid) -> String {
    loc.t(super::step_review::reason_key(reason))
}

fn refresh_footer(
    state: &PlannerState,
    setup: &NewGameSetup,
    catalogue: &GameCatalogue,
    loc: &Localization,
    footers: &Query<(Entity, &FooterButton, Has<Disabled>)>,
    nodes: &mut Query<&mut Node>,
    commands: &mut Commands,
) {
    let full = validate(setup, catalogue);
    let here = step_invalid(state.step, setup, catalogue);
    let review = state.step == PlannerStep::Review;
    for (entity, kind, disabled) in footers {
        let (show, block) = match kind {
            FooterButton::Back => (true, None),
            FooterButton::Next => (!review, here),
            FooterButton::Skip => (!review, full.err()),
            FooterButton::Drive => (review, full.err()),
        };
        if let Ok(mut n) = nodes.get_mut(entity) {
            n.display = if show { Display::Flex } else { Display::None };
        }
        match (block, disabled) {
            (Some(reason), _) => {
                commands
                    .entity(entity)
                    .insert((Disabled, DisabledReason::new(reason_for(loc, reason))));
            }
            (None, true) => {
                commands
                    .entity(entity)
                    .remove::<(Disabled, DisabledReason)>();
            }
            _ => {}
        }
    }
}
