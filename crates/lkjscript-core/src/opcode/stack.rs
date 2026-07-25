use super::{metadata::StackEffect, model::Op};

pub(super) const fn stack_effect(op: Op) -> StackEffect {
    use StackEffect::{Call, MakeProduct};

    match op {
        Op::Nop | Op::Trap => fixed(0, 0, 0),
        Op::LoadConst
        | Op::LoadLocal
        | Op::LoadGlobal
        | Op::Flush
        | Op::ReadByte
        | Op::StdinHandle
        | Op::SysTtyGuardClear
        | Op::False
        | Op::True
        | Op::Unit
        | Op::EmptyList
        | Op::OptionNone
        | Op::Argc
        | Op::EmptyStr
        | Op::SysNowMs
        | Op::SysSocket => fixed(0, 0, 1),
        Op::StoreLocal | Op::StoreGlobal => fixed(1, 0, 0),
        Op::Add
        | Op::Sub
        | Op::Mul
        | Op::Div
        | Op::EqualValue
        | Op::Lt
        | Op::Le
        | Op::Gt
        | Op::Ge
        | Op::BitAnd
        | Op::BitOr
        | Op::BitXor
        | Op::Cons
        | Op::SameObject
        | Op::ListEqual
        | Op::F64BitsEqual
        | Op::StrRef
        | Op::StrAppend
        | Op::SysWriteByte
        | Op::SysPoll
        | Op::SysTtyGet
        | Op::SysTtySet
        | Op::SysBind
        | Op::SysListen
        | Op::SysSend
        | Op::SysTruncate
        | Op::SysRename => fixed(2, 2, 1),
        Op::SysReadInto | Op::SysWriteFrom => fixed(4, 4, 1),
        Op::SysRandomFill => fixed(3, 3, 1),
        Op::SysSha256 => fixed(3, 3, 1),
        Op::SysSqliteClose
        | Op::SysSqliteFinalize
        | Op::SysSqliteReset
        | Op::SysSqliteClearBindings
        | Op::SysSqliteStep
        | Op::SysSqliteColumnCount
        | Op::SysSqliteChanges
        | Op::SysSqliteLastInsertRowid
        | Op::SysSqliteExtendedResultCode => fixed(1, 1, 1),
        Op::SysSqliteOpen
        | Op::SysSqliteBindNull
        | Op::SysSqliteBusyTimeout
        | Op::SysSqliteExec
        | Op::SysSqlitePrepare
        | Op::SysSqliteColumnType
        | Op::SysSqliteColumnI64
        | Op::SysSqliteColumnF64
        | Op::SysSqliteColumnText
        | Op::SysSqliteColumnBytes => fixed(2, 2, 1),
        Op::BufSlice
        | Op::SysSqliteBindI64
        | Op::SysSqliteBindF64
        | Op::SysSqliteBindText
        | Op::SysSqliteBindBytes
        | Op::SysSqliteBackup => fixed(3, 3, 1),
        Op::BufRef | Op::BufGetU32 => fixed(2, 2, 1),
        Op::StrSlice | Op::BufSet => fixed(3, 3, 1),
        Op::BufSetU32 => fixed(3, 3, 1),
        Op::Not
        | Op::Car
        | Op::Cdr
        | Op::IsEmptyList
        | Op::Print
        | Op::WriteByte
        | Op::WriteStr
        | Op::BufNew
        | Op::BufLen
        | Op::BufClone
        | Op::SysIsatty
        | Op::SysTtyGuardSave
        | Op::StrLen
        | Op::StrFromByte
        | Op::SysOpenRead
        | Op::SysOpenWrite
        | Op::SysOpenAppend
        | Op::SysOpenCreateNew
        | Op::SysOpenDir
        | Op::SysFsync
        | Op::SysClose
        | Op::SysReadByte
        | Op::Arg
        | Op::SysWaitMs
        | Op::SysAccept
        | Op::SysPathExists
        | Op::SysRecv
        | Op::OkWrap
        | Op::ErrWrap
        | Op::IsOk
        | Op::UnwrapOk
        | Op::UnwrapErr
        | Op::StrFromI64
        | Op::StrFromF64
        | Op::SomeWrap
        | Op::IsSome
        | Op::UnwrapSome
        | Op::LoadProductField
        | Op::BufFromStr
        | Op::BufToStr => fixed(1, 1, 1),
        Op::Jump => fixed(0, 0, 0),
        Op::JumpIfFalse | Op::Exit | Op::Pop | Op::Return => fixed(1, 1, 0),
        Op::MakeClosure => fixed(1, 1, 1),
        Op::Call => Call,
        Op::MakeProduct => MakeProduct,
        Op::Dup => fixed(1, 0, 1),
        Op::WithProductField => fixed(2, 2, 1),
    }
}

const fn fixed(required: usize, pops: usize, pushes: usize) -> StackEffect {
    StackEffect::Fixed {
        required,
        pops,
        pushes,
    }
}
