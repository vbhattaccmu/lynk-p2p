use libp2p::{identity::PublicKey, Multiaddr};
use std::net::Ipv4Addr;

use crate::PublicAddress;

/// Convert PublicKey in libp2p to PublicAddress. The PublicKey must be an
/// Ed25519 key, otherwise the method returns None.
pub fn public_address(public_key: &PublicKey) -> Option<PublicAddress> {
    match public_key.clone().try_into_ed25519() {
        Ok(kp) => Some(kp.to_bytes()),
        _=> None
    }
}

/// Create Multiaddr from IP address and port number.
pub fn multiaddr(ip_address: Ipv4Addr, port: u16) -> Multiaddr {
    format!("/ip4/{}/tcp/{}", ip_address, port).parse().unwrap()
}
