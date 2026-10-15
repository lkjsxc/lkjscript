//! Host semantic execution reuses public adapters and their process-wide immutable caches.
//! Process behavior and release candidates always retain the executable boundary.

use super::copy_executable;
use lkjscript::platform::contract::MAXIMUM_CLI_RESPONSE_BYTES;
use lkjscript::platform::contract::{
    MAXIMUM_CLI_RESPONSE_RECORDS, diagnostic_class_name, exit_status_for,
};
use lkjscript::platform::control::{CompactResponseLimits, CompactResponseWriter};
use lkjscript::platform::{self, Diagnostic};
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Output;
use std::sync::OnceLock;

const RELEASE_CANDIDATE_ENVIRONMENT: &str = "LKJSCRIPT_RELEASE_CANDIDATE";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Mode {
    HostSemantic,
    CandidateProcess,
    BoundaryProcess,
}

impl Mode {
    pub fn selected() -> Self {
        Self::for_candidate(std::env::var_os(RELEASE_CANDIDATE_ENVIRONMENT).is_some())
    }

    pub fn process() -> Self {
        if Self::selected() == Self::CandidateProcess {
            Self::CandidateProcess
        } else {
            Self::BoundaryProcess
        }
    }

    pub fn for_candidate(candidate: bool) -> Self {
        if candidate {
            Self::CandidateProcess
        } else {
            Self::HostSemantic
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::HostSemantic => "host-semantic",
            Self::CandidateProcess => "candidate-process",
            Self::BoundaryProcess => "boundary-process",
        }
    }
}

/// The shared executable is only used by host process-boundary observations. It is
/// a closed, verified immutable copy, independent of Cargo replacing its output.
pub fn executable(root: &Path, mode: Mode, source: &Path) -> PathBuf {
    if mode != Mode::HostSemantic {
        let executable = root.join("lkjscript");
        copy_executable(source, &executable);
        return executable;
    }
    static HOST: OnceLock<(tempfile::TempDir, PathBuf)> = OnceLock::new();
    HOST.get_or_init(|| {
        let root = tempfile::tempdir().unwrap();
        let executable = root.path().join("lkjscript");
        copy_executable(Path::new(env!("CARGO_BIN_EXE_lkjscript")), &executable);
        #[cfg(unix)]
        std::fs::set_permissions(&executable, std::fs::Permissions::from_mode(0o555)).unwrap();
        (root, executable)
    })
    .1
    .clone()
}

