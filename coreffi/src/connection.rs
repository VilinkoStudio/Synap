//! FFI-compatible connection wrapper for P2P synchronization.

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::sync::{Arc, Mutex};

use crate::error::FfiError;
use crate::FfiUlid;
use synap_core::net::{Addr, Conn};

/// FFI-compatible connection wrapper.
#[derive(uniffi::Object)]
pub struct FfiConnection {
    inner: Arc<Mutex<Box<dyn Conn + Send>>>,
}

impl FfiConnection {
    /// Create a new FFI connection from a Conn trait object.
    pub fn new(conn: Box<dyn Conn + Send>) -> Self {
        Self {
            inner: Arc::new(Mutex::new(conn)),
        }
    }
}

#[uniffi::export]
impl FfiConnection {
    /// Create a TCP connection to the specified host and port.
    ///
    /// # Arguments
    /// * `host` - The hostname or IP address
    /// * `port` - The port number
    ///
    /// # Returns
    /// A new FFI connection
    ///
    /// # Example
    /// ```text
    /// let conn = FfiConnection::connect_tcp("127.0.0.1", 8080)?;
    /// ```
    #[uniffi::constructor]
    pub fn connect_tcp(host: String, port: u16) -> Result<Arc<Self>, FfiError> {
        let addr_str = format!("{}:{}", host, port);
        let addr: SocketAddr = addr_str.parse().map_err(|_| FfiError::Io)?;

        let stream = TcpStream::connect(addr).map_err(|_| FfiError::Io)?;

        let tcp_conn = Box::new(TcpConn::from_stream(stream, host, port)?);
        Ok(Arc::new(Self::new(tcp_conn)))
    }

    /// Write data to the connection.
    ///
    /// # Arguments
    /// * `data` - The data to write
    ///
    /// # Returns
    /// The number of bytes written
    pub fn write(&self, data: Vec<u8>) -> Result<u32, FfiError> {
        let mut conn = self.inner.lock().map_err(|_| FfiError::Io)?;

        let bytes_written = conn.write(&data).map_err(|e| {
            FfiError::Io
        })?;

        Ok(bytes_written as u32)
    }

    /// Read data from the connection.
    ///
    /// # Arguments
    /// * `len` - Maximum number of bytes to read
    ///
    /// # Returns
    /// The data read
    pub fn read(&self, len: u32) -> Result<Vec<u8>, FfiError> {
        let mut conn = self.inner.lock().map_err(|_| FfiError::Io)?;

        let mut buffer = vec![0u8; len as usize];
        let bytes_read = conn.read(&mut buffer).map_err(|e| {
            FfiError::Io
        })?;

        buffer.truncate(bytes_read);
        Ok(buffer)
    }

    /// Close the connection.
    pub fn close(&self) -> Result<(), FfiError> {
        let mut conn = self.inner.lock().map_err(|_| FfiError::Io)?;

        conn.close().map_err(|e| {
            FfiError::Io
        })?;

        Ok(())
    }

    /// Get the local address as a string.
    pub fn local_addr(&self) -> Result<String, FfiError> {
        let conn = self.inner.lock().map_err(|_| FfiError::Io)?;

        Ok(conn.local_addr().to_string())
    }

    /// Get the remote address as a string.
    pub fn remote_addr(&self) -> Result<String, FfiError> {
        let conn = self.inner.lock().map_err(|_| FfiError::Io)?;

        Ok(conn.remote_addr().to_string())
    }
}

/// TCP connection implementation.
struct TcpConn {
    stream: TcpStream,
    local: SocketAddr,
    remote: SocketAddr,
    _host: String,
    _port: u16,
}

impl TcpConn {
    fn from_stream(stream: TcpStream, host: String, port: u16) -> Result<Self, std::io::Error> {
        let local = stream.local_addr()?;
        let remote = stream.peer_addr()?;
        stream.set_nodelay(true)?;
        Ok(Self {
            stream,
            local,
            remote,
            _host: host,
            _port: port,
        })
    }
}

impl Read for TcpConn {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.stream.read(buf)
    }
}

impl Write for TcpConn {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.stream.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.stream.flush()
    }
}

impl Conn for TcpConn {
    fn local_addr(&self) -> Addr {
        Addr::Tcp(self.local)
    }

    fn remote_addr(&self) -> Addr {
        Addr::Tcp(self.remote)
    }

    fn close(&mut self) -> std::io::Result<()> {
        self.stream.shutdown(std::net::Shutdown::Both)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tcp_connection_localhost() {
        // This test requires a server running, so we just test the construction logic
        let result = FfiConnection::connect_tcp("127.0.0.1".to_string(), 9999);
        // It should fail (nothing listening), but with an Io error, not a panic
        assert!(result.is_err());
        match result {
            Err(FfiError::Io) => {}
            _ => panic!("Expected Io error"),
        }
    }

    #[test]
    fn test_invalid_address() {
        let result = FfiConnection::connect_tcp("invalid hostname".to_string(), 8080);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_port() {
        let result = FfiConnection::connect_tcp("127.0.0.1".to_string(), 99999);
        // Port 99999 is invalid (above u16 range in practice)
        // The parse should succeed but connection should fail
        assert!(result.is_err());
    }
}
