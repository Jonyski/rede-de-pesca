use std::{net::SocketAddr, sync::Arc};

use async_channel::{Receiver, Sender};

use async_dup::Mutex;
use server::Server;

use crate::server::FNP;

pub mod server;
pub mod tui;


pub enum Event {
    Join(SocketAddr), // new peer connect to us :)
    Leave(SocketAddr), // peer leave us :(
    ServerMessage(server::FNP), // a peer send a message

    UIMessage(server::FNP), // we send a message/cmd to dispatcher
    Pesca, // usuario quer pescar
}


pub async fn dispatch(
    server_sender: Sender<FNP>,
    fish_catalog: Arc<tui::FishCatalog>,
    receiver: Receiver<Event>,
    ) -> smol::io::Result<()> {
    
    while let Ok(event) = receiver.recv().await {
        match event {
            Event::Join(name) => {
                println!("* {} entrou na rede.", name);
            },
            Event::Leave(name) => {
                println!("* {} saiu da rede.", name);
            },
            // Nesse caso nosso usuário é o destinatario.
            Event::ServerMessage(fnp) => match fnp {
                server::FNP::Message { rem, content, .. } => {
                    println!("{} te disse: {}", rem, content);
                },
                server::FNP::Broadcast { rem, content } => {
                    println!("{} - {}", rem, content);
                },
                server::FNP::TradeOffer { rem, dest, offer } => todo!(),
                server::FNP::TradeConfirm { rem, dest, response } => todo!(),
                server::FNP::InventoryInspection { rem, dest } => todo!(),
                server::FNP::InventoryShowcase { rem, dest, inventory } => todo!(),
            },
            // Nesse caso o usuário é o remetente
            Event::UIMessage(fnp) => {
                server_sender.send(fnp).await.ok();
            },
            Event::Pesca => {
                let fish = crate::tui::fishing(&fish_catalog);
                // TODO: add para o inventario
                println!("{}", fish);
            }
        }
    }

    Ok(())
}
