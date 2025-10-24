mod cli;
mod commands;
mod handler;
mod io;
mod style;

pub use cli::Args;
pub use io::ask_username;
pub use style::err;
pub use style::log;

use crate::AppState;
use crate::Event;
use crate::server;
use crate::server::Peer;
use crate::server::peerstore::PeerStore;
use crate::tui::commands::parse_command;
use crate::tui::handler::handle_command;
use async_channel::Sender;
use smol::Unblock;
use smol::io::{AsyncBufReadExt, BufReader};
use smol::stream::StreamExt;
use std::sync::Arc;

/// Loop para a interface do usuário, aguarda entradas de texto e emite sinais de acordo.
pub async fn eval(
    app_state: Arc<AppState>,
    peer_store: Arc<PeerStore>,
    sender: Sender<Event>,
    my_peer: Peer,
) {
    let stdin = Unblock::new(std::io::stdin());
    let mut lines = BufReader::new(stdin).lines();

    while let Some(Ok(line)) = lines.next().await {
        if let Some(cmd) = parse_command(&line) {
            handle_command(
                cmd,
                app_state.clone(),
                peer_store.clone(),
                sender.clone(),
                my_peer.clone(),
            )
            .await;
            continue;
        }

        let msg = if line.starts_with("@") {
            if let Some((peer_name, text)) = line.split_once(' ') {
                let peer_name = peer_name.strip_prefix('@').unwrap_or(peer_name);
                if let Some(peer_info) = peer_store.get_by_username(peer_name).await {
                    server::FNP::Message {
                        rem: my_peer.clone(),
                        dest: peer_info.peer.clone(),
                        content: text.to_string(),
                    }
                } else {
                    err("Peer não encontrado.");
                    continue;
                }
            } else {
                err("Formato de mensagem inválido. Use @username <message>");
                continue;
            }
        } else {
            server::FNP::Broadcast {
                rem: my_peer.clone(),
                content: line,
            }
        };

        sender.send(Event::UIMessage(msg)).await.ok();
    }
}
