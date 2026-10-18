//! Test-only normalized closure projection. Rust admission remains a Tier 1 boundary.
//! No expression evaluation, native instruction selection, or runtime templates.
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::execution::normalized::prepare::{
    NormalizedFunctionBody, NormalizedInstruction as I, NormalizedProgram,
};
use crate::platform::kernel::{FunctionEffect, ParameterUse, TypeForm};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) const HEADER: usize = 72;
pub(crate) const DECLARATIONS: usize = 176;
pub(crate) const MAX_FUNCTIONS: usize = 256;
pub(crate) const MAX_NODES: usize = 4096;
pub(crate) const MAX_LOCALS: usize = 128;
pub(crate) const MAX_STACK: usize = 128;
pub(crate) const MAX_INPUT: usize = 2 * 1024 * 1024;

fn unsupported() -> Diagnostic {
    Diagnostic::new(
        DiagnosticClass::Source,
        "compiler_input_unsupported",
        "LKJCI1 profile 2 requires a bounded pure scalar function closure with forward branches",
    )
}

#[derive(Clone, Debug, Default)]
pub(crate) struct Node {
    pub tag: u32,
    pub a: u32,
    pub b: u32,
    pub c: u32,
    pub immediate: i64,
    pub initialized: Vec<u8>,
    pub stack: Vec<u8>,
}
#[derive(Clone, Debug, Default)]
pub(crate) struct Function {
    pub result: u32,
    pub parameters: u32,
    pub locals: Vec<u8>,
    pub external: u32,
    pub nodes: Vec<Node>,
}
#[derive(Clone, Debug)]
struct State {
    initialized: Vec<u8>,
    stack: Vec<u8>,
}

fn scalar(
    program: &NormalizedProgram,
    ty: crate::platform::kernel::TypeObjectDigest,
) -> Result<u8, Diagnostic> {
    match program.types.get(&ty).map(|ty| &ty.form) {
        Some(TypeForm::I64) => Ok(1),
        Some(TypeForm::Bool) => Ok(2),
        Some(TypeForm::Unit) => Ok(3),
        _ => Err(unsupported()),
    }
}
fn primitive(name: &str) -> Result<u32, Diagnostic> {
    Ok(match name {
        "core.i64.add" => 1,
        "core.i64.subtract" => 2,
        "core.i64.multiply" => 3,
        "core.i64.divide" => 4,
        "core.i64.equal" => 5,
        "core.i64.less" => 6,
        "core.i64.less-equal" => 7,
        "core.bool.not" => 8,
        "core.bool.and" => 9,
        "core.bool.or" => 10,
        _ => return Err(unsupported()),
    })
}

