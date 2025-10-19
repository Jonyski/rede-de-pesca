/*
 * Módulo responsável pelo Fish Net Protocol
 *
 *
 * Parte dessa seção foi gerada com base em inteligência artificial. DeepSeek v3 set. 2025
 *
 * Foi dado para a LLM o esquema do protocolo e a estrutura enum `FNP`, que então criou o
 * código para o _parser_ e as implementações `FromStr` para as estruturas do enum.
 *
 * Os testes também foram gerados por IA e corrigidos de acordo.
 *
 * Já o código do _encoder_ e as implementações de `Display` foram escritas a mão
 */

/*
 * Especificação do Fish Net Protocol.
 * Inspiração do HTTP/1.1 e do SMTP.
 *
 * FNP 1.0;
 * REM: (fnp://user@127.0.0.1:6000);
 * DEST: (*|fnp://user@129.0.0.1:4848);
 * CMD: (Message|Inspection|InvetoryShowcase|Broadcast|TradeOffer|TradeConfirm|AnnounceName|PeerList);
 * [Content|Invetory|Offer|OfferResponse|Peers]: *;
 *
 *
 * Content: "text"
 * Invetory: fish|10, fish2|100;
 * Offer: fish1|10 > fish2|10;
 * OfferResponse: true|false;
 * Peers: user1@127.0.0.1:6000,user2@127.0.0.1:6001;
 * */

use regex::Regex;
use std::collections::HashMap;
use std::fmt::Display;
use std::net::SocketAddr;
use std::str::FromStr;

/// Fish Net Protocol
#[derive(Debug, PartialEq, Clone)]
pub enum FNP {
    Message {
        rem: Peer,
        dest: Peer,
        content: String,
    },
    Broadcast {
        rem: Peer,
        content: String,
    },
    TradeOffer {
        rem: Peer,
        dest: Peer,
        offer: Offer,
    },
    TradeConfirm {
        rem: Peer,
        dest: Peer,
        response: bool,
        offer: Offer,
    },
    InventoryInspection {
        rem: Peer,
        dest: Peer,
    },
    InventoryShowcase {
        rem: Peer,
        dest: Peer,
        inventory: Inventory,
    },
    AnnounceName {
        rem: Peer,
    },
    PeerList {
        rem: Peer,
        dest: Peer,
        peers: Vec<Peer>,
    },
}

impl FNP {
    pub fn rem(&self) -> &Peer {
        match self {
            FNP::Message { rem, .. }
            | FNP::Broadcast { rem, .. }
            | FNP::TradeOffer { rem, .. }
            | FNP::TradeConfirm { rem, .. }
            | FNP::InventoryInspection { rem, .. }
            | FNP::InventoryShowcase { rem, .. }
            | FNP::AnnounceName { rem }
            | FNP::PeerList { rem, .. } => rem,
        }
    }

    pub fn dest(&self) -> Option<&Peer> {
        match self {
            FNP::Broadcast { .. } | FNP::AnnounceName { .. } => None,
            FNP::Message { dest, .. }
            | FNP::TradeOffer { dest, .. }
            | FNP::TradeConfirm { dest, .. }
            | FNP::InventoryInspection { dest, .. }
            | FNP::InventoryShowcase { dest, .. }
            | FNP::PeerList { dest, .. } => Some(dest),
        }
    }

    // This function was already correct, but is included for completeness.
    pub fn set_rem(self, rem: Peer) -> Self {
        match self {
            FNP::Message { dest, content, .. } => FNP::Message { rem, dest, content },
            FNP::Broadcast { content, .. } => FNP::Broadcast { rem, content },
            FNP::TradeOffer { dest, offer, .. } => FNP::TradeOffer { rem, dest, offer },
            FNP::TradeConfirm {
                dest,
                response,
                offer,
                ..
            } => FNP::TradeConfirm {
                rem,
                dest,
                response,
                offer,
            },
            FNP::InventoryInspection { dest, .. } => FNP::InventoryInspection { rem, dest },
            FNP::InventoryShowcase {
                dest, inventory, ..
            } => FNP::InventoryShowcase {
                rem,
                dest,
                inventory,
            },
            FNP::AnnounceName { .. } => FNP::AnnounceName { rem },
            FNP::PeerList { dest, peers, .. } => FNP::PeerList { rem, dest, peers },
        }
    }
}

