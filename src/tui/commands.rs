//! Parser simples para comandos de linha de comando da TUI.
//!
//! A função principal é `parse_command` que aceita uma linha de entrada e
//! retorna `Some(Command)` se a linha começar com `$`, ou `None` caso contrário.
//!
//! Os comandos representados aqui refletem os que estavam em `eval()`:
//!  - `$p` / `$pescar`
//!  - `$l` / `$listar`
//!  - `$i` / `$inventario` [<peer>]
//!  - `$t` / `$troca` <peer> <offer...>
/// - `$c` / `$confirmar` <s|n> <peer>
/// - `$q` / `$quit`
/// - `$h` / `$help`

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Command {
    Pescar,
    List,
    Inventario(Option<String>),
    Trade { peer_str: String, offer_str: String },
    ConfirmTrade { resp: bool, peer_str: String },
    Quit,
    Help,
    Unknown(String),
}

/// Parser para comandos da UI. Lê uma linha e identifica se há um comando com
///  os argumentos corretos
pub fn parse_command(line: &str) -> Option<Command> {
    let line = line.trim();

    if line.is_empty() {
        return None;
    }
    if !line.starts_with("$") {
        return None;
    }

    let parts = line.split_whitespace().collect::<Vec<_>>();
    let cmd = parts.get(0).map(|s| s.to_lowercase()).unwrap_or_default();
    match cmd.as_str() {
        "$p" | "$pescar" => Some(Command::Pescar),
        "$l" | "$listar" => Some(Command::List),
        "$i" | "$inventario" => {
            let arg = parts.get(1).map(|s| s.to_string());
            Some(Command::Inventario(arg))
        }
        "$t" | "$troca" => {
            let peer_str = parts.get(1).map(|s| s.to_string()).unwrap_or_default();
            let offer_str = parts.get(2..).unwrap_or(&[]).join(" ");
            Some(Command::Trade {
                peer_str,
                offer_str,
            })
        }
        "$c" | "$confirmar" => {
            let resp = parts
                .get(1)
                .map(|s| s.to_lowercase())
                .map(|s| s == "s" || s == "sim")
                .unwrap_or(false);
            let peer_str = parts.get(2).map(|s| s.to_string()).unwrap_or_default();
            Some(Command::ConfirmTrade { resp, peer_str })
        }
        "$q" | "$quit" => Some(Command::Quit),
        "$h" | "$help" => Some(Command::Help),
        _ => Some(Command::Unknown(line.to_string())),
    }
}

/*
 *
 * Testes gerados pelo ChatGpt 5.0 mini
 */
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_pescar_and_listar() {
        assert_eq!(parse_command("$p"), Some(Command::Pescar));
        assert_eq!(parse_command("  $pescar  "), Some(Command::Pescar));
        assert_eq!(parse_command("$l"), Some(Command::List));
        assert_eq!(parse_command("$listar extra ignored"), Some(Command::List));
    }

    #[test]
    fn parse_inventario() {
        assert_eq!(parse_command("$i"), Some(Command::Inventario(None)));
        assert_eq!(
            parse_command("$i alice"),
            Some(Command::Inventario(Some("alice".to_string())))
        );
    }

    #[test]
    fn parse_troca_and_offer_reconstruction() {
        let input = "$t bob peixe|2 > peixe|1";
        assert_eq!(
            parse_command(input),
            Some(Command::Trade {
                peer_str: "bob".to_string(),
                offer_str: "peixe|2 > peixe|1".to_string()
            })
        );

        // Missing peer -> peer empty string, offer empty
        assert_eq!(
            parse_command("$t"),
            Some(Command::Trade {
                peer_str: "".to_string(),
                offer_str: "".to_string()
            })
        );
    }

    #[test]
    fn parse_confirmar() {
        assert_eq!(
            parse_command("$c s alice"),
            Some(Command::ConfirmTrade {
                resp: true,
                peer_str: "alice".to_string()
            })
        );
        assert_eq!(
            parse_command("$c n bob"),
            Some(Command::ConfirmTrade {
                resp: false,
                peer_str: "bob".to_string()
            })
        );
        assert_eq!(
            parse_command("$c sim carla"),
            Some(Command::ConfirmTrade {
                resp: true,
                peer_str: "carla".to_string()
            })
        );
    }

    #[test]
    fn parse_quit_and_help() {
        assert_eq!(parse_command("$q"), Some(Command::Quit));
        assert_eq!(parse_command("$h"), Some(Command::Help));
    }

    #[test]
    fn not_a_command_returns_none() {
        assert_eq!(parse_command("hello world"), None);
        assert_eq!(parse_command(" @bob hi"), None);
        assert_eq!(parse_command(""), None);
        assert_eq!(parse_command("   "), None);
    }

    #[test]
    fn unknown_command_returns_unknown() {
        assert_eq!(
            parse_command("$unknown cmd args"),
            Some(Command::Unknown("$unknown cmd args".to_string()))
        );
    }
}
