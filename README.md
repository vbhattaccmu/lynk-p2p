> [!WARNING]
> This repository is Work Under Progress.

#### Lynk: A Generalized Libp2p Network

The Lynk network implementation utilizes four protocols from the libp2p library, operating on top of a TCP transport secured with Noise authentication. These protocols are:

**1. Kademlia**: This protocol establishes and maintains an efficient network topology, ensuring that every peer can reach any other peer in a small number of hops. It enables efficient communication within the network. For more details, refer to the [Kademlia specification](https://github.com/libp2p/specs/tree/master/kad-dht).

**2. Identify**: The Identify protocol allows peers to inform each other about changes in their basic information, such as peer IDs, supported protocols, and network addresses. This information exchange enables peers to stay updated and maintain accurate knowledge about other peers in the network. For further information, see the [Identify specification](https://github.com/libp2p/specs/tree/master/identify).

**3. Ping**: The Ping protocol enables peers to quickly check the liveness of other peers, facilitating efficient detection of unresponsive or disconnected peers in the network. The [Ping specification](https://github.com/libp2p/specs/blob/master/ping/ping.md) provides more insights into its implementation and usage.

**4. Gossipsub**: Gossipsub is a protocol that allows general publish/subscribe messaging within the network. Peers can publish messages on specific topics and subscribe to receive messages related to those topics. This protocol plays a crucial role in enabling efficient communication and information sharing among peers. For detailed implementation information, refer to the [Gossipsub specification](https://github.com/libp2p/specs/tree/master/pubsub/gossipsub).

**5. Hole Punching**: The crate comes with hole punching enabled for symmetric NATs via AutoNat, Dcutr and Circuit relay over TCP (WIP) .

### Upcoming Tasks

- [ ] Implement circuit relay logic for events in swarm controller
- [ ] Add quic in the transport layer
- [ ] Implement Kadmelia Indexer
- [ ] Add tests
- [ ] Cleanup code
