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

use std::sync::Arc;

use async_channel::unbounded;
use clap::Parser;
use smol::io;

// Importando elementos do nosso próprio pacote (fishnet)/importando lib.rs
use fishnet::AppState;

/// Endereço de IP do primeiro peer: 127.0.0.1:6000
const DEFAULT_HOST: ([u8; 4], u16) = ([127, 0, 0, 1], 6000);

fn main() -> io::Result<()> {
    // Bloqueamos a thread principal a espera de eventos assincronos
    smol::block_on(async {
        // Lemos argumentos do programa
        let args = fishnet::tui::Args::parse();
        // Pergunta nome do usuário
        let username = fishnet::tui::ask_username();

        // O primeiro peer sempre se conecta na porta 6000, os outros escolhem
        let requested_addr = args.bind_port().unwrap_or(DEFAULT_HOST.into());

        // Inicializa channels
        let (sender, receiver) = unbounded();
        let (ssender, sreceiver) = unbounded();

        let app_state = Arc::new(AppState::new());

        let server = Arc::new(fishnet::ServerBackend::new(
            &username,
            requested_addr
        )?);

        let host_peer = server.host();
        app_state.peer_registry
            .lock()
            .insert(username.clone(), host_peer.clone());
        println!("Escutando no endereço {}", host_peer.address());

        // Conectando com os peers passados como argumento
        server.connect_to_many(args.peers(), sender.clone()).await;

        // Spawna a thread do dispatcher
        smol::spawn(fishnet::dispatch(app_state.clone(), server.clone(), ssender, sender.clone(), receiver)).detach();

        println!(
            "Bem vindo {}, à Rede de Pesca!\nFique a vontade para pascar e conversar :)",
            username
        );

        // Spawna o handler de inputs do usuário, que envia msgs de UI para o dispatcher
        smol::spawn(fishnet::tui::eval(
            app_state.clone(),
            sender.clone(),
            host_peer.clone()
        )).detach();

        // Deixa o server escutando sempre novas conexões de peers entrando na rede de pesca
        server.run(sreceiver, sender).await
    })
}
