/*
 * Handlers para diferentes eventos do sistema
 * Todo handler segue o padrõa `handler_{evento}`
 */

use crate::{
    AppState, Event,
    server::{self, FNP, Inventory, Peer, ServerBackend, protocol::Offer},
};
use async_channel::Sender;
use std::net::{self, SocketAddr};

/// Handler para quando um peer se disconecta.
/// Remove da lista de peers conhecidos e anuncia ao usuário
pub async fn handle_peer_disconnected(
    app_state: &AppState,
    server: &ServerBackend,
    client_addr: net::SocketAddr,
) {
    if let Some(peer_info) = server.peer_store().unregister_by_client(&client_addr).await {
        let peer = peer_info.peer;
        crate::tui::log(&format!(
            "{} ({}) saiu da rede.",
            peer.username(),
            peer.address()
        ));
        let mut offer_buffers = app_state.offer_buffers.lock();
        if offer_buffers.offers_made.remove(&peer.address()).is_some() {
            crate::tui::log(&format!(
                "Oferta feita para {} foi cancelada.",
                peer.username()
            ));
        }
        if offer_buffers
            .offers_received
            .remove(&peer.address())
            .is_some()
        {
            crate::tui::log(&format!(
                "Oferta recebida de {} foi cancelada.",
                peer.username()
            ));
        }
    } else {
        crate::tui::err(&format!(
            "Peer desconhecido se desconectou: {}",
            client_addr
        ));
    }
}

/// Pesca um peixe e guarda na cesta
pub async fn handle_pesca(app_state: &AppState) {
    let plain_fish = crate::gameplay::fishing(&app_state.fish_catalog);
    // se houver aquele peixe na sexta, incrementamos sua contagem, senão adicionamos
    // com o valor 1
    app_state
        .basket
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
    app_state: &AppState,
    msg: FNP,
    server: &ServerBackend,
    client_addr: SocketAddr,
    server_sender: Sender<FNP>,
    event_sender: Sender<Event>,
) {
    match msg {
        FNP::Message { rem, content, .. } => {
            handle_server_direct_message(rem, &content).await;
        }
        FNP::Broadcast { rem, content } => {
            handle_server_broadcast_message(rem, &content).await;
        }
        FNP::TradeOffer { rem, offer, .. } => {
            handle_server_tradeoffer(app_state, rem, offer).await;
        }
        FNP::TradeConfirm {
            rem,
            response,
            offer,
            ..
        } => {
            handle_server_tradeconfirm(app_state, response, rem, &offer).await;
        }
        FNP::InventoryInspection { rem, .. } => {
            handle_server_inventory_request(app_state, rem, server, server_sender).await;
        }
        FNP::InventoryShowcase { rem, inventory, .. } => {
            handle_server_inventory_showcase(app_state, rem, inventory).await;
        }
        FNP::AnnounceName { rem } => {
            handle_server_announce_name(server, rem, client_addr, server_sender).await;
        }
        FNP::PeerList { rem, peers, .. } => {
            handle_server_peerlist(&peers, server, rem, event_sender).await;
        }
        FNP::RejectConnection { .. } => {
            handle_rejection().await;
        }
    }
}

/// Trata mensagens geradas pela UI pelo usuário
pub async fn handle_ui_message(app_state: &AppState, msg: FNP, server_sender: Sender<FNP>) {
    // Lida com mensagens enviadas do cliente para ele mesmo
    if msg
        .dest()
        .is_some_and(|d| d.address() == msg.rem().address())
    {
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
        FNP::TradeConfirm {
            dest,
            response,
            offer,
            ..
        } => {
            handle_ui_tradeconfirm(app_state, *response, offer, dest).await;
        }
        FNP::TradeOffer { dest, offer, .. } => {
            handle_ui_tradeoffer(app_state, dest, offer).await;
        }
        _ => (),
    }
    // Enviando a mensagem para o servidor mandar aos peers
    server_sender.send(msg).await.ok();
}

async fn reject_homonym(server: &ServerBackend, rem: &Peer, client_addr: SocketAddr) {
    let reject_msg = FNP::RejectConnection {
        rem: server.host(),
        dest: rem.clone(),
    };
    if let Some(conn) = server.connections().lock().get(&client_addr).cloned() {
        conn.send_fnp(&reject_msg).await.ok();
    }
    server.connections().lock().remove(&client_addr);
    crate::tui::err(&format!(
        "Utilizador '{}' ({}) tentou se conectar, mas o nome já está em uso. Conexão rejeitada.",
        rem.username(),
        rem.address()
    ));
}

async fn handle_rejection() {
    crate::tui::err("------------------------------------------------------");
    crate::tui::err("FALHA AO CONECTAR: Nome de usuário já está em uso!");
    crate::tui::err("Por favor, reinicie a aplicação com um nome diferente.");
    crate::tui::err("------------------------------------------------------");
    std::process::exit(1);
}

