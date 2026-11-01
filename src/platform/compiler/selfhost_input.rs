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
pub(crate) const DESCRIPTORS: usize = 160;
pub(crate) const DESCRIPTOR_BYTES: usize = 32;
pub(crate) const FUNCTION_BYTES: usize = 80;
pub(crate) const NODE_BYTES: usize = 64;
pub(crate) const VALUE_TYPE_BYTES: usize = 16;

#[track_caller]
fn unsupported() -> Diagnostic {
    Diagnostic::new(
        DiagnosticClass::Source,
        "compiler_input_unsupported",
        format!(
            "LKJCI1 profile 6 requires a closed pure byte/aggregate function closure with forward branches (exporter precondition at {})",
            std::panic::Location::caller()
        ),
    )
}

/// Type identity and custody are independent wire fields. In particular, type
/// identity 134 is an ordinary descriptor and never denotes a borrowed buffer.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct ValueType {
    pub identity: u64,
    pub borrowed: bool,
}
impl ValueType {
    pub(crate) const fn owned(identity: u64) -> Self {
        Self {
            identity,
            borrowed: false,
        }
    }
    pub(crate) const fn borrowed_buffer() -> Self {
        Self {
            identity: 6,
            borrowed: true,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub(crate) struct Node {
    pub tag: u64,
    pub a: u64,
    pub b: u64,
    pub c: u64,
    pub immediate: i64,
    pub initialized: Vec<u8>,
    pub stack: Vec<ValueType>,
    pub data: Vec<u8>,
}
#[derive(Clone, Debug, Default)]
pub(crate) struct Function {
    pub result: u64,
    pub parameters: u64,
    pub locals: Vec<ValueType>,
    pub external: u64,
    pub nodes: Vec<Node>,
}
#[derive(Clone, Debug)]
struct State {
    initialized: Vec<u8>,
    stack: Vec<ValueType>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Record {
    pub fields: Vec<(String, u64)>,
}

#[derive(Default)]
struct Types {
    records: Vec<Record>,
    known: BTreeMap<TypeObjectDigest, u64>,
}

impl Types {
    fn record(&mut self, mut fields: Vec<(String, u64)>) -> Result<u64, Diagnostic> {
        fields.sort();
        if fields.iter().any(|(name, ty)| name.is_empty() || *ty == 6)
            || fields.windows(2).any(|w| w[0].0 == w[1].0)
        {
            return Err(unsupported());
        }
        let record = Record { fields };
        if let Some(index) = self.records.iter().position(|r| r == &record) {
            return input_word(index)?.checked_add(8).ok_or_else(unsupported);
        }
        let ty = input_word(self.records.len())?
            .checked_add(8)
            .ok_or_else(unsupported)?;
        self.records.try_reserve(1).map_err(|_| allocation())?;
        self.records.push(record);
        Ok(ty)
    }

    // Explicit enter/exit frames make the complete descriptor walk stack safe.
    // The active set is a semantic cycle check, independent of traversal depth.
    fn code(
        &mut self,
        program: &NormalizedProgram,
        root: TypeObjectDigest,
    ) -> Result<u64, Diagnostic> {
        let mut pending = vec![(root, false)];
        let mut active = BTreeSet::new();
        while let Some((ty, exit)) = pending.pop() {
            if self.known.contains_key(&ty) {
                continue;
            }
            let form = &program.types.get(&ty).ok_or_else(unsupported)?.form;
            if !exit {
                if !active.insert(ty) {
                    return Err(unsupported());
                }
                pending.try_reserve(1).map_err(|_| allocation())?;
                pending.push((ty, true));
                let children: Vec<_> = match form {
                    TypeForm::List { item } => vec![*item],
                    TypeForm::StructuralRecord { fields } => fields.iter().map(|f| f.ty).collect(),
                    TypeForm::I64
                    | TypeForm::Bool
                    | TypeForm::Unit
                    | TypeForm::Text
                    | TypeForm::Bytes
                    | TypeForm::ByteBuffer => vec![],
                    _ => return Err(unsupported()),
                };
                pending
                    .try_reserve(children.len())
                    .map_err(|_| allocation())?;
                for child in children.into_iter().rev() {
                    pending.push((child, false));
                }
                continue;
            }
            let code = match form {
                TypeForm::I64 => 1,
                TypeForm::Bool => 2,
                TypeForm::Unit => 3,
                TypeForm::Text => 4,
                TypeForm::Bytes => 5,
                TypeForm::ByteBuffer => 6,
                TypeForm::List { item } if self.known.get(item) == Some(&1) => 7,
                TypeForm::StructuralRecord { fields } => {
                    let mut mapped = Vec::new();
                    mapped
                        .try_reserve_exact(fields.len())
                        .map_err(|_| allocation())?;
                    for field in fields {
                        mapped.push((
                            field.name.as_str().to_owned(),
                            *self.known.get(&field.ty).ok_or_else(unsupported)?,
                        ));
                    }
                    self.record(mapped)?
                }
                _ => return Err(unsupported()),
            };
            active.remove(&ty);
            self.known.insert(ty, code);
        }
        self.known.get(&root).copied().ok_or_else(unsupported)
    }

    fn parameter(
        &mut self,
        program: &NormalizedProgram,
        ty: TypeObjectDigest,
        mode: ParameterUse,
    ) -> Result<ValueType, Diagnostic> {
        let code = self.code(program, ty)?;
        match (code, mode) {
            (6, ParameterUse::Consume) => Ok(ValueType::owned(6)),
            (6, ParameterUse::Borrow) => Ok(ValueType::borrowed_buffer()),
            (6, ParameterUse::Unrestricted) => Err(unsupported()),
            (_, ParameterUse::Unrestricted) => Ok(ValueType::owned(code)),
            _ => Err(unsupported()),
        }
    }
}
fn primitive(name: &str) -> Result<u64, Diagnostic> {
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
        "core.buffer.append-slice" => 29,
        "core.buffer.append-i64-le" => 30,
        "core.bytes.read-i64-le" => 31,
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
        .map(|(i, &f)| Ok((f, input_word(i)?)))
        .collect::<Result<_, Diagnostic>>()?;
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
            result: types.code(program, resolve(function.result.ty)?)?,
            parameters: u64::from(function.parameter_count),
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
                if u64::from(code.parameter_count) != function.parameters
                    || code.local_count < code.parameter_count
                    || code.instructions.is_empty()
                {
                    return Err(unsupported());
                }
                function
                    .locals
                    .try_reserve(code.local_count as usize - function.locals.len())
                    .map_err(|_| allocation())?;
                function
                    .locals
                    .resize(code.local_count as usize, ValueType::owned(0));
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
                            after.stack.push(ValueType::owned(1));
                        }
                        I::Bool(value) => {
                            node.tag = 2;
                            node.immediate = i64::from(*value);
                            after.stack.push(ValueType::owned(2));
                        }
                        I::Unit => {
                            node.tag = 3;
                            after.stack.push(ValueType::owned(3));
                        }
                        I::Text(value) | I::StaticText(value) => {
                            node.tag = 12;
                            node.b = input_word(value.len())?;
                            node.data = value.as_bytes().to_vec();
                            after.stack.push(ValueType::owned(4));
                        }
                        I::List { items } => {
                            let start = after
                                .stack
                                .len()
                                .checked_sub(*items as usize)
                                .ok_or_else(unsupported)?;
                            if after.stack[start..]
                                .iter()
                                .any(|ty| *ty != ValueType::owned(1))
                            {
                                return Err(unsupported());
                            }
                            after.stack.truncate(start);
                            after.stack.push(ValueType::owned(7));
                            node.tag = 13;
                            node.a = 7;
                            node.b = u64::from(*items);
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
                                if ty.borrowed {
                                    return Err(unsupported());
                                }
                                shape.push((name.as_str().to_owned(), ty.identity));
                            }
                            let ty = types.record(shape.clone())?;
                            let canonical = &types.records[ty as usize - 8].fields;
                            for field in &shape {
                                let member = input_word(
                                    canonical
                                        .iter()
                                        .position(|f| f == field)
                                        .ok_or_else(unsupported)?,
                                )?;
                                node.data.extend_from_slice(&member.to_le_bytes());
                            }
                            node.tag = 14;
                            node.a = ty;
                            node.b = input_word(fields.len())?;
                            after.stack.truncate(start);
                            after.stack.push(ValueType::owned(ty));
                        }
                        I::Field(NormalizedFieldSelector::Structural(name)) => {
                            let ty = after.stack.pop().ok_or_else(unsupported)?;
                            if ty.borrowed {
                                return Err(unsupported());
                            }
                            let ty = ty.identity;
                            let record = types
                                .records
                                .get(
                                    usize::try_from(ty)
                                        .map_err(|_| unsupported())?
                                        .checked_sub(8)
                                        .ok_or_else(unsupported)?,
                                )
                                .ok_or_else(unsupported)?;
                            let index = record
                                .fields
                                .iter()
                                .position(|(field, _)| field == name.as_str())
                                .ok_or_else(unsupported)?;
                            after.stack.push(ValueType::owned(record.fields[index].1));
                            node.tag = 15;
                            node.a = ty;
                            node.b = input_word(index)?;
                        }
                        I::LoadLocal { local, .. } | I::MoveLocal(local) => {
                            node.tag = 4;
                            node.a = u64::from(*local);
                            if after.initialized.get(*local as usize) != Some(&1) {
                                return Err(unsupported());
                            }
                            let ty = *function
                                .locals
                                .get(*local as usize)
                                .filter(|&&ty| ty.identity != 0)
                                .ok_or_else(unsupported)?;
                            let mode = match instruction {
                                I::LoadLocal { use_mode, .. } => *use_mode,
                                _ => ParameterUse::Unrestricted,
                            };
                            match (ty.identity, ty.borrowed, mode) {
                                (6, _, ParameterUse::Borrow) => {
                                    node.b = 1;
                                    after.stack.push(ValueType::borrowed_buffer());
                                }
                                (6, false, ParameterUse::Consume) => {
                                    if after.stack.iter().any(|ty| ty.borrowed) {
                                        return Err(unsupported());
                                    }
                                    node.tag = 16;
                                    after.initialized[*local as usize] = 0;
                                    after.stack.push(ValueType::owned(6));
                                }
                                (6, _, _) => return Err(unsupported()),
                                (_, false, ParameterUse::Unrestricted) => after.stack.push(ty),
                                _ => return Err(unsupported()),
                            }
                        }
                        I::StoreLocal(local) => {
                            node.tag = 5;
                            node.a = u64::from(*local);
                            let ty = after.stack.pop().ok_or_else(unsupported)?;
                            let slot = function
                                .locals
                                .get_mut(*local as usize)
                                .ok_or_else(unsupported)?;
                            if ty.borrowed
                                || (*slot == ValueType::owned(6)
                                    && after.initialized[*local as usize] == 1
                                    && (ty != ValueType::owned(3)
                                        || after.stack.iter().any(|ty| ty.borrowed)))
                            {
                                return Err(unsupported());
                            }
                            if ty == ValueType::owned(3)
                                && slot.identity != 0
                                && *slot != ValueType::owned(3)
                            {
                                after.initialized[*local as usize] = 0;
                            } else {
                                if slot.identity != 0 && *slot != ty {
                                    return Err(unsupported());
                                }
                                *slot = ty;
                                after.initialized[*local as usize] = 1;
                            }
                        }
                        I::Drop => {
                            node.tag = 6;
                            if after.stack.pop().ok_or_else(unsupported)? == ValueType::owned(6)
                                && after.stack.iter().any(|ty| ty.borrowed)
                            {
                                return Err(unsupported());
                            }
                        }
                        I::Jump(target) | I::JumpIfFalse(target) => {
                            node.a = u64::from(*target);
                            if *target as usize <= ip {
                                return Err(unsupported());
                            }
                            if matches!(instruction, I::JumpIfFalse(_)) {
                                node.tag = 8;
                                if after.stack.pop() != Some(ValueType::owned(2)) {
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
                            if u64::from(*arguments) != signature.parameters
                                || after.stack.len() < *arguments as usize
                            {
                                return Err(unsupported());
                            }
                            let start = after.stack.len() - *arguments as usize;
                            if after.stack[start..] != signature.locals {
                                return Err(unsupported());
                            }
                            after.stack.truncate(start);
                            after.stack.push(ValueType::owned(signature.result));
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
                            node.b = u64::from(*arguments);
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
                            if after.stack != [ValueType::owned(function.result)]
                                || live_buffers(function, &after)
                            {
                                return Err(unsupported());
                            }
                            successors.clear();
                        }
                        _ => return Err(unsupported()),
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
                                if *ty == ValueType::owned(6) && current != other {
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
                    if ty.identity == 0 {
                        *ty = ValueType::owned(1);
                    }
                }
            }
        }
    }
    let root = *indices.get(&entry).ok_or_else(unsupported)?;
    let root_function = &functions[root as usize];
    let filter = root_function.parameters == 1
        && root_function.locals.first() == Some(&ValueType::owned(5))
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
        .any(|(ty, initialized)| *ty == ValueType::owned(6) && *initialized != 0)
}

pub(crate) fn encode(functions: &[Function], entry: u64) -> Result<Vec<u8>, Diagnostic> {
    encode_records(functions, &[], entry)
}

fn allocation() -> Diagnostic {
    Diagnostic::new(
        DiagnosticClass::Source,
        "compiler_input_allocation",
        "allocation of the complete normalized compiler input failed",
    )
}

fn checked_input_end(offset: usize, additional: usize) -> Result<usize, Diagnostic> {
    offset
        .checked_add(additional)
        .filter(|end| i64::try_from(*end).is_ok())
        .ok_or_else(unsupported)
}

fn input_word(value: usize) -> Result<u64, Diagnostic> {
    u64::try_from(value).map_err(|_| unsupported())
}

pub(crate) fn encode_records(
    functions: &[Function],
    records: &[Record],
    entry: u64,
) -> Result<Vec<u8>, Diagnostic> {
    let count = functions.len();
    if count == 0 {
        return Err(unsupported());
    }
    let node_count = functions.iter().try_fold(0usize, |total, function| {
        total
            .checked_add(function.nodes.len())
            .ok_or_else(unsupported)
    })?;
    if node_count == 0 {
        return Err(unsupported());
    }
    let types = records.len().checked_add(7).ok_or_else(unsupported)?;
    let declarations = checked_input_end(
        DESCRIPTORS,
        types
            .checked_mul(DESCRIPTOR_BYTES)
            .ok_or_else(unsupported)?,
    )?;
    let node_offset = checked_input_end(
        declarations,
        count.checked_mul(FUNCTION_BYTES).ok_or_else(unsupported)?,
    )?;
    let blob_offset = checked_input_end(
        node_offset,
        node_count.checked_mul(NODE_BYTES).ok_or_else(unsupported)?,
    )?;

    // Check the complete physical extent before allocation. These are host
    // representation boundaries, never configurable quotas or default policies.
    let mut length = blob_offset;
    for record in records {
        length = checked_input_end(
            length,
            record
                .fields
                .len()
                .checked_mul(32)
                .ok_or_else(unsupported)?,
        )?;
        for (name, _) in &record.fields {
            length = checked_input_end(length, name.len())?;
        }
    }
    for function in functions {
        length = checked_input_end(
            length,
            function
                .locals
                .len()
                .checked_mul(VALUE_TYPE_BYTES)
                .ok_or_else(unsupported)?,
        )?;
        for node in &function.nodes {
            if node.initialized.len() != function.locals.len() {
                return Err(unsupported());
            }
            length = checked_input_end(length, node.data.len())?;
            length = checked_input_end(length, node.initialized.len())?;
            length = checked_input_end(
                length,
                node.stack
                    .len()
                    .checked_mul(VALUE_TYPE_BYTES)
                    .ok_or_else(unsupported)?,
            )?;
        }
    }
    let blob_size = length.checked_sub(blob_offset).ok_or_else(unsupported)?;
    let mut bytes = Vec::new();
    bytes.try_reserve_exact(length).map_err(|_| allocation())?;
    bytes.resize(blob_offset, 0);
    for (i, record) in records.iter().enumerate() {
        let row = DESCRIPTORS + (7 + i) * DESCRIPTOR_BYTES;
        let offset = bytes.len();
        word(&mut bytes, row, 8);
        word(&mut bytes, row + 8, input_word(record.fields.len())?);
        word(&mut bytes, row + 16, input_word(offset)?);
        bytes.resize(offset + record.fields.len() * 32, 0);
        for (f, (name, ty)) in record.fields.iter().enumerate() {
            let field = offset + f * 32;
            let name_offset = bytes.len();
            word(&mut bytes, field, *ty);
            word(&mut bytes, field + 8, input_word(name_offset)?);
            word(&mut bytes, field + 16, input_word(name.len())?);
            bytes.extend_from_slice(name.as_bytes());
        }
    }
    let mut node_index = 0;
    for (index, function) in functions.iter().enumerate() {
        let at = declarations + index * FUNCTION_BYTES;
        let local_offset = bytes.len();
        for (i, value) in [
            0,
            function.result,
            function.parameters,
            input_word(function.locals.len())?,
            input_word(node_index)?,
            input_word(function.nodes.len())?,
            function.external,
            input_word(local_offset)?,
            0,
            0,
        ]
        .into_iter()
        .enumerate()
        {
            word(&mut bytes, at + i * 8, value);
        }
        value_types(&mut bytes, &function.locals);
        for node in &function.nodes {
            let at = node_offset + node_index * NODE_BYTES;
            let payload_offset = bytes.len();
            bytes.extend_from_slice(&node.data);
            let state_offset = bytes.len();
            for (i, value) in [node.tag, node.a, node.b, node.c].into_iter().enumerate() {
                word(&mut bytes, at + i * 8, value);
            }
            if node.tag == 12 {
                word(&mut bytes, at + 8, input_word(payload_offset)?);
            }
            if node.tag == 14 {
                word(&mut bytes, at + 24, input_word(payload_offset)?);
            }
            bytes[at + 32..at + 40].copy_from_slice(&node.immediate.to_le_bytes());
            word(&mut bytes, at + 48, input_word(state_offset)?);
            word(&mut bytes, at + 56, input_word(node.stack.len())?);
            bytes.extend_from_slice(&node.initialized);
            value_types(&mut bytes, &node.stack);
            node_index += 1;
        }
    }
    if bytes.len() != length {
        return Err(unsupported());
    }
    bytes[..8].copy_from_slice(b"LKJCI001");
    bytes[8..12].copy_from_slice(&6u32.to_le_bytes());
    // @12 tag 0 + @32 zero allowance means explicitly unbounded.
    word(&mut bytes, 16, input_word(length)?);
    word(&mut bytes, 24, input_word(count)?);
    for (at, value) in [
        (72, input_word(DESCRIPTORS)?),
        (80, input_word(types)?),
        (88, input_word(declarations)?),
        (96, input_word(count)?),
        (104, input_word(node_offset)?),
        (120, input_word(node_count)?),
        (128, input_word(blob_offset)?),
        (136, input_word(blob_size)?),
        (144, entry),
    ] {
        word(&mut bytes, at, value);
    }
    for tag in 1..=7 {
        word(
            &mut bytes,
            DESCRIPTORS + (tag - 1) * DESCRIPTOR_BYTES,
            tag as u64,
        );
    }
    word(&mut bytes, DESCRIPTORS + 6 * DESCRIPTOR_BYTES + 8, 1);
    rechecksum(&mut bytes);
    Ok(bytes)
}

pub(crate) fn set_allowance(bytes: &mut [u8], allowance: Option<u64>) {
    bytes[12..16].copy_from_slice(&u32::from(allowance.is_some()).to_le_bytes());
    word(bytes, 32, allowance.unwrap_or(0));
    rechecksum(bytes);
}

fn value_types(bytes: &mut Vec<u8>, types: &[ValueType]) {
    for ty in types {
        bytes.extend_from_slice(&ty.identity.to_le_bytes());
        bytes.extend_from_slice(&u64::from(ty.borrowed).to_le_bytes());
    }
}
fn word(bytes: &mut [u8], at: usize, value: u64) {
    bytes[at..at + 8].copy_from_slice(&value.to_le_bytes());
}

pub(crate) fn rechecksum(bytes: &mut [u8]) {
    let mut hasher = blake3::Hasher::new();
    hasher.update(&bytes[..40]);
    hasher.update(&bytes[HEADER..]);
    bytes[40..HEADER].copy_from_slice(hasher.finalize().as_bytes());
}

/// Small independent test builder. Arithmetic operation IDs are test notation,
/// not a second wire reader; only profile 6 is emitted.
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
                stack.push(ValueType::owned(1));
            }
            2..=5 => {
                node.tag = 9;
                node.a = u64::from(op - 1);
                node.b = 2;
                stack.truncate(stack.len().saturating_sub(2));
                stack.push(ValueType::owned(1));
            }
            6 => node.tag = 11,
            _ => node.tag = u64::from(op),
        }
        root.nodes.push(node);
    }
    let mut functions = vec![root];
    for external in 1..=4 {
        functions.push(Function {
            result: 1,
            parameters: 2,
            locals: vec![ValueType::owned(1), ValueType::owned(1)],
            external,
            nodes: vec![],
        });
    }
    encode(&functions, 0)
}
