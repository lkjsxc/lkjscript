//! One inventory of methods implemented directly by the front end. Candidate
//! discovery can use this inventory without manufacturing a K receiver layout.
use super::k;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Receiver {
    Integer,
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
    Other,
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Direct {
    Wrapping,
    Character,
    Text,
    Output,
    Slice,
    Length,
    Push,
    Pop,
}
pub(super) fn receiver(kind: &k::TypeKind) -> Receiver {
    match kind {
        k::TypeKind::Int { .. } => Receiver::Integer,
        k::TypeKind::Char => Receiver::Character,
        k::TypeKind::Str => Receiver::Text,
        k::TypeKind::Vec { .. } => Receiver::Vector,
        k::TypeKind::Array { .. } => Receiver::Array,
        k::TypeKind::Slice { .. } => Receiver::Slice,
        k::TypeKind::World => Receiver::World,
        k::TypeKind::Stdout => Receiver::Stdout,
        k::TypeKind::Stdin => Receiver::Stdin,
        k::TypeKind::Stderr => Receiver::Stderr,
        k::TypeKind::Directory => Receiver::Directory,
        k::TypeKind::File => Receiver::File,
        _ => Receiver::Other,
    }
}
pub(super) fn direct(receiver: Receiver, name: &str) -> Option<Direct> {
    if super::capabilities::method(receiver, name).is_some() {
        return Some(Direct::Output);
    }
    match (receiver, name) {
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
