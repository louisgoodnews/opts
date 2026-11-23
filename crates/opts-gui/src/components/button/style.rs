//! Colour roles, elevation and layout node of a button, derived from its variant.

use bevy::prelude::*;

use super::disabled::text_label_role;
use super::{ButtonBuilder, ButtonVariant};
use crate::theme::{color::ColorRole, elevation::ElevationLevel};

impl ButtonBuilder {
    pub(super) fn roles(&self) -> (ColorRole, ColorRole, ElevationLevel) {
        match self.variant {
            ButtonVariant::Filled => (
                ColorRole::Primary,
                ColorRole::OnPrimary,
                ElevationLevel::Level0,
            ),
            ButtonVariant::Elevated => (
                ColorRole::SurfaceContainerLow,
                ColorRole::Primary,
                ElevationLevel::Level1,
            ),
            ButtonVariant::FilledTonal => (
                ColorRole::SecondaryContainer,
                ColorRole::OnSecondaryContainer,
                ElevationLevel::Level0,
            ),
            ButtonVariant::Outlined => (
                ColorRole::Surface,
                ColorRole::Primary,
                ElevationLevel::Level0,
            ),
            ButtonVariant::Text => (
                ColorRole::Surface,
                text_label_role(self.is_disabled),
                ElevationLevel::Level0,
            ),
            ButtonVariant::Icon => (
                ColorRole::SurfaceContainerHighest,
                ColorRole::OnSurfaceVariant,
                ElevationLevel::Level0,
            ),
        }
    }

    pub(super) fn style(&self) -> Node {
        let is_icon_only = self.variant == ButtonVariant::Icon;
        let padding = if is_icon_only {
            UiRect::all(Val::Px(8.0))
        } else {
            UiRect::axes(Val::Px(24.0), Val::Px(10.0))
        };
        Node {
            min_height: Val::Px(40.0),
            min_width: if is_icon_only {
                Val::Px(40.0)
            } else {
                Val::Auto
            },
            padding,
            column_gap: Val::Px(8.0),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            border: if self.variant == ButtonVariant::Outlined {
                UiRect::all(Val::Px(1.0))
            } else {
                UiRect::DEFAULT
            },
            ..default()
        }
    }
}