/// Parser para o FNP
#[derive(Debug)]
pub struct FNPParser;

impl FNPParser {
    pub fn parse(input: &str) -> Result<FNP, String> {
        // Utiliza regex para separar uma mensagem em um hash map de cada campo
        // Regex gerada e aprimorada pelo ChatGPT-4o.
        // Divide linhas em dois grupos separados por `:`
        // Primeiro grupo é o tipo chave do protocolo ou "tag"
        // Segundo grupo é o valor, composto de caracters e ; escapados
        let fields: HashMap<_, _> = Regex::new(r"(\w+):\s*([^;\\]*(?:\\.[^;\\]*)*)?;")
            .unwrap()
            .captures_iter(&input.replace("\n", " "))
            .map(|c| (c[1].to_string(), c[2].trim().to_string()))
            .collect();

        // extrai peer e cmd
        let rem = Peer::from_str(fields.get("REM").ok_or("No REM")?)?;
        let cmd = fields.get("CMD").ok_or("No CMD")?.trim();

        // decodifica de acordo com o cmd
        match cmd {
            "Message" => Ok(FNP::Message {
                rem,
                dest: Peer::from_str(fields.get("DEST").ok_or("No DEST")?)?,
                content: unescape_semicolons(&Self::extract_quoted_content(
                    fields.get("Content").ok_or("No Content")?,
                )?),
            }),
            "Broadcast" => Ok(FNP::Broadcast {
                rem,
                content: unescape_semicolons(&Self::extract_quoted_content(
                    fields.get("Content").ok_or("No Content")?,
                )?),
            }),
            "TradeOffer" => Ok(FNP::TradeOffer {
                rem,
                dest: Peer::from_str(fields.get("DEST").ok_or("No DEST")?)?,
                offer: Offer::from_str(&unescape_semicolons(
                    fields.get("Offer").ok_or("No Offer")?,
                ))?,
            }),
            "TradeConfirm" => Ok(FNP::TradeConfirm {
                rem,
                dest: Peer::from_str(fields.get("DEST").ok_or("No DEST")?)?,
                response: fields
                    .get("Response")
                    .ok_or("No CONFIRM")?
                    .parse()
                    .map_err(|e: std::str::ParseBoolError| e.to_string())?,
                offer: Offer::from_str(&unescape_semicolons(
                    fields.get("Offer").ok_or("No Offer")?,
                ))?,
            }),
            "InvetoryShowcase" => Ok(FNP::InventoryShowcase {
                rem,
                dest: Peer::from_str(fields.get("DEST").ok_or("No DEST")?)?,
                inventory: Inventory::from_str(&unescape_semicolons(
                    fields.get("Invetory").ok_or("No Invetory")?,
                ))?,
            }),
            "InventoryInspection" => Ok(FNP::InventoryInspection {
                rem,
                dest: Peer::from_str(fields.get("DEST").ok_or("No DEST")?)?,
            }),
            "AnnounceName" => Ok(FNP::AnnounceName { rem }),
            "PeerList" => {
                let peers_str = fields.get("Peers").ok_or("No Peers")?;
                let peers = peers_str
                    .split(',')
                    .map(|s| Peer::from_str(s.trim()))
                    .collect::<Result<Vec<Peer>, _>>()?;
                Ok(FNP::PeerList {
                    rem,
                    dest: Peer::from_str(fields.get("DEST").ok_or("No DEST")?)?,
                    peers,
                })
            }
            _ => Err(format!("Unknown CMD: {}", cmd)),
        }
    }

