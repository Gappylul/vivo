use crate::ast::{Statement, Expression};
use tokio::net::TcpListener;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use std::sync::Arc;
use crate::template::eval_template;

/// Extract `On` events from the server body
fn extract_events(body: &[Statement]) -> Vec<Statement> {
    body.iter().filter_map(|stmt| {
        if let Statement::On { event, body } = stmt {
            Some(Statement::On { event: event.clone(), body: body.clone() })
        } else { None }
    }).collect()
}

/// Central helper to apply methods to a string
fn apply_method(
    base: &str,
    method: &str,
    arg: Option<&Expression>,
    message: Option<&str>,
    client: Option<&str>,
) -> String {
    match method {
        "reverse" => base.chars().rev().collect(),
        "upper" => base.to_uppercase(),
        "lower" => base.to_lowercase(),
        "length" | "len" => base.len().to_string(),
        "capitalize" | "cap" => base.chars()
            .next()
            .map(|c| c.to_uppercase().collect::<String>() + &base[1..])
            .unwrap_or_default(),
        "contains" => match arg {
            Some(Expression::String(arg)) if arg != "" => {
                base.contains(arg).to_string()
            }
            Some(_) => {
                eprintln!("Warning: contains requires 1 argument, got {:?}", arg);
                base.to_string()
            }
            None => {
                eprintln!("Warning: contains called without arguments");
                base.to_string()
            }
        },
        "starts_with" => match arg {
            Some(Expression::String(arg)) if arg != "" => {
                base.starts_with(arg).to_string()
            }
            Some(_) => {
                eprintln!("Warning: starts_with requires 1 argument, got {:?}", arg);
                base.to_string()
            }
            None => {
                eprintln!("Warning: starts_with called without arguments");
                base.to_string()
            }
        },
        "ends_with" => match arg {
            Some(Expression::String(arg)) if arg != "" => {
                base.ends_with(arg).to_string()
            }
            Some(_) => {
                eprintln!("Warning: ends_with requires 1 argument, got {:?}", arg);
                base.to_string()
            }
            None => {
                eprintln!("Warning: ends_with called without arguments");
                base.to_string()
            }
        },
        "find" => match arg {
            Some(Expression::String(arg)) if arg != "" => {
                base.find(arg)
                    .map(|i| i.to_string())
                    .unwrap_or_else(|| "-1".to_string())
            }
            Some(_) => {
                eprintln!("Warning: find requires 1 argument, got {:?}", arg);
                base.to_string()
            }
            None => {
                eprintln!("Warning: find called without arguments");
                base.to_string()
            }
        },
        "trim" => base.trim().to_string(),
        "rtrim" => base.trim_end().to_string(),
        "ltrim" => base.trim_start().to_string(),
        "repeat" => match arg {
            Some(Expression::Number(n)) => base.repeat(*n as usize),
            _ => base.repeat(2),
        },
        "repeat_sep" => match arg {
            Some(Expression::Tuple(args_vec)) if args_vec.len() == 2 => {
                let times_str = eval_expression(&args_vec[0], message, client);
                let add = eval_expression(&args_vec[1], message, client);

                let times = times_str.parse::<usize>().unwrap_or(0);

                if times == 0 {
                    base.to_string()
                } else {
                    std::iter::repeat(base)
                        .take(times)
                        .collect::<Vec<&str>>()
                        .join(&add)
                }
            }
            Some(_) => {
                eprintln!("Warning: repeat_sep requires 2 argument, got {:?}", arg);
                base.to_string()
            }
            None => {
                eprintln!("Warning: repeat_sep called without arguments");
                base.to_string()
            }
        }
        "replace" => match arg {
            Some(Expression::Tuple(args_vec)) if args_vec.len() == 2 => {
                let from = eval_expression(&args_vec[0], message, client);
                let to = eval_expression(&args_vec[1], message, client);
                base.replace(&from, &to)
            }
            Some(_) => {
                eprintln!("Warning: replace requires 2 arguments, got {:?}", arg);
                base.to_string()
            }
            None => {
                eprintln!("Warning: replace called without arguments");
                base.to_string()
            }
        },
        "remove" => match arg {
            Some(Expression::String(arg)) if arg != "" => {
                base.replace(arg, "").to_string()
            }
            Some(_) => {
                eprintln!("Warning: remove requires 1 arguments, got {:?}", arg);
                base.to_string()
            }
            None => {
                eprintln!("Warning: remove called without arguments");
                base.to_string()
            }
        },
        "count" => match arg {
            Some(Expression::String(arg)) if arg != "" => {
                base.matches(arg).count().to_string()
            }
            Some(_) => {
                eprintln!("Warning: count requires 1 arguments, got {:?}", arg);
                base.to_string()
            }
            None => {
                eprintln!("Warning: count called without arguments");
                base.to_string()
            }
        },
        "is_empty" => base.is_empty().to_string(),
        unknown => {
            eprintln!("Warning: unknown method '{}'", unknown);
            base.to_string()
        }
    }
}

