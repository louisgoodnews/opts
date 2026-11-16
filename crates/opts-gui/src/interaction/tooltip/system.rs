//! The tooltip system: finds the hovered or focused target and shows, moves or removes the node.

use super::{
    tooltip_text, Tooltip, TooltipNode, TooltipState, TOOLTIP_DELAY_SECS, TOOLTIP_MAX_WIDTH,
};
use crate::interaction::state::{Disabled, DisabledReason, Focused, Hovered};
use crate::theme::{color::ColorRole, typography::TypeScale};
use crate::tokens::components::{BackgroundRole, ForegroundRole, RadiusToken, TypeStyleToken};
use bevy::ecs::system::SystemParam;
use bevy::input_focus::InputFocusVisible;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

/// Query item of [`tooltip_system`].
type Candidate<'a> = (
    Entity,
    Option<&'a Hovered>,
    Has<Focused>,
    Option<&'a Tooltip>,
    Option<&'a DisabledReason>,
    Has<Disabled>,
);

/// Parameters for [`tooltip_system`].
#[derive(SystemParam)]
pub struct TooltipParams<'w, 's> {
    pub time: Res<'w, Time<Real>>,
    pub state: ResMut<'w, TooltipState>,
    pub visible: Option<Res<'w, InputFocusVisible>>,
    pub candidates: Query<'w, 's, Candidate<'static>>,
    pub existing: Query<'w, 's, (Entity, &'static TooltipNode)>,
    pub geometry: Query<'w, 's, (&'static ComputedNode, &'static UiGlobalTransform)>,
    pub windows: Query<'w, 's, &'static Window, With<PrimaryWindow>>,
    pub ui_scale: Res<'w, UiScale>,
}

fn find_tooltip_target(
    candidates: &Query<Candidate>,
    focus_visible: bool,
) -> (Option<Entity>, bool) {
    let mut hovered_target = None;
    let mut focused_target = None;
    for (entity, hovered, focused, tooltip, reason, disabled) in candidates {
        if tooltip_text(tooltip, reason, disabled).is_none() {
            continue;
        }
        if hovered.is_some_and(|h| h.0) {
            hovered_target = Some(entity);
        }
        if focused && focus_visible {
            focused_target = Some(entity);
        }
    }
    match (hovered_target, focused_target) {
        (Some(h), _) => (Some(h), false),
        (None, Some(f)) => (Some(f), true),
        _ => (None, false),
    }
}

fn compute_tooltip_coords(
    target: Entity,
    geometry: &Query<(&ComputedNode, &UiGlobalTransform)>,
    windows: &Query<&Window, With<PrimaryWindow>>,
    ui_scale: f32,
) -> (f32, f32) {
    let (mut left, mut top) = (0.0, 0.0);
    if let Ok((computed, transform)) = geometry.get(target) {
        let scale = computed.inverse_scale_factor;
        let center = transform.translation * scale;
        let size = computed.size() * scale;
        left = center.x - size.x / 2.0;
        top = center.y + size.y / 2.0 + 6.0;
        if let Ok(window) = windows.single() {
            let (width, height) = (window.width() / ui_scale, window.height() / ui_scale);
            left = left.min(width - TOOLTIP_MAX_WIDTH - 8.0).max(8.0);
            if top + 40.0 > height {
                top = (center.y - size.y / 2.0 - 40.0).max(8.0);
            }
        }
    }
    (left, top)
}

fn spawn_tooltip(commands: &mut Commands, target: Entity, text: &str, left: f32, top: f32) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(left),
                top: Val::Px(top),
                max_width: Val::Px(TOOLTIP_MAX_WIDTH),
                padding: UiRect::axes(Val::Px(10.0), Val::Px(6.0)),
                ..default()
            },
            BackgroundRole(ColorRole::InverseSurface),
            RadiusToken(crate::theme::shape::RadiusLevel::Small),
            GlobalZIndex(5000),
            Pickable::IGNORE,
            TooltipNode { target },
            Name::new("Tooltip"),
        ))
        .with_children(|p| {
            p.spawn((
                Text::new(text),
                TextColor::default(),
                TypeStyleToken(TypeScale::Label),
                ForegroundRole(ColorRole::InverseOnSurface),
                Pickable::IGNORE,
            ));
        });
}

/// Tracks hover/focus, spawns and removes the tooltip node.
pub fn tooltip_system(mut commands: Commands, mut params: TooltipParams) {
    let focus_visible = params.visible.is_none_or(|v| v.0);
    let (target, by_focus) = find_tooltip_target(&params.candidates, focus_visible);
    if target != params.state.target {
        params.state.target = target;
        params.state.hover_secs = 0.0;
    }
    params.state.by_focus = by_focus;
    if target.is_some() {
        params.state.hover_secs += params.time.delta_secs();
    }

    let want =
        target.filter(|_| params.state.by_focus || params.state.hover_secs >= TOOLTIP_DELAY_SECS);
    let mut keep = false;
    for (node, info) in &params.existing {
        if Some(info.target) == want {
            keep = true;
        } else {
            commands.entity(node).despawn();
        }
    }
    let Some(target) = want.filter(|_| !keep) else {
        return;
    };
    let Ok((_, _, _, tooltip, reason, disabled)) = params.candidates.get(target) else {
        return;
    };
    let Some(text) = tooltip_text(tooltip, reason, disabled) else {
        return;
    };
    let (left, top) =
        compute_tooltip_coords(target, &params.geometry, &params.windows, params.ui_scale.0);
    spawn_tooltip(&mut commands, target, &text, left, top);
}
