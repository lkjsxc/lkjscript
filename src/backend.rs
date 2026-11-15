//! Checked lkjscript IR and a sealed runtime produce native bytes in memory.
use crate::diagnostic::{Diagnostic, Span};
use crate::ir::{CheckedProgram, EntryGrant, TypeKind};
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
    emit_artifact(checked, runtime, entry, None, None, None)
}

/// The driver holds fd3. A normal return writes only the raw eight-byte scalar;
/// traps report their location/message on stderr and leave that channel empty.
pub fn emit_for_run(
    checked: &CheckedProgram,
    runtime: &runtime::Runtime,
    entry: &str,
) -> Result<Vec<u8>, Diagnostic> {
    emit_artifact(checked, runtime, entry, Some(3), None, None)
}

pub fn emit_profile(checked: &CheckedProgram, runtime: &runtime::Runtime, entry: &str, path: &str) -> Result<Vec<u8>, Diagnostic> {
    emit_artifact(checked, runtime, entry, None, Some(path), None)
}

pub fn emit_granted(
    checked: &CheckedProgram,
    runtime: &runtime::Runtime,
    entry: &str,
    grants: &[EntryGrant],
    result_fd: Option<u32>,
    profile_path: Option<&str>,
) -> Result<Vec<u8>, Diagnostic> {
    emit_artifact(checked, runtime, entry, result_fd, profile_path, Some(grants))
}

pub fn cache_namespace(
    checked: &crate::reuse::Checked,
    runtime: &runtime::Runtime,
    grants: &[EntryGrant],
    profile_path: Option<&str>,
) -> String {
    native::cache_namespace(checked, runtime, grants, profile_path)
}
pub fn emit_granted_reusing(
    checked: &crate::reuse::Checked,
    runtime: &runtime::Runtime,
    entry: &str,
    grants: &[EntryGrant],
    result_fd: Option<u32>,
    profile_path: Option<&str>,
    namespace: &str,
    records: &mut crate::reuse::Records,
) -> Result<Vec<u8>, Diagnostic> {
    Ok(prepare_granted_reusing(checked, runtime, entry, grants, result_fd, profile_path, namespace, records)?.finish()?.bytes)
}
pub fn prepare_granted_reusing(
    checked: &crate::reuse::Checked,
    runtime: &runtime::Runtime,
    entry: &str,
    grants: &[EntryGrant],
    result_fd: Option<u32>,
    profile_path: Option<&str>,
    namespace: &str,
    records: &mut crate::reuse::Records,
) -> Result<native::PreparedImage, Diagnostic> {
    let meaning = checked.program();
    let mut selected = None;
    for (index, function) in meaning.functions.iter().enumerate() {
        if function.name == entry {
            selected = Some(index);
            break;
        }
    }
    let index = selected.ok_or_else(|| error(&format!("unknown function {entry}")))?;
    if !matches!(
        meaning.types[meaning.functions[index].result].kind,
        TypeKind::Unit | TypeKind::Bool | TypeKind::Char | TypeKind::Int { .. } | TypeKind::F64
    ) {
        return Err(error("host entry result must be unit or a scalar"));
    }
    // The public witness bridge cannot accidentally replay a context prepared
    // for another runtime or label-prefix option set.
    if namespace != native::cache_namespace(checked, runtime, grants, profile_path) {
        return prepare_granted(checked.checked(), runtime, entry, grants, result_fd, profile_path);
    }
    native::prepare_granted_reusing(
        checked, runtime, index, grants, result_fd, profile_path, namespace, records,
    )
}

pub fn prepare_granted(
    checked: &CheckedProgram, runtime: &runtime::Runtime, entry: &str,
    grants: &[EntryGrant], result_fd: Option<u32>, profile_path: Option<&str>,
) -> Result<native::PreparedImage, Diagnostic> {
    let meaning = checked.program();
    let mut selected = None;
    for (index, function) in meaning.functions.iter().enumerate() {
        if function.name == entry { selected = Some(index); break; }
    }
    let index = selected.ok_or_else(|| error(&format!("unknown function {entry}")))?;
    if !matches!(meaning.types[meaning.functions[index].result].kind,
        TypeKind::Unit | TypeKind::Bool | TypeKind::Char | TypeKind::Int { .. } | TypeKind::F64) {
        return Err(error("host entry result must be unit or a scalar"));
    }
    native::prepare_granted(checked, runtime, index, grants, result_fd, profile_path)
}

fn emit_artifact(
    checked: &CheckedProgram,
    runtime: &runtime::Runtime,
    entry: &str,
    result_fd: Option<u32>,
    profile_path: Option<&str>,
    grants: Option<&[EntryGrant]>,
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
    if let Some(grants) = grants {
        let image = native::encode_granted(checked, runtime, index, grants, result_fd, profile_path)?;
        return Ok(image.bytes);
    }
    let mut arguments = Vec::new();
    if function.params == 1 && matches!(meaning.types[function.locals[0].ty].kind, TypeKind::World)
    {
        arguments.push(1);
    } else if function.params != 0 {
        return Err(error("entry takes no arguments or exactly one World"));
    }
    let entry = native::Entry { function: index, arguments, result_fd };
    let image = match profile_path {
        Some(path) => native::encode_profile(checked, runtime, entry, path)?,
        None => native::encode(checked, runtime, entry)?,
    };
    Ok(image.bytes)
}
