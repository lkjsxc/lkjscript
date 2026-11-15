//! Conservative source edits for diagnostics reported after type/ownership analysis.
use crate::ast::{self as a, Expr, Pattern, Stmt, Type};
use crate::diagnostic::{Diagnostic, Edit};
use super::borrow_edits::{encloses, function_at, origin, primary, pure_range, qualified, same_place, tight_place};

fn named_path(program: &a::Program, expression: usize, name: &str) -> bool {
    matches!(&program.exprs[expression].kind, Expr::Path(path) if path.name == name && path.args.is_empty())
}

fn unique_binding(program: &a::Program, function: usize, name: &str, expected: usize) -> bool {
    let Some(body) = program.functions[function].body else { return false; };
    let span = program.blocks[body].span;
    let mut count = 0usize;
    for pattern in &program.patterns {
        if encloses(span, pattern.span) {
            if let Pattern::Bind { name: binding, .. } = &pattern.kind {
                if binding == name { count += 1usize; }
            }
        }
    }
    count == expected
}

fn byte_slice(program: &a::Program, ty: usize, names: &crate::resolve::Resolver<'_>) -> bool {
    let Type::Ref { mutable: false, inner, .. } = program.types[ty].kind else { return false; };
    let Type::Slice(element) = program.types[inner].kind else { return false; };
    let node = &program.types[element];
    let Type::Path(path) = &node.kind else { return false; };
    if !path.args.is_empty() || names.type_name(node.scope, &path.name, node.span).ok() != Some("u8".to_string()) { return false; }
    for item in &program.structs { if qualified(&item.module, &item.name) == "u8" { return false; } }
    for item in &program.enums { if qualified(&item.module, &item.name) == "u8" { return false; } }
    for item in &program.aliases { if qualified(&item.module, &item.name) == "u8" { return false; } }
    true
}

fn slice_argument(program: &a::Program, expression: usize, names: &crate::resolve::Resolver<'_>) -> bool {
    for node in &program.exprs {
        let Expr::Call { callee, args } = &node.kind else { continue; };
        let Expr::Path(path) = &program.exprs[*callee].kind else { continue; };
        let callee_node = &program.exprs[*callee];
        if !path.name.contains("::") {
            let Some(owner) = function_at(program, node.span) else { continue; };
            if !unique_binding(program, owner, &path.name, 0usize) { continue; }
            let mut shadowed = false;
            for parameter in &program.functions[owner].params {
                if parameter.name == path.name { shadowed = true; }
            }
            if shadowed { continue; }
        }
        let Ok(callee) = names.value_path(callee_node.scope, path, callee_node.span) else { continue; };
        for (index, argument) in args.iter().enumerate() {
            if *argument != expression { continue; }
            for (function_id, function) in program.functions.iter().enumerate() {
                let mut method = false;
                for implementation in &program.impls { if implementation.methods.contains(&function_id) { method = true; } }
                for declaration in &program.traits { if declaration.methods.contains(&function_id) { method = true; } }
                if method { continue; }
                if qualified(&function.module, &function.name) != callee.name
                    || !function.generics.is_empty() || index >= function.params.len() { continue; }
                if byte_slice(program, function.params[index].ty, names) { return true; }
            }
        }
    }
    false
}

fn branch_slices(program: &a::Program, problem: &Diagnostic) -> Option<(String, Vec<Edit>)> {
    let names = crate::resolve::Resolver::new(program).ok()?;
    for (id, node) in program.exprs.iter().enumerate() {
        let Expr::If { then_block, otherwise: Some(other), .. } = node.kind else { continue; };
        let Some(left) = program.blocks[then_block].tail else { continue; };
        let Expr::Block(other) = program.exprs[other].kind else { continue; };
        let Some(right) = program.blocks[other].tail else { continue; };
        let (Expr::Bytes(a), Expr::Bytes(b)) = (&program.exprs[left].kind, &program.exprs[right].kind) else { continue; };
        if a.len() == b.len() || !primary(problem, node.span) || !slice_argument(program, id, &names) { continue; }
        let mut edits = Vec::new();
        for expression in vec![left, right] {
            let span = program.exprs[expression].span;
            edits.push(Edit { start: span.start, end: span.start, replacement: "(&".to_string() });
            edits.push(Edit { start: span.end, end: span.end, replacement: "[..])".to_string() });
        }
        return Some(("conditional byte strings have different array lengths; a slice parameter needs both branches converted to &[u8]; help: write &b\"...\"[..] in each branch (docs/language.md, Inference, generics and traits)".to_string(), edits));
    }
    None
}

