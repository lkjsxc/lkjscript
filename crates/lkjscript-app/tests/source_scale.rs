use std::fmt::Write as FmtWrite;
use std::fs;
use std::io::Write as IoWrite;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

use lkjscript_compiler::{
    compile_path, compile_path_with_metrics, compile_source, validate_source,
};
use lkjscript_core::{ExecutionOutcome, ExecutionPolicy};
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

    let program = compile_path(&source)?;
    let outcome = run_chunk(
        program.bytecode(),
        &ExecutionInputs::default(),
        &ExecutionPolicy::unrestricted(),
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
    let program = compile_path(&source)?;
    let outcome = run_chunk(
        program.bytecode(),
        &ExecutionInputs::default(),
        &ExecutionPolicy::unrestricted(),
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

    let program = compile_path(&root)?;
    let outcome = run_chunk(
        program.bytecode(),
        &ExecutionInputs::default(),
        &ExecutionPolicy::unrestricted(),
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

fn generated_nested_do_source(depth: usize, result: i64) -> Result<String, std::fmt::Error> {
    let mut source = String::new();
    source.push_str("main/\nsig/\ninputs/\n/inputs\noutput/\ni64\n/output\n/sig\n");
    for _ in 0..depth {
        source.push_str("do/\n");
    }
    writeln!(source, "{result}")?;
    for _ in 0..depth {
        source.push_str("/do\n");
    }
    source.push_str("/main\n");
    Ok(source)
}

fn generated_nested_list_type_source(depth: usize, leaf: &str) -> String {
    let mut ty = String::new();
    for _ in 0..depth {
        ty.push_str("list/\n");
    }
    ty.push_str(leaf);
    ty.push('\n');
    for _ in 0..depth {
        ty.push_str("/list\n");
    }
    let mut element = String::new();
    for _ in 1..depth {
        element.push_str("list/\n");
    }
    element.push_str(leaf);
    element.push('\n');
    for _ in 1..depth {
        element.push_str("/list\n");
    }
    format!(
        "main/\nsig/\ninputs/\n/inputs\noutput/\n{ty}/output\n/sig\nempty-list/\n{element}/empty-list\n/main\n"
    )
}

fn generated_wide_cyclic_type_source(width: usize) -> Result<String, std::fmt::Error> {
    let mut source = String::from("enum/\nname/\nwide\n/name\nforall/\n");
    for index in 0..width {
        writeln!(source, "t-{index}")?;
    }
    source.push_str(
        "/forall\nvariants/\nvariant/\nname/\nempty\n/name\nfields/\n/fields\n/variant\n/variants\n/enum\n",
    );
    source.push_str(concat!(
        "product/\nname/\nrecursive\n/name\nfields/\nfield/\nname/\nnext\n/name\ntype/\n",
        "option/\nproduct/\nrecursive\n/product\n/option\n/type\n/field\n/fields\n/product\n",
    ));
    source.push_str(
        "product/\nname/\nwide-holder\n/name\nfields/\nfield/\nname/\nvalue\n/name\ntype/\nwide/\n",
    );
    for _ in 0..width {
        source.push_str("i64\n");
    }
    source.push_str(concat!(
        "/wide\n/type\n/field\n/fields\n/product\n",
        "main/\nsig/\ninputs/\n/inputs\noutput/\nunit\n/output\n/sig\nunit\n/main\n",
    ));
    Ok(source)
}

fn generated_deep_malformed_source(depth: usize, mismatched: bool) -> String {
    let mut source = String::from("main/\nsig/\ninputs/\n/inputs\noutput/\nunit\n/output\n/sig\n");
    for _ in 0..depth {
        source.push_str("do/\n");
    }
    source.push_str("unit\n");
    for _ in 0..depth / 2 {
        source.push_str("/do\n");
    }
    if mismatched {
        source.push_str("/if\n");
    }
    source
}

fn generated_nested_match_source(
    depth: usize,
    result: i64,
    leaf_pattern: &str,
    trailing_arms: &str,
) -> Result<String, std::fmt::Error> {
    let mut source = String::new();
    for index in 0..depth {
        let field_type = if index + 1 == depth {
            "bool".to_string()
        } else {
            format!("product/\ndeep-{}\n/product", index + 1)
        };
        write!(
            source,
            concat!(
                "product/\nname/\ndeep-{}\n/name\nfields/\nfield/\n",
                "name/\nvalue\n/name\ntype/\n{}\n/type\n/field\n/fields\n/product\n"
            ),
            index, field_type,
        )?;
    }
    source.push_str("main/\nsig/\ninputs/\n/inputs\noutput/\ni64\n/output\n/sig\nmatch/\n");
    for index in 0..depth {
        write!(source, "product-value/\ndeep-{index}\nfield/\nvalue\n")?;
    }
    source.push_str("true\n");
    for _ in 0..depth {
        source.push_str("/field\n/product-value\n");
    }
    source.push_str("arms/\narm/\n");
    for index in 0..depth {
        write!(
            source,
            concat!(
                "product-pattern/\ntype/\nproduct\ndeep-{}\n/type\nfields/\n",
                "product-field-pattern/\nname/\nvalue\n/name\n"
            ),
            index,
        )?;
    }
    source.push_str(leaf_pattern);
    source.push('\n');
    for _ in 0..depth {
        source.push_str("/product-field-pattern\n/fields\n/product-pattern\n");
    }
    writeln!(source, "{result}")?;
    source.push_str("/arm\n");
    source.push_str(trailing_arms);
    source.push_str("/arms\n/match\n/main\n");
    Ok(source)
}

fn generated_broad_match_source(
    fields: usize,
    type_arguments: usize,
    include_missing_arm: bool,
    duplicate_covered_arm: bool,
) -> Result<String, std::fmt::Error> {
    let mut source = String::from("enum/\nname/\npayload\n/name\nforall/\n");
    for index in 0..type_arguments {
        writeln!(source, "t{index}")?;
    }
    source.push_str(concat!(
        "/forall\nvariants/\nvariant/\nname/\nempty\n/name\nfields/\n/fields\n/variant\n",
        "/variants\n/enum\nenum/\nname/\nbroad\n/name\nvariants/\n",
        "variant/\nname/\ncovered\n/name\nfields/\n/fields\n/variant\n",
        "variant/\nname/\nmissing\n/name\nfields/\n",
    ));
    for field in 0..fields {
        write!(
            source,
            "variant-field/\nname/\nf{field}\n/name\ntype/\npayload/\n"
        )?;
        for _ in 0..type_arguments {
            source.push_str("i64\n");
        }
        source.push_str("/payload\n/type\n/variant-field\n");
    }
    source.push_str(concat!(
        "/fields\n/variant\n/variants\n/enum\n",
        "main/\nsig/\ninputs/\n/inputs\noutput/\ni64\n/output\n/sig\nmatch/\n",
        "variant-value/\ntype/\nbroad/\n/broad\n/type\nvariant/\ncovered\n/variant\n",
        "fields/\n/fields\n/variant-value\narms/\n",
        "arm/\nvariant-pattern/\ntype/\nbroad/\n/broad\n/type\nvariant/\ncovered\n/variant\n",
        "fields/\n/fields\n/variant-pattern\n707\n/arm\n",
    ));
    if include_missing_arm {
        source.push_str(concat!(
            "arm/\nvariant-pattern/\ntype/\nbroad/\n/broad\n/type\nvariant/\nmissing\n/variant\n",
            "fields/\n",
        ));
        for field in 0..fields {
            write!(
                source,
                concat!(
                    "variant-field-pattern/\nname/\nf{}\n/name\n",
                    "wildcard/\n/wildcard\n/variant-field-pattern\n"
                ),
                field,
            )?;
        }
        source.push_str("/fields\n/variant-pattern\n0\n/arm\n");
    }
    if duplicate_covered_arm {
        source.push_str(concat!(
            "arm/\nvariant-pattern/\ntype/\nbroad/\n/broad\n/type\nvariant/\ncovered\n/variant\n",
            "fields/\n/fields\n/variant-pattern\n1\n/arm\n",
        ));
    }
    source.push_str("/arms\n/match\n/main\n");
    Ok(source)
}

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
fn deeply_nested_source_compiles_validates_executes_and_drops_on_a_small_stack(
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    const DEPTH: usize = 20_000;
    const HIR_EXPRESSIONS: u64 = 20_001;
    const RESULT: i64 = 20_000;
    const _: () = assert!(HIR_EXPRESSIONS > 16_384);
    let source = generated_nested_do_source(DEPTH, RESULT)?;
    let worker = std::thread::Builder::new()
        .name("deep-source-small-stack".into())
        .stack_size(256 * 1024)
        .spawn(
            move || -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
                let program = compile_source(&source, "deep-source.lkjscript")?;
                if program.memory_plan().work.expressions != HIR_EXPRESSIONS {
                    return Err(format!(
                        "deep source produced {} HIR memory-plan expressions, expected {HIR_EXPRESSIONS}",
                        program.memory_plan().work.expressions
                    )
                    .into());
                }
                if program.ssa().program().functions.is_empty() {
                    return Err("deep source did not reach verified and normalized SSA".into());
                }
                let outcome = run_chunk(
                    program.bytecode(),
                    &ExecutionInputs::default(),
                    &ExecutionPolicy::unrestricted(),
                );
                let value = match outcome {
                    ExecutionOutcome::Returned(value) => value,
                    other => {
                        return Err(format!("deep source program did not return: {other:?}").into())
                    }
                };
                if value.as_i64() != Some(RESULT) {
                    return Err(format!("deep source returned {value:?}, expected {RESULT}").into());
                }
                drop(value);
                drop(program);
                Ok(())
            },
        )?;
    worker
        .join()
        .map_err(|_| std::io::Error::other("deep source worker panicked"))??;
    Ok(())
}

#[test]
fn deeply_nested_type_crosses_analysis_memory_ssa_bytecode_and_vm_on_a_small_stack(
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    const DEPTH: usize = 512;
    const _: () = assert!(DEPTH > 256);
    let valid = generated_nested_list_type_source(DEPTH, "i64");
    let malformed = generated_nested_list_type_source(DEPTH, "missing-type");
    let worker = std::thread::Builder::new()
        .name("deep-type-small-stack".into())
        .stack_size(256 * 1024)
        .spawn(
            move || -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
                validate_source(&valid, "deep-type.lkjscript")?;
                let program = compile_source(&valid, "deep-type.lkjscript")?;
                if program.memory_plan().work.type_nodes <= 256 {
                    return Err("deep type did not cross former type/trait work geometry".into());
                }
                if program.ssa().program().functions.is_empty() {
                    return Err("deep type did not reach verified SSA".into());
                }
                match run_chunk(
                    program.bytecode(),
                    &ExecutionInputs::default(),
                    &ExecutionPolicy::unrestricted(),
                ) {
                    ExecutionOutcome::Returned(_) => {}
                    other => {
                        return Err(format!("deep type program did not return: {other:?}").into())
                    }
                }
                drop(program);

                let diagnose = || {
                    compile_source(&malformed, "malformed-deep-type.lkjscript")
                        .map_err(|error| error.to_string())
                };
                let first = match diagnose() {
                    Err(error) => error,
                    Ok(_) => return Err("malformed deep type unexpectedly compiled".into()),
                };
                let second = match diagnose() {
                    Err(error) => error,
                    Ok(_) => return Err("malformed deep type unexpectedly compiled twice".into()),
                };
                if first != second || !first.contains("unbound type parameter missing-type") {
                    return Err(format!(
                    "deep type diagnostic was not deterministic: first={first:?}, second={second:?}"
                )
                    .into());
                }
                Ok(())
            },
        )?;
    worker
        .join()
        .map_err(|_| std::io::Error::other("deep type worker panicked"))??;
    Ok(())
}

#[test]
fn wide_and_cyclic_types_cross_the_public_compiler_and_runtime_path(
) -> Result<(), Box<dyn std::error::Error>> {
    const WIDTH: usize = 300;
    const _: () = assert!(WIDTH > 256);
    let source = generated_wide_cyclic_type_source(WIDTH)?;
    let program = compile_source(&source, "wide-cyclic-type.lkjscript")?;
    let outcome = run_chunk(
        program.bytecode(),
        &ExecutionInputs::default(),
        &ExecutionPolicy::unrestricted(),
    );
    assert!(
        matches!(outcome, ExecutionOutcome::Returned(_)),
        "{outcome:?}"
    );
    Ok(())
}

#[test]
fn malformed_deep_source_is_deterministic_and_drops_partial_trees_on_a_small_stack(
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    const DEPTH: usize = 8_192;
    let worker = std::thread::Builder::new()
        .name("malformed-deep-source-small-stack".into())
        .stack_size(256 * 1024)
        .spawn(
            move || -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
                for (mismatched, expected) in [
                    (true, "mismatched close marker /if; expected /do"),
                    (false, "unclosed marker do/; expected /do"),
                ] {
                    let source = generated_deep_malformed_source(DEPTH, mismatched);
                    let diagnose = || {
                        validate_source(&source, "malformed-deep-source.lkjscript")
                            .map_err(|error| error.to_string())
                    };
                    let first = diagnose().err().ok_or_else(|| {
                        std::io::Error::other("malformed deep source unexpectedly validated")
                    })?;
                    let second = diagnose().err().ok_or_else(|| {
                        std::io::Error::other("malformed deep source unexpectedly validated twice")
                    })?;
                    if first != second || !first.contains(expected) {
                        return Err(format!(
                        "deep diagnostic was not deterministic: first={first:?}, second={second:?}"
                    )
                        .into());
                    }
                }
                Ok(())
            },
        )?;
    worker
        .join()
        .map_err(|_| std::io::Error::other("malformed source worker panicked"))??;
    Ok(())
}

