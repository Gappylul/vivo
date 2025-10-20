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
    If {
        condition: Expression,
        then_body: Vec<Statement>,
        else_ifs: Vec<(Expression, Vec<Statement>)>,
        else_body: Option<Vec<Statement>>,
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
    Tuple(Vec<Expression>),
    BinaryOp {
        left: Box<Expression>,
        op: BinaryOperator,
        right: Box<Expression>,
    },
    LogicalOp {
        left: Box<Expression>,
        op: LogicalOperator,
        right: Box<Expression>,
    },
    UnaryOp {
        op: UnaryOperator,
        operand: Box<Expression>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum BinaryOperator {
    Equal,
    NotEqual,
    GreaterThan,
    LessThan,
    GreaterEqual,
    LessEqual,
}

#[derive(Debug, Clone, PartialEq)]
pub enum LogicalOperator {
    And,
    Or,
}

#[derive(Debug, Clone, PartialEq)]
pub enum UnaryOperator {
    Not,
}