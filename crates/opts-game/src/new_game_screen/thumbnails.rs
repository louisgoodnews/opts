//! Thumbnails and hero images: decoded off the main thread, cached as small PNGs under the user
//! cache directory (with a size cap) and put into the slots the lists and details provide.
//!
//! Maps use the `picture.jpg` of their folder. Vehicles use a preview image when the content has
//! one (`picture`/`preview`/`thumbnail` next to the `.bus` file); none of the shipped vehicles
//! does, so they keep the neutral placeholder glyph — nothing is invented. Livery swatches take
//! the average colour of the repaint's first readable texture.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::tasks::{futures::check_ready, IoTaskPool, Task};
use opts_gui::patterns::{RowThumb, ThumbGlyph};

use super::livery::{LiveryState, LiverySwatch};
use super::step_where::HeroImage;
use super::types::GameCatalogue;

/// The placeholder glyph inside a hero or thumbnail slot; removed when the image arrives.
#[derive(Component, Debug, Clone, Copy)]
pub struct HeroGlyph;

/// A slot that has been dealt with (filled, or nothing to show).
#[derive(Component, Debug, Clone, Copy)]
pub struct ThumbDone;

/// Default cache size cap.
pub const DEFAULT_CAP_BYTES: u64 = 64 * 1024 * 1024;
/// Size of a map hero image.
pub const HERO_SIZE: (u32, u32) = (320, 180);
/// Size of a square thumbnail.
pub const SQUARE_SIZE: (u32, u32) = (96, 96);

/// What a finished request produced.
#[derive(Debug, Clone, PartialEq)]
pub enum Thumb {
    /// Decoded pixels.
    Pixels {
        /// Width.
        width: u32,
        /// Height.
        height: u32,
        /// RGBA8 data.
        rgba: Vec<u8>,
    },
    /// An average colour (livery swatch).
    Colour([u8; 3]),
}

/// The state of one request.
enum Entry {
    Pending(Task<Option<Thumb>>),
    Ready(Option<Handle<Image>>, Option<[u8; 3]>),
}

/// Requests, cache and results of thumbnails.
#[derive(Resource)]
pub struct ThumbnailService {
    /// Cache directory of the small PNGs.
    pub cache_dir: PathBuf,
    /// Size cap of the cache.
    pub cap_bytes: u64,
    entries: HashMap<String, Entry>,
}

impl Default for ThumbnailService {
    fn default() -> Self {
        let dir = directories::ProjectDirs::from("org", "opts", "opts").map_or_else(
            || std::env::temp_dir().join("opts-thumbnails"),
            |d| d.cache_dir().join("thumbnails"),
        );
        Self::new(dir, DEFAULT_CAP_BYTES)
    }
}

/// The outcome of asking for a thumbnail.
#[derive(Debug, Clone, PartialEq)]
pub enum Lookup {
    /// Still being decoded.
    Pending,
    /// An image is ready.
    Image(Handle<Image>),
    /// A colour is ready.
    Colour([u8; 3]),
    /// There is nothing to show for this request.
    Nothing,
}

impl ThumbnailService {
    /// A service with its own cache directory and cap.
    pub fn new(cache_dir: PathBuf, cap_bytes: u64) -> Self {
        Self {
            cache_dir,
            cap_bytes,
            entries: HashMap::new(),
        }
    }

    /// Starts decoding `source` for `id` unless it is already known.
    pub fn request(&mut self, id: &str, source: Option<PathBuf>, size: (u32, u32), colour: bool) {
        if self.entries.contains_key(id) {
            return;
        }
        let Some(source) = source else {
            self.entries
                .insert(id.to_string(), Entry::Ready(None, None));
            return;
        };
        let cache = self.cache_dir.clone();
        let cap = self.cap_bytes;
        let key = id.to_string();
        let task = IoTaskPool::get().spawn(async move {
            if colour {
                average_colour(&source).map(Thumb::Colour)
            } else {
                load_thumb(&cache, &key, &source, size, cap)
            }
        });
        self.entries.insert(id.to_string(), Entry::Pending(task));
    }

    /// Like [`ThumbnailService::request`], but the source file is looked for on the worker
    /// thread (a stat per map, a folder listing per vehicle: far too slow for the frame that
    /// builds a list of hundreds of rows).
    pub fn request_probe(
        &mut self,
        id: &str,
        root: PathBuf,
        probe: Option<Probe>,
        size: (u32, u32),
    ) {
        if self.entries.contains_key(id) {
            return;
        }
        let Some(probe) = probe else {
            self.entries
                .insert(id.to_string(), Entry::Ready(None, None));
            return;
        };
        let cache = self.cache_dir.clone();
        let cap = self.cap_bytes;
        let key = id.to_string();
        let task = IoTaskPool::get_or_init(bevy::tasks::TaskPool::new).spawn(async move {
            let source = probe.resolve(&root)?;
            load_thumb(&cache, &key, &source, size, cap)
        });
        self.entries.insert(id.to_string(), Entry::Pending(task));
    }

