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

use async_channel::{Receiver, Sender, unbounded};
use async_dup::Arc;
use clap::Parser;
use smol::{Async, Unblock, io, prelude::*};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::Mutex;

/// Argumentos da linha de comando
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Flag dizendo se o peer sendo instanciado é o primeiro da rede
    #[arg(short, long)]
    first: bool,
    /// Lista dos peers inicialmente conhecidos
    #[arg(short, long, value_delimiter = ',', value_parser = parse_addr)]
    peers: Vec<SocketAddr>,
    /// Endereço onde se deseja bindar o peer sendo instanciado
    #[arg(short, long, value_parser = parse_addr)]
    bind: SocketAddr,
}

/// Parseando e validando os endereços
fn parse_addr(s: &str) -> Result<SocketAddr, String> {
    if let Ok(addr) = s.parse::<SocketAddr>() {
        return Ok(addr);
    }
    Err(format!("Invalid bind/peer address: {}", s))
}

/// Tipos de eventos/mensagens do nosso sistema
enum Event {
    /// Um peer se conectou na rede
    Join(SocketAddr, Arc<Async<TcpStream>>),
    /// Um peer saiu
    Leave(SocketAddr),
    /// Um peer enviou uma mensagem
    Message(SocketAddr, String),
}

/// Lida com os Eventos emitidos, podendo fazer broadcast
async fn dispatch(
    streams: Arc<Mutex<Vec<Arc<Async<TcpStream>>>>>,
    receiver: Receiver<Event>,
    my_addr: SocketAddr,
) -> io::Result<()> {
    // Recebe eventos em loop
    while let Ok(event) = receiver.recv().await {
        let (output, streams_to_write, sender_addr) = {
            let mut streams_guard = streams.lock().unwrap();
            // Formatando o output de forma específica para cada evento
            let (output_str, sender_addr_opt) = match &event {
                Event::Join(addr, stream) => {
                    streams_guard.push(stream.clone());
                    (format!("* {} entrou na rede\n", addr), Some(*addr))
                }
                Event::Leave(addr) => {
                    streams_guard
                        .retain(|s| s.get_ref().peer_addr().map_or(false, |peer| peer != *addr));
                    (format!("* {} saiu\n", addr), Some(*addr))
                }
                Event::Message(addr, msg) => (format!("[{}] - {}\n", *addr, msg), Some(*addr)),
            };

            // Criando uma lista apenas com os peers que devem receber a mensagem
            let sender_addr = sender_addr_opt.unwrap();
            let streams_to_write = streams_guard
                .iter()
                .filter(|s| {
                    if let Ok(peer_addr) = s.get_ref().peer_addr() {
                        peer_addr != sender_addr
                    } else {
                        false
                    }
                })
                .cloned()
                .collect::<Vec<_>>();

            (output_str, streams_to_write, sender_addr)
        };

        // Se a mensagem for de outro peer, apenas exibimos ela
        // Mas, se a mensagem for nossa, fazemos um broadcast
        if sender_addr != my_addr {
            print!("{}", output);
        } else if let Event::Message(_, msg) = &event {
            // Adicionando um \n para o destinatário ler a mensagem como uma nova linha
            let network_message = format!("{}\n", msg);
            for mut stream in streams_to_write {
                let message_to_send = network_message.clone();
                smol::spawn(async move {
                    // Enviando a mensagem bruta e como bytes
                    stream.write_all(message_to_send.as_bytes()).await.ok();
                })
                .detach();
            }
        }
    }
    Ok(())
}

/// Lendo mensagens de outros peers e enviando elas para o dispatcher
async fn read_messages(sender: Sender<Event>, peer: Arc<Async<TcpStream>>) -> io::Result<()> {
    let addr = peer.get_ref().peer_addr()?;
    let mut lines = io::BufReader::new(peer).lines();
    // Toda vez que houver uma nova linha na stream, manda a mensagem pro dispatcher
    while let Some(line) = lines.next().await {
        let line = line?;
        sender.send(Event::Message(addr, line)).await.ok();
    }
    Ok(())
}

fn main() -> io::Result<()> {
    smol::block_on(async {
        // Decodificando argumentos da linha de comando
        let args = Args::parse();
        // Criando o listener na porta correspondente
        let listener: Async<TcpListener> = if args.first {
            Async::<TcpListener>::bind(([127, 0, 0, 1], 6000))?
        } else {
            Async::<TcpListener>::bind(args.bind)?
        };

        // Criando o canal de comunicação entre threads, o receiver vai pro dispatcher
        let (sender, receiver) = unbounded();

        // Tentando conectar com os peers que foram passados como argumento
        let mut initial_streams: Vec<Arc<Async<TcpStream>>> = vec![];
        for peer_addr in &args.peers {
            if let Ok(stream) = Async::<TcpStream>::connect(*peer_addr).await {
                let stream_arc = Arc::new(stream);
                initial_streams.push(stream_arc.clone());
                let sender_clone = sender.clone();
                // Spawnando uma nova thread que lê mensagens enviadas por aquele peer específico
                smol::spawn(async move {
                    read_messages(sender_clone, stream_arc).await.ok();
                })
                .detach();
            }
        }

        // Definindo nossa lista de peers e nosso endereço
        let streams = Arc::new(Mutex::new(initial_streams));
        let my_addr = listener.get_ref().local_addr()?;

        // Dando boas vindas ao usuário
        println!("Escutando no endereço {}", listener.get_ref().local_addr()?);
        println!("Bem vindo à Rede de Pesca!\nFique a vontade para pascar e conversar :)");

        // Spawnando o dispatcher
        smol::spawn(dispatch(streams.clone(), receiver, my_addr)).detach();

        // Spawnando uma thread que lê o stdin e envia as mensagens do usuário para o dispatcher
        let sender_stdin = sender.clone();
        smol::spawn(async move {
            let stdin = Unblock::new(std::io::stdin());
            let mut lines = io::BufReader::new(stdin).lines();
            while let Some(Ok(line)) = lines.next().await {
                // Enviando apenas mensagens não vazias
                if !line.trim().is_empty() {
                    sender_stdin.send(Event::Message(my_addr, line)).await.ok();
                }
            }
        })
        .detach();

        loop {
            // Aceitando novas conexões em loop
            let (stream, addr) = listener.accept().await?;
            let peer = Arc::new(stream);
            let sender = sender.clone();

            // Criando a thread que lida com o novo peer
            smol::spawn(async move {
                sender.send(Event::Join(addr, peer.clone())).await.ok();
                read_messages(sender.clone(), peer).await.ok();
                sender.send(Event::Leave(addr)).await.ok();
            })
            .detach();
        }
    })
}
