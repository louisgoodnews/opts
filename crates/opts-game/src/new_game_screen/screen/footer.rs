//! The wizard footer: *Skip*, *Back*, *Next* and *Drive*.

use super::*;

pub(super) fn footer_button(
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

pub(super) fn spawn_footer(commands: &mut Commands) -> Entity {
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
