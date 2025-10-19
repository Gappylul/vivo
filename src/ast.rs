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
    SetVar {
        name: String,
        value: Expression,
    },
}

#[derive(Debug, Clone)]
pub enum Expression {
    String(String),
    Variable(String),
    Number(i64),
    MethodCall {
        object: Box<Expression>,
        method: String,
        arg: Option<Box<Expression>>
    },
    Tuple(Vec<Expression>)
}