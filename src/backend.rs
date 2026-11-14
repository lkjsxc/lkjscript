//! The backend boundary: declared program IR and runtime IR produce native bytes.
//! Parsing, verification, runtime sealing and encoding share the production files.
use super::result_wire::{self, Descriptor, ScalarKind};
use crate::diagnostic::{Diagnostic, Span};
use crate::ir::{CheckedProgram, TypeKind};
use crate::{check, ir_text, lower, native, runtime};

fn error(code: &str, message: &str) -> Diagnostic {
    Diagnostic::new(Span::default(), code, message)
}

pub fn compile(text: &str, runtime_text: &str, entry: &str) -> Result<Vec<u8>, Diagnostic> {
    let checked = check::check(ir_text::read(text)?)?;
    let runtime = runtime::seal(check::check(ir_text::read(runtime_text)?)?)?;
    emit(&checked, &runtime, entry)
}

pub fn emit(
    checked: &CheckedProgram,
    runtime: &runtime::Runtime,
    entry: &str,
) -> Result<Vec<u8>, Diagnostic> {
    Ok(emit_artifact(checked, runtime, entry, false)?.0)
}

/// Emit an fd3 result-packet ELF and metadata from the same checked lowering.
pub fn emit_for_eval(
    checked: &CheckedProgram,
    runtime: &runtime::Runtime,
    entry: &str,
) -> Result<(Vec<u8>, Vec<u8>), Diagnostic> {
    emit_artifact(checked, runtime, entry, true)
}

fn emit_artifact(
    checked: &CheckedProgram,
    runtime: &runtime::Runtime,
    entry: &str,
    packet: bool,
) -> Result<(Vec<u8>, Vec<u8>), Diagnostic> {
    let meaning = checked.program();
    let mut selected = None;
    for function in &meaning.functions {
        if function.name == entry {
            selected = Some(function);
            break;
        }
    }
    let function = match selected {
        Some(value) => value,
        None => return Err(error("E_ENTRY", &format!("unknown function {entry}"))),
    };
    let result = &meaning.types[function.result].kind;
    let kind = match result {
        TypeKind::Unit => ScalarKind::Unit,
        TypeKind::Bool => ScalarKind::Bool,
        TypeKind::Char => ScalarKind::Char,
        TypeKind::Int { bits, signed } => ScalarKind::Int {
            bits: *bits,
            signed: *signed,
        },
        TypeKind::F64 => ScalarKind::F64,
        _ => {
            return Err(error(
                "E_ENTRY",
                "host entry result must be unit or a scalar",
            ));
        }
    };
    let mut arguments = Vec::new();
    if function.params == 1 && matches!(meaning.types[function.locals[0].ty].kind, TypeKind::World)
    {
        arguments.push(1);
    } else if function.params != 0 {
        return Err(error(
            "E_ENTRY",
            "entry takes no arguments or exactly one World",
        ));
    }
    let program = lower::lower(checked, runtime).map_err(|message| error("E_EMIT", &message))?;
    let exported_name = format!("fn:{entry}");
    let mut exported = None;
    for item in &program.exports {
        if item.name == exported_name {
            exported = Some(item.function);
            break;
        }
    }
    let function = match exported {
        Some(value) => value,
        None => return Err(error("E_ENTRY", "lowered entry is missing")),
    };
    let bytes = native::encode(
        &program,
        native::Entry {
            function,
            arguments,
            result_fd: if packet { Some(3) } else { None },
        },
    )
    .map_err(|message| error("E_EMIT", &message))?;
    let descriptor = if packet {
        result_wire::encode(&Descriptor {
            kind,
            traps: program.trap_messages,
        })
        .map_err(|message| error("E_EMIT", &message))?
    } else {
        Vec::new()
    };
    Ok((bytes, descriptor))
}
