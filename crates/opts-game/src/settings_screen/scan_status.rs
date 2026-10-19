//! The scan status chip of *Settings → Content*: is a manual scan recommended?
//!
//! The decision is the pure function [`assess`]; everything else gathers its inputs, shows the
//! result (icon + text + reason, never colour alone) and refreshes it every few seconds.
//!
//! Rules (documented in `docs/src/roadmap/phase-9/M-0086.md`), first match wins:
//!
//! | Level | Reason | When |
//! | :--- | :--- | :--- |
//! | red | folder missing | no OMSI folder is set or it cannot be found |
//! | red | failed | the last scan failed |
//! | red | never scanned | the catalogue has no completed scan |
//! | red | empty | the last scan found no entries |
//! | yellow | running | a scan is running now |
//! | yellow | folder changed | the OMSI folder or one of its top-level folders was modified after the last scan |
//! | yellow | older | the last scan is older than the automatic scan interval |
//! | green | recent | none of the above |

use bevy::prelude::*;
use opts_gui::interaction::tooltip::Tooltip;
use opts_gui::l10n::{L10nArgs, L10nKey, Localization};
use opts_gui::prelude::*;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::catalogue_service::{CatalogueService, DiscoveryStatus};
pub use crate::content_facts::RootState;
use crate::content_facts::{ContentFactsStore, WantsContentFacts};
use crate::settings::GameSettings;
use crate::shell::icons;

/// How urgent a scan is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScanLevel {
    /// Green: nothing to do.
    Good,
    /// Yellow: a scan is recommended.
    Recommended,
    /// Red: the catalogue cannot be trusted or does not exist.
    Needed,
}

/// Why the level is what it is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScanReason {
    /// No OMSI folder, or it is gone.
    FolderMissing,
    /// The last scan failed.
    Failed,
    /// No completed scan yet.
    NeverScanned,
    /// The last scan found nothing.
    Empty,
    /// A scan is running.
    Running,
    /// The OMSI folder changed after the last scan.
    FolderChanged,
    /// The last scan is older than the automatic interval.
    Older,
    /// Scanned recently, nothing changed.
    Recent,
}

/// The verdict.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScanStatus {
    /// Colour and urgency.
    pub level: ScanLevel,
    /// The reason shown to the user.
    pub reason: ScanReason,
}

/// Everything [`assess`] looks at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScanInputs {
    /// Unix time now.
    pub now: i64,
    /// Unix time of the last completed scan.
    pub last_scan: Option<i64>,
    /// Entries the catalogue holds.
    pub entities: i64,
    /// The automatic scan interval in days (the setting).
    pub interval_days: u32,
    /// The OMSI folder.
    pub root: RootState,
    /// Modification times in the OMSI folder are newer than the last scan.
    pub folder_changed: bool,
    /// The last scan failed.
    pub failed: bool,
    /// A scan is running.
    pub running: bool,
}

/// The rules in the table above.
#[must_use]
pub fn assess(i: &ScanInputs) -> ScanStatus {
    use ScanLevel::{Good, Needed, Recommended};
    use ScanReason as R;
    let (level, reason) = if i.root != RootState::Present {
        (Needed, R::FolderMissing)
    } else if i.failed {
        (Needed, R::Failed)
    } else if i.last_scan.is_none() {
        (
            Needed,
            if i.running {
                R::Running
            } else {
                R::NeverScanned
            },
        )
    } else if i.entities <= 0 {
        (Needed, R::Empty)
    } else if i.running {
        (Recommended, R::Running)
    } else if i.folder_changed {
        (Recommended, R::FolderChanged)
    } else if i
        .last_scan
        .is_some_and(|t| i.now - t > i64::from(i.interval_days.max(1)) * 86_400)
    {
        (Recommended, R::Older)
    } else {
        (Good, R::Recent)
    };
    ScanStatus { level, reason }
}

/// The cheap change check: the OMSI folder or one of its direct sub-folders (`Vehicles`, `maps`,
/// `Sceneryobjects`, ...) has a modification time after `last_scan`. Adding or removing a map or
/// vehicle changes the folder that holds it; no file below them is read.
#[must_use]
pub fn folder_changed_since(root: &Path, last_scan: i64) -> bool {
    let newer = |p: &Path| {
        std::fs::metadata(p)
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .is_some_and(|d| i64::try_from(d.as_secs()).unwrap_or(i64::MAX) > last_scan)
    };
    if newer(root) {
        return true;
    }
    std::fs::read_dir(root)
        .into_iter()
        .flatten()
        .flatten()
        .any(|e| e.path().is_dir() && newer(&e.path()))
}

fn now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_secs()).unwrap_or(0))
}

/// Builds the inputs from the background facts and what is cheap to read here (the settings and
/// the service's status, which is a mutex around an enum). `None` while the facts are not there
/// yet: the chip then shows its "Checking..." state.
fn gather(
    settings: &GameSettings,
    service: Option<&CatalogueService>,
    store: &ContentFactsStore,
) -> Option<ScanInputs> {
    let light = store.light()?;
    let status = service.map(CatalogueService::status);
    Some(ScanInputs {
        now: now_secs(),
        last_scan: light.last_scan,
        entities: light.entities,
        interval_days: settings.discovery_interval_days,
        root: light.root,
        folder_changed: light.folder_changed,
        failed: matches!(status, Some(DiscoveryStatus::Failed(_))),
        running: matches!(status, Some(DiscoveryStatus::Running(_))),
    })
}

