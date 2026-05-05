use std::{
    collections::BTreeSet,
    net::{IpAddr, SocketAddr, TcpListener, TcpStream, ToSocketAddrs, UdpSocket},
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};

use crossbeam_channel::{Receiver, Sender};

use crate::{NetError, TcpChannel};

const SOCKET_CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const SOCKET_READ_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Debug, Clone, Default)]
pub struct ListenConfig {
    pub port: Option<u16>,
}

#[derive(Debug, Clone)]
pub struct ConnectConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListenerState {
    pub protocol: String,
    pub backend: String,
    pub is_listening: bool,
    pub listen_port: Option<u16>,
    pub local_addresses: Vec<String>,
    pub status: String,
    pub error_message: Option<String>,
}

impl Default for ListenerState {
    fn default() -> Self {
        Self {
            protocol: "TCP".to_string(),
            backend: "Rust std::net".to_string(),
            is_listening: false,
            listen_port: None,
            local_addresses: Vec::new(),
            status: "未启动".to_string(),
            error_message: None,
        }
    }
}

#[derive(Debug)]
pub struct IncomingConnection {
    pub channel: TcpChannel,
    pub remote_addr: Option<SocketAddr>,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct TcpNetRuntime;

impl TcpNetRuntime {
    pub fn connect(&self, config: ConnectConfig) -> Result<TcpChannel, NetError> {
        let address = (config.host.as_str(), config.port)
            .to_socket_addrs()?
            .next()
            .ok_or_else(|| {
                NetError::Io(std::io::Error::new(
                    std::io::ErrorKind::AddrNotAvailable,
                    format!("unable to resolve {}:{}", config.host, config.port),
                ))
            })?;
        let stream = TcpStream::connect_timeout(&address, SOCKET_CONNECT_TIMEOUT)?;
        Ok(TcpChannel::new(configure_stream(stream)?))
    }

    pub fn listen(&self, config: ListenConfig) -> Result<TcpListenerRuntime, NetError> {
        TcpListenerRuntime::start(config)
    }
}

pub struct TcpListenerRuntime {
    state: Arc<Mutex<ListenerState>>,
    incoming_rx: Receiver<Result<IncomingConnection, NetError>>,
    shutdown_tx: Sender<()>,
    join_handle: Option<thread::JoinHandle<()>>,
}

pub struct IncomingLoopHandle {
    listener: TcpListenerRuntime,
    join_handle: Option<thread::JoinHandle<()>>,
}

impl TcpListenerRuntime {
    fn start(config: ListenConfig) -> Result<Self, NetError> {
        let listener = TcpListener::bind(("0.0.0.0", config.port.unwrap_or(0)))?;
        listener.set_nonblocking(true)?;
        let listen_port = listener.local_addr()?.port();

        let state = Arc::new(Mutex::new(ListenerState {
            protocol: "TCP".to_string(),
            backend: "Rust std::net".to_string(),
            is_listening: true,
            listen_port: Some(listen_port),
            local_addresses: current_local_addresses(),
            status: "已监听".to_string(),
            error_message: None,
        }));

        let (incoming_tx, incoming_rx) = crossbeam_channel::unbounded();
        let (shutdown_tx, shutdown_rx) = crossbeam_channel::bounded(1);
        let state_for_thread = Arc::clone(&state);

        let join_handle = thread::spawn(move || {
            accept_loop(listener, incoming_tx, shutdown_rx, state_for_thread);
        });

        Ok(Self {
            state,
            incoming_rx,
            shutdown_tx,
            join_handle: Some(join_handle),
        })
    }

    pub fn state(&self) -> ListenerState {
        self.state.lock().unwrap().clone()
    }

    pub fn incoming(&self) -> Receiver<Result<IncomingConnection, NetError>> {
        self.incoming_rx.clone()
    }

    pub fn stop(&mut self) -> Result<(), NetError> {
        self.shutdown_tx
            .send(())
            .map_err(|_| NetError::ListenerStopped)?;
        if let Some(join_handle) = self.join_handle.take() {
            let _ = join_handle.join();
        }
        Ok(())
    }
}

impl Drop for TcpListenerRuntime {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

impl IncomingLoopHandle {
    pub fn state(&self) -> ListenerState {
        self.listener.state()
    }