/// Evaluate an expression to a string
pub fn eval_expression(expr: &Expression, message: Option<&str>, client: Option<&str>) -> String {
    match expr {
        Expression::String(s) => eval_template(s, message, client),
        Expression::Variable(v) => match v.as_str() {
            "message" => message.unwrap_or("").to_string(),
            "client" => client.unwrap_or("").to_string(),
            _ => format!("${}", v),
        },
        Expression::Number(n) => n.to_string(),
        Expression::MethodCall { object, method, arg } => {
            let base = eval_expression(object, message, client);
            apply_method(&base, method, arg.as_deref(), message, client)
        }
        Expression::Tuple(_) => {
            eprintln!("Warning: unexpected tuple expression at top level");
            "".to_string()
        }
    }
}

/// Execute statements for a single event
async fn execute_statements(
    statements: &[Statement],
    socket: &mut tokio::net::TcpStream,
    addr: &std::net::SocketAddr,
    message: Option<&str>,
    client: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    for stmt in statements {
        match stmt {
            Statement::Log(expr) => {
                let output = eval_expression(expr, message, client);
                println!("[{}] LOG: {}", addr, output);
            }
            Statement::Send(expr) => {
                let output = eval_expression(expr, message, client);
                let msg_with_newline = format!("{}\n", output);
                socket.write_all(msg_with_newline.as_bytes()).await?;
                socket.flush().await?;
                println!("[{}] SENT: {}", addr, output);
            }
            _ => {}
        }
    }
    Ok(())
}

/// Trigger all events of a certain type
async fn trigger_event(
    events: &[Statement],
    event_name: &str,
    socket: &mut tokio::net::TcpStream,
    addr: &std::net::SocketAddr,
    message: Option<&str>,
    client: Option<&str>,
) {
    for stmt in events {
        if let Statement::On { event, body } = stmt {
            if event == event_name {
                if let Err(e) = execute_statements(body, socket, addr, message, client).await {
                    eprintln!("[{}] Error executing '{}' handler: {}", addr, event_name, e);
                }
            }
        }
    }
}

/// Run the TCP server
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

            // Trigger "connect" events
            trigger_event(&events_clone, "connect", &mut socket, &addr, None, Some(&client_str)).await;

            let mut buf = vec![0u8; 1024];

            loop {
                match socket.read(&mut buf).await {
                    Ok(0) => {
                        println!("Client {} disconnected", addr);
                        trigger_event(&events_clone, "disconnect", &mut socket, &addr, None, Some(&client_str)).await;
                        break;
                    }
                    Ok(n) => {
                        let msg = match String::from_utf8(buf[..n].to_vec()) {
                            Ok(m) => m,
                            Err(e) => {
                                eprintln!("[{}] Invalid UTF-8: {}", addr, e);
                                continue;
                            }
                        };
                        let msg_trimmed = msg.trim_end_matches(&['\r', '\n'][..]);
                        println!("[{}] RECEIVED: {}", addr, msg_trimmed);
                        trigger_event(&events_clone, "message", &mut socket, &addr, Some(msg_trimmed), Some(&client_str)).await;
                    }
                    Err(e) => {
                        eprintln!("[{}] Read error: {}", addr, e);
                        break;
                    }
                }
            }
        });
    }
}