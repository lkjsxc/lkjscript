//! Batch each source once, then execute it with fresh state on every route.
//! Case failures are data; only protocol/host failures stop the batch.
use super::*;

fn quoted(text: &str) -> String {
    let mut bytes = Vec::new();
    lkjscript::json::write_string(&mut bytes, text);
    String::from_utf8(bytes).expect("JSON string is UTF-8")
}
fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(char::from(DIGITS[usize::from(byte >> 4)]));
        out.push(char::from(DIGITS[usize::from(byte & 15)]));
    }
    out
}
fn record(index: usize, engine: &str, checked: bool, status: i32, stdout: &[u8], diagnostic: &str) {
    record_streams(index, engine, checked, status, stdout, &[], diagnostic);
}
fn record_streams(
    index: usize,
    engine: &str,
    checked: bool,
    status: i32,
    stdout: &[u8],
    stderr: &[u8],
    diagnostic: &str,
) {
    println!(
        "{{\"index\":{index},\"engine\":{},\"checkStatus\":{},\"status\":{status},\"stdoutHex\":\"{}\",\"stderrHex\":\"{}\",\"stderr\":{}}}",
        quoted(engine),
        if checked { 0 } else { 1 },
        hex(stdout),
        hex(stderr),
        quoted(diagnostic)
    );
}
pub fn run(argv: &[String]) -> Result<(), String> {
    let usage = "E_USAGE batch <manifest.tsv|directory> [--engine native|k | --engines native,k]";
    let Some(manifest) = argv.first() else {
        return Err(usage.to_string());
    };
    let engines = match argv.len() {
        1 => vec!["k"],
        3 if argv[1] == "--engine" => vec![argv[2].as_str()],
        3 if argv[1] == "--engines" => argv[2].split(',').collect(),
        _ => return Err(usage.to_string()),
    };
    for (index, engine) in engines.iter().enumerate() {
        if !["native", "k"].contains(engine) || engines[..index].contains(engine) {
            return Err("E_USAGE engines must be distinct names from native,k".to_string());
        }
    }
    let requests = if Path::new(manifest).is_dir() {
        let mut files = fs::read_dir(manifest)
            .map_err(|e| format!("E_IO {e}"))?
            .map(|entry| entry.map(|entry| entry.path()))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("E_IO {e}"))?;
        files.sort();
        files
            .into_iter()
            .filter(|path| {
                path.extension()
                    .is_some_and(|ext| ext == "rs" || ext == "k")
            })
            .map(|path| format!("run\t\t{}", path.display()))
            .collect::<Vec<_>>()
            .join("\n")
    } else {
        fs::read_to_string(manifest).map_err(|e| format!("E_IO {e}"))?
    };
    let directory = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target/batch")
        .join(std::process::id().to_string());
    fs::create_dir_all(&directory).map_err(|e| format!("E_IO {e}"))?;
    let result = (|| {
        for (index, row) in requests.lines().enumerate() {
            let columns: Vec<_> = row.splitn(3, '\t').collect();
            if columns.len() != 3 || !["check", "run"].contains(&columns[0]) {
                return Err(format!("E_USAGE invalid batch request {}", index + 1));
            }
            let args = Arguments {
                verb: columns[0].to_string(),
                entry: (!columns[1].is_empty()).then(|| columns[1].to_string()),
                path: columns[2].to_string(),
                output: None,
                engine: engines[0].to_string(),
                census: false,
                unchecked: false,
                program_arguments: Vec::new(),
            };
            let mut timings = timing::Timings::new();
            let checked = match compile(&args.path, false, &mut timings) {
                Ok(value) => value,
                Err(error) => {
                    for engine in &engines {
                        record(index, engine, false, 1, &[], &error);
                    }
                    continue;
                }
            };
            if args.verb == "check" {
                for engine in &engines {
                    record(index, engine, true, 0, &[], "");
                }
                timings.done();
                continue;
            }
            let program = checked.program();
            let entry = match entries(program, &args) {
                Ok(entries) => entries[0],
                Err(error) => {
                    for engine in &engines {
                        record(index, engine, true, 1, &[], &error);
                    }
                    continue;
                }
            };
            // Layout, glue and runtime linking are route independent too.
            let lowered = engines
                .iter()
                .any(|engine| *engine != "k")
                .then(|| timings.measure("lower", || native::lower(&checked)));
            for engine in &engines {
                if *engine == "k" {
                    let mut output = world::Host::captured();
                    let resources = host_process::machine_resources()?;
                    match timings.measure("machine", || {
                        machine::run(program, entry, &[], &mut output, resources)
                    }) {
                        Ok(result) => {
                            output
                                .stdout
                                .extend_from_slice(scalar(result.value).as_bytes());
                            record_streams(
                                index,
                                engine,
                                true,
                                i32::from(output.status),
                                &output.stdout,
                                &output.stderr,
                                "",
                            );
                        }
                        Err(error) => record_streams(
                            index,
                            engine,
                            true,
                            1,
                            &output.stdout,
                            &output.stderr,
                            &format!(
                                "{}:{}: {} {}",
                                args.path,
                                error.line,
                                error.code(),
                                error.message
                            ),
                        ),
                    }
                    continue;
                }
                let lowered = match lowered.as_ref().expect("encoded route has lowering") {
                    Ok(value) => value,
                    Err(error) => {
                        record(index, engine, true, 1, &[], &format!("E_EMIT {error}"));
                        continue;
                    }
                };
                if *engine == "native" {
                    let bytes = match timings.measure("native_encode", || {
                        native::image(lowered, program, entry, true)
                    }) {
                        Ok(value) => value,
                        Err(error) => {
                            record(index, engine, true, 1, &[], &format!("E_EMIT {error}"));
                            continue;
                        }
                    };
                    let path = directory.join(format!("{index}.elf"));
                    native::save(&path, &bytes)?;
                    match timings.measure("native_run", || {
                        native::execute(&path, lowered, &args.path, true, &args.program_arguments)
                    }) {
                        Ok(mut outcome) => {
                            if let Some(error) = outcome.error {
                                record_streams(
                                    index,
                                    engine,
                                    true,
                                    1,
                                    &outcome.stdout,
                                    &outcome.stderr,
                                    &error,
                                );
                            } else {
                                let value = match native::scalar(
                                    program,
                                    &program.functions[entry],
                                    outcome.value,
                                ) {
                                    Ok(value) => value,
                                    Err(error) => {
                                        record(index, engine, true, 1, &outcome.stdout, &error);
                                        continue;
                                    }
                                };
                                outcome.stdout.extend_from_slice(scalar(value).as_bytes());
                                record_streams(
                                    index,
                                    engine,
                                    true,
                                    i32::from(outcome.status),
                                    &outcome.stdout,
                                    &outcome.stderr,
                                    "",
                                );
                            }
                        }
                        Err(error) => record(index, engine, true, 1, &[], &error),
                    }
                }
            }
            timings.done();
        }

        Ok(())
    })();
    let _ = fs::remove_dir_all(&directory);
    result
}
