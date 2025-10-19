use std::{env, fs};

mod token;
mod lexer;
mod ast;
mod parser;
mod interpreter;
mod runtime;
mod template;

#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: vivo <file.vi>");
        return;
    }

    let src = fs::read_to_string(&args[1])
        .expect("Failed to read source file");

    let tokens = lexer::lex(&src);
    let ast = parser::parse(tokens);
    interpreter::interpret(ast).await;
}