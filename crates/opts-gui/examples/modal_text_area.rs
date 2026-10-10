//! Example: a modal dialog with a multiline text area, Report/Cancel buttons and a success toast.
//!
//! Type with the keyboard (Enter = newline, arrows/Home/End, Ctrl+A/C/X/V), Tab cycles inside the
//! modal, Escape or Cancel closes it, Report prints the text to the log.

use bevy::prelude::*;
use opts_gui::icons;
use opts_gui::prelude::*;

#[derive(Component)]
struct ReportButton;
#[derive(Component)]
struct CancelButton;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "OPTS GUI — Modal & TextArea".into(),
                resolution: (1100.0_f32, 760.0_f32).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(OptsGuiPlugin)
        .add_systems(Startup, setup)
        .add_systems(Update, (handle_buttons, handle_escape, screenshot_for_ci))
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
    let (_root, content) = ModalBuilder::new().spawn(&mut commands);
    commands.entity(content).with_children(|p| {
        p.spawn(icons::icon_text(
            &[
                icons::SEARCH,
                icons::DIRECTIONS_BUS,
                icons::THUNDERSTORM,
                icons::LOCATION_ON,
            ]
            .concat(),
            28.0,
            Color::WHITE,
        ));
        p.spawn(TextBundle::from_section(
            "Describe the issue (Größe, Ärger, ✓ ▼ ↺):",
            TextStyle {
                font_size: 16.0,
                color: Color::WHITE,
                ..default()
            },
        ));
        TextAreaBuilder::new()
            .with_placeholder("What happened?")
            .with_text(
                std::env::var("OPTS_EXAMPLE_TEXT")
                    .unwrap_or_default()
                    .replace("\\n", "\n"),
            )
            .autofocus(true)
            .spawn_child(p);
        p.spawn(NodeBundle {
            style: Style {
                column_gap: Val::Px(12.0),
                justify_content: JustifyContent::FlexEnd,
                ..default()
            },
            ..default()
        })
        .with_children(|row| {
            let r = ButtonBuilder::filled("Report").spawn_child(row);
            row.add_command(move |world: &mut World| {
                world.entity_mut(r).insert(ReportButton);
            });
            let c = ButtonBuilder::outlined("Cancel").spawn_child(row);
            row.add_command(move |world: &mut World| {
                world.entity_mut(c).insert(CancelButton);
            });
        });
    });
}

fn handle_buttons(
    mut clicks: EventReader<ButtonClicked>,
    report: Query<(), With<ReportButton>>,
    cancel: Query<(), With<CancelButton>>,
    areas: Query<&TextArea>,
    modals: Query<Entity, With<ModalRoot>>,
    mut commands: Commands,
) {
    for ButtonClicked(e) in clicks.read() {
        if report.contains(*e) {
            let text = areas.iter().next().map(TextArea::value).unwrap_or_default();
            info!("report text: {text:?}");
            ToastBuilder::new("Report saved")
                .with_duration(4.0)
                .spawn(&mut commands);
        } else if !cancel.contains(*e) {
            continue;
        }
        for m in &modals {
            commands.entity(m).despawn_recursive();
        }
    }
}

fn handle_escape(mut events: EventReader<ModalEvent>, mut commands: Commands) {
    for ModalEvent::CancelRequested(m) in events.read() {
        commands.entity(*m).despawn_recursive();
    }
}

/// With `OPTS_EXAMPLE_SCREENSHOT=<file.png>` saves one frame after start-up and exits (visual checks).
fn screenshot_for_ci(
    mut frames: Local<u32>,
    mut shots: ResMut<bevy::render::view::screenshot::ScreenshotManager>,
    window: Query<Entity, With<bevy::window::PrimaryWindow>>,
    mut exit: EventWriter<AppExit>,
) {
    let Ok(path) = std::env::var("OPTS_EXAMPLE_SCREENSHOT") else {
        return;
    };
    *frames += 1;
    if *frames == 30 {
        if let Ok(w) = window.get_single() {
            let _ = shots.save_screenshot_to_disk(w, path);
        }
    }
    if *frames == 60 {
        exit.send(AppExit::Success);
    }
}
