//! Spawning of a toast: its root node, the message, the optional action button.

use bevy::prelude::*;

use crate::components::Button;
use crate::interaction::focus::{FocusRegion, Landmark};
use crate::interaction::layer::StateLayerRole;
use crate::theme::{elevation::ElevationLevel, shape::RadiusLevel, typography::TypeScale};
use crate::tokens::components::{
    BackgroundRole, ElevationToken, ForegroundRole, RadiusToken, TypeStyleToken,
};

use super::{
    M3Toast, ToastActionButton, ToastBuilder, ToastCount, ToastDedupe, ToastSeq, ToastSeverity,
};

static NEXT_SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

impl ToastBuilder {
    fn root_bundle(&self) -> impl Bundle {
        let timer = self
            .duration_secs
            .map(|secs| Timer::from_seconds(secs, TimerMode::Once));
        (
            (
                Node {
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
                GlobalZIndex(100),
            ),
            M3Toast {
                message: self.message.clone(),
                timer,
            },
            FocusRegion(Landmark::Toasts),
            BackgroundRole(self.severity.roles().0),
            RadiusToken(RadiusLevel::ExtraSmall),
            ElevationToken(ElevationLevel::Level3),
            self.severity,
            ToastSeq(NEXT_SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed)),
            ToastCount(1),
        )
    }

    /// Spawns the toast into the Bevy ECS world using top-level [`Commands`].
    pub fn spawn(self, commands: &mut Commands) -> Entity {
        let dedupe = self.dedupe_key.clone();
        let message = self.message.clone();
        let action = self.action_label.clone();
        let severity = self.severity;
        let root = commands.spawn(self.root_bundle()).id();
        if let Some(key) = dedupe {
            commands.entity(root).insert(ToastDedupe(key));
        }
        commands.entity(root).with_children(|parent| {
            populate_toast_children(parent, root, &message, action.as_deref(), severity);
        });
        root
    }

    /// Spawns the toast as a child inside a parent [`ChildSpawnerCommands`].
    pub fn spawn_child(self, parent: &mut ChildSpawnerCommands) -> Entity {
        let message = self.message.clone();
        let action = self.action_label.clone();
        let severity = self.severity;
        let mut root_cmd = parent.spawn(self.root_bundle());
        let root = root_cmd.id();
        root_cmd.with_children(|child_parent| {
            populate_toast_children(child_parent, root, &message, action.as_deref(), severity);
        });
        root
    }
}

fn populate_toast_children(
    parent: &mut ChildSpawnerCommands,
    root: Entity,
    message: &str,
    action_label: Option<&str>,
    severity: ToastSeverity,
) {
    parent.spawn((
        (
            Text::new(message),
            TextFont {
                font_size: FontSize::Px(14.0),
                ..default()
            },
            TextColor::default(),
        ),
        ForegroundRole(severity.roles().1),
        TypeStyleToken(TypeScale::BodyMedium),
    ));

    if let Some(action) = action_label {
        parent
            .spawn((
                (
                    Button,
                    Node {
                        padding: UiRect::axes(Val::Px(12.0), Val::Px(4.0)),
                        min_width: Val::Px(48.0),
                        min_height: Val::Px(32.0),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        ..default()
                    },
                    BackgroundColor(Color::NONE),
                ),
                StateLayerRole(severity.roles().1),
                ToastActionButton(root),
            ))
            .with_children(|btn_parent| {
                btn_parent.spawn((
                    (
                        Text::new(action),
                        TextFont {
                            font_size: FontSize::Px(14.0),
                            ..default()
                        },
                        TextColor::default(),
                    ),
                    ForegroundRole(severity.action_role()),
                    TypeStyleToken(TypeScale::BodyMedium),
                ));
            });
    }
}
