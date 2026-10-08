//! Material Design 3 DateTimeSelect widget for timetable and service departure setup.

use crate::interaction::events::DateTimeSelected;
use bevy::prelude::*;

/// Marker component for an M3 date and time selector.
#[derive(Component, Debug, Clone, PartialEq, Eq)]
pub struct M3DateTimeSelect {
    pub year: i32,
    pub month: u32,
    pub day: u32,
    pub hour: u32,
    pub minute: u32,
}

impl Default for M3DateTimeSelect {
    fn default() -> Self {
        Self {
            year: 2026,
            month: 10,
            day: 8,
            hour: 12,
            minute: 0,
        }
    }
}

/// Builder for constructing a DateTimeSelect widget.
#[derive(Debug, Clone)]
pub struct DateTimeSelectBuilder {
    pub year: i32,
    pub month: u32,
    pub day: u32,
    pub hour: u32,
    pub minute: u32,
}

impl DateTimeSelectBuilder {
    /// Creates a new date and time selector with the given initial timestamp.
    pub fn new(year: i32, month: u32, day: u32, hour: u32, minute: u32) -> Self {
        Self {
            year,
            month: month.clamp(1, 12),
            day: day.clamp(1, 31),
            hour: hour.clamp(0, 23),
            minute: minute.clamp(0, 59),
        }
    }

    /// Spawns the date/time selector container and its child controls.
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
                M3DateTimeSelect {
                    year: self.year,
                    month: self.month,
                    day: self.day,
                    hour: self.hour,
                    minute: self.minute,
                },
            ))
            .id();

        // Spawn child label displaying current format YYYY-MM-DD HH:MM
        let formatted_ts = format!(
            "{:04}-{:02}-{:02} {:02}:{:02}",
            self.year, self.month, self.day, self.hour, self.minute
        );
        commands.entity(root).with_children(|parent| {
            parent.spawn(TextBundle::from_section(
                formatted_ts,
                TextStyle {
                    font_size: 14.0,
                    color: Color::WHITE,
                    ..default()
                },
            ));
        });

        root
    }

    /// Spawns the datetime selector as a child inside a parent [`ChildBuilder`].
    pub fn spawn_child(self, parent: &mut ChildBuilder) -> Entity {
        let formatted_ts = format!(
            "{:04}-{:02}-{:02} {:02}:{:02}",
            self.year, self.month, self.day, self.hour, self.minute
        );
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
            M3DateTimeSelect {
                year: self.year,
                month: self.month,
                day: self.day,
                hour: self.hour,
                minute: self.minute,
            },
        ));

        let root = root_cmd.id();

        root_cmd.with_children(|child_parent| {
            child_parent.spawn(TextBundle::from_section(
                formatted_ts,
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

/// System that detects changes in [`M3DateTimeSelect`] entities and emits [`DateTimeSelected`].
pub fn datetime_select_system(
    query: Query<(Entity, &M3DateTimeSelect), Changed<M3DateTimeSelect>>,
    mut events: EventWriter<DateTimeSelected>,
) {
    for (entity, dt) in &query {
        events.send(DateTimeSelected {
            entity,
            year: dt.year,
            month: dt.month,
            day: dt.day,
            hour: dt.hour,
            minute: dt.minute,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_datetime_select_event_emission() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_event::<DateTimeSelected>()
            .add_systems(Update, datetime_select_system);

        let entity = {
            let mut commands = app.world_mut().commands();
            DateTimeSelectBuilder::new(2026, 10, 8, 14, 30).spawn(&mut commands)
        };

        app.update();

        // Mutate the datetime
        if let Some(mut dt) = app.world_mut().get_mut::<M3DateTimeSelect>(entity) {
            dt.minute = 45;
        }

        app.update();

        let events = app.world().resource::<Events<DateTimeSelected>>();
        let mut reader = events.get_reader();
        let received = reader.read(events).filter(|e| e.entity == entity).last();
        assert_eq!(
            received.map(|e| (e.year, e.month, e.day, e.hour, e.minute)),
            Some((2026, 10, 8, 14, 45))
        );
    }
}
