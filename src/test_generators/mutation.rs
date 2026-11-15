// Direct, deterministic attacks on IR text. The checker is never consulted here.
use super::{integer, Random};

#[derive(Clone, PartialEq, Eq)]
pub struct Edit {
    pub line: usize,
    pub before: String,
    pub after: Vec<String>,
}
#[derive(Clone, PartialEq, Eq)]
pub struct Mutation {
    pub kind: String,
    pub edits: Vec<Edit>,
}
pub struct Seed {
    pub path: String,
    pub text: String,
    pub groups: Vec<Vec<Mutation>>,
}
struct Row {
    line: usize,
    word: Vec<String>,
}
struct Function {
    locals: Vec<Row>,
    statements: Vec<Row>,
}

pub fn lines(text: &str) -> Vec<String> {
    let mut result = Vec::new();
    let mut start = 0usize;
    for (index, byte) in text.as_bytes().iter().enumerate() {
        if *byte == b'\n' {
            result.push(text[start..index].to_string());
            start = index + 1;
        }
    }
    if start < text.len() {
        result.push(text[start..].to_string());
    }
    result
}

// Preserve quoted tokens exactly, including escaped quotes. Comments are ignored.
fn words(text: &str) -> Vec<String> {
    let bytes = text.as_bytes();
    let mut at = 0usize;
    let mut result = Vec::new();
    while at < bytes.len() {
        if bytes[at].is_ascii_whitespace() {
            at += 1;
        } else if bytes[at] == b'#' {
            break;
        } else {
            let start = at;
            if bytes[at] == b'"' {
                at += 1;
                while at < bytes.len() {
                    if bytes[at] == b'\\' {
                        at = (at + 2).min(bytes.len());
                    } else if bytes[at] == b'"' {
                        at += 1;
                        break;
                    } else {
                        at += 1;
                    }
                }
            } else {
                while at < bytes.len() && !bytes[at].is_ascii_whitespace() {
                    at += 1;
                }
            }
            result.push(text[start..at].to_string());
        }
    }
    result
}
fn joined(word: &[String]) -> String {
    let mut out = String::new();
    for (index, text) in word.iter().enumerate() {
        if index != 0 {
            out.push(' ');
        }
        out.push_str(text);
    }
    out
}
fn add(groups: &mut Vec<Vec<Mutation>>, kind: &str, edits: Vec<Edit>) {
    if edits
        .iter()
        .all(|edit| edit.after.len() == 1 && edit.after[0] == edit.before)
    {
        return;
    }
    let mut index = 0usize;
    while index < groups.len() {
        if groups[index][0].kind == kind {
            groups[index].push(Mutation {
                kind: kind.to_string(),
                edits,
            });
            return;
        }
        index += 1;
    }
    groups.push(vec![Mutation {
        kind: kind.to_string(),
        edits,
    }]);
}
fn edit(lines: &[String], line: usize, after: Vec<String>) -> Edit {
    Edit {
        line,
        before: lines[line].clone(),
        after,
    }
}
fn replace(
    groups: &mut Vec<Vec<Mutation>>,
    lines: &[String],
    kind: &str,
    row: &Row,
    position: usize,
    value: &str,
) {
    if position >= row.word.len() || row.word[position] == value {
        return;
    }
    let mut word = row.word.clone();
    word[position] = format!("{value}");
    add(
        groups,
        kind,
        vec![edit(lines, row.line, vec![joined(&word)])],
    );
}
fn control(word: &str) -> bool {
    matches!(
        word,
        "if" | "else"
            | "end"
            | "loop"
            | "break"
            | "continue"
            | "break-to"
            | "continue-to"
            | "return"
    )
}
fn pure_type(word: &str) -> bool {
    matches!(
        word,
        "unit"
            | "bool"
            | "int"
            | "f64"
            | "char"
            | "str"
            | "ref"
            | "array"
            | "slice"
            | "box"
            | "vec"
            | "struct"
            | "enum"
    )
}
fn authority_intrinsic(word: &str) -> bool {
    for prefix in vec![
        "\"raw_".to_string(),
        "\"world_".to_string(),
        "\"stdin_".to_string(),
        "\"stdout_".to_string(),
        "\"stderr_".to_string(),
        "\"directory_".to_string(),
        "\"file_".to_string(),
        "\"process_".to_string(),
        "\"child_".to_string(),
        "\"clock_".to_string(),
        "\"listener_".to_string(),
        "\"connector_".to_string(),
        "\"socket_".to_string(),
    ] {
        if word.starts_with(&prefix) {
            return true;
        }
    }
    false
}
pub fn prepare(path: &str, text: String) -> Result<Seed, String> {
    let rows = lines(&text);
    let mut declarations = Vec::new();
    let mut functions: Vec<Function> = Vec::new();
    let mut active = false;
    for (line, input) in rows.iter().enumerate() {
        let word = words(input);
        if word.is_empty() {
            continue;
        }
        if word[0] == "type" {
            if word.len() < 3 || !pure_type(&word[2]) {
                return Err(format!("{path}: mutation seed contains an authority type"));
            }
            declarations.push(Row { line, word });
        } else if word[0] == "fn" {
            functions.push(Function {
                locals: Vec::new(),
                statements: Vec::new(),
            });
            active = true;
        } else if word[0] == "endfn" {
            active = false;
        } else if active {
            let index = functions.len() - 1;
            if word[0] == "local" {
                functions[index].locals.push(Row { line, word });
            } else if word[0].starts_with('@') {
                if word.len() < 2 {
                    return Err(format!("{path}: malformed mutation seed statement"));
                }
                if word[1] == "intrinsic" && word.len() > 3 && authority_intrinsic(&word[3]) {
                    return Err(format!(
                        "{path}: mutation seed contains a raw or authority intrinsic"
                    ));
                }
                functions[index].statements.push(Row { line, word });
            }
        }
    }
    if declarations.is_empty() || functions.is_empty() {
        return Err(format!("{path}: mutation seed needs types and functions"));
    }
    let mut groups: Vec<Vec<Mutation>> = Vec::new();
    for function in &functions {
        let mut previous: Option<usize> = None;
        for statement in &function.statements {
            if !control(&statement.word[1]) {
                add(
                    &mut groups,
                    "delete",
                    vec![edit(&rows, statement.line, Vec::new())],
                );
                add(
                    &mut groups,
                    "duplicate",
                    vec![edit(
                        &rows,
                        statement.line,
                        vec![rows[statement.line].clone(), rows[statement.line].clone()],
                    )],
                );
                if let Some(line) = previous {
                    add(
                        &mut groups,
                        "swap",
                        vec![
                            edit(&rows, line, vec![rows[statement.line].clone()]),
                            edit(&rows, statement.line, vec![rows[line].clone()]),
                        ],
                    );
                }
                previous = Some(statement.line);
            }
        }
        for statement in &function.statements {
            let op = statement.word[1].as_str();
            if op == "read" && statement.word.len() > 3 {
                let mode = if statement.word[3] == "copy" {
                    "move"
                } else {
                    "copy"
                };
                replace(&mut groups, &rows, "copy-move", statement, 3, mode);
            }
            if (op == "borrow" || op == "view-range") && statement.word.len() > 3 {
                let mode = if statement.word[3] == "shared" {
                    "mut"
                } else {
                    "shared"
                };
                replace(&mut groups, &rows, "borrow-kind", statement, 3, mode);
            }
            if op == "call" {
                for callee in 0..functions.len() {
                    replace(
                        &mut groups,
                        &rows,
                        "callee",
                        statement,
                        3,
                        &format!("{callee}"),
                    );
                }
            }
            if op == "int" {
                replace(&mut groups, &rows, "constant", statement, 3, "-1");
                replace(&mut groups, &rows, "constant", statement, 3, "0");
                replace(&mut groups, &rows, "constant", statement, 3, "1");
                replace(&mut groups, &rows, "constant", statement, 3, "42");
                replace(&mut groups, &rows, "constant", statement, 3, "255");
                replace(
                    &mut groups,
                    &rows,
                    "constant",
                    statement,
                    3,
                    "9223372036854775807",
                );
            }
            if op == "drop" || op == "drop-if" {
                for target in &function.statements {
                    if target.line != statement.line {
                        add(
                            &mut groups,
                            "move-drop",
                            vec![
                                edit(&rows, statement.line, Vec::new()),
                                edit(
                                    &rows,
                                    target.line,
                                    vec![rows[statement.line].clone(), rows[target.line].clone()],
                                ),
                            ],
                        );
                    }
                }
            }
            let mut operands = match op {
                "read" | "borrow" => vec![2usize, 4],
                "assign" | "cast" | "tag-test" => vec![2usize, 3],
                "drop" | "drop-if" | "if" | "return" => vec![2usize],
                "call" | "intrinsic" | "variant" => vec![2usize],
                "binary" => vec![2usize, 4, 5],
                "unary" => vec![2usize, 4],
                _ => Vec::new(),
            };
            let rest = if op == "struct" || op == "array" {
                2usize
            } else if op == "call" || op == "intrinsic" || op == "variant" {
                4usize
            } else {
                statement.word.len()
            };
            for position in rest..statement.word.len() {
                operands.push(position);
            }
            for position in operands {
                if position < statement.word.len()
                    && !statement.word[position].is_empty()
                    && statement.word[position]
                        .as_bytes()
                        .iter()
                        .all(|byte| byte.is_ascii_digit())
                {
                    for local in 0..function.locals.len() {
                        replace(
                            &mut groups,
                            &rows,
                            "operand",
                            statement,
                            position,
                            &format!("{local}"),
                        );
                    }
                }
            }
        }
        for local in &function.locals {
            for ty in 0..declarations.len() {
                replace(&mut groups, &rows, "local-type", local, 2, &format!("{ty}"));
            }
        }
    }
    for declaration in &declarations {
        let kind = declaration.word[2].as_str();
        if kind == "ref" || kind == "slice" {
            let mode = if declaration.word[3] == "shared" {
                "mut"
            } else {
                "shared"
            };
            replace(&mut groups, &rows, "type-borrow-kind", declaration, 3, mode);
        }
        if kind == "struct" || kind == "enum" {
            let mode = if declaration.word[3] == "copy" {
                "move"
            } else {
                "copy"
            };
            replace(&mut groups, &rows, "type-copy-kind", declaration, 3, mode);
        }
        if kind == "array" {
            let length = integer(&declaration.word[4], "array extent")?;
            let mut values = vec![0usize, 1, 2];
            if length < (!0usize) {
                values.push(length + 1);
            }
            if length > 0 {
                values.push(length - 1);
            }
            for (index, value) in values.iter().enumerate() {
                if !values[..index].contains(value) {
                    replace(
                        &mut groups,
                        &rows,
                        "array-extent",
                        declaration,
                        4,
                        &format!("{value}"),
                    );
                }
            }
        }
    }
    if groups.is_empty() {
        return Err(format!("{path}: mutation seed has no editable operations"));
    }
    Ok(Seed {
        path: path.to_string(),
        text,
        groups,
    })
}
pub fn apply(text: &str, mutation: &Mutation) -> Result<String, String> {
    let rows = lines(text);
    for item in &mutation.edits {
        if rows.get(item.line) != Some(&item.before) {
            return Err(format!(
                "mutation preimage differs at line {}",
                item.line + 1
            ));
        }
    }
    let mut out = String::new();
    for (line, original) in rows.iter().enumerate() {
        let mut changed = false;
        for item in &mutation.edits {
            if item.line == line {
                for replacement in &item.after {
                    out.push_str(replacement);
                    out.push('\n');
                }
                changed = true;
                break;
            }
        }
        if !changed {
            out.push_str(original);
            out.push('\n');
        }
    }
    Ok(out)
}
pub fn choose<'a>(seeds: &'a [Seed], random: &mut Random) -> (usize, &'a Mutation) {
    let index = random.pick(seeds.len());
    let seed = &seeds[index];
    let group = &seed.groups[random.pick(seed.groups.len())];
    (index, &group[random.pick(group.len())])
}
