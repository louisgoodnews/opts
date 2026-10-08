//! Material Design 3 Toast / Snackbar notification widget.

use bevy::prelude::*;

use crate::{
    interaction::events::ToastDismissed,
    theme::{
        color::ColorRole, elevation::ElevationLevel, shape::RadiusLevel, typography::TypeScale,
    },
    tokens::components::{
        BackgroundRole, ElevationToken, ForegroundRole, RadiusToken, TypeStyleToken,
    },
};

/// Marker component for an M3 Toast root entity.
#[derive(Component, Debug, Clone)]
pub struct M3Toast {
    pub message: String,
    pub timer: Option<Timer>,
}

/// Marker component for the optional action button on a toast.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ToastActionButton(pub Entity);

/// Builder for constructing Material Design 3 toast notifications.
///
/// # Example
/// ```rust
/// use opts_gui::prelude::*;
/// use bevy::prelude::*;
///
/// fn spawn_toast(mut commands: Commands) {
///     ToastBuilder::new("Service dispatched successfully")
///         .with_duration(4.0)
///         .with_action("DISMISS")
///         .spawn(&mut commands);
/// }
/// ```
#[derive(Debug, Clone)]
pub struct ToastBuilder {
    pub message: String,
    pub duration_secs: Option<f32>,
    pub action_label: Option<String>,
}

impl ToastBuilder {
    /// Creates a new toast builder with the specified message.
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            duration_secs: Some(4.0),
            action_label: None,
        }
    }

    /// Sets the display duration in seconds before automatic dismissal (or None for persistent).
    pub fn with_duration(mut self, seconds: f32) -> Self {
        self.duration_secs = Some(seconds);
        self
    }

    /// Makes the toast persistent until manually dismissed.
    pub fn persistent(mut self) -> Self {
        self.duration_secs = None;
        self
    }

    /// Sets an optional action button label (e.g. "UNDO" or "CLOSE").
    pub fn with_action(mut self, action_label: impl Into<String>) -> Self {
        self.action_label = Some(action_label.into());
        self
    }

    /// Spawns the toast into the Bevy ECS world using top-level [`Commands`].
    pub fn spawn(self, commands: &mut Commands) -> Entity {
        let timer = self
            .duration_secs
            .map(|secs| Timer::from_seconds(secs, TimerMode::Once));

        let root = commands
            .spawn((
                NodeBundle {
                    style: Style {
                        display: Display::Flex,
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::SpaceBetween,
                        min_width: Val::Px(280.0),
                        max_width: Val::Px(560.0),
                        min_height: Val::Px(48.0),
                        padding: UiRect::axes(Val::Px(16.0), Val::Px(12.0)),
                        column_gap: Val::Px(16.0),
                        position_type: PositionType::Absolute,
                        bottom: Val::Px(24.0),
                        right: Val::Px(24.0),
                        ..default()
                    },
                    z_index: ZIndex::Global(100),
                    ..default()
                },
                M3Toast {
                    message: self.message.clone(),
                    timer,
                },
                BackgroundRole(ColorRole::InverseSurface),
                RadiusToken(RadiusLevel::ExtraSmall),
                ElevationToken(ElevationLevel::Level3),
            ))
            .id();

        commands.entity(root).with_children(|parent| {
            // Message text
            parent.spawn((
                TextBundle::from_section(
                    self.message,
                    TextStyle {
                        font_size: 14.0,
                        color: Color::WHITE,
                        ..default()
                    },
                ),
                ForegroundRole(ColorRole::InverseOnSurface),
                TypeStyleToken(TypeScale::BodyMedium),
            ));

            // Optional action button
            if let Some(action) = self.action_label {
                parent
                    .spawn((
                        ButtonBundle {
                            style: Style {
                                padding: UiRect::axes(Val::Px(8.0), Val::Px(4.0)),
                                ..default()
                            },
                            background_color: BackgroundColor(Color::NONE),
                            ..default()
                        },
                        ToastActionButton(root),
                    ))
                    .with_children(|btn_parent| {
                        btn_parent.spawn((
                            TextBundle::from_section(
                                action,
                                TextStyle {
                                    font_size: 14.0,
                                    color: Color::WHITE,
                                    ..default()
                                },
                            ),
                            ForegroundRole(ColorRole::InversePrimary),
                            TypeStyleToken(TypeScale::LabelLarge),
                        ));
                    });
            }
        });

        root
    }

    /// Spawns the toast as a child inside a parent [`ChildBuilder`].
    pub fn spawn_child(self, parent: &mut ChildBuilder) -> Entity {
        let timer = self
            .duration_secs
            .map(|secs| Timer::from_seconds(secs, TimerMode::Once));

        let mut root_cmd = parent.spawn((
            NodeBundle {
                style: Style {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::SpaceBetween,
                    min_width: Val::Px(280.0),
                    max_width: Val::Px(560.0),
                    min_height: Val::Px(48.0),
                    padding: UiRect::axes(Val::Px(16.0), Val::Px(12.0)),
                    column_gap: Val::Px(16.0),
                    position_type: PositionType::Absolute,
                    bottom: Val::Px(24.0),
                    right: Val::Px(24.0),
                    ..default()
                },
                z_index: ZIndex::Global(100),
                ..default()
            },
            M3Toast {
                message: self.message.clone(),
                timer,
            },
            BackgroundRole(ColorRole::InverseSurface),
            RadiusToken(RadiusLevel::ExtraSmall),
            ElevationToken(ElevationLevel::Level3),
        ));

        let root = root_cmd.id();

        root_cmd.with_children(|child_parent| {
            // Message text
            child_parent.spawn((
                TextBundle::from_section(
                    self.message,
                    TextStyle {
                        font_size: 14.0,
                        color: Color::WHITE,
                        ..default()
                    },
                ),
                ForegroundRole(ColorRole::InverseOnSurface),
                TypeStyleToken(TypeScale::BodyMedium),
            ));

            // Optional action button
            if let Some(action) = self.action_label {
                child_parent
                    .spawn((
                        ButtonBundle {
                            style: Style {
                                padding: UiRect::axes(Val::Px(8.0), Val::Px(4.0)),
                                ..default()
                            },
                            background_color: BackgroundColor(Color::NONE),
                            ..default()
                        },
                        ToastActionButton(root),
                    ))
                    .with_children(|btn_parent| {
                        btn_parent.spawn((
                            TextBundle::from_section(
                                action,
                                TextStyle {
                                    font_size: 14.0,
                                    color: Color::WHITE,
                                    ..default()
                                },
                            ),
                            ForegroundRole(ColorRole::InversePrimary),
                            TypeStyleToken(TypeScale::LabelLarge),
                        ));
                    });
            }
        });

        root
    }
}

