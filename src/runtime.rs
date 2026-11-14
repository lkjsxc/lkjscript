//! The host supplies a declared source runtime separately from user input.
//! Preparing that bundle is the sole authority to replace raw primitive stubs.
//! Ordinary source names, imports and IR text cannot request this substitution.
use crate::diagnostic::{Diagnostic, Span};
use crate::ir::{self, CheckedProgram, Op, TypeKind};
use crate::lower::{self, Code, Function, Instruction, Signature};
use std::collections::BTreeMap;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Primitive {
    Load8,
    Load64,
    Store8,
    Store64,
    MemorySize,
    MemoryGrow,
    MemoryCopy,
    HeapTip,
    SetHeapTip,
    Classes,
    SetClasses,
    Fail,
    DropStep,
}
impl Primitive {
    fn declarations() -> Vec<(String, usize, Self)> {
        vec![
            ("raw_load8".to_string(), 1, Self::Load8),
            ("raw_load64".to_string(), 1, Self::Load64),
            ("raw_store8".to_string(), 2, Self::Store8),
            ("raw_store64".to_string(), 2, Self::Store64),
            ("raw_memory_size".to_string(), 0, Self::MemorySize),
            ("raw_memory_grow".to_string(), 1, Self::MemoryGrow),
            ("raw_memory_copy".to_string(), 3, Self::MemoryCopy),
            ("raw_heap_tip".to_string(), 0, Self::HeapTip),
            ("raw_set_heap_tip".to_string(), 1, Self::SetHeapTip),
            ("raw_classes".to_string(), 0, Self::Classes),
            ("raw_set_classes".to_string(), 1, Self::SetClasses),
            ("raw_fail".to_string(), 0, Self::Fail),
            ("raw_drop_step".to_string(), 2, Self::DropStep),
        ]
    }
    fn code(self) -> Code {
        use Instruction::*;
        let mut instructions = match self {
            Self::Load8 => vec![
                LocalGet(0),
                Load {
                    bytes: 1,
                    signed: false,
                    offset: 0,
                },
            ],
            Self::Load64 => vec![
                LocalGet(0),
                Load {
                    bytes: 8,
                    signed: false,
                    offset: 0,
                },
            ],
            Self::Store8 => vec![
                LocalGet(0),
                LocalGet(1),
                Store {
                    bytes: 1,
                    offset: 0,
                },
                I64(0),
            ],
            Self::Store64 => vec![
                LocalGet(0),
                LocalGet(1),
                Store {
                    bytes: 8,
                    offset: 0,
                },
                I64(0),
            ],
            Self::MemorySize => vec![MemorySize],
            Self::MemoryGrow => vec![LocalGet(0), MemoryGrow],
            Self::MemoryCopy => vec![LocalGet(0), LocalGet(1), LocalGet(2), MemoryCopy, I64(0)],
            Self::HeapTip => vec![GlobalGet(0)],
            Self::SetHeapTip => vec![LocalGet(0), GlobalSet(0), I64(0)],
            Self::Classes => vec![GlobalGet(1)],
            Self::SetClasses => vec![LocalGet(0), GlobalSet(1), I64(0)],
            Self::Fail => vec![I64(6), GlobalSet(2), Unreachable],
            Self::DropStep => vec![LocalGet(0), LocalGet(1), CallIndirect(Signature::words(1))],
        };
        instructions.push(End);
        Code {
            locals: Vec::new(),
            instructions,
        }
    }
}

fn entry_declaration(slot: usize) -> (lower::RuntimeFunction, String) {
    match slot {
        0 => (lower::RuntimeFunction::Allocate, "allocate".to_string()),
        1 => (lower::RuntimeFunction::Release, "release".to_string()),
        2 => (
            lower::RuntimeFunction::DropDispatch,
            "drop_walk".to_string(),
        ),
        3 => (
            lower::RuntimeFunction::Utf8Validate,
            "utf8_validate".to_string(),
        ),
        _ => (lower::RuntimeFunction::Reallocate, "reallocate".to_string()),
    }
}

