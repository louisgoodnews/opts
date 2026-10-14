//! Remote control of a running application over a line-delimited JSON-RPC socket.
//!
//! See `docs/src/architecture/control-protocol.md` and ADR-0006. The socket runs on plain threads;
//! the main thread only drains a channel once per frame ([`channel::drain_control_commands`]), so
//! rendering never waits for I/O.

pub mod channel;
pub mod handlers;
pub mod keys;
pub mod params;
pub mod paths;
pub mod server;
pub mod state;

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::mpsc;
use std::sync::Mutex;

use bevy::input::InputSystems;
use bevy::prelude::*;

pub use paths::PathPolicy;
pub use server::{ServerConfig, ServerHandle};
pub use state::{ControlRegistry, Outcome};

/// Settings of the control server (from the command line).
#[derive(Resource, Debug, Clone)]
pub struct ControlConfig {
    /// Port to listen on; `None` keeps the server off, `Some(0)` picks a free port.
    pub port: Option<u16>,
    /// Address to bind; anything but loopback requires a token.
    pub bind: String,
    /// Shared secret for `auth`.
    pub token: Option<String>,
    /// Extra directories commands may read from and write to.
    pub allow_dirs: Vec<PathBuf>,
}

impl Default for ControlConfig {
    fn default() -> Self {
        Self {
            port: None,
            bind: "127.0.0.1".into(),
            token: None,
            allow_dirs: Vec::new(),
        }
    }
}

/// The address the server is bound to, once it runs.
#[derive(Resource, Debug, Clone, Copy)]
pub struct ControlAddr(pub SocketAddr);

/// Keeps the server alive for the lifetime of the app.
#[derive(Resource)]
#[allow(dead_code)]
struct ServerGuard(ServerHandle);

/// Adds the control server and its handlers. Without a port the plugin only registers the
/// handler table (tests drive it through [`channel::dispatch`]).
#[derive(Default)]
pub struct ControlPlugin {
    /// Server settings.
    pub config: ControlConfig,
}

impl ControlPlugin {
    /// A plugin with the given settings.
    #[must_use]
    pub fn new(config: ControlConfig) -> Self {
        Self { config }
    }
}

impl Plugin for ControlPlugin {
    fn build(&self, app: &mut App) {
        let mut registry = ControlRegistry::default();
        handlers::register_all(&mut registry);
        let mut policy = PathPolicy {
            read_roots: self.config.allow_dirs.clone(),
            write_roots: self.config.allow_dirs.clone(),
        };
        if let Some(dir) = crate::reporting::save::reports_dir() {
            let _ = std::fs::create_dir_all(&dir);
            policy.write_roots.push(dir);
        }
        crate::capture::install(app);
        app.insert_resource(registry)
            .insert_resource(policy)
            .insert_resource(self.config.clone())
            .init_resource::<state::ControlState>()
            .init_resource::<state::Deferred>();
        let Some(port) = self.config.port else { return };
        let (tx, rx) = mpsc::sync_channel(server::QUEUE_CAPACITY);
        let server_config = ServerConfig {
            bind: self.config.bind.clone(),
            port,
            token: self.config.token.clone(),
        };
        match server::start(&server_config, tx) {
            Ok(handle) => {
                tracing::info!(target: "opts_bevy::control", addr = %handle.addr, "control server listening addr={}", handle.addr);
                app.insert_resource(ControlAddr(handle.addr))
                    .insert_resource(ServerGuard(handle))
                    .insert_resource(state::ControlInbox(Mutex::new(rx)))
                    .add_systems(
                        PreUpdate,
                        channel::drain_control_commands.before(InputSystems),
                    );
            }
            Err(e) => {
                tracing::error!(target: "opts_bevy::control", "control server could not start: {e}")
            }
        }
    }
}