pub(crate) fn export(program: &NormalizedProgram, entry: usize) -> Result<Vec<u8>, Diagnostic> {
    let mut reachable = BTreeSet::new();
    let mut pending = vec![entry];
    while let Some(index) = pending.pop() {
        if !reachable.insert(index) {
            continue;
        }
        if reachable.len() > MAX_FUNCTIONS {
            return Err(unsupported());
        }
        let function = program.functions.get(index).ok_or_else(unsupported)?;
        if function.effect != FunctionEffect::Pure
            || !function.type_parameters.is_empty()
            || !function.type_arguments.is_empty()
            || !function.implementation_parameters.is_empty()
            || !function.implementation_arguments.is_empty()
            || !function.requirement_parameters.is_empty()
            || !function.requirement_arguments.is_empty()
            || !function.effect_parameters.is_empty()
            || !function.effect_arguments.is_empty()
            || !function.read_scope_parameters.is_empty()
            || !function.read_scope_arguments.is_empty()
            || !function.task_requirements.is_empty()
        {
            return Err(unsupported());
        }
        scalar(program, function.result.ty)?;
        for parameter in function.parameters.iter() {
            scalar(program, parameter.ty)?;
            if parameter.use_mode != ParameterUse::Unrestricted {
                return Err(unsupported());
            }
        }
        if let NormalizedFunctionBody::Code(code) = &function.body {
            for instruction in code.instructions.iter() {
                if let I::Call { function, .. } | I::TailCall { function, .. } = instruction {
                    pending.push(function.0 as usize);
                }
            }
        }
    }
    let indices: BTreeMap<_, _> = reachable
        .iter()
        .enumerate()
        .map(|(i, &f)| (f, i as u32))
        .collect();
    let mut functions = Vec::new();
    for &index in &reachable {
        let function = &program.functions[index];
        let locals = function
            .parameters
            .iter()
            .map(|p| scalar(program, p.ty))
            .collect::<Result<Vec<_>, _>>()?;
        functions.push(Function {
            result: u32::from(scalar(program, function.result.ty)?),
            parameters: function.parameter_count,
            locals,
            external: 0,
            nodes: vec![],
        });
    }
    let signatures = functions.clone();
    for (dense, &index) in reachable.iter().enumerate() {
        let function = &mut functions[dense];
        match &program.functions[index].body {
            NormalizedFunctionBody::External(name) => function.external = primitive(name.as_str())?,
            NormalizedFunctionBody::Code(code) => {
                if code.local_count as usize > MAX_LOCALS
                    || code.parameter_count != function.parameters
                    || code.local_count < code.parameter_count
                    || code.instructions.len() > MAX_NODES
                    || code.instructions.is_empty()
                {
                    return Err(unsupported());
                }
                function.locals.resize(code.local_count as usize, 0);
                let mut initial = State {
                    initialized: vec![0; code.local_count as usize],
                    stack: vec![],
                };
                initial.initialized[..code.parameter_count as usize].fill(1);
                let mut states = vec![None; code.instructions.len()];
                states[0] = Some(initial.clone());
                let mut linear = initial;
                for (ip, instruction) in code.instructions.iter().enumerate() {
                    let before = states[ip].take().unwrap_or_else(|| linear.clone());
                    let mut after = before.clone();
                    let mut node = Node {
                        initialized: before.initialized.clone(),
                        stack: before.stack.clone(),
                        ..Node::default()
                    };
                    let mut successors = vec![ip + 1];
                    match instruction {
                        I::I64(value) => {
                            node.tag = 1;
                            node.immediate = *value;
                            after.stack.push(1);
                        }
                        I::Bool(value) => {
                            node.tag = 2;
                            node.immediate = i64::from(*value);
                            after.stack.push(2);
                        }
                        I::Unit => {
                            node.tag = 3;
                            after.stack.push(3);
                        }
                        I::LoadLocal {
                            local,
                            use_mode: ParameterUse::Unrestricted,
                        }
                        | I::MoveLocal(local) => {
                            node.tag = 4;
                            node.a = *local;
                            if after.initialized.get(*local as usize) != Some(&1) {
                                return Err(unsupported());
                            }
                            let ty = *function
                                .locals
                                .get(*local as usize)
                                .filter(|&&ty| ty != 0)
                                .ok_or_else(unsupported)?;
                            after.stack.push(ty);
                        }
                        I::StoreLocal(local) => {
                            node.tag = 5;
                            node.a = *local;
                            let ty = after.stack.pop().ok_or_else(unsupported)?;
                            let slot = function
                                .locals
                                .get_mut(*local as usize)
                                .ok_or_else(unsupported)?;
                            if ty == 3 && *slot != 0 && *slot != 3 {
                                after.initialized[*local as usize] = 0;
                            } else {
                                if *slot != 0 && *slot != ty {
                                    return Err(unsupported());
                                }
                                *slot = ty;
                                after.initialized[*local as usize] = 1;
                            }
                        }
                        I::Drop => {
                            node.tag = 6;
                            after.stack.pop().ok_or_else(unsupported)?;
                        }
                        I::Jump(target) | I::JumpIfFalse(target) => {
                            node.a = *target;
                            if *target as usize <= ip {
                                return Err(unsupported());
                            }
                            if matches!(instruction, I::JumpIfFalse(_)) {
                                node.tag = 8;
                                if after.stack.pop() != Some(2) {
                                    return Err(unsupported());
                                }
                                successors.push(*target as usize);
                            } else {
                                node.tag = 7;
                                successors = vec![*target as usize];
                            }
                        }
                        I::Call {
                            function: target,
                            arguments,
                            type_arguments,
                            requirement_arguments,
                            effect_arguments,
                            read_scope_arguments,
                        }
                        | I::TailCall {
                            function: target,
                            arguments,
                            type_arguments,
                            requirement_arguments,
                            effect_arguments,
                            read_scope_arguments,
                        } => {
                            if !type_arguments.is_empty()
                                || !requirement_arguments.is_empty()
                                || !effect_arguments.is_empty()
                                || !read_scope_arguments.is_empty()
                            {
                                return Err(unsupported());
                            }
                            let callee =
                                *indices.get(&(target.0 as usize)).ok_or_else(unsupported)?;
                            let signature = &signatures[callee as usize];
                            if *arguments != signature.parameters
                                || after.stack.len() < *arguments as usize
                            {
                                return Err(unsupported());
                            }
                            let start = after.stack.len() - *arguments as usize;
                            if after.stack[start..] != signature.locals {
                                return Err(unsupported());
                            }
                            after.stack.truncate(start);
                            after.stack.push(signature.result as u8);
                            node.tag = if matches!(instruction, I::TailCall { .. }) {
                                10
                            } else {
                                9
                            };
                            node.a = callee;
                            node.b = *arguments;
                            if node.tag == 10 {
                                if start != 0 || signature.result != function.result {
                                    return Err(unsupported());
                                }
                                successors.clear();
                            }
                        }
                        I::Return => {
                            node.tag = 11;
                            if after.stack != [function.result as u8] {
                                return Err(unsupported());
                            }
                            successors.clear();
                        }
                        _ => return Err(unsupported()),
                    }
                    if before.stack.len() > MAX_STACK || after.stack.len() > MAX_STACK {
                        return Err(unsupported());
                    }
                    for successor in successors {
                        let state = states.get_mut(successor).ok_or_else(unsupported)?;
                        if let Some(state) = state {
                            if state.stack != after.stack {
                                return Err(unsupported());
                            }
                            for (current, other) in
                                state.initialized.iter_mut().zip(&after.initialized)
                            {
                                *current &= *other;
                            }
                        } else {
                            *state = Some(after.clone());
                        }
                    }
                    linear = after;
                    function.nodes.push(node);
                }
                // Unused local slots have a canonical scalar storage type; no read can be admitted without initialization.
                for ty in &mut function.locals {
                    if *ty == 0 {
                        *ty = 1;
                    }
                }
            }
        }
    }
    let root = *indices.get(&entry).ok_or_else(unsupported)?;
    if functions[root as usize].parameters != 0 || functions[root as usize].result != 1 {
        return Err(unsupported());
    }
    encode(&functions, root)
}

