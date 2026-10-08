//! Vertical stack layout primitive.

use crate::theme::spacing::SpacingScale;
use bevy::prelude::*;

/// Marker component for a vertical stack container.
#[derive(Component, Debug, Clone, Default)]
pub struct Stack;

/// Fluent builder for constructing a vertical stack [`NodeBundle`].
///
/// # Example
/// ```rust
/// use opts_gui::prelude::*;
/// use bevy::prelude::*;
///
/// fn setup_layout(mut commands: Commands) {
///     commands.spawn(
///         StackBuilder::new()
///             .with_spacing(SpacingScale::Standard)
///             .build()
///     );
/// }
/// ```
#[derive(Debug, Clone)]
pub struct StackBuilder {
    pub spacing: SpacingScale,
    pub align_items: AlignItems,
    pub justify_content: JustifyContent,
}

impl Default for StackBuilder {
    fn default() -> Self {
        Self {
            spacing: SpacingScale::Small,
            align_items: AlignItems::FlexStart,
            justify_content: JustifyContent::FlexStart,
        }
    }
}

impl StackBuilder {
    /// Creates a new stack builder.
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

    /// Builds a Bevy [`NodeBundle`] configured for vertical flex stacking.
    pub fn build(self) -> NodeBundle {
        NodeBundle {
            style: Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                row_gap: self.spacing.val(),
                align_items: self.align_items,
                justify_content: self.justify_content,
                ..default()
            },
            ..default()
        }
    }
}
