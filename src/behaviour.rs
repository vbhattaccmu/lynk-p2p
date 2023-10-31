use libp2p::{
    gossipsub::{Event, ConfigBuilder, IdentTopic, Message as GossipsubMessage, MessageId},
    identity::{Keypair, PublicKey, ed25519},
    kad::{Kademlia, KademliaEvent, store::MemoryStore, KademliaConfig},
    ping::{Behaviour as Ping, Event as PingEvent, Config as PingConfig}, 
    identify::{Behaviour as Identify, Config as  IdentifyConfig, Event as IdentifyEvent},
    Multiaddr, PeerId, StreamProtocol
};

use libp2p::swarm::NetworkBehaviour;
use std::{time::Duration, vec};

use crate::{messages::{Message, NetworkTopic}, PublicAddress};

#[derive(NetworkBehaviour)]
#[behaviour(to_swarm = "PeerNetworkEvent")]
pub(crate) struct PeerNetworkBehaviour {
    kad: Kademlia<MemoryStore>,
    gossip: libp2p::gossipsub::Behaviour,
    identify: Identify,
    ping: Ping,
}

impl PeerNetworkBehaviour {
    pub fn new(id: PublicAddress, local_key: &Keypair, heartbeat_secs: u64) -> Self {
        todo!()
    }
}

impl PeerNetworkBehaviour {
    /// Add address to DHT
    pub fn add_address(&mut self, peer: &PeerId, address: Multiaddr) {
        self.kad.add_address(peer, address);
    }

    /// Remove a peer from DHT
    pub fn remove_peer(&mut self, peer: &PeerId) {
        self.kad.remove_peer(peer);
    }

    /// Query the network with random PeerId so as to discover
    /// peers in the network.
    pub fn random_walk(&mut self) {
        self.kad.get_closest_peers(PeerId::random());
    }

    /// Subscribes libp2p::gossipsub::Behaviour topics
    pub fn subscribe(
        &mut self,
        topics: Vec<NetworkTopic>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        for topic in topics {
            self.gossip.subscribe(&topic.into())?;
        }

        Ok(())
    }

    /// Sends Message to peer with specific public address
    pub fn send_to(&mut self, address: PublicAddress, msg: Message) -> Result<libp2p::gossipsub::MessageId, libp2p::gossipsub::PublishError> {
        let topic: IdentTopic = NetworkTopic::from(address).into();
        self.gossip.publish(topic, msg)
    }

    /// Broadcasts Messages to peers with specific topic
    pub fn broadcast(&mut self, topic: IdentTopic, msg: Message) -> Result<libp2p::gossipsub::MessageId, libp2p::gossipsub::PublishError> {
        self.gossip.publish(topic, msg)
    }

    /// Check if the GossipsubMessage belongs to subscribed message topics
    pub fn is_subscribed(&self, message: &GossipsubMessage) -> bool {
        self.gossip.topics().any(|topic| message.topic.eq(topic))
    }
}

pub(crate) enum PeerNetworkEvent {
    Kad(KademliaEvent),
    Gossip(Event),
    Ping(PingEvent),
    Identify(IdentifyEvent),
}

impl From<Event> for PeerNetworkEvent {
    fn from(event: Event) -> Self {
        Self::Gossip(event)
    }
}

impl From<KademliaEvent> for PeerNetworkEvent {
    fn from(event: KademliaEvent) -> Self {
        Self::Kad(event)
    }
}

impl From<PingEvent> for PeerNetworkEvent {
    fn from(event: PingEvent) -> Self {
        Self::Ping(event)
    }
}

impl From<IdentifyEvent> for PeerNetworkEvent {
    fn from(event: IdentifyEvent) -> Self {
        Self::Identify(event)
    }
}