//! The K compiler boundary: declared program K and runtime K produce native bytes.
//! Parsing, verification, runtime sealing and encoding share the production files.
use crate::diagnostic::{Diagnostic, Span};
use crate::k::{CheckedProgram, TypeKind};
use crate::{check, k_text, lower, native, runtime};

fn error(code: &str, message: &str) -> Diagnostic {
    Diagnostic::new(Span::default(), code, message)
}

pub fn compile(text: &str, runtime_text: &str, entry: &str) -> Result<Vec<u8>, Diagnostic> {
    let checked = check::check(k_text::read(text)?)?;
    let runtime = runtime::seal(check::check(k_text::read(runtime_text)?)?)?;
    emit(&checked, &runtime, entry)
}

pub fn emit(
    checked: &CheckedProgram,
    runtime: &runtime::Runtime,
    entry: &str,
) -> Result<Vec<u8>, Diagnostic> {
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
    if !matches!(
        result,
        TypeKind::Unit | TypeKind::Bool | TypeKind::Char | TypeKind::Int { .. } | TypeKind::F64
    ) {
        return Err(error(
            "E_ENTRY",
            "host entry result must be unit or a scalar",
        ));
    }
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
    native::encode(
        &program,
        native::Entry {
            function,
            arguments,
            result_fd: None,
        },
    )
    .map_err(|message| error("E_EMIT", &message))
}
