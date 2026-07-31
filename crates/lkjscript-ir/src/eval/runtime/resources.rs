use lkjscript_core::{CapabilityKind, ResourceKind, StructuralKind};

use super::super::*;

impl Evaluator<'_> {
    pub(crate) fn runtime_resources(
        &mut self,
        operation: RuntimeOp,
        arguments: Vec<&EvalValue>,
        result_type: &crate::SsaType,
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
                    Ok(()) => self.allocate_result(result_type, EvalValue::Bool(false), true),
                    Err(message) => self.system_error(result_type, &message),
                }
            }
            Op::SysOpenRead => {
                expect_capability_path(&arguments, CapabilityKind::FileSystem)?;
                self.acquire_resource_result(ResourceKind::FileReader, result_type)
            }
            Op::SysOpenWrite | Op::SysOpenCreateNew => {
                expect_capability_path(&arguments, CapabilityKind::FileSystem)?;
                self.acquire_resource_result(ResourceKind::FileWriter, result_type)
            }
            Op::SysOpenAppend => {
                expect_capability_path(&arguments, CapabilityKind::FileSystem)?;
                self.acquire_resource_result(ResourceKind::FileAppender, result_type)
            }
            Op::SysOpenDir => {
                expect_capability_path(&arguments, CapabilityKind::FileSystem)?;
                self.acquire_resource_result(ResourceKind::Directory, result_type)
            }
            Op::SysClose => {
                let resource = expect_resource(&arguments)?;
                let result = self.resources.close_configured(resource);
                self.unit_resource_result(result, result_type)
            }
            Op::SysSqliteOpen => {
                expect_sqlite_open_arguments(&arguments)?;
                self.acquire_resource_result(ResourceKind::SqliteConnection, result_type)
            }
            Op::SysSqlitePrepare => {
                let connection = expect_sqlite_prepare_arguments(&arguments)?;
                let result = self
                    .resources
                    .prepare_statement_configured(&connection)
                    .map(EvalValue::Resource);
                self.resource_result(result, result_type)
            }
            Op::SysSqliteClose => {
                let resource = expect_resource_kind(&arguments, ResourceKind::SqliteConnection)?;
                let result = self.resources.close_sqlite_connection(resource);
                self.unit_resource_result(result, result_type)
            }
            Op::SysSqliteFinalize => {
                let resource = expect_resource_kind(&arguments, ResourceKind::SqliteStatement)?;
                let result = self.resources.finalize_statement(resource);
                self.unit_resource_result(result, result_type)
            }
            _ => unreachable!("runtime operation dispatched to wrong resource family"),
        }
    }

    fn acquire_resource_result(
        &mut self,
        kind: ResourceKind,
        result_type: &crate::SsaType,
    ) -> std::result::Result<EvalValue, Flow> {
        let result = self
            .resources
            .acquire_configured(kind)
            .map(EvalValue::Resource);
        self.resource_result(result, result_type)
    }

    fn unit_resource_result(
        &mut self,
        result: Result<(), String>,
        result_type: &crate::SsaType,
    ) -> std::result::Result<EvalValue, Flow> {
        self.resource_result(result.map(|()| EvalValue::Unit), result_type)
    }

    fn resource_result(
        &mut self,
        result: Result<EvalValue, String>,
        result_type: &crate::SsaType,
    ) -> std::result::Result<EvalValue, Flow> {
        match result {
            Ok(value) => self.allocate_result(result_type, value, true),
            Err(message) => self.system_error(result_type, &message),
        }
    }

    fn system_error(
        &mut self,
        result_type: &crate::SsaType,
        message: &str,
    ) -> std::result::Result<EvalValue, Flow> {
        self.allocate_system_error(result_type, crate::prelude_contract::SYSTEM_IO_ID, message)
    }
}

fn expect_capability(
    arguments: &[&EvalValue],
    expected: CapabilityKind,
) -> std::result::Result<(), Flow> {
    match arguments {
        [EvalValue::Capability(actual)] if *actual == expected => Ok(()),
        _ => Err(Flow::Trap("evaluator resource capability mismatch".into())),
    }
}

fn expect_capability_path(
    arguments: &[&EvalValue],
    expected: CapabilityKind,
) -> std::result::Result<(), Flow> {
    match arguments {
        [EvalValue::Capability(actual), path]
            if *actual == expected && is_structural_kind(path, StructuralKind::Path) =>
        {
            Ok(())
        }
        _ => Err(Flow::Trap(
            "evaluator resource open arguments mismatch".into(),
        )),
    }
}

fn expect_sqlite_open_arguments(arguments: &[&EvalValue]) -> std::result::Result<(), Flow> {
    match arguments {
        [EvalValue::Capability(CapabilityKind::Sqlite), path, EvalValue::I64(_)]
            if is_structural_kind(path, StructuralKind::Path) =>
        {
            Ok(())
        }
        _ => Err(Flow::Trap(
            "evaluator SQLite open arguments mismatch".into(),
        )),
    }
}

fn expect_sqlite_prepare_arguments(
    arguments: &[&EvalValue],
) -> std::result::Result<EvalResource, Flow> {
    match arguments {
        [EvalValue::Resource(resource), text]
            if resource.kind() == ResourceKind::SqliteConnection
                && (matches!(text, EvalValue::StaticString(_))
                    || is_structural_kind(text, StructuralKind::String)) =>
        {
            Ok(resource.clone())
        }
        _ => Err(Flow::Trap(
            "evaluator SQLite prepare arguments mismatch".into(),
        )),
    }
}

fn expect_resource(arguments: &[&EvalValue]) -> std::result::Result<EvalResource, Flow> {
    match arguments {
        [EvalValue::Resource(resource)] => Ok(resource.clone()),
        _ => Err(Flow::Trap("evaluator close argument mismatch".into())),
    }
}

fn expect_resource_kind(
    arguments: &[&EvalValue],
    expected: ResourceKind,
) -> std::result::Result<EvalResource, Flow> {
    let resource = expect_resource(arguments)?;
    (resource.kind() == expected)
        .then_some(resource)
        .ok_or_else(|| Flow::Trap("evaluator resource kind mismatch".into()))
}

fn is_structural_kind(value: &EvalValue, expected: StructuralKind) -> bool {
    match value {
        EvalValue::StructuralOwner(owner) => owner.value_type.kind == expected,
        EvalValue::StructuralView(view) => view.value_type.kind == expected,
        EvalValue::Path(_) => expected == StructuralKind::Path,
        _ => false,
    }
}
