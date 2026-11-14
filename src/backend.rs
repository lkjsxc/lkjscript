//! Checked lkjscript IR and a sealed runtime produce native bytes in memory.
use crate::diagnostic::{Diagnostic, Span};
use crate::ir::{CheckedProgram, TypeKind};
use crate::{check, ir_text, native, runtime};

fn error(message: &str) -> Diagnostic {
    Diagnostic::new(Span::default(), "E_ENTRY", message)
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
    emit_artifact(checked, runtime, entry, None)
}

/// The driver holds fd3. A normal return writes only the raw eight-byte scalar;
/// traps report their location/message on stderr and leave that channel empty.
pub fn emit_for_run(
    checked: &CheckedProgram,
    runtime: &runtime::Runtime,
    entry: &str,
) -> Result<Vec<u8>, Diagnostic> {
    emit_artifact(checked, runtime, entry, Some(3))
}

fn emit_artifact(
    checked: &CheckedProgram,
    runtime: &runtime::Runtime,
    entry: &str,
    result_fd: Option<u32>,
) -> Result<Vec<u8>, Diagnostic> {
    let meaning = checked.program();
    let mut selected = None;
    for (index, function) in meaning.functions.iter().enumerate() {
        if function.name == entry {
            selected = Some(index);
            break;
        }
    }
    let index = selected.ok_or_else(|| error(&format!("unknown function {entry}")))?;
    let function = &meaning.functions[index];
    if !matches!(
        meaning.types[function.result].kind,
        TypeKind::Unit | TypeKind::Bool | TypeKind::Char | TypeKind::Int { .. } | TypeKind::F64
    ) {
        return Err(error("host entry result must be unit or a scalar"));
    }
    let mut arguments = Vec::new();
    if function.params == 1 && matches!(meaning.types[function.locals[0].ty].kind, TypeKind::World)
    {
        arguments.push(1);
    } else if function.params != 0 {
        return Err(error("entry takes no arguments or exactly one World"));
    }
    Ok(native::encode(
        checked,
        runtime,
        native::Entry {
            function: index,
            arguments,
            result_fd,
        },
    )?
    .bytes)
}
