//! [Network], the handle type you use to send messages.
//!
//! Network is returned from [engine::start](crate::engine::start). It keeps the thread operating the
//! peer alive--the peer stops working when it is dropped.
//!
//! To send a message using Network, call its [sender](Network::sender) method to get a sender, then
//! call `.send()` on the sender passing in a [SendCommand].

use crate::messages::{Message, NetworkTopic};
use crate::PublicAddress;

use futures::{Future, FutureExt};
use tokio::{
    sync::Mutex,
    task::{JoinError, JoinHandle},
};

/// Network is the handle returned by [crate::engine::start]. It provides
/// Inter-process messaging between application and p2p network.
pub struct Network {
    /// Network handle for the [tokio::task] which is the main thread for
    /// the p2p network (see [crate::engine]).
    pub(crate) network_thread: JoinHandle<()>,

    /// mpsc sender for delivering message to p2p network
    pub(crate) sender: tokio::sync::mpsc::Sender<SendCommand>,
}

impl Network {
    /// sender is the channel for intake of SendCommand so that message can be sent to network by the Engine.
    pub fn sender(&self) -> tokio::sync::mpsc::Sender<SendCommand> {
        self.sender.clone()
    }

    /// abort the networking process
    pub async fn stop(self) {
        self.network_thread.abort();
        log::debug!("lynk network stop!");
    }
}

impl Future for Network {
    type Output = Result<(), JoinError>;
    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let mut s = self;
        s.network_thread.poll_unpin(cx)
    }
}

/// A command to send a message either to a specific peer ([SendTo](SendCommand::SendTo)), or to all subscribers
/// of a network topic ([Broadcast](SendTo::Broadcast)).
pub enum SendCommand {
    /// expects a peer with specific PublicAddress would be interested in
    SendTo(PublicAddress, Message),

    /// does not care which peer would be interested in
    Broadcast(NetworkTopic, Message),
}
