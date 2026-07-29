#![allow(clippy::panic)]

use lkjscript_executable::{
    ExecutableInstaller, ExecutableLimitKind, ExecutableLimits, InstallError, InvocationError,
    InvocationOutcome, NativeInvocationConfig,
};
use lkjscript_native::{
    encode, BackendLimits, EncodingConfig, F64Comparison, FunctionId, I64Comparison,
    ImageContracts, InstallableImage, MachinePlanBuilder, NativeValue, RuntimeCallSlot, Signature,
    SourceFunctionId, TrapCode, ValueType,
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

mod calls;
mod concurrent;
mod control;
mod declarations;
mod limits;
mod numeric;
mod numeric_tests;
mod outcomes;
mod traps;

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
