//! The error dialog of a load that cannot start: what happened, why, and what to do.

use super::*;

/// The error template for a load that cannot start: what happened (title), why (body), what you
/// can do (*Choose another*, *Back*); the technical text is in the body's last sentence.
pub(super) fn open_error(world: &mut World, name: &str, error: &RestoreError) {
    let mut body_args = vec![("name".to_owned(), Arg::from(name.to_owned()))];
    let body = match error {
        RestoreError::Save(SaveError::NewerFormat { found, .. }) => {
            body_args.push(("found".to_owned(), Arg::from(found.to_string())));
            keys::LOAD_ERR_NEWER
        }
        RestoreError::Save(SaveError::Corrupt(_) | SaveError::NotASave) => {
            body_args.push(("error".to_owned(), Arg::from(error.to_string())));
            keys::LOAD_ERR_CORRUPT
        }
        RestoreError::Blocked(issues) => {
            let blocking = |kind| {
                issues
                    .iter()
                    .filter(|i| i.blocking && i.kind == kind)
                    .map(|i| i.subject.clone())
                    .collect::<Vec<_>>()
                    .join(", ")
            };
            let map = blocking(IssueKind::MissingMap);
            if map.is_empty() {
                body_args.push((
                    "vehicles".to_owned(),
                    Arg::from(blocking(IssueKind::MissingVehicle)),
                ));
                keys::LOAD_ERR_MISSING_VEHICLES
            } else {
                body_args.push(("map".to_owned(), Arg::from(map)));
                keys::LOAD_ERR_MISSING_MAP
            }
        }
        RestoreError::Save(_) => {
            body_args.push(("error".to_owned(), Arg::from(error.to_string())));
            keys::LOAD_ERR_OTHER
        }
    };
    let spec = DialogSpec {
        tag: ERROR_DIALOG,
        title_key: keys::LOAD_ERROR_TITLE,
        body_key: body,
        body_args: L10nArgs(body_args),
        buttons: vec![
            DialogButtonSpec {
                choice: ANOTHER,
                label_key: keys::LOAD_CHOOSE_ANOTHER,
                style: DialogButtonStyle::Primary,
            },
            DialogButtonSpec {
                choice: BACK,
                label_key: keys::LOAD_BACK,
                style: DialogButtonStyle::Quiet,
            },
        ],
        keep_open: Vec::new(),
        error: None,
    };
    let mut commands = world.commands();
    spawn_confirm_dialog(&mut commands, spec);
    world.flush();
}
