use crate::token::Token;
use crate::ast::Statement;

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
                                        while !matches!(tokens[i], Token::RBrace | Token::EOF) {
                                            match &tokens[i] {
                                                Token::Log => {
                                                    if let Token::String(msg) = &tokens[i + 2] {
                                                        inner.push(Statement::Log(msg.clone()));
                                                    }
                                                    i += 1;
                                                }
                                                Token::Send => {
                                                    if let Token::String(msg) = &tokens[i + 2] {
                                                        inner.push(Statement::Send(msg.clone()));
                                                    }
                                                    i += 1;
                                                }
                                                _ => {}
                                            }
                                            i += 1;
                                        }
                                    }

                                    body.push(Statement::On {
                                        event: event.clone(),
                                        body: inner,
                                    });
                                }
                            }
                            i += 1;
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