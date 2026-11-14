// Run with a compiler executable and an existing empty work directory.
// This suite uses only lkjscript capabilities and leaves that directory empty.
struct Outcome {
    status: i64,
    output: Vec<u8>,
    errors: Vec<u8>,
    result: Vec<u8>,
}

struct Config {
    directory: String,
    options: Vec<Vec<u8>>,
}

struct Report {
    failures: Vec<String>,
}

impl Report {
    fn require(&mut self, condition: bool, message: String) {
        if !condition {
            self.failures.push(message);
        }
    }

    fn equal(&mut self, label: &str, stream: &str, actual: &[u8], expected: &[u8]) {
        if actual != expected {
            let mut index = 0usize;
            while index < actual.len() && index < expected.len() && actual[index] == expected[index]
            {
                index = index + 1;
            }
            self.failures.push(format!(
                "{label}: {stream} differs at byte {index}; expected {} bytes, got {} bytes",
                expected.len(),
                actual.len()
            ));
        }
    }

    fn outcome(
        &mut self,
        label: &str,
        actual: &Outcome,
        status: i64,
        output: &[u8],
        errors: &[u8],
    ) {
        if actual.status != status {
            self.failures.push(format!(
                "{label}: expected status {status}, got {}; stderr {}",
                actual.status,
                display_bytes(&actual.errors)
            ));
        }
        self.equal(label, "stdout", &actual.output, output);
        self.equal(label, "stderr", &actual.errors, errors);
        self.equal(label, "driver result descriptor", &actual.result, b"");
    }
}

fn display_bytes(bytes: &[u8]) -> String {
    let mut text = String::new();
    for byte in bytes {
        if *byte >= 32 && *byte < 127 {
            text.push(*byte as char);
        } else {
            text.push_str(&format!("\\x{:02x}", *byte));
        }
    }
    text
}

fn contains(bytes: &[u8], needle: &[u8]) -> bool {
    if needle.len() > bytes.len() {
        return false;
    }
    let mut index = 0usize;
    while index <= bytes.len() - needle.len() {
        if &bytes[index..index + needle.len()] == needle {
            return true;
        }
        index = index + 1;
    }
    false
}

fn io_result<T>(result: Result<T, i64>, label: &str) -> Result<T, String> {
    match result {
        Ok(value) => Ok(value),
        Err(errno) => Err(format!("{label}: errno {errno}")),
    }
}

fn text(bytes: Vec<u8>, label: &str) -> Result<String, String> {
    match String::from_utf8(bytes) {
        Ok(value) => Ok(value),
        Err(_) => Err(format!("{label} is not UTF-8")),
    }
}

fn temporary(world: &mut World) -> Result<File, String> {
    io_result(world.temporary_file(), &format!("temporary file"))
}

fn invoke(
    world: &mut World,
    executable: &File,
    arguments: &[Vec<u8>],
    input: &[u8],
) -> Result<Outcome, String> {
    let mut stdin: File = temporary(world)?;
    let mut stdout: File = temporary(world)?;
    let mut stderr: File = temporary(world)?;
    let mut result: File = temporary(world)?;
    if let Err(errno) = stdin.write_all(input) {
        return Err(format!("input write: errno {errno}"));
    }
    io_result(stdin.rewind(), &format!("input rewind"))?;
    let mut process = world.process();
    let mut child = io_result(
        process.spawn_file(executable, arguments, &stdin, &stdout, &stderr, &result),
        &format!("spawn compiler"),
    )?;
    let status = io_result(child.wait(), &format!("wait compiler"))?;
    io_result(stdout.rewind(), &format!("output rewind"))?;
    io_result(stderr.rewind(), &format!("error rewind"))?;
    io_result(result.rewind(), &format!("result rewind"))?;
    let output = io_result(stdout.read_to_end(), &format!("output read"))?;
    let errors = io_result(stderr.read_to_end(), &format!("error read"))?;
    let result = io_result(result.read_to_end(), &format!("result read"))?;
    Ok(Outcome {
        status,
        output,
        errors,
        result,
    })
}