#[test]
fn deep_match_usefulness_diagnostics_and_lowering_are_stack_safe(
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    const EXECUTION_DEPTH: usize = 32;
    const DIAGNOSTIC_DEPTH: usize = 2_048;
    const RESULT: i64 = 4_040;
    const TRUE_PATTERN: &str = "bool-pattern/\ntrue\n/bool-pattern";
    const WILDCARD_ARM: &str = "arm/\nwildcard/\n/wildcard\n0\n/arm\n";
    let exhaustive =
        generated_nested_match_source(EXECUTION_DEPTH, RESULT, TRUE_PATTERN, WILDCARD_ARM)?;
    let nonexhaustive = generated_nested_match_source(DIAGNOSTIC_DEPTH, RESULT, TRUE_PATTERN, "")?;
    let malformed = generated_nested_match_source(
        DIAGNOSTIC_DEPTH,
        RESULT,
        "i64-pattern/\n0\n/i64-pattern",
        WILDCARD_ARM,
    )?;
    let worker = std::thread::Builder::new()
        .name("deep-match-small-stack".into())
        .stack_size(256 * 1024)
        .spawn(
            move || -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
                let program = compile_source(&exhaustive, "deep-match-scale.lkjscript")?;
                let outcome = run_chunk(
                    program.bytecode(),
                    &ExecutionInputs::default(),
                    &ExecutionPolicy::unrestricted(),
                );
                let value = match outcome {
                    ExecutionOutcome::Returned(value) => value,
                    other => {
                        return Err(format!("deep match program did not return: {other:?}").into())
                    }
                };
                if value.as_i64() != Some(RESULT) {
                    return Err(format!("deep match returned {value:?}, expected {RESULT}").into());
                }
                drop(value);
                drop(program);

                let diagnose = |source: &str, name: &str| {
                    compile_source(source, name).map_err(|error| error.to_string())
                };
                let first = diagnose(&nonexhaustive, "deep-match-nonexhaustive.lkjscript")
                    .err()
                    .ok_or_else(|| {
                        std::io::Error::other("deep match without a fallback unexpectedly compiled")
                    })?;
                let second = diagnose(&nonexhaustive, "deep-match-nonexhaustive.lkjscript")
                    .err()
                    .ok_or_else(|| {
                        std::io::Error::other("deep match unexpectedly compiled twice")
                    })?;
                if first != second
                    || !first.contains("nonexhaustive match; canonical typed witness:")
                    || !first.contains("bool::false")
                    || first.matches("product::product#").count() != DIAGNOSTIC_DEPTH
                {
                    return Err(format!(
                        "deep nonexhaustive diagnostic was incomplete or unstable: {first}"
                    )
                    .into());
                }

                let first = diagnose(&malformed, "deep-match-malformed.lkjscript")
                    .err()
                    .ok_or_else(|| {
                        std::io::Error::other("deep malformed match unexpectedly compiled")
                    })?;
                let second = diagnose(&malformed, "deep-match-malformed.lkjscript")
                    .err()
                    .ok_or_else(|| {
                        std::io::Error::other("deep malformed match unexpectedly compiled twice")
                    })?;
                if first != second || !first.contains("i64-pattern requires an I64 scrutinee") {
                    return Err(format!(
                        "deep malformed match diagnostic was incomplete or unstable: {first}"
                    )
                    .into());
                }
                Ok(())
            },
        )?;
    worker
        .join()
        .map_err(|_| std::io::Error::other("deep match worker panicked"))??;
    Ok(())
}

