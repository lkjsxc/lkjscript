//! Test-only normalized closure projection. Rust admission remains a Tier 1 boundary.
//! No expression evaluation, native instruction selection, or runtime templates.
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::execution::normalized::prepare::{
    NormalizedFieldSelector, NormalizedFunctionBody, NormalizedInstruction as I, NormalizedProgram,
};
use crate::platform::kernel::{
    FunctionEffect, ParameterUse, ResultRelation, TypeForm, TypeObjectDigest,
};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) const HEADER: usize = 72;
pub(crate) const DECLARATIONS: usize = 2176;
pub(crate) const MAX_FUNCTIONS: usize = 1024;
pub(crate) const MAX_NODES: usize = 32768;
pub(crate) const MAX_LOCALS: usize = 128;
pub(crate) const MAX_STACK: usize = 128;
pub(crate) const MAX_INPUT: usize = 8 * 1024 * 1024;

#[track_caller]
fn unsupported() -> Diagnostic {
    Diagnostic::new(
        DiagnosticClass::Source,
        "compiler_input_unsupported",
        format!(
            "LKJCI1 profile 4 requires a bounded closed pure byte/aggregate function closure with forward branches (exporter precondition at {})",
            std::panic::Location::caller()
        ),
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
    pub data: Vec<u8>,
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Record {
    pub fields: Vec<(String, u8)>,
}

#[derive(Default)]
struct Types {
    records: Vec<Record>,
    known: BTreeMap<TypeObjectDigest, u8>,
    active: BTreeSet<TypeObjectDigest>,
}

impl Types {
    fn record(&mut self, mut fields: Vec<(String, u8)>) -> Result<u8, Diagnostic> {
        fields.sort();
        if fields.len() > MAX_STACK
            || fields
                .iter()
                .any(|(name, ty)| name.is_empty() || *ty == 6 || *ty == 134)
            || fields.windows(2).any(|w| w[0].0 == w[1].0)
        {
            return Err(unsupported());
        }
        let record = Record { fields };
        if let Some(index) = self.records.iter().position(|r| r == &record) {
            return Ok(index as u8 + 8);
        }
        if self.records.len() >= 120 {
            return Err(unsupported());
        }
        let ty = self.records.len() as u8 + 8;
        self.records.push(record);
        Ok(ty)
    }

    fn code(
        &mut self,
        program: &NormalizedProgram,
        ty: TypeObjectDigest,
    ) -> Result<u8, Diagnostic> {
        if let Some(code) = self.known.get(&ty) {
            return Ok(*code);
        }
        if !self.active.insert(ty) || self.active.len() > 127 {
            return Err(unsupported());
        }
        let code = match program.types.get(&ty).map(|ty| &ty.form) {
            Some(TypeForm::I64) => 1,
            Some(TypeForm::Bool) => 2,
            Some(TypeForm::Unit) => 3,
            Some(TypeForm::Text) => 4,
            Some(TypeForm::Bytes) => 5,
            Some(TypeForm::ByteBuffer) => 6,
            Some(TypeForm::List { item }) if self.code(program, *item)? == 1 => 7,
            Some(TypeForm::StructuralRecord { fields }) => {
                let mut mapped = vec![];
                for field in fields {
                    mapped.push((
                        field.name.as_str().to_owned(),
                        self.code(program, field.ty)?,
                    ));
                }
                self.record(mapped)?
            }
            _ => return Err(unsupported()),
        };
        self.active.remove(&ty);
        self.known.insert(ty, code);
        Ok(code)
    }

    fn parameter(
        &mut self,
        program: &NormalizedProgram,
        ty: TypeObjectDigest,
        mode: ParameterUse,
    ) -> Result<u8, Diagnostic> {
        let code = self.code(program, ty)?;
        match (code, mode) {
            (6, ParameterUse::Consume) => Ok(6),
            (6, ParameterUse::Borrow) => Ok(134),
            (6, ParameterUse::Unrestricted) => Err(unsupported()),
            (_, ParameterUse::Unrestricted) => Ok(code),
            _ => Err(unsupported()),
        }
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
        "core.bytes.length" => 11,
        "core.bytes.get" => 12,
        "core.bytes.from-list" => 13,
        "core.bytes.from-text" => 14,
        "core.bytes.copy" => 15,
        "core.bytes.slice" => 16,
        "core.bytes.concat" => 17,
        "core.bytes.equal" => 18,
        "core.bytes.blake3" => 19,
        "core.buffer.empty" => 20,
        "core.buffer.push" => 21,
        "core.buffer.length" => 22,
        "core.buffer.get" => 23,
        "core.buffer.freeze" => 24,
        "core.buffer.discard" => 25,
        "core.list.length" => 26,
        "core.list.get" => 27,
        "core.bytes.blake3-derive-key" => 28,
        _ => return Err(unsupported()),
    })
}

pub(crate) fn export(program: &NormalizedProgram, entry: usize) -> Result<Vec<u8>, Diagnostic> {
    let mut types = Types::default();
    let mut reachable = BTreeSet::new();
    let mut instances = BTreeMap::new();
    let mut pending = vec![(entry, Vec::new())];
    while let Some((index, arguments)) = pending.pop() {
        let function = program.functions.get(index).ok_or_else(unsupported)?;
        if function.type_parameters.len() != arguments.len()
            || (!function.type_arguments.is_empty()
                && function.type_arguments.as_ref() != arguments)
        {
            return Err(unsupported());
        }
        let substitutions: BTreeMap<_, _> = function
            .type_parameters
            .iter()
            .copied()
            .zip(arguments)
            .collect();
        if !reachable.insert(index) {
            if instances.get(&index) != Some(&substitutions) {
                return Err(unsupported());
            }
            continue;
        }
        if reachable.len() > MAX_FUNCTIONS {
            return Err(unsupported());
        }
        if function.effect != FunctionEffect::Pure
            || !function.implementation_parameters.is_empty()
            || !function.implementation_arguments.is_empty()
            || !function.requirement_parameters.is_empty()
            || !function.requirement_arguments.is_empty()
            || !function.effect_parameters.is_empty()
            || !function.effect_arguments.is_empty()
            || !function.read_scope_parameters.is_empty()
            || !function.read_scope_arguments.is_empty()
            || !function.task_requirements.is_empty()
            || !matches!(function.result.relation, ResultRelation::Value)
        {
            return Err(unsupported());
        }
        let resolve = |ty| {
            program
                .substitute_type(ty, &substitutions, 0)
                .ok_or_else(unsupported)
        };
        types.code(program, resolve(function.result.ty)?)?;
        for parameter in function.parameters.iter() {
            types.parameter(program, resolve(parameter.ty)?, parameter.use_mode)?;
            if parameter.resource_requirement.is_some() {
                return Err(unsupported());
            }
        }
        if let NormalizedFunctionBody::Code(code) = &function.body {
            for instruction in code.instructions.iter() {
                if let I::Call {
                    function,
                    type_arguments,
                    ..
                }
                | I::TailCall {
                    function,
                    type_arguments,
                    ..
                } = instruction
                {
                    pending.push((
                        function.0 as usize,
                        type_arguments
                            .iter()
                            .map(|ty| resolve(*ty))
                            .collect::<Result<Vec<_>, _>>()?,
                    ));
                }
            }
        }
        instances.insert(index, substitutions);
    }
    let indices: BTreeMap<_, _> = reachable
        .iter()
        .enumerate()
        .map(|(i, &f)| (f, i as u32))
        .collect();
    let mut functions = Vec::new();
    for &index in &reachable {
        let function = &program.functions[index];
        let resolve = |ty| {
            program
                .substitute_type(ty, &instances[&index], 0)
                .ok_or_else(unsupported)
        };
        let locals = function
            .parameters
            .iter()
            .map(|p| types.parameter(program, resolve(p.ty)?, p.use_mode))
            .collect::<Result<Vec<_>, _>>()?;
        functions.push(Function {
            result: u32::from(types.code(program, resolve(function.result.ty)?)?),
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
                        I::Text(value) | I::StaticText(value) => {
                            node.tag = 12;
                            node.b = value.len() as u32;
                            node.data = value.as_bytes().to_vec();
                            after.stack.push(4);
                        }
                        I::List { items } => {
                            let start = after
                                .stack
                                .len()
                                .checked_sub(*items as usize)
                                .ok_or_else(unsupported)?;
                            if after.stack[start..].iter().any(|ty| *ty != 1) {
                                return Err(unsupported());
                            }
                            after.stack.truncate(start);
                            after.stack.push(7);
                            node.tag = 13;
                            node.a = 7;
                            node.b = *items;
                        }
                        I::Record {
                            layout: None,
                            type_arguments,
                            fields,
                        } if type_arguments.is_empty() => {
                            let start = after
                                .stack
                                .len()
                                .checked_sub(fields.len())
                                .ok_or_else(unsupported)?;
                            let mut shape = vec![];
                            for (field, ty) in fields.iter().zip(&after.stack[start..]) {
                                let NormalizedFieldSelector::Structural(name) = field else {
                                    return Err(unsupported());
                                };
                                shape.push((name.as_str().to_owned(), *ty));
                            }
                            let ty = types.record(shape.clone())?;
                            let canonical = &types.records[ty as usize - 8].fields;
                            for field in &shape {
                                node.data.push(
                                    canonical
                                        .iter()
                                        .position(|f| f == field)
                                        .ok_or_else(unsupported)?
                                        as u8,
                                );
                            }
                            node.tag = 14;
                            node.a = u32::from(ty);
                            node.b = fields.len() as u32;
                            after.stack.truncate(start);
                            after.stack.push(ty);
                        }
                        I::Field(NormalizedFieldSelector::Structural(name)) => {
                            let ty = after.stack.pop().ok_or_else(unsupported)?;
                            let record = types
                                .records
                                .get(usize::from(ty).checked_sub(8).ok_or_else(unsupported)?)
                                .ok_or_else(unsupported)?;
                            let index = record
                                .fields
                                .iter()
                                .position(|(field, _)| field == name.as_str())
                                .ok_or_else(unsupported)?;
                            after.stack.push(record.fields[index].1);
                            node.tag = 15;
                            node.a = u32::from(ty);
                            node.b = index as u32;
                        }
                        I::LoadLocal { local, .. } | I::MoveLocal(local) => {
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
                            let mode = match instruction {
                                I::LoadLocal { use_mode, .. } => *use_mode,
                                _ => ParameterUse::Unrestricted,
                            };
                            match (ty, mode) {
                                (6 | 134, ParameterUse::Borrow) => {
                                    node.b = 1;
                                    after.stack.push(134);
                                }
                                (6, ParameterUse::Consume) => {
                                    if after.stack.contains(&134) {
                                        return Err(unsupported());
                                    }
                                    node.tag = 16;
                                    after.initialized[*local as usize] = 0;
                                    after.stack.push(6);
                                }
                                (6 | 134, _) => return Err(unsupported()),
                                (_, ParameterUse::Unrestricted) => after.stack.push(ty),
                                _ => return Err(unsupported()),
                            }
                        }
                        I::StoreLocal(local) => {
                            node.tag = 5;
                            node.a = *local;
                            let ty = after.stack.pop().ok_or_else(unsupported)?;
                            let slot = function
                                .locals
                                .get_mut(*local as usize)
                                .ok_or_else(unsupported)?;
                            if ty == 134
                                || (*slot == 6
                                    && after.initialized[*local as usize] == 1
                                    && (ty != 3 || after.stack.contains(&134)))
                            {
                                return Err(unsupported());
                            }
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
                            if after.stack.pop().ok_or_else(unsupported)? == 6
                                && after.stack.contains(&134)
                            {
                                return Err(unsupported());
                            }
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
                            type_arguments: _,
                            requirement_arguments,
                            effect_arguments,
                            read_scope_arguments,
                        }
                        | I::TailCall {
                            function: target,
                            arguments,
                            type_arguments: _,
                            requirement_arguments,
                            effect_arguments,
                            read_scope_arguments,
                        } => {
                            // Every call's closed arguments were checked while collecting
                            // the complete closure, including untaken instructions.
                            if !requirement_arguments.is_empty()
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
                            // TailCall is a derived dispatch hint. Keep its original
                            // continuation when an owned local needs lexical cleanup
                            // after a borrowed call, as the VM's fallback does.
                            node.tag = if matches!(instruction, I::TailCall { .. })
                                && !live_buffers(function, &after)
                            {
                                10
                            } else {
                                9
                            };
                            node.a = callee;
                            node.b = *arguments;
                            if node.tag == 10 {
                                if start != 0
                                    || signature.result != function.result
                                    || live_buffers(function, &after)
                                {
                                    return Err(unsupported());
                                }
                                successors.clear();
                            }
                        }
                        I::Return => {
                            node.tag = 11;
                            if after.stack != [function.result as u8]
                                || live_buffers(function, &after)
                            {
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
                            for ((current, other), ty) in state
                                .initialized
                                .iter_mut()
                                .zip(&after.initialized)
                                .zip(&function.locals)
                            {
                                if *ty == 6 && current != other {
                                    return Err(unsupported());
                                }
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
    let root_function = &functions[root as usize];
    let filter = root_function.parameters == 1
        && root_function.locals.first() == Some(&5)
        && types
            .records
            .get((root_function.result as usize).wrapping_sub(8))
            .is_some_and(|r| r.fields == [("diagnostic".to_owned(), 1), ("image".to_owned(), 5)]);
    if !filter && (root_function.parameters != 0 || root_function.result != 1) {
        return Err(unsupported());
    }
    encode_records(&functions, &types.records, root)
}

fn live_buffers(function: &Function, state: &State) -> bool {
    function
        .locals
        .iter()
        .zip(&state.initialized)
        .any(|(ty, initialized)| *ty == 6 && *initialized != 0)
}

pub(crate) fn encode(functions: &[Function], entry: u32) -> Result<Vec<u8>, Diagnostic> {
    encode_records(functions, &[], entry)
}

pub(crate) fn encode_records(
    functions: &[Function],
    records: &[Record],
    entry: u32,
) -> Result<Vec<u8>, Diagnostic> {
    let count = functions.len();
    let node_count: usize = functions.iter().map(|f| f.nodes.len()).sum();
    if count == 0
        || count > MAX_FUNCTIONS
        || node_count == 0
        || node_count > MAX_NODES
        || records.len() > 120
    {
        return Err(unsupported());
    }
    let node_offset = DECLARATIONS + count * 40;
    let blob_offset = node_offset + node_count * 32;
    let mut bytes = vec![0u8; blob_offset];
    for (i, record) in records.iter().enumerate() {
        let row = 128 + (7 + i) * 16;
        let offset = bytes.len();
        word(&mut bytes, row, 8);
        word(&mut bytes, row + 4, record.fields.len() as u32);
        word(&mut bytes, row + 8, offset as u32);
        bytes.resize(offset + record.fields.len() * 16, 0);
        for (f, (name, ty)) in record.fields.iter().enumerate() {
            let field = offset + f * 16;
            let name_offset = bytes.len();
            word(&mut bytes, field, u32::from(*ty));
            word(&mut bytes, field + 4, name_offset as u32);
            word(&mut bytes, field + 8, name.len() as u32);
            bytes.extend_from_slice(name.as_bytes());
        }
    }
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
            let payload_offset = bytes.len();
            bytes.extend_from_slice(&node.data);
            let state_offset = bytes.len();
            for (i, value) in [node.tag, node.a, node.b, node.c].into_iter().enumerate() {
                word(&mut bytes, at + i * 4, value);
            }
            if node.tag == 12 {
                word(&mut bytes, at + 4, payload_offset as u32);
            }
            if node.tag == 14 {
                word(&mut bytes, at + 12, payload_offset as u32);
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
    let fuel = 256 * blob_size + 64 * node_count + count + 4096;
    if fuel > 2_147_483_647 {
        return Err(unsupported());
    }
    bytes[..8].copy_from_slice(b"LKJCI001");
    word(&mut bytes, 8, 4);
    word(&mut bytes, 12, entry);
    let length = bytes.len() as u64;
    bytes[16..24].copy_from_slice(&length.to_le_bytes());
    bytes[24..32].copy_from_slice(&(count as u64).to_le_bytes());
    bytes[32..40].copy_from_slice(&(fuel as u64).to_le_bytes());
    for (i, value) in [
        112,
        1,
        128,
        7 + records.len() as u32,
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
    ]
    .into_iter()
    .enumerate()
    {
        word(&mut bytes, HEADER + i * 4, value);
    }
    for tag in 1..=7 {
        word(&mut bytes, 128 + (tag - 1) * 16, tag as u32);
    }
    word(&mut bytes, 128 + 6 * 16 + 4, 1);
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
/// not a second wire reader; only profile 4 is emitted.
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
