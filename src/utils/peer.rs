use ipnetwork::IpNetwork;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Peer {
    pub public_key: String,
    pub public_ip: String,
    pub allowed_ips: Vec<IpNetwork>,
}
