//! Material Design 3 Radio button widget.

use bevy::prelude::*;

use crate::{
    interaction::{events::RadioSelected, focus::Focusable, state::Disabled},
    theme::{color::ColorRole, shape::RadiusLevel, spacing::SpacingScale, typography::TypeScale},
    tokens::components::{BackgroundRole, BorderRole, ForegroundRole, RadiusToken, TypeStyleToken},
};

/// Marker component for an M3 Radio root entity.
#[derive(Component, Debug, Clone, Default)]
pub struct M3Radio {
    pub is_selected: bool,
    pub group_id: Option<u32>,
}

/// Marker component linking an internal radio outer circle to its root entity.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct RadioOuterRing(pub Entity);

/// Marker component linking an internal radio dot to its root entity.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct RadioInnerDot(pub Entity);

/// Builder for constructing Material Design 3 radio buttons.
///
/// # Example
/// ```rust
/// use opts_gui::prelude::*;
/// use bevy::prelude::*;
///
/// fn spawn_radio(mut commands: Commands) {
///     RadioBuilder::new("Express Line")
///         .with_group(1)
///         .selected(true)
///         .spawn(&mut commands);
/// }
/// ```
#[derive(Debug, Clone, Default)]
pub struct RadioBuilder {
    pub label: Option<String>,
    pub is_selected: bool,
    pub group_id: Option<u32>,
    pub disabled: bool,
}

