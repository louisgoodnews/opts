//! The parents of a placed object: the marker, the object it hangs on and the spline it follows.

use opts_bevy::scene_trace::ParentRef;

use super::*;

/// The parents of an object: the marker it belongs to, the object an attached object hangs on and
/// the spline an attachment follows.
pub(super) fn object_parents(
    site: &Site<'_>,
    tile: &MapTile,
    obj: &MapObjectInstance,
    key: (i32, i32),
) -> Vec<ParentRef> {
    let mut parents = Vec::new();
    if let Some(marker) = site.marker {
        parents.push(ParentRef {
            kind: "marker".into(),
            file: marker.to_owned(),
            tile: Some([key.0, key.1]),
            index: Some(site.index),
            id: None,
        });
    }
    if site.list == "attached objects" {
        if let Some(att) = tile.attached_objects.get(site.index) {
            let parent = att.parent_id;
            let file = tile
                .objects
                .iter()
                .find(|o| o.id_code == parent)
                .map(|o| o.sco_file.clone())
                .or_else(|| {
                    tile.attached_objects
                        .iter()
                        .find(|o| o.id_code == parent)
                        .map(|o| o.sco_file.clone())
                })
                .or_else(|| {
                    tile.spline_attachments
                        .iter()
                        .find(|o| o.id_code == parent)
                        .map(|o| o.sco_file.clone())
                })
                .unwrap_or_default();
            parents.push(ParentRef {
                kind: "attach_parent".into(),
                file,
                tile: Some([key.0, key.1]),
                index: None,
                id: None,
            });
        }
    }
    if site.list == "attachments" {
        if let Some(att) = tile
            .spline_attachments
            .iter()
            .find(|a| a.id_code == obj.id_code)
        {
            parents.push(ParentRef {
                kind: kind::SPLINE.into(),
                file: tile
                    .splines
                    .get(att.spline_index)
                    .map(|s| s.sli_file.clone())
                    .unwrap_or_default(),
                tile: Some([key.0, key.1]),
                index: Some(att.spline_index),
                id: None,
            });
        }
    }
    parents
}
