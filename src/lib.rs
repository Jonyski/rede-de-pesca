#![allow(unused)]
use std::{net::SocketAddr, sync::Arc};

use async_channel::{Receiver, Sender};
use async_dup::Mutex;

pub use crate::inventory::FishBasket;
use crate::server::{FNP, Peer, protocol::OfferBuff};

pub mod inventory;
pub mod server;
pub mod tui;

pub enum Event {
    Join(SocketAddr),           // new peer connect to us :)
    Leave(SocketAddr),          // peer leave us :(
    ServerMessage(server::FNP), // a peer send a message

    UIMessage(server::FNP), // we send a message/cmd to dispatcher
    Pesca,                  // usuario quer pescar
}

/// Função central de eventos, recebe sinais por channels e envia para outras partes da aplicação, atualizando o estado geral
pub async fn dispatch(
    host_addr: SocketAddr,
    server_sender: Sender<FNP>,
    fish_catalog: Arc<tui::FishCatalog>,
    fish_basket: Arc<Mutex<FishBasket>>,
    offer_buffers: Arc<Mutex<OfferBuff>>,
    receiver: Receiver<Event>,
) -> smol::io::Result<()> {
    while let Ok(event) = receiver.recv().await {
        match event {
            // Alguem entrou na rede
            Event::Join(name) => {
                println!("* {} entrou na rede.", name);
            }
            // Sairam na rede
            Event::Leave(name) => {
                println!("* {} saiu da rede.", name);
            }
            // Mensagens vindo da conexao TCP
            // Nesse caso nosso usuário é o destinatario.
            Event::ServerMessage(fnp) => match fnp {
                server::FNP::Message { rem, content, .. } => {
                    println!("{} te disse: {}", rem, content);
                }
                server::FNP::Broadcast { rem, content } => {
                    println!("{} - {}", rem, content);
                }
                server::FNP::InventoryInspection { rem, dest } => {
                    // Responde uma inspeção com um inventário.
                    // Cria um inventario no formato do protocolo com base no inventario global
                    let inventory_items: Vec<server::InventoryItem> = fish_basket
                        .lock()
                        .map()
                        .iter()
                        .map(|(k, v)| server::InventoryItem::new(k.to_string(), *v))
                        .collect();

                    let fnp = server::FNP::InventoryShowcase {
                        rem: Peer::new(host_addr),
                        dest: rem,
                        inventory: server::Inventory {
                            items: inventory_items,
                        },
                    };
                    server_sender.send(fnp).await.ok();
                }
                server::FNP::InventoryShowcase { rem, inventory, .. } => {
                    println!("* [{}]: Inventário", rem);
                    println!("{}", inventory);
                }
                server::FNP::TradeOffer { rem, dest, offer } => {
                    // Adicionando a oferta para o fim da fila de ofertas recebidas
                    offer_buffers
                        .lock()
                        .offers_received
                        .insert(rem.address(), offer.clone());
                    println!("{rem} quer realizar a seguinte troca:");
                    offer
                        .offered
                        .into_iter()
                        .for_each(|f| println!("- {} {}(s)", f.quantity, f.fish_type));
                    println!("por");
                    offer
                        .requested
                        .into_iter()
                        .for_each(|f| println!("- {} {}(s)", f.quantity, f.fish_type));
                    println!("digite '$c [s]im' para aceitar, ou '$c [n]ao' para recusar");
                }
                server::FNP::TradeConfirm {
                    rem,
                    dest,
                    response,
                    offer,
                } => {
                    if response {
                        println!("{} aceitou sua oferta de troca!", rem);
                        let mut inventory = fish_basket.lock();
                        offer.offered.into_iter().for_each(|f| {
                            println!("voce perdeu {} {}(s)", f.quantity, f.fish_type);
                            inventory
                                .map_mut()
                                .entry(f.fish_type)
                                .and_modify(|i| *i -= f.quantity);
                        });
                        offer.requested.into_iter().for_each(|f| {
                            println!("voce ganhou {} {}(s)", f.quantity, f.fish_type);
                            inventory
                                .map_mut()
                                .entry(f.fish_type)
                                .and_modify(|i| *i += f.quantity);
                        });
                    } else {
                        println!("{} recusou sua oferta de troca :(", rem);
                    }
                    offer_buffers.lock().offers_made.remove(&rem.address());
                }
            },

            // Nesse caso o usuário é o remetente
            Event::UIMessage(fnp) => {
                // Se o protocolo for para o próprio usuário
                if fnp
                    .dest()
                    .is_some_and(|v| v.address() == fnp.rem().address())
                {
                    match fnp {
                        FNP::InventoryInspection { .. } => {
                            // Transforma o fish basket em um inventario do protocolo e mostra na
                            // tela
                            let inventory_items: Vec<server::InventoryItem> = fish_basket
                                .lock()
                                .map()
                                .iter()
                                .map(|(k, v)| server::InventoryItem::new(k.to_string(), *v))
                                .collect();

                            println!(
                                "{}",
                                server::Inventory {
                                    items: inventory_items
                                }
                            );
                        }
                        _ => println!("* Essa operação não é válida para você mesmo."), // Message, Broadcast, TradeOffer, são todos inválidos se mandados
                                                                                        // para o próprio usuário.
                    }
                } else {
                    if let FNP::TradeOffer { dest, offer, .. } = fnp.clone() {
                        // Adicionando a oferta de troca ao buffer de ofertas feitas
                        offer_buffers
                            .lock()
                            .offers_made
                            .insert(dest.address(), offer.clone());
                    }
                    if let FNP::TradeConfirm {
                        rem,
                        dest,
                        response,
                        offer,
                    } = fnp.clone()
                    {
                        if response {
                            println!("-- OFERTA ACEITA --");
                            let mut inventory = fish_basket.lock();
                            offer.offered.into_iter().for_each(|f| {
                                println!("voce ganhou {} {}(s)", f.quantity, f.fish_type);
                                inventory
                                    .map_mut()
                                    .entry(f.fish_type)
                                    .and_modify(|i| *i += f.quantity);
                            });
                            offer.requested.into_iter().for_each(|f| {
                                println!("voce perdeu {} {}(s)", f.quantity, f.fish_type);
                                inventory
                                    .map_mut()
                                    .entry(f.fish_type)
                                    .and_modify(|i| *i -= f.quantity);
                            });
                        } else {
                            println!("-- OFERTA RECUSADA --");
                        }
                        offer_buffers.lock().offers_received.remove(&dest.address());
                    }
                    // senão deixa o servidor cuidar disso
                    server_sender.send(fnp).await.ok();
                }
            }
            Event::Pesca => {
                // pesca um peixe e adiciona/incrementa ao inventario
                let fish = crate::tui::fishing(&fish_catalog);
                fish_basket
                    .lock()
                    .map_mut()
                    .entry(fish.clone())
                    .and_modify(|f| *f += 1)
                    .or_insert(1);
                println!("Você pescou um(a) {}!", fish);
            }
        }
    }

    Ok(())
}