    /// Extrai texto de dentro de aspas duplas
    fn extract_quoted_content(s: &str) -> Result<String, String> {
        let re = Regex::new(r#""([^"]*)""#).unwrap();
        re.captures(s)
            .and_then(|caps| caps.get(1))
            .map(|m| m.as_str().to_string())
            .ok_or("Invalid content format".to_string())
    }
}

/// Escapa todos os ponto-virgulas que podem não ser divisores no protocolo. Subsitui todos os `;` por `\;`
fn escape_semicolons(s: &str) -> String {
    s.replace(";", r"\;")
}

/// Desfaz a função `escape_semicolons`. Subsitui todos os `\;` por `;`
fn unescape_semicolons(s: &str) -> String {
    s.replace(r"\;", ";")
}

// Converte o protocolo para string
impl Display for FNP {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            FNP::Message { rem, dest, content } => {
                format!("FNP 1.0; REM: {rem}; DEST: {dest}; CMD: Message; Content: \"{content}\";")
            }
            FNP::Broadcast { rem, content } => {
                format!(
                    "FNP 1.0; REM: {rem}; DEST: fnp://*; CMD: Broadcast; Content: \"{content}\";"
                )
            }
            FNP::TradeOffer { rem, dest, offer } => {
                // Cria uma nova oferta com os ponto-e-virgulas escapados
                let scaped_offer = Offer {
                    offered: offer
                        .offered
                        .iter()
                        .map(|o| InventoryItem {
                            fish_type: escape_semicolons(&o.fish_type),
                            quantity: o.quantity,
                        })
                        .collect(),
                    requested: offer
                        .requested
                        .iter()
                        .map(|r| InventoryItem {
                            fish_type: escape_semicolons(&r.fish_type),
                            quantity: r.quantity,
                        })
                        .collect(),
                };
                format!(
                    "FNP 1.0; REM: {rem}; DEST: {dest}; CMD: TradeOffer; Offer: {scaped_offer};"
                )
            }
            FNP::TradeConfirm {
                rem,
                dest,
                response,
                offer,
            } => {
                // Cria uma nova oferta com os ponto-e-virgulas escapados
                let scaped_offer = Offer {
                    offered: offer
                        .offered
                        .iter()
                        .map(|o| InventoryItem {
                            fish_type: escape_semicolons(&o.fish_type),
                            quantity: o.quantity,
                        })
                        .collect(),
                    requested: offer
                        .requested
                        .iter()
                        .map(|r| InventoryItem {
                            fish_type: escape_semicolons(&r.fish_type),
                            quantity: r.quantity,
                        })
                        .collect(),
                };
                format!(
                    "FNP 1.0; REM: {rem}; DEST: {dest}; CMD: TradeConfirm; Response: {response}; Offer: {scaped_offer};"
                )
            }
            FNP::InventoryInspection { rem, dest } => {
                format!("FNP 1.0; REM: {rem}; DEST: {dest}; CMD: InventoryInspection;")
            }
            FNP::InventoryShowcase {
                rem,
                dest,
                inventory,
            } => {
                // Cria um novo inventario com o pontos e virgulas escapados
                let inventory = Inventory {
                    items: inventory
                        .items
                        .iter()
                        .map(|i| InventoryItem {
                            fish_type: escape_semicolons(&i.fish_type),
                            quantity: i.quantity,
                        })
                        .collect(),
                };
                format!(
                    "FNP 1.0; REM: {rem}; DEST: {dest}; CMD: InvetoryShowcase; Invetory: {inventory};"
                )
            }
            FNP::AnnounceName { rem } => {
                format!("FNP 1.0; REM: {rem}; CMD: AnnounceName;")
            }
            FNP::PeerList { rem, dest, peers } => {
                let peers_str = peers
                    .iter()
                    .map(|p| p.to_string())
                    .collect::<Vec<_>>()
                    .join(",");
                format!("FNP 1.0; REM: {rem}; DEST: {dest}; CMD: PeerList; Peers: {peers_str};")
            }
        };
        write!(f, "{}", s)
    }
}

// Peer que representa um endereço de socket com o prefixo fnp://
#[derive(Debug, PartialEq, Clone)]
pub struct Peer {
    username: String,
    address: SocketAddr,
}

impl Peer {
    pub fn new(username: String, address: SocketAddr) -> Self {
        Self { username, address }
    }

    pub fn address(&self) -> SocketAddr {
        self.address
    }

