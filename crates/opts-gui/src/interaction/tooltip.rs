//! Tooltips: shown after 600 ms of hover or immediately on keyboard focus.
//!
//! The text comes from a [`Tooltip`] component or, for a disabled control, from its
//! [`DisabledReason`]; icon-only buttons always carry a [`Tooltip`]. At most one tooltip exists.

use std::borrow::Cow;

use bevy::input_focus::InputFocusVisible;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use crate::interaction::state::{Disabled, DisabledReason, Focused, Hovered};
use crate::theme::{color::ColorRole, typography::TypeScale};
use crate::tokens::components::{BackgroundRole, ForegroundRole, RadiusToken, TypeStyleToken};

/// Hover time before a tooltip appears.
pub const TOOLTIP_DELAY_SECS: f32 = 0.6;
/// Widest tooltip in logical pixels.
pub const TOOLTIP_MAX_WIDTH: f32 = 320.0;

/// Tooltip text of a control (repeat the shortcut here).
#[derive(Component, Debug, Clone, PartialEq, Eq)]
pub struct Tooltip(pub Cow<'static, str>);

impl Tooltip {
    /// Creates a tooltip from any string.
    pub fn new(text: impl Into<Cow<'static, str>>) -> Self {
        Self(text.into())
    }
}

/// Marker of the single tooltip node.
#[derive(Component, Debug, Clone, Copy)]
pub struct TooltipNode {
    /// The control the tooltip describes.
    pub target: Entity,
}

/// What the tooltip system is currently tracking.
#[derive(Resource, Debug, Default)]
pub struct TooltipState {
    /// Control the pointer rests on or that holds keyboard focus.
    pub target: Option<Entity>,
    /// Seconds the target has been hovered.
    pub hover_secs: f32,
    /// Whether the target was chosen by keyboard focus (no delay).
    pub by_focus: bool,
}

/// Text a control shows as tooltip, if any.
pub fn tooltip_text(
    tooltip: Option<&Tooltip>,
    reason: Option<&DisabledReason>,
    disabled: bool,
) -> Option<String> {
    match (disabled, reason, tooltip) {
        (true, Some(r), _) => Some(r.0.to_string()),
        (_, _, Some(t)) => Some(t.0.to_string()),
        _ => None,
    }
}

/// Query item of [`tooltip_system`].
type Candidate<'a> = (
    Entity,
    Option<&'a Hovered>,
    Has<Focused>,
    Option<&'a Tooltip>,
    Option<&'a DisabledReason>,
    Has<Disabled>,
);

/// Tracks hover/focus, spawns and removes the tooltip node.
#[allow(clippy::too_many_arguments)]
pub fn tooltip_system(
    mut commands: Commands,
    time: Res<Time<Real>>,
    mut state: ResMut<TooltipState>,
    visible: Option<Res<InputFocusVisible>>,
    candidates: Query<Candidate>,
    existing: Query<(Entity, &TooltipNode)>,
    geometry: Query<(&ComputedNode, &UiGlobalTransform)>,
    windows: Query<&Window, With<PrimaryWindow>>,
) {
    let focus_visible = visible.is_none_or(|v| v.0);
    let mut hovered_target = None;
    let mut focused_target = None;
    for (entity, hovered, focused, tooltip, reason, disabled) in &candidates {
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
    let (target, by_focus) = match (hovered_target, focused_target) {
        (Some(h), _) => (Some(h), false),
        (None, Some(f)) => (Some(f), true),
        _ => (None, false),
    };
    if target != state.target {
        state.target = target;
        state.hover_secs = 0.0;
    }
    state.by_focus = by_focus;
    if target.is_some() {
        state.hover_secs += time.delta_secs();
    }

    let want = target.filter(|_| state.by_focus || state.hover_secs >= TOOLTIP_DELAY_SECS);
    let mut keep = false;
    for (node, info) in &existing {
        if Some(info.target) == want {
            keep = true;
        } else {
            commands.entity(node).despawn();
        }
    }
    let Some(target) = want.filter(|_| !keep) else {
        return;
    };
    let Ok((_, _, _, tooltip, reason, disabled)) = candidates.get(target) else {
        return;
    };
    let Some(text) = tooltip_text(tooltip, reason, disabled) else {
        return;
    };
    let (mut left, mut top) = (0.0, 0.0);
    if let Ok((computed, transform)) = geometry.get(target) {
        let scale = computed.inverse_scale_factor;
        let center = transform.translation * scale;
        let size = computed.size() * scale;
        left = center.x - size.x / 2.0;
        top = center.y + size.y / 2.0 + 6.0;
        if let Ok(window) = windows.single() {
            left = left.min(window.width() - TOOLTIP_MAX_WIDTH - 8.0).max(8.0);
            if top + 40.0 > window.height() {
                top = (center.y - size.y / 2.0 - 40.0).max(8.0);
            }
        }
    }
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
                TypeStyleToken(TypeScale::BodySmall),
                ForegroundRole(ColorRole::InverseOnSurface),
                Pickable::IGNORE,
            ));
        });
}

/// Registers the tooltip resource and system.
pub fn build(app: &mut App) {
    app.init_resource::<TooltipState>()
        .add_systems(Update, tooltip_system);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disabled_reason_beats_tooltip_text() {
        let t = Tooltip::new("Open");
        let r = DisabledReason::new("Needs a saved game");
        assert_eq!(
            tooltip_text(Some(&t), Some(&r), true).as_deref(),
            Some("Needs a saved game")
        );
        assert_eq!(
            tooltip_text(Some(&t), Some(&r), false).as_deref(),
            Some("Open")
        );
        assert_eq!(tooltip_text(None, None, true), None);
    }

    #[test]
    fn keyboard_focus_shows_the_tooltip_at_once_and_blur_removes_it() {
        use crate::components::button::ButtonBuilder;
        let mut app = crate::testing::headless_app();
        let button = {
            let mut commands = app.world_mut().commands();
            ButtonBuilder::filled("Go").spawn(&mut commands)
        };
        app.update();
        app.world_mut()
            .entity_mut(button)
            .insert((Tooltip::new("Start the drive (Enter)"), Focused));
        app.update();
        app.update();
        let tips = |app: &mut App| {
            app.world_mut()
                .query::<&TooltipNode>()
                .iter(app.world())
                .count()
        };
        assert_eq!(tips(&mut app), 1);
        app.world_mut().entity_mut(button).remove::<Focused>();
        app.update();
        app.update();
        assert_eq!(tips(&mut app), 0);
    }
}
