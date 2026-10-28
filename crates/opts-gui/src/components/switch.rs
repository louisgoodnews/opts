//! Material Design 3 Switch toggle widget.

use crate::components::Button;
use bevy::prelude::*;

use crate::{
    interaction::{
        activation::Activated, events::SwitchToggled, focus::Focusable, state::Disabled,
    },
    theme::{color::ColorRole, shape::RadiusLevel, spacing::SpacingScale, typography::TypeScale},
    tokens::components::{BackgroundRole, BorderRole, ForegroundRole, RadiusToken, TypeStyleToken},
};

/// Marker component for an M3 Switch root entity.
#[derive(Component, Debug, Clone, Default)]
pub struct M3Switch {
    pub is_on: bool,
}

/// Marker component linking an internal switch track to its root switch entity.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct SwitchTrack(pub Entity);

/// Marker component linking an internal switch thumb to its root switch entity.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct SwitchThumb(pub Entity);

/// Builder for constructing Material Design 3 switch toggles.
///
/// # Example
/// ```rust
/// use opts_gui::prelude::*;
/// use bevy::prelude::*;
///
/// fn spawn_switch(mut commands: Commands) {
///     SwitchBuilder::new("Auto-dispatch")
///         .with_value(true)
///         .spawn(&mut commands);
/// }
/// ```
#[derive(Debug, Clone, Default)]
pub struct SwitchBuilder {
    pub label: Option<String>,
    pub is_on: bool,
    pub disabled: bool,
}

impl SwitchBuilder {
    /// Creates a new switch builder with an associated label.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: Some(label.into()),
            is_on: false,
            disabled: false,
        }
    }

    /// Creates a standalone switch builder without a label.
    pub fn standalone() -> Self {
        Self::default()
    }

    /// Sets the initial switch value.
    pub fn with_value(mut self, is_on: bool) -> Self {
        self.is_on = is_on;
        self
    }

    /// Sets whether the switch is disabled.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    fn track_styles(is_on: bool) -> (ColorRole, Option<ColorRole>, JustifyContent) {
        if is_on {
            (ColorRole::Primary, None, JustifyContent::FlexEnd)
        } else {
            (
                ColorRole::SurfaceContainerHighest,
                Some(ColorRole::Outline),
                JustifyContent::FlexStart,
            )
        }
    }

    fn thumb_styles(is_on: bool) -> (ColorRole, f32) {
        if is_on {
            (ColorRole::OnPrimary, 24.0)
        } else {
            (ColorRole::Outline, 16.0)
        }
    }

    /// Spawns the switch into the Bevy ECS world using top-level [`Commands`].
    pub fn spawn(self, commands: &mut Commands) -> Entity {
        let (track_bg, track_border, justify) = Self::track_styles(self.is_on);
        let (thumb_bg, thumb_size) = Self::thumb_styles(self.is_on);

        let mut row_cmd = commands.spawn((
            (
                Button,
                Node {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    column_gap: SpacingScale::Small.val(),
                    min_height: Val::Px(40.0),
                    padding: UiRect::axes(Val::Px(4.0), Val::Px(4.0)),
                    ..default()
                },
                BackgroundColor(Color::NONE),
            ),
            M3Switch { is_on: self.is_on },
            Focusable { tab_index: 0 },
        ));

        if self.disabled {
            row_cmd.insert(Disabled);
        }

        let root = row_cmd.id();

        commands.entity(root).with_children(|parent| {
            // Track
            let mut track_cmd = parent.spawn((
                (Node {
                    width: Val::Px(52.0),
                    height: Val::Px(32.0),
                    padding: UiRect::all(Val::Px(4.0)),
                    display: Display::Flex,
                    align_items: AlignItems::Center,
                    justify_content: justify,
                    border: if track_border.is_some() {
                        UiRect::all(Val::Px(2.0))
                    } else {
                        UiRect::DEFAULT
                    },
                    ..default()
                }),
                SwitchTrack(root),
                BackgroundRole(track_bg),
                RadiusToken(RadiusLevel::Full),
            ));

            if let Some(border_role) = track_border {
                track_cmd.insert(BorderRole(border_role));
            }

            track_cmd.with_children(|track_parent| {
                // Thumb
                track_parent.spawn((
                    (Node {
                        width: Val::Px(thumb_size),
                        height: Val::Px(thumb_size),
                        ..default()
                    }),
                    SwitchThumb(root),
                    BackgroundRole(thumb_bg),
                    RadiusToken(RadiusLevel::Full),
                ));
            });

            // Optional label
            if let Some(lbl) = self.label {
                parent.spawn((
                    (
                        Text::new(lbl),
                        TextFont {
                            font_size: FontSize::Px(14.0),
                            ..default()
                        },
                        TextColor::default(),
                    ),
                    TypeStyleToken(TypeScale::BodyMedium),
                    ForegroundRole(ColorRole::OnSurface),
                ));
            }
        });

        root
    }

    /// Spawns the switch as a child inside a parent [`ChildSpawnerCommands`].
    pub fn spawn_child(self, parent: &mut ChildSpawnerCommands) -> Entity {
        let (track_bg, track_border, justify) = Self::track_styles(self.is_on);
        let (thumb_bg, thumb_size) = Self::thumb_styles(self.is_on);

        let mut row_cmd = parent.spawn((
            (
                Button,
                Node {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    column_gap: SpacingScale::Small.val(),
                    min_height: Val::Px(40.0),
                    padding: UiRect::axes(Val::Px(4.0), Val::Px(4.0)),
                    ..default()
                },
                BackgroundColor(Color::NONE),
            ),
            M3Switch { is_on: self.is_on },
            Focusable { tab_index: 0 },
        ));

        if self.disabled {
            row_cmd.insert(Disabled);
        }

        let root = row_cmd.id();

        row_cmd.with_children(|child_parent| {
            // Track
            let mut track_cmd = child_parent.spawn((
                (Node {
                    width: Val::Px(52.0),
                    height: Val::Px(32.0),
                    padding: UiRect::all(Val::Px(4.0)),
                    display: Display::Flex,
                    align_items: AlignItems::Center,
                    justify_content: justify,
                    border: if track_border.is_some() {
                        UiRect::all(Val::Px(2.0))
                    } else {
                        UiRect::DEFAULT
                    },
                    ..default()
                }),
                SwitchTrack(root),
                BackgroundRole(track_bg),
                RadiusToken(RadiusLevel::Full),
            ));

            if let Some(border_role) = track_border {
                track_cmd.insert(BorderRole(border_role));
            }

            track_cmd.with_children(|track_parent| {
                // Thumb
                track_parent.spawn((
                    (Node {
                        width: Val::Px(thumb_size),
                        height: Val::Px(thumb_size),
                        ..default()
                    }),
                    SwitchThumb(root),
                    BackgroundRole(thumb_bg),
                    RadiusToken(RadiusLevel::Full),
                ));
            });

            // Optional label
            if let Some(lbl) = self.label {
                child_parent.spawn((
                    (
                        Text::new(lbl),
                        TextFont {
                            font_size: FontSize::Px(14.0),
                            ..default()
                        },
                        TextColor::default(),
                    ),
                    TypeStyleToken(TypeScale::BodyMedium),
                    ForegroundRole(ColorRole::OnSurface),
                ));
            }
        });

        root
    }
}

