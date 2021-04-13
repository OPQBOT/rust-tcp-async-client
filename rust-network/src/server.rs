use crate::codec::Codec;
use crate::connections::Connections;
use crate::{
    chatmsg::ChatMessage,
    codec::{DecodeError, EncodeError},
    ping_request::PingRequest,
    pong_response::PongResponse,
    stats::Stats,
    Packet,
};
use failure::Fail;
use futures::channel::mpsc::{self, Sender};
use futures::FutureExt;
use futures::TryFutureExt;
use futures::{SinkExt, StreamExt, TryStreamExt};
use std::{
    collections::{hash_map, HashMap},
    io::{Error, ErrorKind},
};
use std::{io::Error as IoError, net::SocketAddr};
use std::{
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::Duration,
};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::RwLock,
    time::error::Error as TimerError,
};
use tokio_util::codec::Framed;

/// Interval of time for Tcp Ping sender
const TCP_PING_INTERVAL: Duration = Duration::from_secs(5);

/// Interval of time for the TCP handshake.
//const TCP_HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(10);

const SERVER_CHANNEL_SIZE: usize = 2;

/// Error that can happen during server execution
#[derive(Debug, Fail)]
pub enum ServerRunError {
    /// Incoming IO error
    #[fail(display = "Incoming IO error: {:?}", error)]
    IncomingError {
        /// IO error
        #[fail(cause)]
        error: IoError,
    },
    /// Ping wakeups timer error
    #[fail(display = "Ping wakeups timer error: {:?}", error)]
    PingWakeupsError {
        /// Timer error
        error: TimerError,
    },
    /// Send pings error
    #[fail(display = "Send pings error: {:?}", error)]
    SendPingsError {
        /// Send pings error
        #[fail(cause)]
        error: IoError,
    },
}

/// Error that can happen during TCP connection execution
#[derive(Debug, Fail)]
pub enum ConnectionError {
    /// Error indicates that we couldn't get peer address
    #[fail(display = "Failed to get peer address: {}", error)]
    PeerAddrError {
        /// Peer address error
        #[fail(cause)]
        error: IoError,
    },
    /// Sending packet error
    #[fail(display = "Failed to send TCP packet: {}", error)]
    SendPacketError { error: EncodeError },
    /// Decode incoming packet error
    #[fail(display = "Failed to decode incoming packet: {}", error)]
    DecodePacketError { error: DecodeError },
    /// Incoming IO error
    #[fail(display = "Incoming IO error: {:?}", error)]
    IncomingError {
        /// IO error
        #[fail(cause)]
        error: IoError,
    },
    /// Server handshake error
    #[fail(display = "Server handshake error: {:?}", error)]
    ServerHandshakeTimeoutError {
        /// Server handshake error
        #[fail(cause)]
        error: tokio::time::error::Elapsed,
    },
    #[fail(display = "Server handshake error: {:?}", error)]
    ServerHandshakeIoError {
        /// Server handshake error
        #[fail(cause)]
        error: IoError,
    },
    /// Packet handling error
    #[fail(display = "Packet handling error: {:?}", error)]
    PacketHandlingError {
        /// Packet handling error
        #[fail(cause)]
        error: IoError,
    },
    /// Insert client error
    #[fail(display = "Packet handling error: {:?}", error)]
    InsertClientError {
        /// Insert client error
        #[fail(cause)]
        error: IoError,
    },

    #[fail(display = "Packet handling error: {:?}", error)]
    ShutdownError {
        /// Insert client error
        #[fail(cause)]
        error: IoError,
    },
}

#[derive(Clone)]
pub struct Server {
   
    pub clients: Arc<RwLock<HashMap<String, Sender<Packet>>>>,
}

#[derive(Default, Clone)]
struct ServerState {
    //pub connected_clients: HashMap<u32, Client>,
}

