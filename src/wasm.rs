//! Memory64 WebAssembly encoding of the target-independent lowered program.
//! Layout, copy/drop glue, source control flow and runtime construction live in
//! lower.rs. This module only selects encodings and checks format extents.
use crate::encoding::{sleb, uleb};
use crate::lower::{Code, HostFunction, Instruction, Program, ScalarOp, Signature, ValueType};
use std::collections::BTreeMap;

fn text(out: &mut Vec<u8>, name: &str) {
    uleb(out, name.len() as u64);
    out.extend_from_slice(name.as_bytes());
}
fn section(out: &mut Vec<u8>, id: u8, bytes: &[u8]) -> Result<(), String> {
    if bytes.len() > u32::MAX as usize {
        return Err("WebAssembly section exceeds its u32 byte-length representation".to_string());
    }
    out.push(id);
    uleb(out, bytes.len() as u64);
    out.extend_from_slice(bytes);
    Ok(())
}
fn value_type(ty: ValueType) -> u8 {
    match ty {
        ValueType::I32 => 0x7f,
        ValueType::I64 => 0x7e,
        ValueType::F64 => 0x7c,
    }
}
fn constant(out: &mut Vec<u8>, value: i64) {
    out.push(0x42);
    sleb(out, value);
}
fn scalar(out: &mut Vec<u8>, op: ScalarOp) {
    use ScalarOp::*;
    let opcode = match op {
        I32Eqz => 0x45,
        I32And => 0x71,
        I32Or => 0x72,
        I64Eqz => 0x50,
        I64Eq => 0x51,
        I64Ne => 0x52,
        I64LtS => 0x53,
        I64LtU => 0x54,
        I64GtS => 0x55,
        I64GtU => 0x56,
        I64LeS => 0x57,
        I64LeU => 0x58,
        I64GeS => 0x59,
        I64GeU => 0x5a,
        I64Add => 0x7c,
        I64Sub => 0x7d,
        I64Mul => 0x7e,
        I64DivS => 0x7f,
        I64DivU => 0x80,
        I64RemS => 0x81,
        I64RemU => 0x82,
        I64And => 0x83,
        I64Or => 0x84,
        I64Xor => 0x85,
        I64Shl => 0x86,
        I64ShrS => 0x87,
        I64ShrU => 0x88,
        F64Eq => 0x61,
        F64Ne => 0x62,
        F64Lt => 0x63,
        F64Gt => 0x64,
        F64Le => 0x65,
        F64Ge => 0x66,
        F64Add => 0xa0,
        F64Sub => 0xa1,
        F64Mul => 0xa2,
        F64Div => 0xa3,
        I32WrapI64 => 0xa7,
        I64ExtendI32U => 0xad,
        F64ConvertI64S => 0xb9,
        F64ConvertI64U => 0xba,
        I64ReinterpretF64 => 0xbd,
        F64ReinterpretI64 => 0xbf,
        I64TruncSatF64S | I64TruncSatF64U => {
            out.push(0xfc);
            if op == I64TruncSatF64S { 6 } else { 7 }
        }
    };
    out.push(opcode);
}
fn code(body: &Code, signatures: &BTreeMap<Signature, usize>) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    let mut groups = Vec::<(usize, ValueType)>::new();
    for ty in &body.locals {
        if let Some((count, previous)) = groups.last_mut() {
            if previous == ty {
                *count += 1;
                continue;
            }
        }
        groups.push((1, *ty));
    }
    uleb(&mut out, groups.len() as u64);
    for (count, ty) in groups {
        uleb(&mut out, count as u64);
        out.push(value_type(ty));
    }
    for instruction in &body.instructions {
        match instruction {
            Instruction::I32(value) => {
                out.push(0x41);
                sleb(&mut out, i64::from(*value));
            }
            Instruction::I64(value) => constant(&mut out, *value),
            Instruction::LocalGet(index)
            | Instruction::LocalSet(index)
            | Instruction::LocalTee(index)
            | Instruction::GlobalGet(index)
            | Instruction::GlobalSet(index)
            | Instruction::Call(index) => {
                out.push(match instruction {
                    Instruction::LocalGet(_) => 0x20,
                    Instruction::LocalSet(_) => 0x21,
                    Instruction::LocalTee(_) => 0x22,
                    Instruction::GlobalGet(_) => 0x23,
                    Instruction::GlobalSet(_) => 0x24,
                    _ => 0x10,
                });
                uleb(&mut out, *index as u64);
            }
            Instruction::Scalar(op) => scalar(&mut out, *op),
            Instruction::Load {
                bytes,
                signed,
                offset,
            } => {
                out.push(match (*bytes, *signed) {
                    (1, true) => 0x30,
                    (1, false) => 0x31,
                    (2, true) => 0x32,
                    (2, false) => 0x33,
                    (4, true) => 0x34,
                    (4, false) => 0x35,
                    (8, _) => 0x29,
                    _ => return Err("invalid lowered load width".to_string()),
                });
                uleb(&mut out, bytes.trailing_zeros() as u64);
                uleb(&mut out, *offset);
            }
            Instruction::Store { bytes, offset } => {
                out.push(match bytes {
                    1 => 0x3c,
                    2 => 0x3d,
                    4 => 0x3e,
                    8 => 0x37,
                    _ => return Err("invalid lowered store width".to_string()),
                });
                uleb(&mut out, bytes.trailing_zeros() as u64);
                uleb(&mut out, *offset);
            }
            Instruction::Block(result) | Instruction::Loop(result) | Instruction::If(result) => {
                out.push(match instruction {
                    Instruction::Block(_) => 2,
                    Instruction::Loop(_) => 3,
                    _ => 4,
                });
                out.push(result.map(value_type).unwrap_or(0x40));
            }
            Instruction::Branch { depth, conditional } => {
                out.push(if *conditional { 0x0d } else { 0x0c });
                uleb(&mut out, *depth as u64);
            }
            Instruction::CallIndirect(signature) => {
                out.extend_from_slice(&[0xa7, 0x11]);
                uleb(&mut out, signatures[signature] as u64);
                out.push(0);
            }
            Instruction::MemorySize => out.extend_from_slice(&[0x3f, 0]),
            Instruction::MemoryGrow => out.extend_from_slice(&[0x40, 0]),
            Instruction::MemoryCopy => out.extend_from_slice(&[0xfc, 10, 0, 0]),
            Instruction::Drop => out.push(0x1a),
            Instruction::Else => out.push(5),
            Instruction::End => out.push(0x0b),
            Instruction::Return => out.push(0x0f),
            Instruction::Unreachable => out.push(0),
        }
    }
    Ok(out)
}

