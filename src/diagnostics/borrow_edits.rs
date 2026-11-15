//! Source-only explanations never alter an IR verdict or authorize execution.
//! Keep existing expressions in place: edits add scopes or replace fixed syntax.
use crate::ast::{self as a, Expr, Pattern, Span, Stmt, Type};
use crate::diagnostic::{Diagnostic, Edit};

fn encloses(outer: Span, inner: Span) -> bool {
    outer.file == inner.file && outer.start <= inner.start && inner.end <= outer.end
}

fn primary(problem: &Diagnostic, span: Span) -> bool {
    problem.file == span.file && problem.line == span.line && problem.column == span.column
}

// Correlate the checker's reported loan origin; this is only reporting data.
fn origin(problem: &Diagnostic, span: Span) -> bool {
    problem.file == span.file
        && problem.message.ends_with(&format!("(created at {}:{})", span.line, span.column))
}

fn function_at(program: &a::Program, span: Span) -> Option<usize> {
    let mut found: Option<usize> = None;
    let mut size = !0usize;
    for (index, function) in program.functions.iter().enumerate() {
        if let Some(body) = function.body {
            let body_span = program.blocks[body].span;
            if encloses(body_span, span) && body_span.end - body_span.start < size {
                found = Some(index);
                size = body_span.end - body_span.start;
            }
        }
    }
    found
}

fn unused(program: &a::Program, candidate: &str) -> bool {
    for node in &program.exprs {
        match &node.kind {
            Expr::Path(path) if path.name == candidate => return false,
            Expr::String(text) if text.contains(candidate) => return false,
            Expr::Closure { params, .. } => {
                for parameter in params {
                    if parameter.name == candidate { return false; }
                }
            }
            _ => {}
        }
    }
    for node in &program.patterns {
        if let Pattern::Bind { name, .. } = &node.kind {
            if name == candidate { return false; }
        }
    }
    for declaration in &program.declarations {
        if declaration.name == candidate { return false; }
    }
    for node in &program.types {
        if let Type::Path(path) = &node.kind {
            if path.name == candidate { return false; }
        }
    }
    for function in &program.functions {
        if function.name == candidate { return false; }
        for parameter in &function.params {
            if parameter.name == candidate { return false; }
        }
    }
    true
}

fn fresh(program: &a::Program, stem: &str) -> String {
    let mut index = 0usize;
    loop {
        let name = format!("__lkjscript_{stem}_{index}");
        if unused(program, &name) { return name; }
        index += 1usize;
    }
}

// This is deliberately a syntax proof of a vec! value, not a type-name guess.
// Ambiguous/shadowed bindings and unresolved function results get no loop edit.
fn vec_value(program: &a::Program, expression: usize) -> bool {
    let mut at = expression;
    let mut remaining = program.exprs.len();
    while remaining != 0usize {
        remaining -= 1usize;
        match &program.exprs[at].kind {
            Expr::Macro { name, .. } => return name == "vec",
            Expr::Block(block) => {
                let Some(tail) = program.blocks[*block].tail else { return false; };
                at = tail;
            }
            Expr::Path(path) if path.args.is_empty() => {
                let Some(function) = function_at(program, program.exprs[at].span) else { return false; };
                let owner = &program.functions[function];
                let Some(body) = owner.body else { return false; };
                for parameter in &owner.params {
                    if parameter.name == path.name { return false; }
                }
                let mut bindings = 0usize;
                for pattern in &program.patterns {
                    if !encloses(program.blocks[body].span, pattern.span)
                        || pattern.span.start >= program.exprs[at].span.start { continue; }
                    if let Pattern::Bind { name, .. } = &pattern.kind {
                        if name == &path.name { bindings += 1usize; }
                    }
                }
                if bindings != 1usize { return false; }
                let mut initializer: Option<usize> = None;
                for block in &program.blocks {
                    if !encloses(program.blocks[body].span, block.span) { continue; }
                    for statement in &block.statements {
                        if let Stmt::Let { pattern, value, span, .. } = statement {
                            if !encloses(program.blocks[body].span, *span)
                                || span.start >= program.exprs[at].span.start { continue; }
                            if let Pattern::Bind { name, .. } = &program.patterns[*pattern].kind {
                                if name == &path.name {
                                    if initializer.is_some() || value.is_none() { return false; }
                                    initializer = *value;
                                }
                            }
                        }
                    }
                }
                let Some(value) = initializer else { return false; };
                at = value;
            }
            _ => return false,
        }
    }
    false
}

fn pure_place(program: &a::Program, expression: usize) -> bool {
    let mut at = expression;
    loop {
        match &program.exprs[at].kind {
            Expr::Path(path) => return path.args.is_empty(),
            Expr::Field { value, .. } => at = *value,
            _ => return false,
        }
    }
}