    pub fn username(&self) -> &str {
        &self.username
    }
}

impl FromStr for Peer {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let sanitized = match s.strip_prefix("fnp://") {
            Some(striped) => striped,
            None => s,
        };

        let parts: Vec<&str> = sanitized.split('@').collect();
        if parts.len() != 2 {
            return Err("Invalid peer format".to_string());
        }

        let username = parts[0].to_string();
        let address = parts[1].parse::<SocketAddr>().map_err(|e| e.to_string())?;
        Ok(Self { username, address })
    }
}

impl Display for Peer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "fnp://{}@{}", self.username, self.address)
    }
}

/// Item do inventario. Peixe e quantidade
#[derive(Debug, PartialEq, Clone)]
pub struct InventoryItem {
    pub fish_type: String,
    pub quantity: u32,
}

impl InventoryItem {
    pub fn new(fish_type: String, quantity: u32) -> Self {
        Self {
            fish_type,
            quantity,
        }
    }
}

impl Display for InventoryItem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}|{}", self.fish_type, self.quantity)
    }
}

// Inventário, lista de peixes com quantidade
#[derive(Debug, PartialEq, Clone)]
pub struct Inventory {
    pub items: Vec<InventoryItem>,
}

impl FromStr for Inventory {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let items: Result<Vec<InventoryItem>, _> = s
            .split(',')
            .map(|item| item.trim())
            .filter(|item| !item.is_empty())
            .map(|item| {
                let parts: Vec<&str> = item.split('|').collect();
                if parts.len() != 2 {
                    dbg!(parts);
                    return Err("Invalid inventory item format".to_string());
                }
                let fish_type = parts[0].trim().to_string();
                let quantity = parts[1]
                    .trim()
                    .parse()
                    .map_err(|_| "Invalid quantity".to_string())?;
                Ok(InventoryItem {
                    fish_type,
                    quantity,
                })
            })
            .collect();

        Ok(Inventory { items: items? })
    }
}

impl Display for Inventory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // converte cada item para string, e junta todos separados por virgula e espaço
        let s = self
            .items
            .iter()
            .map(|i| i.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        write!(f, "{s}")
    }
}

/// Oferta de troca
#[derive(Debug, PartialEq, Clone)]
pub struct Offer {
    pub offered: Vec<InventoryItem>,
    pub requested: Vec<InventoryItem>,
}

impl FromStr for Offer {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split('>').collect();
        if parts.len() != 2 {
            return Err("Invalid offer format".to_string());
        }

        let parse_items = |s: &str| -> Result<Vec<InventoryItem>, String> {
            s.split(',')
                .map(|item| item.trim())
                .filter(|item| !item.is_empty())
                .map(|item| {
                    let parts: Vec<&str> = item.split('|').collect();
                    if parts.len() != 2 {
                        return Err("Invalid offer item format".to_string());
                    }
                    let fish_type = parts[0].trim().to_string();
                    let quantity = parts[1]
                        .trim()
                        .parse()
                        .map_err(|_| "Invalid quantity".to_string())?;
                    Ok(InventoryItem {
                        fish_type,
                        quantity,
                    })
                })
                .collect()
        };

        let offered = parse_items(parts[0])?;
        let requested = parse_items(parts[1])?;

        Ok(Offer { offered, requested })
    }
}

impl Display for Offer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let offered: String = self
            .offered
            .iter()
            .map(|item| format!("{}|{}", item.fish_type, item.quantity))
            .collect::<Vec<_>>()
            .join(",");

        let requested: String = self
            .requested
            .iter()
            .map(|item| format!("{}|{}", item.fish_type, item.quantity))
            .collect::<Vec<_>>()
            .join(",");

        write!(f, "{} > {}", offered, requested)
    }
}

#[derive(Debug, Clone, Default)]
pub struct OfferBuff {
    pub offers_made: HashMap<SocketAddr, Offer>,
    pub offers_received: HashMap<SocketAddr, Offer>,
}

#[cfg(test)]
mod tests {
    use super::*;

