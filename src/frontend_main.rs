// Native World source producer. frontend.rs owns the shared semantic pipeline;
// source.rs owns module discovery for both explicit bundles and directory inputs.
pub mod ast;
pub mod check;
pub mod diagnostic;
pub mod diagnostics;
pub mod elaborate;
pub(crate) mod facts;
pub mod frontend;
pub mod frontend_bundle;
pub mod frontend_input;
pub mod generics;
pub mod k;
pub mod k_text;
pub mod names;
pub mod resolve;
pub mod source;
pub mod syntax;

fn arguments(world: &mut World) -> Result<Vec<String>, String> {
    let mut arguments: Vec<String> = Vec::new();
    for bytes in world.args() {
        match String::from_utf8(bytes) {
            Ok(argument) => arguments.push(argument),
            Err(_) => return Err("E_USAGE arguments must be valid UTF-8".to_string()),
        }
    }
    Ok(arguments)
}

fn report(default_path: &str, problem: diagnostic::Diagnostic) -> String {
    let path = if problem.path.is_empty() {
        default_path.to_string()
    } else {
        problem.path.clone()
    };
    format!(
        "{path}:{}:{}: {} {}",
        problem.line, problem.column, problem.code, problem.message
    )
}

fn read_bundle(directory: &mut Directory, path: &str) -> Result<source::SourceBundle, String> {
    let mut file = match directory.open(path) {
        Ok(file) => file,
        Err(errno) => return Err(format!("{path}:1:1: E_IO open failed (errno {errno})")),
    };
    let bytes = match file.read_to_end() {
        Ok(bytes) => bytes,
        Err(errno) => return Err(format!("{path}:1:1: E_IO read failed (errno {errno})")),
    };
    match frontend_bundle::decode(&bytes) {
        Ok(bundle) => Ok(bundle),
        Err(message) => Err(format!("{path}:1:1: E_BUNDLE {message}")),
    }
}

fn read_source(directory: &mut Directory, path: &str) -> Result<Option<String>, String> {
    let mut file = match directory.open(path) {
        Ok(file) => file,
        Err(errno) => {
            if errno == 2 || errno == 20 {
                return Ok(None);
            }
            return Err(format!("open failed (errno {errno})"));
        }
    };
    let bytes = match file.read_to_end() {
        Ok(bytes) => bytes,
        Err(errno) => return Err(format!("read failed (errno {errno})")),
    };
    match String::from_utf8(bytes) {
        Ok(text) => Ok(Some(text)),
        Err(_) => Err("source is not valid UTF-8".to_string()),
    }
}

fn write_k(world: &mut World, checked: &k::CheckedProgram) {
    let ir = k_text::write(checked.program());
    let mut output = world.stdout();
    output.write(ir.as_bytes());
}

fn run_bundle(world: &mut World, path: &str, check_only: bool) -> Result<(), String> {
    let mut directory = match world.directory(".") {
        Ok(directory) => directory,
        Err(errno) => return Err(format!(".:1:1: E_IO open directory failed (errno {errno})")),
    };
    let bundle = read_bundle(&mut directory, path)?;
    let checked = match frontend::check_bundle(&bundle) {
        Ok(checked) => checked,
        Err(problem) => return Err(report(&bundle.root, problem)),
    };
    if !check_only {
        write_k(world, &checked);
    }
    Ok(())
}

fn run_source(world: &mut World, options: frontend_input::SourceOptions) -> Result<(), String> {
    let mut directory = match world.directory(&options.directory) {
        Ok(directory) => directory,
        Err(errno) => {
            return Err(format!(
                "{}:1:1: E_IO open source directory failed (errno {errno})",
                options.directory
            ));
        }
    };
    let mut library: Option<Directory> = match &options.library {
        Some(path) => match world.directory(path) {
            Ok(directory) => Some(directory),
            Err(errno) => {
                return Err(format!(
                    "{path}:1:1: E_IO open library directory failed (errno {errno})"
                ));
            }
        },
        None => None,
    };
    let mut cache = frontend_input::Cache::new();
    let standard = if options.library.is_some() {
        Some("$std/lib.rs")
    } else {
        None
    };
    let parsed = {
        let mut read = |path: &str| {
            if path.starts_with("$std/") {
                match library.as_mut() {
                    Some(directory) => read_source(directory, &path[5..]),
                    None => Ok(None),
                }
            } else {
                read_source(&mut directory, path)
            }
        };
        source::load_report(&options.root, standard, &mut |path: &str| {
            cache.read(path, &mut read)
        })
    };
    let loaded = frontend_input::Loaded {
        report: parsed,
        bundle: cache.into_bundle(&options),
    };
    match options.output {
        frontend_input::Output::Pack => {
            // Syntax/module rejection inputs are useful snapshots. An I/O failure
            // cannot be reproduced by omitting a file, so do not emit that bundle.
            for problem in loaded.report.diagnostics {
                if problem.code.as_str() == "E_IO" {
                    return Err(report(&options.root, problem));
                }
            }
            let bytes = frontend_bundle::encode(&loaded.bundle);
            let mut output = world.stdout();
            output.write(&bytes);
        }
        frontend_input::Output::Check | frontend_input::Output::K => {
            let program = match loaded.report.into_result() {
                Ok(program) => program,
                Err(problem) => return Err(report(&options.root, problem)),
            };
            let checked = match frontend::check_source(&program) {
                Ok(checked) => checked,
                Err(problem) => return Err(report(&options.root, problem)),
            };
            if options.output == frontend_input::Output::K {
                write_k(world, &checked);
            }
        }
    }
    Ok(())
}

fn run(world: &mut World) -> Result<(), String> {
    let arguments = arguments(world)?;
    let request = match frontend_input::parse(arguments) {
        Ok(request) => request,
        Err(message) => return Err(format!("E_USAGE {message}")),
    };
    match request {
        frontend_input::Request::Help => {
            let mut output = world.stdout();
            output.write(frontend_input::usage().as_bytes());
            Ok(())
        }
        frontend_input::Request::Bundle { path, check } => run_bundle(world, &path, check),
        frontend_input::Request::Source(options) => run_source(world, options),
    }
}

pub fn main(mut world: World) {
    if let Err(message) = run(&mut world) {
        let mut errors = world.stderr();
        errors.write(format!("{message}\n").as_bytes());
        world.set_exit_status(1u8);
    }
}
