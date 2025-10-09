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

use std::{io::Write, sync::Arc};

use async_dup::Mutex;
use clap::Parser;
use async_channel::unbounded;
use fishnet::FishBasket;
use regex::Regex;
use smol::io;


/// Endereço de IP do primeiro peer: 127.0.0.1:6000
const DEFAULT_HOST: ([u8; 4], u16) = ([127, 0, 0, 1], 6000);


fn main() -> io::Result<()> {
    smol::block_on(async {
         // Decodificando argumentos da linha de comando
        let args = fishnet::tui::Args::parse();
        // Criando o canal de comunicação entre threads, o receiver vai pro dispatcher
        let (sender, receiver) = unbounded();

        // Escolhendo um nome/domínio de usuário
        let username = ask_username();

        // iniciando o catalogo de peixes
        let fish_catalog = Arc::new(fishnet::tui::FishCatalog::new());        
        let basket = Arc::new(Mutex::new(FishBasket::new()));

        // Criando o listener na porta correspondente
        let host = if args.first() { DEFAULT_HOST.into() } else { args.bind() }; 
        let server = Arc::new(fishnet::server::Server::new(&username, host)?);
        
        // Tentando conectar com os peers que foram passados como argumento
        server.connect_to_many(args.peers(), sender.clone()).await;

        // Cria um channel do dispatcher para o server, mensagens criadas na ui ou no dispatcher
        // são enviadas para o servidor enviar a rede de peers.
        let (ssender, sreceiver) = unbounded();
        let serverc = server.clone();
        smol::spawn(async move { serverc.send_messages(sreceiver).await.ok(); }).detach();

        // Spawnando o dispatcher. Recebe eventos das outras threads e envia para o servidor e ui
        smol::spawn(fishnet::dispatch(ssender, fish_catalog.clone(), basket.clone(), receiver)).detach();

        // Dando boas vindas ao usuário
        println!("Escutando no endereço {}", host);
        println!("Bem vindo {}, à Rede de Pesca!\nFique a vontade para pascar e conversar :)", username);
        // criando nova thread para gerenciar a interface do terminal
        smol::spawn(fishnet::tui::eval(sender.clone(), host)).detach();
        
        // Escutando por novas conexões dos peers. Bloqueia a thread principal
        server.listen(sender.clone()).await
    })
}

/// Pergunta o nome do usuário repetidamente até que o nome obedeça as retrições: (no minimo três
/// caracteres, e contenha caracteres alfanuméricos, barra ou underscore)
fn ask_username() -> String {
    let mut username = String::new();
    let username_pattern = Regex::new(r"^[A-Za-z][A-Za-z0-9_-]{2,}$").expect("Invalid regex pattern");
    loop {
        print!("Escolha um nome de usuário: ");
        std::io::stdout().flush().expect("Falha ao limpar o buffer");
        std::io::stdin().read_line(&mut username).expect("Não foi possível ler da entrada padrão.");
        let name = username.trim();
        if username_pattern.is_match(&name) {
            return name.to_owned();
        }
        println!("Nome de usuário inválido. Seu nome de usuário deve começar com um letras do alfabeto. Ter no mínimo 3 caracteres. Use caracteres alphauméricos, hifes ou underscores.");
        username.clear();
    }
} 
