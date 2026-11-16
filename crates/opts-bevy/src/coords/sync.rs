//! Keeping the OMSI and Bevy transforms in step: the sync systems and their filters.

use super::*;


/// One-way synchronization system updating Bevy [`Transform`] when [`OmsiTransform`] changes.
pub fn sync_omsi_to_bevy_transforms(
    mut query: Query<(&OmsiTransform, &mut Transform), Changed<OmsiTransform>>,
) {
    for (omsi_tx, mut bevy_tx) in &mut query {
        let target = omsi_tx.to_bevy_transform();
        if !bevy_tx
            .translation
            .abs_diff_eq(target.translation, SYNC_EPSILON)
            || !bevy_tx.rotation.abs_diff_eq(target.rotation, SYNC_EPSILON)
            || !bevy_tx.scale.abs_diff_eq(target.scale, SYNC_EPSILON)
        {
            *bevy_tx = target;
        }
    }
}

/// One-way synchronization system back-propagating Bevy [`Transform`] changes to [`OmsiTransform`].
pub fn sync_bevy_to_omsi_transforms(
    mut query: Query<(&Transform, &mut OmsiTransform), Changed<Transform>>,
) {
    for (bevy_tx, mut omsi_tx) in &mut query {
        let target = OmsiTransform::from_bevy_transform(bevy_tx);
        if !omsi_tx
            .translation
            .abs_diff_eq(target.translation, SYNC_EPSILON)
            || !omsi_tx
                .rotation_deg
                .abs_diff_eq(target.rotation_deg, DEG_EPSILON)
            || !omsi_tx.scale.abs_diff_eq(target.scale, SYNC_EPSILON)
        {
            *omsi_tx = target;
        }
    }
}

/// Query filter matching entities where either [`OmsiTransform`] or [`Transform`] changed.
pub type SyncTransformFilter = Or<(Changed<OmsiTransform>, Changed<Transform>)>;

/// Query data tuple accessing mutable [`OmsiTransform`] and [`Transform`] components.
pub type SyncTransformData = (Mut<'static, OmsiTransform>, Mut<'static, Transform>);

/// Unified bidirectional synchronization system syncing OMSI and Bevy transforms.
///
/// Prevents race conditions and feedback loops by checking which component was mutated.
/// When both components are mutated concurrently (e.g. initial spawn), [`OmsiTransform`]
/// is treated as authoritative.
pub fn sync_coordinate_transforms(mut query: Query<SyncTransformData, SyncTransformFilter>) {
    for (mut omsi_tx, mut bevy_tx) in &mut query {
        let omsi_changed = omsi_tx.is_changed();
        let bevy_changed = bevy_tx.is_changed();

        if omsi_changed && !bevy_changed {
            let target = omsi_tx.to_bevy_transform();
            if !bevy_tx
                .translation
                .abs_diff_eq(target.translation, SYNC_EPSILON)
                || !bevy_tx.rotation.abs_diff_eq(target.rotation, SYNC_EPSILON)
                || !bevy_tx.scale.abs_diff_eq(target.scale, SYNC_EPSILON)
            {
                *bevy_tx = target;
            }
        } else if bevy_changed && !omsi_changed {
            let target = OmsiTransform::from_bevy_transform(&bevy_tx);
            if !omsi_tx
                .translation
                .abs_diff_eq(target.translation, SYNC_EPSILON)
                || !omsi_tx
                    .rotation_deg
                    .abs_diff_eq(target.rotation_deg, DEG_EPSILON)
                || !omsi_tx.scale.abs_diff_eq(target.scale, SYNC_EPSILON)
            {
                *omsi_tx = target;
            }
        } else if omsi_changed && bevy_changed {
            let target = omsi_tx.to_bevy_transform();
            if !bevy_tx
                .translation
                .abs_diff_eq(target.translation, SYNC_EPSILON)
                || !bevy_tx.rotation.abs_diff_eq(target.rotation, SYNC_EPSILON)
                || !bevy_tx.scale.abs_diff_eq(target.scale, SYNC_EPSILON)
            {
                *bevy_tx = target;
            }
        }
    }
}
