//! The HUD's alert line and camera label systems.

use super::*;

/// Shows the alert of the queue in the alert slot; the pill is hidden when there is none.
pub fn update_alert_slot(
    queue: Res<AlertQueue>,
    loc: Res<Localization>,
    mut pills: Query<&mut Visibility, With<AlertPill>>,
    mut texts: Query<(&HudText, &mut Text, &mut InstrumentInk)>,
) {
    let shown = queue.visible();
    for mut vis in &mut pills {
        let want = if shown.is_some() {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if *vis != want {
            *vis = want;
        }
    }
    let Some(alert) = shown else { return };
    let args: Vec<(String, Arg)> = alert
        .args
        .iter()
        .map(|(k, v)| ((*k).to_owned(), Arg::from(v.as_str())))
        .collect();
    let line = loc.t_args(
        keys::ALERT_LINE,
        &[
            (
                "word".to_owned(),
                Arg::from(loc.t(alert.severity.word_key())),
            ),
            ("text".to_owned(), Arg::from(loc.t_args(alert.key, &args))),
        ],
    );
    let ink = match alert.severity {
        AlertSeverity::Critical => InstrumentInk::Alarm,
        AlertSeverity::Warning => InstrumentInk::Warning,
        AlertSeverity::Info => InstrumentInk::Text,
    };
    for (hud, mut text, mut i) in &mut texts {
        if hud.binding == Binding::Alerts {
            set_text(&mut text, line.clone());
            if *i != ink {
                *i = ink;
            }
        }
    }
}

/// Announces a camera or mode change for 1.5 s. The first sample (the start of the drive) is no
/// change.
pub fn camera_label_changes(
    values: Res<HudValues>,
    mut label: ResMut<CameraLabel>,
    mut pills: Query<(&FadeKind, &mut HudFade)>,
) {
    let now = (values.camera, values.sub_camera);
    match label.last {
        None => label.last = Some(now),
        Some(last) if last != now => {
            label.last = Some(now);
            label.shown = Some((now.0, now.1, values.sub_total));
            for (kind, mut fade) in &mut pills {
                if *kind == FadeKind::CameraLabel {
                    fade.show();
                }
            }
        }
        Some(_) => {}
    }
}
