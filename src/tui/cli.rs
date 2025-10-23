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
    #[arg(short, long, value_parser = parse_addr, required_unless_present = "first")]
    bind: Option<SocketAddr>,
}

impl Args {
    pub fn first(&self) -> bool {
        self.first
    }

    pub fn peers(&self) -> &[SocketAddr] {
        &self.peers
    }

    pub fn bind(&self) -> SocketAddr {
        self.bind.unwrap()
    }
}

/// Parseando e validando os endereços
fn parse_addr(s: &str) -> Result<SocketAddr, String> {
    s.parse().map_err(|e| format!("Invalid bind/peer address: {}", e))
}
