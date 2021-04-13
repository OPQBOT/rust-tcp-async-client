use crate::codec;
use crate::errors::*;
use crate::Packet;
use crate::{chatmsg::ChatMessage, stats};
use codec::Codec;
use failure::Fail;
use futures::channel::mpsc;
use futures::{FutureExt, SinkExt, StreamExt, TryFutureExt};
use mlua::{Function, Lua, MetaMethod, Table, ToLua, UserData, UserDataMethods};
use stats::Stats;
use std::{
    collections::HashMap,
    sync::atomic::{AtomicUsize, Ordering},
    time::Duration,
};
use std::{fs::File, io::Read, net::SocketAddr, sync::Arc, time::Instant};
use tokio::{
    net::TcpStream,
    sync::{oneshot, Mutex, RwLock},
};
use tokio_util::codec::Framed;

type ResponseMap = HashMap<usize, oneshot::Sender<Packet>>;
/// Client connection to a TCP relay.
#[derive(Clone, Debug)]
pub struct Client {
    /// IP address of the TCP relay.
    pub addr: SocketAddr,
    ///  client_id
    pub client_id: Arc<RwLock<String>>,
    /// Sink for packets that should be handled somewhere else.
    /// belongs to TCP relay.
    incoming_tx: mpsc::UnboundedSender<(Client, Packet)>,
    /// Status of the relay.
    status: Arc<RwLock<ClientStatus>>,
    /// Time when a connection to the relay was established.
    connected_time: Arc<RwLock<Option<Instant>>>,
    /// Number of unsuccessful attempts to establish connection to the relay.
    /// This is used to decide what to do after the connection terminates.
    connection_attempts: Arc<RwLock<u32>>,

    pending: Arc<Mutex<ResponseMap>>,

    seq: Arc<AtomicUsize>,
}
/// TCP relay connection status.
#[derive(Debug, Clone)]
enum ClientStatus {
    /// In this status we are not connected to the relay. This is initial
    /// status. Also we can end up in this status if connection was lost due to
    /// errors.
    Disconnected,
    /// This status means that we are trying to connect to the relay i.e.
    /// establish TCP connection and make a handshake.
    Connecting,
    /// In this status we have established connection to the relay. Note that
    /// when the inner sender is dropped the connection to the relay will be
    /// closed. Also this means that the sender object should not be copied
    /// anywhere else unless you want to keep the connection.
    Connected(mpsc::Sender<Packet>),
    /// This status means that we are not connected to the relay but can
    /// reconnect later. Connection becomes sleeping when all friends that might
    /// use it are connected directly via UDP.
    Sleeping,
}

impl Client {
    /// Create new `Client` object.
    pub fn new(
        addr: SocketAddr,
        client_id: Arc<RwLock<String>>,
        incoming_tx: mpsc::UnboundedSender<(Client, Packet)>,
        //conn_mgr: Arc<Mutex<Option<Connections>>>,
    ) -> Client {
        Client {
            addr,
            client_id,
            incoming_tx,
            status: Arc::new(RwLock::new(ClientStatus::Disconnected)),
            connected_time: Arc::new(RwLock::new(None)),
            connection_attempts: Arc::new(RwLock::new(0)),
            //handle: Arc::new(handle),
            //conn_mgr: Arc::new(Mutex::new(None)),
            pending: Arc::new(Mutex::new(HashMap::new())),
            seq: Arc::new(AtomicUsize::new(1)),
        }
    }
    /// Handle packet received from TCP relay.
    pub async fn handle_packet(&self, packet: Packet) -> Result<(), HandlePacketError> {
        // match packet {
        //     Packet::PingRequest(packet) => self.handle_response(Packet::PingRequest(packet)).await,
        //     Packet::PongResponse(packet) => self.handle_response(Packet::PongResponse(packet)).await,
        //     Packet::ChatMessage(packet) => self.handle_response(Packet::ChatMessage(packet)).await,
        // }

        let mut msg_seq = 0;
        match packet {
            Packet::PingRequest(ref packet) => msg_seq = packet.ping_id,
            Packet::PongResponse(ref packet) => msg_seq = packet.ping_id,
            Packet::ChatMessage(ref packet) => msg_seq = packet.msg_id,
        }

        let mut _pending = self.pending.lock().await;
        if let Some(done_sender) = _pending.remove(&(msg_seq as usize)) {
            if done_sender.send(packet).is_ok() {}
            return Ok(());
        }

        let mut tx = self.incoming_tx.clone();
        tx.send((self.clone(), packet))
            .await
            .map_err(|e| e.context(HandlePacketErrorKind::SendTo).into())
    }
    /// 发送数据包
    pub async fn send_packet(&self, packet: Packet) -> Result<(), SendPacketError> {
        if let ClientStatus::Connected(ref tx) = *self.status.read().await {
            let mut tx = tx.clone();
            tx.send(packet)
                .await
                .map_err(|e| e.context(SendPacketErrorKind::SendTo).into())
        } else {
            // Attempt to send packet to TCP relay with wrong status. For
            // instance it can happen when we received ping request from the
            // relay and right after that relay became sleeping so we are not
            // able to respond anymore.
            Err(SendPacketErrorKind::WrongStatus.into())
        }
    }
    ///异步发送请求  同步返回
    pub async fn send_for_response(&self, packet: Packet) -> Result<ChatMessage, SendPacketError> {
        if let ClientStatus::Connected(ref tx) = *self.status.read().await {
            if let Packet::ChatMessage(mut r) = packet {
                let send_id = self.seq.fetch_add(1, Ordering::Relaxed);

                r.msg_id = send_id as u64;
                let mut tx = tx.clone();
                tx.send(Packet::ChatMessage(r))
                    .await
                    .map_err(|e| e.context(SendPacketErrorKind::NotLinked))?;

                let (done_sender, done) = oneshot::channel::<Packet>();

                // insert sender to pending map
                {
                    let mut _pending = self.pending.lock().await;
                    _pending.insert(send_id, done_sender);
                } //离开作用域会自动释放锁

                match tokio::time::timeout(Duration::from_millis(5 * 1000), done).await {
                    Ok(resp) => match resp {
                        Ok(res) => {
                            if let Packet::ChatMessage(packet) = res {
                                return Ok(packet);
                            }
                        }
                        Err(__) => {}
                    },
                    Err(_) => {
                        return Err(SendPacketErrorKind::TimeOut.into());
                    }
                }
            }
        }
        // Attempt to send packet to TCP relay with wrong status. For
        // instance it can happen when we received ping request from the
        // relay and right after that relay became sleeping so we are not
        // able to respond anymore.
        Err(SendPacketErrorKind::WrongStatus.into())
    }

