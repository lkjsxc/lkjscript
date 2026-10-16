//! Explicit finite semantic routes retain distinct host and process outcomes.

use crate::semantic_cli;
use std::cell::Cell;
use std::process::{ExitStatus, Output};

thread_local! {
    static FINITE_SEMANTICS: Cell<bool> = const { Cell::new(false) };
}

pub(super) struct Scope(bool);

impl Scope {
    pub(super) fn enter() -> Self {
        Self(FINITE_SEMANTICS.replace(true))
    }
}

impl Drop for Scope {
    fn drop(&mut self) {
        FINITE_SEMANTICS.set(self.0);
    }
}

#[derive(Debug, Eq, PartialEq)]
pub(super) enum Status {
    Process(ExitStatus),
    Semantic { code: Option<i32>, success: bool },
}

impl Status {
    pub(super) fn success(&self) -> bool {
        match self {
            Self::Process(status) => status.success(),
            Self::Semantic { success, .. } => *success,
        }
    }

    pub(super) fn code(&self) -> Option<i32> {
        match self {
            Self::Process(status) => status.code(),
            Self::Semantic { code, .. } => *code,
        }
    }
}

#[derive(Debug)]
pub(super) struct ObservedOutput {
    pub status: Status,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

impl From<Output> for ObservedOutput {
    fn from(output: Output) -> Self {
        Self {
            status: Status::Process(output.status),
            stdout: output.stdout,
            stderr: output.stderr,
        }
    }
}

impl From<semantic_cli::Response> for ObservedOutput {
    fn from(output: semantic_cli::Response) -> Self {
        Self {
            status: Status::Semantic {
                code: output.code,
                success: output.success,
            },
            stdout: output.stdout,
            stderr: output.stderr,
        }
    }
}

pub(super) fn execute(arguments: &[&str]) -> Option<ObservedOutput> {
    execute_for_mode(semantic_cli::Mode::selected(), arguments)
}

fn execute_for_mode(mode: semantic_cli::Mode, arguments: &[&str]) -> Option<ObservedOutput> {
    if !FINITE_SEMANTICS.get() || mode != semantic_cli::Mode::HostSemantic {
        return None;
    }
    // Preserve all original arguments for the actual public parser. Only select
    // the finite adapter here; malformed/implicit paths keep their process route.
    let operation = if arguments.first() == Some(&"--project") {
        arguments.get(2..)?
    } else {
        arguments
    };
    semantic_cli::execute(
        operation,
        arguments
            .iter()
            .map(|argument| (*argument).to_owned())
            .collect(),
    )
    .map(ObservedOutput::from)
}

#[test]
fn finite_scope_keeps_candidate_processes_and_implicit_paths_outside_host_execution() {
    let command = ["capabilities"];
    assert!(execute_for_mode(semantic_cli::Mode::HostSemantic, &command).is_none());
    {
        let _scope = Scope::enter();
        let output = execute_for_mode(semantic_cli::Mode::HostSemantic, &command).unwrap();
        assert!(matches!(
            output.status,
            Status::Semantic {
                code: Some(0),
                success: true
            }
        ));
        for mode in [
            semantic_cli::Mode::CandidateProcess,
            semantic_cli::Mode::BoundaryProcess,
        ] {
            assert!(execute_for_mode(mode, &command).is_none());
        }
        for arguments in [
            &["status"][..],
            &["--project", "relative", "status"],
            &["serve"],
        ] {
            assert!(execute_for_mode(semantic_cli::Mode::HostSemantic, arguments).is_none());
        }
    }
    assert!(execute_for_mode(semantic_cli::Mode::HostSemantic, &command).is_none());
}
