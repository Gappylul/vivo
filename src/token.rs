#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Ident(String),
    String(String),
    Number(f64),
    LBrace,
    RBrace,
    LParen,
    RParen,
    On,
    Server,
    TCP,
    Colon,
    EOF
}