#[test]
fn broad_match_crosses_the_former_witness_reservation_and_preserves_semantics(
) -> Result<(), Box<dyn std::error::Error>> {
    const FIELDS: usize = 300;
    const TYPE_ARGUMENTS: usize = 16;
    const OLD_SINGLE_PATTERN_WITNESS_RESERVATION: usize = 32_768 + 64;

    let exhaustive = generated_broad_match_source(FIELDS, TYPE_ARGUMENTS, true, false)?;
    let program = compile_source(&exhaustive, "broad-match-exhaustive.lkjscript")?;
    let outcome = run_chunk(
        program.bytecode(),
        &ExecutionInputs::default(),
        &ExecutionPolicy::unrestricted(),
    );
    let value = match outcome {
        ExecutionOutcome::Returned(value) => value,
        other => return Err(format!("broad match program did not return: {other:?}").into()),
    };
    assert_eq!(value.as_i64(), Some(707));

    let nonexhaustive = generated_broad_match_source(FIELDS, TYPE_ARGUMENTS, false, false)?;
    let diagnose = || {
        compile_source(&nonexhaustive, "broad-match-nonexhaustive.lkjscript")
            .err()
            .map(|error| error.to_string())
    };
    let first = diagnose().ok_or("broad match unexpectedly compiled")?;
    assert_eq!(Some(first.clone()), diagnose());
    let marker = "nonexhaustive match; canonical typed witness: ";
    let witness = first
        .split_once(marker)
        .map(|(_, witness)| witness)
        .ok_or("broad match diagnostic omitted its canonical witness")?;
    assert!(
        witness.len() > OLD_SINGLE_PATTERN_WITNESS_RESERVATION,
        "witness length {} did not cross former reservation",
        witness.len()
    );
    assert_eq!(witness.matches("wildcard<Enum#").count(), FIELDS);
    assert!(witness.ends_with(')'), "witness was truncated: {witness}");

    let useless = generated_broad_match_source(FIELDS, TYPE_ARGUMENTS, true, true)?;
    let error = compile_source(&useless, "broad-match-useless.lkjscript")
        .err()
        .map(|error| error.to_string())
        .ok_or("duplicate covered variant arm unexpectedly compiled")?;
    assert!(error.contains("useless or subsumed match arm 2"), "{error}");
    Ok(())
}