/// Compatibility entry for checked callers; all target-independent work is in lower.
pub fn emit(checked: &crate::k::CheckedProgram) -> Result<Vec<u8>, String> {
    encode(&crate::lower::lower(checked)?)
}

pub fn encode(program: &Program) -> Result<Vec<u8>, String> {
    let function_count = program
        .imports
        .len()
        .checked_add(program.functions.len())
        .ok_or("function count overflow")?;
    if function_count > u32::MAX as usize || program.table.len() > u32::MAX as usize {
        return Err("WebAssembly function/table indices exceed the u32 representation".to_string());
    }
    let mut signatures = BTreeMap::<Signature, usize>::new();
    let mut ordered = Vec::new();
    let mut add = |signature: &Signature| {
        if !signatures.contains_key(signature) {
            let index = ordered.len();
            ordered.push(signature.clone());
            signatures.insert(signature.clone(), index);
        }
    };
    for import in &program.imports {
        add(&import.signature);
    }
    for function in &program.functions {
        add(&function.signature);
        if function
            .signature
            .parameters
            .len()
            .checked_add(function.code.locals.len())
            .is_none_or(|len| len > u32::MAX as usize)
        {
            return Err(
                "WebAssembly parameter/local indices exceed the u32 representation".to_string(),
            );
        }
        for instruction in &function.code.instructions {
            if let Instruction::CallIndirect(signature) = instruction {
                add(signature);
            }
        }
    }
    let mut out = vec![0, 97, 115, 109, 1, 0, 0, 0];
    let mut types = Vec::new();
    uleb(&mut types, ordered.len() as u64);
    for signature in ordered {
        types.push(0x60);
        uleb(&mut types, signature.parameters.len() as u64);
        for parameter in signature.parameters {
            types.push(value_type(parameter));
        }
        types.push(u8::from(signature.result.is_some()));
        if let Some(result) = signature.result {
            types.push(value_type(result));
        }
    }
    section(&mut out, 1, &types)?;
    let mut imports = Vec::new();
    uleb(&mut imports, program.imports.len() as u64);
    for import in &program.imports {
        match import.host {
            HostFunction::StdoutWrite => {
                text(&mut imports, "lkjscript");
                text(&mut imports, "stdout");
            }
        }
        imports.push(0);
        uleb(&mut imports, signatures[&import.signature] as u64);
    }
    section(&mut out, 2, &imports)?;
    let mut functions = Vec::new();
    uleb(&mut functions, program.functions.len() as u64);
    for function in &program.functions {
        uleb(&mut functions, signatures[&function.signature] as u64);
    }
    section(&mut out, 3, &functions)?;
    let mut tables = vec![1, 0x70, 0];
    uleb(&mut tables, program.table.len() as u64);
    section(&mut out, 4, &tables)?;
    let mut memory = vec![1, 4];
    uleb(&mut memory, program.minimum_memory_pages);
    section(&mut out, 5, &memory)?;
    let mut globals = Vec::new();
    uleb(&mut globals, program.globals.len() as u64);
    for global in &program.globals {
        globals.extend_from_slice(&[0x7e, u8::from(global.mutable)]);
        constant(&mut globals, global.initial);
        globals.push(0x0b);
    }
    section(&mut out, 6, &globals)?;
    let mut exports = Vec::new();
    uleb(&mut exports, (program.exports.len() + 3) as u64);
    text(&mut exports, "memory");
    exports.extend_from_slice(&[2, 0]);
    for (name, index) in [
        ("__lkj_trap_code", program.trap_code_global),
        ("__lkj_trap_line", program.trap_line_global),
    ] {
        text(&mut exports, name);
        exports.push(3);
        uleb(&mut exports, index as u64);
    }
    for export in &program.exports {
        text(&mut exports, &export.name);
        exports.push(0);
        uleb(&mut exports, export.function as u64);
    }
    section(&mut out, 7, &exports)?;
    let mut elements = vec![1, 0, 0x41, 0, 0x0b];
    uleb(&mut elements, program.table.len() as u64);
    for function in &program.table {
        uleb(&mut elements, *function as u64);
    }
    section(&mut out, 9, &elements)?;
    let mut bodies = Vec::new();
    uleb(&mut bodies, program.functions.len() as u64);
    for function in &program.functions {
        let body = code(&function.code, &signatures)?;
        uleb(&mut bodies, body.len() as u64);
        bodies.extend_from_slice(&body);
    }
    section(&mut out, 10, &bodies)?;
    if !program.data.is_empty() {
        let mut data = vec![1, 0];
        constant(&mut data, program.data_offset as i64);
        data.push(0x0b);
        uleb(&mut data, program.data.len() as u64);
        data.extend_from_slice(&program.data);
        section(&mut out, 11, &data)?;
    }
    let mut names = Vec::new();
    uleb(&mut names, function_count as u64);
    for (i, import) in program.imports.iter().enumerate() {
        uleb(&mut names, i as u64);
        text(
            &mut names,
            match import.host {
                HostFunction::StdoutWrite => "stdout::write",
            },
        );
    }
    for (i, function) in program.functions.iter().enumerate() {
        uleb(&mut names, (program.imports.len() + i) as u64);
        text(&mut names, &function.name);
    }
    let mut custom = Vec::new();
    text(&mut custom, "name");
    custom.push(1);
    uleb(&mut custom, names.len() as u64);
    custom.extend_from_slice(&names);
    section(&mut out, 0, &custom)?;
    if !program.trap_messages.is_empty() {
        let mut metadata = Vec::new();
        text(&mut metadata, "lkjscript.traps");
        metadata.push(b'[');
        for (i, message) in program.trap_messages.iter().enumerate() {
            if i > 0 {
                metadata.push(b',');
            }
            crate::json::write_string(&mut metadata, message);
        }
        metadata.push(b']');
        section(&mut out, 0, &metadata)?;
    }
    Ok(out)
}
