//! Unit tests for `super` (`text_texture/gpu.rs`).

use super::*;

/// A redrawn image marks the materials showing it (and only those) modified, so they are
/// bound to its new GPU texture.
#[test]
fn redrawn_image_rebinds_its_materials() {
    use bevy::prelude::*;
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default()))
        .init_asset::<Image>()
        .init_asset::<StandardMaterial>()
        .init_resource::<RedrawnImages>()
        .add_systems(Update, rebind_redrawn_images);
    let (text, other) = {
        let mut images = app.world_mut().resource_mut::<Assets<Image>>();
        (
            images.add(text_image(4, 4, vec![0; 64])),
            images.add(text_image(4, 4, vec![0; 64])),
        )
    };
    let (shows, unrelated) = {
        let mut materials = app.world_mut().resource_mut::<Assets<StandardMaterial>>();
        (
            materials.add(StandardMaterial::from(text.clone())),
            materials.add(StandardMaterial::from(other)),
        )
    };
    app.update();
    app.world_mut()
        .resource_mut::<Messages<AssetEvent<StandardMaterial>>>()
        .clear();
    app.world_mut()
        .resource_mut::<RedrawnImages>()
        .0
        .insert(text.id());
    app.update();
    app.update();
    let messages = app
        .world()
        .resource::<Messages<AssetEvent<StandardMaterial>>>();
    let modified: Vec<_> = messages
        .iter_current_update_messages()
        .chain(messages.get_cursor().read(messages))
        .filter_map(|e| match e {
            AssetEvent::Modified { id } => Some(*id),
            _ => None,
        })
        .collect();
    assert!(modified.contains(&shows.id()));
    assert!(!modified.contains(&unrelated.id()));
    assert!(app.world().resource::<RedrawnImages>().0.is_empty());
}

/// A redrawn clamped image stays clamped; a repeating one stays repeating.
#[test]
fn redrawn_image_keeps_its_addressing() {
    let clamped = text_image_addressed(4, 4, vec![0; 64], true);
    let plain = text_image(4, 4, vec![0; 64]);
    assert!(is_clamped(&clamped));
    assert!(!is_clamped(&plain));
    assert!(is_clamped(&redrawn_as(
        &clamped,
        text_image(4, 4, vec![9; 64])
    )));
    assert!(!is_clamped(&redrawn_as(
        &plain,
        text_image(4, 4, vec![9; 64])
    )));
}

/// A text image with a mip chain is not copied into its old GPU texture (which would fill
/// only the largest level).
#[test]
fn mipmapped_text_image_gets_a_new_texture() {
    let image = text_image(64, 8, vec![0; 64 * 8 * 4]);
    if image.texture_descriptor.mip_level_count > 1 {
        assert!(!image
            .texture_descriptor
            .usage
            .contains(TextureUsages::COPY_DST));
    }
}
