//! The shared pieces of a setting row: the usable state and its marks, the value text, the buttons and notes.

use super::*;

/// A plain value text (a number, a time, a path).
pub(super) fn value_text(
    c: &mut Commands,
    parent: Entity,
    key: &'static str,
    text: String,
) -> Entity {
    c.spawn((
        SettingValueText(key),
        Text::new(text),
        TextColor::default(),
        TypeStyleToken(TypeScale::BodyMedium),
        ForegroundRole(ColorRole::OnSurface),
        Node {
            min_width: Val::Px(48.0),
            justify_content: JustifyContent::Center,
            ..default()
        },
        ChildOf(parent),
    ))
    .id()
}

/// How a control is shown: usable, or why not.
#[derive(Clone)]
pub(super) enum Usable {
    Yes,
    NotYet(&'static opts_gui::not_yet::NotYetPlace),
    Needs(String),
}

pub(super) fn usable(ctx: &Ctx, d: &SettingDescriptor) -> Usable {
    if ctx.light_locked(d.key) {
        let msg = if d.key == crate::settings_registry::lighting::TARGET_FPS_KEY {
            "set-light-governor-disabled"
        } else {
            "set-light-auto-disabled"
        };
        return Usable::Needs(ctx.loc.t(msg));
    }
    match d.available {
        Available::Always => Usable::Yes,
        Available::NotYet(p) => Usable::NotYet(p),
        Available::Capability(name) if ctx.caps.has(name) => Usable::Yes,
        Available::Capability(name) => Usable::Needs(
            ctx.loc
                .t_args("settings-needs", &[("what".into(), name.into())]),
        ),
    }
}

pub(super) fn mark(c: &mut Commands, e: Entity, u: &Usable) {
    match u {
        Usable::Yes => {}
        Usable::NotYet(p) => {
            c.entity(e).insert(NotAvailableYet::new(p).without_label());
        }
        Usable::Needs(reason) => {
            c.entity(e)
                .insert((Disabled, DisabledReason(reason.clone().into())));
        }
    }
}

/// A button of a row (`role` is the last part of its id).
pub(super) fn button(
    c: &mut Commands,
    parent: Entity,
    d: &SettingDescriptor,
    role: String,
    (label_key, text): (Option<&'static str>, &str),
    (selected, op): (bool, SettingOp),
    u: &Usable,
) -> Entity {
    let mut b = ButtonBuilder::outlined("");
    b.label = label_key.unwrap_or(text).to_string();
    if let Some(k) = label_key {
        b = b.with_key(k);
    }
    let e = b.spawn(c);
    c.entity(e).insert((
        op,
        UiId::new(format!("settings/{}/{}/{role}", d.category.id(), d.key)),
        ChildOf(parent),
    ));
    if selected {
        c.entity(e).insert((
            BackgroundRole(ROLES_SELECTED.0),
            BorderRole(ROLES_SELECTED.2),
        ));
        c.queue(move |world: &mut World| {
            let kids: Vec<Entity> = world
                .get::<Children>(e)
                .map(|k| k.iter().collect())
                .unwrap_or_default();
            for k in kids {
                if let Some(mut fg) = world.get_mut::<ForegroundRole>(k) {
                    fg.0 = ROLES_SELECTED.1;
                }
            }
        });
    }
    mark(c, e, u);
    e
}

/// A small localized note with variables below a language.
pub(super) fn note(
    c: &mut Commands,
    parent: Entity,
    key: &'static str,
    args: Args,
    role: ColorRole,
) {
    c.spawn((
        Text::new(key),
        L10nKey(key),
        L10nArgs(args),
        TextColor::default(),
        TypeStyleToken(TypeScale::Label),
        ForegroundRole(role),
        Node {
            max_width: Val::Px(280.0),
            ..default()
        },
        ChildOf(parent),
    ));
}
