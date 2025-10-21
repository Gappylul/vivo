use crate::token::Token;
use std::fmt;

#[derive(Debug)]
pub enum LexError {
    UnterminatedString { line: usize, column: usize },
    InvalidEscape { line: usize, column: usize, character: char },
    UnexpectedCharacter { line: usize, column: usize, character: char },
}

impl fmt::Display for LexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LexError::UnterminatedString { line, column } => {
                write!(f, "Unterminated string at line {}, column {}", line, column)
            }
            LexError::InvalidEscape { line, column, character } => {
                write!(
                    f,
                    "Invalid escape sequence '\\{}' at line {}, column {}",
                    character, line, column
                )
            }
            LexError::UnexpectedCharacter { line, column, character } => {
                write!(
                    f,
                    "Unexpected character '{}' at line {}, column {}",
                    character, line, column
                )
            }
        }
    }
}

impl std::error::Error for LexError {}

pub fn lex(src: &str) -> Result<Vec<Token>, LexError> {
    let mut tokens = Vec::new();
    let mut chars = src.chars().peekable();
    let mut line = 1;
    let mut column = 1;

    fn read_ident(
        chars: &mut std::iter::Peekable<impl Iterator<Item = char>>,
        column: &mut usize,
    ) -> String {
        let mut ident = String::new();
        while let Some(&c) = chars.peek() {
            if c.is_alphanumeric() || c == '_' {
                ident.push(c);
                chars.next();
                *column += 1;
            } else {
                break;
            }
        }
        ident
    }

    fn read_method_chain(
        chars: &mut std::iter::Peekable<impl Iterator<Item = char>>,
        s: &mut String,
        column: &mut usize,
    ) {
        while let Some(&c) = chars.peek() {
            if c == '.' {
                s.push(c);
                chars.next();
                *column += 1;
                while let Some(&m) = chars.peek() {
                    s.push(m);
                    chars.next();
                    *column += 1;
                    if m == ')' {
                        break;
                    }
                }
            } else {
                break;
            }
        }
    }

    while let Some(c) = chars.next() {
        match c {
            '/' => {
                if let Some(&'/') = chars.peek() {
                    // Line comment
                    chars.next();
                    while let Some(ch) = chars.peek() {
                        if *ch == '\n' {
                            break;
                        }
                        chars.next();
                        column += 1;
                    }
                } else {
                    tokens.push(Token::Slash);
                    column += 1;
                }
            }
            '{' => tokens.push(Token::LBrace),
            '}' => tokens.push(Token::RBrace),
            '(' => tokens.push(Token::LParen),
            ')' => tokens.push(Token::RParen),
            ':' => tokens.push(Token::Colon),
            '.' => tokens.push(Token::Dot),
            ',' => tokens.push(Token::Comma),
            '+' => tokens.push(Token::Plus),
            '-' => tokens.push(Token::Minus),
            '*' => tokens.push(Token::Star),
            '%' => tokens.push(Token::Percent),
            '=' => {
                if let Some(&'=') = chars.peek() {
                    chars.next();
                    tokens.push(Token::EqualsEquals);
                    column += 2;
                } else {
                    tokens.push(Token::Equals);
                    column += 1;
                }
            }
            '&' => {
                if let Some(&'&') = chars.peek() {
                    chars.next();
                    tokens.push(Token::And);
                    column += 2;
                } else {
                    return Err(LexError::UnexpectedCharacter { line, column, character: c });
                }
            }
            '|' => {
                if let Some(&'|') = chars.peek() {
                    chars.next();
                    tokens.push(Token::Or);
                    column += 2;
                } else {
                    return Err(LexError::UnexpectedCharacter { line, column, character: c });
                }
            }
            '!' => {
                if let Some(&'=') = chars.peek() {
                    chars.next();
                    tokens.push(Token::NotEquals);
                    column += 2;
                } else {
                    tokens.push(Token::Not);
                    column += 1;
                }
            }
            '>' => {
                if let Some(&'=') = chars.peek() {
                    chars.next();
                    tokens.push(Token::GreaterEquals);
                    column += 2;
                } else {
                    tokens.push(Token::GreaterThan);
                    column += 1;
                }
            }
            '<' => {
                if let Some(&'=') = chars.peek() {
                    chars.next();
                    tokens.push(Token::LessEquals);
                    column += 2;
                } else {
                    tokens.push(Token::LessThan);
                    column += 1;
                }
            }

            // String literal
            '"' => {
                let string_start_line = line;
                let string_start_column = column;
                column += 1;
                let mut s = String::new();
                let mut terminated = false;

                while let Some(ch) = chars.next() {
                    column += 1;
                    match ch {
                        '"' => {
                            terminated = true;
                            break;
                        }
                        '\n' => {
                            return Err(LexError::UnterminatedString {
                                line: string_start_line,
                                column: string_start_column,
                            });
                        }
                        '\\' => {
                            if let Some(next_ch) = chars.next() {
                                column += 1;
                                s.push(match next_ch {
                                    'n' => '\n',
                                    't' => '\t',
                                    'r' => '\r',
                                    '\\' => '\\',
                                    '"' => '"',
                                    other => {
                                        return Err(LexError::InvalidEscape {
                                            line,
                                            column: column - 1,
                                            character: other,
                                        });
                                    }
                                });
                            } else {
                                return Err(LexError::UnterminatedString {
                                    line: string_start_line,
                                    column: string_start_column,
                                });
                            }
                        }

                        // Interpolation-like support for `${}` or `$ident.method()`
                        '$' => {
                            if chars.peek() == Some(&'{') {
                                // ${ ... }
                                s.push_str("{{$");
                                chars.next(); // consume '{'
                                column += 1;
                                s.push('{');
                                let mut depth = 1;
                                while let Some(next_ch) = chars.next() {
                                    column += 1;
                                    if next_ch == '{' {
                                        depth += 1;
                                    } else if next_ch == '}' {
                                        depth -= 1;
                                        if depth == 0 {
                                            s.push('}');
                                            s.push_str("}");
                                            break;
                                        }
                                    }
                                    s.push(next_ch);
                                }
                            } else {
                                // $var or $var.method()
                                let mut var_expr = String::from("$");
                                var_expr.push_str(&read_ident(&mut chars, &mut column));
                                read_method_chain(&mut chars, &mut var_expr, &mut column);
                                s.push_str("{{");
                                s.push_str(&var_expr);
                                s.push_str("}}");
                            }
                        }

                        _ => s.push(ch),
                    }
                }

                if !terminated {
                    return Err(LexError::UnterminatedString {
                        line: string_start_line,
                        column: string_start_column,
                    });
                }
                tokens.push(Token::String(s));
            }

            // Variables outside strings
            '$' => {
                column += 1;
                let var = read_ident(&mut chars, &mut column);
                tokens.push(Token::Variable(var));
            }

            // Identifiers and keywords
            c if c.is_alphabetic() => {
                let mut ident = String::from(c);
                column += 1;
                ident.push_str(&read_ident(&mut chars, &mut column));
                tokens.push(match ident.as_str() {
                    "server" => Token::Server,
                    "tcp" => Token::TCP,
                    "on" => Token::On,
                    "log" => Token::Log,
                    "send" => Token::Send,
                    "set" => Token::Set,
                    "if" => Token::If,
                    "else" => Token::Else,
                    _ => Token::Ident(ident),
                });
            }

            // Numbers
            c if c.is_ascii_digit() => {
                let mut num = c.to_string();
                column += 1;
                while let Some(&n) = chars.peek() {
                    if n.is_ascii_digit() {
                        num.push(n);
                        chars.next();
                        column += 1;
                    } else {
                        break;
                    }
                }
                tokens.push(Token::Number(num));
            }

            '\n' => {
                line += 1;
                column = 1;
            }

            c if c.is_whitespace() => column += 1,

            c if c == ',' || c == ';' => column += 1,

            c => {
                return Err(LexError::UnexpectedCharacter { line, column, character: c });
            }
        }
    }

    tokens.push(Token::EOF);
    Ok(tokens)
}
