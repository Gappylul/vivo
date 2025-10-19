use crate::token::Token;

pub fn lex(src: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut chars = src.chars().peekable();

    fn read_ident(chars: &mut std::iter::Peekable<impl Iterator<Item = char>>) -> String {
        let mut ident = String::new();
        while let Some(&c) = chars.peek() {
            if c.is_alphanumeric() || c == '_' {
                ident.push(c);
                chars.next();
            } else {
                break;
            }
        }
        ident
    }

    fn read_method_chain(chars: &mut std::iter::Peekable<impl Iterator<Item = char>>, s: &mut String) {
        while let Some(&c) = chars.peek() {
            if c == '.' {
                s.push(c);
                chars.next(); // skip dot

                // Read method name
                while let Some(&m) = chars.peek() {
                    s.push(m);
                    chars.next();
                    if m == ')' { break; } // stop at closing paren
                }
            } else {
                break;
            }
        }
    }

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

                while let Some(ch) = chars.next() {
                    match ch {
                        '"' => break,
                        '\\' => {
                            if let Some(next_ch) = chars.next() {
                                s.push(match next_ch {
                                    'n' => '\n',
                                    't' => '\t',
                                    'r' => '\r',
                                    '\\' => '\\',
                                    '"' => '"',
                                    other => other,
                                });
                            }
                        }
                        '$' => {
                            let mut var_expr = String::from("$");
                            var_expr.push_str(&read_ident(&mut chars));
                            read_method_chain(&mut chars, &mut var_expr);

                            s.push_str("{{");
                            s.push_str(&var_expr);
                            s.push_str("}}");
                        }
                        _ => s.push(ch),
                    }
                }

                tokens.push(Token::String(s));
            }

            '$' => {
                let var = String::from(&read_ident(&mut chars));
                tokens.push(Token::Variable(var));
            }

            c if c.is_alphabetic() => {
                let mut ident = String::from(c);
                ident.push_str(&read_ident(&mut chars));

                tokens.push(match ident.as_str() {
                    "server" => Token::Server,
                    "tcp" => Token::TCP,
                    "on" => Token::On,
                    "log" => Token::Log,
                    "send" => Token::Send,
                    _ => Token::Ident(ident),
                });
            }

            c if c.is_digit(10) => {
                let mut num = c.to_string();
                while let Some(&n) = chars.peek() {
                    if n.is_digit(10) {
                        num.push(n);
                        chars.next();
                    } else {
                        break;
                    }
                }
                tokens.push(Token::Number(num));
            }

            _ if c.is_whitespace() => {}
            _ => {}
        }
    }

    tokens.push(Token::EOF);
    tokens
}
