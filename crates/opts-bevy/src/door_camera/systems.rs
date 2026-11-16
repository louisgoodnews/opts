//! Systems for detecting open doors, driving the door camera slots, and syncing look from the main camera.

use super::*;

mod detect;
mod slots;

pub use detect::*;
pub use slots::*;

/// What the picture copies from the main camera: `Hdr` and fog.
type MainLook<'a> = (Has<Hdr>, Option<&'a DistanceFog>, Option<&'a Tonemapping>);
/// The main camera, not a door camera.
type MainFilter = (With<MainCamera>, Without<DoorCameraMarker>);
/// The same look on a door camera, with its entity.
type DoorLook<'a> = (
    Entity,
    Has<Hdr>,
    Option<&'a DistanceFog>,
    Option<&'a Tonemapping>,
);

/// `Hdr`, fog and the tone mapping follow the main camera. The picture is shown as it is on the
/// HUD (nothing tone-maps a UI image), so it carries the main camera's display transform:
/// without it the picture was dark and harsh next to the world.
pub fn sync_look(
    main: Query<MainLook, MainFilter>,
    doors: Query<DoorLook, With<DoorCameraMarker>>,
    mut commands: Commands,
) {
    let Some((hdr, fog, tonemapping)) = main.iter().next() else {
        return;
    };
    for (entity, has_hdr, has_fog, has_tonemapping) in &doors {
        let mut e = commands.entity(entity);
        if let Some(t) = tonemapping {
            if has_tonemapping != Some(t) {
                e.insert(*t);
            }
        }
        if hdr != has_hdr {
            if hdr {
                e.insert(Hdr);
            } else {
                e.remove::<Hdr>();
            }
        }
        match (fog, has_fog) {
            (Some(f), None) => {
                e.insert(f.clone());
            }
            (None, Some(_)) => {
                e.remove::<DistanceFog>();
            }
            _ => {}
        }
    }
}
