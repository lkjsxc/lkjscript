//! Lower resolved typed HIR into a bytecode chunk.

mod emit;

use std::collections::HashMap;

use emit::{add_constant, emit_expr, emit_sequence, Cx};
use lkjscript_core::{Chunk, Constant, Error, FunctionProto, Op, Result};

use crate::hir::{BindingId, ExprKind, Function, Program, TopLevel, ValueDefinition};

pub(crate) fn compile_program(program: &Program) -> Result<Chunk> {
    let (mut chunk, globals) = initialize_chunk(program)?;
    for form in &program.forms {
        match form {
            TopLevel::Function(function) => {
                compile_function(&mut chunk, &globals, program, function)?;
            }
            TopLevel::Value(value) => {
                compile_value(&mut chunk, &globals, program, value)?;
            }
            TopLevel::Do { expression, .. } => {
                compile_top_level_do(&mut chunk, &globals, program, expression)?;
            }
        }
    }
    chunk.main.emit(Op::Unit);
    chunk.main.emit(Op::Return);
    Ok(chunk)
}

fn initialize_chunk(program: &Program) -> Result<(Chunk, HashMap<BindingId, u16>)> {
    let mut chunk = Chunk::new();
    chunk.main.locals = program.main_locals;
    let mut globals = HashMap::with_capacity(program.global_layout.len());
    for (index, binding_id) in program.global_layout.iter().copied().enumerate() {
        let slot = u16::try_from(index)
            .map_err(|_| Error::msg("too many HIR globals for bytecode u16 slots"))?;
        let binding = program.binding(binding_id).ok_or_else(|| {
            Error::msg(format!(
                "global layout references unknown HIR binding {}",
                binding_id.raw()
            ))
        })?;
        if globals.insert(binding_id, slot).is_some() {
            return Err(Error::msg(format!(
                "duplicate HIR binding {} in global layout",
                binding_id.raw()
            )));
        }
        chunk.global_names.push(binding.name.clone());
    }
    Ok((chunk, globals))
}

fn compile_function(
    chunk: &mut Chunk,
    globals: &HashMap<BindingId, u16>,
    program: &Program,
    function: &Function,
) -> Result<()> {
    let binding = program.binding(function.binding).ok_or_else(|| {
        Error::msg(format!(
            "function references unknown HIR binding {}",
            function.binding.raw()
        ))
    })?;
    let name = binding.name.clone();
    let mut locals = HashMap::with_capacity(function.params.len());
    for (index, parameter) in function.params.iter().copied().enumerate() {
        let slot = u8::try_from(index)
            .map_err(|_| Error::msg(format!("function {name} has too many parameters")))?;
        if locals.insert(parameter, slot).is_some() {
            return Err(Error::msg(format!(
                "function {name} repeats HIR parameter binding {}",
                parameter.raw()
            )));
        }
    }
    let proto = {
        let proto = FunctionProto {
            name: name.clone(),
            arity: function.arity,
            locals: function.local_count,
            code: Vec::new(),
        };
        let mut cx = Cx::new(chunk, globals, locals, 0, proto);
        emit_expr(&mut cx, &function.body)?;
        cx.proto.emit(Op::Return);
        cx.proto
    };

    let proto_id = u32::try_from(chunk.protos.len())
        .map_err(|_| Error::msg("too many function prototypes for bytecode u32 IDs"))?;
    chunk.protos.push(proto);
    let constant = add_constant(chunk, Constant::Proto(proto_id))?;
    let global = global_slot(globals, function.binding)?;
    chunk.main.emit_op_u16(Op::LoadConst, constant);
    chunk.main.emit_op_u16(Op::MakeClosure, 0);
    chunk.main.emit_op_u16(Op::StoreGlobal, global);
    chunk.main.emit(Op::Pop);
    Ok(())
}

fn compile_value(
    chunk: &mut Chunk,
    globals: &HashMap<BindingId, u16>,
    program: &Program,
    value: &ValueDefinition,
) -> Result<()> {
    let name = program
        .binding(value.binding)
        .map(|binding| binding.name.clone())
        .ok_or_else(|| {
            Error::msg(format!(
                "value definition references unknown HIR binding {}",
                value.binding.raw()
            ))
        })?;
    let code_base = u16::try_from(chunk.main.len())
        .map_err(|_| Error::msg("main bytecode offset exceeds u16"))?;
    let mut fragment = {
        let proto = FunctionProto {
            name,
            arity: 0,
            locals: program.main_locals,
            code: Vec::new(),
        };
        let mut cx = Cx::new(chunk, globals, HashMap::new(), code_base, proto);
        emit_expr(&mut cx, &value.value)?;
        let global = global_slot(globals, value.binding)?;
        cx.proto.emit_op_u16(Op::StoreGlobal, global);
        cx.proto.code
    };
    chunk.main.code.append(&mut fragment);
    chunk.main.emit(Op::Pop);
    Ok(())
}

fn compile_top_level_do(
    chunk: &mut Chunk,
    globals: &HashMap<BindingId, u16>,
    program: &Program,
    expression: &crate::hir::Expr,
) -> Result<()> {
    let ExprKind::Do(expressions) = &expression.kind else {
        return Err(Error::msg("top-level HIR do does not contain a Do node"));
    };
    let code_base = u16::try_from(chunk.main.len())
        .map_err(|_| Error::msg("main bytecode offset exceeds u16"))?;
    let mut fragment = {
        let proto = FunctionProto {
            name: "<do>".into(),
            arity: 0,
            locals: program.main_locals,
            code: Vec::new(),
        };
        let mut cx = Cx::new(chunk, globals, HashMap::new(), code_base, proto);
        emit_sequence(&mut cx, expressions, false)?;
        cx.proto.code
    };
    chunk.main.code.append(&mut fragment);
    Ok(())
}

fn global_slot(globals: &HashMap<BindingId, u16>, binding: BindingId) -> Result<u16> {
    globals.get(&binding).copied().ok_or_else(|| {
        Error::msg(format!(
            "resolved HIR binding {} has no bytecode global slot",
            binding.raw()
        ))
    })
}