// Transparent parentheses are absent from some AST spans. Only replace a call
// or wrap a postfix chain when its byte extents prove the delimiters are intact.
// Less regular spacing still receives the explanation, without an unsafe edit.
fn tight_place(program: &a::Program, expression: usize) -> bool {
    let mut at = expression;
    loop {
        let node = &program.exprs[at];
        match &node.kind {
            Expr::Path(path) => return path.args.is_empty() && node.span.end - node.span.start == path.name.len(),
            Expr::Field { value, name } => {
                let base = program.exprs[*value].span;
                if node.span.start != base.start || node.span.end != base.end + 1usize + name.len() { return false; }
                at = *value;
            }
            _ => return false,
        }
    }
}

fn same_place(program: &a::Program, first: usize, second: usize) -> bool {
    let mut left = first;
    let mut right = second;
    loop {
        match (&program.exprs[left].kind, &program.exprs[right].kind) {
            (Expr::Path(a), Expr::Path(b)) => {
                return a.args.is_empty() && b.args.is_empty() && a.name == b.name;
            }
            (Expr::Field { value: a, name: x }, Expr::Field { value: b, name: y }) if x == y => {
                left = *a;
                right = *b;
            }
            _ => return false,
        }
    }
}

fn pure_range(program: &a::Program, expression: usize) -> bool {
    let Expr::Range { start, end, .. } = &program.exprs[expression].kind else { return false; };
    simple_bound(program, *start) && simple_bound(program, *end)
}

fn simple_bound(program: &a::Program, bound: Option<usize>) -> bool {
    match bound {
        None => true,
        Some(value) => matches!(program.exprs[value].kind, Expr::Int(_)) || pure_place(program, value),
    }
}

fn simple_view(program: &a::Program, expression: usize) -> bool {
    let Expr::Borrow { mutable: false, value } = &program.exprs[expression].kind else { return false; };
    if pure_place(program, *value) { return true; }
    let Expr::Index { value: base, index } = &program.exprs[*value].kind else { return false; };
    pure_place(program, *base) && pure_range(program, *index)
}

fn scoped_value(program: &a::Program, span: Span) -> Vec<Edit> {
    let saved = fresh(program, "owned");
    vec![
        Edit { start: span.start, end: span.start, replacement: format!("{{ let {saved} = ") },
        Edit { start: span.end, end: span.end, replacement: format!("; {saved} }}") },
    ]
}

fn scalar_type(program: &a::Program, ty: usize) -> bool {
    match &program.types[ty].kind {
        Type::Tuple(items) => items.is_empty(),
        Type::Path(path) if path.args.is_empty() => {
            matches!(path.name.as_str(),
                "bool" | "char" | "u8" | "u16" | "u32" | "u64" | "usize"
                | "i8" | "i16" | "i32" | "i64" | "isize" | "f64")
        }
        _ => false,
    }
}

fn mutable_method(program: &a::Program, call: Span, method: &str) -> bool {
    let Some(caller) = function_at(program, call) else { return false; };
    for implementation in &program.impls {
        if implementation.trait_ty.is_some() || !implementation.methods.contains(&caller) { continue; }
        for index in &implementation.methods {
            let function = &program.functions[*index];
            if function.name != method || function.params.len() != 2usize { continue; }
            if matches!(program.types[function.params[0].ty].kind, Type::Ref { mutable: true, .. })
                && scalar_type(program, function.params[1].ty) { return true; }
        }
    }
    false
}