fn path(config: &Config, name: &str) -> String {
    format!("{}/{name}", config.directory)
}

fn command(config: &Config, verb: &str, name: &str) -> Vec<Vec<u8>> {
    let mut arguments = vec![
        verb.as_bytes().to_vec(),
        path(config, name).as_bytes().to_vec(),
    ];
    for option in &config.options {
        arguments.push(option.clone());
    }
    arguments
}

fn option(arguments: &mut Vec<Vec<u8>>, name: &str, value: &str) {
    arguments.push(name.as_bytes().to_vec());
    arguments.push(value.as_bytes().to_vec());
}

fn write(directory: &mut Directory, name: &str, bytes: &[u8]) -> Result<(), String> {
    let mut file = match directory.create(name) {
        Ok(file) => file,
        Err(errno) => return Err(format!("create {name}: errno {errno}")),
    };
    match file.write_all(bytes) {
        Ok(()) => Ok(()),
        Err(errno) => Err(format!("write {name}: errno {errno}")),
    }
}

fn read(directory: &mut Directory, name: &str) -> Result<Vec<u8>, String> {
    let mut file = match directory.open(name) {
        Ok(file) => file,
        Err(errno) => return Err(format!("open {name}: errno {errno}")),
    };
    io_result(file.read_to_end(), &format!("read {name}"))
}

fn compile_ir(
    world: &mut World,
    executable: &File,
    config: &Config,
    report: &mut Report,
    source: &str,
    output: &str,
) -> Result<(), String> {
    let arguments = command(config, "check", source);
    let result = invoke(world, executable, &arguments, b"")?;
    report.outcome(&format!("check {source}"), &result, 0, b"", b"");
    let mut arguments = command(config, "ir", source);
    option(&mut arguments, "-o", &path(config, output));
    let result = invoke(world, executable, &arguments, b"")?;
    report.outcome(&format!("ir {source}"), &result, 0, b"", b"");
    let arguments = command(config, "check", output);
    let result = invoke(world, executable, &arguments, b"")?;
    report.outcome(&format!("check {output}"), &result, 0, b"", b"");
    Ok(())
}

fn run_value(
    world: &mut World,
    executable: &File,
    config: &Config,
    report: &mut Report,
    name: &str,
    entry: &str,
    expected: &str,
) -> Result<(), String> {
    for engine in pair("native", "interpreter") {
        let mut arguments = command(config, "run", name);
        option(&mut arguments, "--entry", entry);
        option(&mut arguments, "--engine", &engine);
        let result = invoke(world, executable, &arguments, b"")?;
        report.outcome(
            &format!("run {name} {entry} {engine}"),
            &result,
            0,
            expected.as_bytes(),
            b"",
        );
    }
    Ok(())
}

fn pair(first: &str, second: &str) -> Vec<String> {
    vec![first.to_string(), second.to_string()]
}

struct Scalar {
    name: String,
    kind: String,
    expression: String,
    expected: String,
}

fn scalar(name: &str, kind: &str, expression: &str, expected: &str) -> Scalar {
    Scalar {
        name: name.to_string(),
        kind: kind.to_string(),
        expression: expression.to_string(),
        expected: expected.to_string(),
    }
}

