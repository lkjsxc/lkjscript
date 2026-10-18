//! Safe first-party AOT lowering to a static Linux x86-64 ELF image.
//! The generated executable contains native functions and Linux syscalls only.
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::execution::normalized::prepare::{
    NormalizedCode, NormalizedEntryPoint, NormalizedFieldSelector, NormalizedFunctionBody,
    NormalizedInstruction as I, NormalizedProgram,
};
use crate::platform::kernel::{Name, TypeForm};
use crate::platform::package::RunnerKind;
use std::collections::{BTreeMap, BTreeSet};

mod asm;
mod blake3;
mod buffer;
mod bytes;
mod filter;
mod runtime;
#[cfg(all(test, target_os = "linux", target_arch = "x86_64"))]
mod stack_tests;
use asm::{Asm, Label, R::*, *};
use runtime::Runtime;

type Result<T> = std::result::Result<T, Diagnostic>;
const TEXT_OFFSET: usize = 4096;
const IMAGE_ADDRESS: u64 = 0x400000;
const TEXT_ADDRESS: u64 = IMAGE_ADDRESS + TEXT_OFFSET as u64;
const MAX_FUNCTIONS: usize = 4096;
const MAX_INSTRUCTIONS: usize = 65_536;

fn error(code: &'static str, message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(DiagnosticClass::Source, code, message)
}
fn unsupported(message: impl Into<String>) -> Diagnostic {
    error("native_elf_unsupported", message)
}

pub(crate) struct Image {
    pub bytes: Vec<u8>,
    pub target: Name,
    pub functions: usize,
    pub rewrites: usize,
}

pub(crate) fn build(program: &NormalizedProgram, selection: Option<&str>) -> Result<Image> {
    let candidates: Vec<_> = program
        .targets
        .values()
        .filter(|target| {
            target.package == program.root_package
                && target.runner == RunnerKind::Command
                && selection.is_none_or(|name| name == target.name.as_str())
        })
        .collect();
    let [target] = candidates.as_slice() else {
        return Err(unsupported(
            "native build requires one command target; select it with --target",
        ));
    };
    let component = program
        .components
        .get(target.component.0 as usize)
        .ok_or_else(|| unsupported("native target component is absent"))?;
    if !component.requirements.is_empty() {
        return Err(unsupported(
            "native command targets must have no external capability requirements",
        ));
    }
    let port = program
        .ports
        .get(
            target
                .port
                .ok_or_else(|| unsupported("native target has no entry port"))?
                .0 as usize,
        )
        .ok_or_else(|| unsupported("native entry port is absent"))?;
    let NormalizedEntryPoint::Function(function) = port.entry else {
        return Err(unsupported(
            "native entry must select a named closed function",
        ));
    };
    let (bytes, functions, rewrites) = compile_function(program, function.0 as usize)?;
    Ok(Image {
        bytes,
        target: target.name.clone(),
        functions,
        rewrites,
    })
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
enum Constant {
    Unit,
    Bool(bool),
    I64(i64),
    Text(String),
    Function(usize),
}

struct Emitter<'a> {
    program: &'a NormalizedProgram,
    a: Asm,
    runtime: Runtime,
    functions: BTreeMap<usize, Label>,
    constants: BTreeMap<Constant, Label>,
    data: Vec<(Label, Vec<u8>)>,
    rewrites: usize,
}

