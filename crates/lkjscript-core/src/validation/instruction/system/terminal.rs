fn apply_terminal(
    chunk: &Chunk,
    proto: &FunctionProto,
    instruction: DecodedInstruction,
    state: &mut State,
) -> Result<()> {
    use ResourceKind::{
        Directory, FileAppender, FileReader, FileWriter, InputStream, OutputStream, TcpListener,
        TcpStream,
    };
    let op = instruction.op();
    match op {
        Op::SysTtyGet | Op::SysTtySet => {
            super::unique::pop_used_view(state, op == Op::SysTtyGet, proto, instruction)?;
            expect_resource(state, &[InputStream], proto, instruction)?;
            state.stack.push(structural_value_result(
                chunk,
                crate::StructuralKind::Unit,
                proto,
                instruction,
            )?);
        }
        Op::SysPoll => {
            expect_pop(state, Kind::I64, proto, instruction)?;
            expect_resource(
                state,
                &[InputStream, FileReader, TcpListener, TcpStream],
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
        Op::SysIsatty => {
            expect_resource(state, &[InputStream], proto, instruction)?;
            state.stack.push(structural_value_result(
                chunk,
                crate::StructuralKind::Bool,
                proto,
                instruction,
            )?);
        }
        Op::SysClose | Op::ResourceDrop => {
            let resource = expect_resource(
                state,
                &[
                    OutputStream,
                    FileReader,
                    FileWriter,
                    FileAppender,
                    Directory,
                    TcpListener,
                    TcpStream,
                    ResourceKind::SqliteConnection,
                    ResourceKind::SqliteStatement,
                    ResourceKind::TerminalSession,
                ],
                proto,
                instruction,
            )?;
            consume_resource_owner(state, resource, proto, instruction)?;
            if op == Op::ResourceDrop {
                state.stack.push(Kind::Unit);
            } else {
                state.stack.push(structural_value_result(
                    chunk,
                    crate::StructuralKind::Unit,
                    proto,
                    instruction,
                )?);
            }
        }
        Op::SysTtyGuardSave => {
            super::unique::pop_used_view(state, false, proto, instruction)?;
            expect_capability(state, crate::CapabilityKind::Terminal, proto, instruction)?;
            state.stack.push(structural_value_result(
                chunk,
                crate::StructuralKind::Unit,
                proto,
                instruction,
            )?);
        }
        Op::SysTtyGuardClear => {
            expect_capability(state, crate::CapabilityKind::Terminal, proto, instruction)?;
            state.stack.push(structural_value_result(
                chunk,
                crate::StructuralKind::Unit,
                proto,
                instruction,
            )?);
        }
        Op::SysNowMs => {
            expect_capability(state, crate::CapabilityKind::Clock, proto, instruction)?;
            state.stack.push(structural_value_result(
                chunk,
                crate::StructuralKind::I64,
                proto,
                instruction,
            )?);
        }
        Op::SysWaitMs => {
            expect_pop(state, Kind::I64, proto, instruction)?;
            expect_capability(state, crate::CapabilityKind::Clock, proto, instruction)?;
            state.stack.push(structural_value_result(
                chunk,
                crate::StructuralKind::Unit,
                proto,
                instruction,
            )?);
        }
        Op::SysRandomFill => {
            super::unique::pop_used_view(state, true, proto, instruction)?;
            expect_capability(state, crate::CapabilityKind::Entropy, proto, instruction)?;
            state.stack.push(structural_value_result(
                chunk,
                crate::StructuralKind::Unit,
                proto,
                instruction,
            )?);
        }
        Op::SysSha256 => {
            super::unique::pop_used_view(state, false, proto, instruction)?;
            state
                .stack
                .push(Kind::Bytes(super::bytes::new_owner(instruction)?));
        }
        _ => unreachable!("system terminal opcode family checked"),
    }
    Ok(())
}