fn scalars() -> Vec<Scalar> {
    vec![
        scalar("probe", "i64", "42", "42\n"),
        scalar("i8_min", "i8", "-128", "-128\n"),
        scalar("i8_max", "i8", "127", "127\n"),
        scalar("u8_max", "u8", "255", "255\n"),
        scalar("i16_min", "i16", "-32768", "-32768\n"),
        scalar("i16_max", "i16", "32767", "32767\n"),
        scalar("u16_max", "u16", "65535", "65535\n"),
        scalar("i32_min", "i32", "-2147483648", "-2147483648\n"),
        scalar("i32_max", "i32", "2147483647", "2147483647\n"),
        scalar("u32_max", "u32", "4294967295", "4294967295\n"),
        scalar(
            "i64_min",
            "i64",
            "-9223372036854775808",
            "-9223372036854775808\n",
        ),
        scalar(
            "i64_max",
            "i64",
            "9223372036854775807",
            "9223372036854775807\n",
        ),
        scalar(
            "u64_max",
            "u64",
            "18446744073709551615",
            "18446744073709551615\n",
        ),
        scalar(
            "isize_min",
            "isize",
            "-9223372036854775808",
            "-9223372036854775808\n",
        ),
        scalar(
            "usize_max",
            "usize",
            "18446744073709551615",
            "18446744073709551615\n",
        ),
        scalar("boolean_true", "bool", "true", "true\n"),
        scalar("boolean_false", "bool", "false", "false\n"),
        scalar("unicode", "char", "'🦀'", "🦀\n"),
        scalar("character_zero", "char", "'\\0'", "\0\n"),
        scalar(
            "float_zero",
            "f64",
            "f64::from_bits(0)",
            "f64:0x0000000000000000\n",
        ),
        scalar(
            "float_negative_zero",
            "f64",
            "f64::from_bits(0x8000000000000000)",
            "f64:0x8000000000000000\n",
        ),
        scalar(
            "float_infinity",
            "f64",
            "f64::from_bits(0x7ff0000000000000)",
            "f64:0x7ff0000000000000\n",
        ),
        scalar(
            "float_nan",
            "f64",
            "f64::from_bits(0x7ff8000000000042)",
            "f64:0x7ff8000000000042\n",
        ),
        scalar(
            "float_signaling_nan",
            "f64",
            "f64::from_bits(0xfff0000000000001)",
            "f64:0xfff0000000000001\n",
        ),
        scalar("unit", "()", "()", ""),
    ]
}

