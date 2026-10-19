//! The outcome of a load: the problem rows and the buttons shown when loading ends.

use super::*;

/// What the outcome block shows right now (rebuilt only when this changes).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutcomeSig {
    pub(super) outcome: LoadOutcome,
    pub(super) problems: usize,
    pub(super) details: bool,
}

pub(super) fn problem_row(commands: &mut Commands, p: &LoadProblem, details: bool) -> Entity {
    let wrap = commands.spawn(column(2.0)).id();
    let icon = if p.fatal {
        glyphs::ERROR
    } else {
        glyphs::WARNING
    };
    let role = if p.fatal {
        ColorRole::Error
    } else {
        ColorRole::Warning
    };
    let head = commands.spawn(row(8.0)).id();
    let g = commands.spawn(glyph(icon, 20.0, role)).id();
    let args = L10nArgs(
        p.args
            .iter()
            .map(|(k, v)| (k.clone(), v.clone().into()))
            .collect(),
    );
    let what = commands
        .spawn(keyed_args(
            p.key,
            args,
            TypeScale::BodyMedium,
            ColorRole::OnSurface,
        ))
        .id();
    commands.entity(head).add_children(&[g, what]);
    let mut kids = vec![head];
    if let Some(cause) = p.cause_key {
        kids.push(
            commands
                .spawn((
                    keyed(cause, TypeScale::Label, ColorRole::OnSurfaceVariant),
                    Node {
                        margin: UiRect::left(Val::Px(28.0)),
                        ..default()
                    },
                ))
                .id(),
        );
    }
    if details && !p.detail.is_empty() {
        kids.push(
            commands
                .spawn((
                    plain(
                        p.detail.clone(),
                        TypeScale::Label,
                        ColorRole::OnSurfaceVariant,
                    ),
                    Node {
                        margin: UiRect::left(Val::Px(28.0)),
                        ..default()
                    },
                ))
                .id(),
        );
    }
    commands.entity(wrap).add_children(&kids);
    wrap
}

pub(super) fn outcome_button(
    commands: &mut Commands,
    key: &'static str,
    primary: bool,
    id: &'static str,
) -> Entity {
    let b = if primary {
        ButtonBuilder::filled(key)
    } else {
        ButtonBuilder::tonal(key)
    }
    .with_key(key)
    .spawn(commands);
    commands.entity(b).insert(UiId::new(id));
    b
}

pub(super) fn build_outcome(commands: &mut Commands, container: Entity, report: &LoadReport) {
    let mut kids = Vec::new();
    match report.outcome {
        LoadOutcome::Loading | LoadOutcome::Ready => {}
        LoadOutcome::ReadyWithProblems | LoadOutcome::Failed => {
            let failed = report.outcome == LoadOutcome::Failed;
            let (key, role) = if failed {
                ("load-failed", ColorRole::Error)
            } else {
                ("load-ready-problems", ColorRole::Warning)
            };
            kids.push(
                commands
                    .spawn(keyed_args(
                        key,
                        L10nArgs::one("count", report.blocking_count()), // l10n-ok: variable name
                        TypeScale::TitleLarge,
                        role,
                    ))
                    .id(),
            );
            for p in report
                .problems
                .iter()
                .filter(|p| p.severity() == Severity::Blocking)
            {
                kids.push(problem_row(commands, p, report.details_open || failed));
            }
            kids.push(outcome_buttons(commands, failed));
        }
    }
    commands.entity(container).add_children(&kids);
}

fn outcome_buttons(commands: &mut Commands, failed: bool) -> Entity {
    let line = commands.spawn(row(12.0)).id();
    let mut buttons = Vec::new();
    if failed {
        let retry = outcome_button(commands, "load-retry", true, "loading.retry");
        commands.entity(retry).insert((RetryBtn, DefaultFocus));
        let another = outcome_button(
            commands,
            "load-choose-another",
            false,
            "loading.choose-another",
        );
        commands.entity(another).insert(ChooseAnotherBtn);
        let back = outcome_button(commands, "load-back", false, "loading.back");
        commands.entity(back).insert(LoadingBackBtn);
        buttons.extend([retry, another, back]);
    } else {
        let details = outcome_button(commands, "load-details", false, "loading.details");
        commands.entity(details).insert(DetailsBtn);
        let cont = outcome_button(commands, "load-continue", true, "loading.continue");
        commands.entity(cont).insert((ContinueBtn, DefaultFocus));
        buttons.extend([details, cont]);
    }
    commands.entity(line).add_children(&buttons);
    line
}
