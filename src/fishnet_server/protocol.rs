
#[derive(Debug)]
pub struct Peer(String);

#[derive(Debug)]
pub struct Offer;

/// Fish Net Protocol
/// Protocolo de comunicação entre o sistema rede de pesca
#[derive(Debug)]
pub enum FNP {
    /// Mensagem de propósito geral com destinatário
    Message {rem: Peer, dest: Peer, content: String},
    /// Pedido de inspeção de inventário
    Inspection {target: Peer},
    /// Mensagem de broadcast para todos os usuários
    Broadcast {rem: Peer, content: String},
    /// Oferta de troca, inclui oferta sendo feita
    TradeOffer {dest: Peer, offer: Offer},
    /// Confirmar trocar
    ConfirmTrade(bool),
}



impl FNP {
    
    pub fn encode(msg: &str) -> FNP {
        todo!()
    }

    pub fn decode(proc: FNP) -> String {
        todo!()
    }
}
