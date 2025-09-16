use std::net::{SocketAddr, TcpStream};

use async_dup::Arc;
use smol::Async;

/// Tipos de eventos/mensagens do nosso sistema
#[derive(Debug)]
pub enum Event {
    /// Um peer se conectou na rede
    Join(SocketAddr, Arc<Async<TcpStream>>),
    /// Um peer saiu
    Leave(SocketAddr),
    /// Um peer enviou uma mensagem
    Message(SocketAddr, String),
    /// Usuário realizou a ação "pescar"
    Pesca,
}


