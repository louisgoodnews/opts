//! The panel's node queries, bundled for [`update_panel`](super::update_panel).

use bevy::ecs::system::SystemParam;

use super::*;

/// The nodes of the open panel that follow the editor's state.
#[derive(SystemParam)]
pub struct PanelNodes<'w, 's> {
    /// The numbers.
    pub values: Query<'w, 's, (&'static ViewpointValue, &'static mut Text), Without<ViewpointSubtitle>>,
    /// The subtitle.
    pub subtitle:
        Query<'w, 's, &'static mut Text, (With<ViewpointSubtitle>, Without<ViewpointValue>)>,
    /// The bars.
    pub fills: Query<'w, 's, (&'static ViewpointFill, &'static mut Node)>,
    /// The marks of the selected row.
    pub markers: Query<
        'w,
        's,
        (&'static ViewpointMarker, &'static mut Visibility),
        Without<ViewpointUnsaved>,
    >,
    /// The row names.
    pub names: Query<'w, 's, (&'static ViewpointName, &'static mut InstrumentInk)>,
    /// The *unsaved* mark.
    pub unsaved: Query<
        'w,
        's,
        &'static mut Visibility,
        (With<ViewpointUnsaved>, Without<ViewpointMarker>),
    >,
    /// The painted tracks.
    pub paints: Query<'w, 's, (&'static ViewpointPaint, &'static mut BackgroundColor)>,
    /// The scope text.
    pub scope_text: Query<'w, 's, &'static mut L10nKey, With<ViewpointScopeText>>,
    /// The scope row.
    pub scope_row:
        Query<'w, 's, &'static mut Node, (With<ViewpointScopeRow>, Without<ViewpointFill>)>,
}
