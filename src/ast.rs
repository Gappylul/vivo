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
    Log(String),
}