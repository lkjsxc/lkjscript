//! One inventory of methods implemented directly by the front end. Candidate
//! discovery can use this inventory without manufacturing a IR receiver layout.
use super::ir;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Receiver {
    Integer,
    Float,
    Character,
    Text,
    Vector,
    Array,
    Slice,
    World,
    Stdout,
    Stdin,
    Stderr,
    Directory,
    File,
    Process,
    Clock,
    Child,
    Other,
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Direct {
    Wrapping,
    FloatBits,
    Character,
    Text,
    Output,
    Slice,
    Length,
    Push,
    Pop,
}
pub(super) fn receiver(kind: &ir::TypeKind) -> Receiver {
    match kind {
        ir::TypeKind::Int { .. } => Receiver::Integer,
        ir::TypeKind::Char => Receiver::Character,
        ir::TypeKind::F64 => Receiver::Float,
        ir::TypeKind::Str => Receiver::Text,
        ir::TypeKind::Vec { .. } => Receiver::Vector,
        ir::TypeKind::Array { .. } => Receiver::Array,
        ir::TypeKind::Slice { .. } => Receiver::Slice,
        ir::TypeKind::World => Receiver::World,
        ir::TypeKind::Stdout => Receiver::Stdout,
        ir::TypeKind::Stdin => Receiver::Stdin,
        ir::TypeKind::Stderr => Receiver::Stderr,
        ir::TypeKind::Directory => Receiver::Directory,
        ir::TypeKind::File => Receiver::File,
        ir::TypeKind::Process => Receiver::Process,
        ir::TypeKind::Clock => Receiver::Clock,
        ir::TypeKind::Child => Receiver::Child,
        _ => Receiver::Other,
    }
}
pub(super) fn direct(receiver: Receiver, name: &str) -> Option<Direct> {
    if super::capabilities::method(receiver, name).is_some() {
        return Some(Direct::Output);
    }
    match (receiver, name) {
        (Receiver::Float, "to_bits") => Some(Direct::FloatBits),
        (Receiver::Integer, "wrapping_add" | "wrapping_sub" | "wrapping_mul") => {
            Some(Direct::Wrapping)
        }
        (
            Receiver::Character,
            "len_utf8"
            | "len_utf16"
            | "is_ascii"
            | "is_ascii_digit"
            | "is_ascii_uppercase"
            | "is_ascii_lowercase"
            | "is_ascii_alphabetic"
            | "is_ascii_alphanumeric"
            | "is_ascii_hexdigit"
            | "is_ascii_whitespace"
            | "is_whitespace"
            | "is_control"
            | "to_digit"
            | "to_ascii_lowercase"
            | "to_ascii_uppercase"
            | "to_string",
        ) => Some(Direct::Character),
        (
            Receiver::Text,
            "len"
            | "is_empty"
            | "as_bytes"
            | "to_string"
            | "starts_with"
            | "ends_with"
            | "contains"
            | "find"
            | "rfind"
            | "replace"
            | "repeat"
            | "is_ascii"
            | "eq_ignore_ascii_case"
            | "to_ascii_lowercase"
            | "to_ascii_uppercase",
        ) => Some(Direct::Text),
        (Receiver::Vector | Receiver::Array, "as_slice" | "as_mut_slice") => Some(Direct::Slice),
        (Receiver::Vector | Receiver::Array | Receiver::Slice, "len") => Some(Direct::Length),
        (Receiver::Vector, "push") => Some(Direct::Push),
        (Receiver::Vector, "pop") => Some(Direct::Pop),
        _ => None,
    }
}
