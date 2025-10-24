/*
 * Handlers para diferentes eventos do sistema
 * Todo handler segue o padrõa `handler_{evento}`
 */

use std::net;
use async_channel::Sender;
use crate::{server::{self, protocol::Offer, Inventory, Peer, ServerBackend, FNP}, AppState, Event};

/// Handler para quando um peer se disconecta.
/// Remove da lista de peers conhecidos e anuncia ao usuário
pub async fn handle_peer_disconnected(_app_state: &AppState, peer: net::SocketAddr) {
    // TODO: receber um peer e não um socket
    // if app_state.peer_registry.lock().0.remove(peer.username()).is_some() {
    //     crate::tui::log(&format!(
    //         "{} ({}) saiu da rede.",
    //         peer.username(),
    //         peer.address()
    //     ));
    // }
    crate::tui::log(&format!(
        "({}) saiu da rede.",
        peer
    ));
}

/// Pesca um peixe e guarda na cesta
pub async fn handle_pesca(app_state: &AppState) {
    let plain_fish = crate::gameplay::fishing(&app_state.fish_catalog);
    // se houver aquele peixe na sexta, incrementamos sua contagem, senão adicionamos
    // com o valor 1
    app_state.basket
        .lock()
        .map_mut()
        .entry(plain_fish.clone())
        .and_modify(|f| *f += 1)
        .or_insert(1);

    let style = app_state.fish_catalog.get_style_for_fish(&plain_fish);
    println!("Você pescou um(a) {}!", style.style(&plain_fish));
}

/// Trata mensagens advindas do servidor ou seja de peers pela rede
pub async fn handle_server_message(
    app_state: &AppState, msg: FNP, server: &ServerBackend, server_sender: Sender<FNP>,
    event_sender: Sender<Event>
) {
    match msg {
        FNP::Message { rem, content, .. } => {
            handle_server_direct_message(rem, &content).await;
        },
        FNP::Broadcast { rem, content } => {
            handle_server_broadcast_message(rem, &content).await;
        },
        FNP::TradeOffer { rem, offer, .. } => {
            handle_server_tradeoffer(app_state, rem, offer).await;
        },
        FNP::TradeConfirm { rem, response, offer, .. } => {
            handle_server_tradeconfirm(app_state, response, rem, &offer).await;
        },
        FNP::InventoryInspection { rem, .. } => {
            handle_server_inventory_request(app_state, rem, server, server_sender).await;
        },
        FNP::InventoryShowcase { rem, inventory, .. } => {
            handle_server_inventory_showcase(app_state, rem, inventory).await;
        },
        FNP::AnnounceName { rem } => {
            handle_server_announce_name(app_state, server, rem, server_sender).await;
        },
        FNP::PeerList { rem, peers, .. } => {
            handle_server_peerlist(app_state, &peers, server, rem, event_sender).await;
        },
    }
}

/// Trata mensagens geradas pela UI pelo usuário
pub async fn handle_ui_message(app_state: &AppState, msg: FNP, server_sender: Sender<FNP>) {
    // Lida com mensagens enviadas do cliente para ele mesmo
    if msg.dest().is_some_and(|d| d.address() == msg.rem().address()) {
        // Usuário pode ver o próprio inventário
        if let FNP::InventoryInspection { .. } = msg {
            handle_ui_inventory_inspection(app_state).await;
        } else {
            crate::tui::err("Este comando não é válido para você mesmo");
        }
        return;
    }
    // Lida com mensagens enviadas do cliente para outro peer
    match &msg {
        FNP::TradeConfirm { dest, response, offer, ..} => {
            handle_ui_tradeconfirm(app_state, *response, offer, dest).await;
        }
        FNP::TradeOffer { dest, offer, .. } => {
            handle_ui_tradeoffer(app_state, dest, offer).await;
        }
        _ => {
            // Enviando a mensagem para o servidor mandar aos peers
            server_sender.send(msg).await.ok();
        }
    }
}


async fn handle_server_announce_name(app_state: &AppState, server: &ServerBackend, rem: Peer, server_sender: Sender<FNP>) {
    // Anúncio de nome e conexão, atualiza o registro de peers
    let mut registry = app_state.peer_registry.lock();
    if !registry.contains_key(rem.username()) {
        crate::tui::log(&format!(
            "{} ({}) se conectou.",
            rem.username(),
            rem.address()
        ));
        registry.insert(rem.username().into(), rem.clone());

        let peers = registry.values().cloned().collect();
        let peer_list_msg = FNP::PeerList {
            rem: server.host(),
            dest: rem,
            peers,
        };
        server_sender.send(peer_list_msg).await.ok();
    }
}

async fn handle_server_direct_message(rem: Peer, content: &str) {
    println!("DM de {}: {}", rem.username(), content);
}

async fn handle_server_broadcast_message(rem: Peer, content: &str) {
    println!("{} - {}", rem.username(), content);
}

async fn handle_server_inventory_request(app_state: &AppState, peer: Peer, server: &ServerBackend, server_sender: Sender<FNP>) {
    let inventory_items: Vec<server::InventoryItem> = app_state.basket
        .lock()
        .map()
        .iter()
        .map(|(k, v)| server::InventoryItem::new(k.to_string(), *v))
        .collect();

    let fnp = server::FNP::InventoryShowcase {
        rem: server.host(),
        dest: peer,
        inventory: server::Inventory {
            items: inventory_items,
        },
    };
    server_sender.send(fnp).await.ok();
}

async fn handle_server_inventory_showcase(app_state: &AppState, peer: Peer, inventory: Inventory) {
    println!("-- INVENTÁRIO DE {} --", peer.username().to_uppercase());
    if inventory.items.is_empty() {
        crate::tui::log("[Nenhum peixe aqui]");
    } else {
        // Style the inventory for display
        for item in &inventory.items {
            let style = app_state.fish_catalog.get_style_for_fish(&item.fish_type);
            println!("> [{}] {}", item.quantity, style.style(&item.fish_type));
        }
    }
}

