//! CSS Grid layout primitive helper.

use crate::theme::spacing::SpacingScale;
use bevy::prelude::*;

/// Marker component for a CSS Grid container.
#[derive(Component, Debug, Clone, Default)]
pub struct Grid;

/// Fluent builder for constructing a CSS Grid [`NodeBundle`].
///
/// # Example
/// ```rust
/// use opts_gui::prelude::*;
/// use bevy::prelude::*;
///
/// fn setup_layout(mut commands: Commands) {
///     commands.spawn(
///         GridBuilder::new()
///             .with_columns(3)
///             .with_gap(SpacingScale::Standard)
///             .build()
///     );
/// }
/// ```
#[derive(Debug, Clone)]
pub struct GridBuilder {
    pub column_gap: SpacingScale,
    pub row_gap: SpacingScale,
    pub columns: Vec<RepeatedGridTrack>,
}

impl Default for GridBuilder {
    fn default() -> Self {
        Self {
            column_gap: SpacingScale::Small,
            row_gap: SpacingScale::Small,
            columns: Vec::new(),
        }
    }
}

impl GridBuilder {
    /// Creates a new grid builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets grid column and row gap spacing.
    pub fn with_gap(mut self, gap: SpacingScale) -> Self {
        self.column_gap = gap;
        self.row_gap = gap;
        self
    }

    /// Configures uniform repeat columns of equal width.
    pub fn with_columns(mut self, count: u16) -> Self {
        self.columns = vec![RepeatedGridTrack::repeat_many(
            count,
            vec![GridTrack::fr(1.0)],
        )];
        self
    }

    /// Builds a Bevy [`NodeBundle`] configured for CSS Grid layout.
    pub fn build(self) -> impl Bundle {
        (Node {
            display: Display::Grid,
            column_gap: self.column_gap.val(),
            row_gap: self.row_gap.val(),
            grid_template_columns: self.columns,
            ..default()
        })
    }
}
