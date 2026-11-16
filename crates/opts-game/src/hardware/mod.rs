//! The hardware profile in the game (M-0081, `docs/src/architecture/hardware-probe.md`).
//!
//! The probe itself is the crate `opts-hardware`; this module runs it off the main thread,
//! hands it what only the renderer knows (adapter, limits, monitors, gamepads), keeps the result
//! in [`HardwareState`] and offers it to the settings page and the control protocol. Nothing
//! leaves the machine.

pub mod protocol;
mod settings;
mod systems;
pub mod view;

use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver};
use std::sync::Mutex;

use bevy::prelude::*;
use opts_hardware::inputs::ProbeInputs;
use opts_hardware::{store, HardwareProfile};

pub use settings::descriptors;
use systems::{collect_renderer_inputs, finish, start};

/// Key of the setting that allows the profile in reports.
pub const ALLOW_REPORTS_KEY: &str = "advanced.hardware_allow_reports";

/// Frames the renderer data is awaited (monitors appear after the window system started).
const RENDERER_WAIT_FRAMES: u32 = 600;

/// The profile and the machinery around it.
#[derive(Resource, Default)]
pub struct HardwareState {
    /// The last finished probe.
    pub profile: Option<HardwareProfile>,
    /// Folder of the optional local profile file.
    pub dir: Option<PathBuf>,
    /// What the renderer told us so far.
    pub inputs: ProbeInputs,
    /// A probe is running.
    pending: Option<Mutex<Receiver<HardwareProfile>>>,
    /// Save the finished probe to the local file.
    save_when_done: bool,
    /// The running probe was asked for by the user (toast and page refresh when done).
    notify: bool,
    renderer_applied: bool,
    frames: u32,
}

impl HardwareState {
    /// A state that stores its file in `dir`.
    #[must_use]
    pub fn with_dir(dir: Option<PathBuf>) -> Self {
        Self {
            dir,
            ..Self::default()
        }
    }

    /// Is a probe running?
    #[must_use]
    pub fn is_probing(&self) -> bool {
        self.pending.is_some()
    }

    /// Does the local profile file exist?
    #[must_use]
    pub fn is_stored(&self) -> bool {
        self.dir.as_deref().is_some_and(|d| store::path(d).exists())
    }

    /// Starts a probe on a worker thread (the whole probe takes a few milliseconds, but the
    /// disk and device queries may block). `save` writes the result to the local file.
    pub fn start_probe(&mut self, save: bool, notify: bool) {
        if self.pending.is_some() {
            self.save_when_done |= save;
            self.notify |= notify;
            return;
        }
        let (tx, rx) = channel();
        let inputs = self.inputs.clone();
        let dir = self.dir.clone();
        self.save_when_done = save;
        self.notify = notify;
        self.pending = Some(Mutex::new(rx));
        std::thread::spawn(move || {
            let cached = dir.as_deref().and_then(store::load);
            let fresh = opts_hardware::probe_with(&inputs);
            let _ = tx.send(store::with_cached_bench(fresh, cached.as_ref()));
        });
    }

    /// Deletes the local file; `true` when one was removed. The profile in memory stays.
    pub fn delete_stored(&mut self) -> bool {
        self.dir.as_deref().is_some_and(store::delete)
    }

    /// Takes a finished probe, if any; returns whether the user is waiting for it.
    fn poll(&mut self) -> Option<bool> {
        let rx = self.pending.as_ref()?;
        let done = match rx.lock().ok()?.try_recv() {
            Ok(p) => Some(Some(p)),
            Err(std::sync::mpsc::TryRecvError::Empty) => None,
            Err(std::sync::mpsc::TryRecvError::Disconnected) => Some(None),
        }?;
        self.pending = None;
        let notify = std::mem::take(&mut self.notify);
        if let Some(p) = done {
            if std::mem::take(&mut self.save_when_done) {
                if let Some(dir) = &self.dir {
                    if let Err(e) = store::save(dir, &p) {
                        tracing::warn!(err = %e, "the hardware profile could not be stored");
                    }
                }
            }
            tracing::info!(probe_ms = p.probe_ms, "hardware profile detected");
            self.profile = Some(p);
        }
        Some(notify)
    }
}

/// The folder of the local profile file (the data folder of OPTS).
#[must_use]
pub fn data_dir() -> Option<PathBuf> {
    directories::ProjectDirs::from("org", "opts", "opts").map(|d| d.data_local_dir().to_path_buf())
}

/// Adds the state, the probe systems, the settings and the control method.
pub struct HardwarePlugin;

impl Plugin for HardwarePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<HardwareState>()
            .add_systems(Startup, start)
            .add_systems(Update, (collect_renderer_inputs, finish).chain());
        protocol::register(app);
    }
}

#[cfg(test)]
mod tests;
