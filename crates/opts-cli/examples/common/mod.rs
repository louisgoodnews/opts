//! Helpers shared by the examples: a client from `OPTS_CLI_PORT` (default 44201) and `OPTS_CLI_TOKEN`.

use std::time::Duration;

use opts_cli::client::{Client, ClientConfig};
use opts_cli::protocol::DEFAULT_PORT;

/// A client for the control server named by the environment.
pub fn connect() -> Client {
    let port = std::env::var("OPTS_CLI_PORT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(DEFAULT_PORT);
    Client::new(ClientConfig {
        host: "127.0.0.1".into(),
        port,
        token: std::env::var("OPTS_CLI_TOKEN").ok(),
        timeout: Duration::from_secs(150),
    })
}
