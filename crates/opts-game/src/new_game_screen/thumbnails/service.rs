//! The thumbnail service: requests, the in-memory table of their state and the poll that finishes them.

use super::*;

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