fn string_set(program: &a::Program, problem: &Diagnostic) -> Option<(String, Vec<Edit>)> {
    let names = crate::resolve::Resolver::new(program).ok()?;
    for function in &program.functions {
        if function.name == "to_string" && !function.module.starts_with("$std") { return None; }
    }
    for block in &program.blocks {
        for statement in &block.statements {
            let Stmt::Let { pattern, ty: None, value: Some(value), .. } = statement else { continue; };
            let Pattern::Bind { name, .. } = &program.patterns[*pattern].kind else { continue; };
            let Expr::Call { callee, args } = &program.exprs[*value].kind else { continue; };
            let Expr::Path(path) = &program.exprs[*callee].kind else { continue; };
            let node = &program.exprs[*callee];
            if !args.is_empty() || !path.args.is_empty() || !primary(problem, node.span) { continue; }
            let canonical = names.library_type("ordered_set", "BTreeSet", node.span).ok()?;
            let resolved = names.value_path(node.scope, path, node.span).ok()?;
            if resolved.name != format!("{}::new", canonical.name) { continue; }
            if names.type_name(node.scope, "std::collections::BTreeSet", node.span).ok()? != canonical.name { continue; }
            let string = names.library_type("string", "String", node.span).ok()?;
            if names.type_name(node.scope, "std::String", node.span).ok()? != string.name { continue; }
            let function = function_at(program, node.span)?;
            if !unique_binding(program, function, name, 1usize) { continue; }
            let body = program.functions[function].body?;
            let mut inserts = 0usize;
            let mut known = true;
            for call in &program.exprs {
                let Expr::Method { receiver, name: method, types, args } = &call.kind else { continue; };
                if !encloses(program.blocks[body].span, call.span) || call.span.start <= node.span.end || method != "insert"
                    || !named_path(program, *receiver, name) { continue; }
                if !types.is_empty() || args.len() != 1usize { known = false; continue; }
                let Expr::Method { receiver, name: method, types, args } = &program.exprs[args[0]].kind else { known = false; continue; };
                if method != "to_string" || !types.is_empty() || !args.is_empty()
                    || !matches!(program.exprs[*receiver].kind, Expr::String(_)) { known = false; continue; }
                inserts += 1usize;
            }
            if !known || inserts == 0usize { continue; }
            return Some(("BTreeSet::new needs its element type before later insert calls are checked; these literal.to_string() inserts require std::String; help: write BTreeSet::<std::String>::new() (docs/language.md, Inference, generics and traits)".to_string(),
                vec![Edit { start: node.span.start, end: node.span.end,
                    replacement: "std::collections::BTreeSet::<std::String>::new".to_string() }]));
        }
    }
    None
}

fn repeated_reborrow(program: &a::Program, problem: &Diagnostic) -> Option<(String, Vec<Edit>)> {
    for node in &program.exprs {
        if !primary(problem, node.span) { continue; }
        let body = match node.kind {
            Expr::For { body, .. } | Expr::While { body, .. } | Expr::Loop { body } => body,
            _ => continue,
        };
        let function = function_at(program, node.span)?;
        for parameter in &program.functions[function].params {
            if !matches!(program.types[parameter.ty].kind, Type::Ref { mutable: true, .. })
                || !problem.message.contains(&format!("leaves {} uninitialized or moved", parameter.name))
                || !unique_binding(program, function, &parameter.name, 0usize) { continue; }
            let mut edits: std::collections::BTreeMap<usize, Edit> = std::collections::BTreeMap::new();
            let mut references = 0usize;
            for use_node in &program.exprs {
                if encloses(program.blocks[body].span, use_node.span) {
                    if let Expr::Path(path) = &use_node.kind {
                        if path.name == parameter.name { references += 1usize; }
                    }
                }
            }
            for call in &program.exprs {
                if !encloses(program.blocks[body].span, call.span) { continue; }
                let args = match &call.kind {
                    Expr::Call { args, .. } | Expr::Method { args, .. } => args,
                    _ => continue,
                };
                for argument in args {
                    if !named_path(program, *argument, &parameter.name) { continue; }
                    let span = program.exprs[*argument].span;
                    edits.insert(span.start, Edit { start: span.start, end: span.start, replacement: "&mut *".to_string() });
                }
            }
            if edits.is_empty() || edits.len() != references { continue; }
            let mut ordered = Vec::new();
            for (_, edit) in edits { ordered.push(edit); }
            return Some((format!("generic mutable argument {} is moved, so the next loop iteration cannot reuse it; help: pass &mut *{} to create an explicit reborrow for each call (docs/language.md, Inference, generics and traits)", parameter.name, parameter.name), ordered));
        }
    }
    None
}