pub(crate) fn compile_function(
    program: &NormalizedProgram,
    entry: usize,
) -> Result<(Vec<u8>, usize, usize)> {
    let mut reachable = BTreeSet::new();
    let mut pending = vec![entry];
    let mut instructions = 0usize;
    while let Some(index) = pending.pop() {
        if !reachable.insert(index) {
            continue;
        }
        if reachable.len() > MAX_FUNCTIONS {
            return Err(unsupported("native function closure exceeds its bound"));
        }
        let function = program
            .functions
            .get(index)
            .ok_or_else(|| unsupported("native call target is absent"))?;
        if !function.task_requirements.is_empty()
            || !function.implementation_arguments.is_empty()
            || !function.read_scope_parameters.is_empty()
        {
            return Err(unsupported(
                "native functions require closed capability-free calls without scoped views or implementation witnesses",
            ));
        }
        if let NormalizedFunctionBody::Code(code) = &function.body {
            instructions = instructions.saturating_add(code.instructions.len());
            if instructions > MAX_INSTRUCTIONS || code.local_count as usize > MAX_INSTRUCTIONS {
                return Err(unsupported(
                    "native code exceeds its bounded instruction or local inventory",
                ));
            }
            for instruction in code.instructions.iter() {
                if let I::Call { function, .. }
                | I::TailCall { function, .. }
                | I::FunctionValue { function, .. } = instruction
                {
                    pending.push(function.0 as usize);
                }
            }
        }
    }
    let function = program
        .functions
        .get(entry)
        .ok_or_else(|| unsupported("native entry is absent"))?;
    if !function.type_parameters.is_empty()
        || !function.requirement_parameters.is_empty()
        || !function.effect_parameters.is_empty()
    {
        return Err(unsupported("native entry must have a closed signature"));
    }
    let binary_filter = filter::signature(program, entry);
    for parameter in function.parameters.iter().filter(|_| !binary_filter) {
        if !matches!(
            program.types.get(&parameter.ty).map(|t| &t.form),
            Some(TypeForm::I64)
        ) {
            return Err(unsupported(
                "native command arguments currently require I64 parameters",
            ));
        }
    }
    if !binary_filter {
        admit_result(program, function.result.ty)?;
    }
    let mut a = Asm::new();
    let runtime = Runtime::new(&mut a);
    let functions = reachable.iter().map(|&index| (index, a.label())).collect();
    let mut e = Emitter {
        program,
        a,
        runtime,
        functions,
        constants: BTreeMap::new(),
        data: Vec::new(),
        rewrites: 0,
    };
    if binary_filter {
        e.start_filter(entry)?;
    } else {
        e.start(entry, function.parameters.len())?;
    }
    for &index in &reachable {
        e.function(index)?;
    }
    e.runtime.emit(&mut e.a, &mut e.data);
    for (label, bytes) in &e.data {
        e.a.align();
        e.a.bind(*label);
        e.a.bytes(bytes);
    }
    for (constant, label) in &e.constants {
        e.a.align();
        e.a.bind(*label);
        match constant {
            Constant::Unit => {
                e.a.bytes(&0_u64.to_le_bytes());
                e.a.bytes(&0_u64.to_le_bytes());
            }
            Constant::Bool(value) => {
                e.a.bytes(&1_u64.to_le_bytes());
                e.a.bytes(&u64::from(*value).to_le_bytes());
            }
            Constant::I64(value) => {
                e.a.bytes(&2_u64.to_le_bytes());
                e.a.bytes(&value.to_le_bytes());
            }
            Constant::Text(value) => {
                let encoded = serde_json::to_vec(value)
                    .map_err(|_| unsupported("native text encoding failed"))?;
                let data_label = e.a.label();
                let raw_label = e.a.label();
                e.a.bytes(&3_u64.to_le_bytes());
                e.a.bytes(&(encoded.len() as u64).to_le_bytes());
                e.a.absolute(data_label);
                e.a.bytes(&(value.len() as u64).to_le_bytes());
                e.a.absolute(raw_label);
                e.a.bind(data_label);
                e.a.bytes(&encoded);
                e.a.bind(raw_label);
                e.a.bytes(value.as_bytes());
            }
            Constant::Function(index) => {
                e.a.bytes(&6_u64.to_le_bytes());
                e.a.absolute(*e.functions.get(index).ok_or_else(|| {
                    unsupported("native function constant is outside its closure")
                })?);
                e.a.bytes(&0_u64.to_le_bytes());
            }
        }
    }
    let code = e.a.finish()?;
    let bytes = elf(&code)?;
    Ok((bytes, reachable.len(), e.rewrites))
}

