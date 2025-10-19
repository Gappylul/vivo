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
            '.' => tokens.push(Token::Dot),
            '"' => {
                let mut s = String::new();
                loop {
                    match chars.next() {
                        Some('"') => break,
                        Some('\\') => {
                            if let Some(next_ch) = chars.next() {
                                match next_ch {
                                    'n' => s.push('\n'),
                                    't' => s.push('\t'),
                                    'r' => s.push('\r'),
                                    '\\' => s.push('\\'),
                                    '"' => s.push('"'),
                                    _ => {
                                        s.push('\\');
                                        s.push(next_ch);
                                    }
                                }
                            }
                        }
                        Some('$') => {
                            s.push_str("{{$");
                            while let Some(&n) = chars.peek() {
                                if n.is_alphanumeric() || n == '_' {
                                    s.push(n);
                                    chars.next();
                                } else {
                                    break;
                                }
                            }
                            s.push_str("}}");
                        }
                        Some(ch) => s.push(ch),
                        None => break,
                    }
                }
                tokens.push(Token::String(s));
            }
            '$' => {
                let mut var = String::new();
                while let Some(&n) = chars.peek() {
                    if n.is_alphanumeric() || n == '_' {
                        var.push(n);
                        chars.next();
                    }
                    else { break; }
                }
                tokens.push(Token::Variable(var));
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