/// System ticking toast timers and despawning expired toasts.
pub fn toast_timer_system(
    mut commands: Commands,
    time: Res<Time>,
    mut toast_query: Query<(Entity, &mut M3Toast)>,
    mut events: EventWriter<ToastDismissed>,
) {
    for (entity, mut toast) in &mut toast_query {
        if let Some(ref mut timer) = toast.timer {
            timer.tick(time.delta());
            if timer.just_finished() {
                events.send(ToastDismissed { entity });
                commands.entity(entity).despawn_recursive();
            }
        }
    }
}

/// System handling clicks on toast action buttons.
pub fn toast_action_click_system(
    mut commands: Commands,
    mut action_query: Query<(&ToastActionButton, &Interaction), Changed<Interaction>>,
    mut events: EventWriter<ToastDismissed>,
) {
    for (action, interaction) in &mut action_query {
        if *interaction == Interaction::Pressed {
            events.send(ToastDismissed { entity: action.0 });
            commands.entity(action.0).despawn_recursive();
        }
    }
}

/// Plugin registering Toast widget systems and events.
#[derive(Default)]
pub struct ToastPlugin;

impl Plugin for ToastPlugin {
    fn build(&self, app: &mut App) {
        if !app.world().contains_resource::<Events<ToastDismissed>>() {
            app.add_event::<ToastDismissed>();
        }
        app.add_systems(Update, (toast_timer_system, toast_action_click_system));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_toast_builder_spawn() {
        let mut app = App::new();
        let entity = {
            let mut commands = app.world_mut().commands();
            ToastBuilder::new("Notice text")
                .with_action("CLOSE")
                .spawn(&mut commands)
        };

        app.update();

        let toast = app.world().get::<M3Toast>(entity);
        assert_eq!(toast.map(|t| t.message.as_str()), Some("Notice text"));
    }
}
