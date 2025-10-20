use crate::token::Token;
use crate::ast::{Statement, Expression, BinaryOperator, LogicalOperator, UnaryOperator};
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
    parse_logical_or(tokens, i)
}

fn parse_logical_or(tokens: &[Token], i: &mut usize) -> ParseResult<Expression> {
    let mut left = parse_logical_and(tokens, i)?;

    while *i < tokens.len() && matches!(tokens[*i], Token::Or) {
        *i += 1; // skip '||'
        let right = parse_logical_and(tokens, i)?;
        left = Expression::LogicalOp {
            left: Box::new(left),
            op: LogicalOperator::Or,
            right: Box::new(right),
        };
    }

    Ok(left)
}

fn parse_logical_and(tokens: &[Token], i: &mut usize) -> ParseResult<Expression> {
    let mut left = parse_comparison(tokens, i)?;

    while *i < tokens.len() && matches!(tokens[*i], Token::And) {
        *i += 1; // skip '&&'
        let right = parse_comparison(tokens, i)?;
        left = Expression::LogicalOp {
            left: Box::new(left),
            op: LogicalOperator::And,
            right: Box::new(right),
        };
    }

    Ok(left)
}

fn parse_comparison(tokens: &[Token], i: &mut usize) -> ParseResult<Expression> {
    if *i >= tokens.len() {
        return Err(ParseError::UnexpectedEof {
            expected: "expression".to_string(),
        });
    }

    // Parse base expression
    let mut expr = parse_unary(tokens, i)?;

    // Check for comparison operators
    if *i < tokens.len() {
        let op = match &tokens[*i] {
            Token::EqualsEquals => Some(BinaryOperator::Equal),
            Token::NotEquals => Some(BinaryOperator::NotEqual),
            Token::GreaterThan => Some(BinaryOperator::GreaterThan),
            Token::LessThan => Some(BinaryOperator::LessThan),
            Token::GreaterEquals => Some(BinaryOperator::GreaterEqual),
            Token::LessEquals => Some(BinaryOperator::LessEqual),
            _ => None,
        };

        if let Some(operator) = op {
            *i += 1; // skip operator
            let right = parse_unary(tokens, i)?;
            expr = Expression::BinaryOp {
                left: Box::new(expr),
                op: operator,
                right: Box::new(right),
            };
        }
    }

    Ok(expr)
}

fn parse_unary(tokens: &[Token], i: &mut usize) -> ParseResult<Expression> {
    if *i >= tokens.len() {
        return Err(ParseError::UnexpectedEof {
            expected: "expression".to_string(),
        });
    }

    // Check for unary NOT operator
    if matches!(tokens[*i], Token::Not) {
        *i += 1; // skip '!'
        let operand = parse_unary(tokens, i)?;
        return Ok(Expression::UnaryOp {
            op: UnaryOperator::Not,
            operand: Box::new(operand),
        });
    }

    parse_primary_expression(tokens, i)
}

