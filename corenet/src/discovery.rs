use std::{
    collections::BTreeMap,
    net::IpAddr,
    sync::{Arc, Mutex},
    thread,
    time::{SystemTime, UNIX_EPOCH},
};

use mdns_sd::{Receiver, ResolvedService, ServiceDaemon, ServiceEvent, ServiceInfo};

use crate::runtime::current_local_ip_addrs;
use crate::DiscoveryError;

const SERVICE_TYPE: &str = "_synap._tcp.local.";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveredPeer {
    pub service_name: String,
    pub display_name: String,
    pub host: String,
    pub port: u16,
    pub last_seen_at_ms: u64,
    pub signing_public_key: [u8; 32],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveryState {
    pub service_type: String,
    pub advertised_name: Option<String>,
    pub listen_port: Option<u16>,
    pub is_running: bool,
    pub error_message: Option<String>,
}

impl Default for DiscoveryState {
    fn default() -> Self {
        Self {
            service_type: SERVICE_TYPE.to_string(),
            advertised_name: None,
            listen_port: None,
            is_running: false,
            error_message: None,
        }
    }
}

/// Callback that returns `(signing_public_key, signature)`.
pub type MdnsSignCallback =
    Box<dyn Fn() -> Result<([u8; 32], [u8; 64]), String> + Send + Sync>;

pub struct DiscoveryConfig {
    pub display_name: String,
    pub listen_port: u16,
    /// Signing callback. If `None`, the service will be registered without a
    /// signature and discovered peers without valid signatures will be accepted.
    pub sign: Option<Arc<MdnsSignCallback>>,
}

impl std::fmt::Debug for DiscoveryConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DiscoveryConfig")
            .field("display_name", &self.display_name)
            .field("listen_port", &self.listen_port)
            .field("sign", &self.sign.is_some())
            .finish()
    }
}

impl Clone for DiscoveryConfig {
    fn clone(&self) -> Self {
        Self {
            display_name: self.display_name.clone(),
            listen_port: self.listen_port,
            sign: self.sign.clone(),
        }
    }
}

pub struct SyncDiscoveryRuntime {
    daemon: ServiceDaemon,
    state: Arc<Mutex<DiscoveryState>>,
    peers: Arc<Mutex<BTreeMap<String, DiscoveredPeer>>>,
    browse_thread: Option<thread::JoinHandle<()>>,
    service_fullname: String,
}

impl SyncDiscoveryRuntime {
    pub fn start(config: DiscoveryConfig) -> Result<Self, DiscoveryError> {
        let daemon = ServiceDaemon::new()?;
        let state = Arc::new(Mutex::new(DiscoveryState {
            service_type: SERVICE_TYPE.to_string(),
            advertised_name: Some(config.display_name.clone()),
            listen_port: Some(config.listen_port),
            is_running: true,
            error_message: None,
        }));
        let peers = Arc::new(Mutex::new(BTreeMap::new()));

        let host_name = build_host_name();
        let addresses = current_local_ip_addrs();

        let sign = config.sign.clone();
        let mut properties: Vec<(&str, &str)> = vec![
            ("device_name", config.display_name.as_str()),
            ("protocol", "tcp"),
            ("version", "1"),
        ];

        // If a signing callback is provided, generate a signature and add it
        // to the TXT record.  Signing failures are logged but do not prevent
        // the service from starting.
        let sign_result: Option<([u8; 32], [u8; 64])> =
            if let Some(sign_fn) = &sign {
                match sign_fn() {
                    Ok(signed) => Some(signed),
                    Err(err) => {
                        eprintln!(
                            "[corenet] mDNS signing failed, registering without signature: {err}"
                        );
                        None
                    }
                }
            } else {
                None
            };

        // Hex strings must outlive the properties borrow.
        let hex_key;
        let hex_sig;
        if let Some((ref pub_key, ref sig)) = sign_result {
            hex_key = hex::encode(pub_key);
            hex_sig = hex::encode(sig);
            properties.push(("key", &hex_key));
            properties.push(("sig", &hex_sig));
        }

        let service_info = ServiceInfo::new(
            SERVICE_TYPE,
            &config.display_name,
            &host_name,
            addresses.as_slice(),
            config.listen_port,
            &properties[..],
        )?;
        let service_fullname = service_info.get_fullname().to_string();
        daemon.register(service_info)?;

        let browse_receiver = daemon.browse(SERVICE_TYPE)?;
        let require_signature = sign.is_some();
        let browse_thread = Some(spawn_browse_thread(
            browse_receiver,
            Arc::clone(&state),
            Arc::clone(&peers),
            config.display_name,
            require_signature,
        ));

        Ok(Self {
            daemon,
            state,
            peers,
            browse_thread,
            service_fullname,
        })
    }

