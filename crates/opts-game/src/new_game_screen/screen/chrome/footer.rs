//! The footer buttons: back, next, skip and *Drive*, and why a button is disabled.

use super::*;

fn reason_for(loc: &Localization, reason: Invalid) -> String {
    loc.t(super::super::super::step_review::reason_key(reason))
}

pub(super) fn refresh_footer(
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
