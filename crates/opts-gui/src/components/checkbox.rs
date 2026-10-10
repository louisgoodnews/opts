//! Material Design 3 Checkbox widget.

use bevy::prelude::*;

use crate::{
    interaction::{
        events::CheckboxToggled,
        focus::Focusable,
        state::{CheckState, Disabled},
    },
    theme::{color::ColorRole, shape::RadiusLevel, spacing::SpacingScale, typography::TypeScale},
    tokens::components::{BackgroundRole, BorderRole, ForegroundRole, RadiusToken, TypeStyleToken},
};

/// Marker component for an M3 Checkbox root entity.
#[derive(Component, Debug, Clone, Default)]
pub struct M3Checkbox {
    pub state: CheckState,
}

/// Marker component for the internal indicator box of a checkbox.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct CheckboxIndicator(pub Entity);

/// Marker component for the glyph text inside the indicator.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct CheckboxGlyph(pub Entity);

/// Builder for constructing Material Design 3 checkboxes with optional labels.
///
/// # Example
/// ```rust
/// use opts_gui::prelude::*;
/// use bevy::prelude::*;
///
/// fn spawn_checkbox(mut commands: Commands) {
///     CheckboxBuilder::new("Enable timetable sync")
///         .checked(true)
///         .spawn(&mut commands);
/// }
/// ```
#[derive(Debug, Clone)]
pub struct CheckboxBuilder {
    pub label: Option<String>,
    pub state: CheckState,
    pub disabled: bool,
}

impl Default for CheckboxBuilder {
    fn default() -> Self {
        Self {
            label: None,
            state: CheckState::Unchecked,
            disabled: false,
        }
    }
}

impl CheckboxBuilder {
    /// Creates a new checkbox builder without a label.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: Some(label.into()),
            state: CheckState::Unchecked,
            disabled: false,
        }
    }

    /// Creates an standalone checkbox builder without an attached label.
    pub fn standalone() -> Self {
        Self::default()
    }

    /// Sets the initial check state.
    pub fn with_state(mut self, state: CheckState) -> Self {
        self.state = state;
        self
    }

    /// Sets whether the checkbox is checked.
    pub fn checked(mut self, is_checked: bool) -> Self {
        self.state = if is_checked {
            CheckState::Checked
        } else {
            CheckState::Unchecked
        };
        self
    }

    /// Sets whether the checkbox is indeterminate.
    pub fn indeterminate(mut self, is_indeterminate: bool) -> Self {
        if is_indeterminate {
            self.state = CheckState::Indeterminate;
        }
        self
    }

    /// Sets whether the checkbox is disabled.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    fn glyph_char(state: CheckState) -> &'static str {
        match state {
            CheckState::Checked => "✓",
            CheckState::Indeterminate => "−",
            CheckState::Unchecked => "",
        }
    }

    fn indicator_roles(state: CheckState) -> (ColorRole, ColorRole, ColorRole) {
        match state {
            CheckState::Checked | CheckState::Indeterminate => {
                (ColorRole::Primary, ColorRole::Primary, ColorRole::OnPrimary)
            }
            CheckState::Unchecked => (ColorRole::Surface, ColorRole::Outline, ColorRole::Primary),
        }
    }

    /// Spawns the checkbox into the Bevy ECS world using top-level [`Commands`].
    pub fn spawn(self, commands: &mut Commands) -> Entity {
        let (bg_role, border_role, fg_role) = Self::indicator_roles(self.state);
        let glyph_str = Self::glyph_char(self.state);

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
            M3Checkbox { state: self.state },
            Focusable { tab_index: 0 },
        ));

        if self.disabled {
            row_cmd.insert(Disabled);
        }

        let root = row_cmd.id();

        commands.entity(root).with_children(|parent| {
            // Box indicator
            parent
                .spawn((
                    (Node {
                        width: Val::Px(18.0),
                        height: Val::Px(18.0),
                        border: UiRect::all(Val::Px(2.0)),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    }),
                    CheckboxIndicator(root),
                    BackgroundRole(bg_role),
                    BorderRole(border_role),
                    RadiusToken(RadiusLevel::ExtraSmall),
                ))
                .with_children(|box_parent| {
                    box_parent.spawn((
                        (
                            Text::new(glyph_str),
                            TextFont {
                                font_size: FontSize::Px(13.0),
                                ..default()
                            },
                            TextColor(Color::WHITE),
                        ),
                        CheckboxGlyph(root),
                        ForegroundRole(fg_role),
                        TypeStyleToken(TypeScale::LabelSmall),
                    ));
                });

            // Optional label text
            if let Some(lbl) = self.label {
                parent.spawn((
                    (
                        Text::new(lbl),
                        TextFont {
                            font_size: FontSize::Px(14.0),
                            ..default()
                        },
                        TextColor(Color::WHITE),
                    ),
                    TypeStyleToken(TypeScale::BodyMedium),
                    ForegroundRole(ColorRole::OnSurface),
                ));
            }
        });

        root
    }

    /// Spawns the checkbox as a child inside a parent [`ChildSpawnerCommands`].
    pub fn spawn_child(self, parent: &mut ChildSpawnerCommands) -> Entity {
        let (bg_role, border_role, fg_role) = Self::indicator_roles(self.state);
        let glyph_str = Self::glyph_char(self.state);

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
            M3Checkbox { state: self.state },
            Focusable { tab_index: 0 },
        ));

        if self.disabled {
            row_cmd.insert(Disabled);
        }

        let root = row_cmd.id();

        row_cmd.with_children(|child_parent| {
            // Box indicator
            child_parent
                .spawn((
                    (Node {
                        width: Val::Px(18.0),
                        height: Val::Px(18.0),
                        border: UiRect::all(Val::Px(2.0)),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    }),
                    CheckboxIndicator(root),
                    BackgroundRole(bg_role),
                    BorderRole(border_role),
                    RadiusToken(RadiusLevel::ExtraSmall),
                ))
                .with_children(|box_parent| {
                    box_parent.spawn((
                        (
                            Text::new(glyph_str),
                            TextFont {
                                font_size: FontSize::Px(13.0),
                                ..default()
                            },
                            TextColor(Color::WHITE),
                        ),
                        CheckboxGlyph(root),
                        ForegroundRole(fg_role),
                        TypeStyleToken(TypeScale::LabelSmall),
                    ));
                });

            // Optional label text
            if let Some(lbl) = self.label {
                child_parent.spawn((
                    (
                        Text::new(lbl),
                        TextFont {
                            font_size: FontSize::Px(14.0),
                            ..default()
                        },
                        TextColor(Color::WHITE),
                    ),
                    TypeStyleToken(TypeScale::BodyMedium),
                    ForegroundRole(ColorRole::OnSurface),
                ));
            }
        });

        root
    }
}

