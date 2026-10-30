//! The parts of the home status that follow the catalogue: the chip, the buttons and the banners.

use super::*;

/// The status chip: refreshing (with the stage), ready with the counts, or nothing found.
pub(super) fn update_chips(
    commands: &mut Commands,
    inputs: &HomeInputs,
    chips: &mut Query<(Entity, &mut L10nKey), With<StatusChip>>,
) {
    for (entity, mut key) in chips {
        let (new_key, args) = match (inputs.refreshing, inputs.catalogue) {
            (true, _) => (
                inputs
                    .stage
                    .and_then(stage_key)
                    .unwrap_or(keys::HOME_STATUS_REFRESHING),
                L10nArgs::default(),
            ),
            (false, CatalogueStatus::Ready { maps, vehicles }) => (
                keys::HOME_STATUS_READY,
                L10nArgs(vec![
                    ("maps".to_owned(), maps.into()),
                    ("vehicles".to_owned(), vehicles.into()),
                ]),
            ),
            _ => (keys::HOME_STATUS_NONE, L10nArgs::default()),
        };
        key.0 = new_key;
        commands.entity(entity).try_insert(args);
    }
}

/// Buttons that need a drive to be possible are disabled with the reason while it is blocked.
pub(super) fn update_buttons(
    commands: &mut Commands,
    loc: &Localization,
    inputs: &HomeInputs,
    buttons: &Query<(Entity, &HomeButton)>,
) {
    let block = inputs.drive_block();
    for (entity, button) in buttons {
        if matches!(button, HomeButton::NewDrive | HomeButton::LoadGame) {
            continue;
        }
        let mut e = commands.entity(entity);
        match block_reason_key(block) {
            None => {
                e.try_remove::<(Disabled, DisabledReason)>();
            }
            Some(key) => {
                e.try_insert((Disabled, DisabledReason(loc.t(key).into())));
            }
        }
    }
}

/// Shows the banner the inputs call for (and removes the other one) when it changed.
pub(super) fn update_banners(
    commands: &mut Commands,
    inputs: &HomeInputs,
    previous_banner: Option<HomeBanner>,
) {
    if inputs.banner() != previous_banner || previous_banner.is_none() {
        dismiss_banner(commands, BANNER_NO_OMSI);
        dismiss_banner(commands, BANNER_EMPTY);
        match inputs.banner() {
            Some(HomeBanner::NoOmsi) => show_banner(
                commands,
                BannerSpec {
                    id: BANNER_NO_OMSI,
                    severity: BannerSeverity::Error,
                    title_key: keys::HOME_BANNER_NO_OMSI_TITLE,
                    body_key: keys::HOME_BANNER_NO_OMSI_BODY,
                    args: L10nArgs::default(),
                    action: Some((ACTION_CHOOSE, keys::HOME_BANNER_CHOOSE_FOLDER)),
                    owner: Some(AppState::MainMenu),
                },
            ),
            Some(HomeBanner::EmptyCatalogue) => show_banner(
                commands,
                BannerSpec {
                    id: BANNER_EMPTY,
                    severity: BannerSeverity::Warning,
                    title_key: keys::HOME_BANNER_EMPTY_TITLE,
                    body_key: keys::HOME_BANNER_EMPTY_BODY,
                    args: L10nArgs::default(),
                    action: Some((ACTION_CHOOSE, keys::HOME_BANNER_CHOOSE_FOLDER)),
                    owner: Some(AppState::MainMenu),
                },
            ),
            None => {}
        }
    }
}
