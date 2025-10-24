use regex::Regex;
use std::io::Write;

/// Pergunta o nome do usuário repetidamente até que o nome obedeça as retrições: (no minimo três
/// caracteres, e contenha caracteres alfanuméricos, barra ou underscore)
pub fn ask_username() -> String {
    let mut username = String::new();
    // Regex para verificar se o nome de usuário é válido
    // Condições: min. 3 caracters, inicia com letra, é alphanum ou - ou _
    let username_pattern =
        Regex::new(r"^[A-Za-z][A-Za-z0-9_-]{2,}$").expect("Padrão de regex inválido");
    loop {
        print!("Escolha um nome de usuário: ");
        // Força o print a acontecer. Detalhes do macro print!
        std::io::stdout()
            .flush()
            .expect("Falha ao limpar o buffer da saída padrão.");
        std::io::stdin()
            .read_line(&mut username)
            .expect("Não foi possível ler da entrada padrão.");
        let name = username.trim();
        if username_pattern.is_match(name) {
            // Não podemos retornar uma referência do buffer, copiamos nome para
            //  uma String no heap e retornamos
            return name.to_owned();
        }
        println!(
            "Nome de usuário inválido. Seu nome de usuário deve\n- Começar com um letras do alfabeto\n- Ter no mínimo 3 caracteres\n- Usar apenas letras, números, hífens ou underscores."
        );
        username.clear();
    }
}
