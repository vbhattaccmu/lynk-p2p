use libp2p::{
    autonat::{self, Behaviour as AutoNat, Event as AutoNatEvent},
    dcutr::{Behaviour as Dcutr, Event as DcutrEvent},
    gossipsub::{
        ConfigBuilder, Event as GossipEvent, IdentTopic, Message as GossipsubMessage, MessageId,
    },
    identify::{Behaviour as Identify, Config as IdentifyConfig, Event as IdentifyEvent},
    identity::{ed25519, Keypair, PublicKey},
    kad::{store::MemoryStore, Kademlia, KademliaConfig, KademliaEvent},
    ping::{Behaviour as Ping, Config as PingConfig, Event as PingEvent},
    relay::{
        self,
        client::{Behaviour as RelayClient, Event as RelayEvent},
    },
    swarm::NetworkBehaviour,
    Multiaddr, PeerId, StreamProtocol,
};
use std::time::Duration;

use crate::{
    messages::{Message, NetworkTopic},
    PublicAddress,
};

#[derive(NetworkBehaviour)]
#[behaviour(to_swarm = "PeerNetworkEvent")]
pub(crate) struct PeerNetworkBehaviour {
    kad: Kademlia<MemoryStore>,
    gossip: libp2p::gossipsub::Behaviour,
    identify: Identify,
    ping: Ping,
    auto_nat: AutoNat,
    relay_client: RelayClient,
    dcutr: Dcutr,
}

impl PeerNetworkBehaviour {
    pub fn new(id: PublicAddress, local_key: &Keypair, heartbeat_secs: u64) -> Self {
        let proto_version = "/lynk_p2p/1.0.0";
        let public_key: PublicKey = ed25519::PublicKey::try_from_bytes(&id)
            .expect("Invalid public key to setup peer newtork.")
            .into();
        let local_peer_id = public_key.to_peer_id();

        // Configure Kad
        let proto_names = vec![StreamProtocol::new(proto_version)];
        let kad_config = KademliaConfig::default()
            .set_protocol_names(proto_names)
            .to_owned();
        let mut kad = Kademlia::<MemoryStore>::with_config(
            local_peer_id,
            MemoryStore::new(local_peer_id),
            kad_config,
        );

        kad.set_mode(Some(libp2p::kad::Mode::Server));

        // Configure Identify
        let identify_config = IdentifyConfig::new(proto_version.to_string(), local_key.public());
        let identify = Identify::new(identify_config);

        // Configure Gossip
        let message_id_fn = |message: &GossipsubMessage| {
            // message id is: source + topic + sequence number
            let mut id_str = message.topic.as_str().to_string();
            let src_str = match message.source {
                Some(src) => base64url::encode(src.to_bytes()),
                None => "none".to_string(),
            };
            id_str.push_str(&src_str);
            id_str.push_str(&message.sequence_number.unwrap_or_default().to_string());
            MessageId::from(id_str.to_string())
        };

        let mut gossip = libp2p::gossipsub::Behaviour::new(
            libp2p::gossipsub::MessageAuthenticity::Signed(local_key.clone()),
            ConfigBuilder::default()
                .max_transmit_size(4 * 1024 * 1024) // block size is limitted to 2 MB. Multiply by factor of safety = 2.
                .message_id_fn(message_id_fn)
                .heartbeat_interval(Duration::from_secs(heartbeat_secs))
                .build()
                .unwrap(),
        )
        .unwrap();
        // subscribe a network topic that uses Base64 encoded public address of this network peer as topic.
        gossip.subscribe(&NetworkTopic::from(id).into()).unwrap();

        // Configure Ping
        let ping = Ping::new(PingConfig::new());

        // create AutoNAT Client Config
        let autonat_cfg = autonat::Config {
            ..Default::default()
        };

        let (relay_client_transport, relay_client_behaviour) = relay::client::new(local_peer_id);

        Self {
            gossip,
            kad,
            identify,
            ping,
            dcutr: Dcutr::new(local_peer_id),
            relay_client: relay_client_behaviour,
            auto_nat: AutoNat::new(local_peer_id, autonat_cfg),
        }
    }

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

    /// Subscribes libp2p::gossipsub::PeerNetworkBehaviour topics
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
    pub fn send_to(
        &mut self,
        address: PublicAddress,
        msg: Message,
    ) -> Result<libp2p::gossipsub::MessageId, libp2p::gossipsub::PublishError> {
        let topic: IdentTopic = NetworkTopic::from(address).into();
        self.gossip.publish(topic, msg)
    }

    /// Broadcasts Messages to peers with specific topic
    pub fn broadcast(
        &mut self,
        topic: IdentTopic,
        msg: Message,
    ) -> Result<libp2p::gossipsub::MessageId, libp2p::gossipsub::PublishError> {
        self.gossip.publish(topic, msg)
    }

    /// Check if the GossipsubMessage belongs to subscribed message topics
    pub fn is_subscribed(&self, message: &GossipsubMessage) -> bool {
        self.gossip.topics().any(|topic| message.topic.eq(topic))
    }
}

pub(crate) enum PeerNetworkEvent {
    Kad(KademliaEvent),
    Gossip(GossipEvent),
    Ping(PingEvent),
    Identify(IdentifyEvent),
    AutoNat(AutoNatEvent),
    Relay(RelayEvent),
    Dcutr(DcutrEvent),
}

impl From<GossipEvent> for PeerNetworkEvent {
    fn from(event: GossipEvent) -> Self {
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

impl From<AutoNatEvent> for PeerNetworkEvent {
    fn from(event: AutoNatEvent) -> Self {
        Self::AutoNat(event)
    }
}

impl From<RelayEvent> for PeerNetworkEvent {
    fn from(event: RelayEvent) -> Self {
        Self::Relay(event)
    }
}

impl From<DcutrEvent> for PeerNetworkEvent {
    fn from(event: DcutrEvent) -> Self {
        Self::Dcutr(event)
    }
}
