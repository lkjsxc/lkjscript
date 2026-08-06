use super::{SourceFile, SourceNode, SyntaxKind};

pub(crate) fn format_file(file: &SourceFile) -> String {
    // Tokens, including their exact spans, are retained for diagnostics. The
    // formatter deliberately traverses structural nodes rather than source
    // bytes or the token stream.
    let _retained_token_count = file.tokens.len();
    let mut output = String::new();
    for form in &file.syntax {
        format_node(form, &mut output, true);
    }
    format_trivia(&file.trailing_trivia, &mut output);
    if output.is_empty() {
        output.push('\n');
    }
    output
}

pub(crate) fn format_node_identity(node: &SourceNode) -> String {
    let mut output = String::new();
    format_node(node, &mut output, false);
    output
}

pub(crate) fn format_node_source(node: &SourceNode) -> String {
    let mut output = String::new();
    format_node(node, &mut output, true);
    output
}

fn format_node(node: &SourceNode, output: &mut String, emit_trivia: bool) {
    enum Work<'a> {
        Node(&'a SourceNode),
        Close(&'a str, &'a [String]),
    }

    let mut work = vec![Work::Node(node)];
    while let Some(item) = work.pop() {
        let Work::Node(node) = item else {
            let Work::Close(name, trivia) = item else {
                continue;
            };
            if emit_trivia {
                format_trivia(trivia, output);
            }
            output.push('/');
            output.push_str(name);
            output.push('\n');
            continue;
        };
        if emit_trivia {
            format_trivia(&node.leading_trivia, output);
        }
        match &node.kind {
            SyntaxKind::I64 { value } => {
                output.push_str(&value.to_string());
                output.push('\n');
            }
            SyntaxKind::F64 { value } => {
                output.push_str(&format_f64(*value));
                output.push('\n');
            }
            SyntaxKind::Bool { value } => {
                output.push_str(if *value { "true\n" } else { "false\n" });
            }
            SyntaxKind::Unit => output.push_str("unit\n"),
            SyntaxKind::Str { value } => {
                format_text("string-literal", value, output);
                if emit_trivia {
                    format_trivia(&node.before_close_trivia, output);
                }
            }
            SyntaxKind::Bytes { value } => {
                output.push_str("bytes-literal/\n");
                for byte in value {
                    use std::fmt::Write as _;
                    let _ = write!(output, "{byte:02x}");
                }
                output.push_str("\n/bytes-literal\n");
                if emit_trivia {
                    format_trivia(&node.before_close_trivia, output);
                }
            }
            SyntaxKind::Symbol { name } => {
                output.push_str(name);
                output.push('\n');
            }
            SyntaxKind::Call { name } => {
                if matches!(name.as_str(), "name" | "module") {
                    if let [SourceNode {
                        kind: SyntaxKind::Str { value },
                        ..
                    }] = node.children.as_slice()
                    {
                        format_text(name, value, output);
                        if emit_trivia {
                            format_trivia(&node.before_close_trivia, output);
                        }
                        continue;
                    }
                }
                output.push_str(name);
                output.push_str("/\n");
                work.push(Work::Close(name, &node.before_close_trivia));
                work.extend(node.children.iter().rev().map(Work::Node));
            }
        }
    }
}

fn format_trivia(trivia: &[String], output: &mut String) {
    for line in trivia {
        output.push_str(line);
        output.push('\n');
    }
}

fn format_text(marker: &str, value: &str, output: &mut String) {
    output.push_str(marker);
    output.push_str("/\n");
    let close = format!("/{marker}");
    for line in value.split('\n') {
        if line == close {
            output.push('\\');
        }
        output.push_str(line);
        output.push('\n');
    }
    output.push('/');
    output.push_str(marker);
    output.push('\n');
}

pub(crate) fn format_f64(value: f64) -> String {
    if value == 0.0 && value.is_sign_negative() {
        return "-0.0".into();
    }
    let shortest = value.to_string();
    if let Some((mantissa, exponent)) = shortest
        .split_once('e')
        .or_else(|| shortest.split_once('E'))
    {
        return expand_exponent(mantissa, exponent);
    }
    if shortest.contains('.') {
        shortest
    } else {
        format!("{shortest}.0")
    }
}

fn expand_exponent(mantissa: &str, exponent: &str) -> String {
    let negative = mantissa.starts_with('-');
    let unsigned = mantissa.strip_prefix('-').unwrap_or(mantissa);
    let exponent = exponent.parse::<i32>().unwrap_or(0);
    let mut digits = String::new();
    let mut decimal = None;
    for character in unsigned.chars() {
        if character == '.' {
            decimal = Some(digits.len() as i32);
        } else {
            digits.push(character);
        }
    }
    let original_decimal = decimal.unwrap_or(digits.len() as i32);
    let shifted = original_decimal.saturating_add(exponent);
    let mut output = String::new();
    if negative {
        output.push('-');
    }
    if shifted <= 0 {
        output.push_str("0.");
        for _ in 0..shifted.saturating_neg() {
            output.push('0');
        }
        output.push_str(&digits);
    } else {
        let shifted = shifted as usize;
        if shifted >= digits.len() {
            output.push_str(&digits);
            for _ in digits.len()..shifted {
                output.push('0');
            }
            output.push_str(".0");
        } else {
            output.push_str(&digits[..shifted]);
            output.push('.');
            output.push_str(&digits[shifted..]);
        }
    }
    output
}
