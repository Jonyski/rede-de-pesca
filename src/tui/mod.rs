mod cli;
mod fisher;


pub use cli::Args;

use std::net::SocketAddr;
use std::net::ToSocketAddrs;

use async_channel::Sender;

pub use fisher::FishCatalog;
pub use fisher::fishing;
use smol::io::AsyncBufReadExt;
use smol::stream::StreamExt;
use smol::Unblock;
use crate::Event;
use crate::server;

pub async fn eval(sender: Sender<Event>, my_addr: SocketAddr) {
    let stdin = Unblock::new(std::io::stdin());
    let mut lines = smol::io::BufReader::new(stdin).lines();

    while let Some(Ok(line)) = lines.next().await {
        
        // TODO: parsear o stdin para todas as mensagens e comandos        
        // Enviando apenas mensagens não vazias
        if !line.trim().is_empty() {
            // executando comandos
            if line.starts_with("$") {
                if line == "$p" || line == "$pescar" {
                    sender.send(Event::Pesca).await.ok();
                }
                else {
                    let parts = line.split_whitespace().collect::<Vec<_>>();
                    if parts[0] == "$i" {
                        if let Some(peer_addr) = parts.get(1) {
                            if let Ok(socket) = peer_addr.parse() {
                                dbg!(socket);
                                sender.send(Event::UIMessage(server::FNP::InventoryInspection {
                                    rem: server::Peer::new(my_addr),
                                    dest: server::Peer::new(socket)
                                })).await.ok();
                            } else {
                                println!("* Invalid peer.");
                            }
                        } else {
                            sender.send(Event::UIMessage(server::FNP::InventoryInspection {
                                rem: server::Peer::new(my_addr),
                                dest: server::Peer::new(my_addr),
                            })).await.ok();

                        }
                    }
                }
            } else {
                let msg = server::FNP::Broadcast { rem: server::protocol::Peer::new(my_addr), content: line};
                sender.send(Event::UIMessage(msg)).await.ok();
            }
        }
    }
}
