pub use crate::event::Event;
use crate::event::handlers;
use crate::gameplay::FishBasket;
use crate::gameplay::FishCatalog;
pub use crate::server::ServerBackend;
use crate::server::{FNP, Peer, protocol::OfferBuff};
use async_channel::{Receiver, Sender};
use async_dup::Mutex;
use std::{collections::HashMap, sync::Arc};

pub mod event;
pub mod gameplay;
pub mod server;
pub mod tui;

pub type PeerRegistry = HashMap<String, Peer>;

/// Estado da Aplicação, escapsula regiões críticas de memória
pub struct AppState {
    // Catálogo de peixes
    pub fish_catalog: FishCatalog,
    // Cesta de peixes, nosso inventário
    pub basket: Mutex<FishBasket>,
    // Buffer de ofertas/trocas recebidas
    pub offer_buffers: Mutex<OfferBuff>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            fish_catalog: FishCatalog::new(),
            basket: Mutex::new(FishBasket::new()),
            offer_buffers: Mutex::new(OfferBuff::default()),
        }
    }
}

/// Função de dispatch de eventos, ponte entre as diferentes interfaces do sistema
pub async fn dispatch(
    app_state: Arc<AppState>,
    server: Arc<ServerBackend>,
    server_sender: Sender<FNP>,
    event_sender: Sender<Event>,
    receiver: Receiver<Event>,
) -> smol::io::Result<()> {
    while let Ok(event) = receiver.recv().await {
        match event {
            Event::PeerDisconnected(socket_addr) => {
                handlers::handle_peer_disconnected(&server.clone(), socket_addr).await;
            }
            Event::ServerMessage(fnp, socket_addr) => {
                handlers::handle_server_message(
                    &app_state.clone(),
                    fnp,
                    &server.clone(),
                    socket_addr,
                    server_sender.clone(),
                    event_sender.clone(),
                )
                .await;
            }
            Event::UIMessage(fnp) => {
                handlers::handle_ui_message(&app_state.clone(), fnp, server_sender.clone()).await;
            }
            Event::Pesca => {
                handlers::handle_pesca(&app_state.clone()).await;
            }
        }
    }
    Ok(())
}
