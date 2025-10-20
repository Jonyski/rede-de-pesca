mod cli;
mod fisher;

use crate::PeerRegistry;
use crate::server::protocol::{Offer, OfferBuff};
pub use cli::Args;
use std::sync::Arc;

use crate::Event;
pub use crate::inventory::FishBasket;
use crate::server;
use crate::server::Peer;
use async_channel::Sender;
use async_dup::Mutex;
pub use fisher::FishCatalog;
pub use fisher::fishing;
use smol::Unblock;
use smol::io::{AsyncBufReadExt, BufReader};
use smol::stream::StreamExt;
use std::net::SocketAddr;
use std::str::FromStr;

/// Loop para a interface do usuário, aguarda entradas de texto e emite sinais de acordo.
pub async fn eval(
    sender: Sender<Event>,
    my_peer: Peer,
    offer_buffers: Arc<Mutex<OfferBuff>>,
    peer_registry: PeerRegistry,
    fish_basket: Arc<Mutex<FishBasket>>,
) {
    let stdin = Unblock::new(std::io::stdin());
    let mut lines = BufReader::new(stdin).lines();

    while let Some(Ok(line)) = lines.next().await {
        // Ignorando mensagens vazias
        if line.trim().is_empty() {
            continue;
        }

        // Executando comandos
        if line.starts_with('$') {
            let mut parts = line.split_whitespace().collect::<Vec<_>>();
            // Pesca e listagem de Peers
            if parts[0].to_lowercase() == "$p" || parts[0].to_lowercase() == "$pescar" {
                sender.send(Event::Pesca).await.ok();
                continue;
            } else if parts[0].to_lowercase() == "$l" || parts[0].to_lowercase() == "$listar" {
                println!("* Peers conectados:");
                for peer in peer_registry.lock().values() {
                    println!("- {} ({})", peer.username(), peer.address());
                }
                continue;
            }
            // Inspeção de inventário
            if parts[0].to_lowercase() == "$i" || parts[0].to_lowercase() == "$inventario" {
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
                    // Se não há um peer como argumento, inspeciona o próprio inventário
                    sender
                        .send(Event::UIMessage(server::FNP::InventoryInspection {
                            rem: my_peer.clone(),
                            dest: my_peer.clone(),
                        }))
                        .await
                        .ok();
                }
                continue;
            }
            // Oferta de troca de peixe
            if parts[0].to_lowercase() == "$t" || parts[0].to_lowercase() == "$troca" {
                // Uma troca tem que ter 5 partes
                if parts.len() < 5 {
                    println!(
                        "* Formato de oferta errado, o correto é:\n $t nome peixe|x,peixe|y,... > peixe|z,peixe|w,..."
                    );
                    continue;
                }

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
                            println!("* Argumentos de oferta inválidos.");
                        }
                    } else {
                        println!("* Peer não encontrado.");
                    }
                }
                continue;
            }
            // Confirmação de troca
            if parts[0].to_lowercase() == "$c" || parts[0].to_lowercase() == "$confirmar" {
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
                continue;
            }
            // Se chegou aqui, o comando ${input} não existe
            println!("* Este comando não existe");
            continue;
        }
        // Mensagens normais (DMs e broadcasts)
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
        // Enviando a mensagem para o servidor
        sender.send(Event::UIMessage(msg)).await.ok();
    }
}
