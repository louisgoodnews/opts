//! Banners: the durable place for a problem a screen cannot work around
//! (`docs/src/roadmap/phase-9/interaction-and-feel.md` section 5).
//!
//! A banner sits at the top of the content area, stays until it is resolved or dismissed, is
//! de-duplicated by its id and disappears when its owning screen is left. Errors are never *only*
//! a toast: the owner shows the durable banner as well.

use bevy::prelude::*;
use opts_gui::interaction::activation::Activated;
use opts_gui::l10n::{L10nArgs, L10nKey};
use opts_gui::prelude::*;

use super::frame::ShellBanners;
use super::icons;
use opts_game_state::app_state::AppState;

/// How serious the problem is (colour and icon; the text always says what happened).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BannerSeverity {
    /// Something to know.
    Info,
    /// The screen works with less than it should.
    Warning,
    /// The screen cannot do its job.
    Error,
}

/// What a banner says and offers.
#[derive(Debug, Clone)]
pub struct BannerSpec {
    /// De-duplication key (`home.no-omsi`).
    pub id: &'static str,
    /// Severity.
    pub severity: BannerSeverity,
    /// Message key of the title (what happened).
    pub title_key: &'static str,
    /// Message key of the body (why, and what to do).
    pub body_key: &'static str,
    /// Variables of both messages.
    pub args: L10nArgs,
    /// The one action: `(action id, label key)`.
    pub action: Option<(&'static str, &'static str)>,
    /// The screen the banner belongs to; leaving it removes the banner.
    pub owner: Option<AppState>,
}

/// A banner in the tree.
#[derive(Component, Debug, Clone, Copy)]
pub struct Banner {
    /// The banner id.
    pub id: &'static str,
}

/// The screen a banner belongs to.
#[derive(Component, Debug, Clone, Copy)]
pub struct BannerOwner(pub AppState);

/// The action button of a banner.
#[derive(Component, Debug, Clone, Copy)]
pub struct BannerActionButton {
    /// The banner id.
    pub banner: &'static str,
    /// The action id.
    pub action: &'static str,
}

/// A banner's action was activated.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub struct BannerAction {
    /// The banner id.
    pub banner: &'static str,
    /// The action id.
    pub action: &'static str,
}

mod spawn;
pub use spawn::{dismiss_banner, show_banner};

/// Turns a click on a banner's button into [`BannerAction`].
pub fn banner_action_system(
    mut activated: MessageReader<Activated>,
    buttons: Query<&BannerActionButton>,
    mut out: MessageWriter<BannerAction>,
) {
    for Activated(entity) in activated.read() {
        if let Ok(b) = buttons.get(*entity) {
            out.write(BannerAction {
                banner: b.banner,
                action: b.action,
            });
        }
    }
}

/// Removes the banners of a screen that was left.
pub fn banners_leave_screen(
    state: Res<State<AppState>>,
    banners: Query<(Entity, &BannerOwner)>,
    mut commands: Commands,
) {
    if !state.is_changed() {
        return;
    }
    for (entity, owner) in &banners {
        if owner.0 != *state.get() {
            commands.entity(entity).despawn();
        }
    }
}

/// Registers the message and systems.
pub fn build(app: &mut App) {
    app.add_message::<BannerAction>()
        .add_systems(Update, (banner_action_system, banners_leave_screen));
}
