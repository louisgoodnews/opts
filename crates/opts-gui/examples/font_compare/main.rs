//! Rendered typeface comparison (decision D-UI-03): draws the same specimen lines in each
//! candidate family found on this machine, saves a PNG and exits.
//!
//! Development tool: it reads candidate font files from well-known system locations (they are
//! **not** bundled; only the winner is). Output: `OPTS_FONT_COMPARE_OUT` (default
//! `font_compare.png` in the working directory); `OPTS_FONT_COMPARE_SIZE` the base size (14).

use bevy::prelude::*;
use bevy::render::view::screenshot::{save_to_disk, Screenshot};

mod columns;
mod fonts;

use columns::{line, spawn_candidate_column};
use fonts::{load_font_handles, CANDIDATES, MONO};

#[derive(Resource)]
struct Out(String);

#[derive(Resource, Default)]
struct Frames(u32);

fn main() {
    let out = std::env::var("OPTS_FONT_COMPARE_OUT").unwrap_or_else(|_| "font_compare.png".into());
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "OPTS font comparison".into(),
                resolution: bevy::window::WindowResolution::new(1600, 1000),
                ..default()
            }),
            ..default()
        }))
        .insert_resource(Out(out))
        .init_resource::<Frames>()
        .add_systems(Startup, setup)
        .add_systems(Update, capture_then_exit)
        .run();
}
fn setup(mut commands: Commands, mut fonts: ResMut<Assets<Font>>) {
    commands.spawn(Camera2d);
    let base: f32 = std::env::var("OPTS_FONT_COMPARE_SIZE")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(14.0);
    let handles = load_font_handles(&mut fonts);
    // keep the handles alive for the whole run
    commands.insert_resource(Keep(handles));

    let bg = Color::srgb(0.07, 0.07, 0.09);
    let fg = Color::srgb(0.89, 0.89, 0.90);
    let dim = Color::srgb(0.62, 0.79, 1.0);
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                padding: UiRect::all(Val::Px(16.0)),
                flex_direction: FlexDirection::Row,
                flex_wrap: FlexWrap::Wrap,
                column_gap: Val::Px(24.0),
                row_gap: Val::Px(12.0),
                ..default()
            },
            BackgroundColor(bg),
        ))
        .with_children(|root| {
            for (family, _) in CANDIDATES {
                spawn_candidate_column(root, family, base, fg, dim);
            }
            for (family, _) in MONO {
                root.spawn(Node {
                    flex_direction: FlexDirection::Column,
                    width: Val::Px(760.0),
                    row_gap: Val::Px(2.0),
                    ..default()
                })
                .with_children(|col| {
                    line(
                        col,
                        &format!("{family} (companion)"),
                        family,
                        700,
                        base + 4.0,
                        dim,
                        false,
                    );
                    line(
                        col,
                        "fps 144.2  entities 18432  decode 0.37 ms  il1I O0 {}[]",
                        family,
                        400,
                        base,
                        fg,
                        false,
                    );
                });
            }
        });
}

#[derive(Resource)]
struct Keep(#[allow(dead_code)] Vec<Handle<Font>>);
fn capture_then_exit(
    mut commands: Commands,
    mut frames: ResMut<Frames>,
    out: Res<Out>,
    mut exit: MessageWriter<AppExit>,
) {
    frames.0 += 1;
    if frames.0 == 30 {
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk(out.0.clone()));
    }
    if frames.0 == 90 {
        exit.write(AppExit::Success);
    }
}
