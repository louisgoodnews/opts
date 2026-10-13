//! The hero picture of the *Continue* card: `cover` crop, never a band beside the image.
//!
//! Bevy stretches an image to its node, so the picture is cropped to the node's aspect ratio
//! with the `rect` of the [`ImageNode`], centred, and re-cropped whenever the node is resized.

use std::collections::HashMap;

use bevy::prelude::*;
use bevy::tasks::{futures::check_ready, IoTaskPool, Task};
use opts_gui::prelude::{RadiusLevel, RadiusToken};

use crate::menu::background::{decode_map_picture, upload_picture, DecodedPicture};
use crate::menu::background::MenuBackgroundScrim;

/// A card that wants the picture of this map folder (a folder of `OMSI/maps`).
#[derive(Component, Debug, Clone)]
pub struct HeroPending {
    /// The map's folder name.
    pub folder: String,
}

/// The decodes in flight, by card.
#[derive(Resource, Default)]
pub struct HeroLoads(HashMap<Entity, Task<Option<DecodedPicture>>>);

/// Starts the decode for each new [`HeroPending`] card on the IO pool.
pub fn start_hero_loads(
    mut loads: ResMut<HeroLoads>,
    cards: Query<(Entity, &HeroPending), Added<HeroPending>>,
) {
    for (card, pending) in &cards {
        let folder = pending.folder.clone();
        let task = IoTaskPool::get_or_init(bevy::tasks::TaskPool::new).spawn(async move {
            let dir = crate::omsi_root::known_omsi_root()?.join("maps").join(folder);
            decode_map_picture(&dir)
        });
        loads.0.insert(card, task);
    }
}

/// Puts finished pictures behind the text of their card.
pub fn finish_hero_loads(
    mut commands: Commands,
    mut loads: ResMut<HeroLoads>,
    mut images: ResMut<Assets<Image>>,
    cards: Query<(), With<HeroPending>>,
) {
    let mut done = Vec::new();
    for (card, task) in &mut loads.0 {
        if cards.get(*card).is_err() {
            done.push((*card, None));
        } else if let Some(result) = check_ready(task) {
            done.push((*card, result));
        }
    }
    for (card, picture) in done {
        loads.0.remove(&card);
        let Some(mut card_cmd) = commands.get_entity(card).ok() else {
            continue;
        };
        card_cmd.remove::<HeroPending>();
        let Some(pic) = picture else { continue };
        let (handle, stats) = upload_picture(pic, &mut images);
        let size = images.get(&handle).map_or(Vec2::ONE, Image::size_f32);
        let fill = Node {
            position_type: PositionType::Absolute,
            left: Val::Px(0.0),
            top: Val::Px(0.0),
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            ..default()
        };
        let image = commands
            .spawn((
                ImageNode::new(handle),
                fill.clone(),
                RadiusToken(RadiusLevel::Large),
                HeroImage { size },
                opts_gui::ui_tree::A11yHidden,
            ))
            .id();
        let scrim = commands
            .spawn((
                fill,
                RadiusToken(RadiusLevel::Large),
                BackgroundColor(Color::NONE),
                MenuBackgroundScrim(stats),
            ))
            .id();
        // Before the text, so the picture stays behind it.
        commands.entity(card).insert_children(0, &[image, scrim]);
    }
}

/// The picture's size in pixels; present on the image node of a hero.
#[derive(Component, Debug, Clone, Copy)]
pub struct HeroImage {
    /// Width and height of the texture.
    pub size: Vec2,
}

/// The centred part of an image of `image` pixels that fills a node of `node` size without
/// distortion (CSS `object-fit: cover`).
pub fn cover_rect(image: Vec2, node: Vec2) -> Rect {
    if image.x <= 0.0 || image.y <= 0.0 || node.x <= 0.0 || node.y <= 0.0 {
        return Rect::new(0.0, 0.0, image.x.max(1.0), image.y.max(1.0));
    }
    let image_aspect = image.x / image.y;
    let node_aspect = node.x / node.y;
    let (w, h) = if node_aspect > image_aspect {
        (image.x, image.x / node_aspect)
    } else {
        (image.y * node_aspect, image.y)
    };
    let x = (image.x - w) / 2.0;
    let y = (image.y - h) / 2.0;
    Rect::new(x, y, x + w, y + h)
}

/// Keeps every hero cropped to its node.
pub fn sync_hero_crop(mut heroes: Query<(&HeroImage, &ComputedNode, &mut ImageNode)>) {
    for (hero, node, mut image) in &mut heroes {
        let size = node.size();
        if size.x <= 0.0 || size.y <= 0.0 {
            continue;
        }
        let wanted = cover_rect(hero.size, size);
        if image.rect != Some(wanted) {
            image.rect = Some(wanted);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn aspect(r: Rect) -> f32 {
        r.width() / r.height()
    }

    #[test]
    fn a_wide_node_crops_the_top_and_bottom_of_a_squarer_picture() {
        let r = cover_rect(Vec2::new(1600.0, 1200.0), Vec2::new(1000.0, 250.0));
        assert!((aspect(r) - 4.0).abs() < 1e-3);
        assert_eq!((r.min.x, r.max.x), (0.0, 1600.0));
        assert!((r.min.y - (1200.0 - 400.0) / 2.0).abs() < 1e-3);
    }

    #[test]
    fn a_tall_node_crops_the_sides_and_always_stays_inside_the_picture() {
        let image = Vec2::new(1600.0, 900.0);
        for node in [
            Vec2::new(300.0, 600.0),
            Vec2::new(1600.0, 900.0),
            Vec2::new(10.0, 10.0),
        ] {
            let r = cover_rect(image, node);
            assert!(
                r.min.x >= 0.0
                    && r.min.y >= 0.0
                    && r.max.x <= image.x + 1e-3
                    && r.max.y <= image.y + 1e-3
            );
            assert!((aspect(r) - node.x / node.y).abs() < 1e-3, "{node:?}");
        }
    }

    #[test]
    fn a_degenerate_node_shows_the_whole_picture() {
        let r = cover_rect(Vec2::new(100.0, 50.0), Vec2::ZERO);
        assert_eq!(r.size(), Vec2::new(100.0, 50.0));
    }
}
