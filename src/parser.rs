use crate::token::Token;
use crate::ast::{Statement, Expression};
use std::fmt;

#[derive(Debug)]
pub enum ParseError {
    UnexpectedToken { expected: String, found: String, position: usize },
    UnexpectedEof { expected: String },
    InvalidExpression { position: usize },
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseError::UnexpectedToken { expected, found, position } => {
                write!(f, "Parse error at position {}: expected {}, found {}", position, expected, found)
            }
            ParseError::UnexpectedEof { expected } => {
                write!(f, "Unexpected end of file: expected {}", expected)
            }
            ParseError::InvalidExpression { position } => {
                write!(f, "Invalid expression at position {}", position)
            }
        }
    }
}

impl std::error::Error for ParseError {}

type ParseResult<T> = Result<T, ParseError>;

fn parse_expression(tokens: &[Token], i: &mut usize) -> ParseResult<Expression> {
    if *i >= tokens.len() {
        return Err(ParseError::UnexpectedEof {
            expected: "expression".to_string(),
        });
    }

    // Base expression
    let mut expr = match &tokens[*i] {
        Token::String(s) => {
            *i += 1;
            Expression::String(s.clone())
        }
        Token::Variable(v) | Token::Ident(v) => {
            *i += 1;
            Expression::Variable(v.clone())
        }
        Token::Number(n) => {
            let value = n.parse().map_err(|_| ParseError::InvalidExpression { position: *i })?;
            *i += 1;
            Expression::Number(value)
        }
        _ => {
            return Err(ParseError::UnexpectedToken {
                expected: "expression".to_string(),
                found: format!("{:?}", tokens.get(*i)),
                position: *i,
            });
        }
    };

    // Handle chained method calls: .method(...)
    while *i < tokens.len() && matches!(tokens[*i], Token::Dot) {
        *i += 1; // skip '.'

        // Method name
        let method = if let Some(Token::Ident(name)) = tokens.get(*i) {
            name.clone()
        } else {
            return Err(ParseError::UnexpectedToken {
                expected: "method name".to_string(),
                found: format!("{:?}", tokens.get(*i)),
                position: *i,
            });
        };
        *i += 1;

        // Optional parentheses for arguments
        let mut args = Vec::new();
        if *i < tokens.len() && matches!(tokens[*i], Token::LParen) {
            *i += 1; // skip '('

            while *i < tokens.len() && !matches!(tokens[*i], Token::RParen) {
                let arg_expr = parse_expression(tokens, i)?;
                args.push(arg_expr);

                // Comma between arguments
                if *i < tokens.len() && matches!(tokens[*i], Token::Comma) {
                    *i += 1;
                } else {
                    break;
                }
            }

            // Expect closing ')'
            if *i < tokens.len() && matches!(tokens[*i], Token::RParen) {
                *i += 1;
            } else {
                return Err(ParseError::UnexpectedToken {
                    expected: "')'".to_string(),
                    found: format!("{:?}", tokens.get(*i)),
                    position: *i,
                });
            }
        }

        // Build MethodCall expression
        let arg = match args.len() {
            0 => None,
            1 => Some(Box::new(args.remove(0))),
            _ => Some(Box::new(Expression::Tuple(args))),
        };

        expr = Expression::MethodCall {
            object: Box::new(expr),
            method,
            arg,
        };
    }

    Ok(expr)
}