    /// Spawn a connection to this TCP relay if it is not connected already. The
    /// connection is spawned via `tokio::spawn` so the result future will be
    /// completed after first poll.
    async fn spawn_inner(&mut self) -> Result<(), SpawnError> {
        // TODO: send pings periodically
        match *self.status.write().await {
            ref mut status @ ClientStatus::Disconnected
            | ref mut status @ ClientStatus::Sleeping => *status = ClientStatus::Connecting,
            _ => return Ok(()),
        }
        //println!("socket addr {:#?}", &self.addr);
        let socket = TcpStream::connect(&self.addr)
            .await
            .map_err(|e| e.context(SpawnErrorKind::Io))?;

        let stats = Stats::new();
        let secure_socket = Framed::new(socket, Codec::new(stats));
        let (mut to_server, mut from_server) = secure_socket.split();
        let (to_server_tx, to_server_rx) = mpsc::channel(2);
        match *self.status.write().await {
            ref mut status @ ClientStatus::Connecting => {
                *status = ClientStatus::Connected(to_server_tx)
            }
            _ => return Ok(()),
        }

        *self.connection_attempts.write().await = 0;

        *self.connected_time.write().await = Some(Instant::now());

        let mut to_server_rx = to_server_rx.map(Ok);

        let writer = to_server
            .send_all(&mut to_server_rx)
            .map_err(|e| SpawnError::from(e.context(SpawnErrorKind::Encode)));

        let reader = async {
            while let Some(packet) = from_server.next().await {
                let packet = packet.map_err(|e| e.context(SpawnErrorKind::ReadSocket))?;
                self.handle_packet(packet)
                    .await
                    .map_err(|e| e.context(SpawnErrorKind::HandlePacket))?;
            }

            Result::<(), SpawnError>::Ok(())
        };

        futures::select! {
            res = reader.fuse() => res,
            res = writer.fuse() => res,
        }
    }

    async fn run(&mut self) -> Result<(), SpawnError> {
        let result = self.spawn_inner().await;

        match *self.status.write().await {
            ClientStatus::Sleeping => {}
            ref mut status => *status = ClientStatus::Disconnected,
        }
        if let Err(ref e) = result {
            //error!("TCP relay connection error: {}", e);

            let mut connection_attempts = self.connection_attempts.write().await;
            *connection_attempts = connection_attempts.saturating_add(1);
        }
        *self.connected_time.write().await = None;

        result
    }

    /// Spawn a connection to this TCP relay if it is not connected already. The
    /// connection is spawned via `tokio::spawn` so the result future will be
    /// completed after first poll.
    pub async fn spawn(mut self) -> Result<(), SpawnError> {
        // TODO: send pings periodically

        tokio::spawn(async move { self.run().await });

        Ok(())
    }