    // Testes para Peer
    #[test]
    fn test_peer_parsing_valid() {
        let peer: Peer = "fnp://user@127.0.0.1:6000".parse().unwrap();
        assert_eq!(peer.username, "user");
        assert_eq!(peer.address, "127.0.0.1:6000".parse().unwrap());
    }

    #[test]
    fn test_peer_parsing_invalid() {
        let result = "invalid".parse::<Peer>();
        assert!(result.is_err());
    }

    // Testes para InventoryItem e Inventory
    #[test]
    fn test_inventory_parsing_single_item() {
        let inventory: Inventory = "fish1|10".parse().unwrap();
        assert_eq!(inventory.items.len(), 1);
        assert_eq!(inventory.items[0].fish_type, "fish1");
        assert_eq!(inventory.items[0].quantity, 10);
    }

    #[test]
    fn test_inventory_parsing_multiple_items() {
        let inventory: Inventory = "fish1|10, fish2|20, goldfish|5".parse().unwrap();
        assert_eq!(inventory.items.len(), 3);
        assert_eq!(inventory.items[1].fish_type, "fish2");
        assert_eq!(inventory.items[2].quantity, 5);
    }

    #[test]
    fn test_inventory_parsing_invalid() {
        let result = "invalid".parse::<Inventory>();
        assert!(result.is_err());
    }

    // Testes para Offer
    #[test]
    fn test_offer_parsing_simple() {
        let offer: Offer = "fish1|10 > fish2|5".parse().unwrap();
        assert_eq!(offer.offered.len(), 1);
        assert_eq!(offer.requested.len(), 1);
        assert_eq!(offer.offered[0].fish_type, "fish1");
        assert_eq!(offer.requested[0].quantity, 5);
    }

    #[test]
    fn test_offer_parsing_complex() {
        let offer: Offer = "fish1|10,fish2|5 > fish3|3,fish4|2".parse().unwrap();
        assert_eq!(offer.offered.len(), 2);
        assert_eq!(offer.requested.len(), 2);
        assert_eq!(offer.offered[1].fish_type, "fish2");
        assert_eq!(offer.requested[1].quantity, 2);
    }

    #[test]
    fn test_offer_parsing_invalid() {
        let result = "invalid".parse::<Offer>();
        assert!(result.is_err());
    }

    // Testes para Display implementations
    #[test]
    fn test_peer_display() {
        let peer = Peer {
            username: "user".to_string(),
            address: "127.0.0.1:6000".parse().unwrap(),
        };
        assert_eq!(peer.to_string(), "fnp://user@127.0.0.1:6000");
    }

    #[test]
    fn test_inventory_item_display() {
        let item = InventoryItem {
            fish_type: "goldfish".to_string(),
            quantity: 10,
        };
        assert_eq!(item.to_string(), "goldfish|10");
    }

    #[test]
    fn test_inventory_display() {
        let inventory = Inventory {
            items: vec![
                InventoryItem {
                    fish_type: "fish1".to_string(),
                    quantity: 10,
                },
                InventoryItem {
                    fish_type: "fish2".to_string(),
                    quantity: 5,
                },
            ],
        };
        assert_eq!(inventory.to_string(), "fish1|10, fish2|5");
    }

    #[test]
    fn test_offer_display() {
        let offer = Offer {
            offered: vec![InventoryItem {
                fish_type: "fish1".to_string(),
                quantity: 10,
            }],
            requested: vec![InventoryItem {
                fish_type: "fish2".to_string(),
                quantity: 5,
            }],
        };
        assert_eq!(offer.to_string(), "fish1|10 > fish2|5");
    }

    // Testes específicos para cada tipo de mensagem FNP
    #[test]
    fn test_message_parsing() {
        let protocol = r#"
            REM: fnp://user@127.0.0.1:6000;
            DEST: fnp://user2@129.0.0.1:4848;
            CMD: Message;
            Content: "Hello World";
        "#;

        match FNPParser::parse(protocol) {
            Ok(FNP::Message { rem, dest, content }) => {
                assert_eq!(rem.username(), "user");
                assert_eq!(rem.address().to_string(), "127.0.0.1:6000");
                assert_eq!(dest.username(), "user2");
                assert_eq!(dest.address().to_string(), "129.0.0.1:4848");
                assert_eq!(content, "Hello World");
            }
            _ => panic!("Should parse as Message"),
        }
    }

