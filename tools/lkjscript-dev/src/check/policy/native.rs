//! Typed observation adapter. All extension/shebang decisions belong to native meaning.
use crate::error::DevError;
use base64::Engine;
use lkjscript::platform::{ExecutionControl, JsonLimits, native_tool::PureTool};
use serde::Serialize;
use std::sync::OnceLock;

const ARTIFACT: &[u8] = include_bytes!("../../../../native-policy/generated/policy.lkja");
static PROGRAM: OnceLock<Result<PureTool, String>> = OnceLock::new();

pub(super) fn extensions(values: &[&str]) -> Result<Vec<bool>, DevError> {
    classify("extensions", values)
}

pub(super) fn shebangs(values: &[Vec<u8>]) -> Result<Vec<bool>, DevError> {
    let observations = values.iter().map(|value| {
        serde_json::json!({"$bytes": base64::engine::general_purpose::STANDARD.encode(value)})
    }).collect::<Vec<_>>();
    classify("shebangs", &observations)
}

fn classify<T: Serialize>(target: &str, observations: &[T]) -> Result<Vec<bool>, DevError> {
    let program = PROGRAM
        .get_or_init(|| {
            PureTool::load(ARTIFACT, &ExecutionControl::uncancelled())
                .map_err(|error| error.to_string())
        })
        .as_ref()
        .map_err(|error| DevError::infrastructure(format!("prepare native policy: {error}")))?;
    let input = serde_json::to_vec(&(observations,)).map_err(|error| {
        DevError::infrastructure(format!("encode native policy observations: {error}"))
    })?;
    let output = program
        .run_json(
            target,
            &input,
            JsonLimits::default(),
            &ExecutionControl::uncancelled(),
        )
        .map_err(|error| DevError::infrastructure(format!("execute native policy: {error}")))?;
    let decisions: Vec<bool> = serde_json::from_slice(&output).map_err(|error| {
        DevError::infrastructure(format!("decode native policy decisions: {error}"))
    })?;
    if decisions.len() != observations.len() {
        return Err(DevError::infrastructure(
            "native policy must return exactly one decision per observation",
        ));
    }
    Ok(decisions)
}

#[cfg(test)]
mod tests;