fn place_text(program: &a::Program, expression: usize) -> Option<String> {
    let mut at = expression;
    let mut fields: Vec<String> = Vec::new();
    loop {
        match &program.exprs[at].kind {
            Expr::Field { value, name } => { fields.push(name.clone()); at = *value; }
            Expr::Path(path) if path.args.is_empty() => {
                let mut out = path.name.clone();
                while let Some(field) = fields.pop() { out.push('.'); out.push_str(&field); }
                return Some(out);
            }
            _ => return None,
        }
    }
}

fn return_owner(program: &a::Program, problem: &Diagnostic) -> Option<(String, Vec<Edit>)> {
    for block in &program.blocks {
        let Some(tail) = block.tail else { continue; };
        if !primary(problem, program.exprs[tail].span) || block.statements.len() < 2usize { continue; }
        let Expr::Path(owner) = &program.exprs[tail].kind else { continue; };
        let Stmt::Let { pattern, value: Some(view), span: binding, .. } = &block.statements[block.statements.len() - 2usize] else { continue; };
        let Pattern::Bind { name, .. } = &program.patterns[*pattern].kind else { continue; };
        let Expr::Borrow { mutable: false, value: slice } = program.exprs[*view].kind else { continue; };
        let Expr::Index { value: base, index } = program.exprs[slice].kind else { continue; };
        let Expr::Field { value: root, .. } = program.exprs[base].kind else { continue; };
        if !named_path(program, root, &owner.name) || !pure_range(program, index)
            || !origin(problem, program.exprs[*view].span) { continue; }
        let Stmt::Expr { value: assignment, semicolon: true } = block.statements[block.statements.len() - 1usize] else { continue; };
        let Expr::Assign { op, place, value } = &program.exprs[assignment].kind else { continue; };
        let Expr::Field { value: destination, .. } = program.exprs[*place].kind else { continue; };
        if op != "=" || !same_place(program, root, destination) || !tight_place(program, *place) { continue; }
        let Expr::Method { receiver, name: method, types, args } = &program.exprs[*value].kind else { continue; };
        if method != "len" || !types.is_empty() || !args.is_empty()
            || !named_path(program, *receiver, name) { continue; }
        let input = program.exprs[*value].span;
        let output = program.exprs[assignment].span;
        if input.end != program.exprs[*receiver].span.end + 6usize || output.end != input.end { continue; }
        let destination = place_text(program, *place)?;
        return Some(("returning this owner conflicts with the subslice temporary retained by its local binding; help: compute the scalar length in a nested block, then assign it and return the owner (docs/language.md, Ownership, storage and views)".to_string(), vec![
            Edit { start: binding.start, end: binding.start, replacement: format!("{destination} = {{ ") },
            Edit { start: output.start, end: input.start, replacement: String::new() },
            Edit { start: output.end, end: output.end, replacement: " }".to_string() },
        ]));
    }
    None
}

fn advice(program: &a::Program, problem: &Diagnostic) -> Option<(String, Vec<Edit>)> {
    if problem.code == "E_TYPE" {
        if let Some(value) = branch_slices(program, problem) { return Some(value); }
        return string_set(program, problem);
    }
    if problem.code == "E_MOVE" { return repeated_reborrow(program, problem); }
    if problem.code == "E_BORROW" {
        if let Some(value) = return_owner(program, problem) { return Some(value); }
        if problem.message.contains("$for:") && problem.message.contains(":iterator conflicts") {
            return Some(("returned element view is tied to the loop iterator temporary; help: index the owning input directly when returning a view, preserving the selected iterator's order (docs/language.md, Source library)".to_string(), Vec::new()));
        }
    }
    if problem.code == "E_UNSUPPORTED" && problem.message.contains("static lifetime constraints") {
        for item in &program.structs {
            for field in &item.fields {
                let node = &program.types[field.ty];
                if !primary(problem, node.span) { continue; }
                if let Type::Ref { lifetime, inner, .. } = &node.kind {
                    if lifetime == "'static" && matches!(&program.types[*inner].kind, Type::Path(path) if path.name == "str") {
                        return Some((format!("struct field {}.{} declares an explicit static view; source static constraints on fields are unsupported; help: own this label as String and convert incoming text with to_string() (docs/language.md, Ownership, storage and views)", item.name, field.name), Vec::new()));
                    }
                }
            }
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
