#[derive(Debug)]
#[derive(Clone)]
pub enum Statement {
    Server {
        protocol: String,
        port: String,
        body: Vec<Statement>,
    },
    On {
        event: String,
        body: Vec<Statement>,
    },
    Log(Expression),
    Send(Expression),
}

#[derive(Debug, Clone)]
pub enum Expression {
    String(String),
    Variable(String),
    MethodCall {
        object: Box<Expression>,
        method: String,
    },
}