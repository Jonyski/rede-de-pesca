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

/// Os 4 tipos de eventos com os quais o dispatcher lida
pub enum Event {
    /// Foi percebido que um peer saiu da rede
    PeerDisconnected(Peer),
    /// Mensagem FNP chegando de um peer
    ServerMessage(server::FNP),
    /// Mensagem FNP chegando do próprio peer para ser enviada a outro(s)
    UIMessage(server::FNP),
    /// O peer está tentando pescar
    Pesca,
}

/// Recebe todos os tipos de Eventos e realiza a ação/efeito colateral de cada um
pub async fn dispatch(
    server: Arc<crate::server::Server>,
    esender: Sender<Event>,
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
                    // Anúncio de nome e conexão, atualiza o registro de peers
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
                    println!("* Inventário de {}", rem.username());
                    if inventory.items.is_empty() {
                        println!("[Inventário vazio]");
                    } else {
                        // Style the inventory for display
                        for item in &inventory.items {
                            let style = fish_catalog.get_style_for_fish(&item.fish_type);
                            println!("  - {} {}(s)", item.quantity, style.style(&item.fish_type));
                        }
                    }
                }
                server::FNP::TradeOffer { rem, dest, offer } => {
                    // Adicionando ao buffer de ofertas recebidas
                    offer_buffers
                        .lock()
                        .offers_received
                        .insert(rem.address(), offer.clone());
                    // Exibindo os peixes ofertados e requisitados pelo remetente
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
                        println!("* {} aceitou sua oferta de troca :)", rem.username());
                        let mut inventory = fish_basket.lock();
                        // Removendo os peixes que você deu
                        for item in offer.offered {
                            let style = fish_catalog.get_style_for_fish(&item.fish_type);
                            println!(
                                "* Você perdeu {} {}(s)",
                                item.quantity,
                                style.style(&item.fish_type)
                            );
                            if let Some(count) = inventory.map_mut().get_mut(&item.fish_type) {
                                *count -= item.quantity;
                                if *count == 0 {
                                    inventory.map_mut().remove(&item.fish_type);
                                }
                            }
                        }
                        // Adicionando os peixes que você recebeu
                        for item in offer.requested {
                            let style = fish_catalog.get_style_for_fish(&item.fish_type);
                            println!(
                                "* Você ganhou {} {}(s)",
                                item.quantity,
                                style.style(&item.fish_type)
                            );
                            *inventory.map_mut().entry(item.fish_type).or_insert(0) +=
                                item.quantity;
                        }
                    } else {
                        println!("* {} recusou sua oferta de troca :(", rem.username());
                    }
                    offer_buffers.lock().offers_made.remove(&rem.address());
                }
                server::FNP::PeerList { peers, .. } => { 
                    let mut to_connect: Vec<SocketAddr> = Vec::new();
                    let cur_connected = server.addr_connected_peer();
                    let my_addr = host_peer.address();
                    {
                        let mut registry = peer_registry.lock();
                        for peer in peers {
                            if let std::collections::hash_map::Entry::Vacant(e) = registry.entry(peer.username().to_string()) {
                                println!(
                                    "* Adicionando {} ({}) à lista de peers.",
                                    peer.username(),
                                    peer.address()
                                );
                                e.insert(peer.clone());

                                let peer_addr = peer.address();
                                if peer.username() != host_peer.username() 
                                    && my_addr < peer_addr
                                    && !cur_connected.contains(&peer_addr) {
                                    to_connect.push(peer.address());
                                }
                            }
                        }
                    }

                    if !to_connect.is_empty() {
                        println!("* Conectando aos novos peers...");
                        server.connect_to_many(&to_connect, esender.clone()).await;
                    }
               }
            },
            Event::UIMessage(fnp) => {
                // Lida com mensagens eviadas do cliente para ele mesmo
                if fnp
                    .dest()
                    .is_some_and(|d| d.address() == fnp.rem().address())
                {
                    if let FNP::InventoryInspection { .. } = fnp {
                        let inventory = fish_basket.lock();
                        if inventory.map().is_empty() {
                            println!("* Seu inventário está vazio.");
                        } else {
                            println!("* Seu inventário:");
                            for (fish_type, quantity) in inventory.map().iter() {
                                let style = fish_catalog.get_style_for_fish(fish_type);
                                println!("- {} {}(s)", quantity, style.style(fish_type));
                            }
                        }
                    } else {
                        println!("* Essa operação não é válida para você mesmo.");
                    }
                    continue;
                }
                // Lida com mensagens enviadas do cliente para outro peer
                match &fnp {
                    FNP::TradeConfirm {
                        dest,
                        response,
                        offer,
                        ..
                    } => {
                        if *response {
                            let mut is_valid = true;
                            // Criando um escopo para evitar bugs com o Lock
                            {
                                let mut inventory = fish_basket.lock();
                                // Validação da oferta de troca recebida checando se o cliente tem
                                // peixes o suficiente para aceitar a troca
                                for item in &offer.requested {
                                    let available =
                                        inventory.map().get(&item.fish_type).copied().unwrap_or(0);
                                    if available < item.quantity {
                                        println!(
                                            "* Troca inválida! Você não tem mais {} {}(s).",
                                            item.quantity, item.fish_type
                                        );
                                        is_valid = false;
                                        break;
                                    }
                                }
                                // Se a troca não for válida, mantém a proposta no buffer
                                // Caso contrário, execute a resposta decidida pelo usuário
                                if !is_valid {
                                    continue;
                                } else {
                                    println!("-- OFERTA ACEITA --");
                                    for item in &offer.offered {
                                        let style =
                                            fish_catalog.get_style_for_fish(&item.fish_type);
                                        println!(
                                            "* Você ganhou {} {}(s)",
                                            item.quantity,
                                            style.style(&item.fish_type)
                                        );
                                        *inventory
                                            .map_mut()
                                            .entry(item.fish_type.clone())
                                            .or_insert(0) += item.quantity;
                                    }
                                    for item in &offer.requested {
                                        let style =
                                            fish_catalog.get_style_for_fish(&item.fish_type);
                                        println!(
                                            "* Você perdeu {} {}(s)",
                                            item.quantity,
                                            style.style(&item.fish_type)
                                        );
                                        if let Some(count) =
                                            inventory.map_mut().get_mut(&item.fish_type)
                                        {
                                            *count = count.saturating_sub(item.quantity);
                                            if *count == 0 {
                                                inventory.map_mut().remove(&item.fish_type);
                                            }
                                        }
                                    }
                                }
                            }
                        } else {
                            println!("-- OFERTA RECUSADA --");
                        }
                        offer_buffers.lock().offers_received.remove(&dest.address());
                    }
                    FNP::TradeOffer { dest, offer, .. } => {
                        println!("-- OFERTA FEITA --");
                        offer_buffers
                            .lock()
                            .offers_made
                            .insert(dest.address(), offer.clone());
                    }
                    _ => {}
                }
                // Enviando a mensagem FNP definida no bloco match acima
                server_sender.send(fnp).await.ok();
            }
            Event::Pesca => {
                let plain_fish = crate::tui::fishing(&fish_catalog);
                fish_basket
                    .lock()
                    .map_mut()
                    .entry(plain_fish.clone())
                    .and_modify(|f| *f += 1)
                    .or_insert(1);

                let style = fish_catalog.get_style_for_fish(&plain_fish);
                println!("Você pescou um(a) {}!", style.style(&plain_fish));
            }
        }
    }

    Ok(())
}
