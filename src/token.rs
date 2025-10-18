#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Ident(String),
    String(String),
    LBrace,
    RBrace,
    LParen,
    RParen,
    On,
    Server,
    TCP,
    Log,
    Send,
    Colon,
    EOF
}