#[test]
fn flat_source_beyond_former_quotas_compiles_validates_and_executes(
) -> Result<(), Box<dyn std::error::Error>> {
    let source = generated_flat_source()?;

    // Every non-empty line in this fixture is one lexer token. The geometry is
    // deliberately well beyond the former 384-token, 16-child, and 8-form ceilings.
    assert!(source.lines().count() > 384);

    // ExecutableProgram exposes bytecode only after compiler-side validation.
    let program = compile_source(&source, "source-scale.lkjscript")?;
    let outcome = run_chunk(
        program.bytecode(),
        &ExecutionInputs::default(),
        &ExecutionPolicy::unrestricted(),
    );
    let value = match outcome {
        ExecutionOutcome::Returned(value) => value,
        other => return Err(format!("flat source program did not return: {other:?}").into()),
    };
    assert_eq!(value.as_i64(), Some(EXPECTED_RESULT));
    Ok(())
}

fn generated_many_functions_source(functions: usize) -> Result<String, std::fmt::Error> {
    let mut source = String::new();
    for index in 0..functions {
        write!(
            source,
            concat!(
                "def/\nname/\nwide-function-{index}\n/name\nfn/\nsig/\ninputs/\n/inputs\n",
                "output/\ni64\n/output\n/sig\nparams/\n/params\n{index}\n/fn\n/def\n"
            ),
            index = index,
        )?;
    }
    write!(
        source,
        concat!(
            "main/\nsig/\ninputs/\n/inputs\noutput/\ni64\n/output\n/sig\n",
            "wide-function-{0}/\n/wide-function-{0}\n/main\n"
        ),
        functions - 1,
    )?;
    Ok(source)
}

