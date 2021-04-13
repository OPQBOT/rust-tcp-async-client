use crate::client::Client;
use crate::{errors::*, Packet};
use failure::Fail;
use futures::channel::mpsc;
use futures::TryFutureExt;
use rand_core::{OsRng, RngCore};
use std::collections::{hash_map, HashMap};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
// TCP connections provides reliable connection to a friend via multiple TCP
/// relays.
#[derive(Clone)]
pub struct Connections {
    /// belongs to TCP relay we received packet from.
    incoming_tx: mpsc::UnboundedSender<(Client, Packet)>,
    /// List of TCP relays we are connected to. Key is a `Clientid` of TCP
    /// relay.
    pub clients: Arc<RwLock<HashMap<String, Client>>>,
}

impl Connections {
    /// Create new TCP connections object.
    pub fn new(incoming_tx: mpsc::UnboundedSender<(Client, Packet)>) -> Self {
        Connections {
            incoming_tx,
            clients: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    pub fn gen_random_string(n: usize) -> String {
        let alphabet = b"abcdefghijklmnopqrstuvwxyz0123456789";
        let mut result = String::new();
        while result.len() < n {
            let x = OsRng.next_u64();
            result.push(alphabet[(x % alphabet.len() as u64) as usize] as char);
        }
        result
    }

    /// Add relay we are supposed to be connected to. These relays are necessary
    /// for initial connection so that we are able to find friends and to send
    /// them our relays. Later when more relays are received from our friends
    /// they should be added via `add_client` method.
    pub async fn add_client(
        &self,
        id: String,
        relay_addr: SocketAddr,
    ) -> Result<(), ConnectionError> {
        if let hash_map::Entry::Vacant(vacant) = self.clients.write().await.entry(id.clone()) {
            let client = Client::new(
                relay_addr,
                Arc::new(RwLock::new(id)),
                self.incoming_tx.clone(),
            );
            vacant.insert(client.clone());
            client
                .spawn()
                .map_err(|e| e.context(ConnectionErrorKind::Spawn).into())
                .await
        } else {
            //trace!("Attempt to add relay that already exists: {}", relay_addr);
            Ok(())
        }
    }
    /// Add a connection to our friend via relay. It means that we will send
    /// `RouteRequest` packet to this relay and wait for the friend to become
    /// connected.
    /// Send `Data` packet to a node via one of the relays.
    pub async fn send_data(&self, packet: Packet) -> Result<(), ConnectionError> {
        // send packet to the first relay only that can accept it
        // errors are ignored
        // TODO: return error if stream is exhausted?
        //  if let Some(connection) = connections.get(&node_pk) {
        let clients = self.clients.read().await;

        for (_id, c) in clients.iter() {
            //println!("client id send {:#?}",id);
            let res = c.send_packet(packet.clone()).await;
            if res.is_err() {
                break;
            }
        }
        // }

        Ok(())
    }

    /// Main loop that should be run periodically. It removes unreachable and
    /// redundant relays, reconnects to relays if a connection was lost, puts
    /// relays to sleep if they are not used right now.
    async fn main_loop(&self) -> Result<(), ConnectionError> {
        let mut clients = self.clients.write().await;
        // let mut connections = self.connections.write().await;

        // If we have at least one connected relay that means that our network
        // connection is fine. So if we can't connect to some relays we can
        // drop them.
        let mut connected = false;
        for client in clients.values() {
            if client.is_connected().await {
                connected = true;
                break;
            }
        }

        let mut to_remove = Vec::new();

        for (pk, client) in clients.iter_mut() {
            if client.is_disconnected().await {
                if connected && client.connection_attempts().await > 1 {
                    //重连
                    to_remove.push(pk.clone());
                } else {
                    client
                        .clone()
                        .spawn()
                        .await
                        .map_err(|e| e.context(ConnectionErrorKind::Spawn))?;
                }
            }
        }

        for id in to_remove {
            clients.remove(&id);
        }

        Ok(())
    }
    /// Run TCP periodical tasks. Result future will never be completed
    /// successfully.
    pub async fn run(&self) -> Result<(), ConnectionError> {
        let mut wakeups = tokio::time::interval(Duration::from_secs(1));

        loop {
            wakeups.tick().await;

            self.main_loop().await?
        }
    }
}
#[cfg(test)]
mod tests {

    use std::time::Duration;

    use crate::ping_request::PingRequest;
    use crate::pong_response::PongResponse;
    use crate::Packet;
    use futures::{channel::mpsc, StreamExt};
    use tokio::time::sleep;

    use super::Connections;

    //#[tokio::test]
    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn main_loop_remove_not_used() {
        let (incoming_tx, mut incoming_rx) = mpsc::unbounded();
        let connections = Connections::new(incoming_tx);

        for _ in 0..1 {
            //let &mut conn = &mut connections;
            let id = Connections::gen_random_string(16);
            connections
                .add_client(id, "192.168.198.120:8080".parse().unwrap())
                .await
                .unwrap();
        }

        let on_receive = async {
            while let Some(packet) = incoming_rx.next().await {
                match packet {
                    (c, Packet::PingRequest(pkg)) => {
                        if let Ok(res) = c.send_for_response(Packet::PingRequest(pkg.clone())).await
                        {
                            println!("rcv pkg conn  {:#?} {:#?}", c.client_id.read().await, res);
                        }

                        c.spawn_lua(Packet::PingRequest(pkg)).unwrap();
                    }
                    (_, Packet::PongResponse(pkg)) => {
                        println!("rcv pkg  {:#?}", pkg)
                    }
                    (_, Packet::ChatMessage(pkg)) => {
                        println!("rcv pkg  {:#?}", pkg)
                    }
                }
            }
        };

        let on_send = async {
            loop {
                sleep(Duration::from_millis(2 * 1000)).await;

                connections
                    .send_data(Packet::PingRequest(PingRequest { ping_id: 77 }))
                    .await
                    .unwrap();
            }
        };
        tokio::select! {
            _ = on_receive => {
                println!("write_loop() exit first")
            }
            _ = connections.run() => {
                println!("read_loop() exit first")
            },
            _ = on_send => {
                println!("read_loop() exit first")
            }
        };
    }
}
