use std::fmt::Write as FmtWrite;
use std::fs;
use std::io::Write as IoWrite;
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

fn write_source_with_trailing_trivia(
    path: &std::path::Path,
    prefix: &str,
    total_bytes: usize,
) -> std::io::Result<()> {
    let trivia_bytes = total_bytes
        .checked_sub(prefix.len() + 3)
        .ok_or_else(|| std::io::Error::other("source target is smaller than its prefix"))?;
    let mut file = fs::File::create(path)?;
    file.write_all(prefix.as_bytes())?;
    file.write_all(b";;")?;
    let chunk = vec![b'x'; 64 * 1024];
    let mut remaining = trivia_bytes;
    while remaining != 0 {
        let write = remaining.min(chunk.len());
        file.write_all(&chunk[..write])?;
        remaining -= write;
    }
    file.write_all(b"\n")?;
    file.sync_all()
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

#[test]
fn trusted_source_above_16_mib_compiles_to_validated_bytecode_and_executes(
) -> Result<(), Box<dyn std::error::Error>> {
    const SOURCE_BYTES: usize = 16 * 1024 * 1024 + 1024;
    let directory = TempDir::new()?;
    let source = directory.0.join("main.lkjscript");
    let prefix = "main/\nsig/\ninputs/\n/inputs\noutput/\ni64\n/output\n/sig\n42\n/main\n";
    write_source_with_trailing_trivia(&source, prefix, SOURCE_BYTES)?;
    assert_eq!(fs::metadata(&source)?.len(), u64::try_from(SOURCE_BYTES)?);

    // ExecutableProgram exposes bytecode only after compiler-side validation.
    let program = compile_path(&source, &Limits::default())?;
    let outcome = run_chunk(
        program.bytecode(),
        &ExecutionInputs::default(),
        &ExecutionConfig::default(),
    );
    let value = match outcome {
        ExecutionOutcome::Returned(value) => value,
        other => return Err(format!("large trusted source did not return: {other:?}").into()),
    };
    assert_eq!(value.as_i64(), Some(42));
    Ok(())
}

#[test]
#[ignore = "opt-in 258 MiB aggregate source stress geometry"]
fn trusted_import_closure_above_256_mib_compiles_and_executes(
) -> Result<(), Box<dyn std::error::Error>> {
    const SOURCE_BYTES_PER_UNIT: usize = 129 * 1024 * 1024;
    let directory = TempDir::new()?;
    let root = directory.0.join("main.lkjscript");
    let library = directory.0.join("lib.lkjscript");
    let root_prefix = concat!(
        "imports/\nimport/\nmodule/\nlib.lkjscript\n/module\ndeclarations/\nf\n/declarations\n/import\n/imports\n",
        "main/\nsig/\ninputs/\n/inputs\noutput/\ni64\n/output\n/sig\nf/\n/f\n/main\n",
    );
    let library_prefix = concat!(
        "def/\nname/\nf\n/name\npublic\nfn/\nsig/\ninputs/\n/inputs\noutput/\ni64\n/output\n/sig\n",
        "params/\n/params\n42\n/fn\n/def\n",
    );
    write_source_with_trailing_trivia(&root, root_prefix, SOURCE_BYTES_PER_UNIT)?;
    write_source_with_trailing_trivia(&library, library_prefix, SOURCE_BYTES_PER_UNIT)?;
    assert!(fs::metadata(&root)?.len() + fs::metadata(&library)?.len() > 256 * 1024 * 1024);

    let program = compile_path(&root, &Limits::default())?;
    let outcome = run_chunk(
        program.bytecode(),
        &ExecutionInputs::default(),
        &ExecutionConfig::default(),
    );
    let value = match outcome {
        ExecutionOutcome::Returned(value) => value,
        other => return Err(format!("aggregate source stress did not return: {other:?}").into()),
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
