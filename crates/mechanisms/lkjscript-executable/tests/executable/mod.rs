#![allow(clippy::panic)]

use lkjscript_executable::{
    EnteredInvocationError, ExecutableInstaller, ExecutableLimitKind, ExecutableLimits,
    InstallError, InstalledImage, InvocationOutcome, InvocationReport, NativeInvocationConfig,
    NativeResourceLimitKind, NativeRuntimeServices, NoopNativeIslandRuntimeServices, PreEntryError,
};
use lkjscript_native::{
    encode, BackendLimits, EncodingConfig, F64Comparison, FunctionId, I64Comparison,
    ImageContracts, InstallableImage, MachinePlanBuilder, NativeExecutionDomain, NativeValue,
    RuntimeCallSlot, Signature, SourceFunctionId, TrapCode, ValueType,
};

#[derive(Clone, Copy)]

struct Entries {
    multi_block: FunctionId,
    loop_sum: FunctionId,
    checked_add: FunctionId,
    checked_sub: FunctionId,
    checked_mul: FunctionId,
    checked_div: FunctionId,
    f64_arithmetic: FunctionId,
    i64_to_f64: FunctionId,
    f64_branch: FunctionId,
    f64_comparisons: [FunctionId; 6],
    bool_not: FunctionId,
    bool_equal: FunctionId,
    direct_call: FunctionId,
    exit: FunctionId,
    unit: FunctionId,
    callee: FunctionId,
}

mod boundary;
mod calls;
mod concurrent;
mod control;
mod declarations;
mod limits;
mod numeric;
mod numeric_tests;
mod outcomes;
mod traps;

#[derive(Debug, Eq, PartialEq)]
enum TestInvocationError {
    PreEntry(PreEntryError),
    Entered(EnteredInvocationError),
}

impl std::fmt::Display for TestInvocationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PreEntry(error) => write!(formatter, "pre-entry: {error}"),
            Self::Entered(error) => write!(formatter, "entered: {error}"),
        }
    }
}

impl std::error::Error for TestInvocationError {}

#[derive(Default)]
struct TestRuntimeServices;

impl NativeRuntimeServices for TestRuntimeServices {}

trait TestInvoke {
    fn invoke(
        &self,
        entry: FunctionId,
        arguments: &[NativeValue],
    ) -> Result<InvocationOutcome, TestInvocationError>;

    fn invoke_with_config(
        &self,
        entry: FunctionId,
        arguments: &[NativeValue],
        config: &NativeInvocationConfig,
    ) -> Result<InvocationReport, TestInvocationError>;
}

impl TestInvoke for InstalledImage {
    fn invoke(
        &self,
        entry: FunctionId,
        arguments: &[NativeValue],
    ) -> Result<InvocationOutcome, TestInvocationError> {
        self.invoke_with_config(entry, arguments, &NativeInvocationConfig::unrestricted())
            .map(|report| report.outcome())
    }

    fn invoke_with_config(
        &self,
        entry: FunctionId,
        arguments: &[NativeValue],
        config: &NativeInvocationConfig,
    ) -> Result<InvocationReport, TestInvocationError> {
        match self.execution_domain() {
            NativeExecutionDomain::CollectorFree => {
                let mut services = NoopNativeIslandRuntimeServices;
                let prepared = self
                    .prepare_invocation(entry, arguments, config, &mut services)
                    .map_err(TestInvocationError::PreEntry)?;
                prepared.enter().map_err(TestInvocationError::Entered)
            }
            NativeExecutionDomain::InvocationRegion => {
                let mut services = TestRuntimeServices;
                let prepared = self
                    .prepare_region_invocation(entry, arguments, config, &mut services)
                    .map_err(TestInvocationError::PreEntry)?;
                prepared.enter().map_err(TestInvocationError::Entered)
            }
        }
    }
}

fn scalar_image(
    contracts: ImageContracts,
) -> Result<(InstallableImage, Entries), Box<dyn std::error::Error>> {
    let mut plan = MachinePlanBuilder::new();
    let entries = declarations::declare(&mut plan)?;
    control::define(&mut plan, entries)?;
    numeric::define(&mut plan, entries)?;
    calls::define(&mut plan, entries)?;
    let image = encode(
        plan.verify(BackendLimits::default())?,
        EncodingConfig::new(contracts),
    )?;
    Ok((image, entries))
}
