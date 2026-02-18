use base64::Engine;
use serde::{de, Deserialize};

#[derive(Clone)]
pub struct WireguardPublicKey(String);

impl<'de> Deserialize<'de> for WireguardPublicKey {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;

        let bytes = base64::engine::general_purpose::STANDARD
            .decode(&s)
            .map_err(|_| de::Error::custom("invalid base64 in public key"))?;

        if bytes.len() != 32 {
            return Err(de::Error::invalid_length(
                bytes.len(),
                &"exactly 32 bytes for a Wireguard public key",
            ));
        }

        Ok(WireguardPublicKey(s))
    }
}

impl WireguardPublicKey {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
