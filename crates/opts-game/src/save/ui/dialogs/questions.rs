//! The questions the save overlay asks before a replace, a delete or a load over a running drive.

use super::*;

pub(super) fn args(name: &str) -> L10nArgs {
    L10nArgs(vec![("name".to_owned(), Arg::from(name.to_owned()))])
}

pub(super) fn ask(
    world: &mut World,
    tag: &'static str,
    title: &'static str,
    body: &'static str,
    confirm: &'static str,
    name: &str,
    destructive: bool,
) {
    let spec = DialogSpec {
        tag,
        title_key: title,
        body_key: body,
        body_args: args(name),
        buttons: vec![
            DialogButtonSpec {
                choice: CANCEL,
                label_key: keys::CANCEL,
                style: DialogButtonStyle::Quiet,
            },
            DialogButtonSpec {
                choice: YES,
                label_key: confirm,
                style: if destructive {
                    DialogButtonStyle::Destructive
                } else {
                    DialogButtonStyle::Primary
                },
            },
        ],
        keep_open: Vec::new(),
        error: None,
    };
    let mut commands = world.commands();
    spawn_confirm_dialog(&mut commands, spec);
    world.flush();
}

/// Asks before a save is replaced.
pub fn ask_overwrite(world: &mut World, id: Option<String>, name: String) {
    world.resource_mut::<SavesUi>().pending = Some(Pending::Overwrite(id, name.clone()));
    ask(
        world,
        OVERWRITE_DIALOG,
        keys::SAVES_OVERWRITE_TITLE,
        keys::SAVES_OVERWRITE_BODY,
        keys::SAVES_OVERWRITE,
        &name,
        true,
    );
}

pub(super) fn name_of(world: &World, id: &str) -> String {
    store(world)
        .ok()
        .and_then(|s| s.read(id).ok())
        .map_or_else(|| id.to_owned(), |l| l.file.meta.name)
}

/// A row's button.
pub fn row_action(world: &mut World, id: &str, action: RowAction) {
    match action {
        RowAction::Load => {
            if *world.resource::<State<AppState>>().get() == AppState::InGame {
                let name = name_of(world, id);
                world.resource_mut::<SavesUi>().pending = Some(Pending::Load(id.to_owned()));
                ask(
                    world,
                    LOAD_DIALOG,
                    keys::SAVES_LOAD_TITLE,
                    keys::SAVES_LOAD_BODY,
                    keys::SAVES_LOAD,
                    &name,
                    true,
                );
            } else {
                load(world, id);
            }
        }
        RowAction::Overwrite => {
            let name = name_of(world, id);
            ask_overwrite(world, Some(id.to_owned()), name);
        }
        RowAction::Rename => {
            let name = name_of(world, id);
            set_field_text(world, &name);
            let mut ui = world.resource_mut::<SavesUi>();
            ui.renaming = Some(id.to_owned());
            ui.name_error = None;
        }
        RowAction::Delete => {
            let name = name_of(world, id);
            world.resource_mut::<SavesUi>().pending = Some(Pending::Delete(id.to_owned()));
            ask(
                world,
                DELETE_DIALOG,
                keys::SAVES_DELETE_TITLE,
                keys::SAVES_DELETE_BODY,
                keys::SAVES_DELETE,
                &name,
                true,
            );
        }
    }
}

/// Starts restoring; a refusal opens the error dialog.
pub fn load(world: &mut World, id: &str) {
    if let Err(e) = begin_restore(world, id) {
        tracing::warn!(%id, "load refused: {e}");
        let name = name_of(world, id);
        open_error(world, &name, &e);
    }
}