pub fn parse(tokens: Vec<Token>) -> Result<Vec<Statement>, ParseError> {
    let mut stmts = Vec::new();
    let mut i = 0;

    while i < tokens.len() {
        match &tokens[i] {
            Token::Server => {
                i += 1; // Move past 'server'

                // Ensure we have enough tokens
                if i >= tokens.len() {
                    return Err(ParseError::UnexpectedEof {
                        expected: "protocol (tcp)".to_string(),
                    });
                }

                // Expect TCP token
                if !matches!(tokens[i], Token::TCP) {
                    return Err(ParseError::UnexpectedToken {
                        expected: "tcp".to_string(),
                        found: format!("{:?}", tokens[i]),
                        position: i,
                    });
                }
                i += 1; // Move past 'tcp'

                // Expect port string
                if i >= tokens.len() {
                    return Err(ParseError::UnexpectedEof {
                        expected: "port string".to_string(),
                    });
                }

                let port = if let Token::String(p) = &tokens[i] {
                    p.clone()
                } else {
                    return Err(ParseError::UnexpectedToken {
                        expected: "port string".to_string(),
                        found: format!("{:?}", tokens[i]),
                        position: i,
                    });
                };
                i += 1; // Move past port

                // Expect opening brace
                if i >= tokens.len() {
                    return Err(ParseError::UnexpectedEof {
                        expected: "'{'".to_string(),
                    });
                }

                if !matches!(tokens[i], Token::LBrace) {
                    return Err(ParseError::UnexpectedToken {
                        expected: "'{'".to_string(),
                        found: format!("{:?}", tokens[i]),
                        position: i,
                    });
                }
                i += 1; // Move past '{'

                let mut body = Vec::new();

                // Parse server body
                while i < tokens.len() && !matches!(tokens[i], Token::RBrace | Token::EOF) {
                    if let Token::On = tokens[i] {
                        i += 1; // Move past 'on'

                        if i >= tokens.len() {
                            return Err(ParseError::UnexpectedEof {
                                expected: "event name".to_string(),
                            });
                        }

                        let event = if let Token::Ident(e) = &tokens[i] {
                            e.clone()
                        } else {
                            return Err(ParseError::UnexpectedToken {
                                expected: "event name (connect, message, disconnect)".to_string(),
                                found: format!("{:?}", tokens[i]),
                                position: i,
                            });
                        };
                        i += 1; // Move past event name

                        // Expect opening brace for event block
                        if i >= tokens.len() {
                            return Err(ParseError::UnexpectedEof {
                                expected: "'{'".to_string(),
                            });
                        }

                        if !matches!(tokens[i], Token::LBrace) {
                            return Err(ParseError::UnexpectedToken {
                                expected: "'{'".to_string(),
                                found: format!("{:?}", tokens[i]),
                                position: i,
                            });
                        }
                        i += 1; // Move past '{'

                        let mut inner = Vec::new();

                        // Parse event body
                        loop {
                            if i >= tokens.len() {
                                return Err(ParseError::UnexpectedEof {
                                    expected: "statement or '}'".to_string(),
                                });
                            }

                            match &tokens[i] {
                                Token::Log => {
                                    i += 1;
                                    if i >= tokens.len() {
                                        return Err(ParseError::UnexpectedEof {
                                            expected: "'('".to_string(),
                                        });
                                    }

                                    if !matches!(tokens[i], Token::LParen) {
                                        return Err(ParseError::UnexpectedToken {
                                            expected: "'('".to_string(),
                                            found: format!("{:?}", tokens[i]),
                                            position: i,
                                        });
                                    }
                                    i += 1;

                                    let expr = parse_expression(&tokens, &mut i)?;
                                    inner.push(Statement::Log(expr));

                                    if i < tokens.len() && matches!(tokens[i], Token::RParen) {
                                        i += 1;
                                    } else {
                                        return Err(ParseError::UnexpectedToken {
                                            expected: "')'".to_string(),
                                            found: format!("{:?}", tokens.get(i)),
                                            position: i,
                                        });
                                    }
                                }
                                Token::Send => {
                                    i += 1;
                                    if i >= tokens.len() {
                                        return Err(ParseError::UnexpectedEof {
                                            expected: "'('".to_string(),
                                        });
                                    }

                                    if !matches!(tokens[i], Token::LParen) {
                                        return Err(ParseError::UnexpectedToken {
                                            expected: "'('".to_string(),
                                            found: format!("{:?}", tokens[i]),
                                            position: i,
                                        });
                                    }
                                    i += 1;

                                    let expr = parse_expression(&tokens, &mut i)?;
                                    inner.push(Statement::Send(expr));

                                    if i < tokens.len() && matches!(tokens[i], Token::RParen) {
                                        i += 1;
                                    } else {
                                        return Err(ParseError::UnexpectedToken {
                                            expected: "')'".to_string(),
                                            found: format!("{:?}", tokens.get(i)),
                                            position: i,
                                        });
                                    }
                                }
                                Token::RBrace => {
                                    i += 1;
                                    break;
                                }
                                Token::EOF => {
                                    return Err(ParseError::UnexpectedEof {
                                        expected: "'}'".to_string(),
                                    });
                                }
                                _ => {
                                    return Err(ParseError::UnexpectedToken {
                                        expected: "log, send, or '}'".to_string(),
                                        found: format!("{:?}", tokens[i]),
                                        position: i,
                                    });
                                }
                            }
                        }

                        body.push(Statement::On {
                            event,
                            body: inner,
                        });
                    } else {
                        return Err(ParseError::UnexpectedToken {
                            expected: "'on' or '}'".to_string(),
                            found: format!("{:?}", tokens[i]),
                            position: i,
                        });
                    }
                }

                // Expect closing brace for server
                if i >= tokens.len() {
                    return Err(ParseError::UnexpectedEof {
                        expected: "'}'".to_string(),
                    });
                }

                if matches!(tokens[i], Token::RBrace) {
                    i += 1;
                }

                stmts.push(Statement::Server {
                    protocol: "tcp".into(),
                    port,
                    body,
                });
            }
            Token::EOF => break,
            _ => {
                return Err(ParseError::UnexpectedToken {
                    expected: "'server' declaration".to_string(),
                    found: format!("{:?}", tokens[i]),
                    position: i,
                });
            }
        }
    }

    Ok(stmts)
}