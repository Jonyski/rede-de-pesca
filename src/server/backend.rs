use async_channel::{Receiver, Sender};
use async_dup::Mutex;
use smol::Async;
use std::{collections::HashMap, net::{self, SocketAddr, TcpStream}, sync::Arc};

use crate::{server::{peerstore::{Connection, PeerStore}, Peer, FNP}, Event};


/// Servidor/Cliente da rede p2p
pub struct ServerBackend {
    host: Peer,
    listener: Async<net::TcpListener>,
    connections: Arc<Mutex<HashMap<net::SocketAddr, Arc<Connection>>>>,

    peer_store: Arc<PeerStore>
}

impl ServerBackend {
    /// Cria um novo servidor/backend com um nome de usuário e um endereço (Algo que possa
    /// ser um endereço). Se uma lista de endereços for passada, o código se conecta com
    ///  o primeiro endereço bem sucedido.
    // NOTE: `impl Into<net::SocketAddr>` nos permite aceitar como parametro qualquer valor que
    // possa ser transformado em um socketaddr
    pub fn new(username: &str, addr: impl Into<net::SocketAddr>) -> smol::io::Result<Self> {
        let listener = Async::<net::TcpListener>::bind(addr)?;
        // guardamos o endereço de listener
        let actual_addr = listener.get_ref().local_addr()?;
        let host_peer = Peer::new(username.to_string(), actual_addr);

        Ok(Self {
            host: host_peer,
            listener,
            connections: Arc::new(Mutex::new(HashMap::new())),
            peer_store: Arc::new(PeerStore::new())
        })
    }

    pub fn host(&self) -> Peer {
        self.host.clone()
    }

    pub async fn run(
        self: Arc<Self>,
        msg_receiver: Receiver<FNP>,
        event_sender: Sender<Event>,
        // add shutdown recv
    ) -> smol::io::Result<()> {
        let server_clone = self.clone();

        let listener_sender = event_sender.clone();
        let listener_task = smol::spawn(async move {
            server_clone.listen(listener_sender).await
        });

        let server_clone2 = self.clone();
        let sendout_task = smol::spawn(async move {
            server_clone2.sendout(&msg_receiver).await
        });

        // shut down
        // TODO: implement shutdown waitter
        // self.shutdown().await;

        listener_task.await?;
        sendout_task.await?;
        Ok(())
    }

    /// Recebe mensagens por um channel e as envia para outros peers
    async fn sendout(&self, msg_recv: &Receiver<FNP>) -> smol::io::Result<()> {
        while let Ok(msg) = msg_recv.recv().await {
            match msg {
                // mensagens gerais
                FNP::AnnounceName { .. } | FNP::Broadcast { .. } | FNP::PeerList { .. } => {
                    self.peer_store().broadcast(self.host(), msg).await;
                }
                // mensagens diretas
                _ => {
                    let dest_addr = msg.dest()
                        .expect("Protocolo gerado errado, mensages diretas devem ter um destinatário.")
                        .address();
                    if let Some(info) = self.peer_store().get_by_listener(&dest_addr).await {
                        info.conn.send_fnp(&msg).await.ok();
                    } else {
                        crate::tui::err(&format!("* Peer {} não encontrado na sua rede.", dest_addr));
                    }
                }
            }
        }
        Ok(())
    }

    /// Escuta por novas conexões e envia eventos por um canal
    async fn listen(&self, sender: Sender<Event>) -> smol::io::Result<()> {
        loop {
            let (stream, addr) = self.listener.accept().await?;
            self.handle_connection(addr, stream, sender.clone()).await?;
        }
    }

    /// Conecta-se a um peer com base no endereço, armazena a nova conexão em connections
    pub async fn connect(&self, addr: impl Into<net::SocketAddr>, sender: Sender<Event>)
    -> smol::io::Result<()> {
        let addr = addr.into();
        let stream = Async::<TcpStream>::connect(addr).await?;

        self.handle_connection(addr, stream, sender).await?;
        Ok(())
    }

    /// Tenta se conectar com a lista de peers.
     pub async fn connect_to_many(&self, peers: &[net::SocketAddr], sender: Sender<Event> ) {
         for peer_addr in peers {
             self.connect(*peer_addr, sender.clone()).await.ok();
         }
     }

     /// Hadler de conexões, adiciona a lista de conexões e cria nova task para ler
     ///  mensagens e enviar a um channel
     async fn handle_connection(&self,
         addr: net::SocketAddr,
         stream: Async<TcpStream>,
         sender: Sender<Event>
     ) -> smol::io::Result<()> {
         let conn = Arc::new(Connection::new(stream));
         self.connections.lock().insert(addr, conn.clone());

         let announce_name = FNP::AnnounceName {rem: self.host()};
         let conn_cl = &conn.clone();
         conn_cl.send_fnp(&announce_name).await.ok();

         let reader_sender = sender.clone();
         let conn_cl2 = conn.clone();
         smol::spawn(async move {
             conn_cl2.start_reader(reader_sender).await;
             sender.send(Event::PeerDisconnected(addr)).await.ok();
         }).detach();

         Ok(())
     }

    pub async fn register_peer(&self, peer: Peer, client_addr: SocketAddr) {
         if let Some(conn) = self.connections.lock().get(&client_addr).cloned() {
             self.peer_store.register(peer, client_addr, conn).await;
         } else {
             crate::tui::err(&format!("* Conexão perdida com {}", client_addr));
         }
     }

     pub fn peer_store(&self) -> Arc<PeerStore> {
         self.peer_store.clone()
     }

     pub fn connections(&self) -> Arc<Mutex<HashMap<net::SocketAddr, Arc<Connection>>>> {
         self.connections.clone()
     }

    /// Encerra o servidor fechando todas as conexões abertas.
    /// Faz isso apenas quando conexões está livre
    async fn _shutdown(&self) {
        // Limpa todas as conexões
        self.connections.lock().clear();
    }
}
