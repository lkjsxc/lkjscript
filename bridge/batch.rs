//! Test host batching. Case failures are data; only protocol/host failures stop
//! the batch. Each case has a fresh compiler, machine, and module instance.
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
fn record(index: usize, checked: bool, status: i32, stdout: &[u8], stderr: &str) {
    println!(
        "{{\"index\":{index},\"checkStatus\":{},\"status\":{status},\"stdoutHex\":\"{}\",\"stderr\":{}}}",
        if checked { 0 } else { 1 },
        hex(stdout),
        quoted(stderr)
    );
}
#[derive(Default)]
struct Output(Vec<u8>);
impl machine::Host for Output {
    fn write_stdout(&mut self, bytes: &[u8]) -> Result<(), String> {
        self.0.extend_from_slice(bytes);
        Ok(())
    }
}
pub fn run(argv: &[String]) -> Result<(), String> {
    let Some(manifest) = argv.first() else {
        return Err("E_USAGE batch <manifest.tsv|directory> [--engine node|k|native]".to_string());
    };
    let engine = if argv.len() == 3 && argv[1] == "--engine" {
        argv[2].as_str()
    } else if argv.len() == 1 {
        "k"
    } else {
        return Err("E_USAGE batch <manifest.tsv|directory> [--engine node|k|native]".to_string());
    };
    if !["node", "k", "native"].contains(&engine) {
        return Err("E_USAGE engine is node, k or native".to_string());
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
        let mut jobs = Vec::new();
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
                engine: engine.to_string(),
                census: false,
                unchecked: false,
            };
            let mut timings = timing::Timings::new();
            let checked = match compile(&args.path, false, &mut timings) {
                Ok(value) => value,
                Err(error) => {
                    record(index, false, 1, &[], &error);
                    continue;
                }
            };
            if args.verb == "check" {
                record(index, true, 0, &[], "");
                continue;
            }
            let program = checked.program();
            let entry = match entries(program, &args) {
                Ok(entries) => entries[0],
                Err(error) => {
                    record(index, true, 1, &[], &error);
                    continue;
                }
            };
            if engine == "native" {
                let lowered = match native::lower(&checked) {
                    Ok(value) => value,
                    Err(error) => {
                        record(index, true, 1, &[], &format!("E_EMIT {error}"));
                        continue;
                    }
                };
                let bytes = match native::image(&lowered, program, entry, true) {
                    Ok(value) => value,
                    Err(error) => {
                        record(index, true, 1, &[], &format!("E_EMIT {error}"));
                        continue;
                    }
                };
                let path = directory.join(format!("{index}.elf"));
                native::save(&path, &bytes)?;
                match native::execute(&path, &lowered, &args.path, true) {
                    Ok(mut outcome) => {
                        if let Some(error) = outcome.error {
                            record(index, true, 1, &outcome.stdout, &error);
                        } else {
                            let value = match native::scalar(
                                program,
                                &program.functions[entry],
                                outcome.value,
                            ) {
                                Ok(value) => value,
                                Err(error) => {
                                    record(index, true, 1, &outcome.stdout, &error);
                                    continue;
                                }
                            };
                            outcome.stdout.extend_from_slice(scalar(value).as_bytes());
                            record(index, true, 0, &outcome.stdout, "");
                        }
                    }
                    Err(error) => record(index, true, 1, &[], &error),
                }
            } else if engine == "k" {
                let mut output = Output::default();
                match machine::run(program, entry, &[], &mut output) {
                    Ok(result) => {
                        output.0.extend_from_slice(scalar(result.value).as_bytes());
                        record(index, true, 0, &output.0, "");
                    }
                    Err(error) => record(
                        index,
                        true,
                        1,
                        &output.0,
                        &format!(
                            "{}:{}: {} {}",
                            args.path,
                            error.line,
                            error.code(),
                            error.message
                        ),
                    ),
                }
            } else {
                let function = &program.functions[entry];
                let world = entry_world(program, function)?;
                let bytes = match wasm::emit(&checked) {
                    Ok(bytes) => bytes,
                    Err(error) => {
                        record(
                            index,
                            true,
                            1,
                            &[],
                            &format!("{}:1:1: E_EMIT {error}", args.path),
                        );
                        continue;
                    }
                };
                let module = directory.join(format!("{index}.wasm"));
                fs::write(&module, bytes).map_err(|e| format!("E_IO {e}"))?;
                jobs.push(format!("{{\"index\":{index},\"path\":{},\"source\":{},\"entry\":{},\"kind\":{},\"args\":[{}]}}",quoted(&module.to_string_lossy()),quoted(&args.path),quoted(&function.name),quoted(result_kind(program,function)),if world {"\"1\""} else {""}));
            }
            timings.done();
        }
        if !jobs.is_empty() {
            let path = directory.join("modules.json");
            fs::write(&path, format!("[{}]", jobs.join(","))).map_err(|e| format!("E_IO {e}"))?;
            let mut command = Command::new("node");
            command
                .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("tools/wasm-batch.mjs"))
                .arg(path);
            host_process::prepare_child(&mut command);
            if !command
                .status()
                .map_err(|e| format!("E_HOST Node: {e}"))?
                .success()
            {
                return Err("E_HOST module batch did not complete".to_string());
            }
        }
        Ok(())
    })();
    let _ = fs::remove_dir_all(&directory);
    result
}
