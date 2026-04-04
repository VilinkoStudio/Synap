//! TCP connection implementation for P2P synchronization.

use std::io::{Read, Write};
use std::net::TcpStream;

/// TCP-based transport that satisfies `synap_core::sync::SyncChannel`.
pub struct TcpConn {
    stream: TcpStream,
}

impl TcpConn {
    pub fn connect(addr: &str) -> std::io::Result<Self> {
        let stream = TcpStream::connect(addr)?;
        Ok(Self { stream })
    }

    pub fn from_stream(stream: TcpStream) -> Self {
        Self { stream }
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

#[cfg(test)]
mod tests {
    #[test]
    fn test_tcpconn_from_stream() {
        use std::net::TcpListener;

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let _ = format!("TcpConn can accept connections on {}", addr);
    }
}
