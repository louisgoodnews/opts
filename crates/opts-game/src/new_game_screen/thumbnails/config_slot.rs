//! The picture slot of a saved bus configuration (M-0077): the preview image of the
//! configuration's vehicle when the content has one, otherwise the class glyph of the vehicle.
//! The slot is a row thumbnail (`veh:<id>`), so [`fill_thumbnails`](super::fill_thumbnails)
//! fills it like the thumbnails of the vehicle list and the cache is shared with them.

use bevy::prelude::*;
use opts_gui::patterns::RowThumb;
use opts_gui::prelude::*;

use super::HeroGlyph;
use crate::new_game_screen::step_what::class_glyph;
use crate::new_game_screen::ui::glyph;

/// Side of the slot in pixels.
pub const CONFIG_THUMB_PX: f32 = 48.0;

/// The thumbnail key of a configuration's vehicle (the catalogue id).
#[must_use]
pub fn config_thumb_key(vehicle_id: &str) -> String {
    format!("veh:{vehicle_id}")
}

/// A square slot for the vehicle `vehicle_id` of class `class` (empty for an unknown class: the
/// generic picture glyph); the glyph is removed when an image arrives.
pub fn spawn_config_thumb(commands: &mut Commands, vehicle_id: &str, class: &str) -> Entity {
    let slot = commands
        .spawn((
            Node {
                width: Val::Px(CONFIG_THUMB_PX),
                height: Val::Px(CONFIG_THUMB_PX),
                flex_shrink: 0.0,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundRole(ColorRole::SurfaceContainerHigh),
            RadiusToken(RadiusLevel::Small),
            RowThumb {
                root: Entity::PLACEHOLDER,
                key: config_thumb_key(vehicle_id),
            },
            Name::new("ConfigThumb"),
        ))
        .id();
    let placeholder = commands
        .spawn((
            glyph(class_glyph(class), 24.0, ColorRole::OnSurfaceVariant),
            HeroGlyph,
        ))
        .id();
    commands.entity(slot).add_child(placeholder);
    slot
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_slot_asks_for_the_vehicles_thumbnail_and_carries_a_glyph() {
        let mut world = World::new();
        let slot = {
            let mut commands = world.commands();
            spawn_config_thumb(&mut commands, "citybus", "road")
        };
        world.flush();
        let key = world.get::<RowThumb>(slot).map(|t| t.key.clone());
        assert_eq!(key.as_deref(), Some("veh:citybus"));
        let kids = world.get::<Children>(slot).expect("glyph child");
        assert_eq!(kids.len(), 1);
        assert!(world.get::<HeroGlyph>(kids[0]).is_some());
    }
}
