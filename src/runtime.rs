//! The host supplies a declared source runtime separately from user input.
//! Preparing that bundle is the sole authority to replace raw primitive stubs.
//! Ordinary source names, imports and K text cannot request this substitution.
mod memory;
use crate::ast::{Diagnostic, Span};
use crate::k::{self, CheckedProgram, Op, TypeKind};
use crate::lower::{self, Code, Function, Instruction, Signature};
use crate::source::SourceBundle;
pub(crate) use memory::Action as MemoryAction;
pub use memory::{Memory, MemoryFault};
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
    fn declarations() -> Vec<(&'static str, usize, Self)> {
        vec![
            ("raw_load8", 1, Self::Load8),
            ("raw_load64", 1, Self::Load64),
            ("raw_store8", 2, Self::Store8),
            ("raw_store64", 2, Self::Store64),
            ("raw_memory_size", 0, Self::MemorySize),
            ("raw_memory_grow", 1, Self::MemoryGrow),
            ("raw_memory_copy", 3, Self::MemoryCopy),
            ("raw_heap_tip", 0, Self::HeapTip),
            ("raw_set_heap_tip", 1, Self::SetHeapTip),
            ("raw_classes", 0, Self::Classes),
            ("raw_set_classes", 1, Self::SetClasses),
            ("raw_fail", 0, Self::Fail),
            ("raw_drop_step", 2, Self::DropStep),
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
            Self::Fail => vec![I64(6), GlobalSet(2), I64(0), GlobalSet(3), Unreachable],
            Self::DropStep => vec![LocalGet(0), LocalGet(1), CallIndirect(Signature::words(1))],
        };
        instructions.push(End);
        Code {
            locals: Vec::new(),
            instructions,
        }
    }
}

const ENTRIES: [(lower::RuntimeFunction, &str); 4] = [
    (lower::RuntimeFunction::Allocate, "allocate"),
    (lower::RuntimeFunction::Release, "release"),
    (lower::RuntimeFunction::DropDispatch, "drop_walk"),
    (lower::RuntimeFunction::Utf8Validate, "utf8_validate"),
];

/// The fields are private: a checked user program is not a source runtime.
pub struct Runtime {
    checked: CheckedProgram,
    primitives: BTreeMap<usize, Primitive>,
    entries: [usize; 4],
    functions: Vec<Function>,
}
fn problem(bundle: &SourceBundle, message: &str) -> Diagnostic {
    let mut error = Diagnostic::new(Span::default(), "E_RUNTIME", message);
    error.path = bundle.root.clone();
    error
}
fn word(program: &k::Program, ty: usize) -> bool {
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
pub fn prepare(bundle: &SourceBundle) -> Result<Runtime, Diagnostic> {
    let checked = crate::compiler::check_bundle(bundle)?;
    let program = checked.program();
    let names: BTreeMap<&str, usize> = program
        .functions
        .iter()
        .enumerate()
        .map(|(index, function)| (function.name.as_str(), index))
        .collect();
    let mut primitives = BTreeMap::new();
    for (name, parameters, primitive) in Primitive::declarations() {
        let index = *names
            .get(name)
            .ok_or_else(|| problem(bundle, &format!("missing runtime primitive {name}")))?;
        let function = &program.functions[index];
        if function.params != parameters
            || !word(program, function.result)
            || function.locals[..function.params]
                .iter()
                .any(|local| !word(program, local.ty))
        {
            return Err(problem(
                bundle,
                &format!("runtime primitive {name} has the wrong signature"),
            ));
        }
        if !matches!(
            function.body.first().map(|instruction| &instruction.op),
            Some(Op::Trap { .. })
        ) {
            return Err(problem(
                bundle,
                &format!("runtime primitive {name} must be an ordinary trapping stub"),
            ));
        }
        primitives.insert(index, primitive);
    }
    let mut entries = [0; 4];
    for (slot, (role, name)) in ENTRIES.iter().enumerate() {
        let index = *names
            .get(name)
            .ok_or_else(|| problem(bundle, &format!("missing runtime entry {name}")))?;
        let function = &program.functions[index];
        if function.params != role.signature().parameters.len()
            || !word(program, function.result)
            || function.locals[..function.params]
                .iter()
                .any(|local| !word(program, local.ty))
        {
            return Err(problem(
                bundle,
                &format!("runtime entry {name} has the wrong signature"),
            ));
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
                bundle,
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
                Op::Read { place, .. }
                | Op::Assign { place, .. }
                | Op::Drop { place }
                | Op::DropIf { place } => place.projections.is_empty(),
                _ => false,
            };
            if !scalar {
                return Err(problem(
                    bundle,
                    &format!(
                        "runtime function {} must use scalar operations and the sealed raw interface",
                        function.name
                    ),
                ));
            }
        }
    }
    let mut functions =
        lower::lower_runtime_functions(&checked).map_err(|message| problem(bundle, &message))?;
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
    /// the bytes and compare this source on K and every production encoder.
    pub fn model_program(&self, drop_steps: &[usize]) -> Result<lower::Program, String> {
        let mut functions = Vec::new();
        for (role, _) in ENTRIES {
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
            table.push(4 + index);
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
            exports: self
                .functions
                .iter()
                .enumerate()
                .filter(|(index, _)| !self.primitives.contains_key(index))
                .map(|(index, function)| lower::Export {
                    name: format!("fn:{}", function.name),
                    function: 4 + index,
                })
                .collect(),
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
    /// runtime calls retain their four stable entry slots and use trampolines.
    pub(crate) fn link(&self, program: &mut lower::Program) -> Result<(), String> {
        let base = program
            .imports
            .len()
            .checked_add(program.functions.len())
            .ok_or("runtime function index overflow")?;
        let end = 4_usize
            .checked_add(self.functions.len())
            .ok_or("runtime function index overflow")?;
        for function in &self.functions {
            let mut code = function.code.clone();
            for instruction in &mut code.instructions {
                if let Instruction::Call(index) = instruction {
                    if *index < 4 || *index >= end {
                        return Err(
                            "source runtime has a call outside its checked scalar closure"
                                .to_string(),
                        );
                    }
                    *index = base
                        .checked_add(*index - 4)
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
        for (slot, index) in self.entries.iter().enumerate() {
            let (role, name) = ENTRIES[slot];
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
            let mut instructions: Vec<_> = (0..signature.parameters.len())
                .map(Instruction::LocalGet)
                .collect();
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
