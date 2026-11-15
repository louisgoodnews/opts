//! Filter chips of the master pane and the clear button that follows the filter state.

use bevy::prelude::*;

use super::chrome::set_if;
use crate::components::button::ButtonBuilder;
use crate::icons::icon_text;
use crate::interaction::state::Disabled;
use crate::patterns::glyphs;
use crate::patterns::master_detail::{MasterChipButton, MasterClearButton, MasterDetail, MasterDetailView};
use crate::theme::color::ColorRole;
use crate::tokens::components::ForegroundRole;
use crate::ui_id::{slug, UiId};

pub(super) fn render_chips(
    commands: &mut Commands,
    root: Entity,
    view: &mut MasterDetailView,
    model: &MasterDetail,
) {
    let sig = (model.chips.clone(), model.active_chips.clone());
    if view.rendered_chips.as_ref() == Some(&sig) {
        return;
    }
    for e in std::mem::take(&mut view.chip_entities) {
        commands.entity(e).despawn();
    }
    let prefix = view.config.prefix;
    let mut spawned = Vec::new();
    for chip in &model.chips {
        let active = model.active_chips.contains(&chip.tag);
        let builder = if active {
            ButtonBuilder::tonal(chip.label.clone())
        } else {
            ButtonBuilder::outlined(chip.label.clone())
        };
        let button = builder.spawn(commands);
        commands.entity(button).insert((
            MasterChipButton {
                root,
                tag: chip.tag.clone(),
            },
            UiId::new(format!("{prefix}.chip.{}", slug(&chip.tag))),
        ));
        if active {
            let mark = commands
                .spawn((
                    icon_text(glyphs::CHECK, 16.0, TextColor::default().0),
                    ForegroundRole(ColorRole::OnSecondaryContainer),
                    crate::ui_tree::A11yHidden,
                ))
                .id();
            commands.entity(button).add_child(mark);
        }
        spawned.push(button);
    }
    commands.entity(view.parts.chips).add_children(&spawned);
    view.chip_entities = spawned;
    view.rendered_chips = Some(sig);
}

/// Disables nothing today; the clear button follows the filter state.
pub(super) fn sync_clear_button(
    roots: Query<(&MasterDetail, &MasterDetailView)>,
    clears: Query<(Entity, &MasterClearButton)>,
    mut nodes: Query<&mut Node>,
    mut commands: Commands,
) {
    for (e, clear) in &clears {
        let Ok((model, _)) = roots.get(clear.0) else {
            continue;
        };
        let filtering = !model.query.trim().is_empty() || !model.active_chips.is_empty();
        if let Ok(mut n) = nodes.get_mut(e) {
            set_if(
                &mut n.display,
                if filtering && !model.entries.is_empty() {
                    Display::Flex
                } else {
                    Display::None
                },
            );
        }
        commands.entity(e).try_remove::<Disabled>();
    }
}
