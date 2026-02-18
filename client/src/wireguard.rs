use std::net::SocketAddr;

use ipnetwork::IpNetwork;
use wireguard_control::{
    AllowedIp, Backend, Device, DeviceUpdate, InterfaceName, InvalidKey, Key, PeerConfigBuilder,
};

use crate::config::INTERFACE_NAME;
use crate::error::{Error, Result};

pub fn parse_key(key_b64: &str) -> Result<Key> {
    Key::from_base64(key_b64).map_err(|e: InvalidKey| Error::invalid_key(e.to_string()))
}

pub fn interface_name() -> InterfaceName {
    INTERFACE_NAME.parse().expect("valid interface name")
}

pub fn create_interface() -> Result<()> {
    let name = interface_name();

    if Device::get(&name, Backend::Kernel).is_ok() {
        tracing::debug!(interface = INTERFACE_NAME, "interface already exists");
        return Ok(());
    }

    tracing::info!(interface = INTERFACE_NAME, "creating WireGuard interface");

    DeviceUpdate::new()
        .apply(&name, Backend::Kernel)
        .map_err(|e| Error::create_interface(e, INTERFACE_NAME.to_string()))?;

    Ok(())
}

pub fn configure_interface(private_key: &Key, listen_port: u16) -> Result<()> {
    let name = interface_name();

    tracing::debug!(
        interface = INTERFACE_NAME,
        listen_port,
        "configuring WireGuard interface"
    );

    DeviceUpdate::new()
        .set_private_key(private_key.clone())
        .set_listen_port(listen_port)
        .apply(&name, Backend::Kernel)
        .map_err(|e| Error::configure_device(e, INTERFACE_NAME.to_string()))?;

    Ok(())
}

pub fn configure_peer(
    public_key: &Key,
    endpoint: Option<SocketAddr>,
    allowed_ips: &[IpNetwork],
    persistent_keepalive: Option<u16>,
) -> Result<()> {
    let name = interface_name();

    let allowed: Vec<AllowedIp> = allowed_ips
        .iter()
        .map(|ip| AllowedIp {
            address: ip.ip(),
            cidr: ip.prefix(),
        })
        .collect();

    let mut peer = PeerConfigBuilder::new(public_key)
        .replace_allowed_ips()
        .add_allowed_ips(&allowed);
    if let Some(ep) = endpoint {
        peer = peer.set_endpoint(ep);
    }
    if let Some(keepalive) = persistent_keepalive {
        peer = peer.set_persistent_keepalive_interval(keepalive);
    }

    tracing::debug!(
        peer_key = %public_key.to_base64(),
        endpoint = ?endpoint,
        allowed_ips = ?allowed_ips,
        "configuring peer"
    );

    DeviceUpdate::new()
        .add_peer(peer)
        .apply(&name, Backend::Kernel)
        .map_err(|e| Error::configure_device(e, INTERFACE_NAME.to_string()))?;

    Ok(())
}

#[allow(dead_code)]
pub fn remove_peer(public_key: &Key) -> Result<()> {
    let name = interface_name();

    tracing::info!(
        peer_key = %public_key.to_base64(),
        "removing peer"
    );

    DeviceUpdate::new()
        .remove_peer_by_key(public_key)
        .apply(&name, Backend::Kernel)
        .map_err(|e| Error::configure_device(e, INTERFACE_NAME.to_string()))?;

    Ok(())
}

#[allow(dead_code)]
pub fn get_device() -> Result<Device> {
    let name = interface_name();
    Device::get(&name, Backend::Kernel)
        .map_err(|e| Error::configure_device(e, INTERFACE_NAME.to_string()))
}
