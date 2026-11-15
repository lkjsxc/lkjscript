//! One closed, private label ownership rewrite; no IR rule is weakened.
use crate::ast::{self as a, Expr, Item, Pattern, Type, Visibility};
use crate::diagnostic::{Diagnostic, Edit};

fn encloses(outer: a::Span, inner: a::Span) -> bool {
    outer.file == inner.file && outer.start <= inner.start && inner.end <= outer.end
}
fn primary(problem: &Diagnostic, span: a::Span) -> bool {
    problem.file == span.file && problem.line == span.line && problem.column == span.column
}
fn static_text(program: &a::Program, ty: usize) -> bool {
    let Type::Ref { mutable: false, lifetime, inner } = &program.types[ty].kind else { return false; };
    if lifetime != "'static" { return false; }
    matches!(&program.types[*inner].kind, Type::Path(path)
        if path.declaration.is_none() && path.args.is_empty() && path.name == "str")
}
fn nominal(program: &a::Program, path: &a::Path, item: Item) -> bool {
    match path.declaration {
        Some(id) => program.declarations[id].item == item && path.args.is_empty(),
        None => false,
    }
}
fn nominal_type(program: &a::Program, ty: usize, item: Item) -> bool {
    matches!(&program.types[ty].kind, Type::Path(path) if nominal(program, path, item))
}
fn parameter(function: &a::Function, name: &str) -> Option<usize> {
    let mut found = None;
    for (index, parameter) in function.params.iter().enumerate() {
        if parameter.name == name {
            if found.is_some() { return None; }
            found = Some(index);
        }
    }
    found
}
fn private(function: &a::Function) -> bool {
    function.public == Visibility::Private && function.generics.is_empty() && function.attrs.is_empty()
}
fn string_target(program: &a::Program, ty: usize, string: usize) -> bool {
    let mut at = ty;
    let mut remaining = program.types.len();
    while remaining != 0usize {
        remaining -= 1usize;
        match &program.types[at].kind {
            Type::Ref { inner, .. } => at = *inner,
            Type::Path(path) => {
                let Some(declaration) = path.declaration else { return false; };
                if declaration == string { return true; }
                let Item::Alias(alias) = program.declarations[declaration].item else { return false; };
                let alias = &program.aliases[alias];
                if !alias.generics.is_empty() { return true; }
                at = alias.ty;
            }
            _ => return false,
        }
    }
    true
}
fn canonical_string(program: &a::Program, names: &crate::resolve::Resolver<'_>, span: a::Span, scope: usize) -> Option<usize> {
    let canonical = names.library_type("string", "String", span).ok()?;
    let declaration = canonical.declaration?;
    if names.type_name(scope, "std::String", span).ok()? != canonical.name { return None; }
    let root = program.files[span.file].crate_root;
    for function in &program.functions {
        if program.files[function.span.file].crate_root == root
            && (function.name == "to_string" || function.name == "from_str") { return None; }
    }
    for item in &program.traits {
        if program.files[item.span.file].crate_root == root && item.name == "ToString" { return None; }
    }
    for implementation in &program.impls {
        if program.files[implementation.span.file].crate_root == root
            && string_target(program, implementation.target, declaration) { return None; }
    }
    Some(declaration)
}
fn string_type(program: &a::Program, ty: usize, declaration: usize) -> bool {
    matches!(&program.types[ty].kind, Type::Path(path)
        if path.declaration == Some(declaration) && path.args.is_empty())
}
fn unshadowed_constructor(program: &a::Program, function: &a::Function, root: usize) -> bool {
    for node in &program.patterns {
        if program.files[node.span.file].crate_root != root { continue; }
        if matches!(&node.kind, Pattern::Bind { name, .. } if name == &function.name) { return false; }
    }
    for candidate in &program.functions {
        if program.files[candidate.span.file].crate_root != root { continue; }
        for parameter in &candidate.params {
            if parameter.name == function.name { return false; }
        }
    }
    for node in &program.exprs {
        if program.files[node.span.file].crate_root != root { continue; }
        if let Expr::Closure { params, .. } = &node.kind {
            for parameter in params { if parameter.name == function.name { return false; } }
        }
    }
    true
}

