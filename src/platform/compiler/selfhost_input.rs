//! Bootstrap-only projection of an already admitted normalized function.
//! This exporter performs no instruction selection or expression evaluation.
//! Profile 1 is an intermediate backend boundary, not semantic self-hosting.
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::execution::normalized::prepare::{
    NormalizedFunctionBody, NormalizedInstruction as I, NormalizedProgram,
};
use crate::platform::kernel::{FunctionEffect, TypeForm};

pub(crate) const HEADER: usize = 72;
pub(crate) const NODES: usize = 168;
pub(crate) const MAX_NODES: usize = 1024;

fn unsupported() -> Diagnostic {
    Diagnostic::new(
        DiagnosticClass::Source,
        "compiler_input_unsupported",
        "LKJCI1 profile 1 requires one closed, parameterless I64 expression using literals and checked arithmetic",
    )
}

pub(crate) fn export(program: &NormalizedProgram, entry: usize) -> Result<Vec<u8>, Diagnostic> {
    let function = program.functions.get(entry).ok_or_else(unsupported)?;
    if !function.parameters.is_empty()
        || !function.type_parameters.is_empty()
        || !function.implementation_arguments.is_empty()
        || !function.requirement_parameters.is_empty()
        || !function.read_scope_parameters.is_empty()
        || !function.effect_parameters.is_empty()
        || function.effect != FunctionEffect::Pure
        || !matches!(
            program.types.get(&function.result.ty).map(|t| &t.form),
            Some(TypeForm::I64)
        )
    {
        return Err(unsupported());
    }
    let NormalizedFunctionBody::Code(code) = &function.body else {
        return Err(unsupported());
    };
    if code.instructions.len() > MAX_NODES || code.local_count != 0 {
        return Err(unsupported());
    }
    let mut nodes = Vec::new();
    for instruction in code.instructions.iter() {
        match instruction {
            I::I64(value) => nodes.push((1, *value)),
            I::Return => nodes.push((6, 0)),
            I::Call {
                function,
                arguments: 2,
                ..
            }
            | I::TailCall {
                function,
                arguments: 2,
                ..
            } => {
                let target = program
                    .functions
                    .get(function.0 as usize)
                    .ok_or_else(unsupported)?;
                let NormalizedFunctionBody::External(name) = &target.body else {
                    return Err(unsupported());
                };
                let tag = match name.as_str() {
                    "core.i64.add" => 2,
                    "core.i64.subtract" => 3,
                    "core.i64.multiply" => 4,
                    "core.i64.divide" => 5,
                    _ => return Err(unsupported()),
                };
                nodes.push((tag, 0));
                if matches!(instruction, I::TailCall { .. }) {
                    nodes.push((6, 0));
                }
            }
            _ => return Err(unsupported()),
        }
    }
    if nodes.len() > MAX_NODES {
        return Err(unsupported());
    }
    encode(&nodes)
}

/// Neutral framing also permits deliberately malformed node sequences in tests.
pub(crate) fn encode(nodes: &[(u32, i64)]) -> Result<Vec<u8>, Diagnostic> {
    let size = nodes
        .len()
        .checked_mul(16)
        .and_then(|n| n.checked_add(NODES))
        .filter(|_| nodes.len() <= MAX_NODES)
        .ok_or_else(unsupported)?;
    let count = u32::try_from(nodes.len()).map_err(|_| unsupported())?;
    let mut bytes = Vec::with_capacity(size);
    bytes.extend_from_slice(b"LKJCI001");
    bytes.extend_from_slice(&1_u32.to_le_bytes());
    bytes.extend_from_slice(&0_u32.to_le_bytes());
    bytes.extend_from_slice(&(size as u64).to_le_bytes());
    bytes.extend_from_slice(&1_u64.to_le_bytes());
    bytes.extend_from_slice(&(u64::from(count) + 3).to_le_bytes());
    bytes.extend_from_slice(&[0; 32]);
    for word in [
        104, 1, 120, 1, 136, 1, 168, count, // directory
        0, 1, 0, 0, // module
        1, 0, 0, 0, // I64 type
        0, 0, 0, 0, 0, count, 0, 0, // declaration
    ] {
        bytes.extend_from_slice(&word.to_le_bytes());
    }
    for &(tag, immediate) in nodes {
        bytes.extend_from_slice(&tag.to_le_bytes());
        bytes.extend_from_slice(&0_u32.to_le_bytes());
        bytes.extend_from_slice(&immediate.to_le_bytes());
    }
    rechecksum(&mut bytes);
    Ok(bytes)
}

pub(crate) fn rechecksum(bytes: &mut [u8]) {
    let mut hasher = blake3::Hasher::new();
    hasher.update(&bytes[..40]);
    hasher.update(&bytes[HEADER..]);
    bytes[40..HEADER].copy_from_slice(hasher.finalize().as_bytes());
}
