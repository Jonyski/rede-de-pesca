use owo_colors::Style;


pub fn log(msg: &str) {
    println!("{}", style_log_msg(msg));
}

pub fn err(err_msg: &str) {
    println!("{}", style_err_msg(err_msg));
}

/// Colori a mensagem de cinza
fn style_log_msg(msg: &str) -> String {
    Style::new()
        .fg_rgb::<170, 190, 205>()
        .italic()
        .style(msg)
        .to_string()
}

/// Colori a mensagem de vermelho
fn style_err_msg(err_msg: &str) -> String {
    Style::new()
        .fg_rgb::<220, 40, 80>()
        .italic()
        .style(err_msg)
        .to_string()
}
