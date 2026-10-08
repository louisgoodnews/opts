//! Material Design 3 DateSelect widget for picking dates.

use crate::interaction::events::DateSelected;
use bevy::prelude::*;

/// Marker component for an M3 date selector.
#[derive(Component, Debug, Clone, PartialEq, Eq)]
pub struct M3DateSelect {
    pub year: i32,
    pub month: u32,
    pub day: u32,
}

impl Default for M3DateSelect {
    fn default() -> Self {
        Self {
            year: 2026,
            month: 10,
            day: 8,
        }
    }
}

/// Builder for constructing a DateSelect widget.
#[derive(Debug, Clone)]
pub struct DateSelectBuilder {
    pub year: i32,
    pub month: u32,
    pub day: u32,
}

impl DateSelectBuilder {
    /// Creates a new date selector with the given initial date.
    pub fn new(year: i32, month: u32, day: u32) -> Self {
        Self {
            year,
            month: month.clamp(1, 12),
            day: day.clamp(1, 31),
        }
    }

    /// Spawns the date selector container and its child controls.
    pub fn spawn(self, commands: &mut Commands) -> Entity {
        let root = commands
            .spawn((
                NodeBundle {
                    style: Style {
                        display: Display::Flex,
                        flex_direction: FlexDirection::Row,
                        column_gap: Val::Px(8.0),
                        align_items: AlignItems::Center,
                        padding: UiRect::all(Val::Px(8.0)),
                        ..default()
                    },
                    ..default()
                },
                M3DateSelect {
                    year: self.year,
                    month: self.month,
                    day: self.day,
                },
            ))
            .id();

        let formatted_date = format!("{:04}-{:02}-{:02}", self.year, self.month, self.day);
        commands.entity(root).with_children(|parent| {
            parent.spawn(TextBundle::from_section(
                formatted_date,
                TextStyle {
                    font_size: 14.0,
                    color: Color::WHITE,
                    ..default()
                },
            ));
        });

        root
    }

    /// Spawns the date selector as a child inside a parent [`ChildBuilder`].
    pub fn spawn_child(self, parent: &mut ChildBuilder) -> Entity {
        let formatted_date = format!("{:04}-{:02}-{:02}", self.year, self.month, self.day);
        let mut root_cmd = parent.spawn((
            NodeBundle {
                style: Style {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Row,
                    column_gap: Val::Px(8.0),
                    align_items: AlignItems::Center,
                    padding: UiRect::all(Val::Px(8.0)),
                    ..default()
                },
                ..default()
            },
            M3DateSelect {
                year: self.year,
                month: self.month,
                day: self.day,
            },
        ));

        let root = root_cmd.id();

        root_cmd.with_children(|child_parent| {
            child_parent.spawn(TextBundle::from_section(
                formatted_date,
                TextStyle {
                    font_size: 14.0,
                    color: Color::WHITE,
                    ..default()
                },
            ));
        });

        root
    }
}

/// System that detects changes in [`M3DateSelect`] entities and emits [`DateSelected`].
pub fn date_select_system(
    query: Query<(Entity, &M3DateSelect), Changed<M3DateSelect>>,
    mut events: EventWriter<DateSelected>,
) {
    for (entity, date) in &query {
        events.send(DateSelected {
            entity,
            year: date.year,
            month: date.month,
            day: date.day,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_date_select_event_emission() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_event::<DateSelected>()
            .add_systems(Update, date_select_system);

        let entity = {
            let mut commands = app.world_mut().commands();
            DateSelectBuilder::new(2026, 10, 8).spawn(&mut commands)
        };

        app.update();

        // Mutate the date
        if let Some(mut date) = app.world_mut().get_mut::<M3DateSelect>(entity) {
            date.day = 9;
        }

        app.update();

        let events = app.world().resource::<Events<DateSelected>>();
        let mut reader = events.get_reader();
        let received = reader.read(events).filter(|e| e.entity == entity).last();
        assert_eq!(
            received.map(|e| (e.year, e.month, e.day)),
            Some((2026, 10, 9))
        );
    }
}