/// The fields are private: a checked user program is not a source runtime.
pub struct Runtime {
    checked: CheckedProgram,
    primitives: BTreeMap<usize, Primitive>,
    entries: Vec<usize>,
    functions: Vec<Function>,
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

/// Runtime source is checked normally, then its narrow raw interface is sealed.
/// No filename, attribute or declaration spelling alone grants user authority.
pub fn seal(checked: CheckedProgram) -> Result<Runtime, Diagnostic> {
    let program = checked.program();
    let mut names: BTreeMap<String, usize> = BTreeMap::new();
    for index in 0usize..program.functions.len() {
        names.insert(program.functions[index].name.clone(), index);
    }
    let mut primitives = BTreeMap::new();
    for (name, parameters, primitive) in Primitive::declarations() {
        let index = *names
            .get(&name)
            .ok_or_else(|| problem(&format!("missing runtime primitive {name}")))?;
        let function = &program.functions[index];
        if function.params != parameters
            || !word(program, function.result)
            || function.locals[..function.params]
                .iter()
                .any(|local| !word(program, local.ty))
        {
            return Err(problem(&format!(
                "runtime primitive {name} has the wrong signature"
            )));
        }
        if !matches!(
            function.body.first().map(|instruction| &instruction.op),
            Some(Op::Trap { .. })
        ) {
            return Err(problem(&format!(
                "runtime primitive {name} must be an ordinary trapping stub"
            )));
        }
        primitives.insert(index, primitive);
    }
    let mut entries = vec![0; lower::RUNTIME_ENTRIES];
    for slot in 0usize..lower::RUNTIME_ENTRIES {
        let (role, name) = entry_declaration(slot);
        let index = *names
            .get(&name)
            .ok_or_else(|| problem(&format!("missing runtime entry {name}")))?;
        let function = &program.functions[index];
        if function.params != role.signature().parameters.len()
            || !word(program, function.result)
            || function.locals[..function.params]
                .iter()
                .any(|local| !word(program, local.ty))
        {
            return Err(problem(&format!(
                "runtime entry {name} has the wrong signature"
            )));
        }
        entries[slot] = index;
    }
    for (index, function) in program.functions.iter().enumerate() {
        if function.locals.iter().any(|local| {
            !matches!(
                program.types[local.ty].kind,
                TypeKind::Unit | TypeKind::Bool | TypeKind::Int { .. }
            )
        }) {
            return Err(problem(
                "the raw runtime must use scalar locals; recursive runtime allocation is forbidden",
            ));
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
                Op::Intrinsic { name, .. } => name == "cast",
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
    let mut functions =
        lower::lower_runtime_functions(&checked).map_err(|message| problem(&message))?;
    for (index, primitive) in &primitives {
        functions[*index].code = primitive.code();
    }
    Ok(Runtime {
        checked,
        primitives,
        entries,
        functions,
    })
}

impl Runtime {
    /// The declared runtime IR, suitable for an independent backend input.
    pub fn checked_program(&self) -> &ir::Program {
        self.checked.program()
    }
    pub(crate) fn checked(&self) -> &CheckedProgram {
        &self.checked
    }
    pub(crate) fn primitive(&self, index: usize) -> Option<Primitive> {
        self.primitives.get(&index).copied()
    }
    pub fn function(&self, name: &str) -> Option<usize> {
        self.checked
            .program()
            .functions
            .iter()
            .position(|function| function.name == name)
    }
    /// An isolated executable for the runtime model. Its explicit raw arena
    /// has no application owners or host capabilities. Test hosts may initialize
    /// the bytes and compare this source on IR and every production encoder.
    pub fn model_program(&self, drop_steps: &[usize]) -> Result<lower::Program, String> {
        let mut functions = Vec::new();
        for slot in 0usize..lower::RUNTIME_ENTRIES {
            let (role, _) = entry_declaration(slot);
            functions.push(Function {
                name: "runtime entry".to_string(),
                signature: role.signature(),
                role: lower::FunctionRole::Runtime(role),
                code: Code {
                    locals: Vec::new(),
                    instructions: vec![Instruction::Unreachable, Instruction::End],
                },
            });
        }
        let mut table = Vec::new();
        for index in drop_steps {
            let function = self
                .checked
                .program()
                .functions
                .get(*index)
                .ok_or("invalid runtime model cleanup target")?;
            if function.params != 1
                || !word(self.checked.program(), function.result)
                || !word(self.checked.program(), function.locals[0].ty)
            {
                return Err(
                    "runtime model cleanup target must take and return one word".to_string()
                );
            }
            table.push(lower::RUNTIME_ENTRIES + index);
        }
        let mut exports = Vec::new();
        for index in 0usize..self.functions.len() {
            if !self.primitives.contains_key(&index) {
                exports.push(lower::Export {
                    name: format!("fn:{}", self.functions[index].name),
                    function: lower::RUNTIME_ENTRIES + index,
                });
            }
        }
        let mut program = lower::Program {
            layout: lower::Layouts { types: Vec::new() },
            imports: Vec::new(),
            functions,
            globals: vec![
                lower::Global {
                    initial: 8,
                    mutable: true,
                },
                lower::Global {
                    initial: 0,
                    mutable: true,
                },
                lower::Global {
                    initial: 0,
                    mutable: true,
                },
                lower::Global {
                    initial: 0,
                    mutable: true,
                },
            ],
            exports,
            table,
            data: Vec::new(),
            data_offset: 8,
            minimum_memory_pages: 1,
            trap_code_global: 2,
            trap_line_global: 3,
            trap_messages: Vec::new(),
        };
        self.link(&mut program)?;
        Ok(program)
    }
    /// Link source functions after the application's explicit glue. Existing
    /// runtime calls retain their declared entry slots and use trampolines.
    pub(crate) fn link(&self, program: &mut lower::Program) -> Result<(), String> {
        let base = program
            .imports
            .len()
            .checked_add(program.functions.len())
            .ok_or("runtime function index overflow")?;
        let end = lower::RUNTIME_ENTRIES
            .checked_add(self.functions.len())
            .ok_or("runtime function index overflow")?;
        for function in &self.functions {
            let mut code = function.code.clone();
            for position in 0usize..code.instructions.len() {
                let instruction = &mut code.instructions[position];
                if let Instruction::Call(index) = instruction {
                    if *index < lower::RUNTIME_ENTRIES || *index >= end {
                        return Err(
                            "source runtime has a call outside its checked scalar closure"
                                .to_string(),
                        );
                    }
                    *index = base
                        .checked_add(*index - lower::RUNTIME_ENTRIES)
                        .ok_or("runtime function index overflow")?;
                }
            }
            program.functions.push(Function {
                name: format!("runtime::{}", function.name),
                signature: function.signature.clone(),
                role: lower::FunctionRole::RuntimeSource,
                code,
            });
        }
        for slot in 0usize..self.entries.len() {
            let index = self.entries[slot];
            let (role, name) = entry_declaration(slot);
            let signature = role.signature();
            let entry = program
                .functions
                .get_mut(slot)
                .ok_or("missing runtime entry slot")?;
            if entry.role != lower::FunctionRole::Runtime(role) || entry.signature != signature {
                return Err(format!(
                    "runtime entry {name} has an incompatible lowered signature"
                ));
            }
            let mut instructions: Vec<Instruction> = Vec::new();
            for parameter in 0usize..signature.parameters.len() {
                instructions.push(Instruction::LocalGet(parameter));
            }
            instructions.push(Instruction::Call(base + index));
            instructions.push(Instruction::End);
            entry.code = Code {
                locals: Vec::new(),
                instructions,
            };
        }
        Ok(())
    }
}
