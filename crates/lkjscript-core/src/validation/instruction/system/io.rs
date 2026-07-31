fn apply_io(
    chunk: &Chunk,
    proto: &FunctionProto,
    instruction: DecodedInstruction,
    state: &mut State,
) -> Result<()> {
    use ResourceKind::{
        Directory, FileAppender, FileReader, FileWriter, InputStream, OutputStream, TcpStream,
    };
    let op = instruction.op();
    match op {
        Op::SysReadByte => {
            expect_resource(
                state,
                &[InputStream, FileReader, TcpStream],
                proto,
                instruction,
            )?;
            state.stack.push(structural_value_result(
                chunk,
                crate::StructuralKind::I64,
                proto,
                instruction,
            )?);
        }
        Op::SysOpenRead => file_open(chunk, state, proto, instruction, FileReader)?,
        Op::SysOpenWrite | Op::SysOpenCreateNew => {
            file_open(chunk, state, proto, instruction, FileWriter)?
        }
        Op::SysOpenAppend => file_open(chunk, state, proto, instruction, FileAppender)?,
        Op::SysOpenDir => file_open(chunk, state, proto, instruction, Directory)?,
        Op::SysPathExists => {
            pop_structural_leaf(
                chunk,
                state,
                crate::StructuralKind::Path,
                Kind::Path,
                proto,
                instruction,
            )?;
            expect_capability(state, crate::CapabilityKind::FileSystem, proto, instruction)?;
            state.stack.push(structural_value_result(
                chunk,
                crate::StructuralKind::Bool,
                proto,
                instruction,
            )?);
        }
        Op::SysFsync => {
            expect_resource(
                state,
                &[FileWriter, FileAppender, Directory],
                proto,
                instruction,
            )?;
            state.stack.push(structural_value_result(
                chunk,
                crate::StructuralKind::Unit,
                proto,
                instruction,
            )?);
        }
        Op::SysWriteByte => {
            expect_pop(state, Kind::I64, proto, instruction)?;
            expect_resource(
                state,
                &[OutputStream, FileWriter, FileAppender, TcpStream],
                proto,
                instruction,
            )?;
            state.stack.push(structural_value_result(
                chunk,
                crate::StructuralKind::Unit,
                proto,
                instruction,
            )?);
        }
        Op::SysTruncate => {
            expect_pop(state, Kind::I64, proto, instruction)?;
            expect_resource(state, &[FileWriter, FileAppender], proto, instruction)?;
            state.stack.push(structural_value_result(
                chunk,
                crate::StructuralKind::Unit,
                proto,
                instruction,
            )?);
        }
        Op::SysReadInto | Op::SysWriteFrom => {
            super::unique::pop_used_view(state, op == Op::SysReadInto, proto, instruction)?;
            let kinds = if op == Op::SysReadInto {
                &[InputStream, FileReader, TcpStream][..]
            } else {
                &[OutputStream, FileWriter, FileAppender, TcpStream][..]
            };
            expect_resource(state, kinds, proto, instruction)?;
            state.stack.push(structural_value_result(
                chunk,
                crate::StructuralKind::I64,
                proto,
                instruction,
            )?);
        }
        Op::SysRename => {
            pop_structural_leaf(
                chunk,
                state,
                crate::StructuralKind::Path,
                Kind::Path,
                proto,
                instruction,
            )?;
            pop_structural_leaf(
                chunk,
                state,
                crate::StructuralKind::Path,
                Kind::Path,
                proto,
                instruction,
            )?;
            expect_capability(state, crate::CapabilityKind::FileSystem, proto, instruction)?;
            state.stack.push(structural_value_result(
                chunk,
                crate::StructuralKind::Unit,
                proto,
                instruction,
            )?);
        }
        _ => unreachable!("system I/O opcode family checked"),
    }
    Ok(())
}
