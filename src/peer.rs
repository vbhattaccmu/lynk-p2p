use std::net::Ipv4Addr;

use crate::PublicAddress;
use libp2p::{
    identity::{ed25519, PublicKey},
    PeerId,
};

/// Peer consists of required information to identify an entity in the network, such as
/// Peer Id, IPv4 Address and port number.
#[derive(Clone)]
pub struct Peer {
    /// Peer id in the p2p network
    pub peer_id: PeerId,

    /// IP address (v4) of connection
    pub ip_address: Ipv4Addr,

    /// port number of connection
    pub port: u16,
}

impl Peer {
    /// Instantiation of PeerInfo. It is used in bootstrap nodes in [crate::configuration::Config].
    ///
    /// ## Panics
    /// Panics if address is not a valid Ed25519 public key.
    pub fn new(address: PublicAddress, ip_address: Ipv4Addr, port: u16) -> Self {
        let ed25519_pk: PublicKey = ed25519::PublicKey::try_from_bytes(&address).unwrap().into();
        Self {
            peer_id: ed25519_pk.to_peer_id(),
            ip_address,
            port,
        }
    }
}
