use crate::ast::{Expression, ArithmeticOperator};
use crate::runtime::eval_expression;
use std::collections::HashMap;

fn parse_template_expr(s: &str) -> Expression {
    let mut chars = s.chars().peekable();

    // Must start with $
    if chars.next() != Some('$') {
        return Expression::String(s.to_string());
    }

    // Check if it's ${...} for complex expressions
    if matches!(chars.peek(), Some('{')) {
        chars.next(); // skip '{'

        // Collect the expression until '}'
        let mut expr_str = String::new();
        let mut depth = 1;
        while let Some(ch) = chars.next() {
            if ch == '{' {
                depth += 1;
                expr_str.push(ch);
            } else if ch == '}' {
                depth -= 1;
                if depth == 0 {
                    break;
                }
                expr_str.push(ch);
            } else {
                expr_str.push(ch);
            }
        }

        // Parse the expression inside ${}
        return parse_complex_expr(&expr_str);
    }

    // Read variable name
    let mut var_name = String::new();
    while let Some(&c) = chars.peek() {
        if c.is_alphanumeric() || c == '_' {
            var_name.push(c);
            chars.next();
        } else {
            break;
        }
    }

    let mut expr = Expression::Variable(var_name);

    // Read optional .method(...) chains
    while let Some(&c) = chars.peek() {
        if c == '.' {
            chars.next(); // skip '.'

            // Read method name
            let mut method_name = String::new();
            while let Some(&m) = chars.peek() {
                if m.is_alphabetic() || m == '_' {
                    method_name.push(m);
                    chars.next();
                } else {
                    break;
                }
            }

            // Optional parentheses and argument
            let mut arg = None;
            if matches!(chars.peek(), Some('(')) {
                chars.next(); // skip '('

                // Collect number argument
                let mut num_str = String::new();
                while let Some(&n) = chars.peek() {
                    if n.is_ascii_digit() {
                        num_str.push(n);
                        chars.next();
                    } else {
                        break;
                    }
                }

                if !num_str.is_empty() {
                    arg = Some(Box::new(Expression::Number(num_str.parse().unwrap())));
                }

                // Skip remaining ')' if present
                if matches!(chars.peek(), Some(')')) {
                    chars.next();
                }
            }

            expr = Expression::MethodCall {
                object: Box::new(expr),
                method: method_name,
                arg,
            };
        } else {
            break;
        }
    }

    expr
}

// Parse complex expressions like "x + 1" or "count * 2"
fn parse_complex_expr(s: &str) -> Expression {
    let s = s.trim();

    // Simple recursive descent parser for arithmetic
    parse_additive_expr(s)
}

fn parse_additive_expr(s: &str) -> Expression {
    // Look for + or - operators (lowest precedence)
    let mut depth = 0;
    let chars: Vec<char> = s.chars().collect();

    for i in (0..chars.len()).rev() {
        let ch = chars[i];
        match ch {
            ')' => depth += 1,
            '(' => depth -= 1,
            '+' | '-' if depth == 0 => {
                let left = parse_additive_expr(&s[..i].trim());
                let right = parse_multiplicative_expr(&s[i+1..].trim());
                let op = if ch == '+' {
                    ArithmeticOperator::Add
                } else {
                    ArithmeticOperator::Subtract
                };
                return Expression::Arithmetic {
                    left: Box::new(left),
                    op,
                    right: Box::new(right),
                };
            }
            _ => {}
        }
    }

    parse_multiplicative_expr(s)
}

fn parse_multiplicative_expr(s: &str) -> Expression {
    // Look for *, /, % operators (higher precedence)
    let mut depth = 0;
    let chars: Vec<char> = s.chars().collect();

    for i in (0..chars.len()).rev() {
        let ch = chars[i];
        match ch {
            ')' => depth += 1,
            '(' => depth -= 1,
            '*' | '/' | '%' if depth == 0 => {
                let left = parse_multiplicative_expr(&s[..i].trim());
                let right = parse_primary_expr(&s[i+1..].trim());
                let op = match ch {
                    '*' => ArithmeticOperator::Multiply,
                    '/' => ArithmeticOperator::Divide,
                    '%' => ArithmeticOperator::Modulo,
                    _ => unreachable!(),
                };
                return Expression::Arithmetic {
                    left: Box::new(left),
                    op,
                    right: Box::new(right),
                };
            }
            _ => {}
        }
    }

    parse_primary_expr(s)
}

fn parse_primary_expr(s: &str) -> Expression {
    let s = s.trim();

    // Handle parentheses
    if s.starts_with('(') && s.ends_with(')') {
        return parse_complex_expr(&s[1..s.len()-1]);
    }

    // Handle numbers
    if let Ok(n) = s.parse::<i64>() {
        return Expression::Number(n);
    }

    // Handle variables and method calls
    if s.contains('.') {
        // Variable with method call
        let parts: Vec<&str> = s.splitn(2, '.').collect();
        let var_name = parts[0].trim();
        let method_part = parts[1].trim();

        let mut expr = Expression::Variable(var_name.to_string());

        // Parse method name and args
        if let Some(paren_pos) = method_part.find('(') {
            let method_name = method_part[..paren_pos].trim();
            expr = Expression::MethodCall {
                object: Box::new(expr),
                method: method_name.to_string(),
                arg: None,
            };
        } else {
            expr = Expression::MethodCall {
                object: Box::new(expr),
                method: method_part.to_string(),
                arg: None,
            };
        }

        return expr;
    }

    // Plain variable
    Expression::Variable(s.to_string())
}

pub fn eval_template(
    s: &str,
    message: Option<&str>,
    client: Option<&str>,
    vars: &HashMap<String, String>,
) -> String {
    let mut result = String::new();
    let mut remaining = s;

    while let Some(start) = remaining.find("{{") {
        let (before, after) = remaining.split_at(start);
        result.push_str(before);

        if let Some(end) = after.find("}}") {
            let expr_str = &after[2..end];
            let expr = parse_template_expr(expr_str);
            let evaluated = eval_expression(&expr, message, client, vars);
            result.push_str(&evaluated);
            remaining = &after[end + 2..];
        } else {
            result.push_str(after);
            break;
        }
    }

    result.push_str(remaining);
    result
}