//! One cancellable maintenance operation over the ordinary repository authority.

use super::*;

pub fn execute_gc(arguments: Vec<String>) -> Result<Vec<u8>, Diagnostic> {
    let project = memory::current();
    crate::platform::stack::run("repository-compaction", move || {
        memory::with_project(project, || execute(arguments))
    })
    .map_err(|error| internal_error(format!("cannot execute compaction: {error}")))?
}

fn execute(arguments: Vec<String>) -> Result<Vec<u8>, Diagnostic> {
    let (arguments, project) = extract_global_project(arguments)?;
    if arguments.first().map(String::as_str) != Some("gc") {
        return Err(usage_error("compaction dispatch requires gc"));
    }
    ensure_options(&arguments[1..], &[], &[])?;
    let repository = open_normalized_repository(project)?;
    let result = repository.collect_garbage(&ExecutionControl::uncancelled())?;
    let mut bytes = serde_json::to_vec(&result)
        .map_err(|error| internal_error(format!("cannot encode compaction receipt: {error}")))?;
    bytes.push(b'\n');
    Ok(bytes)
}
