//! Building the pause overlay's widgets and closing it.

use super::*;

fn item(
    column: &mut ChildSpawnerCommands,
    which: PauseItem,
    key: &'static str,
    id: &str,
) -> Entity {
    let button = ButtonBuilder::text(key).with_key(key).spawn_child(column);
    column.commands().entity(button).insert((
        which,
        UiId::new(format!("pause.{id}")),
        Node {
            min_height: Val::Px(44.0),
            padding: UiRect::axes(Val::Px(16.0), Val::Px(10.0)),
            justify_content: JustifyContent::FlexStart,
            align_items: AlignItems::Center,
            ..default()
        },
    ));
    button
}

pub(crate) fn spawn_overlay(commands: &mut Commands) -> Entity {
    let (root, card) = ModalBuilder::new()
        .with_size_percent(30.0, 50.0)
        .with_width_range(320.0, 420.0)
        .spawn(commands);
    commands.queue(move |world: &mut World| {
        if let Some(mut node) = world.entity_mut(card).get_mut::<Node>() {
            node.height = Val::Auto;
        }
    });
    commands.entity(card).with_children(|c| {
        c.spawn((
            Text::new(keys::PAUSE_TITLE),
            L10nKey(keys::PAUSE_TITLE),
            TextColor::default(),
            TypeStyleToken(TypeScale::TitleLarge),
            ForegroundRole(ColorRole::OnSurface),
        ));
        let resume = item(c, PauseItem::Resume, keys::PAUSE_RESUME, "resume");
        c.commands().entity(resume).insert(DefaultFocus);
        item(c, PauseItem::Vehicle, keys::PAUSE_VEHICLE, "vehicle");
        item(
            c,
            PauseItem::Environment,
            keys::PAUSE_ENVIRONMENT,
            "environment",
        );
        item(c, PauseItem::Save, keys::PAUSE_SAVE, "save");
        item(c, PauseItem::Load, keys::PAUSE_LOAD, "load");
        item(c, PauseItem::Settings, keys::PAUSE_SETTINGS, "settings");
        item(c, PauseItem::Controls, keys::PAUSE_CONTROLS, "controls");
        item(c, PauseItem::Report, keys::PAUSE_REPORT, "report");
        c.spawn(Node {
            height: Val::Px(1.0),
            margin: UiRect::vertical(Val::Px(4.0)),
            ..default()
        })
        .insert(BackgroundRole(ColorRole::OutlineVariant));
        let leave = item(c, PauseItem::Leave, keys::PAUSE_LEAVE, "leave");
        c.commands().entity(leave).insert((
            BackgroundRole(ColorRole::ErrorContainer),
            StateLayerRole(ColorRole::OnErrorContainer),
        ));
    });
    commands.entity(root).insert(Name::new("PauseOverlay"));
    root
}

pub(super) fn open_pause(commands: &mut Commands, pause: &mut Pause, time: &mut Time<Virtual>) {
    if pause.open {
        return;
    }
    pause.resume_time = !time.is_paused();
    time.pause();
    pause.open = true;
    pause.root = Some(spawn_overlay(commands));
}

pub(crate) fn close_pause(
    commands: &mut Commands,
    pause: &mut Pause,
    time: &mut Time<Virtual>,
    resume: bool,
) {
    if let Some(root) = pause.root.take() {
        commands.entity(root).try_despawn();
    }
    pause.open = false;
    if resume && pause.resume_time {
        time.unpause();
    }
}
