//! The persistent summary bar under the step body: the map, vehicle and time, each a link to its step.

use super::*;

pub(super) fn spawn_summary(commands: &mut Commands) -> Entity {
    let bar = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                flex_shrink: 0.0,
                padding: UiRect::axes(Val::Px(8.0), Val::Px(4.0)),
                border: UiRect::top(Val::Px(1.0)),
                // At the minimum window (1024 x 640) the three parts do not fit on one line:
                // wrap instead of clipping them (M-0065: only ellipsis with tooltip may truncate).
                flex_wrap: FlexWrap::Wrap,
                align_items: AlignItems::Center,
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
                n.flex_shrink = 0.0;
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
