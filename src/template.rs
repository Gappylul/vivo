use crate::ast::{Expression};
use crate::runtime::eval_expression;

fn parse_template_expr(s: &str) -> Expression {
    let mut chars = s.chars().peekable();

    // Must start with $
    if chars.next() != Some('$') {
        return Expression::String(s.to_string());
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
                if m.is_alphabetic() {
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

pub fn eval_template(s: &str, message: Option<&str>, client: Option<&str>) -> String {
    let mut result = String::new();
    let mut remaining = s;

    while let Some(start) = remaining.find("{{") {
        let (before, after) = remaining.split_at(start);
        result.push_str(before);

        if let Some(end) = after.find("}}") {
            let expr_str = &after[2..end];
            let expr = parse_template_expr(expr_str);
            let evaluated = eval_expression(&expr, message, client);
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

