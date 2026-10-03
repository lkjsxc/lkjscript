//! A policy witness, not a frozen bytecode cost for one allocation of field identities.
//! Nominal initializer order can require ordinary Let bindings when it differs from
//! canonical field order. Calibrate the same immutable artifact, then independently
//! require its result, linear work, bounded live state, and the former quota crossing.
use super::{DevError, LONG_LOOP, LoopCell, require, validate_loop};
use serde_json::Value;

const FORMER_INSTRUCTION_LIMIT: u64 = 10_000_000;
const CALIBRATION_ITERATIONS: u64 = 1000;

fn observation(cell: &LoopCell) -> Result<Value, DevError> {
    let text = cell
        .execution
        .get("production-observation")
        .ok_or_else(|| DevError::corrupt("loop production observation absent"))?;
    Ok(serde_json::from_str(text)?)
}

fn counter(observation: &Value, field: &str) -> Result<u64, DevError> {
    observation
        .get(field)
        .and_then(Value::as_u64)
        .ok_or_else(|| DevError::corrupt(format!("loop counter {field} is absent or not a u64")))
}

pub(super) fn validate_series(cells: &[LoopCell]) -> Result<(), DevError> {
    let [zero, thousand, long, bounded] = cells else {
        return Err(DevError::corrupt("foreground policy cells missing"));
    };
    for (cell, (label, n, is_bounded)) in cells.iter().zip([
        ("calibration-zero", 0, false),
        ("calibration-thousand", 1000, false),
        ("past-ten-million", LONG_LOOP, false),
        ("bounded-equivalent", 1000, true),
    ]) {
        require(
            (cell.label.as_str(), cell.n, cell.bounded) == (label, n, is_bounded),
            "foreground loop workload substituted",
        )?;
        validate_loop(cell)?;
    }
    let observations = [
        observation(zero)?,
        observation(thousand)?,
        observation(long)?,
        observation(bounded)?,
    ];
    let [zero, thousand, long, bounded] = &observations;
    let base = counter(zero, "instructions")?;
    let calibration = counter(thousand, "instructions")?;
    require(
        base > 0 && base < calibration && calibration <= FORMER_INSTRUCTION_LIMIT,
        &format!("loop calibration must remain below its explicit quota: {base}, {calibration}"),
    )?;
    let difference = calibration - base;
    require(
        difference.is_multiple_of(CALIBRATION_ITERATIONS),
        "loop calibration does not describe integral per-iteration work",
    )?;
    let stride = difference / CALIBRATION_ITERATIONS;
    let expected = stride
        .checked_mul(LONG_LOOP as u64)
        .and_then(|work| base.checked_add(work))
        .ok_or_else(|| DevError::corrupt("loop calibrated work overflows u64"))?;
    let observed = counter(long, "instructions")?;
    require(
        observed == expected && observed > FORMER_INSTRUCTION_LIMIT,
        &format!(
            "loop must preserve calibrated linear work and cross the former quota: expected {expected}, observed {observed}"
        ),
    )?;
    require(
        counter(bounded, "instructions")? == calibration,
        "bounded and unbounded executions of the same loop perform different work",
    )?;
    for field in [
        "maximum_call_depth",
        "maximum_control_frames",
        "maximum_live_allowances",
        "maximum_live_locals",
        "maximum_live_type_bindings",
        "maximum_value_stack",
    ] {
        let live = counter(thousand, field)?;
        require(
            counter(zero, field)? <= live
                && counter(long, field)? == live
                && counter(bounded, field)? == live,
            &format!("loop live-state counter {field} grows with lifetime work or quota mode"),
        )?;
    }
    for (cell, observation) in cells.iter().zip(&observations) {
        require(
            counter(observation, "tail_transfers")? == cell.n as u64 + 1,
            "loop tail transfers do not match the independently known iteration count",
        )?;
        for field in [
            "live_call_frames_after",
            "live_handles_after",
            "live_locals_after",
            "live_operands_after",
            "live_transactions_after",
            "live_type_bindings_after",
            "maximum_live_transactions",
            "parallel_scopes",
            "parallel_worker_dispatches",
            "parallel_inline_fallbacks",
        ] {
            require(
                counter(observation, field)? == 0,
                &format!("loop counter {field} retains state or performs unexpected work"),
            )?;
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "foreground_policy_tests.rs"]
mod tests;
