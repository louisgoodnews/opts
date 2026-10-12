//! Map-backed background image loading with contrast scrim and solid fallback.

use bevy::asset::RenderAssetUsages;
use bevy::color::Alpha;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use opts_gui::theme::color::ColorRole;
use opts_gui::theme::{luminance_stats, scrim_for_text, ImageLuminance, MaterialTheme};
use opts_gui::tokens::BackgroundRole;
use std::path::Path;

/// Marker component for the main menu background container.
#[derive(Component)]
pub struct MenuBackgroundRoot;

/// The contrast scrim over the background picture. It carries the picture's lightness statistics;
/// [`sync_menu_scrim`] derives its colour and strength from them and from the theme, so text keeps
/// 4.5:1 over bright and dark pictures alike.
#[derive(Component, Debug, Clone, Copy)]
pub struct MenuBackgroundScrim(pub ImageLuminance);

/// Text roles that sit directly on the picture (the title and the quieter lines below it).
const TEXT_ON_PICTURE: [ColorRole; 2] = [ColorRole::OnSurface, ColorRole::OnSurfaceVariant];

/// The scrim colour for a picture: the strongest of the veils the text roles need.
pub fn menu_scrim_color(theme: &MaterialTheme, stats: ImageLuminance) -> Color {
    TEXT_ON_PICTURE
        .into_iter()
        .map(|role| scrim_for_text(&theme.scheme, role, stats, 4.5, 0.25))
        .max_by(|a, b| a.alpha().total_cmp(&b.alpha()))
        .unwrap_or(Color::BLACK.with_alpha(0.5))
}

/// Recomputes the scrim colour when the theme changes or a scrim appears.
pub fn sync_menu_scrim(
    theme: Res<MaterialTheme>,
    mut scrims: Query<(Ref<MenuBackgroundScrim>, &mut BackgroundColor)>,
) {
    for (scrim, mut bg) in &mut scrims {
        if theme.is_changed() || scrim.is_added() {
            bg.0 = menu_scrim_color(&theme, scrim.0);
        }
    }
}

/// Spawns the main menu background layer.
///
/// If `map_path` contains a readable `picture.jpg`, loads and displays it across the viewport.
/// Otherwise, falls back to a solid `ColorRole::Surface` background.
/// Over a picture a scrim node is inserted whose strength is computed from the picture and the
/// theme ([`menu_scrim_color`]) to guarantee WCAG 2.1 AA text legibility.
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

    if let Some((handle, stats)) = image_handle {
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
                    BackgroundColor(menu_scrim_color(theme, stats)),
                    ZIndex(1),
                ),
                MenuBackgroundScrim(stats),
            ));
        });
    } else {
        tracing::info!(
            "No map background picture provided or found; rendering solid theme surface"
        );
        // A role, not a copy of the colour: the surface follows theme changes.
        commands.entity(root).insert((
            BackgroundColor(theme.scheme.get(ColorRole::Surface)),
            BackgroundRole(ColorRole::Surface),
        ));
    }

    root
}

/// Attempts to load `picture.jpg` from the specified map directory.
pub fn load_map_picture(
    dir: &Path,
    images: &mut ResMut<Assets<Image>>,
) -> Option<(Handle<Image>, ImageLuminance)> {
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
            let stats = luminance_stats(rgba.as_raw(), 4);

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
            Some((handle, stats))
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

#[cfg(test)]
mod tests {
    use super::*;
    use opts_gui::theme::contrast::{composite_over, contrast_ratio};

    /// A flat grey image's stats, as `load_map_picture` would measure them.
    fn grey_stats(v: u8) -> ImageLuminance {
        let px: Vec<u8> = (0..400).flat_map(|_| [v, v, v, 255]).collect();
        luminance_stats(&px, 4)
    }

    #[test]
    fn title_text_keeps_contrast_over_a_bright_and_a_dark_picture_in_every_theme() {
        for theme in [
            MaterialTheme::dark(),
            MaterialTheme::light(),
            MaterialTheme::high_contrast(),
        ] {
            for v in [255, 0, 128] {
                let scrim = menu_scrim_color(&theme, grey_stats(v));
                let backdrop = Color::srgb_u8(v, v, v);
                let over = composite_over(scrim, backdrop);
                for role in TEXT_ON_PICTURE {
                    let ratio = contrast_ratio(theme.scheme.get(role), over);
                    assert!(
                        ratio >= 4.5,
                        "{:?} picture {v} {role:?}: {ratio:.2}",
                        theme.kind()
                    );
                }
            }
        }
    }

    #[test]
    fn the_scrim_follows_a_theme_change() {
        let mut app = App::new();
        app.insert_resource(MaterialTheme::dark())
            .add_systems(Update, sync_menu_scrim);
        let e = app
            .world_mut()
            .spawn((
                MenuBackgroundScrim(grey_stats(255)),
                BackgroundColor(Color::NONE),
            ))
            .id();
        app.update();
        let dark = app.world().get::<BackgroundColor>(e).map(|b| b.0);
        *app.world_mut().resource_mut::<MaterialTheme>() = MaterialTheme::light();
        app.update();
        let light = app.world().get::<BackgroundColor>(e).map(|b| b.0);
        assert_ne!(dark, light);
        assert_ne!(dark, Some(Color::NONE));
    }
}
