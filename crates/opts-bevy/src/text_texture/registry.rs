//! The font registry: lookup by name and the cache of rendered text images.

use super::*;

/// Font lookup by name plus a cache of rendered text images.
#[derive(Default)]
pub struct FontRegistry {
    /// Lower-case font name → (`.oft` path, font).
    fonts: Option<HashMap<String, (PathBuf, Arc<OftFont>)>>,
    bitmaps: HashMap<String, Option<Arc<FontBitmaps>>>,
    rendered: HashMap<TextSpec, Handle<Image>>,
    reported: HashSet<String>,
}

impl FontRegistry {
    fn index(&mut self, install: &OmsiInstall) -> &HashMap<String, (PathBuf, Arc<OftFont>)> {
        self.fonts.get_or_insert_with(|| {
            let mut files = Vec::new();
            if let Some(dir) = install.find_file("Fonts") {
                collect_oft(&dir, &mut files);
            }
            files.sort();
            let mut map = HashMap::new();
            for path in files {
                let parsed = std::fs::read(&path)
                    .map_err(|e| e.to_string())
                    .and_then(|b| OftFile::parse(&b));
                match parsed {
                    // Which of two same-named fonts wins is open (Q-FN01): first in path order.
                    Ok(f) => {
                        for font in f.fonts {
                            map.entry(font.name.to_lowercase())
                                .or_insert_with(|| (path.clone(), Arc::new(font)));
                        }
                    }
                    Err(e) => tracing::error!(at = %crate::position_tag::position_tag(), font = %path.display(), "font parse failed: {e}"),
                }
            }
            tracing::info!(fonts = map.len(), "font registry built");
            map
        })
    }

    fn report_once(&mut self, key: String, message: &str) {
        if self.reported.insert(key.clone()) {
            tracing::error!(at = %crate::position_tag::position_tag(), font = %key, "{message}");
        }
    }

    /// Rasterises `spec` to RGBA8 pixels (`width × height`). `None` when the font or its bitmaps
    /// cannot be loaded; the error is logged once per font.
    pub fn render_data(&mut self, install: &OmsiInstall, spec: &TextSpec) -> Option<Vec<u8>> {
        let key = spec.font.to_lowercase();
        let Some((path, font)) = self.index(install).get(&key).cloned() else {
            self.report_once(key, "font not found in Fonts/");
            return None;
        };
        if !self.bitmaps.contains_key(&key) {
            let dir = path.parent().unwrap_or(Path::new("")).to_path_buf();
            let loaded = match load_bitmaps(install, &dir, &font) {
                Ok(b) => Some(Arc::new(b)),
                Err(e) => {
                    self.report_once(key.clone(), &format!("font bitmaps unusable: {e}"));
                    None
                }
            };
            self.bitmaps.insert(key.clone(), loaded);
        }
        let bmp = self.bitmaps.get(&key)?.clone()?;
        Some(render_rgba(&font, &bmp, spec))
    }

    /// Renders `spec` (or returns the cached image). `None` when the font or its bitmaps cannot
    /// be loaded; the error is logged once per font.
    pub fn render(
        &mut self,
        install: &OmsiInstall,
        images: &mut Assets<Image>,
        spec: &TextSpec,
    ) -> Option<Handle<Image>> {
        if let Some(h) = self.rendered.get(spec) {
            return Some(h.clone());
        }
        let data = self.render_data(install, spec)?;
        let handle = images.add(text_image(spec.width, spec.height, data));
        self.rendered.insert(spec.clone(), handle.clone());
        Some(handle)
    }
}
