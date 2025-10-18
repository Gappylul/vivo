use crate::token::Token;

pub fn lex(src: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut chars = src.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            '{' => tokens.push(Token::LBrace),
            '}' => tokens.push(Token::RBrace),
            '(' => tokens.push(Token::LParen),
            ')' => tokens.push(Token::RParen),
            ':' => tokens.push(Token::Colon),
            '"' => {
                let mut s = String::new();
                while let Some(ch) = chars.next() {
                    if ch == '"' { break; }
                    s.push(ch);
                }
                tokens.push(Token::String(s));
            }
            c if c.is_alphabetic() => {
                let mut ident = c.to_string();
                while let Some(&n) = chars.peek() {
                    if n.is_alphanumeric() { ident.push(n); chars.next(); }
                    else { break; }
                }
                tokens.push(match ident.as_str() {
                    "server" => Token::Server,
                    "tcp" => Token::TCP,
                    "on" => Token::On,
                    "log" => Token::Log,
                    "send" => Token::Send,
                    _ => Token::Ident(ident),
                });
            }
            _ if c.is_whitespace() => {}
            _ => {}
        }
    }

    tokens.push(Token::EOF);
    tokens
}