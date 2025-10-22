// src/server/mod.rs

#![allow(unused)]
use std::net::{SocketAddr, TcpListener, TcpStream};

use crate::Event;
use async_channel::{Receiver, Sender};
use async_dup::{Arc, Mutex};
use smol::{
    Async,
    io::{AsyncBufReadExt, AsyncWriteExt},
    stream::StreamExt,
};

pub mod protocol;

pub use protocol::FNP;
pub use protocol::Inventory;
pub use protocol::InventoryItem;
pub use protocol::Peer;

type PeerStream = Async<TcpStream>;

pub struct Server {
    host_peer: Peer,
    listener: Async<TcpListener>,
    streams: Arc<Mutex<Vec<Arc<PeerStream>>>>,
}

impl Server {
    pub fn new(username: &str, addr: SocketAddr) -> smol::io::Result<Self> {
        let listener = Async::<TcpListener>::bind(addr)?;
        // After binding, we get the *actual* address from the listener.
        let actual_addr = listener.get_ref().local_addr()?;
        let host_peer = Peer::new(username.to_string(), actual_addr);

        Ok(Self {
            host_peer,
            listener,
            streams: Arc::new(Mutex::new(vec![])),
        })
    }

    // A getter to allow main.rs to retrieve the correct Peer object.
    pub fn host_peer(&self) -> &Peer {
        &self.host_peer
    }

    pub async fn connect_to_many(&self, addrs: &[SocketAddr], sender: Sender<Event>) {
        for peer_addr in addrs {
            let to_connect = *peer_addr;

            let connected = {
                let streams = self.streams.lock();
                streams.iter().any(|stream| {
                    stream
                        .get_ref()
                        .peer_addr()
                        .map_or(false, |addr| addr == to_connect)
                })
            };
            if connected {
                continue;
            }

            if let Ok(stream) = Async::<TcpStream>::connect(to_connect).await {
                println!("* Conexão estabelecida com {}", to_connect);
                let stream_arc = Arc::new(stream);
                self.streams.lock().push(stream_arc.clone());

                let announce_name = FNP::AnnounceName {
                    rem: self.host_peer.clone(),
                };
                let mut stream_clone = stream_arc.clone();
                let network_msg = format!("{}\n", announce_name);
                stream_clone.write_all(network_msg.as_bytes()).await.ok();

                let sender = sender.clone();
                smol::spawn(async move {
                    Self::read_messages_from(stream_arc, sender).await.ok();
                })
                .detach();
            } else {
                eprintln!("* Falha ao conectar com o peer em {}", to_connect);
            }
        }
    }

    pub async fn listen(&self, sender: Sender<Event>) -> smol::io::Result<()> {
        loop {
            let (stream, _addr) = self.listener.accept().await?;

            let connected = {
                let streams = self.streams.lock();
                streams.iter().any(|s| {
                    s.get_ref()
                        .peer_addr()
                        .map_or(false, |addr| addr == _addr)
                })
            };
            if connected {
                continue;
            }

            let peer = Arc::new(stream);
            self.streams.lock().push(peer.clone());

            let sender = sender.clone();
            smol::spawn(async move {
                Self::read_messages_from(peer, sender.clone()).await.ok();
            })
            .detach();
        }
    }

    async fn read_messages_from(
        peer: Arc<Async<TcpStream>>,
        sender: Sender<Event>,
    ) -> smol::io::Result<()> {
        let mut lines = smol::io::BufReader::new(peer).lines();
        while let Some(line) = lines.next().await {
            let line = line?;
            let msg = protocol::FNPParser::parse(&line).map_err(smol::io::Error::other)?;
            sender.send(Event::ServerMessage(msg)).await.ok();
        }
        Ok(())
    }

    pub async fn send_messages(
        &self,
        receiver: Receiver<FNP>,
        dispatcher_sender: Sender<Event>,
    ) -> smol::io::Result<()> {
        while let Ok(msg) = receiver.recv().await {
            match &msg {
                FNP::Broadcast { .. } 
                | FNP::AnnounceName { .. }
                | FNP::PeerList { .. } => {
                    for stream in self.streams.lock().iter() {
                        let msg = msg.clone().set_rem(self.host_peer.clone());
                        let network_msg = format!("{}\n", msg);
                        let mut stream = stream.clone();
                        smol::spawn(async move {
                            stream.write_all(network_msg.as_bytes()).await.ok();
                        })
                        .detach();
                    }
                }
                FNP::Message { dest, .. }
                | FNP::TradeOffer { dest, .. }
                | FNP::InventoryInspection { dest, .. }
                | FNP::InventoryShowcase { dest, .. }
                | FNP::TradeConfirm { dest, .. } => {
                    let msg = msg.clone().set_rem(self.host_peer.clone());
                    let network_msg = format!("{}\n", msg);
                    let dest_addr = dest.address();
                    let dest_user = dest.username().to_string();
                    let dest_peer = dest.clone();
                    let dispatcher_sender = dispatcher_sender.clone();

                    smol::spawn(async move {
                        match Async::<TcpStream>::connect(dest_addr).await {
                            Ok(mut stream) => {
                                if let Err(e) = stream.write_all(network_msg.as_bytes()).await {
                                    eprintln!(
                                        "* Erro ao enviar mensagem para {}: {}",
                                        dest_user, e
                                    );
                                }
                            }
                            Err(e) => {
                                eprintln!(
                                    "* Falha ao conectar com o peer {} ({}): {}",
                                    dest_user, dest_addr, e
                                );
                                // If we fail to connect, send the PeerDisconnected event.
                                dispatcher_sender
                                    .send(Event::PeerDisconnected(dest_peer))
                                    .await
                                    .ok();
                            }
                        }
                    })
                    .detach();
                }
            }
        }
        Ok(())
    }
}
