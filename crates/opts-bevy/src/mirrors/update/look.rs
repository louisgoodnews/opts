//! Mirror look: the main camera's HDR, fog and tone mapping copied to the mirror cameras.

use bevy::prelude::*;

use super::*;

type MainCameraLookQuery<'w, 's> = Query<
    'w,
    's,
    (
        Has<Hdr>,
        Option<&'static DistanceFog>,
        Option<&'static Tonemapping>,
    ),
    (
        With<MainCamera>,
        Without<MirrorCamera>,
        Without<AmbientCamera>,
    ),
>;

type MirrorCameraLookItem<'a> = (
    Entity,
    Has<Hdr>,
    Option<&'a DistanceFog>,
    Option<&'a Tonemapping>,
    Option<&'a MirrorCamera>,
);

type MirrorCameraLookFilter = Or<(With<MirrorCamera>, With<AmbientCamera>)>;

/// Copies `Hdr` and `DistanceFog` from the main camera to the mirror cameras (the mirror picture
/// is scene-linear: no tonemapper, no colour grading; the main camera applies its own once). A
/// mirror that streams into a HUD panel takes the main camera's tone mapping too.
pub fn sync_mirror_camera_look(
    main: MainCameraLookQuery,
    mirrors: Query<MirrorCameraLookItem, MirrorCameraLookFilter>,
    mut commands: Commands,
) {
    let Some((hdr, fog, tonemapping)) = main.iter().next() else {
        return;
    };
    for (entity, has_hdr, has_fog, has_tonemapping, mirror) in &mirrors {
        let mut e = commands.entity(entity);
        let want = if mirror.is_some_and(|m| m.enlarged) {
            tonemapping.copied()
        } else {
            Some(Tonemapping::None)
        };
        if let Some(t) = want {
            if has_tonemapping != Some(&t) {
                e.insert(t);
            }
        }
        if hdr && !has_hdr {
            e.insert(Hdr);
        } else if !hdr && has_hdr {
            e.remove::<Hdr>();
        }
        match (fog, has_fog) {
            (Some(f), None) => {
                e.insert(f.clone());
            }
            (Some(f), Some(cur)) if format!("{f:?}") != format!("{cur:?}") => {
                e.insert(f.clone());
            }
            (None, Some(_)) => {
                e.remove::<DistanceFog>();
            }
            _ => {}
        }
    }
}
