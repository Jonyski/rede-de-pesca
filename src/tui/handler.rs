use std::{str::FromStr, sync::Arc};

use async_channel::Sender;

use crate::{server::{self, peerstore::PeerStore, protocol::Offer, Peer}, tui::{commands::Command, err, log}, AppState, Event};


/// Handle para comandos da UI. Trata de forma adequada.
pub async fn handle_command(
    cmd: Command,
    app_state: Arc<AppState>,
    peer_store: Arc<PeerStore>,
    sender: Sender<Event>,
    my_peer: Peer
) {
    match cmd {
        Command::Pescar => {
            sender.send(Event::Pesca).await.ok();
        },
        Command::List => {
            log("-- PESCADORES ONLINE --");
            for peer in peer_store.all_pears().await {
                log(&format!("> {} ({})", peer.username(), peer.address()));
            }
        },
        Command::Inventario(name) =>{
            if let Some(peer_name) = name {
                if let Some(peer_info) = peer_store.get_by_username(&peer_name).await {
                    sender
                        .send(Event::UIMessage(server::FNP::InventoryInspection {
                            rem: my_peer.clone(),
                            dest: peer_info.peer.clone(),
                        }))
                        .await
                        .ok();
                } else {
                    err("Peer não encontrado.");
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
        },
        Command::Trade { peer_str, offer_str } => {
            if peer_str.is_empty() || offer_str.is_empty() {
                err( "Formato de oferta errado, o correto é:\n $t nome peixe|x,peixe|y,... > peixe|z,peixe|w,..." );
                return;
            }

            if let Some(peer_info) = peer_store.get_by_username(&peer_str).await {
                match Offer::from_str(&offer_str) {
                    Ok(parsed_offer) => {
                        // Snapshots do inventario e do buffer, assim não bloqueamos os recursos
                        let basket_snapshot = {
                            let guard = app_state.basket.lock();
                            guard.map().clone()
                        };
                        let offers_made = {
                            let guard = app_state.offer_buffers.lock();
                            guard.offers_made.clone()
                        };

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
                        for item_to_offer in &parsed_offer.offered {
                            let total_in_inventory = basket_snapshot
                                .get(&item_to_offer.fish_type)
                                .copied()
                                .unwrap_or(0);
                            let already_offered = offered_quantities
                                .get(&item_to_offer.fish_type)
                                .copied()
                                .unwrap_or(0);
                            let available = total_in_inventory.saturating_sub(already_offered);

                            if available < item_to_offer.quantity {
                                err(&format!(
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
                                    dest: peer_info.peer.clone(),
                                    offer: parsed_offer,
                                }))
                                .await
                                .ok();
                        }
                    },
                    Err(_) => {
                        err("* Argumentos de oferta inválidos.");
                    }
                }
            } else {
                err("* Peer não encontrado.");
            }
        },
        Command::ConfirmTrade { resp, peer_str } => {
            if let Some(peer_info) = peer_store.get_by_username(&peer_str).await {

                let opt_offer = {
                    let guard = app_state.offer_buffers.lock();
                    guard
                        .offers_received
                        .get(&peer_info.peer.address())
                        .cloned()
                };

                if let Some(offer) = opt_offer {
                    sender
                        .send(Event::UIMessage(server::FNP::TradeConfirm {
                            rem: my_peer.clone(),
                            dest: peer_info.peer.clone(),
                            response: resp,
                            offer: offer.clone(),
                        }))
                        .await
                        .ok();
                } else {
                    err("* Nenhuma oferta encontrada para este peer.");
                }
            } else {
                err("* Peer não encontrado.");
            }
        },
        Command::Quit => {
            log("Encerrando fishnet, boa pescaria...");
            std::process::exit(0);
        },
        Command::Help => {
            log("fishnet 1.0.0");
            log("Options:");
            log("\t anything - Broadcast de mensagens para todos os peers conectados.");
            log("\t @peer - Envia uma mensagem direta para um dado peer.");
            log("\t $[l]istar - Lista todos os peers conectados a você.");
            log("\t $[p]escar - Pesca um peixe aleatorio.");
            log("\t $[i]nventario <peer> - Mostra o inventário do jogador, pode opcionalmente mostrar o inventário de um peer.");
            log("\t $[t]roca <peer> (peixe|quatidade,... > peixe|quantidade,...) - Envia uma oferta de troca para um peer.");
            log("\t $[c]onfirmar <s|n> <peer> - Pedido de confirmação de troca" );
            log("\t $[q]uit - Encerra o programa.");
            log("\t $[h]elp - Mostra essa mensagem de ajuda.");
        },
        Command::Unknown(unk) => {
            err(&format!("Comando ({}) não existe", unk));
        },
    }
}