fn admit_result(
    program: &NormalizedProgram,
    root: crate::platform::kernel::TypeObjectDigest,
) -> Result<()> {
    let mut pending = vec![root];
    let mut seen = BTreeSet::new();
    while let Some(ty) = pending.pop() {
        if !seen.insert(ty) {
            continue;
        }
        if seen.len() > MAX_INSTRUCTIONS || pending.len() > MAX_INSTRUCTIONS {
            return Err(unsupported(
                "native result type exceeds its admission bound",
            ));
        }
        match program.types.get(&ty).map(|ty| &ty.form) {
            Some(
                TypeForm::Unit
                | TypeForm::Bool
                | TypeForm::I64
                | TypeForm::Text
                | TypeForm::StaticText,
            ) => {}
            Some(TypeForm::List { item }) => pending.push(*item),
            Some(TypeForm::StructuralRecord { fields }) => {
                pending.extend(fields.iter().map(|field| field.ty))
            }
            Some(TypeForm::Named { declaration }) => {
                let record = program
                    .records
                    .iter()
                    .find(|r| r.declaration == *declaration && r.arguments.is_empty())
                    .ok_or_else(|| {
                        unsupported("native result requires a concrete record layout")
                    })?;
                pending.extend(record.fields.iter().map(|field| field.ty));
            }
            Some(TypeForm::Applied {
                declaration,
                arguments,
            }) => {
                let record = program
                    .records
                    .iter()
                    .find(|r| {
                        r.declaration == *declaration
                            && r.arguments.as_ref() == arguments.as_slice()
                    })
                    .ok_or_else(|| {
                        unsupported("native result requires a concrete record application")
                    })?;
                pending.extend(record.fields.iter().map(|field| field.ty));
            }
            _ => {
                return Err(unsupported(
                    "native result must be Unit, Bool, I64, text, a list or a record of supported results",
                ));
            }
        }
    }
    Ok(())
}

