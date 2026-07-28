use lkjscript_core::{CapabilityKind, ResourceKind};

use super::super::*;

impl Evaluator<'_> {
    pub(crate) fn runtime_resources(
        &mut self,
        operation: RuntimeOp,
        arguments: Vec<EvalValue>,
    ) -> std::result::Result<EvalValue, Flow> {
        use RuntimeOp as Op;
        match operation {
            Op::StdinHandle => {
                expect_capability(&arguments, CapabilityKind::Stdio)?;
                self.resources
                    .standard_input()
                    .map(EvalValue::Resource)
                    .map_err(Flow::Trap)
            }
            Op::SysIsatty => {
                let resource = expect_resource_kind(&arguments, ResourceKind::InputStream)?;
                let result = self
                    .resources
                    .validate_borrowed(&resource, ResourceKind::InputStream);
                match result {
                    Ok(()) => self.allocate_result(EvalValue::Bool(false), true),
                    Err(message) => {
                        self.allocate_system_error(crate::prelude_contract::SYSTEM_IO_ID, &message)
                    }
                }
            }
            Op::SysOpenRead => {
                expect_capability_path(&arguments, CapabilityKind::FileSystem)?;
                self.acquire_resource_result(ResourceKind::FileReader)
            }
            Op::SysOpenWrite | Op::SysOpenCreateNew => {
                expect_capability_path(&arguments, CapabilityKind::FileSystem)?;
                self.acquire_resource_result(ResourceKind::FileWriter)
            }
            Op::SysOpenAppend => {
                expect_capability_path(&arguments, CapabilityKind::FileSystem)?;
                self.acquire_resource_result(ResourceKind::FileAppender)
            }
            Op::SysOpenDir => {
                expect_capability_path(&arguments, CapabilityKind::FileSystem)?;
                self.acquire_resource_result(ResourceKind::Directory)
            }
            Op::SysClose => {
                let resource = expect_resource(&arguments)?;
                let result = self.resources.close_configured(resource);
                self.unit_resource_result(result)
            }
            Op::SysSqliteOpen => {
                expect_sqlite_open_arguments(&arguments)?;
                self.acquire_resource_result(ResourceKind::SqliteConnection)
            }
            Op::SysSqlitePrepare => {
                let connection = expect_sqlite_prepare_arguments(&arguments)?;
                let result = self
                    .resources
                    .prepare_statement_configured(&connection)
                    .map(EvalValue::Resource);
                self.resource_result(result)
            }
            Op::SysSqliteClose => {
                let resource = expect_resource_kind(&arguments, ResourceKind::SqliteConnection)?;
                let result = self.resources.close_sqlite_connection(resource);
                self.unit_resource_result(result)
            }
            Op::SysSqliteFinalize => {
                let resource = expect_resource_kind(&arguments, ResourceKind::SqliteStatement)?;
                let result = self.resources.finalize_statement(resource);
                self.unit_resource_result(result)
            }
            _ => unreachable!("runtime operation dispatched to wrong resource family"),
        }
    }

    fn acquire_resource_result(
        &mut self,
        kind: ResourceKind,
    ) -> std::result::Result<EvalValue, Flow> {
        let result = self
            .resources
            .acquire_configured(kind)
            .map(EvalValue::Resource);
        self.resource_result(result)
    }

    fn unit_resource_result(
        &mut self,
        result: Result<(), String>,
    ) -> std::result::Result<EvalValue, Flow> {
        self.resource_result(result.map(|()| EvalValue::Unit))
    }

    fn resource_result(
        &mut self,
        result: Result<EvalValue, String>,
    ) -> std::result::Result<EvalValue, Flow> {
        match result {
            Ok(value) => self.allocate_result(value, true),
            Err(message) => {
                self.allocate_system_error(crate::prelude_contract::SYSTEM_IO_ID, &message)
            }
        }
    }
}

fn expect_capability(
    arguments: &[EvalValue],
    expected: CapabilityKind,
) -> std::result::Result<(), Flow> {
    match arguments {
        [EvalValue::Capability(actual)] if *actual == expected => Ok(()),
        _ => Err(Flow::Trap("evaluator resource capability mismatch".into())),
    }
}

fn expect_capability_path(
    arguments: &[EvalValue],
    expected: CapabilityKind,
) -> std::result::Result<(), Flow> {
    match arguments {
        [EvalValue::Capability(actual), EvalValue::Path(_)] if *actual == expected => Ok(()),
        _ => Err(Flow::Trap(
            "evaluator resource open arguments mismatch".into(),
        )),
    }
}

fn expect_sqlite_open_arguments(arguments: &[EvalValue]) -> std::result::Result<(), Flow> {
    match arguments {
        [EvalValue::Capability(CapabilityKind::Sqlite), EvalValue::Path(_), EvalValue::I64(_)] => {
            Ok(())
        }
        _ => Err(Flow::Trap(
            "evaluator SQLite open arguments mismatch".into(),
        )),
    }
}

fn expect_sqlite_prepare_arguments(
    arguments: &[EvalValue],
) -> std::result::Result<EvalResource, Flow> {
    match arguments {
        [EvalValue::Resource(resource), EvalValue::Str(_)]
            if resource.kind() == ResourceKind::SqliteConnection =>
        {
            Ok(resource.clone())
        }
        _ => Err(Flow::Trap(
            "evaluator SQLite prepare arguments mismatch".into(),
        )),
    }
}

fn expect_resource(arguments: &[EvalValue]) -> std::result::Result<EvalResource, Flow> {
    match arguments {
        [EvalValue::Resource(resource)] => Ok(resource.clone()),
        _ => Err(Flow::Trap("evaluator close argument mismatch".into())),
    }
}

fn expect_resource_kind(
    arguments: &[EvalValue],
    expected: ResourceKind,
) -> std::result::Result<EvalResource, Flow> {
    let resource = expect_resource(arguments)?;
    if resource.kind() == expected {
        Ok(resource)
    } else {
        Err(Flow::Trap("evaluator resource kind mismatch".into()))
    }
}
