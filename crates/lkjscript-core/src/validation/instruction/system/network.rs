fn apply_network(
    chunk: &Chunk,
    proto: &FunctionProto,
    instruction: DecodedInstruction,
    state: &mut State,
) -> Result<()> {
    use ResourceKind::{TcpListener, TcpStream};
    match instruction.op() {
        Op::SysAccept => {
            expect_resource(state, &[TcpListener], proto, instruction)?;
            state
                .stack
                .push(resource_result_kind(TcpStream, proto, instruction)?);
        }
        Op::SysRecv => {
            expect_resource(state, &[TcpStream], proto, instruction)?;
            state.stack.push(structural_value_result(
                chunk,
                crate::StructuralKind::String,
                proto,
                instruction,
            )?);
        }
        Op::SysSocket => {
            expect_capability(state, crate::CapabilityKind::Network, proto, instruction)?;
            state
                .stack
                .push(resource_result_kind(TcpListener, proto, instruction)?);
        }
        Op::SysBind | Op::SysListen => {
            expect_pop(state, Kind::I64, proto, instruction)?;
            expect_resource(state, &[TcpListener], proto, instruction)?;
            state.stack.push(structural_value_result(
                chunk,
                crate::StructuralKind::Unit,
                proto,
                instruction,
            )?);
        }
        Op::SysSend => {
            pop_structural_leaf(
                chunk,
                state,
                crate::StructuralKind::String,
                Kind::Str,
                proto,
                instruction,
            )?;
            expect_resource(state, &[TcpStream], proto, instruction)?;
            state.stack.push(structural_value_result(
                chunk,
                crate::StructuralKind::I64,
                proto,
                instruction,
            )?);
        }
        _ => unreachable!("system network opcode family checked"),
    }
    Ok(())
}
