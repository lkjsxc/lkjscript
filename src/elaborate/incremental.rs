//! Authenticated, disposable front-end checkpoints for stable source/catalog topology.
//! This is producer work, never checker authority. Every resumed IR still enters
//! the independent checker, and every uncertain catalog change is a cold miss.
use super::{a, ir, Compiler, TypeKey, generic_check, diagnostic_inference};
use crate::{ir_binary, sha256};
use std::collections::{BTreeMap, BTreeSet};

pub struct Fingerprint {
    pub shape: String,
    pub bodies: Vec<String>,
}
fn word(out: &mut Vec<u8>, value: usize) {
    for shift in 0..8 { out.push((value >> (shift * 8)) as u8); }
}
fn bytes(out: &mut Vec<u8>, value: &[u8]) {
    word(out, value.len()); out.extend_from_slice(value);
}
fn span(out: &mut Vec<u8>, value: a::Span) {
    word(out, value.file); word(out, value.start); word(out, value.end);
    word(out, value.line); word(out, value.column);
}
struct Reader<'a> { data: &'a [u8], at: usize }
impl<'a> Reader<'a> {
    fn new(data: &'a [u8]) -> Self { Self { data, at: 0 } }
    fn word(&mut self) -> Option<usize> {
        if self.data.len() - self.at < 8 { return None; }
        let mut value = 0usize;
        for shift in 0..8 { value |= (self.data[self.at + shift] as usize) << (shift * 8); }
        self.at += 8; Some(value)
    }
    fn count(&mut self) -> Option<usize> {
        let count = self.word()?;
        if count > (self.data.len() - self.at) / 8 { None } else { Some(count) }
    }
    fn bytes(&mut self) -> Option<Vec<u8>> {
        let count = self.word()?;
        if count > self.data.len() - self.at { return None; }
        let value = self.data[self.at..self.at + count].to_vec();
        self.at += count; Some(value)
    }
    fn text(&mut self) -> Option<String> { String::from_utf8(self.bytes()?).ok() }
    fn done(&self) -> bool { self.at == self.data.len() }
}
fn words(out: &mut Vec<u8>, values: &[usize]) {
    word(out, values.len()); for value in values { word(out, *value); }
}
fn take_words(input: &mut Reader<'_>) -> Option<Vec<usize>> {
    let count = input.count()?; let mut values = Vec::new();
    for _ in 0..count { values.push(input.word()?); } Some(values)
}

/// Bind all declarations and external includes exactly. Body slots can change
/// only while their arena/source spans remain identical; source type spellings
/// are bound even inside those slots. Other edits simply use normal compilation.
pub fn fingerprint(source: &a::Program, inputs: &[(String, Vec<u8>)]) -> Option<Fingerprint> {
    let mut data = b"lkjscript-front-topology-2"[..].to_vec();
    let mut bodies = vec![String::new(); source.functions.len()];
    let mut source_paths = BTreeSet::new();
    for (file, descriptor) in source.files.iter().enumerate() {
        let raw = &inputs.iter().find(|input| input.0 == descriptor.path)?.1;
        source_paths.insert(descriptor.path.clone());
        bytes(&mut data, descriptor.path.as_bytes()); word(&mut data, descriptor.crate_root);
        let mut regions: Vec<(usize, usize, usize)> = Vec::new();
        for (index, function) in source.functions.iter().enumerate() {
            if function.span.file == file {
                if let Some(body) = function.body {
                    let site = source.blocks[body].span;
                    if site.start > site.end || site.end > raw.len() { return None; }
                    regions.push((site.start, site.end, index));
                }
            }
        }
        regions.sort();
        let mut at = 0usize;
        for (start, end, index) in regions {
            if start < at { return None; }
            bytes(&mut data, &raw[at..start]); word(&mut data, start); word(&mut data, end);
            bodies[index] = sha256::hex_digest(&raw[start..end]);
            at = end;
        }
        bytes(&mut data, &raw[at..]);
    }
    // Includes which are not parsed source files still affect elaboration.
    for (path, raw) in inputs {
        if !source_paths.contains(path) { bytes(&mut data, path.as_bytes()); bytes(&mut data, raw); }
    }
    word(&mut data, source.types.len());
    for node in &source.types {
        span(&mut data, node.span); word(&mut data, node.scope);
        let path = &source.files[node.span.file].path;
        let raw = &inputs.iter().find(|input| &input.0 == path)?.1;
        if node.span.start > node.span.end || node.span.end > raw.len() { return None; }
        bytes(&mut data, &raw[node.span.start..node.span.end]);
    }
    word(&mut data, source.exprs.len());
    for node in &source.exprs {
        span(&mut data, node.span); word(&mut data, node.scope);
        // Includes expand to literals before this fingerprint. Their payload is
        // a peer dependency even when the included path is also parsed source.
        match &node.kind {
            a::Expr::String(value) => { word(&mut data, 1); bytes(&mut data, value.as_bytes()); }
            a::Expr::Bytes(value) => { word(&mut data, 2); bytes(&mut data, value); }
            _ => word(&mut data, 0),
        }
    }
    word(&mut data, source.patterns.len());
    for node in &source.patterns { span(&mut data, node.span); word(&mut data, node.scope); }
    word(&mut data, source.blocks.len());
    for node in &source.blocks { span(&mut data, node.span); }
    word(&mut data, source.scopes.len());
    for scope in &source.scopes {
        bytes(&mut data, scope.path.as_bytes()); word(&mut data, scope.parent.map_or(0, |v| v + 1));
        word(&mut data, scope.crate_root);
    }
    word(&mut data, source.functions.len());
    for function in &source.functions { span(&mut data, function.span); word(&mut data, function.body.map_or(0, |v| v + 1)); }
    // Local item declarations may affect peers; do not resume them.
    for block in &source.blocks {
        for statement in &block.statements { if let a::Stmt::Item(item) = statement { if !matches!(item, a::Item::Use(_)) { return None; } } }
    }
    Some(Fingerprint { shape: sha256::hex_digest(&data), bodies })
}

fn key(out: &mut Vec<u8>, value: &TypeKey) {
    match value {
        TypeKey::Ref(inner, flag) => { word(out, 0); word(out, *inner); word(out, *flag as usize); }
        TypeKey::Slice(inner, flag) => { word(out, 1); word(out, *inner); word(out, *flag as usize); }
        TypeKey::Array(inner, size) => { word(out, 2); word(out, *inner); word(out, *size); }
        TypeKey::Tuple(values) => { word(out, 3); words(out, values); }
        TypeKey::Vec(inner) => { word(out, 4); word(out, *inner); }
        TypeKey::Box(inner) => { word(out, 5); word(out, *inner); }
        TypeKey::Nominal(id, values) => { word(out, 6); word(out, *id); words(out, values); }
        TypeKey::World => word(out, 7), TypeKey::Stdout => word(out, 8),
        TypeKey::Stdin => word(out, 9), TypeKey::Stderr => word(out, 10),
        TypeKey::Directory => word(out, 11), TypeKey::File => word(out, 12),
        TypeKey::Process => word(out, 13), TypeKey::Clock => word(out, 14),
        TypeKey::Child => word(out, 15), TypeKey::Listener => word(out, 16),
        TypeKey::Connector => word(out, 17), TypeKey::Readiness => word(out, 18),
        TypeKey::Entropy => word(out, 19), TypeKey::Stop => word(out, 20), TypeKey::Str => word(out, 21),
        TypeKey::Arguments => word(out, 22), TypeKey::Environment => word(out, 23), TypeKey::ExitStatus => word(out, 24),
    }
}
fn take_key(input: &mut Reader<'_>) -> Option<TypeKey> {
    Some(match input.word()? {
        0 => TypeKey::Ref(input.word()?, input.word()? != 0),
        1 => TypeKey::Slice(input.word()?, input.word()? != 0),
        2 => TypeKey::Array(input.word()?, input.word()?),
        3 => TypeKey::Tuple(take_words(input)?), 4 => TypeKey::Vec(input.word()?),
        5 => TypeKey::Box(input.word()?), 6 => TypeKey::Nominal(input.word()?, take_words(input)?),
        7 => TypeKey::World, 8 => TypeKey::Stdout, 9 => TypeKey::Stdin, 10 => TypeKey::Stderr,
        11 => TypeKey::Directory, 12 => TypeKey::File, 13 => TypeKey::Process, 14 => TypeKey::Clock,
        15 => TypeKey::Child, 16 => TypeKey::Listener, 17 => TypeKey::Connector,
        18 => TypeKey::Readiness, 19 => TypeKey::Entropy, 20 => TypeKey::Stop, 21 => TypeKey::Str,
        22 => TypeKey::Arguments, 23 => TypeKey::Environment, 24 => TypeKey::ExitStatus,
        _ => return None,
    })
}
pub(super) fn catalog(c: &Compiler<'_>) -> Vec<u8> {
    let mut out = Vec::new();
    word(&mut out, c.type_names.len());
    for (name, id) in &c.type_names { bytes(&mut out, name.as_bytes()); word(&mut out, *id); }
    word(&mut out, c.type_keys.len());
    for (value, id) in &c.type_keys { key(&mut out, value); word(&mut out, *id); }
    word(&mut out, c.applications.len());
    for (id, (name, args)) in &c.applications { word(&mut out, *id); bytes(&mut out, name.as_bytes()); words(&mut out, args); }
    word(&mut out, c.instance_names.len());
    for ((source, args), id) in &c.instance_names {
        word(&mut out, *source); word(&mut out, args.len());
        for (name, value) in args { bytes(&mut out, name.as_bytes()); word(&mut out, *value); }
        word(&mut out, *id);
    }
    word(&mut out, c.derived_functions.len());
    for ((receiver, method), id) in &c.derived_functions {
        word(&mut out, *receiver); bytes(&mut out, method.name().as_bytes()); word(&mut out, *id);
    }
    word(&mut out, c.return_inference.len());
    for id in &c.return_inference { word(&mut out, *id); }
    out
}
fn restore(c: &mut Compiler<'_>, data: &[u8]) -> Option<()> {
    let mut input = Reader::new(data);
    c.type_names.clear();
    for _ in 0..input.count()? { let name = input.text()?; let id = input.word()?; c.type_names.insert(name, id); }
    for _ in 0..input.count()? { let value = take_key(&mut input)?; let id = input.word()?; c.type_keys.insert(value, id); }
    for _ in 0..input.count()? { let id = input.word()?; let name = input.text()?; let args = take_words(&mut input)?; c.applications.insert(id, (name, args)); }
    for _ in 0..input.count()? {
        let source = input.word()?; let mut args = Vec::new();
        for _ in 0..input.count()? { args.push((input.text()?, input.word()?)); }
        let id = input.word()?; c.instance_names.insert((source, args), id);
    }
    for _ in 0..input.count()? {
        let receiver = input.word()?; let name = input.text()?;
        let method = if name == "$order" { generic_check::BuiltinMethod::Order } else { generic_check::BuiltinMethod::named(&name)? };
        let id = input.word()?; c.derived_functions.insert((receiver, method), id);
    }
    for _ in 0..input.count()? { c.return_inference.insert(input.word()?); }
    if input.done() { Some(()) } else { None }
}

pub fn encode(fingerprint: &Fingerprint, catalog: &[u8], program: &ir::Program) -> Vec<u8> {
    let mut out = b"LKJFRONT2"[..].to_vec();
    bytes(&mut out, fingerprint.shape.as_bytes()); word(&mut out, fingerprint.bodies.len());
    for body in &fingerprint.bodies { bytes(&mut out, body.as_bytes()); }
    bytes(&mut out, catalog); bytes(&mut out, &ir_binary::write(program)); out
}

// The ordered storage/call skeleton proves that the old deterministic interning
// catalog is still demanded in the same order. New locals/calls/types/control
// structure fall back; literal and arithmetic changes do not redo peer bodies.
fn skeleton(function: &ir::Function) -> ir::Function {
    let mut function = function.clone();
    for index in 0..function.body.len() {
        let instruction = &mut function.body[index];
        match &mut instruction.op {
            ir::Op::Int { value, .. } => *value = 0,
            ir::Op::Float { bits, .. } => *bits = 0,
            ir::Op::Text { bytes, .. } => bytes.clear(),
            ir::Op::Binary { op, .. } => *op = ir::BinaryOp::Add,
            ir::Op::Unary { op, .. } => *op = ir::UnaryOp::Neg,
            ir::Op::Trap { message } => message.clear(),
            _ => {}
        }
    }
    function
}
fn same_skeleton(left: &ir::Function, right: &ir::Function) -> bool {
    skeleton(left) == skeleton(right)
}

pub struct Resumed {
    pub program: ir::Program,
    pub catalog: Vec<u8>,
    pub definitions: usize,
    pub functions: usize,
}
pub fn resume<F: FnMut(&str)>(source: &a::Program, fingerprint: &Fingerprint, data: &[u8], mark: &mut F) -> Option<Resumed> {
    if !data.starts_with(b"LKJFRONT2") { return None; }
    let mut input = Reader::new(&data[9..]);
    if input.text()? != fingerprint.shape || input.count()? != fingerprint.bodies.len() { return None; }
    let mut changed = BTreeSet::new();
    for (index, body) in fingerprint.bodies.iter().enumerate() {
        if input.text()? != *body { changed.insert(index); }
    }
    for index in &changed {
        let function = &source.functions[*index];
        if source.function_owner(a::FunctionId { index: *index }) != a::FunctionOwner::Free
            || function.generics.iter().any(|value| !value.lifetime) { return None; }
        let site = source.blocks[function.body?].span;
        for expression in &source.exprs {
            if expression.span.file == site.file && expression.span.start >= site.start && expression.span.end <= site.end
                && matches!(expression.kind, a::Expr::Closure { .. }) { return None; }
        }
    }
    let saved = input.bytes()?;
    let original = ir_binary::read(&input.bytes()?)?;
    if !input.done() { return None; }
    let original_types = original.types.clone();
    let original_destructors = original.destructors.clone();
    let mut c = Compiler::new(source).ok()?;
    c.types = original.types; c.functions = original.functions; c.destructors = original.destructors;
    restore(&mut c, &saved)?;
    mark("front_restore");
    let mut selected = Vec::new();
    for index in &changed { selected.push(*index); }
    let inferred = diagnostic_inference::infer_selected(source, &c.names, &c.implementations, &selected);
    if !inferred.problems.is_empty() { return None; }
    let mut verifier = Compiler::new(source).ok()?;
    let (evidence, problems) = generic_check::validate_demand(source, &mut verifier, inferred.definitions).ok()?;
    if !problems.is_empty() { return None; }
    c.definition_types = evidence.types;
    c.method_selections = evidence.methods;
    c.closures.evidence = evidence.closures;

    mark("type");
    let mut instances = Vec::new();
    for ((source, args), id) in &c.instance_names {
        if changed.contains(source) {
            if c.return_inference.contains(id) { return None; }
            let mut substitutions = BTreeMap::new();
            for (name, value) in args { substitutions.insert(name.clone(), *value); }
            instances.push((*id, super::Instance { source: *source, substitutions }));
        }
    }
    let count = instances.len();
    for (id, instance) in instances {
        let before = c.functions[id].clone();
        c.functions[id].body.clear();
        let params = c.functions[id].params;
        c.functions[id].locals.truncate(params);
        let after = c.compile_function(id, &instance).ok()?;
        if !same_skeleton(&before, &after) { return None; }
        c.functions[id] = after;
    }
    if catalog(&c) != saved || c.types != original_types || c.destructors != original_destructors { return None; }
    let mut roots = BTreeSet::new();
    for id in c.scoped_bodies.keys() { roots.insert(*id); }
    let (scopes, problems) = c.scoped_definitions(&roots);
    if !problems.is_empty() { return None; }
    let program = ir::Program { types: c.types, functions: c.functions, destructors: c.destructors };
    if !crate::check::scoped::check_definitions(&program, &scopes).is_empty() { return None; }
    mark("build_ir");
    Some(Resumed { program, catalog: saved, definitions: changed.len(), functions: count })
}