pub struct Response {
    pub code: Option<i32>,
    pub success: bool,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

impl From<Output> for Response {
    fn from(output: Output) -> Self {
        Self {
            code: output.status.code(),
            success: output.status.success(),
            stdout: output.stdout,
            stderr: output.stderr,
        }
    }
}

/// Accept only known finite routes with absolute external paths. The process
/// fallback owns relative-path resolution, environment, signals and output I/O;
/// this adapter never mutates the test process's current directory or environment.
pub fn execute(arguments: &[&str], full_arguments: Vec<String>) -> Option<Response> {
    let full = full_arguments
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    if !absolute_external_paths(arguments) || !absolute_external_paths(&full) {
        return None;
    }
    let operation = *arguments.first()?;
    let project_independent = matches!(operation, "new" | "capabilities")
        || arguments.starts_with(&["package", "builtin"])
        || (operation == "run" && arguments.contains(&"--deployment"));
    if !project_independent && !full.windows(2).any(|pair| pair[0] == "--project") {
        return None;
    }
    let single = |result: Result<Vec<u8>, Diagnostic>| result.map_err(|error| vec![error]);
    let result = match operation {
        "new" => single(platform::execute_new(
            &arguments[1..]
                .iter()
                .map(|s| (*s).to_owned())
                .collect::<Vec<_>>(),
        )),
        "capabilities" => single(platform::execute_capabilities(
            &arguments[1..]
                .iter()
                .map(|s| (*s).to_owned())
                .collect::<Vec<_>>(),
        )),
        "status" if arguments == ["status"] => single(platform::execute_status(full_arguments)),
        "inspect" => single(platform::execute_inspect(full_arguments)),
        "query" => single(platform::execute_query(full_arguments)),
        "change" => platform::execute_change(full_arguments),
        "check" => single(platform::execute_check(full_arguments)),
        "build" => single(platform::execute_build(full_arguments)),
        "run" => single(run(full_arguments)),
        "package" => single(platform::execute_package_builtin(full_arguments)),
        _ => return None,
    };
    Some(match result {
        Ok(stdout) => Response {
            code: Some(0),
            success: true,
            stdout,
            stderr: Vec::new(),
        },
        Err(diagnostics) => failure(operation, &diagnostics),
    })
}

fn run(arguments: Vec<String>) -> Result<Vec<u8>, Diagnostic> {
    // Match the executable's dispatch order: either batch flag selects the batch parser,
    // including malformed batches; deployment otherwise selects the foreground parser.
    if arguments.iter().any(|argument| {
        matches!(
            argument.as_str(),
            "--batch-input-file" | "--batch-result-file"
        )
    }) {
        let options = platform::cli::parse_run_batch(&arguments)?;
        return owned_runtime(move |runtime| {
            runtime.block_on(platform::cli::execute_run_batch(
                options,
                std::future::pending(),
            ))
        });
    }
    if arguments.iter().any(|argument| argument == "--deployment") {
        let options = platform::parse_foreground_run(&arguments)?;
        return owned_runtime(move |runtime| {
            runtime.block_on(platform::execute_foreground_run(
                options,
                std::future::pending(),
            ))
        });
    }
    platform::execute_run(arguments)
}

fn owned_runtime(
    operation: impl FnOnce(tokio::runtime::Runtime) -> Result<Vec<u8>, Diagnostic> + Send,
) -> Result<Vec<u8>, Diagnostic> {
    // A joined thread also permits callers already inside a Tokio runtime. The public
    // operation owns cancellation, blocking-task joins and executor cleanup; dropping
    // this runtime waits for its remaining blocking work before the adapter returns.
    std::thread::scope(|scope| {
        scope
            .spawn(move || {
                let runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .map_err(|error| {
                        Diagnostic::new(
                            platform::DiagnosticClass::Infrastructure,
                            "runtime_initialize",
                            format!("host semantic runtime could not initialize: {error}"),
                        )
                    })?;
                operation(runtime)
            })
            .join()
            .unwrap()
    })
}

fn absolute_external_paths(arguments: &[&str]) -> bool {
    if arguments.first() == Some(&"new")
        && arguments
            .get(1)
            .is_none_or(|path| !Path::new(path).is_absolute())
    {
        return false;
    }
    for pair in arguments.windows(2) {
        if matches!(
            pair[0],
            "--project"
                | "--input-file"
                | "--output"
                | "--deployment"
                | "--arguments-file"
                | "--result-file"
                | "--batch-input-file"
                | "--batch-result-file"
        ) && !Path::new(pair[1]).is_absolute()
        {
            return false;
        }
    }
    true
}

pub fn failure(command: &str, diagnostics: &[Diagnostic]) -> Response {
    // Keep the same bounded compact diagnostic projection as the executable.
    // The parity control compares these exact bytes against the real boundary.
    let infrastructure = exit_status_for(platform::DiagnosticClass::Infrastructure);
    let (code, stdout) = match diagnostics.first() {
        Some(first) => match failure_bytes(command, diagnostics) {
            Ok(stdout) => (
                match first.code.as_str() {
                    "change_stale_base" | "change_authored_stale_base" => 7,
                    _ => exit_status_for(first.class),
                },
                stdout,
            ),
            Err(_) => (infrastructure, Vec::new()),
        },
        None => (infrastructure, Vec::new()),
    };
    Response {
        code: Some(i32::from(code)),
        success: false,
        stdout,
        stderr: Vec::new(),
    }
}

fn failure_bytes(command: &str, diagnostics: &[Diagnostic]) -> Result<Vec<u8>, ()> {
    let mut output = CompactResponseWriter::new(CompactResponseLimits {
        maximum_bytes: MAXIMUM_CLI_RESPONSE_BYTES,
        maximum_records: MAXIMUM_CLI_RESPONSE_RECORDS,
    })
    .map_err(|_| ())?;
    output
        .append_record("result", &[("status", "failure"), ("command", command)])
        .map_err(|_| ())?;
    for error in diagnostics.iter().take(64) {
        let mut fields = vec![
            ("class", diagnostic_class_name(error.class).to_owned()),
            ("code", error.code.clone()),
            ("message", error.message.clone()),
        ];
        if let Some(location) = &error.location {
            fields.extend([
                ("path", location.path.clone()),
                ("line", location.line.to_string()),
                ("column", location.column.to_string()),
            ]);
        }
        if !error.notes.is_empty() {
            fields.push(("notes", serde_json::to_string(&error.notes).unwrap()));
        }
        output
            .append_record(
                "diagnostic",
                &fields
                    .iter()
                    .map(|(name, value)| (*name, value.as_str()))
                    .collect::<Vec<_>>(),
            )
            .map_err(|_| ())?;
    }
    if diagnostics.len() > 64 {
        output
            .append_record(
                "summary",
                &[
                    ("diagnostics", &diagnostics.len().to_string()),
                    ("omitted", &(diagnostics.len() - 64).to_string()),
                ],
            )
            .map_err(|_| ())?;
    }
    Ok(output.finish())
}