async fn handle_server_announce_name(
    server: &ServerBackend,
    rem: Peer,
    client_addr: SocketAddr,
    server_sender: Sender<FNP>,
) {
    // Anúncio de nome e conexão, atualiza o registro de peers
    // Primeiro, verifica se o nome de usuário já está em uso
    if let Some(existing_peer) = server.peer_store().get_by_username(rem.username()).await {
        // Se o nome de usuário já estiver em uso por outro endereço, rejeita a conexão
        if existing_peer.peer.address() != rem.address() {
            reject_homonym(server, &rem, client_addr).await;
            return;
        } else {
            // Se for o mesmo endereço
            // Remove a conexão duplicada
            server.connections().lock().remove(&client_addr);
            return;
        }
    }
    // Depois, verifica se o peer não está tentando usar seu nome
    // Rejeita sua conexão se este for for o caso
    if rem.username() == server.host().username() {
        reject_homonym(server, &rem, client_addr).await;
        return;
    }

    // Se ainda não temos esse peer registrado
    if server
        .peer_store()
        .get_by_listener(&rem.address())
        .await
        .is_none()
    {
        server.register_peer(rem.clone(), client_addr).await;

        crate::tui::log(&format!(
            "{} ({}) se conectou.",
            rem.username(),
            rem.address()
        ));

        let peers = server.peer_store().all_pears().await;
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

async fn handle_server_inventory_request(
    app_state: &AppState,
    peer: Peer,
    server: &ServerBackend,
    server_sender: Sender<FNP>,
) {
    let basket = app_state.basket.lock();

    let mut inventory_items: Vec<server::InventoryItem> = basket
        .map()
        .iter()
        .map(|(k, v)| server::InventoryItem::new(k.to_string(), *v))
        .collect();

    drop(basket);

    // Ordena o vetor de itens com base na raridade
    inventory_items.sort_by(|a, b| {
        let rank_a = app_state.fish_catalog.get_rarity_rank(&a.fish_type);
        let rank_b = app_state.fish_catalog.get_rarity_rank(&b.fish_type);
        rank_a.cmp(&rank_b)
    });

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
    app_state
        .offer_buffers
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

async fn handle_server_tradeconfirm(
    app_state: &AppState,
    response: bool,
    rem: Peer,
    offer: &Offer,
) {
    if response {
        crate::tui::log(&format!(
            "{} aceitou sua oferta de troca :)",
            rem.username()
        ));
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
            *inventory
                .map_mut()
                .entry(item.fish_type.clone())
                .or_insert(0) += item.quantity;
        }
    } else {
        crate::tui::log(&format!(
            "{} recusou sua oferta de troca :(",
            rem.username()
        ));
    }
    app_state
        .offer_buffers
        .lock()
        .offers_made
        .remove(&rem.address());
}

async fn handle_server_peerlist(
    peers: &[Peer],
    server: &ServerBackend,
    rem: Peer,
    sender: Sender<Event>,
) {
    let mut to_connect: Vec<std::net::SocketAddr> = Vec::new();

    // For each peer in the received list, ensure it's in PeerStore.
    for peer in peers {
        // Only add if not already present (by username or address)
        if server
            .peer_store()
            .get_by_username(peer.username())
            .await
            .is_none()
        {
            // Decide if we should connect to this peer
            let peer_addr = peer.address();
            if peer.username() != server.host().username()
                && peer.username() != rem.username()
                && peer_addr < server.host().address()
            {
                crate::tui::log(&format!(
                    "Adicionando {} ({}) à lista de peers.",
                    peer.username(),
                    peer.address()
                ));
                to_connect.push(peer_addr);
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
        let mut items: Vec<(&String, &u32)> = inventory.map().iter().collect();

        // Ordenando a lista de peixes com base na raridade para exibição
        items.sort_by(|(fish_a, _), (fish_b, _)| {
            let rank_a = app_state.fish_catalog.get_rarity_rank(fish_a);
            let rank_b = app_state.fish_catalog.get_rarity_rank(fish_b);
            rank_a.cmp(&rank_b)
        });

        for (fish_type, quantity) in items {
            let style = app_state.fish_catalog.get_style_for_fish(fish_type);
            println!("> [{}] {}", quantity, style.style(fish_type));
        }
    }
}

async fn handle_ui_tradeoffer(app_state: &AppState, dest: &Peer, offer: &Offer) {
    app_state
        .offer_buffers
        .lock()
        .offers_made
        .insert(dest.address(), offer.clone());
    crate::tui::log("-- OFERTA FEITA --");
}

async fn handle_ui_tradeconfirm(app_state: &AppState, response: bool, offer: &Offer, dest: &Peer) {
    if response {
        let mut is_valid = true;
        // Criando um escopo para evitar bugs com o Lock
        {
            let mut inventory = app_state.basket.lock();
            // Validação da oferta de troca recebida checando se o cliente tem
            // peixes o suficiente para aceitar a troca
            for item in &offer.requested {
                let available = inventory.map().get(&item.fish_type).copied().unwrap_or(0);
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
                    let style = app_state.fish_catalog.get_style_for_fish(&item.fish_type);
                    println!("+ {} {}(s)", item.quantity, style.style(&item.fish_type));
                    *inventory
                        .map_mut()
                        .entry(item.fish_type.clone())
                        .or_insert(0) += item.quantity;
                }
                for item in &offer.requested {
                    let style = app_state.fish_catalog.get_style_for_fish(&item.fish_type);
                    println!("- {} {}(s)", item.quantity, style.style(&item.fish_type));
                    if let Some(count) = inventory.map_mut().get_mut(&item.fish_type) {
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
    app_state
        .offer_buffers
        .lock()
        .offers_received
        .remove(&dest.address());
}
