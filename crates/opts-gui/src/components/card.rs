//! Material Design 3 Card container widget.

use bevy::prelude::*;

use crate::{
    theme::{
        color::ColorRole, elevation::ElevationLevel, shape::RadiusLevel, spacing::SpacingScale,
    },
    tokens::components::{BackgroundRole, BorderRole, ElevationToken, RadiusToken},
};

/// Card variants defined by Material Design 3.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CardVariant {
    #[default]
    Filled,
    Outlined,
    Elevated,
}

/// Marker component for an M3 Card container root entity.
#[derive(Component, Debug, Clone, Default)]
pub struct M3Card {
    pub variant: CardVariant,
}

/// Builder for constructing Material Design 3 card containers.
///
/// # Example
/// ```rust
/// use opts_gui::prelude::*;
/// use bevy::prelude::*;
///
/// fn spawn_my_card(mut commands: Commands) {
///     CardBuilder::elevated()
///         .with_padding(SpacingScale::Standard)
///         .spawn(&mut commands);
/// }
/// ```
#[derive(Debug, Clone)]
pub struct CardBuilder {
    pub variant: CardVariant,
    pub padding: SpacingScale,
}

impl Default for CardBuilder {
    fn default() -> Self {
        Self {
            variant: CardVariant::Elevated,
            padding: SpacingScale::Standard,
        }
    }
}

impl CardBuilder {
    /// Creates a new elevated card builder.
    pub fn elevated() -> Self {
        Self {
            variant: CardVariant::Elevated,
            padding: SpacingScale::Standard,
        }
    }

    /// Creates a new filled card builder.
    pub fn filled() -> Self {
        Self {
            variant: CardVariant::Filled,
            padding: SpacingScale::Standard,
        }
    }

    /// Creates a new outlined card builder.
    pub fn outlined() -> Self {
        Self {
            variant: CardVariant::Outlined,
            padding: SpacingScale::Standard,
        }
    }

    /// Sets inner card padding.
    pub fn with_padding(mut self, padding: SpacingScale) -> Self {
        self.padding = padding;
        self
    }

    fn roles(&self) -> (ColorRole, ElevationLevel) {
        match self.variant {
            CardVariant::Elevated => (ColorRole::SurfaceContainerLow, ElevationLevel::Level1),
            CardVariant::Filled => (ColorRole::SurfaceContainerHighest, ElevationLevel::Level0),
            CardVariant::Outlined => (ColorRole::Surface, ElevationLevel::Level0),
        }
    }

    fn style(&self) -> Node {
        Node {
            padding: UiRect::all(self.padding.val()),
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            border: if self.variant == CardVariant::Outlined {
                UiRect::all(Val::Px(1.0))
            } else {
                UiRect::DEFAULT
            },
            ..default()
        }
    }

    /// Spawns the card container into the Bevy ECS world using top-level [`Commands`].
    pub fn spawn(self, commands: &mut Commands) -> Entity {
        let (bg_role, elevation) = self.roles();
        let is_outlined = self.variant == CardVariant::Outlined;
        let mut entity_commands = commands.spawn((
            (self.style()),
            M3Card {
                variant: self.variant,
            },
            BackgroundRole(bg_role),
            RadiusToken(RadiusLevel::Medium),
            ElevationToken(elevation),
        ));

        if is_outlined {
            entity_commands.insert(BorderRole(ColorRole::OutlineVariant));
        }

        entity_commands.id()
    }

    /// Spawns the card container as a child inside a parent [`ChildSpawnerCommands`].
    pub fn spawn_child(self, parent: &mut ChildSpawnerCommands) -> Entity {
        let (bg_role, elevation) = self.roles();
        let is_outlined = self.variant == CardVariant::Outlined;
        let mut entity_commands = parent.spawn((
            (self.style()),
            M3Card {
                variant: self.variant,
            },
            BackgroundRole(bg_role),
            RadiusToken(RadiusLevel::Medium),
            ElevationToken(elevation),
        ));

        if is_outlined {
            entity_commands.insert(BorderRole(ColorRole::OutlineVariant));
        }

        entity_commands.id()
    }
}

/// Plugin registering Card widget systems.
#[derive(Default)]
pub struct CardPlugin;

impl Plugin for CardPlugin {
    fn build(&self, _app: &mut App) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_card_builder_spawn() {
        let mut app = App::new();
        let entity = {
            let mut commands = app.world_mut().commands();
            CardBuilder::outlined().spawn(&mut commands)
        };

        app.update();

        let card = app.world().get::<M3Card>(entity);
        assert_eq!(card.map(|c| c.variant), Some(CardVariant::Outlined));

        let border = app.world().get::<BorderRole>(entity);
        assert_eq!(border.map(|b| b.0), Some(ColorRole::OutlineVariant));
    }
}
