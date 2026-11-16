//! Probing and filling thumbnail, hero, and livery-swatch UI slots in the wizard.

use bevy::ecs::system::SystemParam;

use super::*;

mod probe;
pub use probe::{probe_of, source_of, vehicle_preview, Probe};

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
    pub glyphs: Query<'w, 's, (), GlyphFilter>,
}

/// Either placeholder glyph.
type GlyphFilter = Or<(With<HeroGlyph>, With<ThumbGlyph>)>;

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
