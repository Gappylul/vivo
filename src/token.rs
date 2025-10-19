#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Ident(String),
    String(String),
    Number(String),
    Variable(String),
    LBrace,
    RBrace,
    LParen,
    RParen,
    Dot,
    On,
    Server,
    TCP,
    Log,
    Send,
    Colon,
    EOF
}