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
use crate::app_state::AppState;

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

fn colours(severity: BannerSeverity) -> (ColorRole, ColorRole, &'static str) {
    match severity {
        BannerSeverity::Info => (
            ColorRole::SecondaryContainer,
            ColorRole::OnSecondaryContainer,
            icons::INFO,
        ),
        BannerSeverity::Warning => (
            ColorRole::TertiaryContainer,
            ColorRole::OnTertiaryContainer,
            icons::WARNING,
        ),
        BannerSeverity::Error => (
            ColorRole::ErrorContainer,
            ColorRole::OnErrorContainer,
            icons::ERROR,
        ),
    }
}

/// Shows a banner, replacing one with the same id. Needs the shell frame.
pub fn show_banner(commands: &mut Commands, spec: BannerSpec) {
    commands.queue(move |world: &mut World| {
        dismiss_in_world(world, spec.id);
        let area = world
            .query_filtered::<Entity, With<ShellBanners>>()
            .iter(world)
            .next();
        let Some(area) = area else {
            tracing::warn!(id = spec.id, "no banner area: the shell frame is missing");
            return;
        };
        let mut commands = world.commands();
        spawn_banner(&mut commands, area, &spec);
        world.flush();
    });
}

/// Removes the banner with `id`, if shown.
pub fn dismiss_banner(commands: &mut Commands, id: &'static str) {
    commands.queue(move |world: &mut World| dismiss_in_world(world, id));
}

fn dismiss_in_world(world: &mut World, id: &'static str) {
    let found: Vec<Entity> = world
        .query::<(Entity, &Banner)>()
        .iter(world)
        .filter(|(_, b)| b.id == id)
        .map(|(e, _)| e)
        .collect();
    for entity in found {
        if let Ok(e) = world.get_entity_mut(entity) {
            e.despawn();
        }
    }
}

fn spawn_banner(commands: &mut Commands, area: Entity, spec: &BannerSpec) {
    let (bg, fg, glyph) = colours(spec.severity);
    let banner = commands
        .spawn((
            Node {
                margin: UiRect::axes(Val::Px(16.0), Val::Px(8.0)),
                padding: UiRect::axes(Val::Px(16.0), Val::Px(12.0)),
                column_gap: Val::Px(16.0),
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundRole(bg),
            RadiusToken(RadiusLevel::Medium),
            Banner { id: spec.id },
            UiId::new(format!("banner.{}", spec.id)),
        ))
        .id();
    if let Some(owner) = spec.owner {
        commands.entity(banner).insert(BannerOwner(owner));
    }
    commands.entity(area).add_child(banner);
    commands.entity(banner).with_children(|b| {
        b.spawn((
            Text::new(glyph),
            TextFont {
                font: FontSource::Handle(opts_gui::fonts::ICON_FONT),
                font_size: FontSize::Px(24.0),
                ..default()
            },
            TextColor::default(),
            ForegroundRole(fg),
            opts_gui::ui_tree::A11yHidden,
        ));
        b.spawn(Node {
            flex_direction: FlexDirection::Column,
            flex_grow: 1.0,
            flex_shrink: 1.0,
            min_width: Val::Px(0.0),
            ..default()
        })
        .with_children(|col| {
            col.spawn((
                Text::new(spec.title_key),
                L10nKey(spec.title_key),
                spec.args.clone(),
                TextColor::default(),
                TypeStyleToken(TypeScale::TitleMedium),
                ForegroundRole(fg),
            ));
            col.spawn((
                Text::new(spec.body_key),
                L10nKey(spec.body_key),
                spec.args.clone(),
                TextColor::default(),
                TypeStyleToken(TypeScale::BodyMedium),
                ForegroundRole(fg),
            ));
        });
        if let Some((action, label_key)) = spec.action {
            let button = ButtonBuilder::filled(label_key)
                .with_key(label_key)
                .spawn_child(b);
            b.commands().entity(button).insert((
                BannerActionButton {
                    banner: spec.id,
                    action,
                },
                UiId::new(format!("banner.{}.{}", spec.id, action)),
            ));
        }
    });
}

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
