//! Refreshes the scan chip: the verdict is recomputed when its inputs change and every few seconds.

use bevy::prelude::*;

use crate::settings_screen::info;

use super::chip::ScanChip;
use super::chip_facts;
use super::labels::{level_key, look, reason_key};
use super::verdict::{assess, gather};
use super::{ChipParts, ReasonQuery, ScanInputs};

/// Recomputes the verdict when its inputs change (the facts arrived, a scan started or ended, the
/// language changed) and at least every few seconds (the age of the last scan moves), and updates
/// the chip. While the background facts are not there yet the chip says it is checking.
pub fn refresh_scan_chip(
    time: Res<Time>,
    chip: chip_facts::ChipFacts,
    fresh: Query<(), Added<ScanChip>>,
    mut since: Local<f32>,
    mut last: Local<Option<(Option<ScanInputs>, String)>>,
    mut parts: ChipParts,
    mut reasons: ReasonQuery,
) {
    let chip_facts::ChipFacts {
        loc,
        settings,
        service,
        facts,
    } = chip;
    *since += time.delta_secs();
    let rebuilt = !fresh.is_empty();
    let Some(settings) = settings else { return };
    let inputs = gather(&settings, service.as_deref(), &facts);
    let key = (inputs, loc.locale().to_owned());
    // `now` moves every call: compare the rest.
    let same = last.as_ref().is_some_and(|(i, l)| {
        l == &key.1
            && i.map(|i| ScanInputs { now: 0, ..i }) == inputs.map(|i| ScanInputs { now: 0, ..i })
    });
    if !rebuilt && same && *since < 3.0 {
        return;
    }
    *since = 0.0;
    if !rebuilt && same && inputs.is_none() {
        return;
    }
    let Some(inputs) = inputs else {
        // Not known yet: a quiet neutral chip, never a verdict we cannot back up.
        *last = Some(key);
        super::chip_parts::show_pending(&loc, &mut parts, &mut reasons);
        return;
    };
    *last = Some((Some(inputs), loc.locale().to_owned()));
    let status = assess(&inputs);
    let (bg, fg, glyph) = look(status.level);
    let when = inputs
        .last_scan
        .map_or_else(|| loc.t("scan-never"), info::local_time);
    let reason = reason_key(status.reason);
    for (mut b, mut tip) in &mut parts.0 {
        b.0 = bg;
        tip.0 = loc.t(reason).into();
    }
    for (mut t, mut f) in &mut parts.1 {
        t.0 = glyph.to_owned();
        f.0 = fg;
    }
    for (mut k, mut a, mut f) in &mut parts.2 {
        k.0 = "scan-chip";
        a.0 = vec![
            ("status".into(), loc.t(level_key(status.level)).into()),
            ("when".into(), when.clone().into()),
        ];
        f.0 = fg;
    }
    for (mut k, mut a) in &mut reasons {
        k.0 = reason;
        a.0 = vec![(
            "days".into(),
            inputs.interval_days.max(1).to_string().into(),
        )];
    }
}
