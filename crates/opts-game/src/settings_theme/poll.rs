//! Polling of the desktop's colour scheme while the *System* theme mode is active.

use std::sync::mpsc::{channel, Receiver, Sender};
use std::time::Duration;

use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use opts_gui::theme::{SystemColorScheme, ThemeKind, ThemeMode, ThemePreferences};

use super::desktop::os_color_scheme_hint;

/// Polls the desktop's colour scheme on a worker thread while the *System* theme mode is active
/// and the window itself reports nothing: at once when the mode is chosen, then once a minute.
pub struct OsSchemePoll {
    timer: Timer,
    tx: Sender<Option<ThemeKind>>,
    rx: std::sync::Mutex<Receiver<Option<ThemeKind>>>,
    in_flight: bool,
}

impl Default for OsSchemePoll {
    fn default() -> Self {
        let (tx, rx) = channel();
        let mut timer = Timer::new(Duration::from_secs(60), TimerMode::Repeating);
        timer.set_elapsed(Duration::from_secs(60)); // poll immediately
        Self {
            timer,
            tx,
            rx: std::sync::Mutex::new(rx),
            in_flight: false,
        }
    }
}

/// Feeds the polled desktop preference into [`SystemColorScheme`].
pub fn poll_os_color_scheme(
    time: Res<Time<Real>>,
    prefs: Res<ThemePreferences>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut poll: Local<OsSchemePoll>,
    mut system: ResMut<SystemColorScheme>,
) {
    let answers: Vec<_> = poll
        .rx
        .lock()
        .map(|rx| rx.try_iter().collect())
        .unwrap_or_default();
    for answer in answers {
        poll.in_flight = false;
        if let Some(kind) = answer {
            if system.0 != Some(kind) {
                tracing::info!(?kind, "the desktop prefers this colour scheme (gsettings)");
                system.0 = Some(kind);
            }
        }
    }
    let window_reports = windows.iter().any(|w| w.window_theme.is_some());
    if prefs.mode != ThemeMode::System || window_reports {
        return;
    }
    if poll.timer.tick(time.delta()).just_finished() && !poll.in_flight {
        poll.in_flight = true;
        let tx = poll.tx.clone();
        std::thread::spawn(move || {
            let _ = tx.send(os_color_scheme_hint());
        });
    }
}