/// System toggling a switch when it is [`Activated`].
pub fn switch_toggle_system(
    mut activated: MessageReader<Activated>,
    mut switches: Query<&mut M3Switch, Without<Disabled>>,
    mut events: MessageWriter<SwitchToggled>,
) {
    for Activated(entity) in activated.read() {
        if let Ok(mut switch) = switches.get_mut(*entity) {
            switch.is_on = !switch.is_on;
            events.write(SwitchToggled {
                entity: *entity,
                is_on: switch.is_on,
            });
        }
    }
}

/// System synchronizing switch visual track and thumb nodes when switch state changes.
pub fn switch_sync_system(
    changed_query: Query<(Entity, &M3Switch), Changed<M3Switch>>,
    mut track_query: Query<(&SwitchTrack, &mut Node, &mut BackgroundRole), Without<SwitchThumb>>,
    mut thumb_query: Query<(&SwitchThumb, &mut Node, &mut BackgroundRole), Without<SwitchTrack>>,
) {
    for (entity, switch) in &changed_query {
        let (track_bg, _, justify) = SwitchBuilder::track_styles(switch.is_on);
        let (thumb_bg, thumb_size) = SwitchBuilder::thumb_styles(switch.is_on);

        for (track, mut style, mut bg_role) in &mut track_query {
            if track.0 == entity {
                bg_role.0 = track_bg;
                style.justify_content = justify;
            }
        }

        for (thumb, mut style, mut bg_role) in &mut thumb_query {
            if thumb.0 == entity {
                bg_role.0 = thumb_bg;
                style.width = Val::Px(thumb_size);
                style.height = Val::Px(thumb_size);
            }
        }
    }
}

/// Plugin registering Switch widget systems and events.
#[derive(Default)]
pub struct SwitchPlugin;

impl Plugin for SwitchPlugin {
    fn build(&self, app: &mut App) {
        if !app.world().contains_resource::<Messages<SwitchToggled>>() {
            app.add_message::<SwitchToggled>();
        }
        app.add_systems(Update, (switch_toggle_system, switch_sync_system));
    }
}

#[cfg(test)]
mod tests;
