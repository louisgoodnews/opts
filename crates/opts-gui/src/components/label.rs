//! Material Design 3 Label widget for typography display.

use crate::{
    theme::{color::ColorRole, typography::TypeScale},
    tokens::components::{ForegroundRole, TypeStyleToken},
};
use bevy::prelude::*;

/// Marker component for an M3 typography label.
#[derive(Component, Debug, Clone, Default)]
pub struct M3Label;

/// Builder for constructing Material Design 3 typography labels.
///
/// # Example
/// ```rust
/// use opts_gui::prelude::*;
/// use bevy::prelude::*;
///
/// fn setup_header(mut commands: Commands) {
///     LabelBuilder::new("Map Browser")
///         .with_style(TypeScale::HeadlineLarge)
///         .spawn(&mut commands);
/// }
/// ```
#[derive(Debug, Clone)]
pub struct LabelBuilder {
    pub text: String,
    pub style: TypeScale,
    pub color: ColorRole,
}

impl LabelBuilder {
    /// Creates a new label builder with default body text styling.
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            style: TypeScale::BodyMedium,
            color: ColorRole::OnSurface,
        }
    }

    /// Sets the typography scale role.
    pub fn with_style(mut self, style: TypeScale) -> Self {
        self.style = style;
        self
    }

    /// Sets the semantic foreground color role.
    pub fn with_color(mut self, color: ColorRole) -> Self {
        self.color = color;
        self
    }

    /// Creates the component bundle tuple for this label.
    pub fn bundle(self) -> impl Bundle {
        (
            TextBundle::from_section(
                self.text,
                TextStyle {
                    font_size: 14.0,
                    color: Color::WHITE,
                    ..default()
                },
            ),
            M3Label,
            TypeStyleToken(self.style),
            ForegroundRole(self.color),
        )
    }

    /// Spawns the label into the Bevy ECS world using top-level [`Commands`].
    pub fn spawn(self, commands: &mut Commands) -> Entity {
        commands.spawn(self.bundle()).id()
    }

    /// Spawns the label as a child inside a parent [`ChildBuilder`].
    pub fn spawn_child(self, parent: &mut ChildBuilder) -> Entity {
        parent.spawn(self.bundle()).id()
    }
}

/// Plugin registering Label widget systems.
#[derive(Default)]
pub struct LabelPlugin;

impl Plugin for LabelPlugin {
    fn build(&self, _app: &mut App) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_label_builder_spawn() {
        let mut app = App::new();
        let entity = {
            let mut commands = app.world_mut().commands();
            LabelBuilder::new("Berlin")
                .with_style(TypeScale::TitleLarge)
                .with_color(ColorRole::Primary)
                .spawn(&mut commands)
        };

        app.update();

        let style = app.world().get::<TypeStyleToken>(entity);
        assert_eq!(style.map(|s| s.0), Some(TypeScale::TitleLarge));

        let fg = app.world().get::<ForegroundRole>(entity);
        assert_eq!(fg.map(|f| f.0), Some(ColorRole::Primary));
    }
}
