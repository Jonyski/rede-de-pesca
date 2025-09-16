use std::net::{SocketAddr, TcpListener};

use async_channel::Sender;
use async_dup::Arc;
use std::sync::Mutex;
use smol::{io, Async};
use std::net::TcpStream;

use crate::event_system::{Event, read_messages};


type PeerStream = Arc<Async<TcpStream>>;

pub struct PeerServer {
    listener: Async<TcpListener>,
    streams: Arc<Mutex<Vec<PeerStream>>>
}


impl PeerServer {
    
    pub fn new(addr: SocketAddr) -> smol::io::Result<Self> {
        
        let listener = Async::<TcpListener>::bind(addr)?;

        Ok(Self {
            listener,
            streams: Arc::new(Mutex::new(vec![])),
        })
    }

    pub fn local_addr(&self) -> SocketAddr {
        self.listener.get_ref().local_addr().expect("Expected to have a valid local address.")
    }

    pub fn streams(&self) -> Arc<Mutex<Vec<PeerStream>>> {
        self.streams.clone()
    }

    pub async fn listen(&self, sender: Sender<Event>) -> io::Result<()> {
        loop {
            // Aceitando novas conexões em loop
            let (stream, addr) = self.listener.accept().await?;
            let peer = Arc::new(stream);
            let sender = sender.clone();

            // Criando a thread que lida com o novo peer
            smol::spawn(async move {
                sender.send(Event::Join(addr, peer.clone())).await.ok();
                read_messages(sender.clone(), peer).await.ok();
                sender.send(Event::Leave(addr)).await.ok();
            })
            .detach()
        }
    }

    pub async fn connect_to_many(&mut self, addrs: &[SocketAddr], sender: Sender<Event>) {
        // Tentando conectar com os peers que foram passados como argumento
        for peer_addr in addrs {
            if let Ok(stream) = Async::<TcpStream>::connect(*peer_addr).await {
                let stream_arc = Arc::new(stream);
                self.streams.lock().expect("Streams should be free to add new streams").push(stream_arc.clone());
                let sender_clone = sender.clone();
                // Spawnando uma nova thread que lê mensagens enviadas por aquele peer específico
                smol::spawn(async move {
                    read_messages(sender_clone, stream_arc).await.ok();
                })
                .detach();
            }
        }
    }
}
