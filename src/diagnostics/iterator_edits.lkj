//! An index-loop rewrite is offered only for the canonical shared Vec protocol.
use crate::ast::{self as a, Expr, Item, Pattern, Type};
use crate::diagnostic::{Diagnostic, Edit};
use super::borrow_edits::{encloses, fresh, function_at, primary, tight_place};

fn named_self(program: &a::Program, expression: usize) -> bool {
    matches!(&program.exprs[expression].kind, Expr::Path(path) if path.name == "self" && path.args.is_empty())
}

fn concrete(program: &a::Program, root: usize) -> bool {
    let mut pending = vec![root];
    let mut seen: std::collections::BTreeSet<usize> = std::collections::BTreeSet::new();
    while let Some(id) = pending.pop() {
        if !seen.insert(id) { continue; }
        match &program.types[id].kind {
            Type::Tuple(items) => pending.extend_from_slice(items),
            Type::Array { inner, len } => {
                if len.as_str().parse::<usize>().is_err() { return false; }
                pending.push(*inner);
            }
            Type::Path(path) => {
                pending.extend_from_slice(&path.args);
                if let Some(declaration) = path.declaration {
                    if matches!(program.declarations[declaration].item, Item::Alias(_)) { return false; }
                } else if !matches!(path.name.as_str(), "bool" | "char" | "u8" | "u16" | "u32" | "u64"
                    | "usize" | "i8" | "i16" | "i32" | "i64" | "isize" | "f64" | "Vec" | "Box"
                    | "World" | "Stdout" | "Stdin" | "Stderr" | "Directory" | "File" | "Process"
                    | "Clock" | "Child" | "Listener" | "Connector" | "Readiness") { return false; }
            }
            _ => return false,
        }
    }
    true
}

fn library_iterator(program: &a::Program) -> Option<usize> {
    for (id, declaration) in program.declarations.iter().enumerate() {
        if let Item::Struct(index) = declaration.item {
            let item = &program.structs[index];
            if item.module == "$std::iter" && item.name == "Iter" { return Some(id); }
        }
    }
    None
}

fn ordinary_protocol(program: &a::Program) -> bool {
    let Some(iterator) = library_iterator(program) else { return false; };
    for function in &program.functions {
        if !function.module.starts_with("$std")
            && matches!(function.name.as_str(), "len" | "index" | "index_mut" | "deref" | "deref_mut"
                | "into_iter" | "next" | "iter" | "as_slice") { return false; }
    }
    for implementation in &program.impls {
        let Some(trait_ty) = implementation.trait_ty else { continue; };
        let Type::Path(trait_path) = &program.types[trait_ty].kind else { return false; };
        let name = trait_path.name.as_str();
        if !implementation.module.starts_with("$std")
            && (name == "IntoIterator" || name.ends_with("::IntoIterator")
                || name == "Iterator" || name.ends_with("::Iterator")
                || name == "Index" || name.ends_with("::Index")
                || name == "IndexMut" || name.ends_with("::IndexMut")
                || name == "Deref" || name.ends_with("::Deref")
                || name == "DerefMut" || name.ends_with("::DerefMut")) { return false; }
        if name == "Drop" || name.ends_with("::Drop") {
            let Type::Path(target) = &program.types[implementation.target].kind else { return false; };
            let Some(declaration) = target.declaration else { return false; };
            if declaration == iterator || matches!(program.declarations[declaration].item, Item::Alias(_)) {
                return false;
            }
        }
    }
    true
}

fn vec_field(source: &a::Program, resolved: &a::Program, expression: usize, field: &str) -> bool {
    let Some(function) = function_at(source, source.exprs[expression].span) else { return false; };
    let method = &source.functions[function];
    if !method.generics.is_empty() || method.params.is_empty() { return false; }
    let receiver = &method.params[0];
    if receiver.name != "self" || receiver.mutable { return false; }
    let Type::Ref { mutable: false, inner, .. } = source.types[receiver.ty].kind else { return false; };
    if !matches!(&source.types[inner].kind, Type::Path(path) if path.name == "Self" && path.args.is_empty()) {
        return false;
    }
    let a::FunctionOwner::Implementation(owner) = resolved.function_owners[function] else { return false; };
    let implementation = &resolved.impls[owner.index];
    if implementation.trait_ty.is_some() || !implementation.generics.is_empty() { return false; }
    let Type::Path(target) = &resolved.types[implementation.target].kind else { return false; };
    let Some(declaration) = target.declaration else { return false; };
    let Item::Struct(structure) = resolved.declarations[declaration].item else { return false; };
    let structure = &resolved.structs[structure];
    if !structure.generics.is_empty() { return false; }
    for item in &structure.fields {
        if item.name != field { continue; }
        let Type::Path(path) = &resolved.types[item.ty].kind else { return false; };
        return path.declaration.is_none() && path.name == "Vec" && path.args.len() == 1usize
            && concrete(resolved, path.args[0]);
    }
    false
}

fn advice(program: &a::Program, problem: &Diagnostic) -> Option<Vec<Edit>> {
    if problem.code != "E_BORROW" || !problem.message.contains(":iterator conflicts") { return None; }
    let resolved = crate::resolve::resolve(program).ok()?;
    if !ordinary_protocol(&resolved) { return None; }
    for (id, node) in program.exprs.iter().enumerate() {
        if node.span.file != problem.file
            || !problem.message.starts_with(&format!("$for:{id}:iterator conflicts")) { continue; }
        let Expr::For { pattern, iterator, body } = node.kind else { continue; };
        let Pattern::Bind { name, mutable, by_ref: false, ref_mut: false } = &program.patterns[pattern].kind else { continue; };
        let Expr::Borrow { mutable: false, value } = program.exprs[iterator].kind else { continue; };
        let Expr::Field { value: receiver, name: field } = &program.exprs[value].kind else { continue; };
        if !named_self(program, *receiver) || !tight_place(program, value)
            || !vec_field(program, &resolved, id, field) { continue; }
        let block = program.blocks[body].span;
        if block.end != node.span.end { continue; }
        let mut inside = false;
        for expression in &program.exprs {
            if primary(problem, expression.span) && encloses(block, expression.span) { inside = true; }
        }
        if !inside { continue; }
        let index = fresh(program, "iterator_index");
        let current = fresh(program, "iterator_current");
        let binding = if *mutable { format!("mut {name}") } else { name.clone() };
        return Some(vec![
            Edit { start: node.span.start, end: block.start,
                replacement: format!("{{ let mut {index} = 0usize; while {index} < self.{field}.len() {{ let {current} = {index}; {index} += 1usize; let {binding} = &self.{field}[{current}]; ") },
            Edit { start: block.end, end: block.end, replacement: " } }".to_string() },
        ]);
    }
    None
}

pub(super) fn complete(program: &a::Program, problems: &mut [Diagnostic]) {
    let mut index = 0usize;
    while index < problems.len() {
        let problem = &mut problems[index];
        if problem.edits.is_empty() {
            if let Some(edits) = advice(program, problem) {
                problem.message = format!("{}; returned element view is tied to the loop iterator temporary; help: index this immutable self Vec field directly, advancing before the original body so continue still progresses (docs/language.md, Source library)", problem.message);
                problem.edits = edits;
            }
        }
        index += 1usize;
    }
}