    #[test]
    fn test_broadcast_parsing() {
        let protocol = r#"
            REM: fnp://user@127.0.0.1:6000;
            DEST: *;
            CMD: Broadcast;
            Content: "Broadcast message";
        "#;

        match FNPParser::parse(protocol) {
            Ok(FNP::Broadcast { rem, content }) => {
                assert_eq!(rem.username(), "user");
                assert_eq!(rem.address().to_string(), "127.0.0.1:6000");
                assert_eq!(content, "Broadcast message");
            }
            _ => panic!("Should parse as Broadcast"),
        }
    }

    #[test]
    fn test_trade_offer_parsing() {
        let protocol = r#"
            REM: fnp://user@127.0.0.1:6000;
            DEST: fnp://user2@129.0.0.1:4848;
            CMD: TradeOffer;
            Offer: fish1|10 > fish2|5;
        "#;

        match FNPParser::parse(protocol) {
            Ok(FNP::TradeOffer { rem, dest, offer }) => {
                assert_eq!(rem.username(), "user");
                assert_eq!(rem.address().to_string(), "127.0.0.1:6000");
                assert_eq!(dest.username(), "user2");
                assert_eq!(dest.address().to_string(), "129.0.0.1:4848");
                assert_eq!(offer.offered.len(), 1);
                assert_eq!(offer.requested.len(), 1);
                assert_eq!(offer.offered[0].fish_type, "fish1");
                assert_eq!(offer.requested[0].quantity, 5);
            }
            _ => panic!("Should parse as TradeOffer"),
        }
    }

    #[test]
    fn test_trade_confirm_parsing_true() {
        let protocol = r#"
            REM: fnp://user@127.0.0.1:6000;
            DEST: fnp://user2@129.0.0.1:4848;
            CMD: TradeConfirm;
            Response: true;
            Offer: fish1|10 > fish2|5;
        "#;

        match FNPParser::parse(protocol) {
            Ok(FNP::TradeConfirm {
                rem,
                dest,
                response,
                offer,
            }) => {
                assert_eq!(rem.username(), "user");
                assert_eq!(rem.address().to_string(), "127.0.0.1:6000");
                assert_eq!(dest.username(), "user2");
                assert_eq!(dest.address().to_string(), "129.0.0.1:4848");
                assert!(response);
                assert_eq!(offer.offered.len(), 1);
                assert_eq!(offer.requested.len(), 1);
                assert_eq!(offer.offered[0].fish_type, "fish1");
                assert_eq!(offer.requested[0].quantity, 5);
            }
            _ => panic!("Should parse as TradeConfirm with true"),
        }
    }

    #[test]
    fn test_trade_confirm_parsing_false() {
        let protocol = r#"
            REM: fnp://user@127.0.0.1:6000;
            DEST: fnp://user2@129.0.0.1:4848;
            CMD: TradeConfirm;
            Response: false;
            Offer: fish1|10 > fish2|5;
        "#;

        match FNPParser::parse(protocol) {
            Ok(FNP::TradeConfirm {
                rem,
                dest,
                response,
                offer: _,
            }) => {
                assert_eq!(rem.username(), "user");
                assert_eq!(rem.address().to_string(), "127.0.0.1:6000");
                assert_eq!(dest.username(), "user2");
                assert_eq!(dest.address().to_string(), "129.0.0.1:4848");
                assert!(!response);
            }
            _ => panic!("Should parse as TradeConfirm with false"),
        }
    }

    #[test]
    fn test_inventory_inspection_parsing() {
        let protocol = r#"
            REM: fnp://user@127.0.0.1:6000;
            DEST: fnp://user2@129.0.0.1:4848;
            CMD: InventoryInspection;
        "#;

        match FNPParser::parse(protocol) {
            Ok(FNP::InventoryInspection { rem, dest }) => {
                assert_eq!(rem.username(), "user");
                assert_eq!(rem.address().to_string(), "127.0.0.1:6000");
                assert_eq!(dest.username(), "user2");
                assert_eq!(dest.address().to_string(), "129.0.0.1:4848");
            }
            _ => panic!("Should parse as InventoryInspection"),
        }
    }

