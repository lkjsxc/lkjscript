use super::usefulness::Witness;
use crate::analyze::*;

pub(super) fn render(witness: &Witness) -> String {
    match witness {
        Witness::Wild(ty) => format!("wildcard<{}>", canonical_type(ty)),
        Witness::Constructor(ty, label, fields) if fields.is_empty() => {
            format!("{}::{label}", canonical_type(ty))
        }
        Witness::Constructor(ty, label, fields) => format!(
            "{}::{label}({})",
            canonical_type(ty),
            fields.iter().map(render).collect::<Vec<_>>().join(","),
        ),
    }
}

fn canonical_type(ty: &Type) -> String {
    match ty {
        Type::Bool => "Bool".into(),
        Type::I64 => "I64".into(),
        Type::Product(_) => "Product".into(),
        Type::Enum { id, arguments, .. } => format!(
            "Enum#{}<{}>",
            hex(id.bytes()),
            arguments
                .iter()
                .map(canonical_type)
                .collect::<Vec<_>>()
                .join(","),
        ),
        other => format!("{other:?}"),
    }
}

fn hex(bytes: [u8; 32]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(64);
    for byte in bytes {
        output.push(char::from(DIGITS[usize::from(byte >> 4)]));
        output.push(char::from(DIGITS[usize::from(byte & 0x0f)]));
    }
    output
}
