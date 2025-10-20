use crate::ast::{Statement, Expression, BinaryOperator};
use tokio::net::TcpListener;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::RwLock;
use crate::template::eval_template;

type Variables = Arc<RwLock<HashMap<String, String>>>;

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
    vars: &HashMap<String, String>,
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
        "starts_with" | "startsWith" => match arg {
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
        "ends_with" | "endsWith" => match arg {
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
        "repeat_sep" | "repeatSep" => match arg {
            Some(Expression::Tuple(args_vec)) if args_vec.len() == 2 => {
                let times_str = eval_expression(&args_vec[0], message, client, vars);
                let add = eval_expression(&args_vec[1], message, client, vars);

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
                let from = eval_expression(&args_vec[0], message, client, vars);
                let to = eval_expression(&args_vec[1], message, client, vars);
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
        "is_empty" | "isEmpty" => base.is_empty().to_string(),
        "typeof" | "type_of" => {
            if base.parse::<f64>().is_ok() {
                "number".to_string()
            } else if base.starts_with('(') && base.ends_with(')') {
                "tuple".to_string()
            } else if base == "true" || base == "false" {
                "boolean".to_string()
            } else if base.is_empty() {
                "empty".to_string()
            } else {
                "string".to_string()
            }
        },
        unknown => {
            eprintln!("Warning: unknown method '{}'", unknown);
            base.to_string()
        }
    }
}

/// Evaluate an expression to a string
pub fn eval_expression(
    expr: &Expression,
    message: Option<&str>,
    client: Option<&str>,
    vars: &HashMap<String, String>,
) -> String {
    match expr {
        Expression::String(s) => eval_template(s, message, client, vars),
        Expression::Variable(v) => match v.as_str() {
            "message" => message.unwrap_or("").to_string(),
            "client" => client.unwrap_or("").to_string(),
            _ => vars.get(v).cloned().unwrap_or_else(|| format!("${}", v)),
        },
        Expression::Number(n) => n.to_string(),
        Expression::MethodCall { object, method, arg } => {
            let base = eval_expression(object, message, client, vars);
            apply_method(&base, method, arg.as_deref(), message, client, vars)
        }
        Expression::BinaryOp { left, op, right } => {
            let left_val = eval_expression(left, message, client, vars);
            let right_val = eval_expression(right, message, client, vars);

            // Try to parse as numbers for comparison
            let left_num = left_val.parse::<f64>();
            let right_num = right_val.parse::<f64>();

            let result = match (left_num, right_num) {
                (Ok(l), Ok(r)) => {
                    // Numeric comparison
                    match op {
                        BinaryOperator::Equal => l == r,
                        BinaryOperator::NotEqual => l != r,
                        BinaryOperator::GreaterThan => l > r,
                        BinaryOperator::LessThan => l < r,
                        BinaryOperator::GreaterEqual => l >= r,
                        BinaryOperator::LessEqual => l <= r,
                    }
                }
                _ => {
                    // String comparison
                    match op {
                        BinaryOperator::Equal => left_val == right_val,
                        BinaryOperator::NotEqual => left_val != right_val,
                        BinaryOperator::GreaterThan => left_val > right_val,
                        BinaryOperator::LessThan => left_val < right_val,
                        BinaryOperator::GreaterEqual => left_val >= right_val,
                        BinaryOperator::LessEqual => left_val <= right_val,
                    }
                }
            };

            result.to_string()
        }
        Expression::Tuple(_) => {
            eprintln!("Warning: unexpected tuple expression at top level");
            "".to_string()
        }
    }
}

/// Execute statements for a single event
fn execute_statements<'a>(
    statements: &'a [Statement],
    socket: &'a mut tokio::net::TcpStream,
    addr: &'a std::net::SocketAddr,
    message: Option<&'a str>,
    client: Option<&'a str>,
    vars: Variables,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), Box<dyn std::error::Error>>> + Send + 'a>> {
    Box::pin(async move {
        for stmt in statements {
            match stmt {
                Statement::SetVar { name, value } => {
                    let vars_read = vars.read().await;
                    let evaluated = eval_expression(value, message, client, &vars_read);
                    drop(vars_read);

                    let mut vars_write = vars.write().await;
                    vars_write.insert(name.clone(), evaluated.clone());
                    drop(vars_write);

                    println!("[{}] SET: {} = {}", addr, name, evaluated);
                }
                Statement::If { condition, then_body, else_body } => {
                    let vars_read = vars.read().await;
                    let condition_result = eval_expression(condition, message, client, &vars_read);
                    drop(vars_read);

                    // Evaluate condition as boolean
                    let is_true = condition_result == "true" ||
                        (condition_result != "false" &&
                            condition_result != "0" &&
                            !condition_result.is_empty());

                    if is_true {
                        execute_statements(then_body, socket, addr, message, client, vars.clone()).await?;
                    } else if let Some(else_stmts) = else_body {
                        execute_statements(else_stmts, socket, addr, message, client, vars.clone()).await?;
                    }
                }
                Statement::Log(expr) => {
                    let vars_read = vars.read().await;
                    let output = eval_expression(expr, message, client, &vars_read);
                    drop(vars_read);
                    println!("[{}] LOG: {}", addr, output);
                }
                Statement::Send(expr) => {
                    let vars_read = vars.read().await;
                    let output = eval_expression(expr, message, client, &vars_read);
                    drop(vars_read);
                    let msg_with_newline = format!("{}\n", output);
                    socket.write_all(msg_with_newline.as_bytes()).await?;
                    socket.flush().await?;
                    println!("[{}] SENT: {}", addr, output);
                }
                _ => {}
            }
        }
        Ok(())
    })
}

/// Trigger all events of a certain type
async fn trigger_event(
    events: &[Statement],
    event_name: &str,
    socket: &mut tokio::net::TcpStream,
    addr: &std::net::SocketAddr,
    message: Option<&str>,
    client: Option<&str>,
    vars: Variables,
) {
    for stmt in events {
        if let Statement::On { event, body } = stmt {
            if event == event_name {
                if let Err(e) = execute_statements(body, socket, addr, message, client, vars.clone()).await {
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
            let vars: Variables = Arc::new(RwLock::new(HashMap::new()));

            // Trigger "connect" events
            trigger_event(&events_clone, "connect", &mut socket, &addr, None, Some(&client_str), vars.clone()).await;

            let mut buf = vec![0u8; 1024];

            loop {
                match socket.read(&mut buf).await {
                    Ok(0) => {
                        println!("Client {} disconnected", addr);
                        trigger_event(&events_clone, "disconnect", &mut socket, &addr, None, Some(&client_str), vars.clone()).await;
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
                        trigger_event(&events_clone, "message", &mut socket, &addr, Some(msg_trimmed), Some(&client_str), vars.clone()).await;
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