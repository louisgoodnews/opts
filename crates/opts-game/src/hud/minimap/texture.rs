//! The map's colours and pose, and the texture that the surface shows and the canvas draws into.

use super::*;

/// The map's colours from an instrument palette.
#[must_use]
pub fn colors_of(palette: &InstrumentPalette) -> MapColors {
    let u8s = |c: Color| c.to_srgba().to_u8_array();
    let mut ground = u8s(palette.panel);
    ground[3] = 255;
    MapColors {
        ground,
        road: u8s(palette.text_dim),
        active: u8s(palette.text),
        other: u8s(palette.warning),
        north: u8s(palette.text),
        route: u8s(palette.ok),
    }
}

/// The compass heading (radians) of a world transform, and its (east, north) position.
#[must_use]
pub fn pose_of(t: &GlobalTransform) -> (DVec2, f32) {
    let p = t.translation();
    let f = t.forward();
    (DVec2::new(f64::from(p.x), f64::from(-p.z)), f.x.atan2(-f.z))
}

/// Creates the texture when the surface needs another size and hands it to the surface node.
pub fn ensure_image(
    mut images: ResMut<Assets<Image>>,
    mut handle: ResMut<MiniMapImage>,
    mut canvas: ResMut<MiniMapCanvas>,
    mut state: ResMut<MiniMapState>,
    mut surfaces: Query<(&ComputedNode, &mut ImageNode), With<MiniMapSurface>>,
    palette_owner: (Option<Res<MaterialTheme>>, Res<InstrumentVariant>),
) {
    let Some((computed, mut image_node)) = surfaces.iter_mut().next() else {
        return;
    };
    // The texture matches the physical size of the node, within sensible bounds.
    let wanted = (computed.size().x.round() as u32).clamp(96, 448);
    let ground = palette_owner.0.as_ref().map_or([12, 15, 20, 255], |t| {
        colors_of(t.instrument(*palette_owner.1)).ground
    });
    if canvas.0.as_ref().is_none_or(|c| c.size != wanted) || handle.0.is_none() {
        let c = Canvas::new(wanted, ground);
        let image = Image::new(
            Extent3d {
                width: wanted,
                height: wanted,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            c.data.clone(),
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::default(),
        );
        handle.0 = Some(images.add(image));
        canvas.0 = Some(c);
        state.texture_px = wanted;
        // Draw soon: do not wait for the next tick with an empty map.
        state.since = UPDATE_INTERVAL.as_secs_f32();
    }
    if let Some(h) = &handle.0 {
        if image_node.image != *h {
            image_node.image = h.clone();
        }
    }
}