fn elf(code: &[u8]) -> Result<Vec<u8>> {
    let size = TEXT_OFFSET
        .checked_add(code.len())
        .filter(|size| *size <= MAX_BYTES)
        .ok_or_else(|| unsupported("native ELF exceeds its image bound"))?;
    let mut out = vec![0_u8; size];
    out[..16].copy_from_slice(&[0x7f, b'E', b'L', b'F', 2, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
    out[16..18].copy_from_slice(&2_u16.to_le_bytes());
    out[18..20].copy_from_slice(&62_u16.to_le_bytes());
    out[20..24].copy_from_slice(&1_u32.to_le_bytes());
    out[24..32].copy_from_slice(&TEXT_ADDRESS.to_le_bytes());
    out[32..40].copy_from_slice(&64_u64.to_le_bytes());
    out[52..54].copy_from_slice(&64_u16.to_le_bytes());
    out[54..56].copy_from_slice(&56_u16.to_le_bytes());
    out[56..58].copy_from_slice(&2_u16.to_le_bytes());
    // PT_LOAD: one RX image. Writable storage comes only from sys_mmap.
    out[64..68].copy_from_slice(&1_u32.to_le_bytes());
    out[68..72].copy_from_slice(&5_u32.to_le_bytes());
    out[80..88].copy_from_slice(&IMAGE_ADDRESS.to_le_bytes());
    out[88..96].copy_from_slice(&IMAGE_ADDRESS.to_le_bytes());
    out[96..104].copy_from_slice(&(size as u64).to_le_bytes());
    out[104..112].copy_from_slice(&(size as u64).to_le_bytes());
    out[112..120].copy_from_slice(&4096_u64.to_le_bytes());
    // PT_GNU_STACK explicitly requests a non-executable stack.
    out[120..124].copy_from_slice(&0x6474e551_u32.to_le_bytes());
    out[124..128].copy_from_slice(&6_u32.to_le_bytes());
    out[168..176].copy_from_slice(&16_u64.to_le_bytes());
    out[TEXT_OFFSET..].copy_from_slice(code);
    Ok(out)
}

impl Emitter<'_> {
    fn label(&self, index: usize) -> Result<Label> {
        self.functions
            .get(&index)
            .copied()
            .ok_or_else(|| unsupported("native function is outside the admitted closure"))
    }
    fn constant(&mut self, value: Constant) -> Label {
        if let Some(label) = self.constants.get(&value) {
            return *label;
        }
        let label = self.a.label();
        self.constants.insert(value, label);
        label
    }
    fn push_constant(&mut self, value: Constant) {
        let label = self.constant(value);
        self.a.address(Ax, label);
        self.a.push(Ax);
    }
    fn start(&mut self, entry: usize, parameters: usize) -> Result<()> {
        self.a.mov(R12, Sp);
        self.a.load(Ax, R12, 0);
        self.a.group(7, Ax, parameters as i32 + 1);
        self.a.when(NE, self.runtime.argument);
        self.a.imm(R14, 0);
        self.a.imm(R15, 0);
        for parameter in 0..parameters {
            self.a.load(Di, R12, 16 + parameter as i32 * 8);
            self.a.call(self.runtime.parse);
            self.a.push(Ax);
        }
        self.a.call(self.label(entry)?);
        self.a.group(0, Sp, parameters as i32 * 8);
        self.a.mov(Di, Ax);
        self.a.call(self.runtime.print);
        self.a.imm(Di, 0);
        self.a.imm(Ax, 60);
        self.a.syscall();
        self.a.bytes(&[0x0f, 0x0b]);
        Ok(())
    }
    fn prologue(&mut self, locals: u32) {
        self.a.push(Bp);
        self.a.mov(Bp, Sp);
        self.a.push(Bx);
        self.a.push(R12);
        self.a.push(R13);
        self.a.group(5, Sp, locals as i32 * 8);
    }
    fn epilogue(&mut self) {
        self.a.lea(Sp, Bp, -24);
        self.a.pop(R13);
        self.a.pop(R12);
        self.a.pop(Bx);
        self.a.pop(Bp);
        self.a.ret();
    }
    fn argument(&mut self, destination: R, offset: u32) {
        self.a.load(destination, Bp, 24);
        self.a.load(destination, destination, offset as i32 * 8);
    }
    fn function(&mut self, index: usize) -> Result<()> {
        self.a.bind(self.label(index)?);
        let function = &self.program.functions[index];
        let parameters = function.parameter_count;
        match function.body.clone() {
            NormalizedFunctionBody::External(name) => self.external(name.as_str(), parameters),
            NormalizedFunctionBody::Code(code) => self.code(index, &code),
        }
    }
    fn code(&mut self, function: usize, code: &NormalizedCode) -> Result<()> {
        self.prologue(code.local_count);
        for parameter in 0..code.parameter_count {
            self.argument(Ax, code.parameter_count - 1 - parameter);
            self.a.store(Bp, -32 - parameter as i32 * 8, Ax);
        }
        let labels: Vec<_> = (0..code.instructions.len())
            .map(|_| self.a.label())
            .collect();
        let plan = super::egraph_opt::optimize(self.program, code);
        self.rewrites += plan.rewrites.len();
        let mut ip = 0;
        while ip < code.instructions.len() {
            self.a.bind(labels[ip]);
            if let Some(rewrite) = plan.rewrites.get(&ip) {
                for instruction in &rewrite.instructions {
                    self.instruction(function, code, instruction, &labels)?;
                }
                for label in &labels[ip + 1..rewrite.end] {
                    self.a.bind(*label);
                }
                ip = rewrite.end;
            } else {
                self.instruction(function, code, &code.instructions[ip], &labels)?;
                ip += 1;
            }
        }
        Ok(())
    }
    fn instruction(
        &mut self,
        function: usize,
        code: &NormalizedCode,
        instruction: &I,
        labels: &[Label],
    ) -> Result<()> {
        match instruction {
            I::Unit => self.push_constant(Constant::Unit),
            I::Bool(v) => self.push_constant(Constant::Bool(*v)),
            I::I64(v) => self.push_constant(Constant::I64(*v)),
            I::Text(v) | I::StaticText(v) => self.push_constant(Constant::Text(v.to_string())),
            I::LoadLocal { local, .. } | I::MoveLocal(local) => {
                self.a.load(Ax, Bp, -32 - *local as i32 * 8);
                self.a.push(Ax);
            }
            I::StoreLocal(local) => {
                self.a.pop(Ax);
                self.a.store(Bp, -32 - *local as i32 * 8, Ax);
            }
            I::Drop => self.a.group(0, Sp, 8),
            I::Jump(target) => self.a.jump(
                *labels
                    .get(*target as usize)
                    .ok_or_else(|| unsupported("native jump is outside its body"))?,
            ),
            I::JumpIfFalse(target) => {
                self.a.pop(Ax);
                self.a.load(Ax, Ax, 8);
                self.a.bin(0x85, Ax, Ax);
                self.a.when(
                    E,
                    *labels
                        .get(*target as usize)
                        .ok_or_else(|| unsupported("native branch is outside its body"))?,
                );
            }
            I::Call {
                function: target,
                arguments,
                ..
            }
            | I::TailCall {
                function: target,
                arguments,
                ..
            } => {
                let tail = matches!(instruction, I::TailCall { .. });
                if tail && target.0 as usize == function {
                    for parameter in (0..*arguments).rev() {
                        self.a.pop(Ax);
                        self.a.store(Bp, -32 - parameter as i32 * 8, Ax);
                    }
                    self.a.jump(labels[0]);
                } else {
                    self.a.call(self.label(target.0 as usize)?);
                    self.a.group(0, Sp, *arguments as i32 * 8);
                    if tail {
                        self.epilogue();
                    } else {
                        self.a.push(Ax);
                    }
                }
            }
            I::FunctionValue { function, .. } => {
                self.push_constant(Constant::Function(function.0 as usize))
            }
            I::BeginBind { .. } | I::Capture { .. } => {}
            I::Invoke { arguments } | I::TailInvoke { arguments } => {
                self.invoke(*arguments);
                if matches!(instruction, I::TailInvoke { .. }) {
                    self.epilogue();
                } else {
                    self.a.push(Ax);
                }
            }
            I::Bind { arguments } => self.bind(*arguments),
            I::List { items } => {
                self.a.imm(Di, 16 + u64::from(*items) * 8);
                self.a.call(self.runtime.alloc);
                self.a.imm(Cx, 4);
                self.a.store(Ax, 0, Cx);
                self.a.imm(Cx, u64::from(*items));
                self.a.store(Ax, 8, Cx);
                for item in (0..*items).rev() {
                    self.a.pop(Cx);
                    self.a.store(Ax, 16 + item as i32 * 8, Cx);
                }
                self.a.push(Ax);
            }
            I::Record { fields, .. } => {
                let mut names = Vec::new();
                for field in fields.iter() {
                    names.push(match field {
                        NormalizedFieldSelector::Structural(name) => name.clone(),
                        NormalizedFieldSelector::Nominal { layout, offset } => self
                            .program
                            .records
                            .get(layout.0 as usize)
                            .and_then(|r| r.fields.get(*offset as usize))
                            .ok_or_else(|| unsupported("native record field is absent"))?
                            .name
                            .clone(),
                    });
                }
                let mut order: Vec<_> = (0..names.len()).collect();
                order.sort_by_key(|&i| &names[i]);
                self.a.imm(Di, 16 + fields.len() as u64 * 16);
                self.a.call(self.runtime.alloc);
                self.a.imm(Cx, 5);
                self.a.store(Ax, 0, Cx);
                self.a.imm(Cx, fields.len() as u64);
                self.a.store(Ax, 8, Cx);
                for (offset, &position) in order.iter().enumerate() {
                    let key = self.constant(Constant::Text(names[position].to_string()));
                    self.a.address(Cx, key);
                    self.a.store(Ax, 16 + offset as i32 * 16, Cx);
                    self.a
                        .load(Cx, Sp, (fields.len() - 1 - position) as i32 * 8);
                    self.a.store(Ax, 24 + offset as i32 * 16, Cx);
                }
                self.a.group(0, Sp, fields.len() as i32 * 8);
                self.a.push(Ax);
            }
            I::Field(field) => {
                let name = match field {
                    NormalizedFieldSelector::Structural(name) => name.clone(),
                    NormalizedFieldSelector::Nominal { layout, offset } => self
                        .program
                        .records
                        .get(layout.0 as usize)
                        .and_then(|r| r.fields.get(*offset as usize))
                        .ok_or_else(|| unsupported("native field selector is absent"))?
                        .name
                        .clone(),
                };
                let key = self.constant(Constant::Text(name.to_string()));
                self.a.pop(Ax);
                self.a.load(Dx, Ax, 8);
                self.a.lea(Ax, Ax, 16);
                self.a.address(Cx, key);
                let next = self.a.label();
                let found = self.a.label();
                self.a.bind(next);
                self.a.bin(0x85, Dx, Dx);
                self.a.when(E, self.runtime.bounds);
                self.a.load(Si, Ax, 0);
                self.a.bin(0x39, Si, Cx);
                self.a.when(E, found);
                self.a.group(0, Ax, 16);
                self.a.group(5, Dx, 1);
                self.a.jump(next);
                self.a.bind(found);
                self.a.load(Ax, Ax, 8);
                self.a.push(Ax);
            }
            I::Return => {
                self.a.pop(Ax);
                self.epilogue();
            }
            _ => {
                return Err(unsupported(format!(
                    "native lowering does not support {instruction:?}"
                )));
            }
        }
        let _ = code;
        Ok(())
    }
    fn invoke(&mut self, arguments: u32) {
        self.a.mov(Bx, Sp);
        self.a.load(R12, Bx, arguments as i32 * 8);
        self.a.load(R13, R12, 16);
        self.a.imm(Cx, 0);
        let next = self.a.label();
        let ready = self.a.label();
        self.a.bind(next);
        self.a.bin(0x39, Cx, R13);
        self.a.when(AE, ready);
        self.a.mov(Ax, Cx);
        self.a.shift(4, Ax, 3);
        self.a.bin(0x01, Ax, R12);
        self.a.load(Ax, Ax, 24);
        self.a.push(Ax);
        self.a.group(0, Cx, 1);
        self.a.jump(next);
        self.a.bind(ready);
        for argument in 0..arguments {
            self.a.load(Ax, Bx, (arguments - 1 - argument) as i32 * 8);
            self.a.push(Ax);
        }
        self.a.load(Ax, R12, 8);
        self.a.indirect(Ax);
        self.a.mov(Cx, R13);
        self.a.group(0, Cx, arguments as i32);
        self.a.shift(4, Cx, 3);
        self.a.bin(0x01, Sp, Cx);
        self.a.group(0, Sp, (arguments + 1) as i32 * 8);
    }
    fn bind(&mut self, arguments: u32) {
        self.a.load(R12, Sp, arguments as i32 * 8);
        self.a.load(R13, R12, 16);
        self.a.mov(Di, R13);
        self.a.group(0, Di, arguments as i32);
        self.a.shift(4, Di, 3);
        self.a.group(0, Di, 24);
        self.a.call(self.runtime.alloc);
        self.a.mov(Bx, Ax);
        self.a.imm(Cx, 6);
        self.a.store(Bx, 0, Cx);
        self.a.load(Cx, R12, 8);
        self.a.store(Bx, 8, Cx);
        self.a.mov(Cx, R13);
        self.a.group(0, Cx, arguments as i32);
        self.a.store(Bx, 16, Cx);
        self.a.imm(Cx, 0);
        let next = self.a.label();
        let ready = self.a.label();
        self.a.bind(next);
        self.a.bin(0x39, Cx, R13);
        self.a.when(AE, ready);
        self.a.mov(Ax, Cx);
        self.a.shift(4, Ax, 3);
        self.a.lea(Dx, R12, 24);
        self.a.bin(0x01, Dx, Ax);
        self.a.load(Dx, Dx, 0);
        self.a.bin(0x01, Ax, Bx);
        self.a.store(Ax, 24, Dx);
        self.a.group(0, Cx, 1);
        self.a.jump(next);
        self.a.bind(ready);
        self.a.mov(Ax, R13);
        self.a.shift(4, Ax, 3);
        self.a.bin(0x01, Ax, Bx);
        for argument in 0..arguments {
            self.a.load(Cx, Sp, (arguments - 1 - argument) as i32 * 8);
            self.a.store(Ax, 24 + argument as i32 * 8, Cx);
        }
        self.a.group(0, Sp, (arguments + 1) as i32 * 8);
        self.a.push(Bx);
    }
    fn external(&mut self, name: &str, parameters: u32) -> Result<()> {
        self.prologue(0);
        if parameters > 0 {
            self.argument(Ax, parameters - 1);
        }
        if parameters > 1 {
            self.argument(Cx, parameters - 2);
        }
        match name {
            "core.i64.add"
            | "core.i64.subtract"
            | "core.i64.multiply"
            | "core.i64.divide"
            | "core.i64.equal"
            | "core.i64.less"
            | "core.i64.less-equal" => {
                self.a.load(Ax, Ax, 8);
                self.a.load(Cx, Cx, 8);
                match name {
                    "core.i64.add" => {
                        self.a.bin(0x01, Ax, Cx);
                        self.a.when(O, self.runtime.overflow);
                        self.a.call(self.runtime.integer);
                    }
                    "core.i64.subtract" => {
                        self.a.bin(0x29, Ax, Cx);
                        self.a.when(O, self.runtime.overflow);
                        self.a.call(self.runtime.integer);
                    }
                    "core.i64.multiply" => {
                        self.a.imul(Ax, Cx);
                        self.a.when(O, self.runtime.overflow);
                        self.a.call(self.runtime.integer);
                    }
                    "core.i64.divide" => {
                        self.a.bin(0x85, Cx, Cx);
                        self.a.when(E, self.runtime.division);
                        self.a.imm(Dx, i64::MIN as u64);
                        self.a.bin(0x39, Ax, Dx);
                        let divide = self.a.label();
                        self.a.when(NE, divide);
                        self.a.group(7, Cx, -1);
                        self.a.when(E, self.runtime.division);
                        self.a.bind(divide);
                        self.a.bytes(&[0x48, 0x99]);
                        self.a.unary(7, Cx);
                        self.a.call(self.runtime.integer);
                    }
                    _ => {
                        self.a.bin(0x39, Ax, Cx);
                        self.a.set(if name == "core.i64.equal" {
                            E
                        } else if name == "core.i64.less" {
                            L
                        } else {
                            LE
                        });
                        self.a.call(self.runtime.boolean);
                    }
                }
            }
            "core.i64.to-text" => {
                self.a.load(Di, Ax, 8);
                self.a.call(self.runtime.integer_text);
            }
            "core.text.from-static" => {}
            "core.bool.not" => {
                self.a.load(Ax, Ax, 8);
                self.a.group(7, Ax, 0);
                self.a.set(E);
                self.a.call(self.runtime.boolean);
            }
            "core.bool.and" | "core.bool.or" | "core.bool.equal" => {
                self.a.load(Ax, Ax, 8);
                self.a.load(Cx, Cx, 8);
                if name == "core.bool.equal" {
                    self.a.bin(0x39, Ax, Cx);
                    self.a.set(E);
                } else {
                    self.a
                        .bin(if name == "core.bool.and" { 0x21 } else { 0x09 }, Ax, Cx);
                }
                self.a.call(self.runtime.boolean);
            }
            "core.list.length" => {
                self.a.load(Ax, Ax, 8);
                self.a.call(self.runtime.integer);
            }
            "core.list.get" => {
                self.a.load(Cx, Cx, 8);
                self.a.load(Dx, Ax, 8);
                self.a.bin(0x39, Cx, Dx);
                self.a.when(AE, self.runtime.bounds);
                self.a.shift(4, Cx, 3);
                self.a.bin(0x01, Ax, Cx);
                self.a.load(Ax, Ax, 16);
            }
            "core.list.append" => {
                self.a.mov(R12, Ax);
                self.a.mov(R13, Cx);
                self.a.load(Bx, R12, 8);
                self.a.mov(Di, Bx);
                self.a.group(0, Di, 1);
                self.a.when(O, self.runtime.capacity);
                self.a.imm(Cx, 8);
                self.a.imul(Di, Cx);
                self.a.when(O, self.runtime.capacity);
                self.a.group(0, Di, 16);
                self.a.when(O, self.runtime.capacity);
                self.a.call(self.runtime.alloc);
                self.a.imm(Cx, 4);
                self.a.store(Ax, 0, Cx);
                self.a.lea(Cx, Bx, 1);
                self.a.store(Ax, 8, Cx);
                self.a.mov(R9, Ax);
                self.a.lea(R8, Ax, 16);
                self.a.lea(R10, R12, 16);
                self.a.imm(Cx, 0);
                let next = self.a.label();
                let done = self.a.label();
                self.a.bind(next);
                self.a.bin(0x39, Cx, Bx);
                self.a.when(AE, done);
                self.a.load(Dx, R10, 0);
                self.a.store(R8, 0, Dx);
                self.a.group(0, R10, 8);
                self.a.group(0, R8, 8);
                self.a.group(0, Cx, 1);
                self.a.jump(next);
                self.a.bind(done);
                self.a.store(R8, 0, R13);
                self.a.mov(Ax, R9);
            }
            _ => {
                if !self.bytes_external(name) {
                    return Err(unsupported(format!(
                        "native external implementation {name} is not supported"
                    )));
                }
            }
        }
        self.epilogue();
        Ok(())
    }
}
