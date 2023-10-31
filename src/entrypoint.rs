use futures::StreamExt;
use libp2p::autonat::NatStatus;
use libp2p::dns::TokioDnsConfig;
use libp2p::swarm::SwarmBuilder;
use libp2p::Transport;
use libp2p::{
    autonat::Event as AutoNatEvent, dcutr::Event as DcutrEvent, identify::Event as IdentifyEvent,
    PeerId,
};
use libp2p::{
    core::{muxing::StreamMuxerBox, transport::Boxed},
    gossipsub::Event,
    identity, noise,
    swarm::SwarmEvent,
    tcp::Config as TcpConfig,
    yamux,
};
use log::{debug, error};
use std::error::Error;
use std::net::Ipv4Addr;
use std::time::Duration;

use crate::messages::NetworkTopic;
use crate::network::Network;
use crate::{
    behaviour::{PeerNetworkBehaviour, PeerNetworkEvent},
    config::Config,
    messages::{Envelope, MessageGateChain},
    network::SendCommand,
    utilities,
};

/// start p2p networking and return the handle [Network] of this process.
pub async fn start(
    config: Config,
    subscribe_topics: Vec<NetworkTopic>,
    message_gates: MessageGateChain,
) -> Result<Network, Box<dyn Error>> {
    let local_public_address = utilities::public_address(&config.keypair.public()).unwrap();
    let local_peer_id = config.keypair.public().to_peer_id();
    let local_keypair = config.keypair;

    // 1. Instantiate Swarm
    let transport = build_transport(local_keypair.clone()).await?;
    let behaviour: PeerNetworkBehaviour =
        PeerNetworkBehaviour::new(local_public_address, &local_keypair, 10);
    let mut swarm = SwarmBuilder::with_tokio_executor(transport, behaviour, local_peer_id).build();

    swarm.listen_on(utilities::multiaddr(Ipv4Addr::new(0, 0, 0, 0), config.port))?;

    // 2. Peer Discovery - connection to bootstrap nodes
    if !config.boot_nodes.is_empty() {
        config.boot_nodes.iter().for_each(|peer_info| {
            swarm.behaviour_mut().add_address(
                &peer_info.peer_id,
                utilities::multiaddr(peer_info.ip_address, peer_info.port),
            );
        });
    }

    // 3. Prepare Messaging Protocols
    swarm.behaviour_mut().subscribe(subscribe_topics)?;

    // 4. Start p2p networking
    let (sender, mut receiver) =
        tokio::sync::mpsc::channel::<SendCommand>(config.send_command_buffer_size);
    let mut discover_tick =
        tokio::time::interval(Duration::from_secs(config.peer_discovery_interval));

    let network_thread_handle = tokio::task::spawn(async move {
        loop {
            // 4.1 Wait until an Event comes
            let (send_command, event) = tokio::select! {
                biased;
                // Receive a Libp2p event
                event = swarm.select_next_some() => {
                    (None, Some(event))
                },
                // Receive a command from application
                send_command = receiver.recv() => {
                    (send_command, None)
                },
                // Time for network discovery
                _ = discover_tick.tick() => {
                    // Perform a random walk on DHT
                    swarm.behaviour_mut().random_walk();
                    (None, None)
                },
            };

            // 4.2 Deliver messages when received a Command from application
            if let Some(send_command) = send_command {
                match send_command {
                    SendCommand::SendTo(address, raw_message) => {
                        if address == local_public_address {
                            let envelope = Envelope {
                                origin: local_public_address,
                                message: raw_message,
                            };
                            message_gates
                                .message_in(
                                    &NetworkTopic::from(local_public_address).hash(),
                                    envelope,
                                )
                                .await;
                        } else if let Err(e) = swarm.behaviour_mut().send_to(address, raw_message) {
                            error!("{:?}", e);
                        }
                    }
                    SendCommand::Broadcast(topic, msg) => {
                        log::info!("Broadcast (Topic: {:?})", topic);
                        if let Err(e) = swarm.behaviour_mut().broadcast(topic.into(), msg) {
                            debug!("{:?}", e);
                        }
                    }
                }
            }

            // 4.3 Deliver messages when received a Libp2p Event
            if let Some(event) = event {
                match event {
                    SwarmEvent::Behaviour(PeerNetworkEvent::Gossip(Event::Message {
                        message,
                        ..
                    })) => {
                        if let Some(_) = &message.source {
                            if swarm.behaviour().is_subscribed(&message) {
                                let envelope = Envelope {
                                    origin: local_public_address,
                                    message: message.data,
                                };
                                message_gates.message_in(&message.topic, envelope).await;
                            } else {
                                debug!("Receive unknown gossip message");
                            }
                        }
                    }
                    SwarmEvent::Behaviour(PeerNetworkEvent::Identify(
                        IdentifyEvent::Received { peer_id, info },
                    )) => {
                        info.listen_addrs.iter().for_each(|a| {
                            swarm.behaviour_mut().add_address(&peer_id, a.clone());
                        });
                    }
                    SwarmEvent::ConnectionClosed { peer_id, .. } => {
                        debug!("ConnectionClosed {}", peer_id);
                        swarm.behaviour_mut().remove_peer(&peer_id);
                    }
                    SwarmEvent::Behaviour(PeerNetworkEvent::AutoNat(event)) => match event {
                        AutoNatEvent::InboundProbe(e) => {
                            debug!("AutoNat Inbound Probe: {:#?}", e);
                        }
                        AutoNatEvent::OutboundProbe(e) => {
                            debug!("AutoNat Outbound Probe: {:#?}", e);
                        }
                        AutoNatEvent::StatusChanged { old, new } => {
                            debug!("Old status: {:#?}. New status: {:#?}", old, new);
                            if new == NatStatus::Private || old == NatStatus::Private {
                                debug!("Autonat says we're still private.");
                            };
                        }
                    },
                    SwarmEvent::Behaviour(PeerNetworkEvent::Relay(event)) => {
                        debug! {"Relay Client Event: {event:#?}"};
                    }
                    SwarmEvent::Behaviour(PeerNetworkEvent::Dcutr(event)) => {
                        match event {
                            DcutrEvent::RemoteInitiatedDirectConnectionUpgrade {
                                remote_peer_id,
                                remote_relayed_addr,
                            } => {
                                debug!("Remote with ID: {remote_peer_id:#?} initiated Direct Connection Upgrade through address: {remote_relayed_addr:#?}");
                            }
                            DcutrEvent::InitiatedDirectConnectionUpgrade {
                                remote_peer_id,
                                local_relayed_addr,
                            } => {
                                debug!("Dcutr initiated with remote: {remote_peer_id:#?} on address: {local_relayed_addr:#?}");
                            }
                            DcutrEvent::DirectConnectionUpgradeSucceeded { remote_peer_id } => {
                                debug!("Hole punching success with: {remote_peer_id:#?}")
                            }
                            DcutrEvent::DirectConnectionUpgradeFailed {
                                remote_peer_id,
                                error,
                            } => {
                                debug!("Hole punching failed with: {remote_peer_id:#?}. Error: {error:#?}");
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    });

    Ok(Network {
        network_thread: network_thread_handle,
        sender,
    })
}

async fn build_transport(
    keypair: identity::Keypair,
) -> std::io::Result<Boxed<(PeerId, StreamMuxerBox)>> {
    let transport = {
        let tcp = libp2p::tcp::tokio::Transport::new(TcpConfig::new().nodelay(true));
        TokioDnsConfig::system(tcp)?
    };

    Ok(transport
        .upgrade(libp2p::core::upgrade::Version::V1)
        .authenticate(noise::Config::new(&keypair).unwrap())
        .multiplex(yamux::Config::default())
        .timeout(std::time::Duration::from_secs(20))
        .boxed())
}