impl RadioBuilder {
    /// Creates a new radio builder with an associated label.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: Some(label.into()),
            is_selected: false,
            group_id: None,
            disabled: false,
        }
    }

    /// Creates a standalone radio builder without a label.
    pub fn standalone() -> Self {
        Self::default()
    }

    /// Sets whether the radio button is initially selected.
    pub fn selected(mut self, is_selected: bool) -> Self {
        self.is_selected = is_selected;
        self
    }

    /// Sets the group ID for mutual exclusion.
    pub fn with_group(mut self, group_id: u32) -> Self {
        self.group_id = Some(group_id);
        self
    }

    /// Sets whether the radio button is disabled.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    fn ring_border(is_selected: bool) -> ColorRole {
        if is_selected {
            ColorRole::Primary
        } else {
            ColorRole::Outline
        }
    }

    fn dot_bg(is_selected: bool) -> ColorRole {
        if is_selected {
            ColorRole::Primary
        } else {
            ColorRole::Surface
        }
    }

    /// Spawns the radio button into the Bevy ECS world using top-level [`Commands`].
    pub fn spawn(self, commands: &mut Commands) -> Entity {
        let border_role = Self::ring_border(self.is_selected);
        let dot_bg = Self::dot_bg(self.is_selected);

        let mut row_cmd = commands.spawn((
            ButtonBundle {
                style: Style {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    column_gap: SpacingScale::Small.val(),
                    min_height: Val::Px(40.0),
                    padding: UiRect::axes(Val::Px(4.0), Val::Px(4.0)),
                    ..default()
                },
                background_color: BackgroundColor(Color::NONE),
                ..default()
            },
            M3Radio {
                is_selected: self.is_selected,
                group_id: self.group_id,
            },
            Focusable { tab_index: 0 },
        ));

        if self.disabled {
            row_cmd.insert(Disabled);
        }

        let root = row_cmd.id();

        commands.entity(root).with_children(|parent| {
            // Outer ring
            parent
                .spawn((
                    NodeBundle {
                        style: Style {
                            width: Val::Px(20.0),
                            height: Val::Px(20.0),
                            border: UiRect::all(Val::Px(2.0)),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        ..default()
                    },
                    RadioOuterRing(root),
                    BackgroundRole(ColorRole::Surface),
                    BorderRole(border_role),
                    RadiusToken(RadiusLevel::Full),
                ))
                .with_children(|ring_parent| {
                    // Inner dot
                    ring_parent.spawn((
                        NodeBundle {
                            style: Style {
                                width: Val::Px(10.0),
                                height: Val::Px(10.0),
                                display: if self.is_selected {
                                    Display::Flex
                                } else {
                                    Display::None
                                },
                                ..default()
                            },
                            ..default()
                        },
                        RadioInnerDot(root),
                        BackgroundRole(dot_bg),
                        RadiusToken(RadiusLevel::Full),
                    ));
                });

            // Optional label
            if let Some(lbl) = self.label {
                parent.spawn((
                    TextBundle::from_section(
                        lbl,
                        TextStyle {
                            font_size: 14.0,
                            color: Color::WHITE,
                            ..default()
                        },
                    ),
                    TypeStyleToken(TypeScale::BodyMedium),
                    ForegroundRole(ColorRole::OnSurface),
                ));
            }
        });

        root
    }

    /// Spawns the radio button as a child inside a parent [`ChildBuilder`].
    pub fn spawn_child(self, parent: &mut ChildBuilder) -> Entity {
        let border_role = Self::ring_border(self.is_selected);
        let dot_bg = Self::dot_bg(self.is_selected);

        let mut row_cmd = parent.spawn((
            ButtonBundle {
                style: Style {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    column_gap: SpacingScale::Small.val(),
                    min_height: Val::Px(40.0),
                    padding: UiRect::axes(Val::Px(4.0), Val::Px(4.0)),
                    ..default()
                },
                background_color: BackgroundColor(Color::NONE),
                ..default()
            },
            M3Radio {
                is_selected: self.is_selected,
                group_id: self.group_id,
            },
            Focusable { tab_index: 0 },
        ));

        if self.disabled {
            row_cmd.insert(Disabled);
        }

        let root = row_cmd.id();

        row_cmd.with_children(|child_parent| {
            // Outer ring
            child_parent
                .spawn((
                    NodeBundle {
                        style: Style {
                            width: Val::Px(20.0),
                            height: Val::Px(20.0),
                            border: UiRect::all(Val::Px(2.0)),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        ..default()
                    },
                    RadioOuterRing(root),
                    BackgroundRole(ColorRole::Surface),
                    BorderRole(border_role),
                    RadiusToken(RadiusLevel::Full),
                ))
                .with_children(|ring_parent| {
                    // Inner dot
                    ring_parent.spawn((
                        NodeBundle {
                            style: Style {
                                width: Val::Px(10.0),
                                height: Val::Px(10.0),
                                display: if self.is_selected {
                                    Display::Flex
                                } else {
                                    Display::None
                                },
                                ..default()
                            },
                            ..default()
                        },
                        RadioInnerDot(root),
                        BackgroundRole(dot_bg),
                        RadiusToken(RadiusLevel::Full),
                    ));
                });

            // Optional label
            if let Some(lbl) = self.label {
                child_parent.spawn((
                    TextBundle::from_section(
                        lbl,
                        TextStyle {
                            font_size: 14.0,
                            color: Color::WHITE,
                            ..default()
                        },
                    ),
                    TypeStyleToken(TypeScale::BodyMedium),
                    ForegroundRole(ColorRole::OnSurface),
                ));
            }
        });

        root
    }
}

/// Query item alias for radio button click interaction.
pub type RadioClickItem<'a> = (Entity, &'a Interaction, &'a mut M3Radio);

/// System handling radio button click selection and mutual exclusion within groups.
#[allow(clippy::type_complexity)]
pub fn radio_select_system(
    mut params: ParamSet<(
        Query<RadioClickItem, (Without<Disabled>, Changed<Interaction>)>,
        Query<(Entity, &mut M3Radio), Without<Disabled>>,
    )>,
    mut events: EventWriter<RadioSelected>,
) {
    let mut selected_entities = Vec::new();

    for (entity, interaction, mut radio) in &mut params.p0() {
        if *interaction == Interaction::Pressed && !radio.is_selected {
            radio.is_selected = true;
            selected_entities.push((entity, radio.group_id));
            events.send(RadioSelected { entity });
        }
    }

    // Mutual exclusion: deselect other radios in same group
    for (clicked_entity, group_id) in selected_entities {
        if let Some(gid) = group_id {
            for (other_entity, mut other_radio) in &mut params.p1() {
                if other_entity != clicked_entity && other_radio.group_id == Some(gid) {
                    other_radio.is_selected = false;
                }
            }
        }
    }
}

/// System synchronizing radio visual ring and inner dot when radio state changes.
pub fn radio_sync_system(
    changed_query: Query<(Entity, &M3Radio), Changed<M3Radio>>,
    mut ring_query: Query<(&RadioOuterRing, &mut BorderRole)>,
    mut dot_query: Query<(&RadioInnerDot, &mut Style, &mut BackgroundRole)>,
) {
    for (entity, radio) in &changed_query {
        let border_role = RadioBuilder::ring_border(radio.is_selected);
        let dot_bg = RadioBuilder::dot_bg(radio.is_selected);

        for (ring, mut border) in &mut ring_query {
            if ring.0 == entity {
                border.0 = border_role;
            }
        }

        for (dot, mut style, mut bg_role) in &mut dot_query {
            if dot.0 == entity {
                bg_role.0 = dot_bg;
                style.display = if radio.is_selected {
                    Display::Flex
                } else {
                    Display::None
                };
            }
        }
    }
}

/// Plugin registering Radio widget systems and events.
#[derive(Default)]
pub struct RadioPlugin;

impl Plugin for RadioPlugin {
    fn build(&self, app: &mut App) {
        if !app.world().contains_resource::<Events<RadioSelected>>() {
            app.add_event::<RadioSelected>();
        }
        app.add_systems(Update, (radio_select_system, radio_sync_system));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_radio_builder_spawn() {
        let mut app = App::new();
        let entity = {
            let mut commands = app.world_mut().commands();
            RadioBuilder::new("Option A")
                .with_group(1)
                .selected(true)
                .spawn(&mut commands)
        };

        app.update();

        let radio = app.world().get::<M3Radio>(entity);
        assert_eq!(radio.map(|r| r.is_selected), Some(true));
        assert_eq!(radio.map(|r| r.group_id), Some(Some(1)));
    }

    #[test]
    fn test_radio_mutual_exclusion() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_event::<RadioSelected>()
            .add_systems(Update, radio_select_system);

        let entity_a = {
            let mut commands = app.world_mut().commands();
            RadioBuilder::new("Option A")
                .with_group(1)
                .selected(true)
                .spawn(&mut commands)
        };

        let entity_b = {
            let mut commands = app.world_mut().commands();
            RadioBuilder::new("Option B")
                .with_group(1)
                .selected(false)
                .spawn(&mut commands)
        };

        app.update();

        // Simulate press on Option B
        if let Some(mut interaction) = app.world_mut().get_mut::<Interaction>(entity_b) {
            *interaction = Interaction::Pressed;
        }

        app.update();

        let radio_a = app.world().get::<M3Radio>(entity_a);
        assert_eq!(radio_a.map(|r| r.is_selected), Some(false));

        let radio_b = app.world().get::<M3Radio>(entity_b);
        assert_eq!(radio_b.map(|r| r.is_selected), Some(true));
    }
}
