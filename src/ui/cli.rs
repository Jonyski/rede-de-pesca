use std::net::SocketAddr;

use clap::Parser;

/// Argumentos da linha de comando
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
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

impl Args {
    pub fn first(&self) -> bool {
        self.first
    }

    pub fn peers(&self) -> &[SocketAddr] {
        &self.peers
    }

    pub fn bind(&self) -> SocketAddr {
        self.bind
    }
}

/// Parseando e validando os endereços
fn parse_addr(s: &str) -> Result<SocketAddr, String> {
    if let Ok(addr) = s.parse::<SocketAddr>() {
        return Ok(addr);
    }
    Err(format!("Invalid bind/peer address: {}", s))
}


