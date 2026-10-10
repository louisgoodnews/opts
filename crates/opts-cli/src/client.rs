//! High-level control client: request ids, authentication, reconnection and batches.

use std::time::Duration;

use serde_json::{json, Value};

use crate::protocol::{parse_response_line, Request, RpcError};
use crate::transport::{Connection, TransportError};

/// Why a call failed.
#[derive(Debug)]
pub enum ClientError {
    /// No connection could be made or kept (exit code 2).
    Connection(String),
    /// The server's reply was not valid protocol (exit code 3).
    Protocol(String),
    /// The server answered with an error object (exit code 1).
    Rpc(RpcError),
}

impl std::fmt::Display for ClientError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Connection(m) | Self::Protocol(m) => write!(f, "{m}"),
            Self::Rpc(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for ClientError {}

impl ClientError {
    /// The process exit code the CLI uses for this error.
    #[must_use]
    pub fn exit_code(&self) -> u8 {
        match self {
            Self::Rpc(_) => 1,
            Self::Connection(_) => 2,
            Self::Protocol(_) => 3,
        }
    }
}

/// Connection settings.
#[derive(Debug, Clone)]
pub struct ClientConfig {
    /// Host name or address of the control server.
    pub host: String,
    /// TCP port.
    pub port: u16,
    /// Token for `auth`, when the server requires one.
    pub token: Option<String>,
    /// Connect and read timeout.
    pub timeout: Duration,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".into(),
            port: crate::protocol::DEFAULT_PORT,
            token: None,
            timeout: Duration::from_secs(10),
        }
    }
}

/// A control client. Connects lazily and reconnects once when the connection broke.
pub struct Client {
    config: ClientConfig,
    conn: Option<Connection>,
    next_id: u64,
    was_connected: bool,
}

impl Client {
    /// Creates a client; nothing is connected yet.
    #[must_use]
    pub fn new(config: ClientConfig) -> Self {
        Self {
            config,
            conn: None,
            next_id: 1,
            was_connected: false,
        }
    }

    fn connect(&mut self) -> Result<(), ClientError> {
        let conn = Connection::connect(&self.config.host, self.config.port, self.config.timeout)
            .map_err(|e| ClientError::Connection(e.to_string()))?;
        self.conn = Some(conn);
        self.was_connected = true;
        if let Some(token) = self.config.token.clone() {
            self.exchange("auth", json!({ "token": token }))?;
        }
        Ok(())
    }

    fn id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    fn raw_exchange(&mut self, line: &str) -> Result<String, TransportError> {
        let conn = self.conn.as_mut().ok_or(TransportError::Closed)?;
        conn.send_line(line)?;
        conn.recv_line()
    }

    fn exchange(&mut self, method: &str, params: Value) -> Result<Value, ClientError> {
        let id = self.id();
        let line = serde_json::to_string(&Request::new(id, method, params))
            .map_err(|e| ClientError::Protocol(e.to_string()))?;
        let reply = self
            .raw_exchange(&line)
            .map_err(|e| ClientError::Connection(e.to_string()))?;
        let response =
            parse_response_line(&reply).map_err(|e| ClientError::Protocol(e.to_string()))?;
        if response.id != json!(id) {
            return Err(ClientError::Protocol(format!(
                "reply id {} does not match request id {id}",
                response.id
            )));
        }
        response.into_result().map_err(ClientError::Rpc)
    }

    /// Calls `method` and returns its result.
    ///
    /// If the connection was open before and fails, one reconnect is attempted (the server may have
    /// been restarted).
    ///
    /// # Errors
    /// See [`ClientError`].
    pub fn call(&mut self, method: &str, params: Value) -> Result<Value, ClientError> {
        if self.conn.is_none() {
            self.connect()?;
        }
        match self.exchange(method, params.clone()) {
            Err(ClientError::Connection(_)) if self.was_connected => {
                self.conn = None;
                self.connect()?;
                self.exchange(method, params)
            }
            other => other,
        }
    }

    /// Sets the read timeout for the next long-running calls (`sim.step`, `scene.load_map`).
    pub fn set_read_timeout(&mut self, timeout: Duration) {
        self.config.timeout = timeout;
        if let Some(c) = &self.conn {
            let _ = c.set_read_timeout(timeout);
        }
    }

    /// Sends several commands in one round trip (`batch`) and returns one result per command.
    ///
    /// # Errors
    /// Fails as a whole only when the batch call itself fails; per-command failures are in the vector.
    pub fn batch(
        &mut self,
        commands: &[(String, Value)],
        stop_on_error: bool,
    ) -> Result<Vec<Result<Value, RpcError>>, ClientError> {
        let list: Vec<Value> = commands
            .iter()
            .map(|(m, p)| json!({ "method": m, "params": p }))
            .collect();
        let result = self.call(
            "batch",
            json!({ "commands": list, "stop_on_error": stop_on_error }),
        )?;
        let items = result
            .get("results")
            .and_then(Value::as_array)
            .ok_or_else(|| ClientError::Protocol("batch reply has no `results` array".into()))?;
        items
            .iter()
            .map(|item| {
                if let Some(e) = item.get("error") {
                    serde_json::from_value::<RpcError>(e.clone())
                        .map(Err)
                        .map_err(|e| ClientError::Protocol(e.to_string()))
                } else {
                    Ok(Ok(item.get("result").cloned().unwrap_or(Value::Null)))
                }
            })
            .collect()
    }
}
