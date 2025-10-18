use crate::ast::Statement;
use tokio::net::TcpListener;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use std::sync::Arc;

/// Extract `On` events from the server body
fn extract_events(body: &[Statement]) -> Vec<Statement> {
    body.iter().filter_map(|stmt| match stmt {
        Statement::On { event, body } => Some(Statement::On {
            event: event.clone(),
            body: body.clone(),
        }),
        _ => None,
    }).collect()
}

/// Execute event handlers and send responses
async fn execute_statements(
    statements: &[Statement],
    socket: &mut tokio::net::TcpStream,
    _addr: &std::net::SocketAddr,
    message: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    for stmt in statements {
        match stmt {
            Statement::Log(msg) => {
                // Replace $message placeholder with actual message
                let output = if let Some(m) = message {
                    msg.replace("$message", m)
                } else {
                    msg.clone()
                };
                println!("{}", output);
            }
            Statement::Send(msg) => {
                // Replace $message placeholder with actual message
                let output = if let Some(m) = message {
                    msg.replace("$message", m)
                } else {
                    msg.clone()
                };
                let msg_with_newline = format!("{}\n", output);
                socket.write_all(msg_with_newline.as_bytes()).await?;
                socket.flush().await?;
            }
            _ => {}
        }
    }
    Ok(())
}

pub async fn run_server(_protocol: &str, port: &str, body: Vec<Statement>) {
    let events = Arc::new(extract_events(&body));

    let port_str = if port.starts_with(':') {
        format!("127.0.0.1{}", port)
    } else {
        format!("127.0.0.1:{}", port)
    };
    println!("Vivo TCP server listening on {}", port_str);

    let listener = TcpListener::bind(&port_str).await.expect("Failed to bind");

    loop {
        let (mut socket, addr) = listener.accept().await.unwrap();
        println!("Client connected: {}", addr);

        let events_clone = Arc::clone(&events);

        tokio::spawn(async move {
            // Handle connect event
            for stmt in events_clone.iter() {
                if let Statement::On { event, body } = stmt {
                    if event == "connect" {
                        if let Err(e) = execute_statements(body, &mut socket, &addr, None).await {
                            eprintln!("Error executing connect handler: {}", e);
                            return;
                        }
                    }
                }
            }

            let mut buf = vec![0u8; 1024];

            loop {
                match socket.read(&mut buf).await {
                    Ok(0) => {
                        println!("Client {} disconnected", addr);

                        // Trigger "disconnect" events
                        for stmt in events_clone.iter() {
                            if let Statement::On { event, body } = stmt {
                                if event == "disconnect" {
                                    if let Err(e) = execute_statements(body, &mut socket, &addr, None).await {
                                        eprintln!("Error executing disconnect handler: {}", e);
                                    }
                                }
                            }
                        }

                        break;
                    }
                    Ok(n) => {
                        let msg = match String::from_utf8(buf[..n].to_vec()) {
                            Ok(m) => m,
                            Err(e) => {
                                eprintln!("Invalid UTF-8 from {}: {}", addr, e);
                                continue;
                            }
                        };
                        let msg_trimmed = msg.trim_end_matches(&['\r', '\n'][..]);
                        println!("Received from {}: {}", addr, msg_trimmed);

                        // Trigger "message" events
                        for stmt in events_clone.iter() {
                            if let Statement::On { event, body } = stmt {
                                if event == "message" {
                                    if let Err(e) = execute_statements(body, &mut socket, &addr, Some(msg_trimmed)).await {
                                        eprintln!("Error executing message handler: {}", e);
                                        break;
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Read error from {}: {}", addr, e);
                        break;
                    }
                }
            }
        });
    }
}