/// Colours and icon of a level: the status container roles, with the icon and the word beside
/// them (colour is never the only signal).
#[must_use]
pub fn look(level: ScanLevel) -> (ColorRole, ColorRole, &'static str) {
    match level {
        ScanLevel::Good => (
            ColorRole::SuccessContainer,
            ColorRole::OnSuccessContainer,
            icons::CHECK_CIRCLE,
        ),
        ScanLevel::Recommended => (
            ColorRole::WarningContainer,
            ColorRole::OnWarningContainer,
            icons::WARNING,
        ),
        ScanLevel::Needed => (
            ColorRole::ErrorContainer,
            ColorRole::OnErrorContainer,
            icons::ERROR,
        ),
    }
}

/// Message key of the level word.
#[must_use]
pub fn level_key(level: ScanLevel) -> &'static str {
    match level {
        ScanLevel::Good => "scan-level-good",
        ScanLevel::Recommended => "scan-level-recommended",
        ScanLevel::Needed => "scan-level-needed",
    }
}

/// Message key of the reason and what to do.
#[must_use]
pub fn reason_key(reason: ScanReason) -> &'static str {
    match reason {
        ScanReason::FolderMissing => "scan-why-folder-missing",
        ScanReason::Failed => "scan-why-failed",
        ScanReason::NeverScanned => "scan-why-never",
        ScanReason::Empty => "scan-why-empty",
        ScanReason::Running => "scan-why-running",
        ScanReason::FolderChanged => "scan-why-changed",
        ScanReason::Older => "scan-why-older",
        ScanReason::Recent => "scan-why-recent",
    }
}

/// The chip (frame).
#[derive(Component, Debug, Clone, Copy)]
pub struct ScanChip;
/// The icon inside the chip.
#[derive(Component, Debug, Clone, Copy)]
pub struct ScanChipIcon;
/// The text inside the chip (level word and time of the last scan).
#[derive(Component, Debug, Clone, Copy)]
pub struct ScanChipLabel;
/// The caption below the chip: the reason and what to do.
#[derive(Component, Debug, Clone, Copy)]
pub struct ScanChipReason;

/// Spawns the chip and its caption below the *Scan now* row; [`refresh_scan_chip`] fills it.
pub fn spawn_chip(c: &mut Commands, pane: Entity) {
    let (bg, fg, glyph) = look(ScanLevel::Needed);
    let wrap = c
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(4.0),
                align_items: AlignItems::FlexStart,
                padding: UiRect::axes(Val::Px(4.0), Val::Px(8.0)),
                ..default()
            },
            ChildOf(pane),
        ))
        .id();
    let chip = c
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: Val::Px(8.0),
                padding: UiRect::axes(Val::Px(12.0), Val::Px(6.0)),
                ..default()
            },
            BackgroundRole(bg),
            RadiusToken(RadiusLevel::Full),
            Tooltip::new(""),
            ScanChip,
            WantsContentFacts,
            UiId::new("settings.content.scan-status"),
            ChildOf(wrap),
        ))
        .id();
    c.spawn((
        Text::new(glyph),
        TextFont {
            font: FontSource::Handle(opts_gui::fonts::ICON_FONT),
            font_size: FontSize::Px(18.0),
            ..default()
        },
        TextColor::default(),
        ForegroundRole(fg),
        opts_gui::ui_tree::A11yHidden,
        ScanChipIcon,
        ChildOf(chip),
    ));
    c.spawn((
        Text::new(""),
        L10nKey("scan-chip"),
        L10nArgs::default(),
        TextColor::default(),
        TypeStyleToken(TypeScale::BodyMedium),
        ForegroundRole(fg),
        ScanChipLabel,
        ChildOf(chip),
    ));
    c.spawn((
        Text::new(""),
        L10nKey("scan-why-never"),
        L10nArgs::default(),
        TextColor::default(),
        TypeStyleToken(TypeScale::BodyMedium),
        ForegroundRole(ColorRole::OnSurfaceVariant),
        ScanChipReason,
        ChildOf(wrap),
    ));
}

type ReasonQuery<'w, 's> = Query<
    'w,
    's,
    (&'static mut L10nKey, &'static mut L10nArgs),
    (With<ScanChipReason>, Without<ScanChipLabel>),
>;

type ChipParts<'w, 's> = (
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

/// Recomputes the verdict when its inputs change (the facts arrived, a scan started or ended, the
/// language changed) and at least every few seconds (the age of the last scan moves), and updates
/// the chip. While the background facts are not there yet the chip says it is checking.
#[allow(clippy::too_many_arguments)]
pub fn refresh_scan_chip(
    time: Res<Time>,
    loc: Res<Localization>,
    settings: Option<Res<GameSettings>>,
    service: Option<Res<CatalogueService>>,
    facts: Res<ContentFactsStore>,
    fresh: Query<(), Added<ScanChip>>,
    mut since: Local<f32>,
    mut last: Local<Option<(Option<ScanInputs>, String)>>,
    mut parts: ChipParts,
    mut reasons: ReasonQuery,
) {
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
        for (mut k, mut a) in &mut reasons {
            k.0 = "scan-why-pending";
            a.0.clear();
        }
        return;
    };
    *last = Some((Some(inputs), loc.locale().to_owned()));
    let status = assess(&inputs);
    let (bg, fg, glyph) = look(status.level);
    let when = inputs
        .last_scan
        .map_or_else(|| loc.t("scan-never"), super::info::local_time);
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

#[cfg(test)]
mod tests;
