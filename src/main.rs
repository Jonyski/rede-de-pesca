// --------------------------------------------------------
//              EP 1 de Redes de Computadores
//  AVISOS:
//  - Partes do código foram geradas utilizando IA, mais
//  especificamente o Gemini 2.5 Pro. As principais partes
//  escritas por ele foram trechos da função dispatch()
//  e da main() que corrigiram bugs.
//  - A versão inicial do código foi escrita mesclando
//  duas referências:
//      1. O repositório deste artigo criando um P2P com
//      Web Sockets: https://mohyfahim.info/rust-and-websocket-building-a-peer-to-peer-network
//      2. O exemplo de aplicação cliente-servidor da
//      biblioteca "smol": https://github.com/smol-rs/smol/tree/master/examples
//  Contudo, fizemos todas as alterações necessárias para
//  transformar estas referências em um projeto P2P e que
//  utiliza o protocolo TCP ao invés de WebSockets.
// --------------------------------------------------------
//

use std::{collections::HashMap, io::Write, sync::Arc};

use async_channel::unbounded;
use async_dup::Mutex;
use clap::Parser;
use fishnet::{FishBasket, PeerRegistry, server::protocol::OfferBuff};
use regex::Regex;
use smol::io;

/// Endereço de IP do primeiro peer: 127.0.0.1:6000
const DEFAULT_HOST: ([u8; 4], u16) = ([127, 0, 0, 1], 6000);

fn main() -> io::Result<()> {
    smol::block_on(async {
        // ... (keep setup code as it was in the last correct version)
        let args = fishnet::tui::Args::parse();
        let (sender, receiver) = unbounded();
        let username = ask_username();

        let fish_catalog = Arc::new(fishnet::tui::FishCatalog::new());
        let basket = Arc::new(Mutex::new(FishBasket::new()));
        let offer_buffers = Arc::new(Mutex::new(OfferBuff::default()));
        let peer_registry: PeerRegistry = Arc::new(Mutex::new(HashMap::new()));

        let requested_addr = if args.first() {
            DEFAULT_HOST.into()
        } else {
            args.bind()
        };

        let server = Arc::new(fishnet::server::Server::new(&username, requested_addr)?);
        let host_peer = server.host_peer().clone();
        peer_registry
            .lock()
            .insert(username.clone(), host_peer.clone());
        println!("Escutando no endereço {}", host_peer.address());

        server.connect_to_many(args.peers(), sender.clone()).await;

        let (ssender, sreceiver) = unbounded();
        let serverc = server.clone();

        // --- FIX IS HERE ---
        // Clone the main sender channel to pass to the server's message sending loop.
        let dispatcher_sender = sender.clone();
        smol::spawn(async move {
            serverc
                .send_messages(sreceiver, dispatcher_sender)
                .await
                .ok();
        })
        .detach();

        // ... (rest of the main function is correct)
        smol::spawn(fishnet::dispatch(
            host_peer.clone(),
            ssender,
            fish_catalog.clone(),
            basket.clone(),
            offer_buffers.clone(),
            peer_registry.clone(),
            receiver,
        ))
        .detach();
        println!(
            "Bem vindo {}, à Rede de Pesca!\nFique a vontade para pascar e conversar :)",
            username
        );
        smol::spawn(fishnet::tui::eval(
            sender.clone(),
            host_peer.clone(),
            offer_buffers.clone(),
            peer_registry.clone(),
        ))
        .detach();

        server.listen(sender.clone()).await
    })
}

/// Pergunta o nome do usuário repetidamente até que o nome obedeça as retrições: (no minimo três
/// caracteres, e contenha caracteres alfanuméricos, barra ou underscore)
fn ask_username() -> String {
    let mut username = String::new();
    let username_pattern =
        Regex::new(r"^[A-Za-z][A-Za-z0-9_-]{2,}$").expect("Invalid regex pattern");
    loop {
        print!("Escolha um nome de usuário: ");
        std::io::stdout().flush().expect("Falha ao limpar o buffer");
        std::io::stdin()
            .read_line(&mut username)
            .expect("Não foi possível ler da entrada padrão.");
        let name = username.trim();
        if username_pattern.is_match(name) {
            return name.to_owned();
        }
        println!(
            "Nome de usuário inválido. Seu nome de usuário deve começar com um letras do alfabeto. Ter no mínimo 3 caracteres. Use caracteres alphauméricos, hifes ou underscores."
        );
        username.clear();
    }
}