/// Query item alias for checkbox click interaction.
pub type CheckboxClickItem<'a> = (Entity, &'a Interaction, &'a mut M3Checkbox);

/// System handling click interactions on checkboxes and toggling their state.
pub fn checkbox_toggle_system(
    mut interaction_query: Query<CheckboxClickItem, (Without<Disabled>, Changed<Interaction>)>,
    mut events: MessageWriter<CheckboxToggled>,
) {
    for (entity, interaction, mut checkbox) in &mut interaction_query {
        if *interaction == Interaction::Pressed {
            checkbox.state = match checkbox.state {
                CheckState::Unchecked => CheckState::Checked,
                CheckState::Checked => CheckState::Unchecked,
                CheckState::Indeterminate => CheckState::Checked,
            };
            events.write(CheckboxToggled {
                entity,
                state: checkbox.state,
            });
        }
    }
}

/// System synchronizing checkbox visual indicator nodes when checkbox state changes.
pub fn checkbox_sync_system(
    changed_query: Query<(Entity, &M3Checkbox), Changed<M3Checkbox>>,
    mut indicator_query: Query<(&CheckboxIndicator, &mut BackgroundRole, &mut BorderRole)>,
    mut glyph_query: Query<(&CheckboxGlyph, &mut Text, &mut ForegroundRole)>,
) {
    for (entity, checkbox) in &changed_query {
        let (bg, border, fg) = CheckboxBuilder::indicator_roles(checkbox.state);
        let glyph_str = CheckboxBuilder::glyph_char(checkbox.state);

        for (indicator, mut bg_role, mut border_role) in &mut indicator_query {
            if indicator.0 == entity {
                bg_role.0 = bg;
                border_role.0 = border;
            }
        }

        for (glyph, mut text, mut fg_role) in &mut glyph_query {
            if glyph.0 == entity {
                text.0 = glyph_str.to_string();
                fg_role.0 = fg;
            }
        }
    }
}

/// Plugin registering Checkbox widget systems and events.
#[derive(Default)]
pub struct CheckboxPlugin;

impl Plugin for CheckboxPlugin {
    fn build(&self, app: &mut App) {
        if !app.world().contains_resource::<Messages<CheckboxToggled>>() {
            app.add_message::<CheckboxToggled>();
        }
        app.add_systems(Update, (checkbox_toggle_system, checkbox_sync_system));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_checkbox_builder_spawn() {
        let mut app = App::new();
        let entity = {
            let mut commands = app.world_mut().commands();
            CheckboxBuilder::new("Test option")
                .checked(true)
                .spawn(&mut commands)
        };

        app.update();

        let checkbox = app.world().get::<M3Checkbox>(entity);
        assert_eq!(checkbox.map(|c| c.state), Some(CheckState::Checked));
    }

    #[test]
    fn test_checkbox_toggle_system() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_message::<CheckboxToggled>()
            .add_systems(Update, checkbox_toggle_system);

        let entity = {
            let mut commands = app.world_mut().commands();
            CheckboxBuilder::standalone().spawn(&mut commands)
        };

        app.update();

        // Simulate press interaction
        if let Some(mut interaction) = app.world_mut().get_mut::<Interaction>(entity) {
            *interaction = Interaction::Pressed;
        }

        app.update();

        let checkbox = app.world().get::<M3Checkbox>(entity);
        assert_eq!(checkbox.map(|c| c.state), Some(CheckState::Checked));

        let events = app.world().resource::<Messages<CheckboxToggled>>();
        let mut reader = events.get_cursor();
        let received = reader.read(events).find(|e| e.entity == entity);
        assert_eq!(received.map(|e| e.state), Some(CheckState::Checked));
    }
}
