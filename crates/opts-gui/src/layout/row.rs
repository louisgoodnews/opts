//! Horizontal row layout primitive.

use crate::theme::spacing::SpacingScale;
use bevy::prelude::*;

/// Marker component for a horizontal row container.
#[derive(Component, Debug, Clone, Default)]
pub struct Row;

/// Fluent builder for constructing a horizontal row [`NodeBundle`].
///
/// # Example
/// ```rust
/// use opts_gui::prelude::*;
/// use bevy::prelude::*;
///
/// fn setup_layout(mut commands: Commands) {
///     commands.spawn(
///         RowBuilder::new()
///             .with_spacing(SpacingScale::Medium)
///             .build()
///     );
/// }
/// ```
#[derive(Debug, Clone)]
pub struct RowBuilder {
    pub spacing: SpacingScale,
    pub align_items: AlignItems,
    pub justify_content: JustifyContent,
}

impl Default for RowBuilder {
    fn default() -> Self {
        Self {
            spacing: SpacingScale::Small,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::FlexStart,
        }
    }
}

impl RowBuilder {
    /// Creates a new row builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets inner item gap spacing.
    pub fn with_spacing(mut self, spacing: SpacingScale) -> Self {
        self.spacing = spacing;
        self
    }

    /// Sets cross-axis alignment.
    pub fn with_align_items(mut self, align: AlignItems) -> Self {
        self.align_items = align;
        self
    }

    /// Sets main-axis justification.
    pub fn with_justify_content(mut self, justify: JustifyContent) -> Self {
        self.justify_content = justify;
        self
    }

    /// Builds a Bevy [`NodeBundle`] configured for horizontal row flex.
    pub fn build(self) -> NodeBundle {
        NodeBundle {
            style: Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                column_gap: self.spacing.val(),
                align_items: self.align_items,
                justify_content: self.justify_content,
                ..default()
            },
            ..default()
        }
    }
}
