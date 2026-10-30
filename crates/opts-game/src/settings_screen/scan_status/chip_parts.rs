//! The chip's query bundles and its neutral "checking" look.

use super::*;

pub(super) type ReasonQuery<'w, 's> = Query<
    'w,
    's,
    (&'static mut L10nKey, &'static mut L10nArgs),
    (With<ScanChipReason>, Without<ScanChipLabel>),
>;

pub(super) type ChipParts<'w, 's> = (
    Query<
        'w,
        's,
        (&'static mut BackgroundRole, &'static mut Tooltip),
        (
            With<ScanChip>,
            Without<ScanChipIcon>,
            Without<ScanChipLabel>,
        ),
    >,
    Query<
        'w,
        's,
        (&'static mut Text, &'static mut ForegroundRole),
        (With<ScanChipIcon>, Without<ScanChipLabel>),
    >,
    Query<
        'w,
        's,
        (
            &'static mut L10nKey,
            &'static mut L10nArgs,
            &'static mut ForegroundRole,
        ),
        (With<ScanChipLabel>, Without<ScanChipIcon>),
    >,
);

/// The quiet neutral chip shown while the background facts are not there yet.
pub(super) fn show_pending(loc: &Localization, parts: &mut ChipParts, reasons: &mut ReasonQuery) {
    for (mut b, mut tip) in &mut parts.0 {
        b.0 = ColorRole::SurfaceVariant;
        tip.0 = loc.t("scan-why-pending").into();
    }
    for (mut t, mut f) in &mut parts.1 {
        t.0 = icons::REFRESH.to_owned();
        f.0 = ColorRole::OnSurfaceVariant;
    }
    for (mut k, mut a, mut f) in &mut parts.2 {
        k.0 = "scan-chip-pending";
        a.0.clear();
        f.0 = ColorRole::OnSurfaceVariant;
    }
    for (mut k, mut a) in &mut *reasons {
        k.0 = "scan-why-pending";
        a.0.clear();
    }
}
