use crate::ast::Statement;
use crate::runtime;

pub async fn interpret(ast: Vec<Statement>) {
    let mut handles = vec![];

    for stmt in ast {
        match stmt {
            Statement::Server { protocol, port, body } => {
                let handle = tokio::spawn(async move {
                    runtime::run_server(&protocol, &port, body).await;
                });
                handles.push(handle);
            }
            _ => {}
        }
    }

    // Wait for all servers to complete (they won't, but this keeps the program running)
    for handle in handles {
        let _ = handle.await;
    }
}