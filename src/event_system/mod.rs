use std::net::{SocketAddr, TcpStream};

use async_channel::{Receiver, Sender};
use async_dup::{Arc};
use std::sync::Mutex;
use smol::{io::{self, AsyncBufReadExt, AsyncWriteExt}, stream::StreamExt, Async};

mod event;

pub use event::Event;

/// Lida com os Eventos emitidos, podendo fazer broadcast
pub async fn dispatch(
    streams: Arc<Mutex<Vec<Arc<Async<TcpStream>>>>>,
    receiver: Receiver<Event>,
    my_addr: SocketAddr,
    fish_catalog: crate::ui::FishCatalog,
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
                Event::Pesca => {
                    let fishing_msg = crate::ui::fishing(&fish_catalog);
                    (fishing_msg, None)
                }
            };

            // Criando uma lista apenas com os peers que devem receber a mensagem
            let sender_addr = sender_addr_opt.unwrap_or(my_addr);
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
        match &event {
            Event::Message(_, msg) if sender_addr == my_addr => {
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
            _ => {
                print!("{}", output);
            }
        }
    }
    Ok(())
}

/// Lendo mensagens de outros peers e enviando elas para o dispatcher
pub async fn read_messages(sender: Sender<Event>, peer: Arc<Async<TcpStream>>) -> io::Result<()> {
    let addr = peer.get_ref().peer_addr()?;
    let mut lines = io::BufReader::new(peer).lines();
    // Toda vez que houver uma nova linha na stream, manda a mensagem pro dispatcher
    while let Some(line) = lines.next().await {
        let line = line?;
        sender.send(Event::Message(addr, line)).await.ok();
    }
    Ok(())
}

