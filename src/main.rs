use std::{env, fs};
use std::path::Path;

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

    let path = Path::new(&args[1]);

    if path.extension().and_then(|ext| ext.to_str()) != Some("vi") {
        eprintln!("Error: only .vi files are supported");
        return;
    }

    let src = match fs::read_to_string(path) {
        Ok(src) => src,
        Err(e) => {
            eprintln!("Error: failed to read '{}': {}", path.display(), e);
            return;
        }
    };

    let tokens = match lexer::lex(&src) {
        Ok(tokens) => tokens,
        Err(e) => {
            eprintln!("Lexer error: {}", e);
            return;
        }
    };

    let ast = match parser::parse(tokens) {
        Ok(ast) => ast,
        Err(e) => {
            eprintln!("Parse error: {}", e);
            return;
        }
    };

    interpreter::interpret(ast).await;
}