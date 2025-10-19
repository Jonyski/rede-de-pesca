mod cli;
mod fisher;

use crate::PeerRegistry;
use crate::server::protocol::{Offer, OfferBuff};
pub use cli::Args;
use std::sync::Arc;

use crate::server::Peer;
use async_channel::Sender;
use async_dup::Mutex;
use std::net::SocketAddr;
use std::str::FromStr;

use crate::Event;
use crate::server;
pub use fisher::FishCatalog;
pub use fisher::fishing;
use smol::Unblock;
use smol::io::{AsyncBufReadExt, BufReader};
use smol::stream::StreamExt;

/// Loop para a interface do usuário, aguarda entradas de texto e emite sinais de acordo.
pub async fn eval(
    sender: Sender<Event>,
    my_peer: Peer,
    offer_buffers: Arc<Mutex<OfferBuff>>,
    peer_registry: PeerRegistry,
) {
    let stdin = Unblock::new(std::io::stdin());
    let mut lines = BufReader::new(stdin).lines();

    while let Some(Ok(line)) = lines.next().await {
        if !line.trim().is_empty() {
            // executando comandos
            if line.starts_with('$') {
                if line == "$p" || line == "$pescar" {
                    sender.send(Event::Pesca).await.ok();
                } else if line == "$l" || line == "$listar" {
                    println!("* Peers conectados:");
                    for peer in peer_registry.lock().values() {
                        println!("- {} ({})", peer.username(), peer.address());
                    }
                } else {
                    let parts = line.split_whitespace().collect::<Vec<_>>();
                    if parts[0] == "$i" {
                        if let Some(peer_name) = parts.get(1) {
                            if let Some(peer) = peer_registry.lock().get(*peer_name) {
                                sender
                                    .send(Event::UIMessage(server::FNP::InventoryInspection {
                                        rem: my_peer.clone(),
                                        dest: peer.clone(),
                                    }))
                                    .await
                                    .ok();
                            } else {
                                println!("* Peer não encontrado.");
                            }
                        } else {
                            sender
                                .send(Event::UIMessage(server::FNP::InventoryInspection {
                                    rem: my_peer.clone(),
                                    dest: my_peer.clone(),
                                }))
                                .await
                                .ok();
                        }
                    } else if parts[0] == "$t" {
                        if let Some(peer_name) = parts.get(1) {
                            if let Some(peer) = peer_registry.lock().get(*peer_name) {
                                if let Ok(offer) = Offer::from_str(&parts[2..].join(" ")) {
                                    sender
                                        .send(Event::UIMessage(server::FNP::TradeOffer {
                                            rem: my_peer.clone(),
                                            dest: peer.clone(),
                                            offer,
                                        }))
                                        .await
                                        .ok();
                                } else {
                                    println!("* Formato de oferta inválido.");
                                }
                            } else {
                                println!("* Peer não encontrado.");
                            }
                        } else {
                            println!("* Argumentos inválidos para a oferta de troca...");
                        }
                    } else if parts[0] == "$c" {
                        if let (Some(response), Some(peer_name)) = (parts.get(1), parts.get(2)) {
                            if let Some(peer) = peer_registry.lock().get(*peer_name) {
                                if let Some(offer) =
                                    offer_buffers.lock().offers_received.get(&peer.address())
                                {
                                    let response = *response == "s" || *response == "sim";
                                    sender
                                        .send(Event::UIMessage(server::FNP::TradeConfirm {
                                            rem: my_peer.clone(),
                                            dest: peer.clone(),
                                            response,
                                            offer: offer.clone(),
                                        }))
                                        .await
                                        .ok();
                                } else {
                                    println!("* Nenhuma oferta encontrada para este peer.");
                                }
                            } else {
                                println!("* Peer não encontrado.");
                            }
                        } else {
                            println!("* Argumentos inválidos para a confirmação de troca.");
                        }
                    }
                }
            } else {
                let msg = if line.starts_with('@') {
                    if let Some((peer_name, text)) = line.split_once(' ') {
                        let peer_name = peer_name.strip_prefix('@').unwrap_or(peer_name);
                        if let Some(peer) = peer_registry.lock().get(peer_name) {
                            server::FNP::Message {
                                rem: my_peer.clone(),
                                dest: peer.clone(),
                                content: text.to_string(),
                            }
                        } else {
                            println!("* Peer não encontrado.");
                            continue;
                        }
                    } else {
                        println!("* Formato de mensagem inválido. Use @username <message>");
                        continue;
                    }
                } else {
                    server::FNP::Broadcast {
                        rem: my_peer.clone(),
                        content: line,
                    }
                };
                sender.send(Event::UIMessage(msg)).await.ok();
            }
        }
    }
}
