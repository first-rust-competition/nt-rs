pub mod callback;
pub mod topic;

use crate::Result;

pub use self::topic::*;
use crate::backend::{server::NTServer, NTBackend, Server, State};
use crate::nt::callback::*;
use crate::nt::topic::{Topic, TopicFlags};
use crate::proto::prelude::NTValue;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::{watch, Mutex};
use tokio::net::ToSocketAddrs;
use std::string::ToString;

/// Core struct representing a connection to a NetworkTables server
pub struct NetworkTables<T: NTBackend> {
    state: Arc<Mutex<T::State>>,
    close_tx: watch::Sender<u8>,
}

//impl NetworkTables<Client> {
//    /// Connects over TCP to the given ip, with the given client_name
//    ///
//    /// This call will block the thread until the client has completed the handshake with the server,
//    /// at which point the connection will be valid to send and receive data over
//    pub async fn connect(ip: &str, client_name: &str) -> Result<NetworkTables<Client>> {
//        let (close_tx, close_rx) = oneshot::channel::<()>();
//        // let state = ClientState::new(ip.to_string(), client_name.to_string(), close_rx).await;
//        // Ok(NetworkTables { state, close_tx })
//        todo!()
//    }
//
//    /// Attempts to reconnect to the NetworkTables server if the connection had been terminated.
//    ///
//    /// This function should _only_ be called if you are certain that the previous connection is dead.
//    /// Connection status can be determined using callbacks specified with `add_connection_callback`.
//    pub async fn reconnect(&mut self) {
//        todo!()
//        // let rt_state = self.state.clone();
//        //
//        // let (close_tx, close_rx) = channel::<()>(1);
//        // let (packet_tx, packet_rx) = unbounded::<Box<dyn Packet>>();
//        // let (ready_tx, mut ready_rx) = unbounded();
//        //
//        // self.close_tx = close_tx;
//        // {
//        //     let mut state = self.state.lock().unwrap();
//        //     state.packet_tx = packet_tx;
//        //     state.entries_mut().clear();
//        //     state.pending_entries.clear();
//        // }
//        // thread::spawn(move || {
//        //     let mut rt = Runtime::new().unwrap();
//        //     rt.block_on(crate::backend::client::conn::connection(
//        //         rt_state, packet_rx, ready_tx, close_rx,
//        //     ))
//        //     .unwrap();
//        // });
//        //
//        // let _ = ready_rx.next().await;
//    }
//
//    pub fn add_connection_callback(
//        &self,
//        callback_type: ConnectionCallbackType,
//        action: impl FnMut(&SocketAddr) + Send + 'static,
//    ) {
//        todo!()
//        // self.state
//        //     .lock()
//        //     .unwrap()
//        //     .add_connection_callback(callback_type, action);
//    }
//}

impl NetworkTables<Server> {
    /// Initializes an NT server over TCP and binds it to the given ip, with the given server name.
    ///
    /// # Time Source
    /// `time_source` is a closure returning an NT4 timestamp in microseconds, used for synchronizing
    /// time across the network.
    ///
    /// ## Defaults
    /// If unsure about what time source to use, the library provides the default [`system_time`] source
    /// when compiled with the `chrono` feature. This time source uses the system clock to synchronize the network.
    ///
    /// ## FRC Robots
    /// If running on an FRC robot, it is recommended to use FPGA time over system clock time. Simply create a function that
    /// creates the timestamp given FPGA time and pass it as the time source instead.
    ///
    /// ## Time Source Considerations
    /// The time source closure is used to synchronize topic timestamps across the network, and when a client
    /// updates its own time reference from the server, it is important to minimize slowdown caused by processing by the server.
    /// Due to this, a closure passed as time source **must** return its timestamp as fast as practical. Try to avoid unnecessary logging
    /// or IO operations in a custom time source.
    ///
    /// [`system_time`]: ../fn.system_time.html
    pub async fn bind(
        ip: impl ToSocketAddrs + Send + Sync + 'static,
        time_source: impl Fn() -> u64 + Send + Sync + 'static,
    ) -> NetworkTables<Server> {
        let (close_tx, close_rx) = watch::channel::<u8>(0);
        let state = NTServer::new(ip, time_source, close_rx).await;
        NetworkTables { state, close_tx }
    }

    /// Adds a callback for connection state updates regarding clients.
    ///
    /// Depending on the chosen callback type, the callback will be called when a new client connects,
    /// or when an existing client disconnects from the server
    pub fn add_connection_callback(
        &mut self,
        callback_type: ConnectionCallbackType,
        action: impl FnMut(&SocketAddr) + Send + 'static,
    ) {
        todo!()
        // self.state
        //     .lock()
        //     .unwrap()
        //     .add_server_callback(callback_type, action);
    }
}

impl<T: NTBackend> NetworkTables<T> {
    /// Returns a copy of the entries recognizes by the connection
    pub async fn topics(&self) -> HashMap<String, Topic> {
        self.state.lock().await.topics().clone()
    }

    /// Begin publishing the given topic.
    ///
    /// Once this function returns, values can be published to the topic specified,
    /// using its name as a key.
    pub async fn publish(&self, topic: Topic) {
        self.state.lock().await.publish(topic).await;
    }

    /// Releases the given topic, if it was being published by this NT instance.
    ///
    /// This does not necessarily delete the value associated with the topic.
    /// Topic values are deleted when the last client publishing to a non-persistent topic releases it.
    pub async fn release(&self, name: &str) {
        self.state.lock().await.release(name).await;
    }

    /// Updates the topic with the given name, with the new value
    pub async fn update_topic(&self, name: &str, value: NTValue) {
        self.state.lock().await.update_topic(name, value).await;
    }

    pub async fn subscribe(&self, prefix: impl ToString, callback: impl FnMut(&TopicSnapshot, CallbackType) + Send + Sync + 'static) -> u32 {
        self.state.lock().await.subscribe(prefix.to_string(), Box::new(callback)).await
    }

    pub async fn unsubscribe(&self, subuid: u32) {
        self.state.lock().await.unsubscribe(subuid).await;
    }

    /// Updates the flags associated with the topic of the given name
    pub async fn update_topic_flags(&self, name: &str, new_flags: TopicFlags) {
        self.state
            .lock()
            .await
            .update_topic_flags(name, new_flags).await;
    }
}

impl<T: NTBackend> Drop for NetworkTables<T> {
    fn drop(&mut self) {
        let _ = self.close_tx.broadcast(1);
    }
}
