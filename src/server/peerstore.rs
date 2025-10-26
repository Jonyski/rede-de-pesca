use crate::{Event, FNP, server::protocol::FNPParser};
use async_channel::Sender;
use async_dup::Mutex;
use smol::{
    Async,
    io::{AsyncBufReadExt, AsyncWriteExt},
    stream::StreamExt,
};
use std::{
    collections::HashMap,
    fmt::Display,
    net::{SocketAddr, TcpStream},
    str::FromStr,
    sync::Arc,
};

pub struct PeerStore {
    // Mapeia endereço de escuta para peers
    listener_map: Mutex<HashMap<SocketAddr, PeerInfo>>,
    // Mapeia o endereço de cliente para o endereço de listener.
    client_to_listener_map: Mutex<HashMap<SocketAddr, SocketAddr>>,
    // Mapeia o nome dos peers ao seus endereços de listener.
    name_addr_map: Mutex<HashMap<Box<str>, SocketAddr>>,
}

#[derive(Clone, Debug)]
pub struct PeerInfo {
    pub peer: Peer,
    pub client_addr: SocketAddr,
    pub conn: Arc<Connection>,
}

/// Peer que representa um username e um endereço de socket com o prefixo fnp://
#[derive(Debug, PartialEq, Clone)]
pub struct Peer {
    username: String,
    address: SocketAddr,
}

/// Representa uma conexão TCP entre dois peers
#[derive(Debug)]
pub struct Connection {
    stream: Async<TcpStream>,
}

impl PeerStore {
    pub fn new() -> Self {
        Self {
            listener_map: Mutex::new(HashMap::new()),
            client_to_listener_map: Mutex::new(HashMap::new()),
            name_addr_map: Mutex::new(HashMap::new()),
        }
    }

    /// Registra o peer, seu endereço de cliente e sua conexão
    pub async fn register(&self, peer: Peer, client_addr: SocketAddr, conn: Arc<Connection>) {
        let listener = peer.address();
        let info = PeerInfo {
            peer: peer.clone(),
            client_addr,
            conn,
        };
        self.listener_map.lock().insert(listener, info);
        self.client_to_listener_map
            .lock()
            .insert(client_addr, listener);
        self.name_addr_map
            .lock()
            .insert(peer.username().into(), listener);
    }

    /// Remove informações do peer através do seu endereço de cliente.
    pub async fn unregister_by_client(&self, client_addr: &SocketAddr) -> Option<PeerInfo> {
        if let Some(listener) = self.client_to_listener_map.lock().remove(client_addr) {
            let peer_info = self.listener_map.lock().remove(&listener);
            if let Some(ref i) = peer_info {
                self.name_addr_map.lock().remove(i.peer.username());
            }
            return peer_info;
        }
        None
    }

    pub async fn unregister_by_username(&self, username: &str) -> Option<PeerInfo> {
        if let Some(client_addr) = self.name_addr_map.lock().remove(username)
            && let Some(listener) = self.client_to_listener_map.lock().remove(&client_addr)
        {
            return self.listener_map.lock().remove(&listener);
        }
        None
    }

    /// Retorna a informação de um peer com base no seu endereço de escuta, se houver.
    pub async fn get_by_listener(&self, listener: &SocketAddr) -> Option<PeerInfo> {
        self.listener_map.lock().get(listener).cloned()
    }

    /// Retorna informações do peer com base no seu username.
    pub async fn get_by_username(&self, username: &str) -> Option<PeerInfo> {
        if let Some(listener) = self.name_addr_map.lock().get(username) {
            self.get_by_listener(listener).await
        } else {
            None
        }
    }

    pub async fn all_pears(&self) -> Vec<Peer> {
        self.listener_map
            .lock()
            .values()
            .map(|i| &i.peer)
            .cloned()
            .collect()
    }

    /// Envia uma mensagem a todos os peer registrados
    pub async fn broadcast(&self, host: Peer, msg: FNP) {
        let conns: Vec<Arc<Connection>> = self
            .listener_map
            .lock()
            .values()
            .map(|i| i.conn.clone())
            .collect();
        let m = msg.set_rem(host);
        for c in conns {
            c.send_fnp(&m).await.ok();
        }
    }

    /// Envia uma pensagem para um peer através do seu endereço de escuta
    pub async fn send_through_listener(&self, host: Peer, listener: &SocketAddr, msg: FNP) {
        if let Some(info) = self.get_by_listener(listener).await {
            let m = msg.set_rem(host);
            info.conn.send_fnp(&m).await.ok();
        }
    }
}

impl Connection {
    pub fn new(stream: Async<TcpStream>) -> Self {
        Self { stream }
    }

    pub fn stream(&self) -> &Async<TcpStream> {
        &self.stream
    }

    pub async fn send_fnp(&self, msg: &FNP) -> smol::io::Result<()> {
        let s = format!("{}\n", msg);
        self.stream().write_all(s.as_bytes()).await
    }

    pub async fn start_reader(
        self: Arc<Self>,
        sender: Sender<Event>,
        // shutdown receiver
    ) {
        let peer_addr = self
            .stream()
            .get_ref()
            .peer_addr()
            .expect("Peer address deveria estar acessível.");
        let mut lines = smol::io::BufReader::new(self.stream()).lines();
        while let Some(Ok(line)) = lines.next().await {
            if let Ok(msg) = FNPParser::parse(&line) {
                sender.send(Event::ServerMessage(msg, peer_addr)).await.ok();
            } else {
                dbg!(line);
            }
        }
    }
}

impl Peer {
    pub fn new(username: String, address: SocketAddr) -> Self {
        Self { username, address }
    }

    pub fn address(&self) -> SocketAddr {
        self.address
    }

    pub fn username(&self) -> &str {
        &self.username
    }
}

impl FromStr for Peer {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let sanitized = match s.strip_prefix("fnp://") {
            Some(striped) => striped,
            None => s,
        };

        let parts: Vec<&str> = sanitized.split('@').collect();
        if parts.len() != 2 {
            return Err("Invalid peer format".to_string());
        }

        let username = parts[0].to_string();
        let address = parts[1].parse::<SocketAddr>().map_err(|e| e.to_string())?;
        Ok(Self { username, address })
    }
}

impl Display for Peer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "fnp://{}@{}", self.username, self.address)
    }
}
