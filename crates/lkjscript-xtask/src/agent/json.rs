use std::fs::File;
use std::io::Read;
use std::path::Path;

use serde::de::DeserializeOwned;

use super::bounds;
use super::model::{CheckpointRequest, WorkState};

pub fn read_request(path: &Path) -> Result<CheckpointRequest, String> {
    let bytes = read_bounded(path, bounds::REQUEST_BYTES)?;
    let request: CheckpointRequest = decode(&bytes, path)?;
    bounds::request(&request, bytes.len())?;
    Ok(request)
}

pub fn read_state_bytes(bytes: &[u8], path: &Path) -> Result<WorkState, String> {
    bounds::output(bytes.len()).map_err(|error| format!("state {}: {error}", path.display()))?;
    let state: WorkState = decode(bytes, path)?;
    bounds::state(&state)?;
    Ok(state)
}

pub fn encode_state(state: &WorkState) -> Result<Vec<u8>, String> {
    bounds::state(state)?;
    let mut bytes =
        serde_json::to_vec_pretty(state).map_err(|error| format!("serialize state: {error}"))?;
    bytes.push(b'\n');
    bounds::output(bytes.len())?;
    Ok(bytes)
}

fn decode<T: DeserializeOwned>(bytes: &[u8], path: &Path) -> Result<T, String> {
    let mut decoder = serde_json::Deserializer::from_slice(bytes);
    let value = T::deserialize(&mut decoder)
        .map_err(|error| format!("parse {}: {error}", path.display()))?;
    decoder
        .end()
        .map_err(|error| format!("trailing JSON in {}: {error}", path.display()))?;
    Ok(value)
}

pub fn read_bounded(path: &Path, limit: usize) -> Result<Vec<u8>, String> {
    let metadata = path
        .symlink_metadata()
        .map_err(|error| format!("inspect {}: {error}", path.display()))?;
    if !metadata.is_file() {
        return Err(format!("{} is not a regular file", path.display()));
    }
    let size = usize::try_from(metadata.len()).map_err(|_| "input size overflow")?;
    if size > limit {
        return Err(format!("{} exceeds byte limit {limit}", path.display()));
    }
    let capacity = size.checked_add(1).ok_or("input allocation overflow")?;
    let mut bytes = Vec::with_capacity(capacity);
    File::open(path)
        .and_then(|file| file.take((limit as u64) + 1).read_to_end(&mut bytes))
        .map_err(|error| format!("read {}: {error}", path.display()))?;
    if bytes.len() > limit {
        return Err(format!("{} changed beyond byte limit", path.display()));
    }
    Ok(bytes)
}
