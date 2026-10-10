//! Panic hook capturing unhandled panics and writing crash dumps to `panics.log`.

use std::backtrace::Backtrace;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

static PANIC_HOOK_INSTALLED: AtomicBool = AtomicBool::new(false);

/// Installs the OPTS crash reporting panic hook.
///
/// When an unhandled panic occurs in any thread:
/// 1. Formats a comprehensive crash record including timestamp, thread name, panic location,
///    message, and captured backtrace.
/// 2. Atomically appends the crash report to `panics.log` in the designated `log_dir`.
/// 3. Flushes the log file to disk.
/// 4. Invokes the standard panic hook to preserve standard abort/stack unwinding semantics.
pub fn install_panic_hook(log_dir: &Path) {
    if PANIC_HOOK_INSTALLED.swap(true, Ordering::SeqCst) {
        return;
    }

    let panic_file_path: PathBuf = log_dir.join("panics.log");
    let prev_hook = std::panic::take_hook();

    std::panic::set_hook(Box::new(move |panic_info| {
        let timestamp = time::OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap_or_else(|_| "UNKNOWN_TIME".to_string());

        let thread = std::thread::current();
        let thread_name = thread.name().unwrap_or("unnamed");

        let location = panic_info
            .location()
            .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()))
            .unwrap_or_else(|| "unknown location".to_string());

        let message = if let Some(s) = panic_info.payload().downcast_ref::<&str>() {
            (*s).to_string()
        } else if let Some(s) = panic_info.payload().downcast_ref::<String>() {
            s.clone()
        } else {
            "Box<dyn Any> (unknown panic payload)".to_string()
        };

        let backtrace = Backtrace::capture();

        let report = format!(
            "\n============================== OPTS FATAL PANIC ==============================\n\
             Timestamp:   {timestamp}\n\
             Thread:      {thread_name}\n\
             Location:    {location}\n\
             Message:     {message}\n\
             \n\
             Backtrace:\n\
             {backtrace}\n\
             ==============================================================================\n"
        );

        // Attempt to create parent dirs if missing
        if let Some(parent) = panic_file_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        // Write directly to panics.log with synchronous flush
        if let Ok(mut file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&panic_file_path)
        {
            let _ = file.write_all(report.as_bytes());
            let _ = file.flush();
        }

        // Delegate to original hook
        prev_hook(panic_info);
    }));
}
