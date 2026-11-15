//! Checked source runtime over real addresses and a sealed kernel-memory interface.
//! The seal belongs to this explicit input; ordinary application names confer no authority.
use crate::diagnostic::{Diagnostic, Span};
use crate::ir::{self, CheckedProgram, Op, TypeKind};
use std::collections::BTreeMap;

/// Free-list heads, followed by the region cursor, remaining bytes and growth hint.
pub const ALLOCATOR_STATE_BYTES: u64 = (64 + 3) * 8;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Primitive {
    Load8,
    Load64,
    Store8,
    Store64,
    MemoryCopy,
    Map,
    Unmap,
    Remap,
    AllocatorState,
    Fail,
}
impl Primitive {
    pub fn parameters(self) -> usize {
        match self {
            Self::Load8 | Self::Load64 | Self::Map => 1,
            Self::Store8 | Self::Store64 | Self::Unmap => 2,
            Self::MemoryCopy | Self::Remap => 3,
            Self::AllocatorState | Self::Fail => 0,
        }
    }
    fn declarations() -> Vec<(String, Self)> {
        vec![
            ("raw_load8".to_string(), Self::Load8),
            ("raw_load64".to_string(), Self::Load64),
            ("raw_store8".to_string(), Self::Store8),
            ("raw_store64".to_string(), Self::Store64),
            ("raw_memory_copy".to_string(), Self::MemoryCopy),
            ("raw_map".to_string(), Self::Map),
            ("raw_unmap".to_string(), Self::Unmap),
            ("raw_remap".to_string(), Self::Remap),
            ("raw_allocator_state".to_string(), Self::AllocatorState),
            ("raw_fail".to_string(), Self::Fail),
        ]
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Entry {
    Allocate,
    Release,
    Reallocate,
    Utf8Validate,
}
impl Entry {
    pub fn parameters(self) -> usize {
        match self {
            Self::Allocate | Self::Release => 1,
            Self::Reallocate | Self::Utf8Validate => 2,
        }
    }
    pub fn slot(self) -> usize {
        match self {
            Self::Allocate => 0,
            Self::Release => 1,
            Self::Reallocate => 2,
            Self::Utf8Validate => 3,
        }
    }
    fn declarations() -> Vec<(String, Self)> {
        vec![
            ("allocate".to_string(), Self::Allocate),
            ("release".to_string(), Self::Release),
            ("reallocate".to_string(), Self::Reallocate),
            ("utf8_validate".to_string(), Self::Utf8Validate),
        ]
    }
}

pub struct Runtime {
    checked: CheckedProgram,
    primitives: BTreeMap<usize, Primitive>,
    entries: Vec<usize>,
}
fn problem(message: &str) -> Diagnostic {
    Diagnostic::new(Span::default(), "E_RUNTIME", message)
}
fn word(program: &ir::Program, ty: usize) -> bool {
    matches!(
        program.types[ty].kind,
        TypeKind::Int {
            bits: 64,
            signed: false
        }
    )
}
fn signature(program: &ir::Program, index: usize, parameters: usize) -> bool {
    let function = &program.functions[index];
    if function.params != parameters || !word(program, function.result) {
        return false;
    }
    for index in 0usize..function.params {
        if !word(program, function.locals[index].ty) {
            return false;
        }
    }
    true
}

/// Validate the raw interface after ordinary IR checking. No lowering is performed.
pub fn seal(checked: CheckedProgram) -> Result<Runtime, Diagnostic> {
    let program = checked.program();
    let mut names: BTreeMap<String, usize> = BTreeMap::new();
    for index in 0usize..program.functions.len() {
        names.insert(program.functions[index].name.clone(), index);
    }
    let mut primitives = BTreeMap::new();
    for (name, primitive) in Primitive::declarations() {
        let index = *names
            .get(&name)
            .ok_or_else(|| problem(&format!("missing runtime primitive {name}")))?;
        if !signature(program, index, primitive.parameters()) {
            return Err(problem(&format!(
                "runtime primitive {name} has the wrong signature"
            )));
        }
        if !matches!(
            program.functions[index]
                .body
                .first()
                .map(|instruction| &instruction.op),
            Some(Op::Trap { .. })
        ) {
            return Err(problem(&format!(
                "runtime primitive {name} must be an ordinary trapping stub"
            )));
        }
        primitives.insert(index, primitive);
    }
    let mut entries = vec![0; 4];
    for (name, entry) in Entry::declarations() {
        let index = *names
            .get(&name)
            .ok_or_else(|| problem(&format!("missing runtime entry {name}")))?;
        if !signature(program, index, entry.parameters()) {
            return Err(problem(&format!(
                "runtime entry {name} has the wrong signature"
            )));
        }
        entries[entry.slot()] = index;
    }
    for (index, function) in program.functions.iter().enumerate() {
        for local in &function.locals {
            if !matches!(
                program.types[local.ty].kind,
                TypeKind::Unit | TypeKind::Bool | TypeKind::Int { .. }
            ) {
                return Err(problem(
                    "the raw runtime must use scalar locals; recursive runtime allocation is forbidden",
                ));
            }
        }
        if primitives.contains_key(&index) {
            continue;
        }
        for instruction in &function.body {
            let scalar = match &instruction.op {
                Op::Int { .. }
                | Op::Unit { .. }
                | Op::Unary { .. }
                | Op::Binary { .. }
                | Op::Call { .. }
                | Op::If { .. }
                | Op::Else
                | Op::End
                | Op::Loop
                | Op::Break
                | Op::Continue
                | Op::Return { .. }
                | Op::BreakTo { .. }
                | Op::ContinueTo { .. } => true,
                Op::Intrinsic { name, .. } => *name == ir::IntrinsicOp::Cast,
                Op::Read { place, .. }
                | Op::Assign { place, .. }
                | Op::Drop { place }
                | Op::DropIf { place } => place.projections.is_empty(),
                _ => false,
            };
            if !scalar {
                return Err(problem(&format!(
                    "runtime function {} must use scalar operations and the sealed raw interface",
                    function.name
                )));
            }
        }
    }
    Ok(Runtime {
        checked,
        primitives,
        entries,
    })
}
impl Runtime {
    pub fn checked_program(&self) -> &ir::Program {
        self.checked.program()
    }
    pub fn checked(&self) -> &CheckedProgram {
        &self.checked
    }
    pub fn primitive(&self, index: usize) -> Option<Primitive> {
        self.primitives.get(&index).copied()
    }
    pub fn entry(&self, role: Entry) -> usize {
        self.entries[role.slot()]
    }
    pub fn entries(&self) -> Vec<usize> {
        self.entries.clone()
    }
    pub fn function(&self, name: &str) -> Option<usize> {
        self.checked_program()
            .functions
            .iter()
            .position(|function| function.name == name)
    }
}
