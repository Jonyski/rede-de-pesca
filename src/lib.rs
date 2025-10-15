#![allow(unused)]
use std::{net::SocketAddr, sync::Arc};

use async_channel::{Receiver, Sender};
use async_dup::Mutex;

use crate::server::{Peer, FNP};
pub use crate::inventory::FishBasket;

pub mod server;
pub mod tui;
pub mod inventory;


pub enum Event {
    Join(SocketAddr), // new peer connect to us :)
    Leave(SocketAddr), // peer leave us :(
    ServerMessage(server::FNP), // a peer send a message

    UIMessage(server::FNP), // we send a message/cmd to dispatcher
    Pesca, // usuario quer pescar
}

/// Função central de eventos, recebe sinais por channels e envia para outras partes da aplicação, atualizando o estado geral
pub async fn dispatch(
    host_addr: SocketAddr,
    server_sender: Sender<FNP>,
    fish_catalog: Arc<tui::FishCatalog>,
    fish_basket: Arc<Mutex<FishBasket>>,
    receiver: Receiver<Event>,
    ) -> smol::io::Result<()> {

    while let Ok(event) = receiver.recv().await {
        match event {
            // Alguem entrou na rede
            Event::Join(name) => {
                println!("* {} entrou na rede.", name);
            },
            // Sairam na rede
            Event::Leave(name) => {
                println!("* {} saiu da rede.", name);
            },
            // Mensagens vindo da conexao TCP
            // Nesse caso nosso usuário é o destinatario.
            Event::ServerMessage(fnp) => match fnp {
                server::FNP::Message { rem, content, .. } => {
                    println!("{} te disse: {}", rem, content);
                },
                server::FNP::Broadcast { rem, content } => {
                    println!("{} - {}", rem, content);
                },
                server::FNP::InventoryInspection { rem, dest } => {
                    // Responde uma inspeção com um inventário.
                    // Cria um inventario no formato do protocolo com base no inventario global
                    let inventory_items: Vec<server::InventoryItem> = fish_basket.lock().map()
                        .iter()
                        .map(|(k, v)| server::InventoryItem::new(k.to_string(), *v))
                        .collect();

                    let fnp = server::FNP::InventoryShowcase{
                        rem: Peer::new(host_addr),
                        dest: rem,
                        inventory: server::Inventory {items: inventory_items}
                    };
                    server_sender.send(fnp).await.ok();
                },
                server::FNP::InventoryShowcase { rem, inventory, .. } => {
                    println!("* [{}]: Inventário", rem);
                    println!("{}", inventory);
                },
                server::FNP::TradeOffer { rem, dest, offer } => todo!(),
                server::FNP::TradeConfirm { rem, dest, response } => todo!(),
            },

            // Nesse caso o usuário é o remetente
            Event::UIMessage(fnp) => {
                // Se o protocolo for para o próprio usuário
                if fnp.dest().map_or(false, |v| v.address() == fnp.rem().address()) {
                    match fnp {
                        FNP::InventoryInspection { .. } => {
                            // Transforma o fish basket em um inventario do protocolo e mostra na
                            // tela
                            let inventory_items: Vec<server::InventoryItem> = fish_basket.lock().map()
                                .iter()
                                .map(|(k, v)| server::InventoryItem::new(k.to_string(), *v))
                                .collect();

                            println!("{}", server::Inventory { items: inventory_items });
                        },
                        _ => println!("* Essa operação não é válida para você mesmo."), // Message, Broadcast, TradeOffer, são todos inválidos se mandados
                                 // para o próprio usuário.
                    }
                } else {
                    // senão deixa o servidor cuidar disso
                    server_sender.send(fnp).await.ok();
                }
            },
            Event::Pesca => {
                // pesca um peixe e adiciona/incrementa ao inventario
                let fish = crate::tui::fishing(&fish_catalog);
                fish_basket.lock().map_mut().entry(fish.clone()).and_modify(|f| *f += 1).or_insert(1);
                println!("Você pescou um(a) {}!", fish);
            }
        }
    }

    Ok(())
}