fn advice(program: &a::Program, problem: &Diagnostic) -> Option<(String, Vec<Edit>)> {
    for node in &program.exprs {
        if node.span.file != problem.file { continue; }
        if problem.code == "E_TRAIT" && primary(problem, node.span) {
            if let Expr::For { pattern, iterator, body } = node.kind {
                let Expr::Borrow { mutable: true, value } = program.exprs[iterator].kind else { continue; };
                let Pattern::Bind { name, mutable, by_ref: false, ref_mut: false } = &program.patterns[pattern].kind else { continue; };
                if !vec_value(program, value) { continue; }
                let input = program.exprs[iterator].span;
                let block = program.blocks[body].span;
                if input.start <= node.span.start || input.end > block.start || block.end != node.span.end { continue; }
                let collection = fresh(program, "collection");
                let index = fresh(program, "index");
                let current = fresh(program, "current");
                let binding = if *mutable { format!("mut {name}") } else { name.clone() };
                return Some((
                    "mutable Vec iteration requires IntoIterator for &mut Vec<T>, which is not implemented; help: evaluate the collection once, then use an index loop borrowing one element at a time (docs/language.md, Source library)".to_string(),
                    vec![
                        Edit { start: node.span.start, end: input.start, replacement: format!("{{ let {collection} = ") },
                        Edit { start: input.end, end: block.start, replacement: format!("; let mut {index} = 0usize; while {index} < {collection}.len() {{ let {current} = {index}; {index} += 1usize; let {binding} = &mut {collection}[{current}]; ") },
                        Edit { start: block.end, end: block.end, replacement: " } }".to_string() },
                    ],
                ));
            }
        }
        if problem.code != "E_BORROW" { continue; }
        if let Expr::Method { receiver, name, types, args } = &node.kind {
            if !types.is_empty() || args.len() != 1usize || !mutable_method(program, node.span, name) { continue; }
            let argument = args[0];
            let Expr::Path(path) = &program.exprs[*receiver].kind else { continue; };
            let Expr::Field { value, .. } = program.exprs[argument].kind else { continue; };
            if path.name != "self" || !path.args.is_empty() || !same_place(program, *receiver, value)
                || !primary(problem, program.exprs[argument].span)
                || !origin(problem, program.exprs[*receiver].span) { continue; }
            let saved = fresh(program, "argument");
            let input = program.exprs[argument].span;
            let receiver_span = program.exprs[*receiver].span;
            let edits = if tight_place(program, argument)
                && input.start == receiver_span.end + name.len() + 2usize
                && node.span.end == input.end + 1usize
            {
                vec![
                    Edit { start: node.span.start, end: input.start, replacement: format!("({{ let {saved} = ") },
                    Edit { start: input.end, end: node.span.end, replacement: format!("; self.{name}({saved}) }})") },
                ]
            } else { Vec::new() };
            return Some((
                "mutable method receiver is borrowed before its arguments; this scalar field read conflicts with that exclusive borrow; help: read the argument into a local before borrowing self for the call (docs/language.md, Ownership, storage and views)".to_string(),
                edits,
            ));
        }
        if let Expr::Assign { op, place, value } = &node.kind {
            if op != "=" || !primary(problem, node.span) { continue; }
            let Expr::Method { receiver, name, types, args } = &program.exprs[*value].kind else { continue; };
            if name != "to_vec" || !types.is_empty() || !args.is_empty() { continue; }
            let Expr::Index { value: base, index } = program.exprs[*receiver].kind else { continue; };
            if !same_place(program, *place, base) || !pure_range(program, index)
                || !origin(problem, program.exprs[*receiver].span) { continue; }
            let base_span = program.exprs[base].span;
            let range_span = program.exprs[index].span;
            let slice_span = program.exprs[*receiver].span;
            let result_span = program.exprs[*value].span;
            let edits = if tight_place(program, base)
                && range_span.start == base_span.end + 1usize
                && slice_span.end == range_span.end + 1usize
                && result_span.end == slice_span.end + name.len() + 3usize
            { scoped_value(program, result_span) } else { Vec::new() };
            return Some((
                "self-slice assignment keeps the source view alive while replacing its owner; help: create the owned to_vec copy in a separate lexical block before the assignment (docs/language.md, Ownership, storage and views)".to_string(),
                edits,
            ));
        }
        if let Expr::Match { value, arms } = &node.kind {
            let Expr::Call { callee, args } = &program.exprs[*value].kind else { continue; };
            if !matches!(program.exprs[*callee].kind, Expr::Path(_)) { continue; };
            if args.len() != 1usize || !simple_view(program, args[0])
                || !origin(problem, program.exprs[args[0]].span) { continue; }
            let mut in_arm = false;
            for candidate in &program.exprs {
                if !primary(problem, candidate.span) { continue; }
                for arm in arms {
                    if encloses(arm.span, candidate.span) { in_arm = true; }
                }
            }
            if !in_arm { continue; }
            let call_span = program.exprs[*value].span;
            let argument_span = program.exprs[args[0]].span;
            let edits = if tight_place(program, *callee)
                && argument_span.start == program.exprs[*callee].span.end + 1usize
                && call_span.end == argument_span.end + 1usize
            { scoped_value(program, call_span) } else { Vec::new() };
            return Some((
                "match scrutinee retains its temporary source view through the arms; help: evaluate the owned scrutinee result in a separate lexical block before matching it (docs/language.md, Ownership, storage and views)".to_string(),
                edits,
            ));
        }
    }
    None
}

pub(super) fn complete(program: &a::Program, problems: &mut [Diagnostic]) {
    let mut index = 0usize;
    while index < problems.len() {
        let problem = &mut problems[index];
        if problem.edits.is_empty() {
            if let Some((message, edits)) = advice(program, problem) {
                problem.message = format!("{}; {message}", problem.message);
                problem.edits = edits;
            }
        }
        index += 1usize;
    }
}
