// A program directory supplies invocation values through existing source constants.
// Its entry signature remains the authority; this reader never executes source.
use crate::{ast, entry_grants, ir, syntax};

#[derive(Clone)]
pub struct Plan {
    pub manifest: String,
    pub directory: String,
    pub source: String,
    pub entry: String,
    pub grants: Vec<String>,
    pub cases: Vec<String>,
}

pub fn join(directory: &str, path: &str) -> String {
    if directory.ends_with('/') { format!("{directory}{path}") }
    else { format!("{directory}/{path}") }
}

fn problem(path: &str, message: &str) -> String {
    format!("{path}: E_PROGRAM {message}")
}

fn string_type(program: &ast::Program, ty: usize) -> bool {
    match &program.types[ty].kind {
        ast::Type::Ref { mutable: false, inner, .. } => {
            match &program.types[*inner].kind {
                ast::Type::Path(path) => path.name == "str" && path.args.is_empty(),
                _ => false,
            }
        }
        _ => false,
    }
}

fn strings_type(program: &ast::Program, ty: usize) -> bool {
    match &program.types[ty].kind {
        ast::Type::Ref { mutable: false, inner, .. } => {
            match &program.types[*inner].kind {
                ast::Type::Slice(inner) => string_type(program, *inner),
                _ => false,
            }
        }
        _ => false,
    }
}

fn string(program: &ast::Program, value: usize) -> Option<String> {
    match &program.exprs[value].kind {
        ast::Expr::String(text) => Some(text.clone()),
        _ => None,
    }
}

fn strings(program: &ast::Program, value: usize) -> Option<Vec<String>> {
    let inner = match &program.exprs[value].kind {
        ast::Expr::Borrow { mutable: false, value } => *value,
        _ => return None,
    };
    match &program.exprs[inner].kind {
        ast::Expr::Array(items) => {
            let mut result = Vec::new();
            for item in items { result.push(string(program, *item)?); }
            Some(result)
        }
        _ => None,
    }
}

fn beneath(path: &str) -> bool {
    if path.is_empty() || path.starts_with('/') || path.as_bytes().contains(&0u8) { return false; }
    let mut begin = 0usize;
    for end in 0..=path.len() {
        if end == path.len() || path.as_bytes()[end] == b'/' {
            if &path[begin..end] == ".." { return false; }
            begin = end + 1;
        }
    }
    true
}

