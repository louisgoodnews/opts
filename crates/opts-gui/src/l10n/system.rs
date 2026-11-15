//! The systems that keep localised texts, tooltips and reasons current, and the plugin wiring.

use bevy::prelude::*;

use super::fluent::Args;
use super::{keys, L10nArgs, L10nKey, L10nTooltip, Localization};

/// The key of an element or of one of its direct children (the label of a control). The
/// not-yet marker's own key does not name a control.
pub fn key_of(world: &World, entity: Entity) -> Option<String> {
    let e = world.get_entity(entity).ok()?;
    let own = |e: &EntityRef| {
        e.get::<L10nKey>()
            .filter(|k| k.0 != keys::NOT_AVAILABLE)
            .map(|k| k.0.to_string())
    };
    own(&e).or_else(|| {
        e.get::<Children>()
            .into_iter()
            .flat_map(|c| c.iter())
            .find_map(|child| world.get_entity(child).ok().and_then(|c| own(&c)))
    })
}

/// Query item: a text with a key.
type LocalizedText<'a> = (Ref<'a, L10nKey>, Option<Ref<'a, L10nArgs>>, &'a mut Text);
/// Query item: a tooltip with a key.
type LocalizedTip<'a> = (
    Ref<'a, L10nTooltip>,
    &'a mut crate::interaction::tooltip::Tooltip,
);
/// Query item: the reason of a not-yet control.
type LocalizedReason<'a> = (
    Ref<'a, crate::not_yet::NotAvailableYet>,
    &'a mut crate::interaction::state::DisabledReason,
);

/// Keeps the text of every [`L10nKey`] element, every [`L10nTooltip`] and every not-yet reason
/// current.
pub fn localize_system(
    loc: Res<Localization>,
    mut texts: Query<LocalizedText>,
    mut tips: Query<LocalizedTip>,
    mut reasons: Query<LocalizedReason>,
) {
    let all = loc.is_changed();
    for (key, args, mut text) in &mut texts {
        if all || key.is_changed() || args.as_ref().is_some_and(|a| a.is_changed()) {
            let a = args.as_ref().map_or(&[][..], |a| a.0.as_slice());
            let new = loc.t_args(key.0, a);
            if text.0 != new {
                text.0 = new;
            }
        }
    }
    for (key, mut tip) in &mut tips {
        if all || key.is_added() {
            tip.0 = loc.t(key.0).into();
        }
    }
    for (marker, mut reason) in &mut reasons {
        if all || marker.is_added() {
            let args: Args = vec![("milestone".into(), marker.place.milestone.into())]; // l10n-ok: argument name
            reason.0 = loc.t_args(keys::NOT_AVAILABLE_REASON, &args).into();
        }
    }
}

/// A `Text` bundle that shows the message `key` (resolved in the next frame).
pub fn text(key: &'static str) -> impl Bundle {
    (Text::new(key), L10nKey(key))
}

/// Registers the resource and the update system.
pub fn build(app: &mut App) {
    if !app.world().contains_resource::<Localization>() {
        app.insert_resource(Localization::detect());
    }
    app.add_systems(
        PostUpdate,
        localize_system.before(bevy::ui::UiSystems::Content),
    );
}