    /// Collects finished decodes into images.
    pub fn poll(&mut self, images: &mut Assets<Image>) {
        for entry in self.entries.values_mut() {
            let Entry::Pending(task) = entry else {
                continue;
            };
            let Some(result) = check_ready(task) else {
                continue;
            };
            *entry = match result {
                Some(Thumb::Pixels {
                    width,
                    height,
                    rgba,
                }) => Entry::Ready(Some(images.add(to_image(width, height, rgba))), None),
                Some(Thumb::Colour(c)) => Entry::Ready(None, Some(c)),
                None => Entry::Ready(None, None),
            };
        }
    }

    /// The state of request `id`.
    pub fn lookup(&self, id: &str) -> Option<Lookup> {
        Some(match self.entries.get(id)? {
            Entry::Pending(_) => Lookup::Pending,
            Entry::Ready(Some(h), _) => Lookup::Image(h.clone()),
            Entry::Ready(None, Some(c)) => Lookup::Colour(*c),
            Entry::Ready(None, None) => Lookup::Nothing,
        })
    }
}

fn to_image(width: u32, height: u32, rgba: Vec<u8>) -> Image {
    Image::new(
        Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        rgba,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    )
}

fn fnv(text: &str) -> u64 {
    text.bytes().fold(0xcbf2_9ce4_8422_2325, |h, b| {
        (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3)
    })
}

/// Cache file of a request: the key, the source's identity (path, size, modification time) and
/// the target size, so a changed picture is rebuilt.
fn cache_file(cache: &Path, key: &str, src: &Path, size: (u32, u32)) -> Option<PathBuf> {
    let meta = std::fs::metadata(src).ok()?;
    let stamp = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
        .map_or(0, |d| d.as_secs());
    let id = format!(
        "{key}|{}|{}|{stamp}|{}x{}",
        src.display(),
        meta.len(),
        size.0,
        size.1
    );
    Some(cache.join(format!("{:016x}.png", fnv(&id))))
}

/// Decodes `src` to `size` (cropped to fill), through the cache.
pub fn load_thumb(
    cache: &Path,
    key: &str,
    src: &Path,
    size: (u32, u32),
    cap_bytes: u64,
) -> Option<Thumb> {
    let file = cache_file(cache, key, src, size)?;
    if let Ok(img) = image::open(&file) {
        // A hit counts as use: the file's time moves forward for the least-recently-used rule.
        let _ = std::fs::File::options()
            .append(true)
            .open(&file)
            .and_then(|f| f.set_modified(SystemTime::now()));
        return Some(pixels(&img));
    }
    let img = image::open(src).ok()?;
    let small = img.resize_to_fill(size.0, size.1, image::imageops::FilterType::Triangle);
    if std::fs::create_dir_all(cache).is_ok() && small.save(&file).is_ok() {
        prune(cache, cap_bytes);
    }
    Some(pixels(&small))
}

fn pixels(img: &image::DynamicImage) -> Thumb {
    let rgba = img.to_rgba8();
    Thumb::Pixels {
        width: rgba.width(),
        height: rgba.height(),
        rgba: rgba.into_raw(),
    }
}

/// The average colour of an image file (8 × 8 samples).
pub fn average_colour(src: &Path) -> Option<[u8; 3]> {
    let img = image::open(src).ok()?.thumbnail(8, 8).to_rgb8();
    let n = u64::from(img.width() * img.height());
    if n == 0 {
        return None;
    }
    let mut sum = [0_u64; 3];
    for p in img.pixels() {
        for (s, c) in sum.iter_mut().zip(p.0) {
            *s += u64::from(c);
        }
    }
    Some([(sum[0] / n) as u8, (sum[1] / n) as u8, (sum[2] / n) as u8])
}

/// Removes the least recently used cache files until the cache fits `cap_bytes`.
pub fn prune(cache: &Path, cap_bytes: u64) {
    let Ok(read) = std::fs::read_dir(cache) else {
        return;
    };
    let mut files: Vec<(SystemTime, u64, PathBuf)> = read
        .flatten()
        .filter(|e| e.path().extension().is_some_and(|x| x == "png"))
        .filter_map(|e| {
            let m = e.metadata().ok()?;
            Some((m.modified().ok()?, m.len(), e.path()))
        })
        .collect();
    let mut total: u64 = files.iter().map(|f| f.1).sum();
    files.sort_by_key(|f| f.0);
    for (_, len, path) in files {
        if total <= cap_bytes {
            break;
        }
        if std::fs::remove_file(&path).is_ok() {
            total = total.saturating_sub(len);
        }
    }
}

mod config_slot;
mod slots;
pub use config_slot::{config_thumb_key, spawn_config_thumb, CONFIG_THUMB_PX};
pub use slots::{fill_thumbnails, probe_of, source_of, vehicle_preview, Probe};

#[cfg(test)]
mod tests;
