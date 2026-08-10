use std::io::{self, Write};
use std::path::Path;
use std::process::ExitCode;

use lkjscript_compiler::{PackageCompileError, SourceDiagnostic, SourcePosition, SourceSpan};
use lkjscript_core::{Error, ErrorClass};
use serde::Serialize;

const USAGE: &str = "usage: lkjscript check <file.lkjscript> [--json]";
const SCHEMA: &str = "lkjscript.check";

pub fn command(args: &[String]) -> Result<ExitCode, String> {
    let request = parse(args)?;
    match lkjscript_compiler::compile_package_path(Path::new(request.file)) {
        Ok(_) => {
            if request.json {
                emit(&SuccessDocument {
                    schema: SCHEMA,
                    status: "ok",
                })?;
            }
            Ok(ExitCode::SUCCESS)
        }
        Err(error) => {
            if request.json {
                emit_failure(&error)?;
            } else {
                render_human(&error);
            }
            Ok(ExitCode::from(1))
        }
    }
}

struct Request<'a> {
    file: &'a str,
    json: bool,
}

fn parse(args: &[String]) -> Result<Request<'_>, String> {
    match args {
        [command, file] if command == "check" && !file.starts_with("--") => {
            Ok(Request { file, json: false })
        }
        [command, file, flag]
            if command == "check" && !file.starts_with("--") && flag == "--json" =>
        {
            Ok(Request { file, json: true })
        }
        [command, option, ..] if command == "check" && option == "--json" => Err(USAGE.into()),
        [command, _, option, ..] if command == "check" && option == "--json" => Err(USAGE.into()),
        [command, option, ..] if command == "check" && option.starts_with("--") => {
            Err(format!("unknown check option: {option}"))
        }
        [command, _, option, ..] if command == "check" && option.starts_with("--") => {
            Err(format!("unknown check option: {option}"))
        }
        _ => Err(USAGE.into()),
    }
}

fn render_human(error: &PackageCompileError) {
    match error {
        PackageCompileError::Incomplete(error) => {
            eprintln!("{error}");
            for blocker in &error.blockers {
                eprintln!("  blocker: {}", blocker.kind());
            }
        }
        _ => eprintln!("{error}"),
    }
}

fn emit_failure(error: &PackageCompileError) -> Result<(), String> {
    match error {
        PackageCompileError::Source(diagnostic) => emit_source_failure(diagnostic),
        PackageCompileError::Package(error) => emit(&FailureDocument {
            schema: SCHEMA,
            status: "error",
            failure: BasicFailure {
                phase: "package",
                class: error_class(error),
                message: error.as_str(),
            },
        }),
        PackageCompileError::Incomplete(error) => {
            let mut blockers = Vec::new();
            blockers
                .try_reserve(error.blockers.len())
                .map_err(|_| "allocate lkjscript.check blockers".to_string())?;
            blockers.extend(error.blockers.iter().map(|blocker| BlockerDocument {
                kind: blocker.kind(),
            }));
            emit(&FailureDocument {
                schema: SCHEMA,
                status: "error",
                failure: IncompleteFailure {
                    phase: "incomplete",
                    message: error.to_string(),
                    blockers,
                },
            })
        }
        PackageCompileError::Compiler(error) => emit(&FailureDocument {
            schema: SCHEMA,
            status: "error",
            failure: BasicFailure {
                phase: "compiler",
                class: error_class(error),
                message: error.as_str(),
            },
        }),
    }
}

fn emit_source_failure(diagnostic: &SourceDiagnostic) -> Result<(), String> {
    let mut related = Vec::new();
    related
        .try_reserve(diagnostic.related_spans().len())
        .map_err(|_| "allocate lkjscript.check related locations".to_string())?;
    related.extend(
        diagnostic
            .related_spans()
            .iter()
            .map(|related| RelatedDocument {
                message: related.label(),
                path: related.origin().map(|origin| origin.logical_path()),
                range: related.span().map(range),
            }),
    );
    let primary = diagnostic.primary_span();
    emit(&FailureDocument {
        schema: SCHEMA,
        status: "error",
        failure: SourceFailure {
            phase: "source",
            class: (diagnostic.code() == "LKJ-SRC-HOST").then_some("host"),
            code: diagnostic.code(),
            severity: diagnostic.severity().as_str(),
            category: diagnostic.category().as_str(),
            message: diagnostic.message(),
            path: diagnostic.origin().map(|origin| origin.logical_path()),
            range: primary.map(range),
            related,
        },
    })
}

fn emit(document: &impl Serialize) -> Result<(), String> {
    let json = serde_json::to_string(document)
        .map_err(|error| format!("encode {SCHEMA} result: {error}"))?;
    writeln!(io::stdout().lock(), "{json}")
        .map_err(|error| format!("write {SCHEMA} result: {error}"))
}

fn error_class(error: &Error) -> &'static str {
    match error.class() {
        ErrorClass::Ordinary => "error",
        ErrorClass::Deadline => "deadline",
        ErrorClass::Resource(_) => "resource",
        ErrorClass::BytecodePolicy => "bytecode-policy",
        ErrorClass::Host => "host",
    }
}

fn range(span: SourceSpan) -> RangeDocument {
    RangeDocument {
        start: position(span.start()),
        end: position(span.end()),
    }
}

const fn position(position: SourcePosition) -> PositionDocument {
    PositionDocument {
        line: position.line(),
        column: position.column(),
    }
}

#[derive(Serialize)]
struct SuccessDocument<'a> {
    schema: &'a str,
    status: &'a str,
}

#[derive(Serialize)]
struct FailureDocument<'a, T> {
    schema: &'a str,
    status: &'a str,
    failure: T,
}

#[derive(Serialize)]
struct BasicFailure<'a> {
    phase: &'a str,
    class: &'a str,
    message: &'a str,
}

#[derive(Serialize)]
struct IncompleteFailure<'a> {
    phase: &'a str,
    message: String,
    blockers: Vec<BlockerDocument<'a>>,
}

#[derive(Serialize)]
struct BlockerDocument<'a> {
    kind: &'a str,
}

#[derive(Serialize)]
struct SourceFailure<'a> {
    phase: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    class: Option<&'a str>,
    code: &'a str,
    severity: &'a str,
    category: &'a str,
    message: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    path: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    range: Option<RangeDocument>,
    related: Vec<RelatedDocument<'a>>,
}

#[derive(Serialize)]
struct RelatedDocument<'a> {
    message: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    path: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    range: Option<RangeDocument>,
}

#[derive(Clone, Copy, Serialize)]
struct RangeDocument {
    start: PositionDocument,
    end: PositionDocument,
}

#[derive(Clone, Copy, Serialize)]
struct PositionDocument {
    /// One-based line number.
    line: u64,
    /// One-based Unicode-scalar column number.
    column: u64,
}

#[cfg(test)]
mod tests {
    use super::parse;

    #[test]
    fn check_accepts_one_path_and_an_optional_trailing_json_flag() {
        assert!(parse(&["check".into(), "main.lkjscript".into()]).is_ok());
        assert!(parse(&["check".into(), "main.lkjscript".into(), "--json".into(),]).is_ok());

        assert!(parse(&["check".into()]).is_err());
        assert!(parse(&["check".into(), "main.lkjscript".into(), "argument".into(),]).is_err());
        assert!(parse(&["check".into(), "--json".into()]).is_err());
        assert!(parse(&["check".into(), "main.lkjscript".into(), "--unknown".into(),]).is_err());
    }
}
