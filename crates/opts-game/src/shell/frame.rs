//! The shell frame: navigation rail, banner area and content area.
//!
//! Spawned once at start-up from the [`ScreenRegistry`]. The rail is 72 px wide with icons only in
//! the *Compact* layout class and 240 px with labels otherwise. While a drive runs the frame is
//! hidden and [`ShellContent`] is `None`, so full-window overlays build as roots.

use bevy::prelude::*;
use opts_gui::interaction::state::{Disabled, DisabledReason, Selected};
use opts_gui::interaction::tooltip::Tooltip;
use opts_gui::l10n::{L10nArgs, L10nKey, L10nTooltip};
use opts_gui::not_yet::{Availability, NotAvailableYet};
use opts_gui::prelude::*;
use opts_gui::theme::LayoutClass;

use super::registry::{ScreenDescriptor, ScreenGroup, ScreenRegistry};
use super::{icons, keys, nav, ShellContent};
use crate::app_state::AppState;

/// Rail width with icons only.
pub const RAIL_COMPACT_PX: f32 = 72.0;
/// Rail width with labels.
pub const RAIL_WIDE_PX: f32 = 240.0;

/// Root of the shell frame.
#[derive(Component, Debug, Clone, Copy)]
pub struct ShellRoot;
/// The navigation rail.
#[derive(Component, Debug, Clone, Copy)]
pub struct ShellRail;
/// Where banners go (top of the content column).
#[derive(Component, Debug, Clone, Copy)]
pub struct ShellBanners;

/// A rail entry for a registered destination.
#[derive(Component, Debug, Clone, Copy)]
pub struct RailItem {
    /// The destination id.
    pub id: &'static str,
}

/// The actions of the rail footer.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub enum RailAction {
    /// Switch between the dark and the light theme (F2).
    ThemeToggle,
    /// Quit OPTS (with confirmation).
    Quit,
}

/// The icon of a rail entry.
#[derive(Component, Debug, Clone, Copy)]
pub struct RailIcon;
/// Text that is only shown when the rail is wide (labels, brand, version).
#[derive(Component, Debug, Clone, Copy)]
pub struct RailWideOnly;
/// The red *Feature not available yet!* line of a rail entry.
#[derive(Component, Debug, Clone, Copy)]
pub struct RailNotYet;
/// What a rail entry offered when it was last updated.
#[derive(Component, Debug, Clone, PartialEq, Eq)]
pub struct RailAvailability(pub Availability);

/// Spawns the frame and publishes [`ShellContent`].
pub fn spawn_shell_frame(mut commands: Commands, registry: Res<ScreenRegistry>) {
    let root = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                top: Val::Px(0.0),
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Row,
                ..default()
            },
            BackgroundRole(ColorRole::Surface),
            GlobalZIndex(-10),
            ShellRoot,
            Name::new("ShellRoot"),
        ))
        .id();
    let rail = spawn_rail(&mut commands, &registry);
    let main = commands
        .spawn(Node {
            flex_grow: 1.0,
            min_width: Val::Px(0.0),
            // lets the content area shrink below its page and scroll
            min_height: Val::Px(0.0),
            flex_direction: FlexDirection::Column,
            ..default()
        })
        .id();
    let banners = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(8.0),
                // Banners never take more than half the window: at a large UI scale the page
                // below keeps room and the banners scroll.
                max_height: Val::Percent(50.0),
                flex_shrink: 0.0,
                overflow: Overflow::scroll_y(),
                ..default()
            },
            ShellBanners,
        ))
        .id();
    let content = commands
        .spawn((
            Node {
                flex_grow: 1.0,
                min_height: Val::Px(0.0),
                flex_direction: FlexDirection::Column,
                // A page taller than the window (large UI scale, small window) scrolls.
                overflow: Overflow::scroll_y(),
                ..default()
            },
            FocusRegion(Landmark::Content),
            Name::new("ShellContent"),
        ))
        .id();
    opts_gui::patterns::scroll::wheel_scroll(&mut commands, content);
    opts_gui::patterns::scroll::wheel_scroll(&mut commands, rail);
    opts_gui::patterns::scroll::wheel_scroll(&mut commands, banners);
    commands.entity(main).add_children(&[banners, content]);
    commands.entity(root).add_children(&[rail, main]);
    commands.insert_resource(ShellContent(Some(content)));
}

mod entry;
mod rail_spawn;
use rail_spawn::spawn_rail;

mod rail;
pub use rail::{
    hide_shell, rail_click_system, refresh_rail_availability, show_shell, sync_rail_layout,
    sync_rail_not_yet_labels, sync_rail_selection,
};
