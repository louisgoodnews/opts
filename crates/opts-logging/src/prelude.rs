//! Convenient re-exports for the `opts-logging` crate.

pub use crate::config::{
    AnsiMode, ConsoleConfig, ConsoleVerbosity, FilesConfig, LogConfig, LogFormat, RetentionPolicy,
    RotationPolicy,
};
pub use crate::error::{LogError, LogResult};
pub use crate::guard::LogGuard;
pub use crate::init::{init, init_default, init_from_env, set_global_level, set_level};
pub use crate::panic::install_panic_hook;