pub fn parse(directory: &str, text: &str) -> Result<Plan, String> {
    let manifest = join(directory, "program.rs");
    let program = syntax::parse(text).map_err(|error: ast::Diagnostic| {
        problem(&format!("{manifest}:{}:{}", error.line, error.column), &error.message)
    })?;
    let mut source: Option<String> = None;
    let mut entry: Option<String> = None;
    let mut grants: Option<Vec<String>> = None;
    let mut cases: Option<Vec<String>> = None;
    for item in &program.items {
        let constant = match item {
            ast::Item::Constant(index) => &program.constants[*index],
            _ => return Err(problem(&manifest, "only SOURCE, ENTRY, GRANTS and CASES constants are accepted")),
        };
        let location = format!("{manifest}:{}:{}", constant.span.line, constant.span.column);
        match constant.name.as_str() {
            "SOURCE" | "ENTRY" => {
                if !string_type(&program, constant.ty) {
                    return Err(problem(&location, &format!("{} must have type &str", constant.name)));
                }
                let value = match string(&program, constant.value) {
                    Some(value) => value,
                    None => return Err(problem(&location, &format!("{} must be a string literal", constant.name))),
                };
                if constant.name == "SOURCE" {
                    if source.is_some() { return Err(problem(&location, "duplicate SOURCE")); }
                    source = Some(value);
                } else {
                    if entry.is_some() { return Err(problem(&location, "duplicate ENTRY")); }
                    entry = Some(value);
                }
            }
            "GRANTS" | "CASES" => {
                if !strings_type(&program, constant.ty) {
                    return Err(problem(&location, &format!("{} must have type &[&str]", constant.name)));
                }
                let value = match strings(&program, constant.value) {
                    Some(value) => value,
                    None => return Err(problem(&location, &format!("{} must be a borrowed array of string literals", constant.name))),
                };
                if constant.name == "GRANTS" {
                    if grants.is_some() { return Err(problem(&location, "duplicate GRANTS")); }
                    grants = Some(value);
                } else {
                    if cases.is_some() { return Err(problem(&location, "duplicate CASES")); }
                    cases = Some(value);
                }
            }
            _ => return Err(problem(&location, &format!("unknown constant {}", constant.name))),
        }
    }
    let source = source.ok_or_else(|| problem(&manifest, "missing SOURCE: &str"))?;
    let entry = entry.ok_or_else(|| problem(&manifest, "missing ENTRY: &str"))?;
    let grants = grants.ok_or_else(|| problem(&manifest, "missing GRANTS: &[&str]"))?;
    let cases = cases.ok_or_else(|| problem(&manifest, "missing CASES: &[&str]"))?;
    if !beneath(&source) {
        return Err(problem(&manifest, "SOURCE must be a nonempty relative path beneath the program directory"));
    }
    if entry.is_empty() || entry.as_bytes().iter().any(|byte: &u8| *byte <= 32 || *byte == 127) {
        return Err(problem(&manifest, "ENTRY must name one nonempty function"));
    }
    let mut seen: Vec<String> = Vec::new();
    for path in &cases {
        if !beneath(path) {
            return Err(problem(&manifest, "each CASES path must stay beneath the program directory"));
        }
        if seen.contains(path) { return Err(problem(&manifest, &format!("duplicate CASES path {path}"))); }
        seen.push(path.clone());
    }
    Ok(Plan { manifest, directory: directory.to_string(), source, entry, grants, cases })
}

pub fn load(world: &mut World, path: &str) -> Result<Option<Plan>, String> {
    let mut directory = match world.directory(path) {
        Ok(directory) => directory,
        Err(2) | Err(20) => return Ok(None),
        Err(errno) => return Err(problem(path, &format!("open program directory: errno {errno}"))),
    };
    let manifest = join(path, "program.rs");
    let mut file = match directory.open("program.rs") {
        Ok(file) => file,
        Err(2) => return Ok(None),
        Err(errno) => return Err(problem(&manifest, &format!("open manifest: errno {errno}"))),
    };
    let bytes = file.read_to_end().map_err(|errno: i64| problem(&manifest, &format!("read manifest: errno {errno}")))?;
    let text = String::from_utf8(bytes).map_err(|_| problem(&manifest, "manifest is not UTF-8"))?;
    parse(path, &text).map(|plan: Plan| Some(plan))
}

pub fn prepare(plan: &Plan, program: &ir::Program, entry: usize) -> Result<Vec<ir::EntryGrant>, String> {
    let function = &program.functions[entry];
    let mut values = plan.grants.clone();
    for grant in 0..values.len() {
        let value = values[grant].clone();
        if let Some(at) = value.find('=') {
            let mut directory = false;
            for local in function.locals.iter().take(function.params) {
                if local.name == value[..at] && matches!(program.types[local.ty].kind, ir::TypeKind::Directory) {
                    directory = true;
                }
            }
            if directory && !value[at + 1..].is_empty() && !value[at + 1..].starts_with('/')
                && !value[at + 1..].starts_with("env:") {
                values[grant] = format!("{}={}", &value[..at], join(&plan.directory, &value[at + 1..]));
            }
        }
    }
    entry_grants::prepare(program, entry, &values).map_err(|error: String| {
        let mut signature = String::new();
        for local in function.locals.iter().take(function.params) {
            if !signature.is_empty() { signature.push_str(", "); }
            signature.push_str(&local.name);
        }
        problem(&plan.manifest, &format!("GRANTS versus {}::{}({signature}): {error}", join(&plan.directory, &plan.source), function.name))
    })
}