impl Server {
    /**
    Create a new `Server` without onion
    */
    pub fn new() -> Server {
        Server {
            clients: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    /// 解析接收的请求
    pub async fn handle_packet(&self, packet: Packet, mut tx: Sender<Packet>) -> Result<(), Error> {
        match packet {
            Packet::PingRequest(packet) => self.handle_ping_request(&packet).await,
            Packet::PongResponse(packet) => self.handle_pong_response(&packet).await,
            Packet::ChatMessage(mut p) => {
                println!(
                    "收到客户端消息 消息ID {} to {} from {} content {}",
                    p.msg_id,
                    &p.to_user,
                    &p.from_user,
                    std::str::from_utf8(&p.content).unwrap().to_string()
                );
                p.content = format!("{}{}", "来自服务端消息", Connections::gen_random_string(16))
                    .as_bytes()
                    .to_vec();

                println!(
                    "发送内容 {}",
                    std::str::from_utf8(&p.content).unwrap().to_string()
                );
                tx.send(Packet::ChatMessage(p)).await;
                Ok(())
            }
        }
    }

    /// 解析ping
    async fn handle_ping_request(&self, packet: &PingRequest) -> Result<(), Error> {
        if packet.ping_id == 0 {
            return Err(Error::new(ErrorKind::Other, "PingRequest.ping_id == 0"));
        }
        // let state = self.state.read().await;
        // if let Some(client_a) = state.connected_clients.get(pk) {
        //     client_a.send_pong_response(packet.ping_id).await
        // } else {
        //     Err(Error::new(ErrorKind::Other, "PingRequest: no such PK"))
        // }
        Ok(())
    }
    /// 解析pong
    async fn handle_pong_response(&self, packet: &PongResponse) -> Result<(), Error> {
        if packet.ping_id == 0 {
            return Err(Error::new(ErrorKind::Other, "PongResponse.ping_id == 0"));
        }
        //let mut state = self.state.write().await;
        //     if let Some(client_a) = state.connected_clients.get_mut(pk) {
        //         if packet.ping_id == client_a.ping_id() {
        //             client_a.set_last_pong_resp(Instant::now());

        //             Ok(())
        //         } else {
        //             Err(Error::new(ErrorKind::Other, "PongResponse.ping_id does not match"))
        //         }
        //     } else {
        //         Err(Error::new(ErrorKind::Other, "PongResponse: no such PK"))
        //     }
        // }
        Ok(())
    }
}
/// Running TCP ping sender and incoming `TcpStream`. This function uses
/// `tokio::spawn` inside so it should be executed via tokio to be able to
/// get tokio default executor.
pub async fn tcp_run(
    server: &Server,
    addr: SocketAddr,
    stats: Stats,
    connections_limit: usize,
) -> Result<(), ServerRunError> {
    let connections_count = Arc::new(AtomicUsize::new(0));
    
    let listener = TcpListener::bind(&addr).await.unwrap();

    println!("Tcp server bind {} ", addr);
    let connections_future = async {
        loop {
            let (stream, _) = listener
                .accept()
                .await
                .map_err(|error| ServerRunError::IncomingError { error })?;
            if connections_count.load(Ordering::SeqCst) < connections_limit {
                connections_count.fetch_add(1, Ordering::SeqCst);
                let connections_count_c = connections_count.clone();
               
                let stats = stats.clone();
                let server = server.clone();

                tokio::spawn(async move {
                    let res = tcp_run_connection(&server, stream, stats).await;

                    if let Err(ref e) = res {
                        println!("Error while running tcp connection: {:?}", e)
                    }

                    connections_count_c.fetch_sub(1, Ordering::SeqCst);

                    res
                });
            } else {
                // trace!(
                //     "Tcp server has reached the limit of {} connections",
                //     connections_limit
                // );
            }
        }
    };

    let mut wakeups = tokio::time::interval(TCP_PING_INTERVAL);
    let ping_future = async {
        loop {
            wakeups.tick().await;

            //trace!("Tcp server ping sender wake up");
            // server.send_pings().await
            //     .map_err(|error| ServerRunError::SendPingsError { error })?;

            let msg = ChatMessage {
                msg_id: 66778899,
                to_user: "123".to_string(),
                from_user: "789".to_string(),
                content: Connections::gen_random_string(16).as_bytes().to_vec(),
            };

            for (_, client) in server.clients.write().await.iter_mut() {
                client.send(Packet::ChatMessage(msg.clone())).await;
            }
        }
    };

    futures::select! {
        res = connections_future.fuse() => res,
        res = ping_future.fuse() => res,
    }
}

/// Running TCP server on incoming `TcpStream`
pub async fn tcp_run_connection(
    server: &Server,
    stream: TcpStream,
    stats: Stats,
) -> Result<(), ConnectionError> {
    let addr = match stream.peer_addr() {
        Ok(addr) => addr,
        Err(error) => return Err(ConnectionError::PeerAddrError { error }),
    };

    let secure_socket = Framed::new(stream, Codec::new(stats));
    let (mut to_client, from_client) = secure_socket.split();
    let (to_client_tx, mut to_client_rx) = mpsc::channel(SERVER_CHANNEL_SIZE);

    if let hash_map::Entry::Vacant(vacant) =
        server.clients.write().await.entry(addr.clone().to_string())
    {
        vacant.insert(to_client_tx.clone());
    }

    // processor = for each Packet from client process it
    let processor = from_client
        .map_err(|error| ConnectionError::DecodePacketError { error })
        .try_for_each(|packet| {
            println!("Handle  => {:?}", packet);
            let tx_clone = to_client_tx.clone();
            server
                .handle_packet(packet, tx_clone)
                .map_err(|error| ConnectionError::PacketHandlingError { error })
        });

    let writer = async {
        while let Some(packet) = to_client_rx.next().await {
            println!("Sending TCP packet {:?} to ", &addr);
            to_client
                .send(packet)
                .await
                .map_err(|error| ConnectionError::SendPacketError { error })?;
        }

        Ok(())
    };

    let r_processing = futures::select! {
        res = processor.fuse() => res,
        res = writer.fuse() => res
    };

    println!("Client Disconnect {}", addr);
    r_processing
}