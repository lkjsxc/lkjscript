//! Exact IR signatures for authority-bearing host operations.
use crate::ir::*;

fn authority(types: &[Type], ty: usize, kind: &TypeKind) -> bool {
    matches!(types[ty].kind, TypeKind::Ref { inner, mutable: true }
        if types[inner].kind == *kind)
}
fn bytes(types: &[Type], ty: usize, mutable: bool) -> bool {
    matches!(types[ty].kind, TypeKind::Ref { inner, mutable: m } if m == mutable
        && matches!(types[inner].kind, TypeKind::Slice { inner: byte, mutable: false } if byte == U8))
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
pub(super) fn signature(
    types: &[Type],
    result: usize,
    args: &[usize],
    name: IntrinsicOp,
) -> Option<bool> {
    let valid = match name {
        IntrinsicOp::WorldStdout | IntrinsicOp::WorldStdin | IntrinsicOp::WorldStderr => {
            let kind = match name {
                IntrinsicOp::WorldStdout => TypeKind::Stdout,
                IntrinsicOp::WorldStdin => TypeKind::Stdin,
                _ => TypeKind::Stderr,
            };
            args.len() == 1
                && authority(types, args[0], &TypeKind::World)
                && types[result].kind == kind
        }
        IntrinsicOp::WorldArgCount => {
            args.len() == 1 && authority(types, args[0], &TypeKind::World) && result == U64
        }
        IntrinsicOp::WorldArgLen => {
            args.len() == 2
                && authority(types, args[0], &TypeKind::World)
                && args[1] == U64
                && result == U64
        }
        IntrinsicOp::WorldArgCopy => {
            args.len() == 3
                && authority(types, args[0], &TypeKind::World)
                && args[1] == U64
                && bytes(types, args[2], true)
                && result == U64
        }
        IntrinsicOp::WorldExitStatus => {
            args.len() == 2
                && authority(types, args[0], &TypeKind::World)
                && args[1] == U8
                && result == UNIT
        }
        IntrinsicOp::WorldDirectory => {
            args.len() == 2
                && authority(types, args[0], &TypeKind::World)
                && bytes(types, args[1], false)
                && opened(types, result, &TypeKind::Directory)
        }
        IntrinsicOp::DirectoryOpen => {
            args.len() == 3
                && authority(types, args[0], &TypeKind::Directory)
                && bytes(types, args[1], false)
                && args[2] == BOOL
                && opened(types, result, &TypeKind::File)
        }
        IntrinsicOp::StdinRead | IntrinsicOp::FileRead => {
            let kind = if name == IntrinsicOp::StdinRead {
                TypeKind::Stdin
            } else {
                TypeKind::File
            };
            args.len() == 2
                && authority(types, args[0], &kind)
                && bytes(types, args[1], true)
                && result == I64
        }
        IntrinsicOp::FileWrite => {
            args.len() == 2
                && authority(types, args[0], &TypeKind::File)
                && bytes(types, args[1], false)
                && result == I64
        }
        IntrinsicOp::StdoutWrite | IntrinsicOp::StderrWrite => {
            let kind = if name == IntrinsicOp::StdoutWrite {
                TypeKind::Stdout
            } else {
                TypeKind::Stderr
            };
            args.len() == 2
                && authority(types, args[0], &kind)
                && bytes(types, args[1], false)
                && result == UNIT
        }
        IntrinsicOp::WorldProcess | IntrinsicOp::WorldClock => {
            args.len() == 1
                && authority(types, args[0], &TypeKind::World)
                && types[result].kind
                    == if name == IntrinsicOp::WorldProcess {
                        TypeKind::Process
                    } else {
                        TypeKind::Clock
                    }
        }
        IntrinsicOp::WorldFile => {
            args.len() == 2
                && authority(types, args[0], &TypeKind::World)
                && args[1] == U8
                && opened(types, result, &TypeKind::File)
        }
        IntrinsicOp::ClockRead => {
            args.len() == 2
                && authority(types, args[0], &TypeKind::Clock)
                && bytes(types, args[1], true)
                && result == I64
        }
        IntrinsicOp::ProcessSpawn => {
            if args.len() != 7
                || !authority(types, args[0], &TypeKind::Process)
                || !bytes(types, args[2], false)
                || !opened(types, result, &TypeKind::Child)
            {
                false
            } else {
                let mut valid = true;
                for index in vec![1usize, 3, 4, 5, 6] {
                    if !matches!(types[args[index]].kind, TypeKind::Ref { inner, mutable: false } if types[inner].kind == TypeKind::File)
                    {
                        valid = false;
                    }
                }
                valid
            }
        }
        IntrinsicOp::ChildWait => {
            args.len() == 2
                && authority(types, args[0], &TypeKind::Child)
                && args[1] == BOOL
                && result == I64
        }
        IntrinsicOp::ChildTerminate => {
            args.len() == 1 && authority(types, args[0], &TypeKind::Child) && result == I64
        }
        IntrinsicOp::DirectoryOpenDirectory => {
            args.len() == 2
                && authority(types, args[0], &TypeKind::Directory)
                && bytes(types, args[1], false)
                && opened(types, result, &TypeKind::Directory)
        }
        IntrinsicOp::DirectoryRead => {
            args.len() == 2
                && authority(types, args[0], &TypeKind::Directory)
                && bytes(types, args[1], true)
                && result == I64
        }
        IntrinsicOp::DirectoryRewind => {
            args.len() == 1 && authority(types, args[0], &TypeKind::Directory) && result == I64
        }
        IntrinsicOp::DirectoryCreate => {
            args.len() == 2
                && authority(types, args[0], &TypeKind::Directory)
                && bytes(types, args[1], false)
                && result == I64
        }
        IntrinsicOp::DirectoryRemove => {
            args.len() == 3
                && authority(types, args[0], &TypeKind::Directory)
                && bytes(types, args[1], false)
                && args[2] == BOOL
                && result == I64
        }
        IntrinsicOp::DirectoryRename => {
            args.len() == 4
                && authority(types, args[0], &TypeKind::Directory)
                && bytes(types, args[1], false)
                && authority(types, args[2], &TypeKind::Directory)
                && bytes(types, args[3], false)
                && result == I64
        }
        IntrinsicOp::FileDuplicate | IntrinsicOp::FileReadOnly => {
            args.len() == 1
                && authority(types, args[0], &TypeKind::File)
                && opened(types, result, &TypeKind::File)
        }
        IntrinsicOp::FileRewind | IntrinsicOp::FileExecutable => {
            args.len() == 1 && authority(types, args[0], &TypeKind::File) && result == I64
        }
        _ => return None,
    };
    Some(valid)
}
