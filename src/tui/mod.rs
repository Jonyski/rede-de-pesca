mod cli;
mod fisher;

use crate::Event;
use crate::PeerRegistry;
pub use crate::inventory::FishBasket;
use crate::server;
use crate::server::Peer;
use crate::server::protocol::{Offer, OfferBuff};
use async_channel::Sender;
use async_dup::Mutex;
pub use cli::Args;
pub use fisher::FishCatalog;
pub use fisher::fishing;
use owo_colors::{Style, Styled};
use smol::Unblock;
use smol::io::{AsyncBufReadExt, BufReader};
use smol::stream::StreamExt;
use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::Arc;

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
                log("-- PESCADORES ONLINE --".to_string());
                for peer in peer_registry.lock().values() {
                    log(format!("> {} ({})", peer.username(), peer.address()));
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
                        err("Peer não encontrado.".to_string());
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
                    err(
                        "Formato de oferta errado, o correto é:\n $t nome peixe|x,peixe|y,... > peixe|z,peixe|w,...".to_string()
                    );
                    continue;
                }

                if let Some(peer_name) = parts.get(1) {
                    if let Some(peer) = peer_registry.lock().get(*peer_name) {
                        if let Ok(offer) = Offer::from_str(&parts[2..].join(" ")) {
                            let basket = fish_basket.lock();
                            let offers_made = &offer_buffers.lock().offers_made;

                            // First, calculate how many of each fish are tied up in other offers
                            let mut offered_quantities: std::collections::HashMap<String, u32> =
                                std::collections::HashMap::new();
                            for existing_offer in offers_made.values() {
                                for item in &existing_offer.offered {
                                    *offered_quantities
                                        .entry(item.fish_type.clone())
                                        .or_insert(0) += item.quantity;
                                }
                            }

                            let mut is_valid = true;
                            // Now, validate the new offer against the available amount
                            for item_to_offer in &offer.offered {
                                let total_in_inventory = basket
                                    .map()
                                    .get(&item_to_offer.fish_type)
                                    .copied()
                                    .unwrap_or(0);
                                let already_offered = offered_quantities
                                    .get(&item_to_offer.fish_type)
                                    .copied()
                                    .unwrap_or(0);
                                let available = total_in_inventory.saturating_sub(already_offered);

                                if available < item_to_offer.quantity {
                                    err(format!(
                                        "Você não tem peixes suficientes para a troca. (Disponível: {} {})",
                                        available, item_to_offer.fish_type
                                    ));
                                    is_valid = false;
                                    break;
                                }
                            }
                            if is_valid {
                                sender
                                    .send(Event::UIMessage(server::FNP::TradeOffer {
                                        rem: my_peer.clone(),
                                        dest: peer.clone(),
                                        offer,
                                    }))
                                    .await
                                    .ok();
                            }
                        } else {
                            err("Argumentos de oferta inválidos.".to_string());
                        }
                    } else {
                        err("Peer não encontrado.".to_string());
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
                            err("Nenhuma oferta encontrada para este peer.".to_string());
                        }
                    } else {
                        err("Peer não encontrado.".to_string());
                    }
                } else {
                    err("Argumentos inválidos para a confirmação de troca.".to_string());
                }
                continue;
            }
            // Se chegou aqui, o comando ${input} não existe
            err("Este comando não existe".to_string());
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
                    err("Peer não encontrado.".to_string());
                    continue;
                }
            } else {
                err("Formato de mensagem inválido. Use @username <message>".to_string());
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

pub fn log(msg: String) {
    println!("{}", style_log_msg(msg));
}

pub fn err(err_msg: String) {
    println!("{}", style_err_msg(err_msg));
}

pub fn style_log_msg(msg: String) -> String {
    Style::new()
        .fg_rgb::<170, 190, 205>()
        .italic()
        .style(msg)
        .to_string()
}

pub fn style_err_msg(err_msg: String) -> String {
    Style::new()
        .fg_rgb::<220, 40, 80>()
        .italic()
        .style(err_msg)
        .to_string()
}
