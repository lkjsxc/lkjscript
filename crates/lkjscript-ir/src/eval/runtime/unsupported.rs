use super::super::*;

impl Evaluator<'_> {
    pub(crate) fn runtime_unsupported(
        &mut self,
        operation: RuntimeOp,
        _arguments: Vec<EvalValue>,
    ) -> std::result::Result<EvalValue, Flow> {
        reject_unsupported_host(operation)
    }
}

fn reject_unsupported_host(operation: RuntimeOp) -> std::result::Result<EvalValue, Flow> {
    use RuntimeOp as Op;
    match operation {
        Op::SysSqliteBusyTimeout
        | Op::SysSqliteExec
        | Op::SysSqliteReset
        | Op::SysSqliteClearBindings
        | Op::SysSqliteBindNull
        | Op::SysSqliteBindI64
        | Op::SysSqliteBindF64
        | Op::SysSqliteBindText
        | Op::SysSqliteBindBytes
        | Op::SysSqliteStep
        | Op::SysSqliteColumnCount
        | Op::SysSqliteColumnType
        | Op::SysSqliteColumnI64
        | Op::SysSqliteColumnF64
        | Op::SysSqliteColumnText
        | Op::SysSqliteColumnBytes
        | Op::SysSqliteChanges
        | Op::SysSqliteLastInsertRowid
        | Op::SysSqliteExtendedResultCode
        | Op::SysSqliteBackup
        | Op::Print
        | Op::Flush
        | Op::ReadByte
        | Op::WriteByte
        | Op::WriteStr
        | Op::SysReadByte
        | Op::SysWriteByte
        | Op::SysReadInto
        | Op::SysWriteFrom
        | Op::SysTtyGuardSave
        | Op::SysTtyGuardClear
        | Op::SysFsync
        | Op::SysTruncate
        | Op::SysRename
        | Op::SysRandomFill
        | Op::SysSha256
        | Op::SysPathExists
        | Op::SysWaitMs
        | Op::SysNowMs
        | Op::SysSocket
        | Op::SysBind
        | Op::SysListen
        | Op::SysAccept
        | Op::SysRecv
        | Op::SysSend
        | Op::SysPoll
        | Op::SysTtyGet
        | Op::SysTtySet => Err(Flow::Unsupported(operation)),
        _ => unreachable!("runtime operation dispatched to the wrong family"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unimplemented_host_operation_dispatch_remains_explicitly_unsupported() {
        let operations = [
            RuntimeOp::SysReadByte,
            RuntimeOp::SysSocket,
            RuntimeOp::SysTtyGuardSave,
            RuntimeOp::SysSqliteStep,
            RuntimeOp::SysSqliteColumnBytes,
        ];
        for operation in operations {
            assert!(matches!(
                reject_unsupported_host(operation),
                Err(Flow::Unsupported(actual)) if actual == operation
            ));
        }
    }
}
