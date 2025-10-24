/*
 * Módulo de eventos do sistema
 *
 * Armazena eventos possíveis dentro do sistema
 * Bem como handlers para esses eventos.
 *
 */

pub mod handlers;

use std::net::SocketAddr;
use crate::server;

/// Os 4 tipos de eventos com os quais o dispatcher lida
pub enum Event {
    /// Foi percebido que um peer saiu da rede
    PeerDisconnected(SocketAddr),
    /// Mensagem FNP chegando de um peer
    ServerMessage(server::FNP),
    /// Mensagem FNP chegando do próprio peer para ser enviada a outro(s)
    UIMessage(server::FNP),
    /// O peer está tentando pescar
    Pesca,
}
