mod cli;
mod fisher;

pub use crate::server::protocol::Offer;
use crate::server::protocol::OfferBuff;
use async_dup::Mutex;
pub use cli::Args;
use std::sync::Arc;

use async_channel::Sender;
use std::net::SocketAddr;
use std::str::FromStr;

use crate::Event;
use crate::server;
pub use fisher::FishCatalog;
pub use fisher::fishing;
use smol::Unblock;
use smol::io::AsyncBufReadExt;
use smol::stream::StreamExt;

/// Loop para a interface do usuário, aguarda entradas de texto e emite sinais de acordo.
pub async fn eval(
    sender: Sender<Event>,
    my_addr: SocketAddr,
    offer_buffers: Arc<Mutex<OfferBuff>>,
) {
    let stdin = Unblock::new(std::io::stdin());
    let mut lines = smol::io::BufReader::new(stdin).lines();

    while let Some(Ok(line)) = lines.next().await {
        if !line.trim().is_empty() {
            // executando comandos
            if line.starts_with("$") {
                if line == "$p" || line == "$pescar" {
                    sender.send(Event::Pesca).await.ok();
                } else {
                    let parts = line.split_whitespace().collect::<Vec<_>>();
                    if parts[0] == "$i" {
                        if let Some(peer_addr) = parts.get(1) {
                            if let Ok(socket) = peer_addr.parse() {
                                sender
                                    .send(Event::UIMessage(server::FNP::InventoryInspection {
                                        rem: server::Peer::new(my_addr),
                                        dest: server::Peer::new(socket),
                                    }))
                                    .await
                                    .ok();
                            } else {
                                println!("* Invalid peer.");
                            }
                        } else {
                            sender
                                .send(Event::UIMessage(server::FNP::InventoryInspection {
                                    rem: server::Peer::new(my_addr),
                                    dest: server::Peer::new(my_addr),
                                }))
                                .await
                                .ok();
                        }
                    } else if parts[0] == "$t" {
                        if let Some(peer_addr) = parts.get(1)
                            && let Ok(socket) = peer_addr.parse()
                            && let Ok(offer) = Offer::from_str(&parts[2..=4].join(" "))
                        {
                            sender
                                .send(Event::UIMessage(server::FNP::TradeOffer {
                                    rem: server::Peer::new(my_addr),
                                    dest: server::Peer::new(socket),
                                    offer,
                                }))
                                .await
                                .ok();
                        } else {
                            println!("* Invalid arguments for trade offer...");
                        }
                    } else if parts[0] == "$c"
                        && let Some(peer_addr) = parts.get(2)
                        && let Ok(socket) = peer_addr.parse()
                    {
                        let offer = offer_buffers
                            .lock()
                            .offers_received
                            .get(&socket)
                            .unwrap()
                            .clone();
                        if parts[1] == "s" || parts[1] == "sim" {
                            sender
                                .send(Event::UIMessage(server::FNP::TradeConfirm {
                                    rem: server::Peer::new(my_addr),
                                    dest: server::Peer::new(socket),
                                    response: true,
                                    offer,
                                }))
                                .await
                                .ok();
                        } else if parts[1] == "n" || parts[1] == "nao" {
                            sender
                                .send(Event::UIMessage(server::FNP::TradeConfirm {
                                    rem: server::Peer::new(my_addr),
                                    dest: server::Peer::new(socket),
                                    response: false,
                                    offer,
                                }))
                                .await
                                .ok();
                        } else {
                            println!("* Invalid offer response");
                        }
                    }
                }
            } else {
                let msg = if line.starts_with("@")
                    && let Some((peer_addr, text)) = line.split_once(" ")
                    && let Some(strip_addr) = peer_addr.strip_prefix("@")
                    && let Ok(addr) = strip_addr.parse()
                {
                    server::FNP::Message {
                        rem: server::Peer::new(my_addr),
                        dest: server::Peer::new(addr),
                        content: text.to_string(),
                    }
                } else {
                    server::FNP::Broadcast {
                        rem: server::protocol::Peer::new(my_addr),
                        content: line,
                    }
                };
                sender.send(Event::UIMessage(msg)).await.ok();
            }
        }
    }
}