fn exercise(
    world: &mut World,
    executable: &File,
    directory: &mut Directory,
    config: &Config,
    report: &mut Report,
) -> Result<(), String> {
    let cases = scalars();
    let mut source = String::new();
    for case in &cases {
        source.push_str(&format!(
            "fn {}() -> {} {{ {} }}\n",
            case.name, case.kind, case.expression
        ));
    }
    write(directory, "scalars.rs", source.as_bytes())?;
    compile_ir(
        world,
        executable,
        config,
        report,
        "scalars.rs",
        "scalars.lkjir",
    )?;
    run_value(
        world,
        executable,
        config,
        report,
        "scalars.rs",
        "probe",
        "42\n",
    )?;
    for case in &cases {
        run_value(
            world,
            executable,
            config,
            report,
            "scalars.lkjir",
            &case.name,
            &case.expected,
        )?;
    }
    let original = read(directory, "scalars.lkjir")?;
    let arguments = command(config, "ir", "scalars.lkjir");
    let result = invoke(world, executable, &arguments, b"")?;
    report.outcome("IR text roundtrip", &result, 0, &original, b"");
    let mut arguments = command(config, "check", "scalars.rs");
    arguments.push("--stats".as_bytes().to_vec());
    let result = invoke(world, executable, &arguments, b"")?;
    report.require(
        result.status == 0 && result.output.is_empty(),
        "check --stats failed".to_string(),
    );
    report.require(
        contains(&result.errors, b"LKJSCRIPT_STATS {")
            && contains(&result.errors, b"\"total_ms\":")
            && contains(&result.errors, b"\"type_ms\":"),
        "check --stats omitted phase timings".to_string(),
    );

    let io_source = "fn main(mut world: World) {\n    let arguments = world.args();\n    let mut input = world.stdin();\n    let bytes = input.read_to_end().unwrap();\n    let mut out = world.stdout();\n    out.write(b\"prefix\\0\\xff\\n\");\n    out.write(&bytes);\n    for argument in arguments { out.write(b\"[\"); out.write(&argument); out.write(b\"]\"); }\n    let mut errors = world.stderr();\n    errors.write(b\"error\\0\\xfe\\n\");\n    world.set_exit_status(7);\n}\n";
    write(directory, "streams.rs", io_source.as_bytes())?;
    compile_ir(
        world,
        executable,
        config,
        report,
        "streams.rs",
        "streams.lkjir",
    )?;
    let program_arguments = vec!["hello".as_bytes().to_vec(), Vec::new(), vec![255u8, 120u8]];
    let input = b"input\0\xff\n";
    let expected_output = b"prefix\0\xff\ninput\0\xff\n[hello][][\xffx]";
    let expected_errors = b"error\0\xfe\n";
    for name in pair("streams.rs", "streams.lkjir") {
        for engine in pair("native", "interpreter") {
            let mut arguments = command(config, "run", &name);
            option(&mut arguments, "--engine", &engine);
            arguments.push("--".as_bytes().to_vec());
            for argument in &program_arguments {
                arguments.push(argument.clone());
            }
            let result = invoke(world, executable, &arguments, input)?;
            report.outcome(
                &format!("binary streams {name} {engine}"),
                &result,
                7,
                expected_output,
                expected_errors,
            );
        }
        let artifact = if name == "streams.rs" {
            "from-source"
        } else {
            "from-ir"
        };
        let mut arguments = command(config, "build", &name);
        option(&mut arguments, "-o", &path(config, artifact));
        let result = invoke(world, executable, &arguments, b"")?;
        report.outcome(&format!("build {name}"), &result, 0, b"", b"");
        let built = io_result(directory.open(artifact), &format!("open {artifact}"))?;
        let result = invoke(world, &built, &program_arguments, input)?;
        report.outcome(
            &format!("standalone {artifact}"),
            &result,
            7,
            expected_output,
            expected_errors,
        );
    }
    let from_source = read(directory, "from-source")?;
    let from_ir = read(directory, "from-ir")?;
    report.equal("source and IR build", "artifact", &from_source, &from_ir);

    let trap_source = "fn main(mut world: World) {\n    let mut out = world.stdout();\n    out.write(b\"before\\0\"); world.stderr().write(b\"prefix\\0\\xff\");\n    let maximum: i64 = 9223372036854775807;\n    let answer = maximum + 1;\n}\n";
    write(directory, "trap.rs", trap_source.as_bytes())?;
    compile_ir(world, executable, config, report, "trap.rs", "trap.lkjir")?;
    for name in pair("trap.rs", "trap.lkjir") {
        for engine in pair("native", "interpreter") {
            let mut arguments = command(config, "run", &name);
            option(&mut arguments, "--engine", &engine);
            let result = invoke(world, executable, &arguments, b"")?;
            let label = format!("trap {name} {engine}");
            report.require(
                result.status == 1,
                format!("{label}: expected status 1, got {}", result.status),
            );
            report.equal(&label, "stdout before trap", &result.output, b"before\0");
            report.require(
                contains(&result.errors, b"prefix\0\xff"),
                format!("{label}: missing stderr bytes before trap"),
            );
            let location = ":5: R0001 ";
            report.require(
                contains(&result.errors, location.as_bytes()),
                format!(
                    "{label}: missing exact code and line: {}",
                    display_bytes(&result.errors)
                ),
            );
        }
    }

    io_result(
        directory.create_directory("assets"),
        &format!("create assets"),
    )?;
    write(directory, "assets/text.txt", b"embed\n")?;
    write(directory, "assets/data.bin", b"\0\xff\x7f*")?;
    let include_source = "fn probe() -> i64 {\n    let text = include_str!(\"assets/text.txt\");\n    let bytes = include_bytes!(\"assets/data.bin\");\n    assert_eq!(text, \"embed\\n\");\n    assert_eq!(bytes.len(), 4);\n    assert_eq!(bytes[0], 0u8);\n    assert_eq!(bytes[3], 42u8);\n    bytes[1] as i64 + text.len() as i64\n}\n";
    write(directory, "includes.rs", include_source.as_bytes())?;
    compile_ir(
        world,
        executable,
        config,
        report,
        "includes.rs",
        "includes.lkjir",
    )?;
    run_value(
        world,
        executable,
        config,
        report,
        "includes.rs",
        "probe",
        "261\n",
    )?;
    io_result(
        directory.remove("assets/text.txt"),
        &format!("remove embedded text"),
    )?;
    io_result(
        directory.remove("assets/data.bin"),
        &format!("remove embedded bytes"),
    )?;
    run_value(
        world,
        executable,
        config,
        report,
        "includes.lkjir",
        "probe",
        "261\n",
    )?;

    let self_spawn = include_str!("../tests/fixtures/world/self_spawn.rs");
    write(directory, "self-spawn.rs", self_spawn.as_bytes())?;
    compile_ir(
        world,
        executable,
        config,
        report,
        "self-spawn.rs",
        "self-spawn.lkjir",
    )?;
    run_value(
        world,
        executable,
        config,
        report,
        "self-spawn.rs",
        "main",
        "42\n",
    )?;
    run_value(
        world,
        executable,
        config,
        report,
        "self-spawn.lkjir",
        "main",
        "42\n",
    )?;
    let mut arguments = command(config, "run", "self-spawn.lkjir");
    option(&mut arguments, "--engine", "interpreter");
    arguments.push("--unchecked-ir".as_bytes().to_vec());
    arguments.push("--".as_bytes().to_vec());
    arguments.push("unchecked".as_bytes().to_vec());
    let result = invoke(world, executable, &arguments, b"")?;
    report.outcome(
        "unchecked IR cannot emit native executable",
        &result,
        0,
        b"unchecked ok\n",
        b"",
    );
    let mut arguments = command(config, "run", "scalars.lkjir");
    option(&mut arguments, "--engine", "interpreter");
    option(
        &mut arguments,
        "--runtime-source",
        &path(config, "unneeded-runtime.rs"),
    );
    let result = invoke(world, executable, &arguments, b"")?;
    report.outcome(
        "interpreter executable preparation is lazy",
        &result,
        0,
        b"42\n",
        b"",
    );

    let tests = "#[test]\nfn first() { assert_eq!(6 * 7, 42); }\n#[test]\nfn second() { let values = vec![3, 4]; assert_eq!(values[0] + values[1], 7); }\nfn probe() -> i64 { 42 }\n";
    write(directory, "tests.rs", tests.as_bytes())?;
    compile_ir(world, executable, config, report, "tests.rs", "tests.lkjir")?;
    for name in pair("tests.rs", "tests.lkjir") {
        for engine in pair("native", "interpreter") {
            let mut arguments = command(config, "test", &name);
            option(&mut arguments, "--engine", &engine);
            let result = invoke(world, executable, &arguments, b"")?;
            report.outcome(
                &format!("test {name} {engine}"),
                &result,
                0,
                b"ok 2 tests\n",
                b"",
            );
        }
        let arguments = command(config, "test", &name);
        let result = invoke(world, executable, &arguments, b"")?;
        report.outcome(
            &format!("test {name} both routes"),
            &result,
            0,
            b"ok 4 tests\n",
            b"",
        );
    }

    write(directory, "invalid.rs", b"fn probe() -> i64 { true }\n")?;
    write(directory, "invalid.lkjir", b"lkjscript-ir 999\n")?;
    write(directory, "preserved", b"existing artifact\0\xff")?;
    for name in pair("invalid.rs", "invalid.lkjir") {
        let diagnostic = if name == "invalid.rs" {
            "E_TYPE"
        } else {
            "E_IR"
        };
        let arguments = command(config, "check", &name);
        let result = invoke(world, executable, &arguments, b"")?;
        report.require(
            result.status == 1 && contains(&result.errors, diagnostic.as_bytes()),
            format!(
                "check {name}: expected diagnostic {diagnostic}, got {}",
                display_bytes(&result.errors)
            ),
        );
        for artifact in pair("absent", "preserved") {
            let mut arguments = command(config, "build", &name);
            option(&mut arguments, "-o", &path(config, &artifact));
            let result = invoke(world, executable, &arguments, b"")?;
            report.require(
                result.status == 1 && contains(&result.errors, diagnostic.as_bytes()),
                format!(
                    "build {name}: expected diagnostic {diagnostic}, got {}",
                    display_bytes(&result.errors)
                ),
            );
        }
        report.require(
            directory.open("absent").is_err(),
            format!("build {name} published a rejected artifact"),
        );
        let preserved = read(directory, "preserved")?;
        report.equal(
            &format!("build {name}"),
            "existing artifact",
            &preserved,
            b"existing artifact\0\xff",
        );
    }
    Ok(())
}

