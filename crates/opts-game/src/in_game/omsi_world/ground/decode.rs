//! Decoding the layer textures once per map and uploading them as one mipmapped array.

use super::*;

/// The base and the painted layers: (layer number, texture, detail texture). `None` when the base
/// layer cannot be decoded; another layer that cannot is skipped.
pub(super) type DecodedLayers = Vec<(
    usize,
    opts_bevy::ground::CpuImage,
    Option<opts_bevy::ground::CpuImage>,
)>;

pub(super) fn decode_layers(
    job: &mut WorldLoadJob,
    (meshes, materials, images): (
        &mut Assets<Mesh>,
        &mut Assets<StandardMaterial>,
        &mut Assets<Image>,
    ),
    (wanted, dirs): (&[usize], &[std::path::PathBuf]),
) -> Option<DecodedLayers> {
    use opts_bevy::ground::CpuImage;
    let mut decoded: DecodedLayers = Vec::new();
    for &n in wanted {
        let tex = &job.manifest.ground_textures[n];
        let (name, detail_name) = (tex.ground_texture.clone(), tex.detail_texture.clone());
        let mut loader = ContentLoader {
            install: &job.install,
            cache: &mut job.cache,
            meshes,
            materials,
            images,
        };
        let img = loader
            .texture(&name, dirs)
            .and_then(|h| loader.images.get(&h).and_then(CpuImage::from_image));
        let detail = if detail_name.trim().is_empty() {
            None
        } else {
            loader
                .texture(&detail_name, dirs)
                .and_then(|h| loader.images.get(&h).and_then(CpuImage::from_image))
        };
        match img {
            Some(img) => decoded.push((n, img, detail)),
            None if n == 0 => return None,
            None => {
                tracing::warn!(layer = n, texture = %name, "ground layer texture cannot be decoded; layer skipped");
            }
        }
    }
    Some(decoded)
}

/// Decodes the base and every painted layer once per map and uploads them as one mipmapped array
/// (`job.ground_array`). `None` when the base layer cannot be decoded.
pub(super) fn build_ground_array(
    job: &mut WorldLoadJob,
    (images, materials, meshes): (
        &mut Assets<Image>,
        &mut Assets<StandardMaterial>,
        &mut Assets<Mesh>,
    ),
    masks: &[(usize, opts_bevy::ground::CpuImage)],
    layer_total: usize,
) -> Option<()> {
    use opts_bevy::ground::CpuImage;
    use opts_bevy::ground_material::{layer_array, layer_array_size};
    // Decode the base and every painted layer once per map and upload them as one
    // mipmapped array.
    let dirs = vec![
        job.config.map_dir.clone(),
        job.config.map_dir.join("texture"),
    ];
    let mut wanted = vec![0];
    wanted.extend(painted_layers(&job.config.map_dir, layer_total));
    // A mask found through the install index but missed by the folder scan still counts.
    for (n, _) in masks {
        if !wanted.contains(n) {
            wanted.push(*n);
        }
    }
    wanted.sort_unstable();
    let decoded = decode_layers(job, (meshes, materials, images), (&wanted, &dirs))?;
    let largest = decoded
        .iter()
        .map(|(_, d, _)| d.width.max(d.height))
        .max()
        .unwrap_or(512);
    let size = layer_array_size(decoded.len(), largest);
    let refs: Vec<&CpuImage> = decoded.iter().map(|(_, d, _)| d).collect();
    let layers = images.add(layer_array(&refs, size));
    // Detail textures modulate their layer; a missing one is white (no change).
    let white = CpuImage {
        width: 1,
        height: 1,
        rgba: vec![255; 4],
    };
    let detail_refs: Vec<&CpuImage> = decoded
        .iter()
        .map(|(_, _, d)| d.as_ref().unwrap_or(&white))
        .collect();
    let details = images.add(layer_array(&detail_refs, 256));
    let mut index = vec![None; layer_total];
    for (slot, (n, _, _)) in decoded.iter().enumerate() {
        index[*n] = Some(slot as u32);
    }
    tracing::info!(
        listed = layer_total,
        in_array = decoded.len(),
        size,
        "ground layer array built"
    );
    job.ground_array = Some(GroundArray {
        layers,
        details,
        index,
    });
    Some(())
}
