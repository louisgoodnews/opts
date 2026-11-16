//! The despawn dialog: asking before a vehicle is despawned, and the choices it offers.

use super::*;

/// Asks the user whether to despawn a session vehicle (the shell's confirmation dialog). The
/// answer is handled by [`despawn_choices`].
pub fn ask_despawn(world: &mut World, id: VehicleId) {
    let Some(rec) = world.resource::<SessionVehicles>().get(id).cloned() else {
        return;
    };
    world.resource_mut::<PendingDespawn>().0 = Some(id);
    let spec = DialogSpec {
        tag: DESPAWN_DIALOG,
        title_key: keys::DESPAWN_TITLE,
        body_key: keys::DESPAWN_BODY,
        body_args: L10nArgs(vec![("name".into(), Arg::from(rec.name.as_str()))]),
        buttons: vec![
            DialogButtonSpec {
                choice: CANCEL,
                label_key: keys::CANCEL,
                style: DialogButtonStyle::Quiet,
            },
            DialogButtonSpec {
                choice: DESPAWN,
                label_key: keys::DESPAWN_CONFIRM,
                style: DialogButtonStyle::Destructive,
            },
        ],
        keep_open: Vec::new(),
        error: None,
    };
    let mut commands = world.commands();
    spawn_confirm_dialog(&mut commands, spec);
    world.flush();
}

/// The answer of the despawn confirmation.
pub fn despawn_choices(
    mut commands: Commands,
    mut choices: MessageReader<DialogChoice>,
    mut pending: ResMut<PendingDespawn>,
) {
    for choice in choices.read().filter(|c| c.tag == DESPAWN_DIALOG) {
        let id = pending.0.take();
        if choice.choice != DESPAWN {
            continue;
        }
        if let Some(id) = id {
            defer(&mut commands, move |world: &mut World| {
                if let Err(e) = ops::despawn(world, id, DespawnReason::User) {
                    tracing::warn!("despawn refused: {e}");
                }
            });
        }
    }
}