fn parse_primary_expression(tokens: &[Token], i: &mut usize) -> ParseResult<Expression> {
    if *i >= tokens.len() {
        return Err(ParseError::UnexpectedEof {
            expected: "expression".to_string(),
        });
    }

    // Handle parentheses
    if matches!(tokens[*i], Token::LParen) {
        *i += 1; // skip '('
        let expr = parse_expression(tokens, i)?;

        if *i >= tokens.len() || !matches!(tokens[*i], Token::RParen) {
            return Err(ParseError::UnexpectedToken {
                expected: "')'".to_string(),
                found: format!("{:?}", tokens.get(*i)),
                position: *i,
            });
        }
        *i += 1; // skip ')'
        return Ok(expr);
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

// Helper function to parse a single statement
fn parse_single_statement(tokens: &[Token], i: &mut usize) -> ParseResult<Statement> {
    if *i >= tokens.len() {
        return Err(ParseError::UnexpectedEof {
            expected: "statement".to_string(),
        });
    }

    match &tokens[*i] {
        Token::Ident(name) if *i + 1 < tokens.len() && matches!(tokens[*i + 1], Token::Equals) => {
            let var_name = name.clone();
            *i += 1;
            *i += 1;
            let value = parse_expression(&tokens, i)?;
            Ok(Statement::SetVar { name: var_name, value })
        }
        Token::If => {
            *i += 1;
            let condition = parse_expression(&tokens, i)?;

            if *i >= tokens.len() || !matches!(tokens[*i], Token::LBrace) {
                return Err(ParseError::UnexpectedToken {
                    expected: "'{'".to_string(),
                    found: format!("{:?}", tokens.get(*i)),
                    position: *i,
                });
            }
            *i += 1;

            let mut then_body = Vec::new();
            while *i < tokens.len() && !matches!(tokens[*i], Token::RBrace) {
                then_body.push(parse_single_statement(tokens, i)?);
            }

            if *i >= tokens.len() || !matches!(tokens[*i], Token::RBrace) {
                return Err(ParseError::UnexpectedEof {
                    expected: "'}'".to_string(),
                });
            }
            *i += 1;

            // Parse else if chains
            let mut else_ifs = Vec::new();
            while *i < tokens.len() && matches!(tokens[*i], Token::Else) {
                // Peek ahead to see if it's "else if" or just "else"
                if *i + 1 < tokens.len() && matches!(tokens[*i + 1], Token::If) {
                    *i += 1; // skip 'else'
                    *i += 1; // skip 'if'

                    let else_if_condition = parse_expression(&tokens, i)?;

                    if *i >= tokens.len() || !matches!(tokens[*i], Token::LBrace) {
                        return Err(ParseError::UnexpectedToken {
                            expected: "'{'".to_string(),
                            found: format!("{:?}", tokens.get(*i)),
                            position: *i,
                        });
                    }
                    *i += 1;

                    let mut else_if_body = Vec::new();
                    while *i < tokens.len() && !matches!(tokens[*i], Token::RBrace) {
                        else_if_body.push(parse_single_statement(tokens, i)?);
                    }

                    if *i >= tokens.len() || !matches!(tokens[*i], Token::RBrace) {
                        return Err(ParseError::UnexpectedEof {
                            expected: "'}'".to_string(),
                        });
                    }
                    *i += 1;

                    else_ifs.push((else_if_condition, else_if_body));
                } else {
                    // It's just "else", not "else if"
                    break;
                }
            }

            // Parse final else block
            let else_body = if *i < tokens.len() && matches!(tokens[*i], Token::Else) {
                *i += 1;

                if *i >= tokens.len() || !matches!(tokens[*i], Token::LBrace) {
                    return Err(ParseError::UnexpectedToken {
                        expected: "'{'".to_string(),
                        found: format!("{:?}", tokens.get(*i)),
                        position: *i,
                    });
                }
                *i += 1;

                let mut else_stmts = Vec::new();
                while *i < tokens.len() && !matches!(tokens[*i], Token::RBrace) {
                    else_stmts.push(parse_single_statement(tokens, i)?);
                }

                if *i >= tokens.len() || !matches!(tokens[*i], Token::RBrace) {
                    return Err(ParseError::UnexpectedEof {
                        expected: "'}'".to_string(),
                    });
                }
                *i += 1;

                Some(else_stmts)
            } else {
                None
            };

            Ok(Statement::If { condition, then_body, else_ifs, else_body })
        }
        Token::Set => {
            *i += 1;
            if *i >= tokens.len() {
                return Err(ParseError::UnexpectedEof {
                    expected: "variable name".to_string(),
                });
            }

            let name = if let Token::Ident(n) = &tokens[*i] {
                n.clone()
            } else {
                return Err(ParseError::UnexpectedToken {
                    expected: "variable name".to_string(),
                    found: format!("{:?}", tokens[*i]),
                    position: *i,
                });
            };
            *i += 1;

            if *i >= tokens.len() || !matches!(tokens[*i], Token::Equals) {
                return Err(ParseError::UnexpectedToken {
                    expected: "'='".to_string(),
                    found: format!("{:?}", tokens.get(*i)),
                    position: *i,
                });
            }
            *i += 1;

            let value = parse_expression(&tokens, i)?;
            Ok(Statement::SetVar { name, value })
        }
        Token::Log => {
            *i += 1;
            if *i >= tokens.len() || !matches!(tokens[*i], Token::LParen) {
                return Err(ParseError::UnexpectedToken {
                    expected: "'('".to_string(),
                    found: format!("{:?}", tokens.get(*i)),
                    position: *i,
                });
            }
            *i += 1;

            let expr = parse_expression(&tokens, i)?;

            if *i >= tokens.len() || !matches!(tokens[*i], Token::RParen) {
                return Err(ParseError::UnexpectedToken {
                    expected: "')'".to_string(),
                    found: format!("{:?}", tokens.get(*i)),
                    position: *i,
                });
            }
            *i += 1;

            Ok(Statement::Log(expr))
        }
        Token::Send => {
            *i += 1;
            if *i >= tokens.len() || !matches!(tokens[*i], Token::LParen) {
                return Err(ParseError::UnexpectedToken {
                    expected: "'('".to_string(),
                    found: format!("{:?}", tokens.get(*i)),
                    position: *i,
                });
            }
            *i += 1;

            let expr = parse_expression(&tokens, i)?;

            if *i >= tokens.len() || !matches!(tokens[*i], Token::RParen) {
                return Err(ParseError::UnexpectedToken {
                    expected: "')'".to_string(),
                    found: format!("{:?}", tokens.get(*i)),
                    position: *i,
                });
            }
            *i += 1;

            Ok(Statement::Send(expr))
        }
        _ => {
            Err(ParseError::UnexpectedToken {
                expected: "statement (set, if, log, send)".to_string(),
                found: format!("{:?}", tokens[*i]),
                position: *i,
            })
        }
    }
}

pub fn parse(tokens: Vec<Token>) -> Result<Vec<Statement>, ParseError> {
    let mut stmts = Vec::new();
    let mut i = 0;

    while i < tokens.len() {
        match &tokens[i] {
            Token::Server => {
                i += 1;

                if i >= tokens.len() {
                    return Err(ParseError::UnexpectedEof {
                        expected: "protocol (tcp)".to_string(),
                    });
                }

                if !matches!(tokens[i], Token::TCP) {
                    return Err(ParseError::UnexpectedToken {
                        expected: "tcp".to_string(),
                        found: format!("{:?}", tokens[i]),
                        position: i,
                    });
                }
                i += 1;

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
                i += 1;

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
                i += 1;

                let mut body = Vec::new();

                while i < tokens.len() && !matches!(tokens[i], Token::RBrace | Token::EOF) {
                    if let Token::On = tokens[i] {
                        i += 1;

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
                        i += 1;

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
                        i += 1;

                        let mut inner = Vec::new();

                        // Parse event body using helper function
                        while i < tokens.len() && !matches!(tokens[i], Token::RBrace | Token::EOF) {
                            inner.push(parse_single_statement(&tokens, &mut i)?);
                        }

                        if i >= tokens.len() {
                            return Err(ParseError::UnexpectedEof {
                                expected: "'}'".to_string(),
                            });
                        }

                        if matches!(tokens[i], Token::RBrace) {
                            i += 1;
                        } else {
                            return Err(ParseError::UnexpectedToken {
                                expected: "'}'".to_string(),
                                found: format!("{:?}", tokens[i]),
                                position: i,
                            });
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