fn clean(directory: &mut Directory) -> Result<(), String> {
    let names: Vec<Vec<u8>> = match directory.list() {
        Ok(names) => names,
        Err(errno) => return Err(format!("cleanup list: errno {errno}")),
    };
    for bytes in names {
        let name = text(bytes, "cleanup name")?;
        match directory.open_directory(&name) {
            Ok(mut child) => {
                clean(&mut child)?;
                drop(child);
                io_result(
                    directory.remove_directory(&name),
                    &format!("cleanup directory {name}"),
                )?;
            }
            Err(_) => {
                io_result(directory.remove(&name), &format!("cleanup file {name}"))?;
            }
        }
    }
    Ok(())
}

fn open_executable(world: &mut World, path: &str) -> Result<File, String> {
    let mut slash = None;
    let bytes = path.as_bytes();
    let mut index = 0usize;
    while index < bytes.len() {
        if bytes[index] == b'/' {
            slash = Some(index);
        }
        index = index + 1;
    }
    let parent: String = match slash {
        Some(0) => "/".to_string(),
        Some(at) => path[..at].to_string(),
        None => ".".to_string(),
    };
    let name: String = match slash {
        Some(at) => path[at + 1..].to_string(),
        None => path.to_string(),
    };
    let mut directory = io_result(world.directory(&parent), &format!("compiler directory"))?;
    match directory.open(&name) {
        Ok(file) => Ok(file),
        Err(errno) => Err(format!("open compiler: errno {errno}")),
    }
}

