//! The Rust bootstrap supplies code bytes, execution budgets and output/result
//! capabilities. The ELF itself has no Rust, Node, loader or libc dependency.
use super::*;
use lkjscript::{bootstrap_runtime, lower};
use std::ffi::OsString;
use std::io::Read;
use std::os::unix::fs::PermissionsExt;
use std::process::Stdio;

pub struct Outcome {
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub status: u8,
    pub value: u64,
    pub error: Option<String>,
}
pub fn lower(checked: &k::CheckedProgram) -> Result<lower::Program, String> {
    lower::lower(checked, bootstrap_runtime::get()?)
}
pub fn image(
    program: &lower::Program,
    meaning: &k::Program,
    entry: usize,
    packet: bool,
) -> Result<Vec<u8>, String> {
    let function = &meaning.functions[entry];
    let world = entry_world(meaning, function)?;
    let exported = program
        .exports
        .iter()
        .find(|export| export.name == format!("fn:{}", function.name))
        .ok_or_else(|| format!("E_ENTRY missing native entry {}", function.name))?;
    lkjscript::native::encode(
        program,
        lkjscript::native::Entry {
            function: exported.function,
            arguments: if world { vec![1] } else { Vec::new() },
            result_fd: if packet { Some(3) } else { None },
        },
    )
}
pub fn save(path: &Path, bytes: &[u8]) -> Result<(), String> {
    fs::write(path, bytes).map_err(|error| format!("E_IO {}: {error}", path.display()))?;
    fs::set_permissions(path, fs::Permissions::from_mode(0o755))
        .map_err(|error| format!("E_IO {}: {error}", path.display()))
}
fn trap_message(program: &lower::Program, code: u64) -> &str {
    match code {
        1 => "integer overflow",
        2 => "division by zero",
        3 => "index out of bounds",
        4 => "explicit trap",
        5 => "assertion failed",
        6 => "heap allocation failed",
        8 => "stack extent exhausted",
        7 => "host output failure",
        _ => program
            .trap_messages
            .get(code.saturating_sub(16) as usize)
            .map_or("trap", String::as_str),
    }
}
pub fn execute(
    path: &Path,
    program: &lower::Program,
    source: &str,
    capture: bool,
    arguments: &[OsString],
) -> Result<Outcome, String> {
    let mut command = Command::new(path);
    command
        .args(arguments)
        .stdin(if capture {
            Stdio::null()
        } else {
            Stdio::inherit()
        })
        .stdout(if capture {
            Stdio::piped()
        } else {
            Stdio::inherit()
        })
        .stderr(if capture {
            Stdio::piped()
        } else {
            Stdio::inherit()
        });
    let mut channel = host_process::result_channel(&mut command)
        .map_err(|error| format!("E_HOST result channel: {error}"))?;
    host_process::prepare_native_child(&mut command)?;
    let output = command
        .output()
        .map_err(|error| format!("E_HOST native: {error}"))?;
    drop(command);
    let mut packet = Vec::new();
    channel
        .read_to_end(&mut packet)
        .map_err(|error| format!("E_HOST result channel: {error}"))?;
    if packet.len() != 24 {
        return Err(format!(
            "E_HOST native image exited {} without its result packet: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    let word = |offset| u64::from_le_bytes(packet[offset..offset + 8].try_into().unwrap());
    let code = word(0);
    let value = word(8);
    let line = word(16);
    let status = if code == 0 {
        u8::try_from(line).map_err(|_| "E_HOST invalid native exit status".to_string())?
    } else {
        1
    };
    let error = if output.status.code() != Some(i32::from(status)) {
        Some(format!(
            "E_HOST native image exited {} after declaring status {status}",
            output.status
        ))
    } else if code != 0 {
        Some(format!(
            "{source}:{line}: R{code:04} {}",
            trap_message(program, code)
        ))
    } else {
        None
    };
    Ok(Outcome {
        stdout: output.stdout,
        stderr: output.stderr,
        status,
        value,
        error,
    })
}

pub fn scalar(
    program: &k::Program,
    function: &k::Function,
    value: u64,
) -> Result<machine::Scalar, String> {
    Ok(match program.types[function.result].kind {
        k::TypeKind::Unit => machine::Scalar::Unit,
        k::TypeKind::Bool => machine::Scalar::Bool(value != 0),
        k::TypeKind::Char => machine::Scalar::Char(
            u32::try_from(value)
                .ok()
                .and_then(char::from_u32)
                .ok_or_else(|| "E_HOST native image returned an invalid character".to_string())?,
        ),
        k::TypeKind::F64 => machine::Scalar::F64 { bits: value },
        k::TypeKind::Int { bits, signed } => machine::Scalar::Int {
            value,
            bits,
            signed,
        },
        _ => machine::Scalar::Int {
            value,
            bits: 64,
            signed: true,
        },
    })
}
pub fn run(
    checked: &k::CheckedProgram,
    args: &Arguments,
    timings: &mut timing::Timings,
) -> Result<u8, String> {
    let mut status = 0;
    let lowered = timings
        .measure("lower", || lower(checked))
        .map_err(|error| format!("E_EMIT {error}"))?;
    let meaning = checked.program();
    let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("target/run");
    fs::create_dir_all(&directory).map_err(|error| format!("E_IO {error}"))?;
    for entry in entries(meaning, args)? {
        let bytes = timings
            .measure("native_encode", || {
                image(&lowered, meaning, entry, args.verb != "build")
            })
            .map_err(|error| format!("E_EMIT {error}"))?;
        timings.count("emitted_bytes", bytes.len());
        if args.verb == "build" {
            let path = PathBuf::from(args.output.clone().unwrap_or_else(|| {
                Path::new(&args.path)
                    .with_extension("elf")
                    .to_string_lossy()
                    .into_owned()
            }));
            save(&path, &bytes)?;
        } else {
            let path = directory.join(format!("{}-{entry}.elf", std::process::id()));
            save(&path, &bytes)?;
            let outcome = timings.measure("native_run", || {
                execute(&path, &lowered, &args.path, false, &args.program_arguments)
            });
            let _ = fs::remove_file(&path);
            let outcome = outcome?;
            if status == 0 {
                status = outcome.status;
            }
            if let Some(error) = outcome.error {
                return Err(error);
            }
            if args.verb != "test" {
                std::io::stdout()
                    .write_all(
                        super::scalar(scalar(meaning, &meaning.functions[entry], outcome.value)?)
                            .as_bytes(),
                    )
                    .map_err(|error| format!("E_IO {error}"))?;
            }
        }
    }
    Ok(status)
}