fn edits(program: &a::Program, problem: &Diagnostic) -> Option<Vec<Edit>> {
    let names = crate::resolve::Resolver::new(program).ok()?;
    for (item_index, item) in program.structs.iter().enumerate() {
        if item.public != Visibility::Private || !item.generics.is_empty()
            || !item.attrs.is_empty() || item.tuple || item.unit || item.fields.len() != 2usize { continue; }
        let mut target = None;
        for (field_index, field) in item.fields.iter().enumerate() {
            if primary(problem, program.types[field.ty].span) && static_text(program, field.ty) {
                target = Some(field_index);
            }
        }
        let Some(target) = target else { continue; };
        for field in &item.fields { if field.public != Visibility::Private { return None; } }
        let field = &item.fields[target];
        let type_node = &program.types[field.ty];
        let string = canonical_string(program, &names, type_node.span, type_node.scope)?;
        for (index, other) in item.fields.iter().enumerate() {
            if index != target && !string_type(program, other.ty, string) { return None; }
        }
        let identity = Item::Struct(item_index);
        let root = program.files[item.span.file].crate_root;
        // A field read may require its old borrowed type. No such migration is
        // inferred here, nor are destructuring patterns or another nominal use.
        for node in &program.exprs {
            if program.files[node.span.file].crate_root == root
                && matches!(&node.kind, Expr::Field { name, .. } if name == &field.name) { return None; }
        }
        for node in &program.patterns {
            let path = match &node.kind {
                Pattern::Path(path) | Pattern::Variant { path, .. } | Pattern::Struct { path, .. } => Some(path),
                _ => None,
            };
            if let Some(path) = path {
                if nominal(program, path, identity) { return None; }
            }
        }
        let mut constructor = None;
        for (index, node) in program.exprs.iter().enumerate() {
            if let Expr::Struct { path, .. } = &node.kind {
                if nominal(program, path, identity) {
                    if constructor.is_some() { return None; }
                    constructor = Some(index);
                }
            }
        }
        let constructor = constructor?;
        let expression = &program.exprs[constructor];
        let Expr::Struct { fields, base: None, .. } = &expression.kind else { return None; };
        if expression.span.file != item.span.file || fields.len() != 2usize
            || fields[0].0 != field.name || fields[1].0 != item.fields[1usize - target].name { return None; }
        let mut owner = None;
        for (index, function) in program.functions.iter().enumerate() {
            let Some(body) = function.body else { continue; };
            let block = &program.blocks[body];
            if block.tail == Some(constructor) && block.statements.is_empty() {
                if owner.is_some() { return None; }
                owner = Some(index);
            }
        }
        let owner = owner?;
        let function = &program.functions[owner];
        if !private(function) || program.function_owner(a::FunctionId { index: owner }) != a::FunctionOwner::Free
            || function.span.file != item.span.file || item.span.end > function.span.start
            || !nominal_type(program, function.result, identity) { return None; }
        // This also excludes impls, aliases, typed external API uses, containers
        // and additional return/parameter contracts mentioning the private type.
        for (index, node) in program.types.iter().enumerate() {
            if let Type::Path(path) = &node.kind {
                if nominal(program, path, identity) && index != function.result { return None; }
            }
        }
        if !unshadowed_constructor(program, function, root) { return None; }
        let mut used = vec![false; function.params.len()];
        let mut input = None;
        for (index, (_, value)) in fields.iter().enumerate() {
            let node = &program.exprs[*value];
            if index == 0usize && matches!(node.kind, Expr::String(_)) { continue; }
            let Expr::Path(path) = &node.kind else { return None; };
            if !path.args.is_empty() || path.name.contains("::") || node.span.end - node.span.start != path.name.len() {
                return None;
            }
            let parameter = parameter(function, &path.name)?;
            let declaration = &function.params[parameter];
            if declaration.mutable || used[parameter] { return None; }
            used[parameter] = true;
            if index == 0usize {
                if !static_text(program, declaration.ty) { return None; }
                input = Some(parameter);
            } else if !string_type(program, declaration.ty, string) { return None; }
        }
        for present in &used { if !*present { return None; } }
        // Expression paths still have lexical-local precedence. After ruling out
        // shadowing, resolve every constructor reference and require a direct call.
        let mut calls = 0usize;
        for (index, node) in program.exprs.iter().enumerate() {
            let Expr::Path(path) = &node.kind else { continue; };
            let Ok(resolved) = names.value_path(node.scope, path, node.span) else { continue; };
            if !nominal(program, &resolved, Item::Function(owner)) { continue; }
            let mut called = false;
            for call in &program.exprs {
                let Expr::Call { callee, args } = &call.kind else { continue; };
                if *callee != index { continue; }
                if args.len() != function.params.len() { return None; }
                if let Some(input) = input {
                    if !matches!(program.exprs[args[input]].kind, Expr::String(_)) { return None; }
                }
                called = true;
                calls += 1usize;
            }
            if !called { return None; }
        }
        if calls == 0usize { return None; }
        let mut result = vec![Edit {
            start: type_node.span.start, end: type_node.span.end, replacement: "std::String".to_string(),
        }];
        if let Some(input) = input {
            let parameter = &function.params[input];
            let parameter_type = program.types[parameter.ty].span;
            if parameter_type.file != item.span.file || type_node.span.end > parameter_type.start
                || parameter_type.end > expression.span.start { return None; }
            result.push(Edit { start: parameter_type.start, end: parameter_type.end, replacement: "&str".to_string() });
            // The AST intentionally forgets shorthand punctuation. A scoped
            // shadow works for both kind and kind: kind without guessing bytes.
            // The label is the first field and every remaining field is a plain
            // parameter move, so evaluation and cleanup order are unchanged.
            result.push(Edit { start: expression.span.start, end: expression.span.start,
                replacement: format!("{{ let {} = ({}).to_string(); ", parameter.name, parameter.name) });
            result.push(Edit { start: expression.span.end, end: expression.span.end, replacement: " }".to_string() });
        } else {
            let value = program.exprs[fields[0].1].span;
            if value.file != item.span.file || !encloses(expression.span, value) { return None; }
            result.push(Edit { start: value.start, end: value.start, replacement: "(".to_string() });
            result.push(Edit { start: value.end, end: value.end, replacement: ").to_string()".to_string() });
        }
        return Some(result);
    }
    None
}

pub(super) fn complete(source: &a::Program, problems: &mut [Diagnostic]) {
    let mut relevant = false;
    for problem in problems.iter() {
        if problem.edits.is_empty() && problem.code == "E_UNSUPPORTED"
            && problem.message.contains("static lifetime constraints") { relevant = true; }
    }
    if !relevant { return; }
    let Ok(program) = crate::resolve::resolve(source) else { return; };
    let mut index = 0usize;
    while index < problems.len() {
        let problem = &mut problems[index];
        if problem.edits.is_empty() && problem.code == "E_UNSUPPORTED"
            && problem.message.contains("static lifetime constraints") {
            if let Some(edits) = edits(&program, problem) {
                problem.message.push_str("; this private static label can be owned as std::String; copy the label with to_string() before moving the remaining fields, changing a static constructor input to &str when present (docs/language.md, Ownership, storage and views)");
                problem.edits = edits;
            }
        }
        index += 1usize;
    }
}