async fn handle_server_tradeoffer(app_state: &AppState, rem: Peer, offer: Offer) {
    // Adicionando ao buffer de ofertas recebidas
    app_state.offer_buffers
        .lock()
        .offers_received
        .insert(rem.address(), offer.clone());
    // Exibindo os peixes ofertados e requisitados pelo remetente
    println!("{} quer realizar a seguinte troca:", rem.username());
    offer.offered.into_iter().for_each(|f| {
        let style = app_state.fish_catalog.get_style_for_fish(&f.fish_type);
        println!("> {} {}(s)", f.quantity, style.style(&f.fish_type));
    });
    println!("por");
    offer.requested.into_iter().for_each(|f| {
        let style = app_state.fish_catalog.get_style_for_fish(&f.fish_type);
        println!("> {} {}(s)", f.quantity, style.style(&f.fish_type))
    });
    crate::tui::log(&format!(
        "Digite '$c [s]im {}' para aceitar, ou '$c [n]ao {}' para recusar",
        rem.username(),
        rem.username()
    ));
}

async fn handle_server_tradeconfirm(app_state: &AppState, response: bool, rem: Peer, offer: &Offer) {
    if response {
        crate::tui::log(&format!("{} aceitou sua oferta de troca :)", rem.username()));
        let mut inventory = app_state.basket.lock();
        // Removendo os peixes que você deu
        for item in &offer.offered {
            let style = app_state.fish_catalog.get_style_for_fish(&item.fish_type);
            println!("- {} {}(s)", item.quantity, style.style(&item.fish_type));
            if let Some(count) = inventory.map_mut().get_mut(&item.fish_type) {
                *count -= item.quantity;
                if *count == 0 {
                    inventory.map_mut().remove(&item.fish_type);
                }
            }
        }
        // Adicionando os peixes que você recebeu
        for item in &offer.requested {
            let style = app_state.fish_catalog.get_style_for_fish(&item.fish_type);
            println!("+ {} {}(s)", item.quantity, style.style(&item.fish_type));
            *inventory.map_mut().entry(item.fish_type.clone()).or_insert(0) +=
                item.quantity;
        }
    } else {
        crate::tui::log(&format!("{} recusou sua oferta de troca :(", rem.username()));
    }
    app_state.offer_buffers.lock().offers_made.remove(&rem.address());
}

async fn handle_server_peerlist(app_state: &AppState, peers: &[Peer], server: &ServerBackend, rem: Peer, sender: Sender<Event>) {
    let mut to_connect: Vec<net::SocketAddr> = Vec::new();
    {
        let mut registry = app_state.peer_registry.lock();
        for peer in peers {
            if let std::collections::hash_map::Entry::Vacant(e) = registry.entry(peer.username().into()) {
                crate::tui::log(&format!(
                    "Adicionando {} ({}) à lista de peers.",
                    peer.username(),
                    peer.address()
                ));
                e.insert(peer.clone());

                let peer_addr = peer.address();

                if peer.username() != server.host().username()
                    && peer.username() != rem.username()
                    && peer_addr < server.host().address() {

                    to_connect.push(peer.address());
                }
            }
        }
    }

    if !to_connect.is_empty() {
        println!("* Conectando aos novos peers...");
        server.connect_to_many(&to_connect, sender.clone()).await;
    }
}

async fn handle_ui_inventory_inspection(app_state: &AppState) {
    let inventory = app_state.basket.lock();
    println!("-- INVENTÁRIO --");
    if inventory.map().is_empty() {
        crate::tui::log("[Nenhum peixe aqui, digite $[p]esca para pescar]");
    } else {
        for (fish_type, quantity) in inventory.map().iter() {
            let style = app_state.fish_catalog.get_style_for_fish(fish_type);
            println!("> [{}] {}", quantity, style.style(fish_type));
        }
    }
}

async fn handle_ui_tradeoffer(app_state: &AppState, dest: &Peer, offer: &Offer) {
    crate::tui::log("-- OFERTA FEITA --");
    app_state.offer_buffers
        .lock()
        .offers_made
        .insert(dest.address(), offer.clone());}

async fn handle_ui_tradeconfirm(app_state: &AppState, response: bool, offer: &Offer, dest: &Peer) {
    if response {
        let mut is_valid = true;
        // Criando um escopo para evitar bugs com o Lock
        {
            let mut inventory = app_state.basket.lock();
            // Validação da oferta de troca recebida checando se o cliente tem
            // peixes o suficiente para aceitar a troca
            for item in &offer.requested {
                let available =
                    inventory.map().get(&item.fish_type).copied().unwrap_or(0);
                if available < item.quantity {
                    crate::tui::err(&format!(
                        "Troca inválida! Você não tem {} {}(s) para trocar.",
                        item.quantity, item.fish_type
                    ));
                    is_valid = false;
                    break;
                }
            }
            // Se a troca não for válida, mantém a proposta no buffer
            // Caso contrário, execute a resposta decidida pelo usuário
            if !is_valid {
                return;
            } else {
                crate::tui::log("-- OFERTA ACEITA --");
                for item in &offer.offered {
                    let style =
                        app_state.fish_catalog.get_style_for_fish(&item.fish_type);
                    println!(
                        "+ {} {}(s)",
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
                        app_state.fish_catalog.get_style_for_fish(&item.fish_type);
                    println!(
                        "- {} {}(s)",
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
        crate::tui::log("-- OFERTA RECUSADA --");
    }
    app_state.offer_buffers.lock().offers_received.remove(&dest.address());
}
