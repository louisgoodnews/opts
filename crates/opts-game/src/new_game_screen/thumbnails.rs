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

/// A preview image the content provides for a vehicle, if any.
pub fn vehicle_preview(root: &Path, bus_file: &str) -> Option<PathBuf> {
    let bus = root.join(bus_file);
    let dir = bus.parent()?;
    for folder in [Some(dir), dir.parent()].into_iter().flatten() {
        for entry in std::fs::read_dir(folder).ok()?.flatten() {
            let name = entry.file_name().to_string_lossy().to_lowercase();
            let image_ext = [".jpg", ".jpeg", ".png"].iter().any(|e| name.ends_with(e));
            let preview = ["picture", "preview", "thumbnail"]
                .iter()
                .any(|w| name.starts_with(w));
            if image_ext && preview {
                return Some(entry.path());
            }
        }
    }
    None
}

/// What to look for when a thumbnail is asked for; resolved on a worker thread.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Probe {
    /// `picture.jpg` of a map folder.
    Map(String),
    /// A preview image next to a vehicle's `.bus` file (path relative to the OMSI root).
    Vehicle(String),
}

impl Probe {
    /// The file on disk, if there is one. Touches the file system.
    pub fn resolve(&self, root: &Path) -> Option<PathBuf> {
        match self {
            Self::Map(folder) => {
                let p = root.join("maps").join(folder).join("picture.jpg");
                p.is_file().then_some(p)
            }
            Self::Vehicle(bus_file) => vehicle_preview(root, bus_file),
        }
    }
}

/// What thumbnail key `key` (`map:<folder>`, `veh:<id>`) needs; no file system access.
pub fn probe_of(key: &str, catalogue: &GameCatalogue) -> Option<Probe> {
    if let Some(folder) = key.strip_prefix("map:") {
        return Some(Probe::Map(folder.to_string()));
    }
    let id = key.strip_prefix("veh:")?;
    let v = catalogue.vehicles.iter().find(|v| v.id == id)?;
    (!v.bus_file.is_empty()).then(|| Probe::Vehicle(v.bus_file.clone()))
}

/// The source file of thumbnail key `key` (`map:<folder>`, `veh:<id>`).
pub fn source_of(key: &str, catalogue: &GameCatalogue, root: &Path) -> Option<PathBuf> {
    probe_of(key, catalogue)?.resolve(root)
}

fn size_for(key: &str, hero: bool) -> (u32, u32) {
    if key.starts_with("map:") && hero {
        HERO_SIZE
    } else {
        SQUARE_SIZE
    }
}

fn slot_id(key: &str, hero: bool) -> String {
    let s = size_for(key, hero);
    format!("{key}@{}x{}", s.0, s.1)
}