    pub fn stop(&mut self) -> Result<(), NetError> {
        self.listener.stop()?;
        if let Some(join_handle) = self.join_handle.take() {
            let _ = join_handle.join();
        }
        Ok(())
    }
}

impl Drop for IncomingLoopHandle {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

pub fn spawn_incoming_loop<F>(listener: TcpListenerRuntime, on_incoming: F) -> IncomingLoopHandle
where
    F: Fn(Result<IncomingConnection, NetError>) + Send + Sync + 'static,
{
    let callback = Arc::new(on_incoming);
    let receiver = listener.incoming();
    let join_handle = thread::spawn({
        let callback = Arc::clone(&callback);
        move || {
            for incoming in receiver {
                callback(incoming);
            }
        }
    });

    IncomingLoopHandle {
        listener,
        join_handle: Some(join_handle),
    }
}

fn accept_loop(
    listener: TcpListener,
    incoming_tx: Sender<Result<IncomingConnection, NetError>>,
    shutdown_rx: Receiver<()>,
    state: Arc<Mutex<ListenerState>>,
) {
    loop {
        if shutdown_rx.try_recv().is_ok() {
            break;
        }

        match listener.accept() {
            Ok((stream, remote_addr)) => {
                let result = configure_stream(stream).map(|stream| IncomingConnection {
                    channel: TcpChannel::new(stream),
                    remote_addr: Some(remote_addr),
                });
                let _ = incoming_tx.send(result.map_err(NetError::from));
            }
            Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(100));
            }
            Err(err) => {
                let message = err.to_string();
                {
                    let mut guard = state.lock().unwrap();
                    guard.is_listening = false;
                    guard.status = "监听失败".to_string();
                    guard.error_message = Some(message.clone());
                }
                let _ = incoming_tx.send(Err(NetError::Io(err)));
                break;
            }
        }
    }

    let mut guard = state.lock().unwrap();
    guard.is_listening = false;
    if guard.error_message.is_none() {
        guard.status = "已停止".to_string();
    }
}

fn configure_stream(stream: TcpStream) -> std::io::Result<TcpStream> {
    stream.set_nodelay(true)?;
    stream.set_read_timeout(Some(SOCKET_READ_TIMEOUT))?;
    stream.set_write_timeout(Some(SOCKET_READ_TIMEOUT))?;
    Ok(stream)
}

fn current_local_addresses() -> Vec<String> {
    current_local_ip_addrs()
        .into_iter()
        .map(|ip| ip.to_string())
        .collect()
}

pub(crate) fn current_local_ip_addrs() -> Vec<IpAddr> {
    let mut addresses = BTreeSet::new();

    if let Some(ip) = discover_local_ip_v4().map(IpAddr::V4) {
        addresses.insert(ip);
    }

    if let Some(ip) = discover_local_ip_v6().map(IpAddr::V6) {
        addresses.insert(ip);
    }

    addresses.into_iter().collect()
}

fn discover_local_ip_v4() -> Option<std::net::Ipv4Addr> {
    let socket = UdpSocket::bind("0.0.0.0:0").ok()?;
    socket.connect("192.0.2.1:9").ok()?;
    let ip = match socket.local_addr().ok()?.ip() {
        IpAddr::V4(ip) => ip,
        IpAddr::V6(_) => return None,
    };
    if ip.is_loopback() || ip.is_unspecified() {
        return None;
    }
    Some(ip)
}

fn discover_local_ip_v6() -> Option<std::net::Ipv6Addr> {
    let socket = UdpSocket::bind("[::]:0").ok()?;
    socket.connect("[2001:db8::1]:9").ok()?;
    let ip = match socket.local_addr().ok()?.ip() {
        IpAddr::V6(ip) => ip,
        IpAddr::V4(_) => return None,
    };
    if ip.is_loopback() || ip.is_unspecified() {
        return None;
    }
    Some(ip)
}