    pub fn spawn_lua(self, packet: Packet) -> Result<(), SpawnError> {
        tokio::spawn(async move {
            let paths = std::fs::read_dir("./Plugins").unwrap();
            let lua = Lua::new();
            for path in paths {
                let path = path.unwrap().path();

                if path.extension().unwrap() == "lua" {
                    println!("load plugins {}", &path.display());
                    let mut file = File::open(&path).unwrap();
                    let mut lua_code = String::new();
                    file.read_to_string(&mut lua_code).unwrap();

                    let globals = lua.globals();

                    if let Err(ref e) = lua
                        .load(&lua_code)
                        .set_name(path.to_str().unwrap())
                        .unwrap()
                        .exec()
                    {
                        println!("{}", e);
                        return;
                    }

                    let mut ret: u32 = 0;
                    //let packet = packet.clone();
                    if let Packet::ChatMessage(ref pkg) = packet {
                        let on_chat_msg = globals.get::<_, Function>("OnChatMsg").unwrap();
                        ret = match on_chat_msg
                            .call::<(Client, ChatMessage), u32>((self.clone(), pkg.clone()))
                        {
                            Ok(result) => {
                                println!("  call reutrn {}", result);
                                result
                            }
                            Err(e) => {
                                println!("{}", e);
                                0
                            }
                        }
                    }

                    if let Packet::ChatMessage(ref pkg) = packet {
                        let on_chat_msg = globals.get::<_, Function>("OnChatEvent").unwrap();
                        ret = match on_chat_msg
                            .call::<(Client, ChatMessage), u32>((self.clone(), pkg.clone()))
                        {
                            Ok(result) => {
                                println!("  call reutrn {}", result);
                                result
                            }
                            Err(e) => {
                                println!("{}", e);
                                0
                            }
                        }
                    }

                    if ret == 1 {
                        // 继续执行后续插件
                        continue;
                    }
                    if ret == 2 {
                        // 满足条件后 中断后续插件执行
                        break;
                    }
                }
            }
        });
        Ok(())
    }
    /// Drop connection to the TCP relay if it's connected.
    pub async fn disconnect(&self) {
        // just drop the sink to stop the connection
        *self.status.write().await = ClientStatus::Disconnected;
    }

    /// Drop connection to the TCP relay if it's connected changing status to
    /// `Sleeping`.
    pub async fn sleep(&self) {
        // just drop the sink to stop the connection
        *self.status.write().await = ClientStatus::Sleeping;
    }

    /// Check if TCP connection to the relay is established.
    pub async fn is_connected(&self) -> bool {
        matches!(*self.status.read().await, ClientStatus::Connected(_))
    }

    /// Check if TCP connection to the relay is not established.
    pub async fn is_disconnected(&self) -> bool {
        //println!("{:#?}",*self.status.read().await);
        matches!(*self.status.read().await, ClientStatus::Disconnected)
    }

    /// Check if TCP connection to the relay is sleeping.
    pub async fn is_sleeping(&self) -> bool {
        matches!(*self.status.read().await, ClientStatus::Sleeping)
    }

    /// Number of unsuccessful attempts to establish connection to the relay.
    /// This value is always 0 for successfully connected relays.
    pub async fn connection_attempts(&self) -> u32 {
        *self.connection_attempts.read().await
    }

    /// Time when a connection to the relay was established. Only connected
    /// relays have this value.
    pub async fn connected_time(&self) -> Option<Instant> {
        *self.connected_time.read().await
    }
}

impl UserData for Client {
    //绑定lua API
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_method("prints", |_, this, arg: String| {
            //lua self:prints()
            println!("{:#?}", arg);
            Ok(())
        });

        methods.add_async_method("SendPkg", |_, this, t: Table| async move {
            //lua self:SendPkg(table)
            let from_user_name = t.get::<_, String>("FromUserName")?;
            let to_user_name = t.get::<_, String>("ToUserName")?;
            println!(
                "from_user_name {}  to_user_name {}",
                from_user_name, to_user_name
            );

            Ok(())
        });

        methods.add_function("print", |_, args: String| {
            //lua self.print("hello")
            println!("args is {:#?}", args);
            Ok(())
        });
        methods.add_meta_method(MetaMethod::Index, |ctx, this: &Client, arg: String| {
            let r = match arg.as_str() {
                "clientid" => this.client_id.try_read().unwrap().as_str().to_lua(ctx).ok(), //lua self.Clientid
                _ => {
                    println!("arg {}", arg);
                    None
                }
            };
            Ok(r)
        });
    }
}
