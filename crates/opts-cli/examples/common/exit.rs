//! Ends an example after a failed call: the message and the client's exit code.

/// Prints a failed call and exits with the client's exit code.
pub fn die(context: &str, error: &opts_cli::client::ClientError) -> ! {
    eprintln!("{context}: {error}");
    std::process::exit(i32::from(error.exit_code()));
}
