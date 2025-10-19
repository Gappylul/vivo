use crate::token::Token;
use crate::ast::{Statement, Expression};

fn parse_expression(tokens: &[Token], i: &mut usize) -> Expression {
    match &tokens[*i] {
        Token::String(s) => {
            let expr = Expression::String(s.clone());
            *i += 1;
            expr
        }

        Token::Variable(v) | Token::Ident(v) => {
            let mut expr = Expression::Variable(v.clone());
            *i += 1;

            // Support chained method calls
            while matches!(tokens.get(*i), Some(Token::Dot)) {
                *i += 1; // skip '.'

                // method name
                let method = if let Some(Token::Ident(name)) = tokens.get(*i) {
                    name.clone()
                } else {
                    break;
                };
                *i += 1;

                // parse optional ( ... ) arguments
                let mut arg = None;
                if matches!(tokens.get(*i), Some(Token::LParen)) {
                    *i += 1; // skip '('

                    // number argument
                    if let Some(Token::Number(n)) = tokens.get(*i) {
                        arg = Some(Box::new(Expression::Number(n.parse().unwrap())));
                        *i += 1;
                    }

                    // string argument (optional)
                    else if let Some(Token::String(s)) = tokens.get(*i) {
                        arg = Some(Box::new(Expression::String(s.clone())));
                        *i += 1;
                    }

                    // skip ')'
                    if matches!(tokens.get(*i), Some(Token::RParen)) {
                        *i += 1;
                    }
                }

                expr = Expression::MethodCall {
                    object: Box::new(expr),
                    method,
                    arg,
                };
            }

            expr
        }

        Token::Number(n) => {
            *i += 1;
            Expression::Number(n.parse().unwrap())
        }

        _ => {
            *i += 1;
            Expression::String("".to_string())
        }
    }
}

pub fn parse(tokens: Vec<Token>) -> Vec<Statement> {
    let mut stmts = Vec::new();
    let mut i = 0;

    while i < tokens.len() {
        match &tokens[i] {
            Token::Server => {
                if let (Token::TCP, Token::String(port)) = (&tokens[i + 1], &tokens[i + 2]) {
                    let mut body = Vec::new();
                    i += 3;

                    if let Token::LBrace = tokens[i] {
                        i += 1;
                        while !matches!(tokens[i], Token::RBrace | Token::EOF) {
                            if let Token::On = tokens[i] {
                                if let Token::Ident(event) = &tokens[i + 1] {
                                    i += 2; // skip `on <event>`
                                    let mut inner = Vec::new();

                                    if let Token::LBrace = tokens[i] {
                                        i += 1;

                                        // Parse all statements in the on block
                                        loop {
                                            match &tokens[i] {
                                                Token::Log => {
                                                    i += 1;
                                                    if let Token::LParen = tokens[i] {
                                                        i += 1;
                                                        let expr = parse_expression(&tokens, &mut i);
                                                        inner.push(Statement::Log(expr));
                                                        if let Token::RParen = tokens[i] {
                                                            i += 1;
                                                        }
                                                    }
                                                }
                                                Token::Send => {
                                                    i += 1;
                                                    if let Token::LParen = tokens[i] {
                                                        i += 1;
                                                        let expr = parse_expression(&tokens, &mut i);
                                                        inner.push(Statement::Send(expr));
                                                        if let Token::RParen = tokens[i] {
                                                            i += 1;
                                                        }
                                                    }
                                                }
                                                Token::RBrace => {
                                                    i += 1;
                                                    break;
                                                }
                                                Token::EOF => break,
                                                _ => {
                                                    i += 1;
                                                }
                                            }
                                        }
                                    }

                                    body.push(Statement::On {
                                        event: event.clone(),
                                        body: inner,
                                    });
                                }
                            } else {
                                i += 1;
                            }
                        }
                    }

                    stmts.push(Statement::Server {
                        protocol: "tcp".into(),
                        port: port.clone(),
                        body,
                    });
                }
            }
            _ => {}
        }
        i += 1;
    }

    stmts
}