    #[test]
    fn test_inventory_showcase_parsing() {
        let protocol = r#"
            REM: fnp://user@127.0.0.1:6000;
            DEST: fnp://user2@129.0.0.1:4848;
            CMD: InvetoryShowcase;
            Invetory: goldfish|10, shark|1, tuna|5;
        "#;

        match FNPParser::parse(protocol) {
            Ok(FNP::InventoryShowcase {
                rem,
                dest,
                inventory,
            }) => {
                assert_eq!(rem.username(), "user");
                assert_eq!(rem.address().to_string(), "127.0.0.1:6000");
                assert_eq!(dest.username(), "user2");
                assert_eq!(dest.address().to_string(), "129.0.0.1:4848");
                assert_eq!(inventory.items.len(), 3);
                assert_eq!(inventory.items[0].fish_type, "goldfish");
                assert_eq!(inventory.items[1].quantity, 1);
            }
            _ => panic!("Should parse as InventoryShowcase"),
        }
    }

    #[test]
    fn test_announce_name_parsing() {
        let protocol = r#"
            REM: fnp://new_user@127.0.0.1:6001;
            CMD: AnnounceName;
        "#;

        match FNPParser::parse(protocol) {
            Ok(FNP::AnnounceName { rem }) => {
                assert_eq!(rem.username(), "new_user");
                assert_eq!(rem.address().to_string(), "127.0.0.1:6001");
            }
            _ => panic!("Should parse as AnnounceName"),
        }
    }

    #[test]
    fn test_peer_list_parsing() {
        let protocol = r#"
            REM: fnp://user1@127.0.0.1:6000;
            DEST: fnp://new_user@127.0.0.1:6001;
            CMD: PeerList;
            Peers: fnp://user1@127.0.0.1:6000,fnp://user2@127.0.0.1:6002;
        "#;

        match FNPParser::parse(protocol) {
            Ok(FNP::PeerList { rem, dest, peers }) => {
                assert_eq!(rem.username(), "user1");
                assert_eq!(dest.username(), "new_user");
                assert_eq!(peers.len(), 2);
                assert_eq!(peers[0].username(), "user1");
                assert_eq!(peers[1].address().to_string(), "127.0.0.1:6002");
            }
            _ => panic!("Should parse as PeerList"),
        }
    }

