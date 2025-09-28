use std::net::{SocketAddr, TcpStream};

use async_dup::Arc;
use smol::Async;

use crate::fishnet_server::FNP;

/// Tipos de eventos/mensagens do nosso sistema
#[derive(Debug)]
pub enum Event {
    /// Um peer se conectou na rede
    Join(SocketAddr, Arc<Async<TcpStream>>),
    /// Um peer saiu
    Leave(SocketAddr),
    /// Um peer enviou uma mensagem
    Message(SocketAddr, FNP),
    /// Usuário realizou a ação "pescar"
    Pesca,
}


