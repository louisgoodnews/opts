//! Probing and filling thumbnail, hero, and livery-swatch UI slots in the wizard.

use bevy::ecs::system::SystemParam;

use super::*;

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

/// Livery swatches take the average colour of the repaint's first readable texture.
fn fill_swatches(
    service: &mut ThumbnailService,
    liveries: &LiveryState,
    swatches: &Query<(Entity, &LiverySwatch), Without<ThumbDone>>,
    commands: &mut Commands,
) {
    for (entity, sw) in swatches {
        let id = format!("sw:{}:{}", sw.vehicle, sw.livery);
        if service.lookup(&id).is_none() {
            let src = (liveries.vehicle == sw.vehicle)
                .then_some(liveries.repaints.as_ref())
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

/// The slots still to fill and the glyphs they may carry.
#[derive(SystemParam)]
pub struct ThumbSlots<'w, 's> {
    /// Row thumbnails.
    pub rows: Query<'w, 's, (Entity, &'static RowThumb), Without<ThumbDone>>,
    /// Hero images.
    pub heroes: Query<'w, 's, (Entity, &'static HeroImage), Without<ThumbDone>>,
    /// Livery swatches.
    pub swatches: Query<'w, 's, (Entity, &'static LiverySwatch), Without<ThumbDone>>,
    /// The children of a slot.
    pub children: Query<'w, 's, &'static Children>,
    /// The placeholder glyphs.
    pub glyphs: Query<'w, 's, (), Or<(With<HeroGlyph>, With<ThumbGlyph>)>>,
}

/// What the slots are filled from.
#[derive(SystemParam)]
pub struct ThumbSources<'w> {
    /// The maps and vehicles read.
    pub catalogue: Res<'w, GameCatalogue>,
    /// The livery state.
    pub liveries: Res<'w, LiveryState>,
    /// The catalogue service (it knows the OMSI root).
    pub service_root: Option<Res<'w, crate::catalogue_service::CatalogueService>>,
}

/// Finishes decodes and fills the slots of lists, details and livery swatches.
pub fn fill_thumbnails(
    mut service: ResMut<ThumbnailService>,
    mut images: ResMut<Assets<Image>>,
    sources: ThumbSources,
    pending: ThumbSlots,
    mut commands: Commands,
    mut known_root: Local<Option<PathBuf>>,
    mut wait: Local<u32>,
) {
    let ThumbSources {
        catalogue,
        liveries,
        service_root,
    } = sources;
    let ThumbSlots {
        rows,
        heroes,
        swatches,
        children,
        glyphs,
    } = pending;
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
    fill_swatches(&mut service, &liveries, &swatches, &mut commands);
}