    pub fn state(&self) -> DiscoveryState {
        self.state.lock().unwrap().clone()
    }

    pub fn peers(&self) -> Vec<DiscoveredPeer> {
        self.peers.lock().unwrap().values().cloned().collect()
    }

    pub fn stop(&mut self) -> Result<(), DiscoveryError> {
        {
            let mut state = self.state.lock().unwrap();
            state.is_running = false;
        }

        let _ = self.daemon.stop_browse(SERVICE_TYPE);
        let _ = self.daemon.unregister(&self.service_fullname);
        self.daemon.shutdown()?;

        if let Some(join_handle) = self.browse_thread.take() {
            let _ = join_handle.join();
        }

        Ok(())
    }
}

impl Drop for SyncDiscoveryRuntime {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

fn spawn_browse_thread(
    receiver: Receiver<ServiceEvent>,
    state: Arc<Mutex<DiscoveryState>>,
    peers: Arc<Mutex<BTreeMap<String, DiscoveredPeer>>>,
    local_display_name: String,
    require_signature: bool,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        while let Ok(event) = receiver.recv() {
            match event {
                ServiceEvent::ServiceResolved(service) => {
                    if let Some(peer) =
                        resolved_to_peer(&service, &local_display_name, require_signature)
                    {
                        peers
                            .lock()
                            .unwrap()
                            .insert(peer.service_name.clone(), peer);
                    }
                }
                ServiceEvent::ServiceRemoved(_, fullname) => {
                    peers.lock().unwrap().remove(&fullname);
                }
                ServiceEvent::SearchStopped(_) => break,
                ServiceEvent::SearchStarted(_) | ServiceEvent::ServiceFound(_, _) => {}
                _ => {}
            }
        }

        state.lock().unwrap().is_running = false;
    })
}

fn resolved_to_peer(
    service: &ResolvedService,
    local_display_name: &str,
    require_signature: bool,
) -> Option<DiscoveredPeer> {
    let display_name = service
        .get_property_val_str("device_name")
        .unwrap_or_else(|| service.get_fullname());
    if display_name == local_display_name {
        return None;
    }

    // Verify signature from TXT record.
    let key_hex = service.get_property_val_str("key");
    let sig_hex = service.get_property_val_str("sig");

    let signing_public_key = match (key_hex, sig_hex) {
        (Some(key), Some(sig)) => {
            match synap_core::service::discovery::verify_mdns_discovery_txt(key, sig) {
                Ok(pub_key) => pub_key,
                Err(err) => {
                    eprintln!(
                        "[corenet] mDNS signature verification failed for {}: {err}",
                        service.get_fullname()
                    );
                    return None;
                }
            }
        }
        _ if require_signature => {
            eprintln!(
                "[corenet] rejecting unsigned mDNS peer: {}",
                service.get_fullname()
            );
            return None;
        }
        _ => {
            // No signature and not required — accept with a zero key.
            [0u8; 32]
        }
    };

    let host = service
        .get_addresses_v4()
        .into_iter()
        .map(IpAddr::V4)
        .chain(
            service
                .get_addresses()
                .iter()
                .filter_map(|addr| match addr {
                    mdns_sd::ScopedIp::V6(v6) => Some(IpAddr::V6(*v6.addr())),
                    _ => None,
                }),
        )
        .find(|ip| !ip.is_loopback())?;

    Some(DiscoveredPeer {
        service_name: service.get_fullname().to_string(),
        display_name: display_name.to_string(),
        host: host.to_string(),
        port: service.get_port(),
        last_seen_at_ms: now_ms(),
        signing_public_key,
    })
}

fn build_host_name() -> String {
    let host = std::env::var("HOSTNAME")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "synap-desktop".to_string());
    format!("{host}.local.")
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}
