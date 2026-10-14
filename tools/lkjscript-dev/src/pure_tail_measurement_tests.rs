//! Explicit measurements over the same publicly authored pure-tail artifact.
//!
//! Set LKJSCRIPT_MEASURE_ARTIFACT to an absolute artifact path and
//! LKJSCRIPT_MEASURE_N to 256, 1024, 4096 or 8192. Run this ignored selector
//! serially in an externally bounded process on each compared runtime build.
//! Preserve the artifact and harness bytes, Cargo identity and all raw samples.
//! These timings do not replace public/reference acceptance or measure allocator calls.

use crate::error::DevError;
use lkjscript::platform::{Diagnostic, ExecutionControl, JsonLimits, native_tool::PureTool};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

struct Case {
    target: &'static str,
    arguments: Vec<u8>,
    expected: Vec<u8>,
}

impl Case {
    fn invoke(
        &self,
        tool: &PureTool,
        control: &ExecutionControl,
    ) -> Result<Vec<u8>, Box<Diagnostic>> {
        tool.run_json(
            self.target,
            std::hint::black_box(&self.arguments),
            JsonLimits::default(),
            control,
        )
        .map_err(Box::new)
    }

    fn report(
        &self,
        lane: &str,
        sample: u32,
        elapsed_nanoseconds: Option<u64>,
        result: Result<Vec<u8>, Box<Diagnostic>>,
    ) -> Result<(), DevError> {
        let mut record = json!({
            "stage":"invocation", "target":self.target, "lane":lane, "sample":sample,
            "elapsed_nanoseconds":elapsed_nanoseconds,
            "arguments_bytes":self.arguments.len(), "arguments_sha256":hash(&self.arguments),
            "expected_bytes":self.expected.len(), "expected_sha256":hash(&self.expected)
        });
        match result {
            Ok(actual) => {
                let equal = actual == self.expected;
                record["output_bytes"] = json!(actual.len());
                record["output_sha256"] = json!(hash(&actual));
                record["independent_output_equal"] = json!(equal);
                emit(&record)?;
                if !equal {
                    return Err(DevError::corrupt(format!(
                        "{} {lane} sample {sample} differs from the independent complete output",
                        self.target
                    )));
                }
            }
            Err(error) => {
                record["failure"] = json!(error);
                emit(&record)?;
                return Err(DevError::corrupt(format!(
                    "{} {lane} sample {sample}: {error}",
                    self.target
                )));
            }
        }
        Ok(())
    }
}

