//! Material Design 3 Icon widget for scalable graphical glyphs and icons.

use crate::{theme::color::ColorRole, tokens::components::ForegroundRole};
use bevy::prelude::*;

/// Standard sizing variants for icons.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum IconSize {
    Small,
    #[default]
    Medium,
    Large,
}

impl IconSize {
    /// Nominal icon size in pixels.
    pub const fn px(&self) -> f32 {
        match self {
            Self::Small => 18.0,
            Self::Medium => 24.0,
            Self::Large => 32.0,
        }
    }
}

/// Marker component for an M3 icon root entity.
#[derive(Component, Debug, Clone, Default)]
pub struct M3Icon {
    pub size: IconSize,
}

/// Builder for constructing Material Design 3 icon widgets.
///
/// # Example
/// ```rust
/// use opts_gui::prelude::*;
/// use bevy::prelude::*;
///
/// fn setup_icon(mut commands: Commands, server: Res<AssetServer>) {
///     IconBuilder::new(server.load("icons/bus.png"))
///         .with_size(IconSize::Large)
///         .spawn(&mut commands);
/// }
/// ```
#[derive(Debug, Clone)]
pub struct IconBuilder {
    pub image: Handle<Image>,
    pub size: IconSize,
    pub color: ColorRole,
}

impl IconBuilder {
    /// Creates a new icon builder with medium size and default icon color.
    pub fn new(image: Handle<Image>) -> Self {
        Self {
            image,
            size: IconSize::Medium,
            color: ColorRole::OnSurfaceVariant,
        }
    }

    /// Sets the icon dimension size variant.
    pub fn with_size(mut self, size: IconSize) -> Self {
        self.size = size;
        self
    }

    /// Sets the icon foreground tint color role.
    pub fn with_color(mut self, color: ColorRole) -> Self {
        self.color = color;
        self
    }

    /// Creates the component bundle tuple for this icon.
    pub fn bundle(self) -> impl Bundle {
        let size_px = self.size.px();
        (
            (
                ImageNode::new(self.image),
                Node {
                    width: Val::Px(size_px),
                    height: Val::Px(size_px),
                    ..default()
                },
            ),
            M3Icon { size: self.size },
            ForegroundRole(self.color),
        )
    }

    /// Spawns the icon entity into the Bevy ECS world using top-level [`Commands`].
    pub fn spawn(self, commands: &mut Commands) -> Entity {
        commands.spawn(self.bundle()).id()
    }

    /// Spawns the icon entity as a child inside a parent [`ChildSpawnerCommands`].
    pub fn spawn_child(self, parent: &mut ChildSpawnerCommands) -> Entity {
        parent.spawn(self.bundle()).id()
    }
}

/// Plugin registering Icon widget systems.
#[derive(Default)]
pub struct IconPlugin;

impl Plugin for IconPlugin {
    fn build(&self, _app: &mut App) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_icon_builder_spawn() {
        let mut app = App::new();
        let entity = {
            let mut commands = app.world_mut().commands();
            IconBuilder::new(Handle::default())
                .with_size(IconSize::Large)
                .with_color(ColorRole::Primary)
                .spawn(&mut commands)
        };

        app.update();

        let icon = app.world().get::<M3Icon>(entity);
        assert_eq!(icon.map(|i| i.size), Some(IconSize::Large));

        let fg = app.world().get::<ForegroundRole>(entity);
        assert_eq!(fg.map(|f| f.0), Some(ColorRole::Primary));
    }
}
