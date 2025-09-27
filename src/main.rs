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

use clap::Parser;
use async_channel::unbounded;
use smol::io;


mod event_system;
mod ui;
mod fishnet_server;


/// Endereço de IP do primeiro peer: 127.0.0.1:6000
const DEFAULT_HOST: ([u8; 4], u16) = ([127, 0, 0, 1], 6000);


fn main() -> io::Result<()> {
    smol::block_on(async {
         // Decodificando argumentos da linha de comando
        let args = ui::Args::parse();
        // Criando o canal de comunicação entre threads, o receiver vai pro dispatcher
        let (sender, receiver) = unbounded();

        // Criando o listener na porta correspondente
        let host = if args.first() { DEFAULT_HOST.into() } else { args.bind() }; 
        let mut server = fishnet_server::PeerServer::new(host)?;

        // Tentando conectar com os peers que foram passados como argumento
        server.connect_to_many(args.peers(), sender.clone()).await;

        // Dando boas vindas ao usuário
        println!("Escutando no endereço {}", server.local_addr());
        println!("Bem vindo à Rede de Pesca!\nFique a vontade para pascar e conversar :)");

        // iniciando o catalogo de peixes
        let fish_catalog = ui::FishCatalog::new();

        // Spawnando o dispatcher
        smol::spawn(event_system::dispatch(server.streams(), receiver, server.local_addr(), fish_catalog)).detach();

        // criando nova thread para gerenciar a interface do terminal
        smol::spawn(ui::eval(sender.clone(), server.local_addr())).detach();
        
        // Escutando por novas conexões dos peers
        server.listen(sender.clone()).await
    })
}
