//! Small macros shared by the client modules.

/// Writes a line to stderr, ignoring a closed pipe (`eprintln!` would panic).
#[cfg(feature = "client")]
macro_rules! errln {
    ($($arg:tt)*) => {{
        use std::io::Write as _;
        let _ = writeln!(std::io::stderr(), $($arg)*);
    }};
}
