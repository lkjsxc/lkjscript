//! Lower resolved typed HIR into a bytecode chunk.

mod emit;

use std::collections::HashMap;

use emit::{add_constant, emit_expr, Cx};
use lkjscript_core::{Chunk, Constant, Error, FunctionProto, Op, ProductMetadata, Result};

use crate::hir::{BindingId, Function, Program};

pub(crate) fn compile_program(program: &Program) -> Result<Chunk> {
    let (mut chunk, globals) = initialize_chunk(program)?;
    for function in &program.functions {
        compile_function(&mut chunk, &globals, program, function)?;
    }
    compile_main(&mut chunk, &globals, program)?;
    Ok(chunk)
}

fn initialize_chunk(program: &Program) -> Result<(Chunk, HashMap<BindingId, u16>)> {
    let mut chunk = Chunk::new();
    chunk.main.name = "main".into();
    chunk.main.locals = program.main.local_count;
    for product in &program.products {
        if product.id.index() != chunk.products.len() {
            return Err(Error::msg(format!(
                "HIR product {} has inconsistent ProductId {}",
                product.name,
                product.id.raw()
            )));
        }
        let _field_count = u8::try_from(product.fields.len()).map_err(|_| {
            Error::msg(format!(
                "HIR product {} has too many bytecode fields",
                product.name
            ))
        })?;
        chunk.products.push(ProductMetadata {
            id: product.id,
            name: product.name.clone(),
            fields: product
                .fields
                .iter()
                .map(|field| field.name.clone())
                .collect(),
        });
    }
    let mut globals = HashMap::with_capacity(program.global_layout.len());
    for (index, binding_id) in program.global_layout.iter().copied().enumerate() {
        let slot = u16::try_from(index)
            .map_err(|_| Error::msg("too many HIR functions for bytecode u16 slots"))?;
        let binding = program.binding(binding_id).ok_or_else(|| {
            Error::msg(format!(
                "function layout references unknown HIR binding {}",
                binding_id.raw()
            ))
        })?;
        if globals.insert(binding_id, slot).is_some() {
            return Err(Error::msg(format!(
                "duplicate HIR binding {} in function layout",
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
    if usize::from(function.arity) != function.params.len() {
        return Err(Error::msg(format!(
            "function {name} has inconsistent HIR arity"
        )));
    }
    let proto = {
        let proto = FunctionProto {
            name: name.clone(),
            arity: function.arity,
            locals: function.local_count,
            code: Vec::new(),
        };
        let mut cx = Cx::new(chunk, globals, 0, proto);
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

fn compile_main(
    chunk: &mut Chunk,
    globals: &HashMap<BindingId, u16>,
    program: &Program,
) -> Result<()> {
    let code_base = u16::try_from(chunk.main.len())
        .map_err(|_| Error::msg("main bytecode offset exceeds u16"))?;
    let mut fragment = {
        let proto = FunctionProto {
            name: "main".into(),
            arity: 0,
            locals: program.main.local_count,
            code: Vec::new(),
        };
        let mut cx = Cx::new(chunk, globals, code_base, proto);
        emit_expr(&mut cx, &program.main.body)?;
        cx.proto.emit(Op::Return);
        cx.proto.code
    };
    chunk.main.code.append(&mut fragment);
    Ok(())
}

fn global_slot(globals: &HashMap<BindingId, u16>, binding: BindingId) -> Result<u16> {
    globals.get(&binding).copied().ok_or_else(|| {
        Error::msg(format!(
            "resolved HIR function binding {} has no bytecode slot",
            binding.raw()
        ))
    })
}