    // Testes de erro
    #[test]
    fn test_missing_rem_field() {
        let protocol = r#"
            DEST: fnp://user2@129.0.0.1:4848;
            CMD: Message;
            Content: "test";
        "#;

        let result = FNPParser::parse(protocol);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("REM"));
    }

    #[test]
    fn test_missing_cmd_field() {
        let protocol = r#"
            REM: fnp://user@127.0.0.1:6000;
            DEST: fnp://user2@129.0.0.1:4848;
            Content: "test";
        "#;

        let result = FNPParser::parse(protocol);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("CMD"));
    }

    #[test]
    fn test_unknown_command() {
        let protocol = r#"
            REM: fnp://user@127.0.0.1:6000;
            DEST: fnp://user2@129.0.0.1:4848;
            CMD: UnknownCommand;
            Content: "test";
        "#;

        let result = FNPParser::parse(protocol);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("UnknownCommand"));
    }

    #[test]
    fn test_invalid_offer_response() {
        let protocol = r#"
            REM: fnp://user@127.0.0.1:6000;
            DEST: fnp://user2@129.0.0.1:4848;
            CMD: TradeConfirm;
            Response: invalid;
            Offer: fish1|1 > fish2|1;
        "#;

        let result = FNPParser::parse(protocol);
        assert!(result.is_err());
    }

    // Testes de formato inválido
    #[test]
    fn test_invalid_peer_format() {
        let protocol = r#"
            REM: invalid_format;
            DEST: fnp://user2@129.0.0.1:4848;
            CMD: Message;
            Content: "test";
        "#;

        let result = FNPParser::parse(protocol);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_inventory_format() {
        let protocol = r#"
            REM: fnp://user@127.0.0.1:6000;
            DEST: fnp://user2@129.0.0.1:4848;
            CMD: InvetoryShowcase;
            Invetory: invalid;
        "#;

        let result = FNPParser::parse(protocol);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_offer_format() {
        let protocol = r#"
            REM: fnp://user@127.0.0.1:6000;
            DEST: fnp://user2@129.0.0.1:4848;
            CMD: TradeOffer;
            Offer: invalid;
        "#;

        let result = FNPParser::parse(protocol);
        assert!(result.is_err());
    }

    // Testes de edge cases
    #[test]
    fn test_empty_content() {
        let protocol = r#"
            REM: fnp://user@127.0.0.1:6000;
            DEST: fnp://user2@129.0.0.1:4848;
            CMD: Message;
            Content: "";
        "#;

        match FNPParser::parse(protocol) {
            Ok(FNP::Message { content, .. }) => {
                assert_eq!(content, "");
            }
            _ => panic!("Should parse empty content"),
        }
    }

    #[test]
    fn test_special_characters_in_content() {
        let protocol = r#"
            REM: fnp://user@127.0.0.1:6000;
            DEST: fnp://user2@129.0.0.1:4848;
            CMD: Message;
            Content: "Hello\nWorld\tTest";
        "#;

        match FNPParser::parse(protocol) {
            Ok(FNP::Message { content, .. }) => {
                assert_eq!(content, "Hello\\nWorld\\tTest");
            }
            _ => panic!("Should parse special characters"),
        }
    }

    #[test]
    fn test_complex_trade_offer() {
        let protocol = r#"
            REM: fnp://user@127.0.0.1:6000;
            DEST: fnp://user2@129.0.0.1:4848;
            CMD: TradeOffer;
            Offer: fish1|10,fish2|5,fish3|3 > fish4|8,fish5|2;
        "#;

        match FNPParser::parse(protocol) {
            Ok(FNP::TradeOffer { offer, .. }) => {
                assert_eq!(offer.offered.len(), 3);
                assert_eq!(offer.requested.len(), 2);
                assert_eq!(offer.offered[2].fish_type, "fish3");
                assert_eq!(offer.requested[1].quantity, 2);
            }
            _ => panic!("Should parse complex trade offer"),
        }
    }

    // Testes de formato com diferentes espaçamentos
    #[test]
    fn test_various_spacing_formats() {
        let protocols = vec![
            r#"REM:fnp://u@1.1.1.1:1;DEST:fnp://u2@2.2.2.2:2;CMD:Message;Content:"test";"#,
            r#"REM: fnp://u@1.1.1.1:1 ; DEST: fnp://u2@2.2.2.2:2 ; CMD: Message; Content: "test" ;"#,
            r#"  REM: fnp://u@1.1.1.1:1;
                DEST: fnp://u2@2.2.2.2:2;
                CMD: Message;
                Content: "test";"#,
        ];

        for protocol in protocols {
            let result = FNPParser::parse(protocol);
            assert!(result.is_ok(), "Failed to parse protocol: {}", protocol);
        }
    }
}

// Testes de integração
#[test]
fn test_complete_round_trip() {
    let original_protocol = r#"
        REM: fnp://user@127.0.0.1:6000;
        DEST: fnp://user2@129.0.0.1:4848;
        CMD: TradeOffer;
        Offer: goldfish|10,shark|1 > tuna|5;
    "#;

    let fnp = FNPParser::parse(original_protocol).unwrap();

    if let FNP::TradeOffer { rem, dest, offer } = fnp {
        assert_eq!(rem.username(), "user");
        assert_eq!(rem.address().to_string(), "127.0.0.1:6000");
        assert_eq!(dest.username(), "user2");
        assert_eq!(dest.address().to_string(), "129.0.0.1:4848");
        assert_eq!(offer.offered.len(), 2);
        assert_eq!(offer.requested.len(), 1);
        assert_eq!(offer.to_string(), "goldfish|10,shark|1 > tuna|5");
    } else {
        panic!("Round trip test failed");
    }
}
