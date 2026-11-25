//! Background saves: the settings are handed to one writer thread, which merges the newest copy
//! per file and writes it; the caller never waits for SQLite (M-0088).

use opts_game_state::settings::GameSettings;

use super::save;

/// Hands a copy of the settings to the single background writer: the caller never waits for
/// SQLite (M-0088). Writes made in quick succession are merged: only the newest copy per
/// settings file is written. [`flush_background_saves`] waits for the writer (shutdown).
pub fn save_in_background(settings: &GameSettings) {
    let saver = saver();
    let Ok(mut st) = saver.state.lock() else {
        return;
    };
    // Newer copies replace older ones for the same file only.
    match st
        .latest
        .iter_mut()
        .find(|s| s.db_path_override == settings.db_path_override)
    {
        Some(slot) => *slot = settings.clone(),
        None => st.latest.push(settings.clone()),
    }
    if st.running {
        return;
    }
    st.running = true;
    drop(st);
    let worker = saver.clone();
    let spawned = std::thread::Builder::new()
        .name("settings-writer".into())
        .spawn(move || loop {
            let next = {
                let Ok(mut st) = worker.state.lock() else {
                    return;
                };
                let next = std::mem::take(&mut st.latest);
                if next.is_empty() {
                    st.running = false;
                    worker.idle.notify_all();
                }
                next
            };
            if next.is_empty() {
                return;
            }
            for settings in next {
                if let Err(err) = save(&settings) {
                    tracing::warn!(err = ?err, "could not save settings");
                }
            }
        });
    if let Err(err) = spawned {
        tracing::warn!(err = %err, "cannot start the settings writer; saving here");
        if let Ok(mut st) = saver.state.lock() {
            st.running = false;
            st.latest.clear();
        }
        let _ = save(settings);
    }
}

/// Waits until the background writer has written everything handed to it.
pub fn flush_background_saves() {
    let saver = saver();
    let Ok(mut st) = saver.state.lock() else {
        return;
    };
    while st.running {
        st = match saver.idle.wait(st) {
            Ok(g) => g,
            Err(_) => return,
        };
    }
}

/// The state shared between the callers of [`save_in_background`] and the writer.
struct Saver {
    state: std::sync::Mutex<SaverState>,
    idle: std::sync::Condvar,
}

#[derive(Default)]
struct SaverState {
    latest: Vec<GameSettings>,
    running: bool,
}

fn saver() -> std::sync::Arc<Saver> {
    static SAVER: std::sync::OnceLock<std::sync::Arc<Saver>> = std::sync::OnceLock::new();
    SAVER
        .get_or_init(|| {
            std::sync::Arc::new(Saver {
                state: std::sync::Mutex::new(SaverState::default()),
                idle: std::sync::Condvar::new(),
            })
        })
        .clone()
}
