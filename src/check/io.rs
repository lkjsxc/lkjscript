//! Exact K signatures for authority-bearing host operations.
use crate::k::*;

fn authority(types: &[Type], ty: usize, kind: &TypeKind) -> bool {
    matches!(types[ty].kind, TypeKind::Ref { inner, mutable: true }
        if types[inner].kind == *kind)
}
fn bytes(types: &[Type], ty: usize, mutable: bool) -> bool {
    matches!(types[ty].kind, TypeKind::Ref { inner, mutable: m } if m == mutable
        && matches!(types[inner].kind, TypeKind::Slice { inner: U8, mutable: false }))
}
fn opened(types: &[Type], ty: usize, kind: &TypeKind) -> bool {
    match &types[ty].kind {
        TypeKind::Enum {
            variants,
            copy: false,
        } if variants.len() == 2 => {
            variants[0].fields.len() == 1
                && variants[1].fields.len() == 1
                && types[variants[0].fields[0].ty].kind == *kind
                && variants[1].fields[0].ty == I64
        }
        _ => false,
    }
}
pub(super) fn signature(types: &[Type], result: usize, args: &[usize], name: &str) -> Option<bool> {
    let valid = match name {
        "world_stdout" | "world_stdin" | "world_stderr" => {
            let kind = match name {
                "world_stdout" => TypeKind::Stdout,
                "world_stdin" => TypeKind::Stdin,
                _ => TypeKind::Stderr,
            };
            args.len() == 1
                && authority(types, args[0], &TypeKind::World)
                && types[result].kind == kind
        }
        "world_arg_count" => {
            args.len() == 1 && authority(types, args[0], &TypeKind::World) && result == U64
        }
        "world_arg_len" => {
            args.len() == 2
                && authority(types, args[0], &TypeKind::World)
                && args[1] == U64
                && result == U64
        }
        "world_arg_copy" => {
            args.len() == 3
                && authority(types, args[0], &TypeKind::World)
                && args[1] == U64
                && bytes(types, args[2], true)
                && result == U64
        }
        "world_exit_status" => {
            args.len() == 2
                && authority(types, args[0], &TypeKind::World)
                && args[1] == U8
                && result == UNIT
        }
        "world_directory" => {
            args.len() == 2
                && authority(types, args[0], &TypeKind::World)
                && bytes(types, args[1], false)
                && opened(types, result, &TypeKind::Directory)
        }
        "directory_open" => {
            args.len() == 3
                && authority(types, args[0], &TypeKind::Directory)
                && bytes(types, args[1], false)
                && args[2] == BOOL
                && opened(types, result, &TypeKind::File)
        }
        "stdin_read" | "file_read" => {
            let kind = if name == "stdin_read" {
                TypeKind::Stdin
            } else {
                TypeKind::File
            };
            args.len() == 2
                && authority(types, args[0], &kind)
                && bytes(types, args[1], true)
                && result == I64
        }
        "file_write" => {
            args.len() == 2
                && authority(types, args[0], &TypeKind::File)
                && bytes(types, args[1], false)
                && result == I64
        }
        "stdout_write" | "stderr_write" => {
            let kind = if name == "stdout_write" {
                TypeKind::Stdout
            } else {
                TypeKind::Stderr
            };
            args.len() == 2
                && authority(types, args[0], &kind)
                && bytes(types, args[1], false)
                && result == UNIT
        }
        _ => return None,
    };
    Some(valid)
}
