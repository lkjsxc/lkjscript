// Native World source producer. The production module list follows frontend_lib.rs;
// frontend.rs remains the single source-to-K pipeline over explicit bundle bytes.
pub mod ast;
pub mod check;
pub mod diagnostic;
pub mod diagnostics;
pub mod elaborate;
pub(crate) mod facts;
pub mod frontend;
pub mod frontend_bundle;
pub mod generics;
pub mod k;
pub mod k_text;
pub mod names;
pub mod resolve;
pub mod source;
pub mod syntax;

fn bundle_path(world: &mut World) -> Result<String, String> {
    let mut arguments = world.args();
    if arguments.len() != 1 {
        return Err("usage: frontend BUNDLE (a path beneath the current directory)".to_string());
    }
    let bytes = arguments.pop().unwrap();
    match String::from_utf8(bytes) {
        Ok(path) => Ok(path),
        Err(_) => Err("bundle path must be valid UTF-8".to_string()),
    }
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

fn run(world: &mut World) -> Result<(), String> {
    let path = bundle_path(world)?;
    let mut directory = match world.directory(".") {
        Ok(directory) => directory,
        Err(errno) => return Err(format!(".:1:1: E_IO open directory failed (errno {errno})")),
    };
    let bundle = read_bundle(&mut directory, &path)?;
    let ir = match frontend::source_to_k(&bundle) {
        Ok(ir) => ir,
        Err(problem) => return Err(report(&bundle.root, problem)),
    };
    let mut output = world.stdout();
    output.write(ir.as_bytes());
    Ok(())
}

pub fn main(mut world: World) {
    if let Err(message) = run(&mut world) {
        let mut errors = world.stderr();
        errors.write(format!("{message}\n").as_bytes());
        world.set_exit_status(1u8);
    }
}
