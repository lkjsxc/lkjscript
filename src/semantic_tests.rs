//! Source/IR contracts; these helpers use the production compiler modules.
mod allocator;
mod array_repeat;
mod diagnostic_contracts;
mod entry_grants;
mod facts;
mod function_owners;
mod interpreter_contracts;
mod library_demand;
mod parser_contracts;
mod resolver;
mod reuse;
mod scalar_clone;
mod return_contracts;
mod source_cases;
mod source_contracts;
mod units;
use crate::{embedded, frontend, source, syntax};

fn source_file(path: &str, text: Option<&str>) -> source::SourceInput {
    let text = match text {
        Some(text) => text.to_string(),
        None => String::from_utf8(embedded::library(path).unwrap()).unwrap(),
    };
    source::SourceInput {
        path: path.to_string(),
        text,
    }
}
fn source_case_at(
    name: &str,
    root: &str,
    library: Option<&str>,
    files: Vec<source::SourceInput>,
    expected: &str,
    expected_path: &str,
    expected_line: usize,
    expected_column: usize,
) -> Result<(), String> {
    let library = library.map(|path| path.to_string());
    let result = frontend::check_bundle(&source::SourceBundle {
        root: root.to_string(),
        library,
        files,
    });
    match result {
        Ok(_) => {
            if expected.is_empty() {
                Ok(())
            } else {
                Err(format!("{name}: expected {expected}, accepted"))
            }
        }
        Err(error) => {
            if error.code == expected {
                if (!expected_path.is_empty() && error.path != expected_path)
                    || (expected_line != 0 && error.line != expected_line)
                    || (expected_column != 0 && error.column != expected_column)
                {
                    Err(format!(
                        "{name}: expected diagnostic {expected_path}:{expected_line}:{expected_column}, got {}:{}:{}",
                        error.path, error.line, error.column
                    ))
                } else {
                    Ok(())
                }
            } else {
                Err(format!(
                    "{name}: expected {expected}, got {}: {}",
                    error.code, error.message
                ))
            }
        }
    }
}
fn parse_case(name: &str, text: &str, expected: &str) -> Result<(), String> {
    match syntax::parse(text) {
        Ok(_) => Err(format!("{name}: expected {expected}, parser accepted")),
        Err(error) => {
            if error.code == expected {
                Ok(())
            } else {
                Err(format!("{name}: expected {expected}, got {}", error.code))
            }
        }
    }
}
fn record(result: Result<(), String>, errors: &mut Vec<String>) {
    if let Err(error) = result {
        errors.push(error);
    }
}
pub fn run(world: &mut World, shard: usize, workers: usize, trace: bool) -> (usize, Vec<String>) {
    let mut errors = Vec::new();
    let mut count = source_cases::run(shard, workers, &mut errors);
    if shard == 0 {
        count += units::run(world, trace);
        count += diagnostic_contracts::run(world, trace);
        count += function_owners::run(world, trace);
        count += library_demand::run(world, trace);
        count += source_contracts::run(world, trace);
        count += parser_contracts::run(world, trace);
        count += return_contracts::run(world, trace);
        count += array_repeat::run(world, trace);
        count += scalar_clone::run(world, trace);
        count += interpreter_contracts::run(world, trace);
        count += entry_grants::run(world, trace);
        count += allocator::run(world);
        count += facts::run();
        count += resolver::run();
        count += reuse::run(world, trace);
        crate::interpreter::assert_reference_reuse();
        count += 1;
    }
    (count, errors)
}

fn source_case(
    name: &str,
    root: &str,
    library: Option<&str>,
    files: Vec<source::SourceInput>,
    expected: &str,
) -> Result<(), String> {
    source_case_at(name, root, library, files, expected, "", 0, 0)
}
fn derived_source_case(
    name: &str,
    root: &str,
    library: Option<&str>,
    files: Vec<source::SourceInput>,
    expected: &str,
) -> Result<(), String> {
    let mut expected_line = 1usize;
    for file in &files {
        if file.path == root {
            let mut line_number = 1usize;
            for line in file.text.as_str().lines() {
                if line.contains("fn leak") || line.contains("fn invalid") {
                    expected_line = line_number;
                    break;
                }
                line_number += 1;
            }
        }
    }
    source_case_at(name, root, library, files, expected, root, expected_line, 0)
}
