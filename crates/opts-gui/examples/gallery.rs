//! Component gallery: every `opts-gui` component in every state (rest, hover, focus, pressed,
//! disabled with a reason, "Feature not available yet!", error).
//!
//! ```text
//! cargo run -p opts-gui --example gallery
//! OPTS_LOCALE=en-XA cargo run -p opts-gui --example gallery     # pseudo-locale (+40 % text)
//! OPTS_GALLERY_SHOT=/path/shot.png cargo run -p opts-gui --example gallery   # save a PNG, exit
//! ```
//!
//! Keys: Tab / arrows move focus, Enter or Space activate, Esc goes back, F2 toggles light/dark.
//! The pinned hover/focus/pressed rows are display aids (`ForcedState`); everything else is live.

use bevy::prelude::*;
use bevy::render::view::screenshot::{save_to_disk, Screenshot};
use opts_gui::prelude::*;
use opts_gui::showcase::spawn_showcase;

#[derive(Component)]
struct StatusLabel;

#[derive(Resource)]
struct ShotPlan {
    path: String,
    frames: u32,
}

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title: "OPTS component gallery".into(),
            resolution: bevy::window::WindowResolution::new(1700, 1000),
            ..default()
        }),
        ..default()
    }))
    .add_plugins(OptsGuiPlugin)
    .add_systems(Startup, setup)
    .add_systems(Update, (theme_hotkey, status_line, take_screenshot));
    if let Ok(path) = std::env::var("OPTS_GALLERY_SHOT") {
        app.insert_resource(ShotPlan { path, frames: 0 });
    }
    app.run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);
    let root = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(20.0)),
                row_gap: Val::Px(12.0),
                overflow: Overflow::scroll_y(),
                ..default()
            },
            BackgroundRole(ColorRole::Surface),
        ))
        .id();
    let title = LabelBuilder::new("OPTS component gallery")
        .with_style(TypeScale::Headline)
        .spawn(&mut commands);
    let status = commands
        .spawn((
            Text::new("Use Tab, arrows and Enter; F2 switches the theme."),
            TypeStyleToken(TypeScale::BodyMedium),
            ForegroundRole(ColorRole::OnSurfaceVariant),
            StatusLabel,
        ))
        .id();
    commands.entity(root).add_children(&[title, status]);
    spawn_showcase(&mut commands, root);
}

fn theme_hotkey(
    keyboard: Res<ButtonInput<KeyCode>>,
    theme: Res<MaterialTheme>,
    mut events: MessageWriter<ThemeChanged>,
) {
    if keyboard.just_pressed(KeyCode::F2) {
        events.write(ThemeChanged {
            is_dark: !theme.scheme.is_dark(),
        });
    }
}

#[allow(clippy::too_many_arguments)]
fn status_line(
    mut clicks: MessageReader<ButtonClicked>,
    mut checks: MessageReader<CheckboxToggled>,
    mut switches: MessageReader<SwitchToggled>,
    mut radios: MessageReader<RadioSelected>,
    mut combos: MessageReader<ComboboxSelected>,
    mut trees: MessageReader<TreeNodeSelected>,
    mut texts: MessageReader<TextChanged>,
    mut label: Query<&mut Text, With<StatusLabel>>,
) {
    let Ok(mut text) = label.single_mut() else {
        return;
    };
    if let Some(e) = clicks.read().last() {
        text.0 = format!("Button {:?} activated", e.0);
    }
    if let Some(e) = checks.read().last() {
        text.0 = format!("Check box {:?} is now {:?}", e.entity, e.state);
    }
    if let Some(e) = switches.read().last() {
        text.0 = format!(
            "Switch {:?} is now {}",
            e.entity,
            if e.is_on { "on" } else { "off" }
        );
    }
    if let Some(e) = radios.read().last() {
        text.0 = format!("Radio {:?} selected", e.entity);
    }
    if let Some(e) = combos.read().last() {
        text.0 = format!("Combo box chose {}", e.selected_value);
    }
    if let Some(e) = trees.read().last() {
        text.0 = format!("Tree row {} selected", e.node_id);
    }
    if let Some(e) = texts.read().last() {
        text.0 = format!("Text is now {:?}", e.text);
    }
}

fn take_screenshot(
    mut commands: Commands,
    plan: Option<ResMut<ShotPlan>>,
    mut exit: MessageWriter<AppExit>,
    mut theme: MessageWriter<ThemeChanged>,
) {
    let Some(mut plan) = plan else { return };
    plan.frames += 1;
    if plan.frames == 5 && std::env::var_os("OPTS_GALLERY_LIGHT").is_some() {
        theme.write(ThemeChanged { is_dark: false });
    }
    if plan.frames == 40 {
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk(plan.path.clone()));
    }
    if plan.frames == 80 {
        exit.write(AppExit::Success);
    }
}
