//! Explicit process-local project authority for finite public command adapters.

use super::*;
use crate::platform::MemProjectStore;
use std::cell::RefCell;

thread_local! {
    static PROJECT: RefCell<Option<MemProjectStore>> = const { RefCell::new(None) };
}

pub(super) fn current() -> Option<MemProjectStore> {
    PROJECT.with(|project| project.borrow().clone())
}

pub(super) fn with_project<T>(project: Option<MemProjectStore>, action: impl FnOnce() -> T) -> T {
    struct Restore(Option<MemProjectStore>);
    impl Drop for Restore {
        fn drop(&mut self) {
            PROJECT.with(|project| *project.borrow_mut() = self.0.take());
        }
    }
    let previous = PROJECT.with(|slot| slot.replace(project));
    let _restore = Restore(previous);
    action()
}

/// Execute a finite public command against an explicit memory project. Graph reads,
/// publication and compilation caches stay in the arena. Explicit input/output file
/// options retain their ordinary meaning; deployment and process lifecycles require
/// their own public adapters. Clones share authority; `fork` creates an isolated head.
pub fn execute_in_memory(
    project: &MemProjectStore,
    arguments: Vec<String>,
) -> Result<Vec<u8>, Vec<Diagnostic>> {
    if arguments.iter().any(|argument| argument == "--project") {
        return Err(single_diagnostic(usage_error(
            "an explicit memory project cannot also select --project",
        )));
    }
    let command = arguments
        .first()
        .map(String::as_str)
        .unwrap_or("")
        .to_owned();
    if arguments.iter().any(|argument| argument == "--deployment") {
        return Err(single_diagnostic(usage_error(
            "memory project commands do not select deployment lifecycles",
        )));
    }
    with_project(Some(project.clone()), || match command.as_str() {
        "status" => execute_status(arguments).map_err(single_diagnostic),
        "inspect" => execute_inspect(arguments).map_err(single_diagnostic),
        "query" => execute_query(arguments).map_err(single_diagnostic),
        "change" => execute_change(arguments),
        "tx" => execute_tx(arguments),
        "gc" => execute_gc(arguments).map_err(single_diagnostic),
        "check" => execute_check(arguments).map_err(single_diagnostic),
        "build" => execute_build(arguments).map_err(single_diagnostic),
        "run" => execute_run(arguments).map_err(single_diagnostic),
        "package" => execute_package_builtin(arguments).map_err(single_diagnostic),
        "capabilities" => execute_capabilities(&arguments[1..]).map_err(single_diagnostic),
        _ => Err(single_diagnostic(usage_error(
            "memory project commands are status, inspect, query, change, tx, gc, check, build, run, package and capabilities",
        ))),
    })
}
