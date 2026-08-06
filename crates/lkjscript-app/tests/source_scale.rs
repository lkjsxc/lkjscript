use std::fmt::Write;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use lkjscript_compiler::{compile_path, compile_source};
use lkjscript_core::{ExecutionConfig, ExecutionOutcome, Limits};
use lkjscript_vm::{run_chunk, ExecutionInputs};

static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(0);

struct TempDir(PathBuf);

impl TempDir {
    fn new() -> std::io::Result<Self> {
        let id = NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "lkjscript-wide-production-{}-{id}",
            std::process::id()
        ));
        fs::create_dir(&path)?;
        Ok(Self(path))
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn wide_source_directory_compiles_and_executes_through_the_generic_path(
) -> Result<(), Box<dyn std::error::Error>> {
    const UNRELATED_ENTRIES: usize = 1_500;
    let directory = TempDir::new()?;
    for index in 0..UNRELATED_ENTRIES {
        fs::write(directory.0.join(format!("asset-{index:04}")), [])?;
    }
    let source = directory.0.join("main.lkjscript");
    fs::write(
        &source,
        "main/\nsig/\ninputs/\n/inputs\noutput/\ni64\n/output\n/sig\n42\n/main\n",
    )?;

    let program = compile_path(&source, &Limits::default())?;
    let outcome = run_chunk(
        program.bytecode(),
        &ExecutionInputs::default(),
        &ExecutionConfig::default(),
    );
    let value = match outcome {
        ExecutionOutcome::Returned(value) => value,
        other => return Err(format!("wide-directory program did not return: {other:?}").into()),
    };
    assert_eq!(value.as_i64(), Some(42));
    Ok(())
}

const HELPER_DECLARATIONS: usize = 64;
const MAIN_DO_CHILDREN: usize = 128;
const EXPECTED_RESULT: i64 = 4_242;

const _: () = assert!(HELPER_DECLARATIONS + 1 > 8);
const _: () = assert!(MAIN_DO_CHILDREN > 16);

fn generated_flat_source() -> Result<String, std::fmt::Error> {
    let mut source = String::new();
    for index in 0..HELPER_DECLARATIONS {
        write!(
            source,
            "def/\nname/\nhelper-{index}\n/name\nfn/\nsig/\ninputs/\n/inputs\noutput/\nunit\n/output\n/sig\nparams/\n/params\nunit\n/fn\n/def\n"
        )?;
    }
    source.push_str("main/\nsig/\ninputs/\n/inputs\noutput/\ni64\n/output\n/sig\ndo/\n");
    for value in 0..MAIN_DO_CHILDREN - 1 {
        writeln!(source, "{value}")?;
    }
    writeln!(source, "{EXPECTED_RESULT}")?;
    source.push_str("/do\n/main\n");
    Ok(source)
}

#[test]
fn flat_source_beyond_former_quotas_compiles_validates_and_executes(
) -> Result<(), Box<dyn std::error::Error>> {
    let source = generated_flat_source()?;

    // Every non-empty line in this fixture is one lexer token. The geometry is
    // deliberately well beyond the former 384-token, 16-child, and 8-form ceilings.
    assert!(source.lines().count() > 384);

    // ExecutableProgram exposes bytecode only after compiler-side validation.
    let program = compile_source(&source, "source-scale.lkjscript", &Limits::default())?;
    let outcome = run_chunk(
        program.bytecode(),
        &ExecutionInputs::default(),
        &ExecutionConfig::default(),
    );
    let value = match outcome {
        ExecutionOutcome::Returned(value) => value,
        other => return Err(format!("flat source program did not return: {other:?}").into()),
    };
    assert_eq!(value.as_i64(), Some(EXPECTED_RESULT));
    Ok(())
}
