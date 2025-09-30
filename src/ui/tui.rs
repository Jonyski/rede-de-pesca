use std::net::SocketAddr;

use async_channel::Sender;
use smol::{io::{self, AsyncBufReadExt}, stream::StreamExt, Unblock};

use crate::{event_system::Event, fishnet_server::{protocol::Peer, FNP}};

pub async fn eval(sender_stdin: Sender<Event>, my_addr: SocketAddr) {
    // lê o stdin e envia as mensagens do usuário para o dispatcher
    let stdin = Unblock::new(std::io::stdin());
    let mut lines = io::BufReader::new(stdin).lines();
    while let Some(Ok(line)) = lines.next().await {
        // Enviando apenas mensagens não vazias
        if !line.trim().is_empty() {
            if line == "$p" || line == "$pescar" {
                sender_stdin.send(Event::Pesca).await.ok();
            } else {
                let msg = FNP::Broadcast { rem: Peer::new(my_addr), content: line};
                sender_stdin.send(Event::Message(my_addr, msg)).await.ok();
            }
        }
    }
}