fn generated_borrow_calls_source(calls: usize) -> Result<String, std::fmt::Error> {
    let mut source = String::from(concat!(
        "def/\nname/\nobserve-wide-borrow\n/name\nfn/\nsig/\ninputs/\nstring\n/inputs\n",
        "output/\nunit\n/output\n/sig\nparams/\nvalue\nstring\n/params\nunit\n/fn\n/def\n",
        "main/\nsig/\ninputs/\n/inputs\noutput/\ni64\n/output\n/sig\nlet/\nbind/\n",
        "text\nempty-string/\n/empty-string\n/bind\ndo/\n"
    ));
    for _ in 0..calls {
        source.push_str("observe-wide-borrow/\ntext\n/observe-wide-borrow\n");
    }
    source.push_str("42\n/do\n/let\n/main\n");
    Ok(source)
}

fn compile_and_run_borrow_call_scale(calls: usize) -> Result<(), Box<dyn std::error::Error>> {
    let body_started = Instant::now();
    let source = generated_borrow_calls_source(calls)?;
    let source_bytes = source.len();
    let directory = TempDir::new()?;
    let path = directory.0.join("wide-borrow-calls.lkjscript");
    fs::write(&path, source)?;
    let (program, metrics) = compile_path_with_metrics(&path)?;
    assert!(program.memory_plan().calls.len() >= calls);
    assert_eq!(program.memory_plan().borrow_scopes.len(), calls);
    assert!(program.memory_plan().work.calls >= u64::try_from(calls)?);
    assert_eq!(
        program.memory_plan().work.borrow_scopes,
        u64::try_from(calls)?
    );

    let vm_started = Instant::now();
    let outcome = run_chunk(
        program.bytecode(),
        &ExecutionInputs::default(),
        &ExecutionPolicy::unrestricted(),
    );
    let vm_execution = vm_started.elapsed();
    let value = match outcome {
        ExecutionOutcome::Returned(value) => value,
        other => return Err(format!("wide borrow-call program did not return: {other:?}").into()),
    };
    assert_eq!(value.as_i64(), Some(42));

    let main = program.bytecode().main();
    eprintln!(
        concat!(
            "LKJSCRIPT_BORROW_SCALE {{",
            "\"calls\":{},\"source_bytes\":{},\"instructions\":{},",
            "\"locals\":{},\"unique_places\":{},\"cleanup_ranges\":{},",
            "\"source_loading_ns\":{},\"parsing_ns\":{},",
            "\"hir_analysis_ns\":{},\"effect_analysis_ns\":{},",
            "\"memory_planning_ns\":{},\"ssa_construction_ns\":{},",
            "\"ssa_verification_ns\":{},\"normalization_ns\":{},",
            "\"bytecode_lowering_ns\":{},\"bytecode_validation_ns\":{},",
            "\"package_validation_ns\":{},\"compile_total_ns\":{},",
            "\"vm_execution_ns\":{},\"test_body_ns\":{},\"result\":42",
            "}}"
        ),
        calls,
        source_bytes,
        program.bytecode().main_instructions().len(),
        main.locals,
        main.unique_places,
        main.failure_cleanup_ranges.len(),
        metrics.source_loading.as_nanos(),
        metrics.parsing.as_nanos(),
        metrics.hir_analysis.as_nanos(),
        metrics.effect_analysis.as_nanos(),
        metrics.memory_planning.as_nanos(),
        metrics.ssa_construction.as_nanos(),
        metrics.ssa_verification.as_nanos(),
        metrics.normalization.as_nanos(),
        metrics.bytecode_lowering.as_nanos(),
        metrics.bytecode_validation.as_nanos(),
        metrics.package_validation.as_nanos(),
        metrics.total.as_nanos(),
        vm_execution.as_nanos(),
        body_started.elapsed().as_nanos(),
    );
    Ok(())
}

