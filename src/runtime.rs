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
            // Send welcome message
            if let Err(e) = socket.write_all(b"Welcome to Vivo!\n").await {
                eprintln!("Failed to send welcome to {}: {}", addr, e);
                return;
            }
            if let Err(e) = socket.flush().await {
                eprintln!("Failed to flush welcome to {}: {}", addr, e);
                return;
            }

            // Handle connect event logs
            for stmt in events_clone.iter() {
                if let Statement::On { event, body } = stmt {
                    if event == "connect" {
                        for inner in body {
                            if let Statement::Log(msg) = inner {
                                println!("[connect] {}", msg);
                            }
                        }
                    }
                }
            }

            let mut buf = vec![0u8; 1024];

            loop {
                match socket.read(&mut buf).await {
                    Ok(0) => {
                        println!("Client {} disconnected", addr);
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
                                    for inner in body {
                                        if let Statement::Log(log_msg) = inner {
                                            println!("[message] {}", log_msg);
                                        }
                                    }
                                }
                            }
                        }

                        // Echo back
                        let response = format!("Echo: {}\n", msg_trimmed);
                        if let Err(e) = socket.write_all(response.as_bytes()).await {
                            eprintln!("Write error to {}: {}", addr, e);
                            break;
                        }
                        if let Err(e) = socket.flush().await {
                            eprintln!("Flush error to {}: {}", addr, e);
                            break;
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