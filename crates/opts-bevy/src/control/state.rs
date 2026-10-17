//! Shared types of the control server: handler registry, outcomes, deferred calls, per-run state.

use std::collections::HashMap;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use bevy::ecs::world::World;
use bevy::input::keyboard::{Key, KeyCode};
use bevy::input::mouse::MouseButton;
use bevy::prelude::Resource;
use opts_cli::protocol::{Request, Response, RpcError};
use serde_json::Value;

/// Result of a handler: either an immediate answer or a waiter that is polled every frame.
pub enum Outcome {
    /// Answered in the frame in which the command ran.
    Done(Result<Value, RpcError>),
    /// Answered later: polled once per frame until it returns `Some`.
    Wait(Waiter),
}

/// A pending answer; see [`Outcome::Wait`].
pub type Waiter = Box<dyn FnMut(&mut World) -> Option<Result<Value, RpcError>> + Send>;

impl Outcome {
    /// `Done(Ok(value))`.
    #[must_use]
    pub fn ok(value: Value) -> Self {
        Self::Done(Ok(value))
    }

    /// `Done(Err(error))`.
    #[must_use]
    pub fn err(error: RpcError) -> Self {
        Self::Done(Err(error))
    }
}

impl From<Result<Value, RpcError>> for Outcome {
    fn from(r: Result<Value, RpcError>) -> Self {
        Self::Done(r)
    }
}

/// A command implementation. Runs on the main thread with full world access.
pub type HandlerFn = fn(&mut World, &Value) -> Outcome;

/// Method name → handler. Plugins add their own methods (`opts-game` registers the game ones).
#[derive(Resource, Default)]
pub struct ControlRegistry {
    handlers: HashMap<&'static str, HandlerFn>,
}

impl ControlRegistry {
    /// Registers (or replaces) the handler of `method`.
    pub fn register(&mut self, method: &'static str, handler: HandlerFn) {
        self.handlers.insert(method, handler);
    }

    /// The handler of `method`, if any.
    #[must_use]
    pub fn get(&self, method: &str) -> Option<HandlerFn> {
        self.handlers.get(method).copied()
    }

    /// Every registered method name, sorted.
    #[must_use]
    pub fn methods(&self) -> Vec<&'static str> {
        let mut m: Vec<&'static str> = self.handlers.keys().copied().collect();
        m.sort_unstable();
        m
    }

    /// Whether `method` is implemented in this build.
    #[must_use]
    pub fn contains(&self, method: &str) -> bool {
        self.handlers.contains_key(method)
    }
}

/// Flags shared between a connection thread and the Bevy side for one request.
#[derive(Debug, Default)]
pub struct PendingFlags {
    /// Set when the app picked the request up.
    pub started: AtomicBool,
    /// Set by the connection thread when it gave up waiting (the request is then skipped).
    pub cancelled: AtomicBool,
}

/// One request travelling from a connection thread to the main thread.
pub struct Pending {
    /// The parsed request.
    pub request: Request,
    /// Where to send the answer.
    pub reply: Sender<Response>,
    /// Shared flags.
    pub flags: Arc<PendingFlags>,
}

/// The receiving end of the command channel.
#[derive(Resource)]
pub struct ControlInbox(pub Mutex<Receiver<Pending>>);

/// A command whose answer is not ready yet.
pub struct DeferredCall {
    /// Id to answer with.
    pub id: Value,
    /// Reply channel.
    pub reply: Sender<Response>,
    /// Polled each frame.
    pub waiter: Waiter,
    /// Gives up (error 1005) after this instant.
    pub deadline: Instant,
}

/// Commands waiting for a later frame.
#[derive(Resource, Default)]
pub struct Deferred(pub Mutex<Vec<DeferredCall>>);

/// An injected input that must be released later.
#[derive(Debug, Clone)]
pub struct Release {
    /// Keyboard key to release.
    pub key: Option<(KeyCode, Key)>,
    /// Mouse button to release.
    pub mouse: Option<MouseButton>,
    /// `Time<Real>` seconds at which to release.
    pub at: f64,
}

/// Mutable bookkeeping of the control server.
#[derive(Resource)]
pub struct ControlState {
    /// When the plugin was built (for `uptime_secs`).
    pub started: Instant,
    /// Frames drained so far.
    pub frame: u64,
    /// Smoothed frames per second.
    pub fps: f32,
    /// Frame durations of the last ten seconds as `(Time<Real>` seconds at the frame, milliseconds)`.
    pub frame_log: std::collections::VecDeque<(f64, f32)>,
    /// Injected inputs waiting for their release time.
    pub releases: Vec<Release>,
}

impl Default for ControlState {
    fn default() -> Self {
        Self {
            started: Instant::now(),
            frame: 0,
            fps: 0.0,
            frame_log: std::collections::VecDeque::new(),
            releases: Vec::new(),
        }
    }
}
