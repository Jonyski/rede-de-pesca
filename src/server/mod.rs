#![allow(unused)]
use std::net::{SocketAddr, TcpListener, TcpStream};

use async_channel::{Receiver, Sender};
use async_dup::{Arc, Mutex};
use smol::{io::{AsyncBufReadExt, AsyncWriteExt}, stream::StreamExt, Async};
use crate::Event;

pub mod protocol;

pub use protocol::FNP;
pub use protocol::Inventory;
pub use protocol::InventoryItem;
pub use protocol::Peer;

type PeerStream = Async<TcpStream>;


pub struct Server {
    hostname: Box<str>,
    listener: Async<TcpListener>,
    streams: Arc<Mutex<Vec<Arc<PeerStream>>>>
}


impl Server {

    pub fn new(hostname: &str, addr: SocketAddr) -> smol::io::Result<Self> {
        let listener = Async::<TcpListener>::bind(addr)?;

        Ok(Self {
            hostname: Box::from(hostname),
            listener,
            streams: Arc::new(Mutex::new(vec![])),
        })
    }

    pub async fn connect_to_many(&self, addrs: &[SocketAddr], sender: Sender<Event>) {
        for peer_addr in addrs {
            if let Ok(stream) = Async::<TcpStream>::connect(*peer_addr).await{
                let stream_arc = Arc::new(stream);
                self.streams.lock().push(stream_arc.clone());
                let sender = sender.clone();
                smol::spawn(async move {
                    Self::read_messages_from(stream_arc, sender).await.ok();
                }).detach();
            }
        }
    }

    pub async fn listen(&self, sender: Sender<Event>) -> smol::io::Result<()> {

        loop {
            let (stream, addr) = self.listener.accept().await?;
            let peer = Arc::new(stream);
            // TODO: ler o nome de usário do novo peer conectado

            // Adiciona na lista de peers
            self.streams.lock().push(peer.clone());

            let sender = sender.clone();
            smol::spawn(async move {
                sender.send(Event::Join(addr)).await.ok();
                Self::read_messages_from(peer, sender.clone()).await
                    .unwrap_or_else(|err| {eprintln!("{}", err); ()});
                sender.send(Event::Leave(addr)).await.ok();
            }).detach()
        }
    }

    async fn read_peername() -> Box<str> {
        todo!()
    }

    async fn read_messages_from(peer: Arc<Async<TcpStream>>, sender: Sender<Event>) -> smol::io::Result<()> {
        let mut lines = smol::io::BufReader::new(peer).lines();
        // Toda vez que houver uma nova linha na stream, manda a mensagem pro dispatcher
        while let Some(line) = lines.next().await {
            let line = line?;
            let msg = protocol::FNPParser::parse(&line).map_err(|e| smol::io::Error::other(e))?;
            sender.send(Event::ServerMessage(msg)).await.ok();
        }
        Ok(())
    }

    // Recebe mensagens em FNP produzidas pelo dispatcher através de um channel e processa de acordo.
    pub async fn send_messages(&self, receiver: Receiver<FNP>) -> smol::io::Result<()> {
        while let Ok(msg) = receiver.recv().await {
            match &msg {
                FNP::Broadcast { .. } => {
                    for stream in self.streams.lock().iter() {
                        // Altera o remetende para o IP que o peer destinatario esta se comunicando
                        let msg = msg.clone().set_rem(Peer::new(stream.get_ref().local_addr().unwrap()));

                        let network_msg = format!("{}\n", msg);
                        let mut stream = stream.clone();
                        smol::spawn(async move {
                            stream.write_all(network_msg.as_bytes()).await.ok();
                        }).detach();
                    }
                },
                // Envia uma mensagem do protocolo para um peer específico
                FNP::Message { dest, .. } |
                FNP::TradeOffer { dest, .. } |
                FNP::InventoryInspection { dest, .. } |
                FNP::InventoryShowcase { dest, .. } |
                FNP::TradeConfirm { dest, .. } => {
                    let mut stream = self.streams.lock().iter()
                        .find(|s| s.get_ref().peer_addr().map_or(false, |addr| addr == dest.address()))
                        .ok_or(smol::io::Error::other("Peer not found"))?
                        .clone();

                    // Altera o remetente da mensagem para o endereço que ESTE peer está escutando/esperando a resposta
                    let msg = msg.set_rem(Peer::new(stream.get_ref().local_addr().unwrap()));
                    let network_msg = format!("{}\n", msg);
                    stream.write_all(network_msg.as_bytes()).await?;
                }
            }
        }

        Ok(())
    }
}
