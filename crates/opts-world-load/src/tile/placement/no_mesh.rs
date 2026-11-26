//! The accounting of a placed object whose meshes gave no parts: a sound source without geometry, or
//! an object counted as no geometry, hidden, protected or failed.

use super::*;

/// An object whose meshes gave no parts: a sound source without geometry becomes a root with its
/// voices; any other is counted as having no geometry, being protected or having failed.
pub(in crate::tile) fn no_mesh_parts(
    (job, stats, commands, meshes, materials): (
        &mut WorldLoadJob,
        &mut WorldStats,
        &mut Commands,
        &mut Assets<Mesh>,
        &mut Assets<StandardMaterial>,
    ),
    (obj, sco, sco_path, sco_dir, key): (
        &MapObjectInstance,
        &opts_core::formats::scenery::SceneryObject,
        &Path,
        &Path,
        (i32, i32),
    ),
    (transform, mesh_parts, template, children): ObjParts<'_>,
    (hidden_before, protected_before): (usize, usize),
) {
    if sco.sound.is_some() && (sco.meshes.is_empty() || sco.is_editor_only()) {
        // A sound source without geometry (the `Generic` ambient objects: `[onlyeditor]`
        // mesh, a script that raises the triggers): the root carries the script and the
        // voices.
        let name = sco
            .friendly_name
            .clone()
            .unwrap_or_else(|| obj.sco_file.clone());
        let parent = commands.spawn((transform, Name::new(name))).id();
        children.push(parent);
        job.finish_object(
            commands,
            (&mut *meshes, &mut *materials),
            parent,
            (sco, obj),
            mesh_parts,
            template,
        );
        job.sound()
            .attach_scenery(job, commands, parent, sco, sco_dir);
        stats.objects_loaded += 1;
        return;
    }
    if sco.meshes.is_empty() || sco.is_editor_only() {
        tracing::debug!(sco = %sco_path.display(), "object has no mesh (helper/path object)");
        stats.objects_no_geometry += 1;
    } else if job.cache.stats.hidden_parts > hidden_before {
        // Every slot is scaled by a script variable (`[alphascale]`, e.g. a night overlay):
        // nothing to draw, which is not a failure (P-1014, MI-0029).
        tracing::debug!(sco = %sco_path.display(), "object has only variable-hidden slots");
        stats.objects_no_geometry += 1;
    } else if job.cache.stats.protected_meshes > protected_before {
        stats.objects_protected += 1;
        tracing::warn!(sco = %sco_path.display(), tile = ?key, pos = ?obj.position, "object skipped: no mesh has usable geometry");
    } else {
        tracing::error!(at = %opts_bevy::position_tag(), sco = %sco_path.display(), "object has meshes but none could be loaded");
        stats.objects_failed += 1;
    }
}