fn run(world: &mut World, report: &mut Report) -> Result<(), String> {
    let arguments = world.args();
    if arguments.len() < 2 {
        return Err("usage: toolchain-test COMPILER EMPTY-DIRECTORY [--library-dir DIR] [--runtime-source PATH]".to_string());
    }
    let compiler = text(arguments[0].clone(), "compiler path")?;
    let directory_path = text(arguments[1].clone(), "work directory")?;
    let mut options = Vec::new();
    let mut index = 2usize;
    while index < arguments.len() {
        let name = text(arguments[index].clone(), "option")?;
        if (name != "--library-dir" && name != "--runtime-source") || index + 1 == arguments.len() {
            return Err(format!("unsupported or incomplete harness option {name}"));
        }
        options.push(arguments[index].clone());
        options.push(arguments[index + 1].clone());
        index = index + 2;
    }
    let executable = open_executable(world, &compiler)?;
    let mut directory = io_result(world.directory(&directory_path), &format!("work directory"))?;
    let names = io_result(directory.list(), &format!("work directory list"))?;
    if !names.is_empty() {
        return Err("work directory must be empty".to_string());
    }
    let config = Config {
        directory: directory_path,
        options,
    };
    let outcome = exercise(world, &executable, &mut directory, &config, report);
    let cleanup = clean(&mut directory);
    if let Err(problem) = outcome {
        report.failures.push(problem);
    }
    if let Err(problem) = cleanup {
        report.failures.push(problem);
    }
    Ok(())
}

fn main(mut world: World) {
    let mut report = Report {
        failures: Vec::new(),
    };
    if let Err(problem) = run(&mut world, &mut report) {
        report.failures.push(problem);
    }
    if report.failures.is_empty() {
        let mut output = world.stdout();
        output.write(b"ok toolchain\n");
    } else {
        let mut errors = world.stderr();
        for failure in report.failures {
            errors.write(format!("FAIL {failure}\n").as_bytes());
        }
        world.set_exit_status(1);
    }
}
