use serde::{Deserialize, Serialize};

use ipnetwork::IpNetwork;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Peer {
    pub public_key: String,
    pub public_ip: String,
    pub port: String,
    pub allowed_ips: Vec<IpNetwork>,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum PeerMessage {
    HydratePeers { peers: Vec<Peer> },
    PeerUpdate { peer: Peer },
}
