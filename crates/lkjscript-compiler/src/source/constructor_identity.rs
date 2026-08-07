use super::{SourceNode, SyntaxKind};

/// Deterministic syntax-independent key material for constructor declarations
/// that do not have a source-level name (currently implementations).
pub(crate) fn constructor_identity(node: &SourceNode) -> String {
    enum Work<'a> {
        Node(&'a SourceNode),
        Close,
    }

    let mut output = String::new();
    let mut work = vec![Work::Node(node)];
    while let Some(item) = work.pop() {
        match item {
            Work::Close => output.push(')'),
            Work::Node(node) => match &node.kind {
                SyntaxKind::I64 { value } => {
                    output.push_str("i64:");
                    output.push_str(&value.to_string());
                    output.push(';');
                }
                SyntaxKind::F64 { value } => {
                    output.push_str("f64:");
                    output.push_str(&format_f64(*value));
                    output.push(';');
                }
                SyntaxKind::Bool { value } => {
                    output.push_str(if *value { "bool:true;" } else { "bool:false;" });
                }
                SyntaxKind::Unit => output.push_str("unit;"),
                SyntaxKind::Str { value } => {
                    output.push_str("str:");
                    push_framed_text(&mut output, value);
                }
                SyntaxKind::Bytes { value } => {
                    output.push_str("bytes:");
                    output.push_str(&value.len().to_string());
                    output.push(':');
                    for byte in value {
                        const HEX: &[u8; 16] = b"0123456789abcdef";
                        output.push(char::from(HEX[usize::from(byte >> 4)]));
                        output.push(char::from(HEX[usize::from(byte & 0x0f)]));
                    }
                    output.push(';');
                }
                SyntaxKind::Symbol { name } => {
                    output.push_str("symbol:");
                    push_framed_text(&mut output, name);
                }
                SyntaxKind::Call { name } => {
                    output.push_str("call:");
                    push_framed_text(&mut output, name);
                    output.push('(');
                    work.push(Work::Close);
                    work.extend(node.children.iter().rev().map(Work::Node));
                }
            },
        }
    }
    output
}

fn push_framed_text(output: &mut String, value: &str) {
    output.push_str(&value.len().to_string());
    output.push(':');
    output.push_str(value);
    output.push(';');
}

fn format_f64(value: f64) -> String {
    if value == 0.0 && value.is_sign_negative() {
        "-0.0".into()
    } else {
        format!("{:016x}", value.to_bits())
    }
}
