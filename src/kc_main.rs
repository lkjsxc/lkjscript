// World entry for the self-built production K compiler. Inputs are explicit K
// paths beneath the current directory; a completed ELF is written to stdout.
pub mod check;
pub mod diagnostic;
pub(crate) mod facts;
pub mod k;
pub mod k_text;
pub mod kc;
pub mod lower;
pub mod native;
pub mod runtime;

struct Options {
    program: String,
    runtime: Option<String>,
    entry: Option<String>,
    check_only: bool,
}

fn options(world: &mut World) -> Result<Options, String> {
    let mut arguments: Vec<String> = Vec::new();
    for bytes in world.args() {
        let text = match String::from_utf8(bytes) {
            Ok(text) => text,
            Err(_) => return Err("arguments must be valid UTF-8".to_string()),
        };
        arguments.push(text);
    }
    if arguments.is_empty() {
        return Err(
            "usage: kc PROGRAM.k [--runtime RUNTIME.k] [--entry NAME] [--check]".to_string(),
        );
    }
    let program = arguments[0].clone();
    let mut runtime = None;
    let mut entry = None;
    let mut check_only = false;
    let mut at = 1usize;
    while at < arguments.len() {
        let option = &arguments[at];
        if option == "--check" {
            check_only = true;
        } else if option == "--runtime" || option == "--entry" {
            at += 1;
            if at == arguments.len() {
                return Err(format!("{option} needs a value"));
            }
            let value = arguments[at].clone();
            if option == "--runtime" {
                runtime = Some(value);
            } else {
                entry = Some(value);
            }
        } else {
            return Err(format!("unknown option {option}"));
        }
        at += 1;
    }
    Ok(Options {
        program,
        runtime,
        entry,
        check_only,
    })
}

fn report(path: &str, problem: diagnostic::Diagnostic) -> String {
    format!(
        "{path}:{}:{}: {} {}",
        problem.line, problem.column, problem.code, problem.message
    )
}

fn read_checked(directory: &mut Directory, path: &str) -> Result<k::CheckedProgram, String> {
    let mut file = match directory.open(path) {
        Ok(file) => file,
        Err(errno) => return Err(format!("{path}: open failed (errno {errno})")),
    };
    let bytes = match file.read_to_end() {
        Ok(bytes) => bytes,
        Err(errno) => return Err(format!("{path}: read failed (errno {errno})")),
    };
    let text = match String::from_utf8(bytes) {
        Ok(text) => text,
        Err(_) => return Err(format!("{path}: K text is not valid UTF-8")),
    };
    let meaning = match k_text::read(&text) {
        Ok(meaning) => meaning,
        Err(problem) => return Err(report(path, problem)),
    };
    match check::check(meaning) {
        Ok(checked) => Ok(checked),
        Err(problem) => Err(report(path, problem)),
    }
}

fn default_entry(checked: &k::CheckedProgram) -> String {
    for function in &checked.program().functions {
        if function.name == "main" {
            return "main".to_string();
        }
    }
    "probe".to_string()
}

fn run(world: &mut World) -> Result<(), String> {
    let options = options(world)?;
    let mut directory = match world.directory(".") {
        Ok(directory) => directory,
        Err(errno) => return Err(format!(".: open directory failed (errno {errno})")),
    };
    let checked = read_checked(&mut directory, &options.program)?;
    if options.check_only {
        return Ok(());
    }
    let runtime_path = options
        .runtime
        .ok_or("--runtime is required when emitting an artifact")?;
    let runtime_k = read_checked(&mut directory, &runtime_path)?;
    let runtime = match runtime::seal(runtime_k) {
        Ok(runtime) => runtime,
        Err(problem) => return Err(report(&runtime_path, problem)),
    };
    let entry = match options.entry {
        Some(entry) => entry,
        None => default_entry(&checked),
    };
    let bytes = match kc::emit(&checked, &runtime, &entry) {
        Ok(bytes) => bytes,
        Err(problem) => return Err(report(&options.program, problem)),
    };
    let mut output = world.stdout();
    output.write(&bytes);
    Ok(())
}

pub fn main(mut world: World) {
    if let Err(message) = run(&mut world) {
        let mut errors = world.stderr();
        errors.write(format!("{message}\n").as_bytes());
        world.set_exit_status(1u8);
    }
}