/// Finishes decodes and fills the slots of lists, details and livery swatches.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn fill_thumbnails(
    mut service: ResMut<ThumbnailService>,
    mut images: ResMut<Assets<Image>>,
    catalogue: Res<GameCatalogue>,
    liveries: Res<LiveryState>,
    rows: Query<(Entity, &RowThumb), Without<ThumbDone>>,
    heroes: Query<(Entity, &HeroImage), Without<ThumbDone>>,
    swatches: Query<(Entity, &LiverySwatch), Without<ThumbDone>>,
    children: Query<&Children>,
    glyphs: Query<(), Or<(With<HeroGlyph>, With<ThumbGlyph>)>>,
    mut commands: Commands,
    service_root: Option<Res<crate::catalogue_service::CatalogueService>>,
    mut known_root: Local<Option<PathBuf>>,
    mut wait: Local<u32>,
) {
    service.poll(&mut images);
    let slots_waiting = !rows.is_empty() || !heroes.is_empty() || !swatches.is_empty();
    if !slots_waiting {
        return;
    }
    // The root comes from the catalogue service (no file system access); without a service the
    // search is repeated at most every 120 frames.
    *wait = wait.saturating_sub(1);
    if known_root.is_none() && *wait == 0 {
        *known_root = service_root.as_deref().map(|s| s.root.clone()).or_else(|| {
            *wait = 120;
            None
        });
    }
    let Some(root) = known_root.clone() else {
        return;
    };
    let mut slots: Vec<(Entity, String, bool)> = Vec::new();
    slots.extend(rows.iter().map(|(e, r)| (e, r.key.clone(), false)));
    slots.extend(heroes.iter().map(|(e, h)| (e, h.key.clone(), !h.square)));
    for (entity, key, hero) in slots {
        let id = slot_id(&key, hero);
        if service.lookup(&id).is_none() {
            let probe = probe_of(&key, &catalogue);
            service.request_probe(&id, root.clone(), probe, size_for(&key, hero));
        }
        match service.lookup(&id) {
            Some(Lookup::Image(handle)) => {
                for c in children.get(entity).into_iter().flat_map(|c| c.iter()) {
                    if glyphs.contains(c) {
                        commands.entity(c).despawn();
                    }
                }
                commands
                    .entity(entity)
                    .insert((ImageNode::new(handle), ThumbDone));
            }
            Some(Lookup::Nothing) => {
                commands.entity(entity).insert(ThumbDone);
            }
            _ => {}
        }
    }
    for (entity, sw) in &swatches {
        let id = format!("sw:{}:{}", sw.vehicle, sw.livery);
        if service.lookup(&id).is_none() {
            let src = (liveries.vehicle == sw.vehicle)
                .then(|| liveries.repaints.as_ref())
                .flatten()
                .and_then(|r| r.textures.get(&sw.livery))
                .and_then(|t| t.first().map(|(_, p)| p.clone()));
            if src.is_none() && liveries.repaints.is_none() {
                continue;
            }
            service.request(&id, src, (8, 8), true);
        }
        match service.lookup(&id) {
            Some(Lookup::Colour([r, g, b])) => {
                commands
                    .entity(entity)
                    .remove::<opts_gui::tokens::components::BackgroundRole>()
                    .insert((
                        BackgroundColor(Color::Srgba(Srgba {
                            red: f32::from(r) / 255.0,
                            green: f32::from(g) / 255.0,
                            blue: f32::from(b) / 255.0,
                            alpha: 1.0,
                        })),
                        ThumbDone,
                    ));
            }
            Some(Lookup::Nothing) => {
                commands.entity(entity).insert(ThumbDone);
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dir(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("opts_thumbs_{tag}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).expect("dir");
        d
    }

    fn picture(path: &Path, w: u32, h: u32, rgb: [u8; 3]) {
        image::RgbImage::from_pixel(w, h, image::Rgb(rgb))
            .save(path)
            .expect("write picture");
    }

    #[test]
    fn a_picture_is_scaled_cropped_cached_and_read_back_from_the_cache() {
        let d = dir("cache");
        let src = d.join("picture.png");
        picture(&src, 640, 480, [200, 30, 30]);
        let cache = d.join("cache");
        let a = load_thumb(&cache, "map:x", &src, HERO_SIZE, DEFAULT_CAP_BYTES).expect("thumb");
        let Thumb::Pixels { width, height, .. } = &a else {
            panic!("pixels")
        };
        assert_eq!((*width, *height), HERO_SIZE);
        let files: Vec<_> = std::fs::read_dir(&cache)
            .expect("cache")
            .flatten()
            .collect();
        assert_eq!(files.len(), 1, "one small PNG is cached");
        std::fs::remove_file(&src).expect("remove source");
        let b = load_thumb(&cache, "map:x", &src, HERO_SIZE, DEFAULT_CAP_BYTES);
        assert!(
            b.is_none(),
            "without the source the cache key cannot be built"
        );
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn the_cache_respects_its_size_cap_and_drops_the_oldest_first() {
        let d = dir("cap");
        let cache = d.join("cache");
        std::fs::create_dir_all(&cache).expect("cache");
        for i in 0..4 {
            let f = cache.join(format!("{i}.png"));
            std::fs::write(&f, vec![0_u8; 1000]).expect("file");
            let t = SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1000 + i);
            std::fs::File::options()
                .append(true)
                .open(&f)
                .and_then(|h| h.set_modified(t))
                .expect("time");
        }
        prune(&cache, 2500);
        let mut left: Vec<_> = std::fs::read_dir(&cache)
            .expect("cache")
            .flatten()
            .map(|e| e.file_name().to_string_lossy().to_string())
            .collect();
        left.sort();
        assert_eq!(left, vec!["2.png", "3.png"], "the two oldest went");
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn the_average_colour_of_a_flat_picture_is_that_colour() {
        let d = dir("avg");
        let src = d.join("t.png");
        picture(&src, 32, 32, [10, 120, 250]);
        assert_eq!(average_colour(&src), Some([10, 120, 250]));
        assert_eq!(average_colour(&d.join("none.png")), None);
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn vehicles_without_a_preview_image_have_none_and_maps_use_picture_jpg() {
        let d = dir("src");
        std::fs::create_dir_all(d.join("Vehicles/V")).expect("dir");
        std::fs::write(d.join("Vehicles/V/v.bus"), "x").expect("bus");
        assert_eq!(vehicle_preview(&d, "Vehicles/V/v.bus"), None);
        picture(&d.join("Vehicles/V/preview.png"), 4, 4, [1, 2, 3]);
        assert!(vehicle_preview(&d, "Vehicles/V/v.bus").is_some());
        std::fs::create_dir_all(d.join("maps/Grundorf")).expect("dir");
        let cat = GameCatalogue::default();
        assert_eq!(source_of("map:Grundorf", &cat, &d), None);
        picture(&d.join("maps/Grundorf/picture.jpg"), 4, 4, [1, 2, 3]);
        assert!(source_of("map:Grundorf", &cat, &d).is_some());
        let _ = std::fs::remove_dir_all(&d);
    }
}