fn hash(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn file_hash(path: &Path) -> Result<String, DevError> {
    let mut file = std::fs::File::open(path)?;
    let mut digest = Sha256::new();
    let mut buffer = [0_u8; 65_536];
    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        digest.update(&buffer[..count]);
    }
    Ok(digest
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}

fn nanoseconds(duration: Duration) -> Result<u64, DevError> {
    u64::try_from(duration.as_nanos())
        .map_err(|_| DevError::infrastructure("measurement elapsed time overflowed"))
}

fn emit(record: &Value) -> Result<(), DevError> {
    let mut stdout = std::io::stdout().lock();
    serde_json::to_writer(&mut stdout, record)?;
    stdout.write_all(b"\n")?;
    stdout.flush()?;
    Ok(())
}

fn measure() -> Result<(), DevError> {
    let path = PathBuf::from(
        std::env::var_os("LKJSCRIPT_MEASURE_ARTIFACT")
            .ok_or_else(|| DevError::usage("measurement requires LKJSCRIPT_MEASURE_ARTIFACT"))?,
    );
    if !path.is_absolute() {
        return Err(DevError::usage(
            "measurement artifact path must be absolute",
        ));
    }
    let n: i64 = std::env::var("LKJSCRIPT_MEASURE_N")
        .map_err(|_| DevError::usage("measurement requires LKJSCRIPT_MEASURE_N"))?
        .parse()
        .map_err(|_| DevError::usage("measurement N must be an integer"))?;
    if ![256, 1024, 4096, 8192].contains(&n) {
        return Err(DevError::usage(
            "measurement N must be 256, 1024, 4096 or 8192",
        ));
    }
    // Read and hash outside every timer. Both runtime builds consume these exact bytes.
    let artifact = std::fs::read(&path)?;
    let executable = std::env::current_exe()?;
    emit(&json!({
        "kind":"pure-tool-measurement", "stage":"binding", "n":n,
        "artifact_path":path, "artifact_bytes":artifact.len(), "artifact_sha256":hash(&artifact),
        "harness_sha256":hash(include_bytes!("pure_tail_measurement_tests.rs")),
        "executable_path":executable, "executable_sha256":file_hash(&executable)?,
        "manifest_directory":env!("CARGO_MANIFEST_DIR"),
        "os":std::env::consts::OS, "arch":std::env::consts::ARCH,
        "debug_assertions":cfg!(debug_assertions),
        "execution":"production only; fresh control and invocation state; unchanged foreground policy",
        "timing":"in-memory strict load/preparation, first invocation, then one warmup and three reused-object invocations",
        "run_includes":"target selection, JSON ingress, VM, encoding and joined cleanup",
        "run_excludes":"file IO, input/expected construction, validation, reporting and final returned-output destruction",
        "allocation_and_reference_counters":"not supplied by PureTool; retain separate maintained pure-tail evidence",
        "environment_and_source_binding":"owned by the external source-bound executor; no speed threshold"
    }))?;
    let input: Vec<i64> = (0..n).collect();
    let expected: Vec<i64> = (0..n).map(|item| 3 * item + 5).collect();
    let cases = [
        Case {
            target: "map",
            arguments: serde_json::to_vec(&json!([input, 3, 5]))?,
            expected: serde_json::to_vec(&expected)?,
        },
        Case {
            target: "count",
            arguments: serde_json::to_vec(&json!([n]))?,
            expected: b"0".to_vec(),
        },
    ];
    for case in cases {
        emit(&json!({"stage":"started", "target":case.target, "lane":"cold"}))?;
        let load_control = ExecutionControl::uncancelled();
        let first_control = ExecutionControl::uncancelled();
        let cold_started = Instant::now();
        let loaded = PureTool::load(std::hint::black_box(&artifact), &load_control);
        let load_nanoseconds = nanoseconds(cold_started.elapsed())?;
        let tool = match loaded {
            Ok(tool) => tool,
            Err(error) => {
                emit(&json!({"stage":"load", "target":case.target,
                    "elapsed_nanoseconds":load_nanoseconds, "failure":error}))?;
                return Err(DevError::corrupt(format!(
                    "measurement artifact load: {error}"
                )));
            }
        };
        let first_started = Instant::now();
        let first = case.invoke(&tool, &first_control);
        let first_nanoseconds = nanoseconds(first_started.elapsed())?;
        let cold_nanoseconds = nanoseconds(cold_started.elapsed())?;
        emit(&json!({"stage":"load", "target":case.target,
            "elapsed_nanoseconds":load_nanoseconds}))?;
        emit(&json!({"stage":"load-plus-first", "target":case.target,
            "elapsed_nanoseconds":cold_nanoseconds,
            "independent_output_equal":first.as_ref().is_ok_and(|actual| actual.as_slice() == case.expected.as_slice())}))?;
        case.report("first", 0, Some(first_nanoseconds), first)?;
        // The same immutable loaded object is reused; each call owns fresh execution state.
        let control = ExecutionControl::uncancelled();
        case.report("warmup", 0, None, case.invoke(&tool, &control))?;
        for sample in 1..=3 {
            emit(&json!({"stage":"started", "target":case.target,
                "lane":"warm", "sample":sample}))?;
            let control = ExecutionControl::uncancelled();
            let started = Instant::now();
            let result = case.invoke(&tool, &control);
            let elapsed_nanoseconds = nanoseconds(started.elapsed())?;
            case.report("warm", sample, Some(elapsed_nanoseconds), result)?;
        }
    }
    emit(&json!({"stage":"complete", "n":n,
        "independent_outputs_validated":true, "performance_conclusion":null}))
}

#[test]
#[ignore = "requires exact external pure-tail artifact/N and exclusive bounded measurement execution"]
fn prepared_pure_tool_cold_and_warm_measurements() -> Result<(), DevError> {
    let result = measure();
    if let Err(error) = &result {
        emit(&json!({"kind":"pure-tool-measurement", "stage":"failed",
            "failure":{"kind":error.kind(), "message":error.message()}}))?;
    }
    result
}
