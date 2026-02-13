use registry::Peer;

use crate::config::InterfaceConfig;

pub fn generate_config(interface: &InterfaceConfig, peers: &[Peer]) -> String {
    let mut config = format!(
        "[Interface]\nPrivateKey = {}\nListenPort = {}\nAddress = {}\n",
        interface.private_key, interface.listen_port, interface.address,
    );
    for peer in peers {
        config.push_str(&format!(
            "\n[Peer]\nPublicKey = {}\nEndpoint = {}:{}\nAllowedIPs = {}\n",
            peer.public_key,
            peer.public_ip,
            peer.port,
            peer.allowed_ips
                .iter()
                .map(|ip| ip.to_string())
                .collect::<Vec<_>>()
                .join(", "),
        ));
    }
    config
}
