// src/lib.rs

#![allow(unused)]
use std::{collections::HashMap, net::SocketAddr, sync::Arc};

use async_channel::{Receiver, Sender};
use async_dup::Mutex;

pub use crate::inventory::FishBasket;
use crate::server::{FNP, Peer, protocol::OfferBuff};

pub mod inventory;
pub mod server;
pub mod tui;

pub type PeerRegistry = Arc<Mutex<HashMap<String, Peer>>>;

// Removed Join and Leave, added PeerDisconnected.
pub enum Event {
    PeerDisconnected(Peer),
    ServerMessage(server::FNP),
    UIMessage(server::FNP),
    Pesca,
}

pub async fn dispatch(
    host_peer: Peer,
    server_sender: Sender<FNP>,
    fish_catalog: Arc<tui::FishCatalog>,
    fish_basket: Arc<Mutex<FishBasket>>,
    offer_buffers: Arc<Mutex<OfferBuff>>,
    peer_registry: PeerRegistry,
    receiver: Receiver<Event>,
) -> smol::io::Result<()> {
    while let Ok(event) = receiver.recv().await {
        match event {
            Event::PeerDisconnected(peer) => {
                if peer_registry.lock().remove(peer.username()).is_some() {
                    println!("* {} ({}) saiu da rede.", peer.username(), peer.address());
                }
            }
            Event::ServerMessage(fnp) => match fnp {
                server::FNP::AnnounceName { rem } => {
                    let mut registry = peer_registry.lock();
                    if !registry.contains_key(rem.username()) {
                        println!("* {} ({}) se conectou.", rem.username(), rem.address());
                        registry.insert(rem.username().to_string(), rem.clone());

                        let peers = registry.values().cloned().collect();
                        let peer_list_msg = FNP::PeerList {
                            rem: host_peer.clone(),
                            dest: rem,
                            peers,
                        };
                        server_sender.send(peer_list_msg).await.ok();
                    }
                }
                server::FNP::Message { rem, content, .. } => {
                    println!("DM de {}: {}", rem.username(), content);
                }
                server::FNP::Broadcast { rem, content } => {
                    println!("{} - {}", rem.username(), content);
                }
                server::FNP::InventoryInspection { rem, dest } => {
                    let inventory_items: Vec<server::InventoryItem> = fish_basket
                        .lock()
                        .map()
                        .iter()
                        .map(|(k, v)| server::InventoryItem::new(k.to_string(), *v))
                        .collect();

                    let fnp = server::FNP::InventoryShowcase {
                        rem: host_peer.clone(),
                        dest: rem,
                        inventory: server::Inventory {
                            items: inventory_items,
                        },
                    };
                    server_sender.send(fnp).await.ok();
                }
                server::FNP::InventoryShowcase { rem, inventory, .. } => {
                    println!("* [{}]: Inventário", rem.username());
                    println!("{}", inventory);
                }
                server::FNP::TradeOffer { rem, dest, offer } => {
                    offer_buffers
                        .lock()
                        .offers_received
                        .insert(rem.address(), offer.clone());
                    println!("{} quer realizar a seguinte troca:", rem.username());
                    offer
                        .offered
                        .into_iter()
                        .for_each(|f| println!("- {} {}(s)", f.quantity, f.fish_type));
                    println!("por");
                    offer
                        .requested
                        .into_iter()
                        .for_each(|f| println!("- {} {}(s)", f.quantity, f.fish_type));
                    println!(
                        "digite '$c [s]im {}' para aceitar, ou '$c [n]ao {}' para recusar",
                        rem.username(),
                        rem.username()
                    );
                }
                server::FNP::TradeConfirm {
                    rem,
                    dest,
                    response,
                    offer,
                } => {
                    if response {
                        println!("{} aceitou sua oferta de troca!", rem.username());
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
                        println!("{} recusou sua oferta de troca :(", rem.username());
                    }
                    offer_buffers.lock().offers_made.remove(&rem.address());
                }
                server::FNP::PeerList { peers, .. } => {
                    let mut registry = peer_registry.lock();
                    for peer in peers {
                        if !registry.contains_key(peer.username()) {
                            println!(
                                "* Adicionando {} ({}) à lista de peers.",
                                peer.username(),
                                peer.address()
                            );
                            registry.insert(peer.username().to_string(), peer);
                        }
                    }
                }
            },
            Event::UIMessage(fnp) => {
                if fnp
                    .dest()
                    .is_some_and(|v| v.address() == fnp.rem().address())
                {
                    match fnp {
                        FNP::InventoryInspection { .. } => {
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
                        _ => println!("* Essa operação não é válida para você mesmo."),
                    }
                } else {
                    if let FNP::TradeOffer { dest, offer, .. } = fnp.clone() {
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
                    server_sender.send(fnp).await.ok();
                }
            }
            Event::Pesca => {
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
