//! Map-backed background image loading with contrast scrim and solid fallback.

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use opts_gui::theme::color::ColorRole;
use opts_gui::theme::MaterialTheme;
use std::path::Path;

/// Marker component for the main menu background container.
#[derive(Component)]
pub struct MenuBackgroundRoot;

/// Marker component for the contrast scrim overlay.
#[derive(Component)]
pub struct MenuBackgroundScrim;

/// Spawns the main menu background layer.
///
/// If `map_path` contains a readable `picture.jpg`, loads and displays it across the viewport.
/// Otherwise, falls back to a solid `ColorRole::Surface` background.
/// Always inserts a semi-transparent scrim node (alpha ~0.45) over the background
/// to guarantee WCAG 2.1 AA text legibility.
pub fn spawn_menu_background(
    commands: &mut Commands,
    map_dir: Option<&Path>,
    images: &mut ResMut<Assets<Image>>,
    theme: &MaterialTheme,
) -> Entity {
    let root = commands
        .spawn((
            (
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(0.0),
                    right: Val::Px(0.0),
                    top: Val::Px(0.0),
                    bottom: Val::Px(0.0),
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    ..default()
                },
                ZIndex(0),
            ),
            MenuBackgroundRoot,
        ))
        .id();

    let image_handle = map_dir.and_then(|dir| load_map_picture(dir, images));

    if let Some(handle) = image_handle {
        tracing::info!("Map background picture successfully attached to main menu");
        commands.entity(root).with_children(|parent| {
            // Background image layer (scaled to fill)
            parent.spawn((
                ImageNode::new(handle),
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(0.0),
                    right: Val::Px(0.0),
                    top: Val::Px(0.0),
                    bottom: Val::Px(0.0),
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    ..default()
                },
            ));

            // Contrast scrim layer (z=1 relative to background root)
            parent.spawn((
                (
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(0.0),
                        right: Val::Px(0.0),
                        top: Val::Px(0.0),
                        bottom: Val::Px(0.0),
                        width: Val::Percent(100.0),
                        height: Val::Percent(100.0),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.45)),
                    ZIndex(1),
                ),
                MenuBackgroundScrim,
            ));
        });
    } else {
        tracing::info!(
            "No map background picture provided or found; rendering solid theme surface"
        );
        let fallback_color = theme.scheme.get(ColorRole::Surface);
        commands
            .entity(root)
            .insert(BackgroundColor(fallback_color));
    }

    root
}

/// Attempts to load `picture.jpg` from the specified map directory.
fn load_map_picture(dir: &Path, images: &mut ResMut<Assets<Image>>) -> Option<Handle<Image>> {
    let picture_path = dir.join("picture.jpg");
    tracing::info!(path = ?picture_path, "Attempting to load map background image");

    if !picture_path.exists() {
        tracing::warn!(path = ?picture_path, "Map picture.jpg does not exist");
        return None;
    }

    match image::open(&picture_path) {
        Ok(dyn_img) => {
            let rgba = dyn_img.to_rgba8();
            let (width, height) = rgba.dimensions();

            let bevy_image = Image::new(
                Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                TextureDimension::D2,
                rgba.into_raw(),
                TextureFormat::Rgba8UnormSrgb,
                RenderAssetUsages::default(),
            );

            let handle = images.add(bevy_image);
            tracing::info!(
                width = width,
                height = height,
                "Loaded picture.jpg texture successfully"
            );
            Some(handle)
        }
        Err(err) => {
            tracing::warn!(
                err = ?err,
                path = ?picture_path,
                "Failed to decode map picture.jpg; falling back to solid surface"
            );
            None
        }
    }
}
