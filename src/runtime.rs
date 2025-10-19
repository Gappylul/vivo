use crate::ast::{Statement, Expression};
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

/// Evaluate an expression to a string
fn eval_expression(expr: &Expression, message: Option<&str>, client: Option<&str>) -> String {
    match expr {
        Expression::String(s) => {
            // Replace {{variables}} first
            let mut result = s.clone();
            result = result.replace("{{$message}}", message.unwrap_or(""));
            result = result.replace("{{$client}}", client.unwrap_or(""));

            // ✅ New logic: detect `$variable.method()` inside the string
            // Example: "$client.reverse()" → evaluate it properly
            let mut evaluated = result.clone();

            // Simple regex-like scanning (no regex crate)
            let parts: Vec<&str> = result.split('$').collect();
            if parts.len() > 1 {
                let mut new_string = String::new();
                new_string.push_str(parts[0]);
                for p in &parts[1..] {
                    if let Some((var, rest)) = p.split_once('.') {
                        let var_val = match var.trim() {
                            "client" => client.unwrap_or("").to_string(),
                            "message" => message.unwrap_or("").to_string(),
                            _ => format!("${}", var.trim()),
                        };

                        // handle ".reverse()", ".upper()", ".lower()", etc.
                        if rest.starts_with("reverse()") {
                            new_string.push_str(&var_val.chars().rev().collect::<String>());
                            new_string.push_str(&rest["reverse()".len()..]);
                        } else if rest.starts_with("upper()") {
                            new_string.push_str(&var_val.to_uppercase());
                            new_string.push_str(&rest["upper()".len()..]);
                        } else if rest.starts_with("lower()") {
                            new_string.push_str(&var_val.to_lowercase());
                            new_string.push_str(&rest["lower()".len()..]);
                        } else if rest.starts_with("length()") {
                            new_string.push_str(&var_val.len().to_string());
                            new_string.push_str(&rest["length()".len()..]);
                        } else {
                            new_string.push('$');
                            new_string.push_str(p);
                        }
                    } else {
                        new_string.push('$');
                        new_string.push_str(p);
                    }
                }
                evaluated = new_string;
            }

            evaluated
        }

        Expression::Variable(v) => {
            match v.as_str() {
                "message" => message.unwrap_or("").to_string(),
                "client" => client.unwrap_or("").to_string(),
                _ => format!("${}", v),
            }
        }

        Expression::MethodCall { object, method } => {
            let base = eval_expression(object, message, client);
            match method.as_str() {
                "reverse" => base.chars().rev().collect(),
                "upper" => base.to_uppercase(),
                "lower" => base.to_lowercase(),
                "length" => base.len().to_string(),
                _ => base,
            }
        }
    }
}


/// Execute event handlers and send responses
async fn execute_statements(
    statements: &[Statement],
    socket: &mut tokio::net::TcpStream,
    _addr: &std::net::SocketAddr,
    message: Option<&str>,
    client: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    for stmt in statements {
        match stmt {
            Statement::Log(expr) => {
                let output = eval_expression(expr, message, client);
                println!("{}", output);
            }
            Statement::Send(expr) => {
                let output = eval_expression(expr, message, client);
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
            let client_str = addr.port().to_string();

            // Handle connect event
            for stmt in events_clone.iter() {
                if let Statement::On { event, body } = stmt {
                    if event == "connect" {
                        if let Err(e) = execute_statements(body, &mut socket, &addr, None, Some(&client_str)).await {
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
                                    if let Err(e) = execute_statements(body, &mut socket, &addr, None, Some(&client_str)).await {
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
                                    if let Err(e) = execute_statements(body, &mut socket, &addr, Some(msg_trimmed), Some(&client_str)).await {
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