pub(crate) fn encode(functions: &[Function], entry: u32) -> Result<Vec<u8>, Diagnostic> {
    let count = functions.len();
    let node_count: usize = functions.iter().map(|f| f.nodes.len()).sum();
    if count == 0 || count > MAX_FUNCTIONS || node_count == 0 || node_count > MAX_NODES {
        return Err(unsupported());
    }
    let node_offset = DECLARATIONS + count * 40;
    let blob_offset = node_offset + node_count * 32;
    let mut bytes = vec![0u8; blob_offset];
    let mut node_index = 0;
    for (index, function) in functions.iter().enumerate() {
        if function.locals.len() > MAX_LOCALS {
            return Err(unsupported());
        }
        let at = DECLARATIONS + index * 40;
        let local_offset = bytes.len();
        for (i, value) in [
            0,
            function.result,
            function.parameters,
            function.locals.len() as u32,
            node_index as u32,
            function.nodes.len() as u32,
            function.external,
            local_offset as u32,
            0,
            0,
        ]
        .into_iter()
        .enumerate()
        {
            word(&mut bytes, at + i * 4, value);
        }
        bytes.extend_from_slice(&function.locals);
        for node in &function.nodes {
            if node.initialized.len() != function.locals.len() || node.stack.len() > MAX_STACK {
                return Err(unsupported());
            }
            let at = node_offset + node_index * 32;
            let state_offset = bytes.len();
            for (i, value) in [node.tag, node.a, node.b, node.c].into_iter().enumerate() {
                word(&mut bytes, at + i * 4, value);
            }
            bytes[at + 16..at + 24].copy_from_slice(&node.immediate.to_le_bytes());
            word(&mut bytes, at + 24, state_offset as u32);
            word(&mut bytes, at + 28, node.stack.len() as u32);
            bytes.extend_from_slice(&node.initialized);
            bytes.extend_from_slice(&node.stack);
            node_index += 1;
        }
    }
    if bytes.len() > MAX_INPUT {
        return Err(unsupported());
    }
    let blob_size = bytes.len() - blob_offset;
    let fuel = 6 * blob_size + 32 * node_count + count + 3;
    bytes[..8].copy_from_slice(b"LKJCI001");
    word(&mut bytes, 8, 2);
    word(&mut bytes, 12, entry);
    let length = bytes.len() as u64;
    bytes[16..24].copy_from_slice(&length.to_le_bytes());
    bytes[24..32].copy_from_slice(&(count as u64).to_le_bytes());
    bytes[32..40].copy_from_slice(&(fuel as u64).to_le_bytes());
    for (i, value) in [
        112,
        1,
        128,
        3,
        DECLARATIONS as u32,
        count as u32,
        node_offset as u32,
        node_count as u32,
        blob_offset as u32,
        blob_size as u32,
        0,
        count as u32,
        0,
        0,
        1,
        0,
        0,
        0,
        2,
        0,
        0,
        0,
        3,
        0,
        0,
        0,
    ]
    .into_iter()
    .enumerate()
    {
        word(&mut bytes, HEADER + i * 4, value);
    }
    rechecksum(&mut bytes);
    Ok(bytes)
}
fn word(bytes: &mut [u8], at: usize, value: u32) {
    bytes[at..at + 4].copy_from_slice(&value.to_le_bytes());
}

