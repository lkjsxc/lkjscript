mod algebraic;
mod buffers;
mod calls;
mod collections;
mod data;
mod enums;
mod io;
mod numeric;
mod required;
mod sqlite;
mod system;
mod system_types;
mod types;

use super::{decode::instruction_error, Kind, State};
use crate::{Chunk, DecodedInstruction, FunctionProto, Op, Result};

pub(super) fn apply_instruction(
    chunk: &Chunk,
    proto: &FunctionProto,
    instruction: DecodedInstruction,
    state: &mut State,
    is_main: bool,
) -> Result<()> {
    let required = required::stack(chunk, proto, instruction)?;
    if state.stack.len() < required {
        return Err(instruction_error(
            proto,
            instruction.op(),
            instruction.offset(),
            "operand stack underflow",
        ));
    }
    match instruction.op() {
        Op::Nop
        | Op::Jump
        | Op::Trap
        | Op::LoadConst
        | Op::LoadLocal
        | Op::StoreLocal
        | Op::LoadGlobal
        | Op::StoreGlobal
        | Op::Pop
        | Op::Dup
        | Op::StdinHandle
        | Op::False
        | Op::True
        | Op::Unit
        | Op::EmptyList
        | Op::Argc
        | Op::EmptyStr => data::apply(chunk, proto, instruction, state),
        Op::Add
        | Op::Sub
        | Op::Mul
        | Op::Div
        | Op::Lt
        | Op::Le
        | Op::Gt
        | Op::Ge
        | Op::BitAnd
        | Op::BitOr
        | Op::BitXor
        | Op::EqualValue
        | Op::Not
        | Op::JumpIfFalse
        | Op::F64BitsEqual
        | Op::F64FromI64Exact
        | Op::F64FromI64Rounded
        | Op::I64FromF64Exact
        | Op::I64FromF64Trunc => numeric::apply(chunk, proto, instruction, state),
        Op::Call | Op::Return | Op::MakeClosure => {
            calls::apply(chunk, proto, instruction, state, is_main)
        }
        Op::Cons | Op::Car | Op::Cdr | Op::IsEmptyList | Op::SameObject | Op::ListEqual => {
            collections::apply(chunk, proto, instruction, state)
        }
        Op::BufNew
        | Op::BufFromStr
        | Op::BufToStr
        | Op::PathFromStr
        | Op::PathFromBuf
        | Op::PathToBuf
        | Op::PathToStr
        | Op::BufSlice
        | Op::BufLen
        | Op::BufClone
        | Op::BufRef
        | Op::BufGetU32
        | Op::BufSet
        | Op::BufSetU32
        | Op::StrLen
        | Op::StrRef
        | Op::StrAppend
        | Op::StrSlice
        | Op::StrFromByte
        | Op::StrFromI64
        | Op::StrFromF64 => buffers::apply(chunk, proto, instruction, state),
        Op::Print
        | Op::Flush
        | Op::ReadByte
        | Op::WriteByte
        | Op::Exit
        | Op::WriteStr
        | Op::Arg => io::apply(chunk, proto, instruction, state),
        Op::SysTtyGet
        | Op::SysTtySet
        | Op::SysPoll
        | Op::SysIsatty
        | Op::SysClose
        | Op::SysReadByte
        | Op::SysAccept
        | Op::SysRecv
        | Op::SysTtyGuardSave
        | Op::SysTtyGuardClear
        | Op::SysNowMs
        | Op::SysSocket
        | Op::SysOpenRead
        | Op::SysOpenWrite
        | Op::SysOpenAppend
        | Op::SysOpenCreateNew
        | Op::SysOpenDir
        | Op::SysPathExists
        | Op::SysFsync
        | Op::SysWriteByte
        | Op::SysBind
        | Op::SysListen
        | Op::SysTruncate
        | Op::SysWaitMs
        | Op::SysSend
        | Op::SysReadInto
        | Op::SysWriteFrom
        | Op::SysRandomFill
        | Op::SysSha256
        | Op::SysRename => system::apply(chunk, proto, instruction, state),
        Op::SysSqliteOpen
        | Op::SysSqliteClose
        | Op::SysSqliteFinalize
        | Op::SysSqliteReset
        | Op::SysSqliteClearBindings
        | Op::SysSqliteBindNull
        | Op::SysSqliteStep
        | Op::SysSqliteColumnCount
        | Op::SysSqliteChanges
        | Op::SysSqliteLastInsertRowid
        | Op::SysSqliteExtendedResultCode
        | Op::SysSqliteBusyTimeout
        | Op::SysSqliteExec
        | Op::SysSqlitePrepare
        | Op::SysSqliteBindI64
        | Op::SysSqliteBindF64
        | Op::SysSqliteBindText
        | Op::SysSqliteBindBytes
        | Op::SysSqliteColumnType
        | Op::SysSqliteColumnI64
        | Op::SysSqliteColumnF64
        | Op::SysSqliteColumnText
        | Op::SysSqliteColumnBytes
        | Op::SysSqliteBackup => sqlite::apply(chunk, proto, instruction, state),
        Op::MakeProduct
        | Op::LoadProductField
        | Op::WithProductField
        | Op::MakeEnum
        | Op::IsEnumVariant
        | Op::LoadEnumField => algebraic::apply(chunk, proto, instruction, state),
    }
}
