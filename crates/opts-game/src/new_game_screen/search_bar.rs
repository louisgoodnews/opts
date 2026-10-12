//! Search bar of the new game screen: the library `TextField` plus the catalogue marker.

use bevy::prelude::*;
use opts_gui::components::text_field::TextFieldBuilder;

use crate::new_game_screen::types::{ScrollKind, SearchInputBox};

/// Spawns a search bar (library text field with clear button) for one catalogue list.
pub fn spawn_search_bar(
    parent: &mut ChildSpawnerCommands,
    kind: ScrollKind,
    placeholder: &str,
    current_query: &str,
) {
    let field = TextFieldBuilder::outlined("Search")
        .compact()
        .full_width()
        .with_margin(UiRect::bottom(Val::Px(6.0)))
        .with_placeholder(placeholder)
        .with_text(current_query)
        .spawn_child(parent);
    parent
        .commands_mut()
        .entity(field)
        .insert(SearchInputBox { kind });
}