fn generated_wide_transport_source(width: usize) -> Result<String, std::fmt::Error> {
    let mut source = String::from("def/\nname/\nwide-transport\n/name\nfn/\nforall/\n");
    for index in 0..width {
        writeln!(source, "t-{index}")?;
    }
    source.push_str("/forall\nsig/\ninputs/\n");
    for index in 0..width {
        writeln!(source, "t-{index}")?;
    }
    source.push_str("/inputs\noutput/\nt-0\n/output\n/sig\nparams/\n");
    for index in 0..width {
        writeln!(source, "value-{index}")?;
        writeln!(source, "t-{index}")?;
    }
    source.push_str(concat!(
        "/params\nvalue-0\n/fn\n/def\n",
        "main/\nsig/\ninputs/\n/inputs\noutput/\ni64\n/output\n/sig\nwide-transport/\n"
    ));
    for index in 0..width {
        writeln!(source, "{index}")?;
    }
    source.push_str("/wide-transport\n/main\n");
    Ok(source)
}

#[test]
fn seventeen_witness_arguments_cross_hir_ssa_validated_bytecode_and_vm(
) -> Result<(), Box<dyn std::error::Error>> {
    const WIDTH: usize = 17;
    let source = generated_wide_transport_source(WIDTH)?;
    let program = compile_source(&source, "wide-memory-witness-transport.lkjscript")?;
    assert_eq!(
        program.memory_plan().functions[0]
            .signature
            .witness_parameters
            .len(),
        WIDTH,
    );
    assert_eq!(
        program.memory_plan().calls[0].witness_arguments.len(),
        WIDTH
    );
    assert_eq!(
        program.ssa().program().functions[0]
            .signature
            .memory_witness_parameters
            .len(),
        WIDTH,
    );
    assert_eq!(
        program.bytecode().protos()[0]
            .memory_witness_parameters
            .len(),
        WIDTH
    );
    assert_eq!(
        program.bytecode().main().call_witnesses[0].bindings.len(),
        WIDTH
    );
    let outcome = run_chunk(
        program.bytecode(),
        &ExecutionInputs::default(),
        &ExecutionPolicy::unrestricted(),
    );
    let value = match outcome {
        ExecutionOutcome::Returned(value) => value,
        other => return Err(format!("wide witness transport did not return: {other:?}").into()),
    };
    assert_eq!(value.as_i64(), Some(0));
    Ok(())
}