pub(crate) fn rechecksum(bytes: &mut [u8]) {
    let mut hasher = blake3::Hasher::new();
    hasher.update(&bytes[..40]);
    hasher.update(&bytes[HEADER..]);
    bytes[40..HEADER].copy_from_slice(hasher.finalize().as_bytes());
}

/// Small independent test builder. Arithmetic operation IDs are test notation,
/// not a second wire reader; only profile 2 is emitted.
pub(crate) fn arithmetic(operations: &[(u32, i64)]) -> Result<Vec<u8>, Diagnostic> {
    let mut root = Function {
        result: 1,
        ..Function::default()
    };
    let mut stack = Vec::new();
    for &(op, immediate) in operations {
        let mut node = Node {
            stack: stack.clone(),
            immediate,
            ..Node::default()
        };
        match op {
            1 => {
                node.tag = 1;
                stack.push(1);
            }
            2..=5 => {
                node.tag = 9;
                node.a = op - 1;
                node.b = 2;
                stack.truncate(stack.len().saturating_sub(2));
                stack.push(1);
            }
            6 => node.tag = 11,
            _ => node.tag = op,
        }
        root.nodes.push(node);
    }
    let mut functions = vec![root];
    for external in 1..=4 {
        functions.push(Function {
            result: 1,
            parameters: 2,
            locals: vec![1, 1],
            external,
            nodes: vec![],
        });
    }
    encode(&functions, 0)
}
