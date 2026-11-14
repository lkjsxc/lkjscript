// Read native evaluation metadata and the parent's result/status control stream.
// Target stdout and stderr remain ordinary inherited process streams.
pub mod result_wire;

fn arguments(world: &mut World) -> Result<Vec<String>, String> {
    let mut arguments: Vec<String> = Vec::new();
    for bytes in world.args() {
        match String::from_utf8(bytes) {
            Ok(text) => arguments.push(text),
            Err(_) => return Err("arguments must be valid UTF-8".to_string()),
        }
    }
    if arguments.len() != 2 {
        return Err("usage: result DESCRIPTOR LABEL".to_string());
    }
    Ok(arguments)
}

fn run(world: &mut World) -> Result<u8, String> {
    let arguments = arguments(world)?;
    let path = &arguments[0];
    let label = &arguments[1];
    let mut directory = match world.directory(".") {
        Ok(directory) => directory,
        Err(errno) => return Err(format!(".: open directory failed (errno {errno})")),
    };
    let mut file = match directory.open(path) {
        Ok(file) => file,
        Err(errno) => return Err(format!("{path}: open failed (errno {errno})")),
    };
    let bytes = match file.read_to_end() {
        Ok(bytes) => bytes,
        Err(errno) => return Err(format!("{path}: read failed (errno {errno})")),
    };
    let descriptor = match result_wire::decode(&bytes) {
        Ok(descriptor) => descriptor,
        Err(message) => return Err(format!("{path}: E_HOST {message}")),
    };
    let mut input = world.stdin();
    let control = match input.read_to_end() {
        Ok(bytes) => bytes,
        Err(errno) => return Err(format!("{label}: control read failed (errno {errno})")),
    };
    let outcome = match result_wire::evaluate(&descriptor, label, &control) {
        Ok(outcome) => outcome,
        Err(message) => return Err(format!("{label}: E_HOST {message}")),
    };
    if !outcome.stdout.is_empty() {
        let mut output = world.stdout();
        output.write(outcome.stdout.as_bytes());
    }
    if !outcome.stderr.is_empty() {
        let mut errors = world.stderr();
        errors.write(outcome.stderr.as_bytes());
    }
    Ok(outcome.status)
}

pub fn main(mut world: World) {
    match run(&mut world) {
        Ok(status) => world.set_exit_status(status),
        Err(message) => {
            let mut errors = world.stderr();
            errors.write(format!("{message}\n").as_bytes());
            world.set_exit_status(1u8);
        }
    }
}