#[test]
#[ignore = "opt-in release 4,097-function production-pipeline stress geometry"]
fn four_thousand_ninety_seven_functions_compile_validate_and_execute_in_vm(
) -> Result<(), Box<dyn std::error::Error>> {
    const FUNCTIONS: usize = 4_097;
    let source = generated_many_functions_source(FUNCTIONS)?;
    let program = compile_source(&source, "wide-functions.lkjscript")?;
    assert_eq!(program.bytecode().protos().len(), FUNCTIONS);
    assert_eq!(program.memory_plan().work.functions, 4_098);
    let outcome = run_chunk(
        program.bytecode(),
        &ExecutionInputs::default(),
        &ExecutionPolicy::unrestricted(),
    );
    let value = match outcome {
        ExecutionOutcome::Returned(value) => value,
        other => return Err(format!("wide function program did not return: {other:?}").into()),
    };
    assert_eq!(value.as_i64(), Some(4_096));
    Ok(())
}

#[test]
#[ignore = "opt-in release scaling sample selected by LKJSCRIPT_BORROW_CALLS"]
fn borrow_call_scale_sample() -> Result<(), Box<dyn std::error::Error>> {
    let calls = std::env::var("LKJSCRIPT_BORROW_CALLS")
        .map_err(|_| "LKJSCRIPT_BORROW_CALLS must select the generated test geometry")?
        .parse::<usize>()?;
    compile_and_run_borrow_call_scale(calls)
}

#[test]
#[ignore = "opt-in release 16,385-call and borrow-scope production-pipeline stress geometry"]
fn sixteen_thousand_three_hundred_eighty_five_calls_and_borrow_scopes_execute_in_vm(
) -> Result<(), Box<dyn std::error::Error>> {
    compile_and_run_borrow_call_scale(16_385)
}
