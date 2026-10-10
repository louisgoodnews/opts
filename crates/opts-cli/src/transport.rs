//! Blocking TCP transport: one JSON message per line.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::time::Duration;

use crate::protocol::MAX_LINE_BYTES;

/// Failures of the byte-level connection.
#[derive(Debug)]
pub enum TransportError {
    /// The address could not be resolved or connected to.
    Connect(String),
    /// A read or write failed.
    Io(std::io::Error),
    /// The server closed the connection.
    Closed,
    /// A line exceeded [`MAX_LINE_BYTES`].
    LineTooLong,
}

impl std::fmt::Display for TransportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Connect(m) => write!(f, "cannot connect: {m}"),
            Self::Io(e) => write!(f, "i/o error: {e}"),
            Self::Closed => write!(f, "connection closed by the server"),
            Self::LineTooLong => write!(f, "response line exceeds {MAX_LINE_BYTES} bytes"),
        }
    }
}

impl std::error::Error for TransportError {}

impl From<std::io::Error> for TransportError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

/// An open connection to a control server.
pub struct Connection {
    writer: TcpStream,
    reader: BufReader<TcpStream>,
}

impl Connection {
    /// Connects to `host:port`, trying every resolved address.
    ///
    /// # Errors
    /// [`TransportError::Connect`] when no address accepts the connection within `timeout`.
    pub fn connect(host: &str, port: u16, timeout: Duration) -> Result<Self, TransportError> {
        let addrs: Vec<SocketAddr> = (host, port)
            .to_socket_addrs()
            .map_err(|e| TransportError::Connect(format!("{host}:{port}: {e}")))?
            .collect();
        let mut last = format!("{host}:{port}: no address");
        for addr in addrs {
            match TcpStream::connect_timeout(&addr, timeout) {
                Ok(stream) => {
                    stream.set_read_timeout(Some(timeout))?;
                    stream.set_write_timeout(Some(timeout))?;
                    stream.set_nodelay(true)?;
                    let reader = BufReader::new(stream.try_clone()?);
                    return Ok(Self {
                        writer: stream,
                        reader,
                    });
                }
                Err(e) => last = format!("{addr}: {e}"),
            }
        }
        Err(TransportError::Connect(last))
    }

    /// Changes the time a read may block (used for long commands such as `sim.step`).
    ///
    /// # Errors
    /// Fails when the socket option cannot be set.
    pub fn set_read_timeout(&self, timeout: Duration) -> Result<(), TransportError> {
        self.writer.set_read_timeout(Some(timeout))?;
        Ok(())
    }

    /// Sends one line (the newline is appended).
    ///
    /// # Errors
    /// [`TransportError::Io`] when the write fails.
    pub fn send_line(&mut self, line: &str) -> Result<(), TransportError> {
        self.writer.write_all(line.as_bytes())?;
        self.writer.write_all(b"\n")?;
        self.writer.flush()?;
        Ok(())
    }

    /// Reads one line without its terminator.
    ///
    /// # Errors
    /// [`TransportError::Closed`] at end of stream, [`TransportError::LineTooLong`] beyond the cap.
    pub fn recv_line(&mut self) -> Result<String, TransportError> {
        let mut buf = String::new();
        let n = (&mut self.reader)
            .take(MAX_LINE_BYTES as u64 + 1)
            .read_line(&mut buf)?;
        if n == 0 {
            return Err(TransportError::Closed);
        }
        if !buf.ends_with('\n') && n > MAX_LINE_BYTES {
            return Err(TransportError::LineTooLong);
        }
        while buf.ends_with(['\n', '\r']) {
            buf.pop();
        }
        Ok(buf)
    }
}
