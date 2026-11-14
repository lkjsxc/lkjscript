//! Rust-compatible syntax to explicit K. Inference lives here; K trusts none of it.
mod assignment;
mod borrow_cleanup;
mod capabilities;
pub(crate) mod census;
mod census_ownership;
mod closures;
mod constructors;
mod definition_types;
mod derive_check;
mod derive_methods;
mod diagnostic_inference;
mod floats;
mod generic_check;
mod inference;
mod integers;
mod methods;
mod operators;
mod origins;
mod output;
mod pattern_choices;
mod patterns;
mod ranges;
mod scoped_calls;
mod scoped_locals;
mod sequences;
mod static_lifetimes;
mod sugars;
mod text_format;
mod trait_proofs;
mod type_display;
mod type_resolve;
mod variants;

use crate::ast as a;
use crate::check::{borrowed_projection, is_copy, place_type};
use crate::facts::Facts;
use crate::k;
use crate::names::qualified;
use std::collections::{BTreeMap, BTreeSet};

type Result<T> = std::result::Result<T, a::Diagnostic>;
fn err(span: a::Span, code: &str, message: &str) -> a::Diagnostic {
    a::Diagnostic::new(span, code, message)
}
fn at_use_site(problem: a::Diagnostic, span: a::Span) -> a::Diagnostic {
    a::Diagnostic::new(span, &problem.code, &problem.message)
}
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
enum TypeKey {
    Ref(usize, bool),
    Slice(usize, bool),
    Array(usize, usize),
    Tuple(Vec<usize>),
    Vec(usize),
    Box(usize),
    World,
    Stdout,
    Stdin,
    Stderr,
    Directory,
    File,
    Str,
    Nominal(a::DeclarationId, Vec<usize>),
}
#[derive(Clone)]
struct Instance {
    source: usize,
    substitutions: BTreeMap<String, usize>,
}
#[derive(Clone)]
enum PendingFunction {
    Source(Instance),
    Derived(derive_methods::DerivedInstance),
}
struct Compiler<'a> {
    source: &'a a::Program,
    names: crate::resolve::Resolver<'a>,
    implementations: crate::generics::ImplementationIndex,
    field_positions: Vec<BTreeMap<String, usize>>,
    variant_positions: Vec<BTreeMap<String, usize>>,
    anonymous_fields: BTreeMap<usize, BTreeMap<String, usize>>,
    types: Vec<k::Type>,
    type_names: BTreeMap<String, usize>,
    type_keys: BTreeMap<TypeKey, usize>,
    declaration_names: BTreeMap<String, a::DeclarationId>,
    functions: Vec<k::Function>,
    pending_functions: Vec<(usize, PendingFunction)>,
    derived_functions: BTreeMap<(usize, generic_check::BuiltinMethod), usize>,
    instance_names: BTreeMap<(usize, Vec<(String, usize)>), usize>,
    applications: BTreeMap<usize, (String, Vec<usize>)>,
    destructors: Vec<(usize, usize)>,
    uses: BTreeMap<(a::FileId, String), Vec<usize>>,
    contracts: Option<generic_check::Contracts>,
    method_selections: generic_check::MethodSelections,
    definition_types: BTreeMap<usize, definition_types::DefinitionTypes>,
    closures: closures::Catalog,
    scoped_bodies: BTreeMap<usize, scoped_locals::Body>,
    trait_proofs: Option<trait_proofs::Session>,
    return_inference: BTreeSet<usize>,
    diagnostic_functions: Option<&'a BTreeSet<usize>>,
}

fn live_indices(facts: &Facts, start: usize) -> Vec<usize> {
    let mut indices = Vec::new();
    let mut remaining = facts.indices_from(start);
    while let Some(index) = remaining.next() {
        indices.push(index);
    }
    indices
}
// Source construction retains the defining instruction's location. Diagnose an
// impossible owned unsized local here before K's independent structural gate;
// this provenance check never grants checked-program authority.
fn unsized_local_problem(
    types: &[k::Type],
    function: &k::Function,
    source_span: a::Span,
) -> Option<a::Diagnostic> {
    for local_index in 0..function.locals.len() {
        let local = &function.locals[local_index];
        let Some(ty) = types.get(local.ty) else {
            continue;
        };
        if !matches!(ty.kind, k::TypeKind::Slice { .. } | k::TypeKind::Str) {
            continue;
        }
        let mut problem = err(
            source_span,
            "E_UNSUPPORTED",
            &format!(
                "unsized values need a borrowed type: local {} ({}) in function {}",
                local.name, ty.name, function.name
            ),
        );
        for instruction in &function.body {
            let destination = match &instruction.op {
                k::Op::Text { dst, .. }
                | k::Op::Int { dst, .. }
                | k::Op::Float { dst, .. }
                | k::Op::Unit { dst }
                | k::Op::Read { dst, .. }
                | k::Op::Borrow { dst, .. }
                | k::Op::SliceBorrow { dst, .. }
                | k::Op::ViewRange { dst, .. }
                | k::Op::Array { dst, .. }
                | k::Op::Unary { dst, .. }
                | k::Op::Binary { dst, .. }
                | k::Op::Call { dst, .. }
                | k::Op::Struct { dst, .. }
                | k::Op::Variant { dst, .. }
                | k::Op::Discriminant { dst, .. }
                | k::Op::TagTest { dst, .. }
                | k::Op::Intrinsic { dst, .. } => Some(*dst),
                k::Op::Assign { place, .. } if place.projections.is_empty() => Some(place.local),
                _ => None,
            };
            if destination == Some(local_index) {
                problem.line = instruction.line;
                problem.column = instruction.column;
                break;
            }
        }
        return Some(problem);
    }
    None
}
fn first_problem(problems: Vec<a::Diagnostic>) -> a::Diagnostic {
    problems
        .into_iter()
        .next()
        .expect("failed analysis has a diagnostic")
}

pub fn elaborate(source: &a::Program) -> Result<k::Program> {
    match elaborate_all(source) {
        std::result::Result::Ok(program) => std::result::Result::Ok(program),
        std::result::Result::Err(problems) => std::result::Result::Err(first_problem(problems)),
    }
}

/// Resolve declarations before collecting independent definition diagnostics.
pub fn elaborate_all(source: &a::Program) -> std::result::Result<k::Program, Vec<a::Diagnostic>> {
    let resolved = match crate::resolve::resolve(source) {
        std::result::Result::Ok(resolved) => resolved,
        std::result::Result::Err(error) => return std::result::Result::Err(vec![error]),
    };
    elaborate_resolved_all(&resolved)
}

/// Elaborate an already resolved source graph; the independent K gate follows.
pub fn elaborate_resolved(source: &a::Program) -> Result<k::Program> {
    match elaborate_resolved_all(source) {
        std::result::Result::Ok(program) => std::result::Result::Ok(program),
        std::result::Result::Err(problems) => std::result::Result::Err(first_problem(problems)),
    }
}

/// Recorded definition constraints and rigid generic evidence. Only this phase
/// selects source-level generic witnesses; concrete construction consumes them.
pub struct TypedDefinitions<'source> {
    compiler: Compiler<'source>,
    evidence: generic_check::DefinitionEvidence,
    problems: Vec<a::Diagnostic>,
}

/// Keep independent body results when signatures are valid. This report cannot
/// authorize emission: lower_definitions returns all saved errors before K can
/// escape, after checking the independently constructed bodies.
pub fn infer_resolved_report(
    source: &a::Program,
) -> std::result::Result<TypedDefinitions<'_>, Vec<a::Diagnostic>> {
    if let std::result::Result::Err(error) = static_lifetimes::annotations(source) {
        return std::result::Result::Err(vec![error]);
    }
    if let std::result::Result::Err(error) = crate::generics::validate(source) {
        return std::result::Result::Err(vec![error]);
    }
    if let std::result::Result::Err(error) = static_lifetimes::outputs(source) {
        return std::result::Result::Err(vec![error]);
    }
    let mut compiler = match Compiler::new(source) {
        std::result::Result::Ok(compiler) => compiler,
        std::result::Result::Err(error) => return std::result::Result::Err(vec![error]),
    };
    if let std::result::Result::Err(error) = compiler.check_capability_methods() {
        return std::result::Result::Err(vec![error]);
    }
    let inferred =
        diagnostic_inference::infer_all(source, &compiler.names, &compiler.implementations);
    let (evidence, reported) = match generic_check::validate_report(source, inferred.definitions) {
        std::result::Result::Ok(report) => report,
        std::result::Result::Err(error) => {
            let mut errors = inferred.problems;
            errors.push(error);
            return std::result::Result::Err(errors);
        }
    };
    let mut problems = inferred.problems;
    for problem in reported {
        problems.push(problem);
    }
    std::result::Result::Ok(TypedDefinitions {
        compiler,
        evidence,
        problems,
    })
}

/// The strict type-phase API preserves its all-or-error result.
pub fn infer_resolved_all(
    source: &a::Program,
) -> std::result::Result<TypedDefinitions<'_>, Vec<a::Diagnostic>> {
    let definitions = infer_resolved_report(source)?;
    if definitions.problems.is_empty() {
        std::result::Result::Ok(definitions)
    } else {
        std::result::Result::Err(definitions.problems)
    }
}

/// Construct and check independent bodies even after peer bodies fail. Failed
/// bodies have their declared signatures and a diagnostic-only trap body; the
/// accumulated error list prevents this program from leaving this function.
pub fn lower_definitions(
    definitions: TypedDefinitions<'_>,
) -> std::result::Result<k::Program, Vec<a::Diagnostic>> {
    let source = definitions.compiler.source;
    let mut problems = definitions.problems;
    let meaning = match elaborate_definitions(
        source,
        definitions.compiler,
        definitions.evidence,
        &mut problems,
    ) {
        std::result::Result::Ok(meaning) => meaning,
        std::result::Result::Err(error) => {
            problems.push(error);
            return std::result::Result::Err(problems);
        }
    };
    if problems.is_empty() {
        return std::result::Result::Ok(meaning);
    }
    if let std::result::Result::Err(errors) = crate::check::check_all(meaning) {
        for error in errors {
            problems.push(error);
        }
    }
    let mut seen: BTreeSet<(usize, usize, usize, String, String)> = BTreeSet::new();
    let mut unique = Vec::new();
    for problem in problems {
        if seen.insert((
            problem.file,
            problem.line,
            problem.column,
            problem.code.clone(),
            problem.message.clone(),
        )) {
            unique.push(problem);
        }
    }
    std::result::Result::Err(unique)
}

pub fn elaborate_resolved_all(
    source: &a::Program,
) -> std::result::Result<k::Program, Vec<a::Diagnostic>> {
    match infer_resolved_report(source) {
        std::result::Result::Ok(definitions) => lower_definitions(definitions),
        std::result::Result::Err(problems) => std::result::Result::Err(problems),
    }
}

fn elaborate_definitions(
    source: &a::Program,
    mut c: Compiler<'_>,
    evidence: generic_check::DefinitionEvidence,
    problems: &mut Vec<a::Diagnostic>,
) -> Result<k::Program> {
    c.method_selections = evidence.methods;
    c.definition_types = evidence.types;
    c.closures.evidence = evidence.closures;
    let empty = BTreeMap::new();
    for s in &source.structs {
        if s.generics.is_empty() {
            let declaration = c
                .declaration_names
                .get(&qualified(&s.module, &s.name))
                .copied();
            let _ = c.named_type(
                &a::Path {
                    declaration,
                    name: qualified(&s.module, &s.name),
                    args: Vec::new(),
                },
                &empty,
                s.span,
            )?;
        }
    }
    for e in &source.enums {
        if e.generics.is_empty() {
            let declaration = c
                .declaration_names
                .get(&qualified(&e.module, &e.name))
                .copied();
            let _ = c.named_type(
                &a::Path {
                    declaration,
                    name: qualified(&e.module, &e.name),
                    args: Vec::new(),
                },
                &empty,
                e.span,
            )?;
        }
    }
    for i in 0..source.functions.len() {
        let f = &source.functions[i];
        if f.body.is_some()
            && f.generics.iter().all(|g| g.lifetime)
            && source.function_owner(a::FunctionId { index: i }) == a::FunctionOwner::Free
        {
            c.instantiate(i, empty.clone())?;
        }
    }
    let mut next = 0;
    let mut next_type = 0;
    while next < c.pending_functions.len() || next_type < c.types.len() {
        while next_type < c.types.len() {
            c.register_drop(next_type)?;
            next_type += 1;
        }
        if next < c.pending_functions.len() {
            let (index, pending) = c.pending_functions[next].clone();
            let compiled = match pending {
                PendingFunction::Source(instance)
                    if !c.definition_types.contains_key(&instance.source) =>
                {
                    None
                }
                PendingFunction::Source(instance) => Some(c.compile_function(index, &instance)),
                PendingFunction::Derived(instance) => Some(c.compile_derived(index, &instance)),
            };
            match compiled {
                Some(Ok(function)) => c.functions[index] = function,
                failed => {
                    if let Some(Err(problem)) = failed {
                        problems.push(problem);
                    }
                    if problems.is_empty() {
                        return Err(err(
                            a::Span::default(),
                            "E_BLOCKED",
                            "a body has no successful definition inference",
                        ));
                    }
                    c.functions[index].body = vec![k::Instruction {
                        column: 1,
                        op: k::Op::Trap {
                            message: "body blocked by a source diagnostic".to_string(),
                        },
                        line: 1,
                    }];
                }
            }
            next += 1;
        }
    }
    let mut refinement = c.closures.checked_functions.clone();
    for function in &c.return_inference {
        refinement.insert(*function);
    }
    let mut scoped_roots = BTreeSet::new();
    for function in c.scoped_bodies.keys() {
        scoped_roots.insert(*function);
    }
    let (scopes, scoped_problems) = c.scoped_definitions(&scoped_roots);
    for problem in scoped_problems {
        problems.push(problem);
    }
    let mut program = k::Program {
        types: c.types,
        functions: c.functions,
        destructors: c.destructors,
    };
    for problem in crate::check::refine_return_contracts(&mut program, &refinement) {
        problems.push(problem);
    }
    for problem in crate::check::scoped::check_definitions(&program, &scopes) {
        problems.push(problem);
    }
    Ok(program)
}

impl<'a> Compiler<'a> {
    fn new(source: &'a a::Program) -> Result<Self> {
        Ok(Self::with_indexes(
            source,
            crate::resolve::Resolver::new(source)?,
            crate::generics::ImplementationIndex::new(source)?,
        ))
    }
    fn with_indexes(
        source: &'a a::Program,
        names: crate::resolve::Resolver<'a>,
        implementations: crate::generics::ImplementationIndex,
    ) -> Self {
        let mut field_positions = Vec::new();
        for definition in &source.structs {
            let mut positions = BTreeMap::new();
            for i in 0..definition.fields.len() {
                positions.insert(definition.fields[i].name.clone(), i);
            }
            field_positions.push(positions);
        }
        let mut variant_positions = Vec::new();
        for definition in &source.enums {
            let mut positions = BTreeMap::new();
            for i in 0..definition.variants.len() {
                positions.insert(definition.variants[i].name.clone(), i);
            }
            variant_positions.push(positions);
        }
        let mut declaration_names = BTreeMap::new();
        for id in 0..source.declarations.len() {
            declaration_names.insert(source.declarations[id].name.clone(), id);
        }
        let mut c = Compiler {
            source,
            names,
            implementations,
            field_positions,
            variant_positions,
            anonymous_fields: BTreeMap::new(),
            types: k::primitives(),
            type_names: BTreeMap::new(),
            type_keys: BTreeMap::new(),
            declaration_names,
            functions: Vec::new(),
            pending_functions: Vec::new(),
            derived_functions: BTreeMap::new(),
            instance_names: BTreeMap::new(),
            applications: BTreeMap::new(),
            destructors: Vec::new(),
            uses: BTreeMap::new(),
            contracts: None,
            method_selections: BTreeMap::new(),
            definition_types: BTreeMap::new(),
            closures: closures::Catalog::default(),
            scoped_bodies: BTreeMap::new(),
            trait_proofs: None,
            return_inference: BTreeSet::new(),
            diagnostic_functions: None,
        };
        for i in 0..c.types.len() {
            c.type_names.insert(c.types[i].name.clone(), i);
        }
        for index in 0..6 {
            let (name, bits, signed): (&str, u8, bool) = match index {
                0 => ("u64", 64, false),
                1 => ("isize", 64, true),
                2 => ("u16", 16, false),
                3 => ("u32", 32, false),
                4 => ("i8", 8, true),
                _ => ("i16", 16, true),
            };
            let id = c.types.len();
            c.types.push(k::Type {
                name: name.to_string(),
                kind: k::TypeKind::Int { bits, signed },
            });
            c.type_names.insert(name.to_string(), id);
        }
        let character = c.types.len();
        c.types.push(k::Type {
            name: "char".to_string(),
            kind: k::TypeKind::Char,
        });
        c.type_names.insert("char".to_string(), character);
        for node in &source.exprs {
            if let a::Expr::Path(path) = &node.kind {
                let key = (node.span.file, path.name.clone());
                if !c.uses.contains_key(&key) {
                    c.uses.insert(key.clone(), Vec::new());
                }
                c.uses.get_mut(&key).unwrap().push(node.span.start);
            }
        }
        let mut sorted_uses = BTreeMap::new();
        for (key, mut positions) in c.uses {
            positions.sort();
            sorted_uses.insert(key, positions);
        }
        c.uses = sorted_uses;
        c
    }
}

impl Compiler<'_> {
    fn declaration_item(&self, path: &a::Path) -> Option<a::Item> {
        path.declaration
            .and_then(|id| self.source.declarations.get(id))
            .filter(|declaration| declaration.name == path.name)
            .map(|declaration| declaration.item)
    }
    fn field_index(&mut self, ty: usize, name: &str) -> Option<usize> {
        if let Some((owner, _)) = self.applications.get(&ty) {
            if let Some(a::Item::Struct(index)) = self
                .declaration_names
                .get(owner)
                .map(|id| self.source.declarations[*id].item)
            {
                return self.field_positions[index].get(&name.to_string()).copied();
            }
        }
        if !self.anonymous_fields.contains_key(&ty) {
            let k::TypeKind::Struct { fields, .. } = &self.types[ty].kind else {
                return None;
            };
            let mut positions = BTreeMap::new();
            for i in 0..fields.len() {
                positions.insert(fields[i].name.clone(), i);
            }
            self.anonymous_fields.insert(ty, positions);
        }
        self.anonymous_fields
            .get(&ty)
            .unwrap()
            .get(&name.to_string())
            .copied()
    }
    fn field_access(&self, ty: usize, name: &str, scope: a::ModuleId, span: a::Span) -> Result<()> {
        if let Some((owner, _)) = self.applications.get(&ty) {
            if let Some(a::Item::Struct(index)) = self
                .declaration_names
                .get(owner)
                .map(|id| self.source.declarations[*id].item)
            {
                let definition = &self.source.structs[index];
                if let Some(position) = self.field_positions[index].get(&name.to_string()) {
                    self.declared_field_access(
                        definition,
                        &definition.fields[*position],
                        scope,
                        span,
                    )?;
                }
            }
        }
        Ok(())
    }
    fn declared_field_access(
        &self,
        definition: &a::Struct,
        field: &a::Field,
        scope: a::ModuleId,
        span: a::Span,
    ) -> Result<()> {
        let owner = self.names.scope(&definition.module);
        if !crate::resolve::visible(self.source, owner, field.public, scope) {
            return Err(err(
                span,
                "E_PRIVACY",
                &format!(
                    "field {}::{} is private",
                    qualified(&definition.module, &definition.name),
                    field.name
                ),
            ));
        }
        Ok(())
    }
    fn method_access(&self, index: usize, scope: a::ModuleId, span: a::Span) -> Result<()> {
        let method = &self.source.functions[index];
        let implementation = match self.source.function_owner(a::FunctionId { index }) {
            a::FunctionOwner::Implementation(owner) => Some(&self.source.impls[owner.index]),
            _ => None,
        };
        if let Some(ty) = implementation.and_then(|im| im.trait_ty) {
            if let a::Type::Path(path) = &self.source.types[ty].kind {
                if path
                    .declaration
                    .is_some_and(|id| !self.names.declaration_visible(scope, id))
                {
                    return Err(err(span, "E_PRIVACY", "method belongs to a private trait"));
                }
            }
            return Ok(());
        }
        if !crate::resolve::visible(
            self.source,
            self.names.scope(&method.module),
            method.public,
            scope,
        ) {
            return Err(err(
                span,
                "E_PRIVACY",
                &format!("method {} is private", method.name),
            ));
        }
        Ok(())
    }

    fn intern(&mut self, _description: String, kind: k::TypeKind) -> usize {
        let (key, name) = match &kind {
            k::TypeKind::Ref { inner, mutable } => (
                TypeKey::Ref(*inner, *mutable),
                format!("&{}t{}", if *mutable { "mut " } else { "" }, inner),
            ),
            k::TypeKind::Slice { inner, mutable } => (
                TypeKey::Slice(*inner, *mutable),
                format!("[{}t{}]", if *mutable { "mut " } else { "" }, inner),
            ),
            k::TypeKind::Array { inner, len } => (
                TypeKey::Array(*inner, *len),
                format!("[t{};{}]", inner, len),
            ),
            k::TypeKind::Struct { fields, .. } => {
                let mut ids = Vec::new();
                let mut name = "(".to_string();
                for i in 0..fields.len() {
                    let ty = fields[i].ty;
                    ids.push(ty);
                    if i > 0 {
                        name.push(',');
                    }
                    name.push_str(&format!("t{ty}"));
                }
                name.push(')');
                (TypeKey::Tuple(ids), name)
            }
            k::TypeKind::Vec { inner } => (TypeKey::Vec(*inner), format!("Vec<t{inner}>")),
            k::TypeKind::Box { inner } => (TypeKey::Box(*inner), format!("Box<t{inner}>")),
            k::TypeKind::World => (TypeKey::World, "World".to_string()),
            k::TypeKind::Stdout => (TypeKey::Stdout, "Stdout".to_string()),
            k::TypeKind::Stdin => (TypeKey::Stdin, "Stdin".to_string()),
            k::TypeKind::Stderr => (TypeKey::Stderr, "Stderr".to_string()),
            k::TypeKind::Directory => (TypeKey::Directory, "Directory".to_string()),
            k::TypeKind::File => (TypeKey::File, "File".to_string()),
            k::TypeKind::Str => (TypeKey::Str, "str".to_string()),
            _ => unreachable!("only structural types use intern"),
        };
        if let Some(id) = self.type_keys.get(&key) {
            return *id;
        }
        let id = self.types.len();
        self.types.push(k::Type { name, kind });
        self.type_keys.insert(key, id);
        id
    }

    fn reference(&mut self, inner: usize, mutable: bool) -> usize {
        self.intern(String::new(), k::TypeKind::Ref { inner, mutable })
    }
    fn has_view(&self, ty: usize) -> bool {
        let mut todo = vec![ty];
        let mut seen = vec![false; self.types.len()];
        while let Some(t) = todo.pop() {
            if seen[t] {
                continue;
            }
            seen[t] = true;
            match &self.types[t].kind {
                k::TypeKind::Ref { .. } | k::TypeKind::Slice { .. } => return true,
                k::TypeKind::Struct { fields, .. } => {
                    for f in fields {
                        todo.push(f.ty);
                    }
                }
                k::TypeKind::Enum { variants, .. } => {
                    for v in variants {
                        for f in &v.fields {
                            todo.push(f.ty);
                        }
                    }
                }
                k::TypeKind::Array { inner, .. } => todo.push(*inner),
                _ => {}
            }
        }
        false
    }
    fn lifetimes(&self, id: usize) -> Vec<String> {
        let mut found = Vec::new();
        let mut todo = vec![id];
        while let Some(t) = todo.pop() {
            match &self.source.types[t].kind {
                a::Type::Ref {
                    lifetime, inner, ..
                } => {
                    if !lifetime.is_empty() && lifetime != "'_" && !found.contains(lifetime) {
                        found.push(lifetime.clone());
                    }
                    todo.push(*inner);
                }
                a::Type::Path(p) => {
                    if p.name.starts_with("'") && p.name != "'_" && !found.contains(&p.name) {
                        found.push(p.name.clone());
                    }
                    todo.extend_from_slice(&p.args);
                }
                a::Type::Tuple(ts) => todo.extend_from_slice(ts),
                a::Type::Slice(t) | a::Type::Array { inner: t, .. } => todo.push(*t),
                _ => {}
            }
        }
        found
    }
    fn return_source(
        &self,
        source_index: usize,
        source: &a::Function,
        locals: &[k::Local],
        result: usize,
        substitutions: &BTreeMap<String, usize>,
    ) -> Result<(Option<k::ReturnSource>, bool)> {
        if !self.has_view(result) {
            return Ok((None, false));
        }
        self.return_elision(source_index, locals, result)?;
        if let Some(leaves) =
            self.produced_return_leaf_contract(source_index, source, locals, result, substitutions)
        {
            return Ok((Some(k::ReturnSource::Leaves(leaves)), true));
        }
        if let Some(leaves) = self.return_leaf_contract(source_index, source, locals, result)? {
            return Ok((Some(k::ReturnSource::Leaves(leaves)), false));
        }
        if static_lifetimes::has_static(self.source, source.result) {
            return Ok((Some(k::ReturnSource::Static), false));
        }
        let lifetimes = self.lifetimes(source.result);
        if lifetimes.len() > 1 {
            return Err(err(
                source.span,
                "E_BORROW",
                "a returned view has one source lifetime",
            ));
        }
        let mut candidates: Vec<usize> = Vec::new();
        if let Some(lifetime) = lifetimes.first() {
            for i in 0..source.params.len() {
                if self.lifetimes(source.params[i].ty).contains(lifetime) {
                    candidates.push(i);
                }
            }
        } else if source.params.first().is_some_and(|p| p.name == "self")
            && self.has_view(locals[0].ty)
        {
            candidates.push(0);
        } else {
            for i in 0..source.params.len() {
                if self.has_view(locals[i].ty) {
                    candidates.push(i);
                }
            }
        }
        if candidates.len() > 1 {
            return Err(err(
                source.span,
                "E_BORROW",
                "returned view selects more than one source input",
            ));
        }
        let Some(index) = candidates.first().copied() else {
            // A named output lifetime not connected to any input is universal.
            // Every live shared leaf therefore needs static backing; inactive
            // enum leaves also satisfy this explicit K contract.
            if !lifetimes.is_empty() && crate::check::shared_view(&self.types, result) {
                return Ok((Some(k::ReturnSource::Static), false));
            }
            return Ok((None, false));
        };
        let mut dereferences = 0;
        let mut selected = locals[index].ty;
        if let Some(required) = lifetimes.first() {
            // An explicitly returned inner lifetime does not borrow the outer
            // reference carrier. Stop when that outer lifetime is itself named.
            let mut syntax = source.params[index].ty;
            while let a::Type::Ref {
                lifetime, inner, ..
            } = &self.source.types[syntax].kind
            {
                if lifetime == required || !self.lifetimes(*inner).contains(required) {
                    break;
                }
                let k::TypeKind::Ref { inner: value, .. } = self.types[selected].kind else {
                    break;
                };
                selected = value;
                syntax = *inner;
                dereferences += 1;
            }
        } else if selected != result {
            // Returning the borrowed value itself (including a specialized T)
            // transports its reference leaves, rather than the borrow of its slot.
            while let k::TypeKind::Ref { inner, .. } = self.types[selected].kind {
                selected = inner;
                dereferences += 1;
                if selected == result {
                    break;
                }
            }
            if selected != result {
                selected = locals[index].ty;
                dereferences = 0;
            }
        }
        if dereferences > 0 && !crate::check::shared_view(&self.types, selected) {
            return Err(err(
                source.span,
                "E_UNSUPPORTED",
                "projected return sources require shared reference leaves; exclusive reborrows retain their outer lifetime",
            ));
        }
        Ok((
            Some(k::ReturnSource::Parameter {
                index,
                projections: vec![k::Projection::Deref; dereferences],
            }),
            false,
        ))
    }
    fn instantiate(&mut self, index: usize, mut sub: BTreeMap<String, usize>) -> Result<usize> {
        if self
            .diagnostic_functions
            .is_some_and(|functions| !functions.contains(&index))
        {
            return Err(err(
                self.source.functions[index].span,
                "E_BLOCKED",
                "callee contracts are outside the available ownership region",
            ));
        }
        self.prepare_associated(index, &mut sub)?;
        let source = self.source.functions[index].clone();
        let mut substitutions = Vec::new();
        for (name, ty) in &sub {
            substitutions.push((name.clone(), *ty));
        }
        let key = (index, substitutions);
        if let Some(id) = self.instance_names.get(&key) {
            return Ok(*id);
        }
        let mut name = qualified(&source.module, &source.name);
        if self.source.function_owner(a::FunctionId { index }) != a::FunctionOwner::Free {
            name.push_str(&format!("$f{index}"));
        }
        if !sub.is_empty() {
            name.push('<');
            for (key, value) in &sub {
                name.push_str(key);
                name.push('=');
                name.push_str(&format!("t{value}"));
                name.push(';');
            }
            name.push('>');
        }
        self.bounds(&source.generics, &sub)?;
        let mut locals = Vec::new();
        for p in &source.params {
            locals.push(k::Local {
                name: p.name.clone(),
                ty: self.resolve(p.ty, &sub)?,
                mutable: p.mutable,
            });
        }
        let result = self.resolve(source.result, &sub)?;
        let (return_source, infer_return) =
            self.return_source(index, &source, &locals, result, &sub)?;
        let id = self.functions.len();
        if infer_return {
            self.return_inference.insert(id);
        }
        self.instance_names.insert(key, id);
        self.pending_functions.push((
            id,
            PendingFunction::Source(Instance {
                source: index,
                substitutions: sub,
            }),
        ));
        let mut test = false;
        for attribute in &source.attrs {
            if attribute.name == "test" {
                test = true;
                break;
            }
        }
        self.functions.push(k::Function {
            file: source.span.file,
            name,
            params: locals.len(),
            result,
            return_source,
            locals,
            body: Vec::new(),
            test,
        });
        Ok(id)
    }
    fn bounds(
        &mut self,
        parameters: &[a::GenericParam],
        sub: &BTreeMap<String, usize>,
    ) -> Result<()> {
        // Method selection checks implementation bounds before it instantiates
        // the selected method. Callable signatures need the same concrete
        // associated substitutions that ordinary function instantiation uses.
        let mut callable = false;
        for parameter in parameters {
            for bound in &parameter.bounds {
                if self.is_callable_bound(*bound) {
                    callable = true;
                    break;
                }
            }
            if callable {
                break;
            }
        }
        let prepared: Option<BTreeMap<String, usize>> = if callable {
            let mut prepared = sub.clone();
            self.prepare_callable_arguments(parameters, &mut prepared)?;
            self.prepare_parameter_associated(parameters, &mut prepared)?;
            Some(prepared)
        } else {
            None
        };
        let sub = match &prepared {
            Some(prepared) => prepared,
            None => sub,
        };
        for parameter in parameters {
            if parameter.lifetime {
                continue;
            }
            let ty = *sub
                .get(&parameter.name)
                .ok_or_else(|| err(parameter.span, "E_TYPE", "cannot infer generic argument"))?;
            for id in &parameter.bounds {
                if let a::Type::Path(path) = self.source.types[*id].kind.clone() {
                    if self.callable_bound(ty, &path, sub, parameter.span)? {
                        continue;
                    }
                }
                let (bound, identity) = match &self.source.types[*id].kind {
                    a::Type::Path(p) if p.args.is_empty() => (
                        p.name.clone(),
                        crate::generics::bound_identity(self.source, p),
                    ),
                    _ => {
                        return Err(err(
                            parameter.span,
                            "E_UNSUPPORTED",
                            "parameterized trait bounds are not implemented yet",
                        ));
                    }
                };
                if !self.satisfies_identity(ty, &identity) {
                    return Err(err(
                        parameter.span,
                        "E_TRAIT",
                        &format!("{} does not implement {}", self.display_type(ty), bound),
                    ));
                }
            }
        }
        Ok(())
    }
    fn register_drop(&mut self, ty: usize) -> Result<()> {
        if !matches!(
            self.types[ty].kind,
            k::TypeKind::Struct { .. } | k::TypeKind::Enum { .. }
        ) {
            return Ok(());
        }
        let candidates = self.implementations.candidates(
            &self.types,
            &self.applications,
            ty,
            &crate::generics::TraitIdentity::Builtin("Drop".to_string()),
        );
        for index in candidates {
            let im = self.source.impls[index].clone();
            if let Some(trait_ty) = im.trait_ty {
                let builtin_drop = matches!(
                    &self.source.types[trait_ty].kind,
                    a::Type::Path(path) if crate::generics::bound_identity(self.source, path)
                        == crate::generics::TraitIdentity::Builtin("Drop".to_string())
                );
                if builtin_drop {
                    let mut sub = BTreeMap::new();
                    let pattern = match self.source.types[im.target].kind.clone() {
                        a::Type::Path(p) => p,
                        _ => return Err(err(im.span, "E_TRAIT", "Drop requires a nominal type")),
                    };
                    let (name, actual) = self
                        .applications
                        .get(&ty)
                        .cloned()
                        .unwrap_or((self.types[ty].name.clone(), Vec::new()));
                    if pattern.name != name {
                        continue;
                    }
                    let mut parameters = Vec::new();
                    for parameter in &pattern.args {
                        if !matches!(&self.source.types[*parameter].kind, a::Type::Path(path) if path.name.starts_with("'"))
                        {
                            parameters.push(*parameter);
                        }
                    }
                    if parameters.len() != actual.len() {
                        continue;
                    }
                    for i in 0..parameters.len() {
                        match &self.source.types[parameters[i]].kind {
                            a::Type::Path(p) if p.args.is_empty() => {
                                sub.insert(p.name.clone(), actual[i]);
                            }
                            _ => {
                                return Err(err(
                                    im.span,
                                    "E_TRAIT",
                                    "Drop implementations cannot specialize type arguments",
                                ));
                            }
                        }
                    }
                    sub.insert("Self".to_string(), ty);
                    self.bounds(&im.generics, &sub)?;
                    let mut methods = Vec::new();
                    for method in &im.methods {
                        if self.source.functions[*method].name == "drop" {
                            methods.push(*method);
                        }
                    }
                    if methods.len() != 1 {
                        return Err(err(im.span, "E_TRAIT", "Drop requires one drop method"));
                    }
                    let function = self.instantiate(methods[0], sub)?;
                    self.destructors.push((ty, function));
                }
            }
        }
        Ok(())
    }
    fn compile_function(&mut self, index: usize, instance: &Instance) -> Result<k::Function> {
        let source = &self.source.functions[instance.source];
        let span = source.span;
        let name = qualified(&source.module, &source.name);
        let function = self
            .compile_function_body(index, instance)
            .map_err(|problem| {
                if problem.line == 0 {
                    err(
                        span,
                        &problem.code,
                        &format!("{} in function {name}", problem.message),
                    )
                } else {
                    problem
                }
            })?;
        if let Some(problem) = unsized_local_problem(&self.types, &function, span) {
            return Err(problem);
        }
        Ok(function)
    }
    #[allow(clippy::mem_replace_with_default)]
    fn compile_function_body(&mut self, index: usize, instance: &Instance) -> Result<k::Function> {
        let definition = match self.definition_types.remove(&instance.source) {
            Some(definition) => definition,
            None => definition_types::infer(
                self.source,
                &self.names,
                &self.implementations,
                instance.source,
            )?,
        };
        let inferred = definition.instantiate(self, &instance.substitutions);
        self.definition_types.insert(instance.source, definition);
        let inferred = inferred?;
        let source = self.source.functions[instance.source].clone();
        let mut function = self.functions[index].clone();
        let bindings = Bindings::from_locals(&function.locals, 0);
        let live = Facts::filled(function.locals.len(), true);
        let references = Facts::new(live.len());
        let scope = self.names.scope(&source.module);
        let mut builder = Builder {
            source_function: instance.source,
            scope,
            file: source.span.file,
            compiler: self,
            function: &mut function,
            return_annotated: true,
            scoped_body: scoped_locals::Body::default(),
            typed_bindings: Vec::new(),
            sub: instance.substitutions.clone(),
            bindings,
            guard_aliases: BTreeMap::new(),
            guard_shells: Vec::new(),
            capture_aliases: BTreeMap::new(),
            capture_locals: std::collections::BTreeSet::new(),
            active_span: source.span,
            reference_updates: references.clone(),
            reference_schedules: BTreeMap::new(),
            references,
            carriers: BTreeMap::new(),
            dependents: BTreeMap::new(),
            assignment_types: BTreeMap::new(),
            range_borrows: BTreeMap::new(),
            inferred,
            written: live.clone(),
            definite_live: live.clone(),
            live,
            loops: Vec::new(),
            reachable: true,
        };
        let result_ty = builder.function.result;
        let source_body = match source.body {
            Some(body) => body,
            None => return Err(err(source.span, "E_TYPE", "function has no body")),
        };
        let result = builder.block(source_body, Some(result_ty))?;
        if builder.reachable {
            if result.1 != result_ty {
                return Err(err(source.span, "E_TYPE", "function result type differs"));
            }
            builder.cleanup(0, Some(result.0), source.span.line);
            builder.emit(k::Op::Return { src: result.0 }, source.span.line);
        }
        let body = std::mem::replace(&mut builder.scoped_body, scoped_locals::Body::default());
        drop(builder);
        if !body.is_empty() {
            self.scoped_bodies.insert(index, body);
        }
        Ok(function)
    }
}

/// The stack records lexical scope restoration; the index answers each use
/// without scanning earlier declarations. A shadow has its own stack entry.
#[derive(Default)]
struct Bindings {
    entries: Vec<(String, usize)>,
    visible: BTreeMap<String, Vec<usize>>,
}
impl Bindings {
    fn push(&mut self, binding: (String, usize)) {
        if !self.visible.contains_key(&binding.0) {
            self.visible.insert(binding.0.clone(), Vec::new());
        }
        self.visible
            .get_mut(&binding.0)
            .unwrap()
            .push(self.entries.len());
        self.entries.push(binding);
    }
    fn len(&self) -> usize {
        self.entries.len()
    }
    fn entry(&self, name: &str) -> Option<(usize, usize)> {
        let entry = *self.visible.get(&name.to_string())?.last()?;
        Some((entry, self.entries[entry].1))
    }
    fn get(&self, name: &str) -> Option<usize> {
        self.entry(name).map(|(_, value)| value)
    }
    fn truncate(&mut self, len: usize) {
        while self.entries.len() > len {
            let (name, _) = self.entries.pop().unwrap();
            let empty = {
                let locals = self.visible.get_mut(&name).unwrap();
                locals.pop();
                locals.is_empty()
            };
            if empty {
                self.visible.remove(&name);
            }
        }
    }
}
impl Bindings {
    fn from_locals(locals: &[k::Local], offset: usize) -> Self {
        let mut bindings = Self::default();
        let mut index = 0;
        while index < locals.len() {
            bindings.push((locals[index].name.clone(), offset + index));
            index += 1;
        }
        bindings
    }
}

#[derive(Clone)]
struct Snapshot {
    live: Facts,
    definite_live: Facts,
    written: Facts,
    references: Facts,
    carriers: borrow_cleanup::Dependencies,
    dependents: borrow_cleanup::Dependencies,
}
#[derive(Clone, Copy, Default)]
struct ExpressionContext {
    // Source annotations and declared parameter types permit a reborrow.
    expected: Option<usize>,
    // Solved types constrain shape without changing whether a place moves.
    inferred: Option<usize>,
}
impl ExpressionContext {
    fn ty(self) -> Option<usize> {
        self.expected.or(self.inferred)
    }
}
struct LoopScope {
    synthetic: bool,
    start: usize,
    range: Option<(usize, usize, bool)>,
    result: Option<usize>,
    result_typed: bool,
    result_expected: Option<usize>,
    exits: Vec<Snapshot>,
}
struct Builder<'a, 'b> {
    source_function: usize,
    scope: a::ModuleId,
    file: a::FileId,
    compiler: &'a mut Compiler<'b>,
    function: &'a mut k::Function,
    return_annotated: bool,
    scoped_body: scoped_locals::Body,
    typed_bindings: Vec<crate::check::scoped::Annotation>,
    sub: BTreeMap<String, usize>,
    bindings: Bindings,
    guard_aliases: BTreeMap<String, k::Place>,
    guard_shells: Vec<k::Place>,
    capture_aliases: BTreeMap<a::ExprId, k::Place>,
    capture_locals: std::collections::BTreeSet<usize>,
    active_span: a::Span,
    references: Facts,
    reference_updates: Facts,
    reference_schedules: BTreeMap<(usize, usize), borrow_cleanup::ReferenceSchedule>,
    carriers: borrow_cleanup::Dependencies,
    dependents: borrow_cleanup::Dependencies,
    assignment_types: BTreeMap<usize, usize>,
    range_borrows: BTreeMap<usize, usize>,
    inferred: definition_types::Types,
    live: Facts,
    definite_live: Facts,
    written: Facts,
    loops: Vec<LoopScope>,
    reachable: bool,
}
impl Builder<'_, '_> {
    fn trait_entails(&self, source: a::DeclarationId, target: a::DeclarationId) -> bool {
        let mut pending = vec![source];
        let mut seen = std::collections::BTreeSet::new();
        while let Some(id) = pending.pop() {
            if id == target {
                return true;
            }
            if !seen.insert(id) {
                continue;
            }
            if let a::Item::Trait(index) = self.compiler.source.declarations[id].item {
                for bound in &self.compiler.source.traits[index].bounds {
                    if let a::Type::Path(path) = &self.compiler.source.types[*bound].kind {
                        if let Some(parent) = path.declaration {
                            pending.push(parent);
                        }
                    }
                }
            }
        }
        false
    }
    fn parameter_allows_trait(
        &self,
        params: &[a::GenericParam],
        base: usize,
        declaration: a::DeclarationId,
    ) -> bool {
        for parameter in params {
            if self.sub.get(&parameter.name) != Some(&base) {
                continue;
            }
            for bound in &parameter.bounds {
                if let a::Type::Path(path) = &self.compiler.source.types[*bound].kind {
                    if path
                        .declaration
                        .is_some_and(|id| self.trait_entails(id, declaration))
                    {
                        return true;
                    }
                }
            }
        }
        false
    }
    fn trait_available(&self, implementation: &a::Impl, base: usize, span: a::Span) -> bool {
        let ty = match implementation.trait_ty {
            Some(t) => t,
            None => return true,
        };
        let declaration = match &self.compiler.source.types[ty].kind {
            a::Type::Path(path) => path.declaration,
            _ => None,
        };
        let declaration = match declaration {
            Some(id) => id,
            None => return true,
        };
        if self
            .compiler
            .names
            .trait_in_scope(self.scope, declaration, span)
        {
            return true;
        }
        let function = &self.compiler.source.functions[self.source_function];
        if self.parameter_allows_trait(&function.generics, base, declaration) {
            return true;
        }
        if let a::FunctionOwner::Implementation(owner) =
            self.compiler.source.function_owner(a::FunctionId {
                index: self.source_function,
            })
        {
            let owner = &self.compiler.source.impls[owner.index];
            if self.parameter_allows_trait(&owner.generics, base, declaration) {
                return true;
            }
            if self.sub.get(&"Self".to_string()) == Some(&base) {
                if let Some(trait_ty) = owner.trait_ty {
                    if let a::Type::Path(path) = &self.compiler.source.types[trait_ty].kind {
                        if path
                            .declaration
                            .is_some_and(|id| self.trait_entails(id, declaration))
                        {
                            return true;
                        }
                    }
                }
            }
        }
        false
    }

    fn local(&mut self, ty: usize, name: &str, mutable: bool) -> usize {
        let id = self.function.locals.len();
        self.function.locals.push(k::Local {
            name: name.to_string(),
            ty,
            mutable,
        });
        self.live.push(false);
        self.definite_live.push(false);
        self.written.push(false);
        self.references.push(false);
        self.reference_updates.push(false);
        id
    }
    fn temp(&mut self, ty: usize) -> usize {
        self.local(ty, "$", false)
    }
    fn ty(&self, id: usize) -> usize {
        self.function.locals[id].ty
    }
    fn binding(&self, name: &str) -> Option<usize> {
        self.bindings.get(name)
    }
    fn set_live(&mut self, local: usize, live: bool) {
        self.live.set(local, live);
        self.definite_live.set(local, live);
        let reference = live
            && !self.function.locals[local].name.starts_with('$')
            && !self.capture_locals.contains(&local)
            && (matches!(
                self.compiler.types[self.ty(local)].kind,
                k::TypeKind::Ref { .. }
            ) || !self
                .compiler
                .closure_reference_fields(self.ty(local))
                .is_empty());
        self.references.set(local, reference);
        if reference {
            self.reference_updates.set(local, true);
        }
    }
    fn emit(&mut self, op: k::Op, line: usize) {
        if let k::Op::Assign { place, src } = &op {
            if self.live.at(place.local) && place.local != *src {
                if let Some(t) = place_type(&self.compiler.types, self.function, place) {
                    if !matches!(
                        self.compiler.types[t].kind,
                        k::TypeKind::Unit
                            | k::TypeKind::Bool
                            | k::TypeKind::Int { .. }
                            | k::TypeKind::F64
                    ) {
                        if borrowed_projection(&self.compiler.types, self.function, place) {
                            // Borrowed and indexed storage stays initialized. Move
                            // its old value out through checked replacement, then
                            // destroy that separate owner instead of dropping a
                            // payload through its container's borrowed address.
                            let reference = self.compiler.reference(t, true);
                            let receiver = self.temp(reference);
                            self.emit(
                                k::Op::Borrow {
                                    dst: receiver,
                                    place: place.clone(),
                                    mutable: true,
                                },
                                line,
                            );
                            let old = self.temp(t);
                            self.emit(
                                k::Op::Intrinsic {
                                    dst: old,
                                    name: "replace".to_string(),
                                    args: vec![receiver, *src],
                                },
                                line,
                            );
                            self.emit(
                                k::Op::Drop {
                                    place: k::Place::local(old),
                                },
                                line,
                            );
                            self.written.set(place.local, true);
                            return;
                        }
                        // The root is live, but an owned projected field may
                        // already have moved. Its flag controls destruction at
                        // this assignment; other fields and owners stay live.
                        let destruction =
                            if place.projections.is_empty() && self.definite_live.at(place.local) {
                                k::Op::Drop {
                                    place: place.clone(),
                                }
                            } else {
                                k::Op::DropIf {
                                    place: place.clone(),
                                }
                            };
                        self.emit(destruction, line);
                    }
                }
            }
        }
        self.record_carriers(&op);
        match &op {
            k::Op::Int { dst, .. }
            | k::Op::Text { dst, .. }
            | k::Op::Float { dst, .. }
            | k::Op::Unit { dst }
            | k::Op::Borrow { dst, .. }
            | k::Op::SliceBorrow { dst, .. }
            | k::Op::Discriminant { dst, .. }
            | k::Op::TagTest { dst, .. } => self.set_live(*dst, true),
            k::Op::Read { dst, place, moving } => {
                if *moving && place.projections.is_empty() {
                    self.set_live(place.local, false);
                }
                self.set_live(*dst, true);
            }
            k::Op::ViewRange {
                dst, start, end, ..
            } => {
                self.set_live(*start, false);
                self.set_live(*end, false);
                self.set_live(*dst, true);
            }
            k::Op::Assign { place, src } => {
                self.set_live(*src, false);
                self.set_live(place.local, true);
                self.written.set(place.local, true);
            }
            k::Op::Drop { place } | k::Op::DropIf { place } => {
                if place.projections.is_empty() {
                    self.set_live(place.local, false);
                }
            }
            k::Op::Unary { dst, src, .. } => {
                self.set_live(*src, false);
                self.set_live(*dst, true);
            }
            k::Op::Binary {
                dst, left, right, ..
            } => {
                self.set_live(*left, false);
                self.set_live(*right, false);
                self.set_live(*dst, true);
            }
            k::Op::Call { dst, args, .. } | k::Op::Intrinsic { dst, args, .. } => {
                for arg in args {
                    self.set_live(*arg, false);
                }
                self.set_live(*dst, true);
            }
            k::Op::Struct { dst, fields }
            | k::Op::Variant { dst, fields, .. }
            | k::Op::Array {
                dst,
                elements: fields,
            } => {
                for field in fields {
                    self.set_live(*field, false);
                }
                self.set_live(*dst, true);
            }
            k::Op::If { condition } => self.set_live(*condition, false),
            k::Op::Return { src } => {
                self.set_live(*src, false);
                self.reachable = false;
            }
            k::Op::Break | k::Op::BreakTo { .. } => {
                let depth = match op {
                    k::Op::BreakTo { depth } => depth,
                    _ => 0,
                };
                let state = self.snapshot();
                if depth < self.loops.len() {
                    let index = self.loops.len() - depth - 1;
                    self.loops[index].exits.push(state);
                }
                self.reachable = false;
            }
            k::Op::Continue | k::Op::ContinueTo { .. } | k::Op::Trap { .. } => {
                self.reachable = false
            }
            _ => {}
        }
        self.function.body.push(k::Instruction {
            op,
            line,
            column: if self.active_span.line == line {
                self.active_span.column.max(1)
            } else {
                1
            },
        });
    }
    fn cleanup(&mut self, start: usize, except: Option<usize>, line: usize) {
        let locals = live_indices(&self.live, start);
        for i in locals {
            if Some(i) != except && !self.capture_locals.contains(&i) {
                self.cleanup_place(k::Place::local(i), line);
            }
        }
    }
    fn cleanup_place(&mut self, place: k::Place, line: usize) {
        let op = if place.projections.is_empty() && self.definite_live.at(place.local) {
            k::Op::Drop { place }
        } else {
            k::Op::DropIf { place }
        };
        self.emit(op, line);
    }
    fn unit(&mut self, line: usize) -> (usize, usize) {
        let d = self.temp(k::UNIT);
        self.emit(k::Op::Unit { dst: d }, line);
        (d, k::UNIT)
    }
    fn snapshot(&self) -> Snapshot {
        Snapshot {
            live: self.live.clone(),
            definite_live: self.definite_live.clone(),
            written: self.written.clone(),
            references: self.references.clone(),
            carriers: self.carriers.clone(),
            dependents: self.dependents.clone(),
        }
    }
    fn restore(&mut self, state: &Snapshot) {
        let changed = self.references.symmetric_difference(&state.references);
        self.reference_updates = self.reference_updates.union(&changed);
        self.carriers = state.carriers.clone();
        self.dependents = state.dependents.clone();
        self.live = state.live.clone();
        self.live.resize(self.function.locals.len(), false);
        self.definite_live = state.definite_live.clone();
        self.definite_live.resize(self.function.locals.len(), false);
        self.written = state.written.clone();
        self.written.resize(self.function.locals.len(), false);
        self.references = state.references.clone();
        self.references.resize(self.function.locals.len(), false);
        self.reachable = true;
    }
    fn guard_write(&self, place: &k::Place, span: a::Span) -> Result<()> {
        let protected = self.guard_shells.iter().any(|guarded| {
            place.local == guarded.local
                && place.projections.len() <= guarded.projections.len()
                && place
                    .projections
                    .iter()
                    .zip(&guarded.projections)
                    .all(|(a, b)| {
                        a == b
                            || matches!((a, b), (k::Projection::Index(_), k::Projection::Index(_)))
                    })
        });
        if protected {
            Err(err(
                span,
                "E_BORROW",
                "cannot change a tested scrutinee place in a match guard",
            ))
        } else {
            Ok(())
        }
    }
    fn read(&mut self, place: k::Place, line: usize) -> Result<(usize, usize)> {
        let ty = place_type(&self.compiler.types, self.function, &place).ok_or_else(|| {
            err(
                a::Span {
                    line,
                    ..a::Span::default()
                },
                "E_TYPE",
                "invalid place",
            )
        })?;
        let dst = self.temp(ty);
        let moving = !is_copy(&self.compiler.types, ty);
        self.emit(k::Op::Read { dst, place, moving }, line);
        Ok((dst, ty))
    }
    fn borrow(&mut self, place: k::Place, mutable: bool, line: usize) -> Result<(usize, usize)> {
        if mutable {
            let span = self.active_span;
            self.promote_range_place(&place, span)?;
            self.guard_write(&place, span)?;
        }
        let ty = place_type(&self.compiler.types, self.function, &place).ok_or_else(|| {
            err(
                a::Span {
                    line,
                    ..a::Span::default()
                },
                "E_TYPE",
                "invalid borrowed place",
            )
        })?;
        let rt = self.compiler.reference(ty, mutable);
        let dst = self.temp(rt);
        self.emit(
            k::Op::Borrow {
                dst,
                place,
                mutable,
            },
            line,
        );
        Ok((dst, rt))
    }
    fn hint(&mut self, id: usize) -> Option<usize> {
        if let Some(place) = self.capture_aliases.get(&id) {
            return place_type(&self.compiler.types, self.function, place);
        }
        if let Some(ty) = self.inferred.expressions.get(&id) {
            if !self.compiler.has_view(*ty) {
                return Some(*ty);
            }
        }
        match self.compiler.source.exprs[id].kind.clone() {
            a::Expr::Path(p) => {
                let guarded = match self.guard_aliases.get(&p.name) {
                    Some(place) => place_type(&self.compiler.types, self.function, place),
                    None => None,
                };
                match guarded {
                    Some(ty) => Some(ty),
                    None => match self.binding(&p.name) {
                        Some(local) => Some(self.ty(local)),
                        None => None,
                    },
                }
            }
            a::Expr::Cast { ty, .. } => self.compiler.resolve(ty, &self.sub).ok(),
            a::Expr::Bool(_) => Some(k::BOOL),
            a::Expr::Char(_) => self.compiler.type_names.get(&"char".to_string()).copied(),
            a::Expr::Float(_) => Some(k::F64),
            a::Expr::Int(s) => {
                for (name, t) in &self.compiler.type_names {
                    if s.ends_with(name.as_str()) && s.len() > name.len() {
                        return Some(*t);
                    }
                }
                None
            }
            a::Expr::Unary { op, value } => {
                if op != "*" {
                    return None;
                }
                let ty = self.hint(value)?;
                match self.compiler.types[ty].kind {
                    k::TypeKind::Ref { inner, .. } | k::TypeKind::Box { inner } => Some(inner),
                    _ => None,
                }
            }
            a::Expr::Field { value, name } => {
                let mut t = self.hint(value)?;
                while let k::TypeKind::Ref { inner, .. } | k::TypeKind::Box { inner } =
                    self.compiler.types[t].kind
                {
                    t = inner;
                }
                let index = self.compiler.field_index(t, &name)?;
                match &self.compiler.types[t].kind {
                    k::TypeKind::Struct { fields, .. } => Some(fields[index].ty),
                    _ => None,
                }
            }
            a::Expr::Index { value, index } => {
                let mut t = self.hint(value)?;
                while let k::TypeKind::Ref { inner, .. } = self.compiler.types[t].kind {
                    t = inner;
                }
                if self.is_range(index) {
                    let span = self.compiler.source.exprs[id].span;
                    return self.range_referent(t, span).ok();
                }
                match self.compiler.types[t].kind {
                    k::TypeKind::Vec { inner }
                    | k::TypeKind::Slice { inner, .. }
                    | k::TypeKind::Array { inner, .. } => Some(inner),
                    _ => None,
                }
            }
            _ => None,
        }
    }
    fn place(&mut self, id: usize) -> Result<k::Place> {
        self.place_with_mutability(id, false)
    }
    fn place_with_mutability(&mut self, id: usize, mutable: bool) -> Result<k::Place> {
        if let Some(place) = self.capture_aliases.get(&id) {
            return Ok(place.clone());
        }
        let node = self.compiler.source.exprs[id].clone();
        match node.kind {
            a::Expr::Path(path) => {
                if let Some(place) = self.guard_aliases.get(&path.name) {
                    Ok(place.clone())
                } else if let Some(local) = self.binding(&path.name) {
                    Ok(k::Place::local(local))
                } else {
                    Err(err(
                        node.span,
                        "E_TYPE",
                        &format!("unknown value {}", path.name),
                    ))
                }
            }
            a::Expr::Unary { op, value } => {
                if op != "*" {
                    return Err(err(node.span, "E_TYPE", "expression is not a place"));
                }
                let mut place = self.addressable_with_mutability(value, mutable)?;
                place.projections.push(k::Projection::Deref);
                Ok(place)
            }
            a::Expr::Field { value, name } => {
                let mut place = self.addressable_with_mutability(value, mutable)?;
                let mut ty = match place_type(&self.compiler.types, self.function, &place) {
                    Some(ty) => ty,
                    None => return Err(err(node.span, "E_TYPE", "invalid field base")),
                };
                while let k::TypeKind::Ref { inner, .. } | k::TypeKind::Box { inner } =
                    self.compiler.types[ty].kind
                {
                    place.projections.push(k::Projection::Deref);
                    ty = inner;
                }
                if matches!(&self.compiler.types[ty].kind, k::TypeKind::Struct { .. }) {
                    let index = match self.compiler.field_index(ty, &name) {
                        Some(index) => index,
                        None => return Err(err(node.span, "E_TYPE", "unknown field")),
                    };
                    self.compiler
                        .field_access(ty, &name, self.scope, node.span)?;
                    place.projections.push(k::Projection::Field(index));
                    Ok(place)
                } else {
                    Err(err(node.span, "E_TYPE", "field access requires a struct"))
                }
            }
            a::Expr::Index { value, index } => {
                if self.is_range(index) {
                    return self.range_place(value, index, mutable, node.span);
                }
                let mut place = self.addressable_with_mutability(value, mutable)?;
                let mut ty = match place_type(&self.compiler.types, self.function, &place) {
                    Some(ty) => ty,
                    None => return Err(err(node.span, "E_TYPE", "invalid index base")),
                };
                while let k::TypeKind::Ref { inner, .. } = self.compiler.types[ty].kind {
                    place.projections.push(k::Projection::Deref);
                    ty = inner;
                }
                let (index, _) = self.expr(index, Some(k::U64))?;
                place.projections.push(k::Projection::Index(index));
                Ok(place)
            }
            _ => Err(err(node.span, "E_TYPE", "expression is not a place")),
        }
    }
    fn addressable(&mut self, id: usize) -> Result<k::Place> {
        self.addressable_with_mutability(id, false)
    }
    fn addressable_with_mutability(&mut self, id: usize, mutable: bool) -> Result<k::Place> {
        let place = match &self.compiler.source.exprs[id].kind {
            a::Expr::Path(path) => {
                self.capture_aliases.contains_key(&id)
                    || self.binding(&path.name).is_some()
                    || self.guard_aliases.contains_key(&path.name)
            }
            a::Expr::Field { .. } | a::Expr::Index { .. } => true,
            a::Expr::Unary { op, .. } => op == "*",
            _ => false,
        };
        if place {
            self.place_with_mutability(id, mutable)
        } else {
            let v = self.expr(id, None)?;
            // Value expressions create writable temporary storage. This
            // does not change a named binding or permission through a
            // reference. Range borrows still follow the requested mode.
            self.function.locals[v.0].mutable = true;
            Ok(k::Place::local(v.0))
        }
    }
    fn end_unused_borrows(&mut self, start: usize, position: usize, end: usize, line: usize) {
        // Each named reference is scheduled at its last lexical use in this
        // block. Changed bindings and restored branch states invalidate only
        // their own entry; a statement never rescans all live references.
        let key = (start, end);
        let mut schedule = match self.reference_schedules.remove(&key) {
            Some(schedule) => schedule,
            None => borrow_cleanup::ReferenceSchedule::default(),
        };
        let updated = live_indices(&self.reference_updates, start);
        for local in updated {
            self.reference_updates.set(local, false);
            if !self.references.at(local) {
                continue;
            }
            let name = &self.function.locals[local].name;
            let last = match self.compiler.uses.get(&(self.file, name.clone())) {
                Some(positions) => {
                    let mut at = 0;
                    let mut limit = positions.len();
                    while at < limit {
                        let middle = at + (limit - at) / 2;
                        if positions[middle] < end {
                            at = middle + 1;
                        } else {
                            limit = middle;
                        }
                    }
                    if at == 0 {
                        None
                    } else {
                        Some(positions[at - 1])
                    }
                }
                None => None,
            };
            let deadline = last.unwrap_or(0);
            if !schedule.deadlines.contains_key(&deadline) {
                schedule.deadlines.insert(deadline, BTreeSet::new());
            }
            schedule.deadlines.get_mut(&deadline).unwrap().insert(local);
        }
        while schedule
            .deadlines
            .first_key_value()
            .is_some_and(|(last, _)| *last < position)
        {
            let (_, locals) = schedule.deadlines.pop_first().unwrap();
            for local in locals {
                schedule.pending.insert(local);
            }
        }
        let mut pending = BTreeSet::new();
        for local in schedule.pending {
            if self.references.at(local) {
                pending.insert(local);
            }
        }
        schedule.pending = pending;
        self.drop_unused_references(schedule.pending.clone(), line);
        let mut pending = BTreeSet::new();
        for local in schedule.pending {
            if self.references.at(local) {
                pending.insert(local);
            }
        }
        schedule.pending = pending;
        self.reference_schedules.insert(key, schedule);
    }
    fn bind_name(
        &mut self,
        name: String,
        mutable: bool,
        value: (usize, usize),
        line: usize,
    ) -> usize {
        let dst = self.local(value.1, &name, mutable);
        self.emit(
            k::Op::Assign {
                place: k::Place::local(dst),
                src: value.0,
            },
            line,
        );
        self.bindings.push((name, dst));
        dst
    }

    fn bind(&mut self, pattern: usize, value: (usize, usize), line: usize) -> Result<()> {
        let p = self.compiler.source.patterns[pattern].clone();
        match p.kind {
            a::Pattern::Bind {
                name,
                mutable,
                by_ref,
                ref_mut,
            } => {
                let val = if by_ref {
                    self.borrow(k::Place::local(value.0), ref_mut, line)?
                } else {
                    value
                };
                let destination = self.bind_name(name, mutable, val, line);
                self.annotation_binding(&k::Place::local(value.0), destination, by_ref);
                Ok(())
            }
            a::Pattern::Wild => {
                self.emit(
                    k::Op::Drop {
                        place: k::Place::local(value.0),
                    },
                    line,
                );
                Ok(())
            }
            a::Pattern::Tuple(items) => {
                for (i, item) in items.iter().enumerate() {
                    let v = self.read(
                        k::Place {
                            local: value.0,
                            projections: vec![k::Projection::Field(i)],
                        },
                        line,
                    )?;
                    self.bind(*item, v, line)?;
                }
                Ok(())
            }
            _ => Err(err(
                p.span,
                "E_UNSUPPORTED",
                "this binding pattern is not implemented yet",
            )),
        }
    }
    fn extends_temporary(&self, value: usize, pattern: usize) -> bool {
        let mut patterns = vec![pattern];
        while let Some(pattern) = patterns.pop() {
            match &self.compiler.source.patterns[pattern].kind {
                a::Pattern::Bind { by_ref: true, .. } => return true,
                a::Pattern::Tuple(fields)
                | a::Pattern::Variant { fields, .. }
                | a::Pattern::Or(fields) => patterns.extend_from_slice(fields),
                a::Pattern::Struct { fields, .. } => {
                    for (_, id) in fields {
                        patterns.push(*id);
                    }
                }
                a::Pattern::Ref { inner, .. } => patterns.push(*inner),
                _ => {}
            }
        }
        let mut todo = vec![value];
        while let Some(id) = todo.pop() {
            match &self.compiler.source.exprs[id].kind {
                a::Expr::Borrow { .. } => return true,
                a::Expr::Match { arms, .. }
                    if arms.iter().any(|arm| {
                        matches!(
                            self.compiler.source.exprs[arm.value].kind,
                            a::Expr::Diverge(_)
                        )
                    }) =>
                {
                    return true;
                }
                a::Expr::Tuple(items) | a::Expr::Array(items) => todo.extend_from_slice(items),
                a::Expr::Struct { fields, .. } => {
                    for (_, value) in fields {
                        todo.push(*value);
                    }
                }
                _ => {}
            }
        }
        false
    }
    fn cleanup_temporaries(&mut self, start: usize, line: usize) {
        let locals = live_indices(&self.live, start);
        for local in locals {
            let temporary = {
                let name = &self.function.locals[local].name;
                name.starts_with('$') && self.binding(name) != Some(local)
            };
            if temporary {
                self.cleanup_place(k::Place::local(local), line);
            }
        }
    }
    fn block(&mut self, id: usize, expected: Option<usize>) -> Result<(usize, usize)> {
        self.block_with_context(
            id,
            ExpressionContext {
                expected,
                inferred: None,
            },
        )
    }
    fn block_with_context(
        &mut self,
        id: usize,
        context: ExpressionContext,
    ) -> Result<(usize, usize)> {
        let expected = context.expected;
        let block = self.compiler.source.blocks[id].clone();
        let start = self.live.len();
        let bindings = self.bindings.len();
        let mut checking_dead = false;
        for stmt in block.statements {
            if !self.reachable {
                checking_dead = true;
                self.reachable = true;
                for local in 0..self.live.len() {
                    self.set_live(local, true);
                }
            }
            let end = match &stmt {
                a::Stmt::Let { span, .. } => span.end,
                a::Stmt::Expr { value, .. } => self.compiler.source.exprs[*value].span.end,
                _ => 0,
            };
            match stmt {
                a::Stmt::Let {
                    pattern,
                    ty,
                    value,
                    span,
                } => {
                    let expected = match ty {
                        Some(t) => Some(self.compiler.resolve(t, &self.sub)?),
                        None => None,
                    };
                    if let Some(value) = value {
                        let before = self.live.len();
                        let extends = self.extends_temporary(value, pattern);
                        self.pattern_initializer(pattern, value, expected, ty, span.line)?;
                        if !extends {
                            self.cleanup_temporaries(before, span.line);
                        }
                    } else {
                        let p = self.compiler.source.patterns[pattern].clone();
                        if let a::Pattern::Bind { name, mutable, .. } = p.kind {
                            let t = match expected {
                                Some(ty) => ty,
                                None => {
                                    return Err(err(
                                        span,
                                        "E_TYPE",
                                        "uninitialized binding requires a type",
                                    ));
                                }
                            };
                            let local = self.local(t, &name, mutable);
                            self.bindings.push((name, local));
                            self.annotation_uninitialized(ty.unwrap(), local)?;
                        } else {
                            return Err(err(span, "E_UNSUPPORTED", "uninitialized destructuring"));
                        }
                    }
                }
                a::Stmt::Expr { value, semicolon } => {
                    let before = self.live.len();
                    let result = self.expr(value, None)?;
                    if !semicolon && self.reachable && result.1 != k::UNIT {
                        return Err(err(
                            self.compiler.source.exprs[value].span,
                            "E_TYPE",
                            "a block statement without `;` must have unit type",
                        ));
                    }
                    self.cleanup(before, None, block.span.line);
                }
                a::Stmt::Item(a::Item::Use(_)) => {}
                a::Stmt::Item(_) => {
                    return Err(err(
                        block.span,
                        "E_UNSUPPORTED",
                        "local declarations are not implemented yet",
                    ));
                }
            }
            if checking_dead {
                self.reachable = false;
            }
            if self.reachable {
                self.end_unused_borrows(start, end, block.span.end, block.span.line);
            }
        }
        if !self.reachable {
            if let Some(tail) = block.tail {
                self.reachable = true;
                for local in 0..self.live.len() {
                    self.set_live(local, true);
                }
                let value = self.expr_with_inference(tail, expected, context.inferred)?;
                if let Some(ty) = expected {
                    if value.1 != ty {
                        return Err(err(
                            block.span,
                            "E_TYPE",
                            "unreachable tail expression type differs",
                        ));
                    }
                }
                self.reachable = false;
            }
        }
        let result = if self.reachable {
            if let Some(tail) = block.tail {
                self.expr_with_inference(tail, expected, context.inferred)?
            } else {
                self.unit(block.span.line)
            }
        } else {
            self.unit(block.span.line)
        };
        self.cleanup(start, Some(result.0), block.span.line);
        self.bindings.truncate(bindings);
        Ok(result)
    }
    // Inferred closure results constrain types without creating a reborrow site.
    // A declared return annotation keeps its ordinary coercion behavior.
    fn return_value(&mut self, id: usize) -> Result<(usize, usize)> {
        let result = self.function.result;
        let expected = if self.return_annotated {
            Some(result)
        } else {
            None
        };
        self.expr_with_inference(id, expected, Some(result))
    }
    fn expr(&mut self, id: usize, expected: Option<usize>) -> Result<(usize, usize)> {
        self.expr_with_inference(id, expected, None)
    }
    fn expr_with_inference(
        &mut self,
        id: usize,
        expected: Option<usize>,
        inferred: Option<usize>,
    ) -> Result<(usize, usize)> {
        let previous = self.active_span;
        self.active_span = self.compiler.source.exprs[id].span;
        let result = self.expr_with_inference_inner(id, expected, inferred);
        self.active_span = previous;
        result
    }
    fn expr_with_inference_inner(
        &mut self,
        id: usize,
        expected: Option<usize>,
        inferred: Option<usize>,
    ) -> Result<(usize, usize)> {
        // A solved type is not by itself a coercion site. In particular a bare
        // mutable reference binding moves its value; it must not acquire the
        // automatic reborrow used at an explicitly typed argument or binding.
        let inferred = inferred.or_else(|| self.inferred.expressions.get(&id).copied());
        let reference_context = expected.is_some_and(|target| {
            matches!(self.compiler.types[target].kind, k::TypeKind::Ref { .. })
        });
        let place_expression = match &self.compiler.source.exprs[id].kind {
            a::Expr::Path(path) => {
                self.capture_aliases.contains_key(&id)
                    || self.binding(&path.name).is_some()
                    || self.guard_aliases.contains_key(&path.name)
            }
            a::Expr::Field { .. } | a::Expr::Index { .. } => true,
            a::Expr::Unary { op, .. } => op == "*",
            _ => false,
        };
        let value = if reference_context && place_expression {
            // A declared reference context reborrows a mutable reference held
            // in any checked place. Resolve the projection only once: an index
            // or the base of a field can have observable effects.
            let mut place = self.place(id)?;
            let span = self.compiler.source.exprs[id].span;
            let source = place_type(&self.compiler.types, self.function, &place)
                .ok_or_else(|| err(span, "E_TYPE", "invalid reference coercion place"))?;
            let line = span.line;
            if matches!(
                self.compiler.types[source].kind,
                k::TypeKind::Ref { mutable: true, .. }
            ) {
                place.projections.push(k::Projection::Deref);
                let mutable = expected.is_some_and(|target| {
                    matches!(
                        self.compiler.types[target].kind,
                        k::TypeKind::Ref { mutable: true, .. }
                    )
                });
                self.borrow(place, mutable, line)?
            } else {
                self.read(place, line)?
            }
        } else {
            self.expr_inner(id, ExpressionContext { expected, inferred })?
        };
        // A solved target may require a conversion after evaluation. It does
        // not create the place reborrow above: an inferred &mut value moves.
        let span = self.compiler.source.exprs[id].span;
        self.coerce_value(value, expected.or(inferred), span)
    }
    // Apply a declared reference conversion after evaluating its operand once.
    // This does not reborrow a source binding that has already been moved.
    fn coerce_value(
        &mut self,
        value: (usize, usize),
        expected: Option<usize>,
        span: a::Span,
    ) -> Result<(usize, usize)> {
        if let Some(target) = expected {
            if target != value.1 {
                if let (
                    k::TypeKind::Ref {
                        inner: want,
                        mutable: want_mut,
                    },
                    k::TypeKind::Ref {
                        inner: have,
                        mutable: have_mut,
                    },
                ) = (
                    self.compiler.types[target].kind.clone(),
                    self.compiler.types[value.1].kind.clone(),
                ) {
                    if !want_mut || have_mut {
                        if !want_mut
                            && matches!(self.compiler.types[want].kind, k::TypeKind::Str)
                            && self.library_string(have, span)
                        {
                            let argument = if have_mut {
                                self.borrow(
                                    k::Place {
                                        local: value.0,
                                        projections: vec![k::Projection::Deref],
                                    },
                                    false,
                                    span.line,
                                )?
                                .0
                            } else {
                                value.0
                            };
                            let result = self.source_library_call(
                                "string",
                                "__lkj_as_str",
                                BTreeMap::new(),
                                vec![argument],
                                span,
                            )?;
                            if have_mut {
                                self.emit(
                                    k::Op::Drop {
                                        place: k::Place::local(value.0),
                                    },
                                    span.line,
                                );
                            }
                            return Ok(result);
                        }
                        let mut inner = have;
                        let mut place = k::Place {
                            local: value.0,
                            projections: vec![k::Projection::Deref],
                        };
                        if let k::TypeKind::Slice { inner: element, .. } =
                            self.compiler.types[want].kind
                        {
                            if matches!(self.compiler.types[inner].kind,k::TypeKind::Vec{inner:t}|k::TypeKind::Array{inner:t,..} if t==element)
                            {
                                let dst = self.temp(target);
                                self.emit(
                                    k::Op::SliceBorrow {
                                        dst,
                                        place,
                                        mutable: want_mut,
                                    },
                                    span.line,
                                );
                                self.emit(
                                    k::Op::Drop {
                                        place: k::Place::local(value.0),
                                    },
                                    span.line,
                                );
                                return Ok((dst, target));
                            }
                        }
                        while inner != want {
                            if let k::TypeKind::Box { inner: next } =
                                self.compiler.types[inner].kind
                            {
                                inner = next;
                                place.projections.push(k::Projection::Deref);
                            } else {
                                break;
                            }
                        }
                        if inner == want {
                            let result = self.borrow(place, want_mut, span.line)?;
                            self.emit(
                                k::Op::Drop {
                                    place: k::Place::local(value.0),
                                },
                                span.line,
                            );
                            return Ok(result);
                        }
                    }
                }
            }
        }
        Ok(value)
    }
    fn integer(
        &mut self,
        text: &str,
        expected: Option<usize>,
        negative: bool,
        span: a::Span,
    ) -> Result<(usize, usize)> {
        let (value, ty) = integers::literal(
            &self.compiler.types,
            &self.compiler.type_names,
            text,
            expected,
            negative,
            span,
        )?;
        let dst = self.temp(ty);
        self.emit(k::Op::Int { dst, value }, span.line);
        Ok((dst, ty))
    }
    fn expr_inner(&mut self, id: usize, context: ExpressionContext) -> Result<(usize, usize)> {
        let expected = context
            .expected
            .or(context.inferred.filter(|ty| !self.compiler.has_view(*ty)));
        let node = self.compiler.source.exprs[id].clone();
        match &node.kind {
            a::Expr::Path(path) | a::Expr::Struct { path, .. } => {
                self.explicit_type_arguments(&path.args, node.span)?
            }
            a::Expr::Method { types, .. } => self.explicit_type_arguments(types, node.span)?,
            a::Expr::Call { callee, .. } => {
                if let a::Expr::Path(path) = &self.compiler.source.exprs[*callee].kind {
                    self.explicit_type_arguments(&path.args, node.span)?;
                }
            }
            _ => {}
        }
        let capture: Option<k::Place> = match self.capture_aliases.get(&id) {
            Some(place) => Some((*place).clone()),
            None => None,
        };
        if let Some(place) = capture {
            return self.read(place, node.span.line);
        }
        let line = node.span.line;
        match node.kind {
            a::Expr::Int(text) => self.integer(&text, expected, false, node.span),
            a::Expr::Char(value) => {
                let ty = *self.compiler.type_names.get(&"char".to_string()).unwrap();
                let dst = self.temp(ty);
                self.emit(
                    k::Op::Int {
                        dst,
                        value: value as i64,
                    },
                    line,
                );
                Ok((dst, ty))
            }
            a::Expr::String(text) => {
                let inner = self.compiler.intern(String::new(), k::TypeKind::Str);
                let ty = self.compiler.reference(inner, false);
                let dst = self.temp(ty);
                self.emit(
                    k::Op::Text {
                        dst,
                        bytes: text.into_bytes(),
                    },
                    line,
                );
                Ok((dst, ty))
            }
            a::Expr::Bytes(bytes) => {
                let inner = self.compiler.intern(
                    String::new(),
                    k::TypeKind::Array {
                        inner: k::U8,
                        len: bytes.len(),
                    },
                );
                let ty = self.compiler.reference(inner, false);
                let dst = self.temp(ty);
                self.emit(k::Op::Text { dst, bytes }, line);
                Ok((dst, ty))
            }
            a::Expr::Bool(value) => {
                let d = self.temp(k::BOOL);
                self.emit(
                    k::Op::Int {
                        dst: d,
                        value: if value { 1i64 } else { 0i64 },
                    },
                    line,
                );
                Ok((d, k::BOOL))
            }
            a::Expr::Float(text) => {
                let mut end = text.len();
                while text[..end].ends_with("f64") {
                    end -= 3;
                }
                let normalized = text[..end].replace('_', "");
                let bits = match floats::decimal_bits(&normalized) {
                    floats::Parsed::Finite(bits) => bits,
                    floats::Parsed::Invalid => {
                        return Err(err(node.span, "E_TYPE", "invalid float literal"));
                    }
                    floats::Parsed::Overflow => {
                        return Err(err(
                            node.span,
                            "E_TYPE",
                            "floating-point literal exceeds f64",
                        ));
                    }
                };
                let dst = self.temp(k::F64);
                self.emit(k::Op::Float { dst, bits }, line);
                Ok((dst, k::F64))
            }
            a::Expr::Path(path) => {
                if self.binding(&path.name).is_some() || self.guard_aliases.contains_key(&path.name)
                {
                    let place = self.place(id)?;
                    return self.read(place, line);
                }
                let path = self
                    .compiler
                    .names
                    .value_path(node.scope, &path, node.span)?;
                if let Some(a::Item::Constant(index)) = self.compiler.declaration_item(&path) {
                    if self.compiler.diagnostic_functions.is_some() {
                        return Err(err(
                            node.span,
                            "E_BLOCKED",
                            "constant initializer requires its complete dependency checks",
                        ));
                    }
                    let constant = self.compiler.source.constants[index].clone();
                    let t = self.compiler.resolve(constant.ty, &self.sub)?;
                    return self.expr(constant.value, Some(t));
                }
                let empty_fields: Vec<(String, usize)> = Vec::new();
                if let Some(value) = self.construct_variant(
                    &path,
                    &empty_fields,
                    a::VariantStyle::Unit,
                    context,
                    node.span,
                )? {
                    return Ok(value);
                }
                if let Some(a::Item::Struct(index)) = self.compiler.declaration_item(&path) {
                    if self.compiler.source.structs[index].unit {
                        return self.construct_struct(
                            &path,
                            &empty_fields,
                            None,
                            context,
                            node.span,
                        );
                    }
                }
                Err(err(
                    node.span,
                    "E_TYPE",
                    &format!("unknown value {}", path.name),
                ))
            }
            a::Expr::Field { .. } | a::Expr::Index { .. } => {
                let place = self.place(id)?;
                self.read(place, line)
            }
            a::Expr::Borrow { mutable, value } => {
                let is_place = match &self.compiler.source.exprs[value].kind {
                    a::Expr::Path(path) => {
                        self.capture_aliases.contains_key(&value)
                            || self.binding(&path.name).is_some()
                            || self.guard_aliases.contains_key(&path.name)
                    }
                    a::Expr::Field { .. } | a::Expr::Index { .. } => true,
                    a::Expr::Unary { op, .. } => op == "*",
                    _ => false,
                };
                let place = if is_place {
                    self.place_with_mutability(value, mutable)?
                } else {
                    let inner = expected.and_then(|t| match self.compiler.types[t].kind {
                        k::TypeKind::Ref { inner, .. } => Some(inner),
                        _ => None,
                    });
                    let v = self.expr(value, inner)?;
                    if mutable {
                        self.function.locals[v.0].mutable = true;
                    }
                    k::Place::local(v.0)
                };
                self.borrow(place, mutable, line)
            }
            a::Expr::Unary { op, value } => {
                if op == "-" {
                    if let a::Expr::Int(text) = self.compiler.source.exprs[value].kind.clone() {
                        return self.integer(&text, expected, true, node.span);
                    }
                }
                if op == "*" {
                    let place = self.place(id)?;
                    return self.read(place, line);
                }
                let (src, ty) = self.primitive_operand(value, expected, node.span)?;
                let dst = self.temp(ty);
                self.emit(k::Op::Unary { dst, op, src }, line);
                Ok((dst, ty))
            }
            a::Expr::Binary { op, left, right } => {
                if op == "<<" || op == ">>" {
                    return self.shift_expr(&op, left, right, expected, node.span);
                }
                if op == "&&" || op == "||" {
                    return self.short_circuit(&op, left, right, line);
                }
                if op == "==" || op == "!=" {
                    return self.equality_expr(&op, left, right, node.span);
                }
                if matches!(op.as_str(), "<" | "<=" | ">" | ">=") {
                    return self.ordering_expr(&op, left, right, node.span);
                }
                let operand = self
                    .hint(left)
                    .or_else(|| self.hint(right))
                    .or(expected)
                    .map(|ty| self.primitive_value_type(ty));
                let (left, ty) = self.primitive_operand(left, operand, node.span)?;
                let (right, rt) = self.primitive_operand(right, Some(ty), node.span)?;
                if ty != rt {
                    return Err(err(
                        node.span,
                        "E_TYPE",
                        &format!(
                            "binary operand types differ: {} and {}",
                            self.compiler.display_type(ty),
                            self.compiler.display_type(rt)
                        ),
                    ));
                }
                let dst = self.temp(ty);
                self.emit(
                    k::Op::Binary {
                        dst,
                        op,
                        left,
                        right,
                    },
                    line,
                );
                Ok((dst, ty))
            }
            a::Expr::Assign { op, place, value } => {
                self.assignment_expr(&op, place, value, node.span)
            }
            a::Expr::Cast { value, ty } => {
                let t = self.compiler.resolve(ty, &self.sub)?;
                let (src, _) = self.expr(value, None)?;
                let dst = self.temp(t);
                self.emit(
                    k::Op::Intrinsic {
                        dst,
                        name: "cast".to_string(),
                        args: vec![src],
                    },
                    line,
                );
                Ok((dst, t))
            }
            a::Expr::Call { callee, args } => self.call(callee, &args, context, node.span),
            a::Expr::Method {
                receiver,
                name,
                types,
                args,
            } => self.method(id, receiver, &name, &types, &args, node.span),
            a::Expr::Struct { path, fields, base } => {
                self.construct_struct(&path, &fields, base, context, node.span)
            }
            a::Expr::Array(elements) => self.array_expr(&elements, expected, node.span),
            a::Expr::Repeat { value, count } => {
                let length = match &self.compiler.source.exprs[count].kind {
                    a::Expr::Int(text) => {
                        let digits = text.replace('_', "");
                        match digits.as_str().parse::<usize>().ok() {
                            Some(length) => length,
                            None => {
                                return Err(err(
                                    node.span,
                                    "E_TYPE",
                                    "array repeat length must be an integer literal",
                                ));
                            }
                        }
                    }
                    _ => {
                        return Err(err(
                            node.span,
                            "E_UNSUPPORTED",
                            "array repeat length must be constant",
                        ));
                    }
                };
                let inner = expected.and_then(|t| match self.compiler.types[t].kind {
                    k::TypeKind::Array { inner, .. } => Some(inner),
                    _ => None,
                });
                let v = self.expr(value, inner)?;
                if length > 1 && !is_copy(&self.compiler.types, v.1) {
                    return Err(err(
                        node.span,
                        "E_TYPE",
                        "repeated array element must be Copy",
                    ));
                }
                let mut elements = Vec::new();
                if length <= 1 {
                    for _ in 0..length {
                        elements.push(self.read(k::Place::local(v.0), line)?.0);
                    }
                }
                let ty = self.compiler.intern(
                    String::new(),
                    k::TypeKind::Array {
                        inner: v.1,
                        len: length,
                    },
                );
                let dst = self.temp(ty);
                if length > 1 {
                    self.emit(
                        k::Op::Intrinsic {
                            dst,
                            name: "array_repeat".to_string(),
                            args: vec![v.0],
                        },
                        line,
                    );
                } else {
                    self.emit(k::Op::Array { dst, elements }, line);
                }
                if self.live.at(v.0) {
                    self.emit(
                        k::Op::Drop {
                            place: k::Place::local(v.0),
                        },
                        line,
                    );
                }
                Ok((dst, ty))
            }
            a::Expr::Tuple(items) => {
                if items.is_empty() {
                    return Ok(self.unit(line));
                }
                let mut fields = Vec::new();
                let mut values = Vec::new();
                for (i, item) in items.iter().enumerate() {
                    let field_type = |ty: usize| match &self.compiler.types[ty].kind {
                        k::TypeKind::Struct { fields, .. } => fields.get(i).map(|field| field.ty),
                        _ => None,
                    };
                    let declared = context.expected.and_then(field_type);
                    let inferred = context.inferred.and_then(field_type);
                    let v = self.expr_with_inference(*item, declared, inferred)?;
                    fields.push(k::Field {
                        name: format!("{i}"),
                        ty: v.1,
                    });
                    values.push(v.0);
                }
                let copy = fields.iter().all(|f| is_copy(&self.compiler.types, f.ty));
                let ty = self
                    .compiler
                    .intern(String::new(), k::TypeKind::Struct { fields, copy });
                let dst = self.temp(ty);
                self.emit(
                    k::Op::Struct {
                        dst,
                        fields: values,
                    },
                    line,
                );
                Ok((dst, ty))
            }
            a::Expr::Block(block) => self.block_with_context(block, context),
            a::Expr::If {
                condition,
                then_block,
                otherwise,
            } => self.if_expr(condition, then_block, otherwise, context, line),
            a::Expr::While { condition, body } => {
                self.while_expr(Some(condition), body, ExpressionContext::default(), line)
            }
            a::Expr::Loop { body } => self.while_expr(None, body, context, line),
            a::Expr::For {
                pattern,
                iterator,
                body,
            } => self.for_expr(pattern, iterator, body, line),
            a::Expr::Match { value, arms } => self.match_expr(value, &arms, context, node.span),
            a::Expr::Return(value) => {
                let result = if let Some(v) = value {
                    self.return_value(v)?
                } else {
                    self.unit(line)
                };
                self.cleanup(0, Some(result.0), line);
                self.emit(k::Op::Return { src: result.0 }, line);
                Ok(result)
            }
            a::Expr::Break(value) => {
                let index = match self.last_loop() {
                    Some(index) => index,
                    None => return Err(err(node.span, "E_TYPE", "break outside loop")),
                };
                let scope = &self.loops[index];
                let (start, result, typed, declared) = (
                    scope.start,
                    scope.result,
                    scope.result_typed,
                    scope.result_expected,
                );
                if let Some(result) = result {
                    let value = if let Some(value) = value {
                        let inferred = if typed { Some(self.ty(result)) } else { None };
                        self.expr_with_inference(value, declared, inferred)?
                    } else {
                        self.unit(line)
                    };
                    if typed && self.ty(result) != value.1 && self.reachable {
                        return Err(err(node.span, "E_TYPE", "break result type differs"));
                    }
                    if !typed {
                        self.function.locals[result].ty = value.1;
                        self.loops[index].result_typed = true;
                    }
                    self.emit(
                        k::Op::Assign {
                            place: k::Place::local(result),
                            src: value.0,
                        },
                        line,
                    );
                } else if value.is_some() {
                    return Err(err(node.span, "E_TYPE", "only loop accepts a break value"));
                }
                self.cleanup(start, result, line);
                self.loop_transfer(index, false, line);
                Ok(self.unit(line))
            }
            a::Expr::Continue => {
                let index = match self.last_loop() {
                    Some(index) => index,
                    None => return Err(err(node.span, "E_TYPE", "continue outside loop")),
                };
                let start = self.loops[index].start;
                self.advance_range(line)?;
                self.cleanup(start, None, line);
                self.loop_transfer(index, true, line);
                Ok(self.unit(line))
            }
            a::Expr::Try(value) => self.try_expr(value, node.span),
            a::Expr::Diverge(value) => {
                let result = self.expr(value, None)?;
                if self.reachable {
                    return Err(err(
                        node.span,
                        "E_TYPE",
                        "let-else alternative must diverge",
                    ));
                }
                Ok(result)
            }
            a::Expr::Macro {
                name,
                args,
                separator,
            } => self.macro_expr(&name, &args, &separator, expected, node.span),
            a::Expr::Closure { .. } => self.closure_expression(id, node.span),
            _ => Err(err(
                node.span,
                "E_UNSUPPORTED",
                "this expression form is not implemented yet",
            )),
        }
    }
    fn array_expr(
        &mut self,
        elements: &[usize],
        expected: Option<usize>,
        span: a::Span,
    ) -> Result<(usize, usize)> {
        let mut inner = expected.and_then(|t| match self.compiler.types[t].kind {
            k::TypeKind::Array { inner, .. } => Some(inner),
            _ => None,
        });
        let mut values = Vec::new();
        for element in elements {
            let value = self.expr(*element, inner)?;
            if inner.is_none() {
                inner = Some(value.1);
            }
            values.push(value.0);
        }
        let inner =
            inner.ok_or_else(|| err(span, "E_TYPE", "empty array needs an element type"))?;
        self.compiler.check_owned_element(inner, span)?;
        let ty = self.compiler.intern(
            String::new(),
            k::TypeKind::Array {
                inner,
                len: elements.len(),
            },
        );
        let dst = self.temp(ty);
        self.emit(
            k::Op::Array {
                dst,
                elements: values,
            },
            span.line,
        );
        Ok((dst, ty))
    }
    fn short_circuit(
        &mut self,
        op: &str,
        left: usize,
        right: usize,
        line: usize,
    ) -> Result<(usize, usize)> {
        let (condition, _) = self.expr(left, Some(k::BOOL))?;
        let result = self.local(k::BOOL, "$if", true);
        self.emit(k::Op::If { condition }, line);
        let state = self.snapshot();
        let first = if op == "&&" {
            self.expr(right, Some(k::BOOL))?
        } else {
            let d = self.temp(k::BOOL);
            self.emit(k::Op::Int { dst: d, value: 1 }, line);
            (d, k::BOOL)
        };
        self.emit(
            k::Op::Assign {
                place: k::Place::local(result),
                src: first.0,
            },
            line,
        );
        self.cleanup(state.live.len(), Some(result), line);
        let first_state = self.snapshot();
        self.emit(k::Op::Else, line);
        self.restore(&state);
        let second = if op == "||" {
            self.expr(right, Some(k::BOOL))?
        } else {
            let d = self.temp(k::BOOL);
            self.emit(k::Op::Int { dst: d, value: 0 }, line);
            (d, k::BOOL)
        };
        self.emit(
            k::Op::Assign {
                place: k::Place::local(result),
                src: second.0,
            },
            line,
        );
        self.cleanup(state.live.len(), Some(result), line);
        self.emit(k::Op::End, line);
        self.merge_carriers(&first_state);
        Ok((result, k::BOOL))
    }
    fn if_expr(
        &mut self,
        condition: usize,
        then_block: usize,
        otherwise: Option<usize>,
        context: ExpressionContext,
        line: usize,
    ) -> Result<(usize, usize)> {
        let expected = context.ty();
        let (condition, _) = self.expr(condition, Some(k::BOOL))?;
        self.emit(k::Op::If { condition }, line);
        let state = self.snapshot();
        let before = self.live.len();
        let bindings = self.bindings.len();
        let first = self.block_with_context(then_block, context)?;
        let first_reachable = self.reachable;
        let result = self.local(
            expected.unwrap_or(if first_reachable { first.1 } else { k::UNIT }),
            "$if",
            true,
        );
        if first_reachable {
            self.emit(
                k::Op::Assign {
                    place: k::Place::local(result),
                    src: first.0,
                },
                line,
            );
            self.cleanup(before, Some(result), line);
        }
        let first_state = self.snapshot();
        self.emit(k::Op::Else, line);
        self.restore(&state);
        self.bindings.truncate(bindings);
        let second = if let Some(other) = otherwise {
            let inferred = match context.inferred {
                Some(ty) => Some(ty),
                None if first_reachable => Some(self.ty(result)),
                None => None,
            };
            self.expr_with_inference(other, context.expected, inferred)?
        } else {
            self.unit(line)
        };
        let second_reachable = self.reachable;
        if second_reachable && !first_reachable && expected.is_none() {
            self.function.locals[result].ty = second.1;
        }
        if second_reachable {
            self.emit(
                k::Op::Assign {
                    place: k::Place::local(result),
                    src: second.0,
                },
                line,
            );
            self.cleanup(before, Some(result), line);
        }
        self.emit(k::Op::End, line);
        if !second_reachable && first_reachable {
            self.restore(&first_state);
        } else if second_reachable && first_reachable {
            self.merge_carriers(&first_state);
        }
        self.reachable = first_reachable || second_reachable;
        Ok((result, self.ty(result)))
    }
    fn last_loop(&self) -> Option<usize> {
        let mut index = self.loops.len();
        while index > 0 {
            index -= 1;
            if !self.loops[index].synthetic {
                return Some(index);
            }
        }
        None
    }
    fn loop_transfer(&mut self, index: usize, continuing: bool, line: usize) {
        let depth = self.loops.len() - index - 1;
        let op = match (continuing, depth) {
            (false, 0) => k::Op::Break,
            (true, 0) => k::Op::Continue,
            (false, depth) => k::Op::BreakTo { depth },
            (true, depth) => k::Op::ContinueTo { depth },
        };
        self.emit(op, line);
    }
    fn advance_range(&mut self, line: usize) -> Result<()> {
        let Some(index) = self.last_loop() else {
            return Ok(());
        };
        let (start, range) = (self.loops[index].start, self.loops[index].range);
        if let Some((counter, finish, inclusive)) = range {
            if inclusive {
                let before = self.snapshot();
                let left = self.read(k::Place::local(counter), line)?.0;
                let right = self.read(k::Place::local(finish), line)?.0;
                let done = self.temp(k::BOOL);
                self.emit(
                    k::Op::Binary {
                        dst: done,
                        op: "==".to_string(),
                        left,
                        right,
                    },
                    line,
                );
                self.emit(k::Op::If { condition: done }, line);
                self.cleanup(start, None, line);
                self.loop_transfer(index, false, line);
                self.emit(k::Op::End, line);
                self.restore(&before);
            }
            let t = self.ty(counter);
            let left = self.read(k::Place::local(counter), line)?.0;
            let right = self.temp(t);
            self.emit(
                k::Op::Int {
                    dst: right,
                    value: 1,
                },
                line,
            );
            let dst = self.temp(t);
            self.emit(
                k::Op::Binary {
                    dst,
                    op: "+".to_string(),
                    left,
                    right,
                },
                line,
            );
            self.emit(
                k::Op::Assign {
                    place: k::Place::local(counter),
                    src: dst,
                },
                line,
            );
        }
        Ok(())
    }
    fn while_expr(
        &mut self,
        condition: Option<usize>,
        body: usize,
        context: ExpressionContext,
        line: usize,
    ) -> Result<(usize, usize)> {
        let expected = context.ty();
        let result = if condition.is_none() {
            Some(self.local(expected.unwrap_or(k::UNIT), "$loop", true))
        } else {
            None
        };
        let start = self.live.len();
        let entry = self.snapshot();
        self.loops.push(LoopScope {
            synthetic: false,
            start,
            range: None,
            result,
            result_typed: expected.is_some(),
            result_expected: context.expected,
            exits: Vec::new(),
        });
        self.emit(k::Op::Loop, line);
        if let Some(condition) = condition {
            let (c, _) = self.expr(condition, Some(k::BOOL))?;
            // Condition temporaries finish on both edges, before the body or
            // exit. In particular, index locals and owned method receivers
            // cannot remain live only in K across the next loop iteration.
            self.cleanup(start, Some(c), line);
            let not = self.temp(k::BOOL);
            self.emit(
                k::Op::Unary {
                    dst: not,
                    op: "!".to_string(),
                    src: c,
                },
                line,
            );
            self.emit(k::Op::If { condition: not }, line);
            let condition_state = self.snapshot();
            self.cleanup(start, None, line);
            self.emit(k::Op::Break, line);
            self.emit(k::Op::End, line);
            self.restore(&condition_state);
        }
        let _ = self.block(body, None)?;
        if self.reachable {
            self.cleanup(start, None, line);
            self.emit(k::Op::Continue, line);
        }
        self.emit(k::Op::End, line);
        let scope = self.loops.pop().unwrap();
        if let Some(exit) = scope.exits.first() {
            self.restore(exit);
            for other in &scope.exits[1..] {
                self.merge_carriers(other);
            }
        } else {
            self.restore(&entry);
            self.reachable = false;
        }
        Ok(if let Some(result) = result {
            (result, self.ty(result))
        } else {
            self.unit(line)
        })
    }
    fn for_expr(
        &mut self,
        pattern: usize,
        iterator: usize,
        body: usize,
        line: usize,
    ) -> Result<(usize, usize)> {
        let range = self.compiler.source.exprs[iterator].clone();
        let (start, end, inclusive) = match range.kind {
            a::Expr::Range {
                start: Some(s),
                end: Some(e),
                inclusive,
            } => (s, e, inclusive),
            _ => {
                return Err(err(
                    range.span,
                    "E_UNSUPPORTED",
                    "for currently requires an integer range",
                ));
            }
        };
        let p = self.compiler.source.patterns[pattern].clone();
        let (name, mutable) = match p.kind {
            a::Pattern::Bind { name, mutable, .. } => (name, mutable),
            _ => return Err(err(p.span, "E_UNSUPPORTED", "for requires a name binding")),
        };
        let expected = self.hint(end).or_else(|| self.hint(start));
        let (s, t) = self.expr(start, expected)?;
        let (e, _) = self.expr(end, Some(t))?;
        let counter = self.local(t, "$counter", true);
        self.emit(
            k::Op::Assign {
                place: k::Place::local(counter),
                src: s,
            },
            line,
        );
        let finish = self.local(t, "$end", false);
        self.emit(
            k::Op::Assign {
                place: k::Place::local(finish),
                src: e,
            },
            line,
        );
        let bindings = self.bindings.len();
        let first = self.live.len();
        let binding = self.local(t, &name, mutable);
        self.bindings.push((name, binding));
        let state = self.snapshot();
        self.loops.push(LoopScope {
            synthetic: false,
            start: first,
            range: Some((counter, finish, inclusive)),
            result: None,
            result_typed: true,
            result_expected: None,
            exits: Vec::new(),
        });
        self.emit(k::Op::Loop, line);
        let left = self.read(k::Place::local(counter), line)?.0;
        let right = self.read(k::Place::local(finish), line)?.0;
        let done = self.temp(k::BOOL);
        self.emit(
            k::Op::Binary {
                dst: done,
                op: if inclusive { ">" } else { ">=" }.to_string(),
                left,
                right,
            },
            line,
        );
        self.emit(k::Op::If { condition: done }, line);
        self.cleanup(first, None, line);
        self.emit(k::Op::Break, line);
        self.emit(k::Op::End, line);
        self.restore(&state);
        let current = self.read(k::Place::local(counter), line)?.0;
        self.emit(
            k::Op::Assign {
                place: k::Place::local(binding),
                src: current,
            },
            line,
        );
        let _ = self.block(body, None)?;
        if self.reachable {
            self.advance_range(line)?;
            self.cleanup(first, None, line);
            self.emit(k::Op::Continue, line);
        }
        self.emit(k::Op::End, line);
        let scope = self.loops.pop().unwrap();
        if let Some(exit) = scope.exits.first() {
            self.restore(exit);
            for other in &scope.exits[1..] {
                self.merge_carriers(other);
            }
        } else {
            self.restore(&state);
            self.reachable = false;
        }
        self.bindings.truncate(bindings);
        self.emit(
            k::Op::Drop {
                place: k::Place::local(counter),
            },
            line,
        );
        self.emit(
            k::Op::Drop {
                place: k::Place::local(finish),
            },
            line,
        );
        Ok(self.unit(line))
    }
    fn variant(
        &mut self,
        path: &a::Path,
        expected: Option<usize>,
        span: a::Span,
    ) -> Result<(usize, usize)> {
        let name = self.variant_name(path).ok_or_else(|| {
            err(
                span,
                "E_TYPE",
                "variant requires a resolved enum declaration",
            )
        })?;
        let mut target = self
            .constructor_target(path, expected, span)?
            .ok_or_else(|| {
                err(
                    span,
                    "E_TYPE",
                    "variant requires a resolved enum declaration",
                )
            })?;
        if !matches!(
            self.compiler.source.declarations[target.declaration].item,
            a::Item::Enum(_)
        ) {
            return Err(err(span, "E_TYPE", "variant requires an enum declaration"));
        }
        self.constrain_constructor_target(&mut target, span)?;
        let ty = self.finish_constructor(target, span)?;
        let definition = self.enum_definition(ty, span)?;
        let tag = self.compiler.variant_positions[definition]
            .get(&name)
            .copied()
            .ok_or_else(|| err(span, "E_TYPE", "unknown enum variant"))?;
        Ok((ty, tag))
    }
    fn call(
        &mut self,
        callee: usize,
        args: &[usize],
        context: ExpressionContext,
        span: a::Span,
    ) -> Result<(usize, usize)> {
        let expected = context
            .expected
            .or(context.inferred.filter(|ty| !self.compiler.has_view(*ty)));
        let path = match self.compiler.source.exprs[callee].kind.clone() {
            a::Expr::Path(p)
                if self.binding(&p.name).is_some()
                    || self.capture_aliases.contains_key(&callee) =>
            {
                return self.callable_call(callee, args, expected, span);
            }
            a::Expr::Path(p) => self.compiler.names.value_path(
                self.compiler.source.exprs[callee].scope,
                &p,
                span,
            )?,
            _ => return self.callable_call(callee, args, expected, span),
        };
        let line = span.line;
        if path.declaration.is_none() && (path.name == "scope" || path.name == "std::thread::scope")
        {
            return Err(err(
                span,
                "E_UNSUPPORTED",
                "structured task scopes are not implemented yet",
            ));
        }
        if path.declaration.is_none()
            && (path.name == "char::from_u32" || path.name == "std::char::from_u32")
        {
            if !path.args.is_empty() || args.len() != 1 {
                return Err(err(
                    span,
                    "E_TYPE",
                    "char::from_u32 requires one u32 argument",
                ));
            }
            let integer = *self.compiler.type_names.get(&"u32".to_string()).unwrap();
            let value = self.expr(args[0], Some(integer))?;
            let character = *self.compiler.type_names.get(&"char".to_string()).unwrap();
            let path = self
                .compiler
                .names
                .library_type("prelude", "Option", span)?;
            let mut substitutions = BTreeMap::new();
            substitutions.insert("T".to_string(), character);
            let result = self.compiler.named_type(&path, &substitutions, span)?;
            let destination = self.temp(result);
            self.emit(
                k::Op::Intrinsic {
                    dst: destination,
                    name: "char_from_u32".to_string(),
                    args: vec![value.0],
                },
                span.line,
            );
            return Ok((destination, result));
        }
        if path.declaration.is_none() && path.name == "std::str::from_utf8" {
            if !path.args.is_empty() {
                return Err(err(span, "E_TYPE", "from_utf8 takes no generic arguments"));
            }
            if args.len() != 1 {
                return Err(err(span, "E_TYPE", "from_utf8 requires one byte slice"));
            }
            let slice = self.compiler.intern(
                String::new(),
                k::TypeKind::Slice {
                    inner: k::U8,
                    mutable: false,
                },
            );
            let bytes = self.compiler.reference(slice, false);
            let value = self.expr(args[0], Some(bytes))?;
            if value.1 != bytes {
                return Err(err(span, "E_TYPE", "from_utf8 requires &[u8]"));
            }
            let text_inner = self.compiler.intern(String::new(), k::TypeKind::Str);
            let text = self.compiler.reference(text_inner, false);
            let error_path = self.compiler.names.library_type("str", "Utf8Error", span)?;
            let error = self
                .compiler
                .named_type(&error_path, &BTreeMap::new(), span)?;
            let result_path = self
                .compiler
                .names
                .library_type("prelude", "Result", span)?;
            let mut sub = BTreeMap::new();
            sub.insert("T".to_string(), text);
            sub.insert("E".to_string(), error);
            let result = self.compiler.named_type(&result_path, &sub, span)?;
            let dst = self.temp(result);
            self.emit(
                k::Op::Intrinsic {
                    dst,
                    name: "str_from_utf8".to_string(),
                    args: vec![value.0],
                },
                line,
            );
            return Ok((dst, result));
        }
        if path.declaration.is_none()
            && (path.name == "std::mem::replace" || path.name == "mem::replace")
        {
            if args.len() != 2 {
                return Err(err(span, "E_TYPE", "replace requires a place and a value"));
            }
            // replace declares its first parameter as &mut T. Its solved
            // operand type supplies T at that coercion site, so a mutable
            // reference binding is reborrowed rather than moved.
            let receiver_type = self.inferred.expressions.get(&args[0]).copied();
            let receiver = self.expr(args[0], receiver_type)?;
            let inner = match self.compiler.types[receiver.1].kind {
                k::TypeKind::Ref {
                    inner,
                    mutable: true,
                } => inner,
                _ => return Err(err(span, "E_TYPE", "replace requires &mut T")),
            };
            let value = self.expr(args[1], Some(inner))?;
            let dst = self.temp(inner);
            self.emit(
                k::Op::Intrinsic {
                    dst,
                    name: "replace".to_string(),
                    args: vec![receiver.0, value.0],
                },
                line,
            );
            return Ok((dst, inner));
        }
        if path.declaration.is_none() && (path.name == "drop" || path.name == "std::mem::drop") {
            if args.len() != 1 {
                return Err(err(span, "E_TYPE", "drop requires one argument"));
            }
            let (src, _) = self.expr(args[0], None)?;
            self.emit(
                k::Op::Drop {
                    place: k::Place::local(src),
                },
                line,
            );
            return Ok(self.unit(line));
        }
        if path.declaration.is_none() && (path.name == "Vec::new" || path.name == "Box::new") {
            let boxed = path.name.contains("Box");
            let ty = if let Some(t) = expected {
                t
            } else if !path.args.is_empty() {
                self.compiler.named_type(
                    &a::Path {
                        declaration: None,
                        name: if boxed { "Box" } else { "Vec" }.to_string(),
                        args: path.args.clone(),
                    },
                    &self.sub,
                    span,
                )?
            } else if boxed && args.len() == 1 {
                let value = self.expr(args[0], None)?;
                self.compiler.check_owned_element(value.1, span)?;
                let t = self
                    .compiler
                    .intern(String::new(), k::TypeKind::Box { inner: value.1 });
                let d = self.temp(t);
                self.emit(
                    k::Op::Intrinsic {
                        dst: d,
                        name: "box_new".to_string(),
                        args: vec![value.0],
                    },
                    line,
                );
                return Ok((d, t));
            } else {
                return Err(err(
                    span,
                    "E_TYPE",
                    "collection element type needs an annotation",
                ));
            };
            let mut values = Vec::new();
            if boxed {
                let inner = match self.compiler.types[ty].kind {
                    k::TypeKind::Box { inner } => inner,
                    _ => return Err(err(span, "E_TYPE", "Box constructor result type differs")),
                };
                if args.len() != 1 {
                    return Err(err(span, "E_TYPE", "Box::new requires one argument"));
                }
                values.push(self.expr(args[0], Some(inner))?.0);
            } else if !args.is_empty() {
                return Err(err(span, "E_TYPE", "Vec::new takes no arguments"));
            }
            let dst = self.temp(ty);
            self.emit(
                k::Op::Intrinsic {
                    dst,
                    name: if boxed { "box_new" } else { "vec_new" }.to_string(),
                    args: values,
                },
                line,
            );
            return Ok((dst, ty));
        }
        let mut fields = Vec::new();
        for index in 0..args.len() {
            fields.push((format!("{index}"), args[index]));
        }
        if let Some(value) =
            self.construct_variant(&path, &fields, a::VariantStyle::Tuple, context, span)?
        {
            return Ok(value);
        }
        if let Some(a::Item::Struct(index)) = self.compiler.declaration_item(&path) {
            let s = self.compiler.source.structs[index].clone();
            if !s.tuple {
                return Err(err(
                    span,
                    "E_TYPE",
                    "named struct constructor requires fields",
                ));
            }
            return self.construct_struct(&path, &fields, None, context, span);
        }
        let mut initial_sub = BTreeMap::new();
        let mut associated = None;
        if let Some(at) = path.name.rfind("::") {
            let owner = &path.name[..at];
            let method = &path.name[at + 2..];
            let target_type = if path.args.is_empty()
                && expected.is_some_and(|t| {
                    self.compiler
                        .applications
                        .get(&t)
                        .is_some_and(|(name, _)| name == owner)
                }) {
                expected
            } else {
                self.compiler
                    .named_type(
                        &a::Path {
                            declaration: path.declaration,
                            name: owner.to_string(),
                            args: path.args.clone(),
                        },
                        &self.sub,
                        span,
                    )
                    .ok()
            };
            if let Some(t) = target_type {
                if let Some(instance) = self.compiler.static_contract(t, method, callee, span)? {
                    associated = Some(instance.source);
                    initial_sub = instance.substitutions;
                }
                let impls = if associated.is_some() {
                    Vec::new()
                } else {
                    self.compiler.implementations.method_candidates(
                        &self.compiler.types,
                        &self.compiler.applications,
                        t,
                        method,
                    )
                };
                let mut candidates: Vec<(usize, BTreeMap<String, usize>)> = Vec::new();
                let mut inherent: Vec<(usize, BTreeMap<String, usize>)> = Vec::new();
                let mut prerequisite_problem = None;
                for index in impls {
                    let im = self.compiler.source.impls[index].clone();
                    let mut methods = Vec::new();
                    for candidate in &im.methods {
                        let function = &self.compiler.source.functions[*candidate];
                        if function.name == method
                            && function.params.first().is_none_or(|p| p.name != "self")
                        {
                            methods.push(*candidate);
                        }
                    }
                    if methods.is_empty() {
                        continue;
                    }
                    let mut sub = BTreeMap::new();
                    if !self.infer_type_matches(im.target, t, &mut sub)? {
                        continue;
                    }
                    sub.insert("Self".to_string(), t);
                    if self.compiler.resolve(im.target, &sub).ok() != Some(t)
                        || !self.trait_available(&im, t, span)
                    {
                        continue;
                    }
                    if let Err(problem) = self.compiler.bounds(&im.generics, &sub) {
                        prerequisite_problem.get_or_insert(problem);
                        continue;
                    }
                    for candidate in methods {
                        if im.trait_ty.is_none() {
                            inherent.push((candidate, sub.clone()));
                        } else {
                            candidates.push((candidate, sub.clone()));
                        }
                    }
                }
                if !inherent.is_empty() {
                    candidates = inherent;
                }
                if candidates.len() > 1 {
                    return Err(err(span, "E_TRAIT", "ambiguous associated function"));
                }
                if let Some((candidate, substitutions)) = candidates.pop() {
                    associated = Some(candidate);
                    initial_sub = substitutions;
                } else if let Some(problem) = prerequisite_problem {
                    return Err(problem);
                }
            }
        }
        let index = if let Some(index) = associated {
            index
        } else {
            match self.compiler.declaration_item(&path) {
                Some(a::Item::Function(index)) => index,
                _ => {
                    return Err(err(
                        span,
                        "E_TYPE",
                        &format!("unknown function {}", path.name),
                    ));
                }
            }
        };
        if associated.is_some()
            && !matches!(
                self.compiler.source.function_owner(a::FunctionId { index }),
                a::FunctionOwner::Trait(_)
            )
        {
            self.compiler.method_access(index, self.scope, span)?;
        }
        let target = self.compiler.source.functions[index].clone();
        if target.params.len() != args.len() {
            return Err(err(span, "E_TYPE", "function argument count differs"));
        }
        let mut sub = initial_sub;
        let mut generic = Vec::new();
        for index in 0..target.generics.len() {
            if !target.generics[index].lifetime {
                generic.push(index);
            }
        }
        if !path.args.is_empty() && associated.is_none() {
            if path.args.len() != generic.len() {
                return Err(err(span, "E_TYPE", "wrong generic argument count"));
            }
            for i in 0..generic.len() {
                sub.insert(
                    target.generics[generic[i]].name.clone(),
                    self.compiler.resolve(path.args[i], &self.sub)?,
                );
            }
        }
        // Only the call's actual source context feeds argument coercions.
        // Solved operand types do not establish a reborrow site.
        if let Some(expected) = expected {
            self.infer_type_matches(target.result, expected, &mut sub)?;
        }
        let mut values = Vec::new();
        for i in 0..args.len() {
            let declared = target.params[i].ty;
            let mut parameter = self.compiler.resolve(declared, &sub).ok();
            // A known context supplies its coercion before matching the operand.
            // Only an unresolved declared reference shape needs early inference;
            // a bare T still keeps ordinary move behavior.
            if parameter.is_none() && self.declared_reference_shape(declared, &sub) {
                let inferred = self.inferred.expressions.get(&args[i]).copied();
                let actual = match inferred {
                    Some(actual) => Some(actual),
                    None => self.hint(args[i]),
                };
                if let Some(actual) = actual {
                    if !self
                        .compiler
                        .infer_argument_bindings(declared, actual, &mut sub)?
                    {
                        return Err(err(span, "E_TYPE", "generic argument types disagree"));
                    }
                }
                parameter = self.compiler.resolve(declared, &sub).ok();
            }
            let value = self.expr(args[i], parameter)?;
            if !self
                .compiler
                .infer_argument_bindings(declared, value.1, &mut sub)?
            {
                return Err(err(span, "E_TYPE", "generic argument types disagree"));
            }
            let parameter = self.compiler.resolve(declared, &sub).ok();
            let argument_span = self.compiler.source.exprs[args[i]].span;
            let value = self.coerce_value(value, parameter, argument_span)?;
            values.push(value.0);
        }
        if let Some(inferred) = context.inferred {
            self.infer_type_matches(target.result, inferred, &mut sub)?;
        }
        let function = self
            .compiler
            .instantiate(index, sub.clone())
            .map_err(|problem| at_use_site(problem, span))?;
        self.compiler.record_static_contract(function, index, sub);
        for (position, value) in values.iter().enumerate() {
            let expected = self.compiler.functions[function].locals[position].ty;
            let actual = self.ty(*value);
            if actual != expected {
                return Err(err(
                    self.compiler.source.exprs[args[position]].span,
                    "E_TYPE",
                    &format!(
                        "argument {} of {} expects {}, found {}; help: pass a value with the declared parameter type",
                        position + 1,
                        target.name,
                        self.compiler.display_type(expected),
                        self.compiler.display_type(actual)
                    ),
                ));
            }
        }
        let ty = self.compiler.functions[function].result;
        let dst = self.temp(ty);
        self.emit(
            k::Op::Call {
                dst,
                function,
                args: values,
            },
            line,
        );
        Ok((dst, ty))
    }
    fn has_declared_method(&mut self, base: usize, name: &str, span: a::Span) -> Result<bool> {
        let implementations = self.compiler.implementations.method_candidates(
            &self.compiler.types,
            &self.compiler.applications,
            base,
            name,
        );
        for index in implementations {
            let implementation = self.compiler.source.impls[index].clone();
            let mut has_name = false;
            for method in &implementation.methods {
                if self.compiler.source.functions[*method].name == name {
                    has_name = true;
                    break;
                }
            }
            if !has_name {
                continue;
            }
            let mut substitutions = BTreeMap::new();
            if !self.infer_type_matches(implementation.target, base, &mut substitutions)? {
                continue;
            }
            substitutions.insert("Self".to_string(), base);
            if self
                .compiler
                .resolve(implementation.target, &substitutions)
                .ok()
                == Some(base)
                && self.trait_available(&implementation, base, span)
                && self
                    .compiler
                    .bounds(&implementation.generics, &substitutions)
                    .is_ok()
            {
                return Ok(true);
            }
        }
        Ok(false)
    }

    fn text_method(
        &mut self,
        target: k::Place,
        name: &str,
        args: &[usize],
        span: a::Span,
    ) -> Result<(usize, usize)> {
        if matches!(
            name,
            "starts_with"
                | "ends_with"
                | "contains"
                | "find"
                | "rfind"
                | "replace"
                | "repeat"
                | "is_ascii"
                | "eq_ignore_ascii_case"
                | "to_ascii_lowercase"
                | "to_ascii_uppercase"
        ) {
            return self.source_text_method(target, name, args, span);
        }
        if !args.is_empty() {
            return Err(err(span, "E_TYPE", "text method takes no arguments"));
        }
        let receiver = self.borrow(target, false, span.line)?.0;
        if name == "to_string" {
            let path = self.compiler.names.library_type("string", "String", span)?;
            let string = self.compiler.named_type(&path, &BTreeMap::new(), span)?;
            let mut sub = BTreeMap::new();
            sub.insert("Self".to_string(), string);
            let mut selected = None;
            let implementations = self.compiler.implementations.method_candidates(
                &self.compiler.types,
                &self.compiler.applications,
                string,
                "from_str",
            );
            for index in implementations {
                let implementation = self.compiler.source.impls[index].clone();
                if implementation.trait_ty.is_none()
                    && self.compiler.resolve(implementation.target, &sub).ok() == Some(string)
                {
                    for index in implementation.methods {
                        let function = &self.compiler.source.functions[index];
                        if function.name == "from_str"
                            && function.params.first().is_none_or(|p| p.name != "self")
                        {
                            if selected.replace(index).is_some() {
                                return Err(err(
                                    span,
                                    "E_TRAIT",
                                    "ambiguous source String::from_str",
                                ));
                            }
                        }
                    }
                }
            }
            let index = selected
                .ok_or_else(|| err(span, "E_NAME", "source library String::from_str is missing"))?;
            self.compiler.method_access(index, self.scope, span)?;
            let function = self.compiler.instantiate(index, sub)?;
            let contract = &self.compiler.functions[function];
            if contract.params != 1
                || contract.locals[0].ty != self.ty(receiver)
                || contract.result != string
            {
                return Err(err(
                    span,
                    "E_TYPE",
                    "source String::from_str must take &str and return String",
                ));
            }
            let dst = self.temp(string);
            self.emit(
                k::Op::Call {
                    dst,
                    function,
                    args: vec![receiver],
                },
                span.line,
            );
            return Ok((dst, string));
        }
        let ty = if name == "as_bytes" {
            let slice = self.compiler.intern(
                String::new(),
                k::TypeKind::Slice {
                    inner: k::U8,
                    mutable: false,
                },
            );
            self.compiler.reference(slice, false)
        } else {
            k::U64
        };
        let dst = self.temp(ty);
        self.emit(
            k::Op::Intrinsic {
                dst,
                name: if name == "as_bytes" {
                    "str_as_bytes"
                } else {
                    "str_len"
                }
                .to_string(),
                args: vec![receiver],
            },
            span.line,
        );
        if name == "is_empty" {
            let zero = self.temp(k::U64);
            self.emit(
                k::Op::Int {
                    dst: zero,
                    value: 0,
                },
                span.line,
            );
            let result = self.temp(k::BOOL);
            self.emit(
                k::Op::Binary {
                    dst: result,
                    op: "==".to_string(),
                    left: dst,
                    right: zero,
                },
                span.line,
            );
            Ok((result, k::BOOL))
        } else {
            Ok((dst, ty))
        }
    }

    fn method(
        &mut self,
        expression: a::ExprId,
        receiver: usize,
        name: &str,
        _types: &[usize],
        args: &[usize],
        span: a::Span,
    ) -> Result<(usize, usize)> {
        let place = self.addressable(receiver)?;
        let mut base = place_type(&self.compiler.types, self.function, &place)
            .ok_or_else(|| err(span, "E_TYPE", "invalid method receiver"))?;
        let mut target = place.clone();
        let selection = if self.compiler.contracts.is_none() {
            self.compiler.method_selections.get(&expression).cloned()
        } else {
            None
        };
        let mut dereferences = 0;
        while let k::TypeKind::Ref { inner, .. } = self.compiler.types[base].kind {
            if selection
                .as_ref()
                .is_some_and(|selected| dereferences == selected.dereferences)
            {
                break;
            }
            target.projections.push(k::Projection::Deref);
            base = inner;
            dereferences += 1;
        }
        if let Some(selected) = &selection {
            if dereferences != selected.dereferences {
                return Err(err(
                    span,
                    "E_GENERIC",
                    "specialized receiver differs from checked dereference path",
                ));
            }
            match &selected.target {
                generic_check::MethodTarget::Trait(identity) => {
                    return self
                        .witnessed_method(&target, base, identity, expression, _types, args);
                }
                generic_check::MethodTarget::Source(index) => {
                    return self.fixed_source_method(&target, base, *index, _types, args, span);
                }
                generic_check::MethodTarget::Builtin => {}
            }
        }
        if self.compiler.contracts.is_some() {
            self.compiler.method_selections.insert(
                expression,
                generic_check::MethodSelection {
                    dereferences,
                    target: generic_check::MethodTarget::Builtin,
                },
            );
        }
        if let Some(value) = self.wrapping_method(&target, base, name, _types, args, span)? {
            return Ok(value);
        }
        if let Some(value) = self.generic_method(expression, &target, base, name, _types, args)? {
            return Ok(value);
        }
        let direct = methods::direct(methods::receiver(&self.compiler.types[base].kind), name);
        if direct == Some(methods::Direct::Character) {
            let count = if name == "to_digit" { 1 } else { 0 };
            if args.len() != count || !_types.is_empty() {
                return Err(err(
                    span,
                    "E_TYPE",
                    "character method argument count differs",
                ));
            }
            let value = self.read(target, span.line)?.0;
            let mut arguments = vec![value];
            if name == "to_digit" {
                let integer = *self.compiler.type_names.get(&"u32".to_string()).unwrap();
                arguments.push(self.expr(args[0], Some(integer))?.0);
            }
            let source =
                self.compiler
                    .names
                    .library_function("char", &format!("__lkj_{name}"), span)?;
            let function = self.compiler.instantiate(source, BTreeMap::new())?;
            let ty = self.compiler.functions[function].result;
            let dst = self.temp(ty);
            self.emit(
                k::Op::Call {
                    dst,
                    function,
                    args: arguments,
                },
                span.line,
            );
            return Ok((dst, ty));
        }
        if direct == Some(methods::Direct::Text) {
            if !_types.is_empty() {
                return Err(err(
                    span,
                    "E_TYPE",
                    "text methods take no generic arguments",
                ));
            }
            return self.text_method(target, name, args, span);
        }
        if direct == Some(methods::Direct::Output) {
            if !_types.is_empty() {
                return Err(err(span, "E_TYPE", "output methods take no type arguments"));
            }
            return self.output_method(target, base, name, args, span);
        }
        if let Some(method) = generic_check::BuiltinMethod::named(name) {
            if selection.is_some() || self.builtin_method_available(base, name, span)? {
                if self.compiler.contracts.is_some() {
                    self.compiler.method_selections.insert(
                        expression,
                        generic_check::MethodSelection {
                            dereferences,
                            target: generic_check::MethodTarget::Trait(
                                crate::generics::TraitIdentity::Builtin(
                                    method.required().to_string(),
                                ),
                            ),
                        },
                    );
                }
                return self.builtin_trait_method(&target, base, name, _types, args, span);
            }
        }
        match direct {
            Some(methods::Direct::Slice) => {
                if !args.is_empty() {
                    return Err(err(span, "E_TYPE", "slice projection takes no arguments"));
                }
                let inner = match self.compiler.types[base].kind {
                    k::TypeKind::Vec { inner } | k::TypeKind::Array { inner, .. } => inner,
                    _ => return Err(err(span, "E_TYPE", "slice projection needs a collection")),
                };
                let slice = self.compiler.intern(
                    String::new(),
                    k::TypeKind::Slice {
                        inner,
                        mutable: false,
                    },
                );
                let mutable = name == "as_mut_slice";
                let ty = self.compiler.reference(slice, mutable);
                let dst = self.temp(ty);
                self.emit(
                    k::Op::SliceBorrow {
                        dst,
                        place: target,
                        mutable,
                    },
                    span.line,
                );
                Ok((dst, ty))
            }
            Some(methods::Direct::Length) => {
                if !args.is_empty() {
                    return Err(err(span, "E_TYPE", "len takes no arguments"));
                }
                let r = self.borrow(target, false, span.line)?.0;
                let dst = self.temp(k::U64);
                self.emit(
                    k::Op::Intrinsic {
                        dst,
                        name: "vec_len".to_string(),
                        args: vec![r],
                    },
                    span.line,
                );
                Ok((dst, k::U64))
            }
            Some(methods::Direct::Push) => {
                if args.len() != 1 {
                    return Err(err(span, "E_TYPE", "push takes one argument"));
                }
                let inner = match self.compiler.types[base].kind {
                    k::TypeKind::Vec { inner } => inner,
                    _ => return Err(err(span, "E_TYPE", "push requires Vec")),
                };
                let value = self.expr(args[0], Some(inner))?;
                let r = self.borrow(target, true, span.line)?.0;
                let dst = self.temp(k::UNIT);
                self.emit(
                    k::Op::Intrinsic {
                        dst,
                        name: "vec_push".to_string(),
                        args: vec![r, value.0],
                    },
                    span.line,
                );
                Ok((dst, k::UNIT))
            }
            Some(methods::Direct::Pop) => {
                if !args.is_empty() {
                    return Err(err(span, "E_TYPE", "pop takes no arguments"));
                }
                let inner = match self.compiler.types[base].kind {
                    k::TypeKind::Vec { inner } => inner,
                    _ => return Err(err(span, "E_TYPE", "pop requires Vec")),
                };
                let mut sub = BTreeMap::new();
                sub.insert("T".to_string(), inner);
                let option_name = self.compiler.names.prelude_type("Option", span)?;
                let ty = self.compiler.named_type(
                    &a::Path {
                        declaration: None,
                        name: option_name,
                        args: Vec::new(),
                    },
                    &sub,
                    span,
                )?;
                let receiver = self.borrow(target, true, span.line)?.0;
                let dst = self.temp(ty);
                self.emit(
                    k::Op::Intrinsic {
                        dst,
                        name: "vec_pop".to_string(),
                        args: vec![receiver],
                    },
                    span.line,
                );
                Ok((dst, ty))
            }
            _ => {
                let mut candidates: Vec<(usize, BTreeMap<String, usize>)> = Vec::new();
                let mut inherent_candidates: Vec<(usize, BTreeMap<String, usize>)> = Vec::new();
                let mut unavailable_trait = false;
                let mut private_trait = false;
                let mut prerequisite_problem = None;
                let impls = self.compiler.implementations.method_candidates(
                    &self.compiler.types,
                    &self.compiler.applications,
                    base,
                    name,
                );
                for index in impls {
                    let im = self.compiler.source.impls[index].clone();
                    let mut sub = BTreeMap::new();
                    if !self.infer_type_matches(im.target, base, &mut sub)? {
                        continue;
                    }
                    sub.insert("Self".to_string(), base);
                    if let Ok(t) = self.compiler.resolve(im.target, &sub) {
                        if t == base {
                            let mut has_name = false;
                            for method in &im.methods {
                                if self.compiler.source.functions[*method].name == name {
                                    has_name = true;
                                    break;
                                }
                            }
                            if !has_name {
                                continue;
                            }
                            if !self.trait_available(&im, base, span) {
                                unavailable_trait = true;
                                if let Some(ty) = im.trait_ty {
                                    if let a::Type::Path(path) =
                                        &self.compiler.source.types[ty].kind
                                    {
                                        private_trait |= path.declaration.is_some_and(|id| {
                                            !self.compiler.names.declaration_visible(self.scope, id)
                                        });
                                    }
                                }
                                continue;
                            }
                            if let Err(problem) = self.compiler.bounds(&im.generics, &sub) {
                                prerequisite_problem.get_or_insert(problem);
                                continue;
                            }
                            for method in &im.methods {
                                if self.compiler.source.functions[*method].name == name {
                                    if im.trait_ty.is_none() {
                                        inherent_candidates.push((*method, sub.clone()));
                                    } else {
                                        candidates.push((*method, sub.clone()));
                                    }
                                }
                            }
                        }
                    }
                }
                if !inherent_candidates.is_empty() {
                    candidates = inherent_candidates;
                }
                if candidates.is_empty() {
                    if let Some(problem) = prerequisite_problem {
                        return Err(problem);
                    }
                }
                if candidates.is_empty() && unavailable_trait {
                    return Err(err(
                        span,
                        if private_trait {
                            "E_PRIVACY"
                        } else {
                            "E_TRAIT"
                        },
                        if private_trait {
                            "method belongs to a private trait"
                        } else {
                            "method trait is not in scope"
                        },
                    ));
                }
                if candidates.len() != 1 {
                    return Err(err(
                        span,
                        if candidates.is_empty() {
                            "E_UNSUPPORTED"
                        } else {
                            "E_TRAIT"
                        },
                        &format!(
                            "method {name} on {} has {} applicable implementations; help: use an implemented method or write an explicit loop",
                            self.compiler.display_type(base),
                            candidates.len()
                        ),
                    ));
                }
                let (index, sub) = candidates[0].clone();
                if self.compiler.contracts.is_some() {
                    self.compiler
                        .method_selections
                        .get_mut(&expression)
                        .unwrap()
                        .target = generic_check::MethodTarget::Source(index);
                }
                self.source_method(
                    &target,
                    base,
                    Instance {
                        source: index,
                        substitutions: sub,
                    },
                    _types,
                    args,
                    span,
                )
            }
        }
    }
    fn construct_struct(
        &mut self,
        path: &a::Path,
        fields: &[(String, usize)],
        base: Option<usize>,
        context: ExpressionContext,
        span: a::Span,
    ) -> Result<(usize, usize)> {
        if base.is_none() {
            if let Some(value) =
                self.construct_variant(path, fields, a::VariantStyle::Named, context, span)?
            {
                return Ok(value);
            }
        }
        let expected = context.expected;
        let inferred = context
            .inferred
            .or_else(|| base.and_then(|base| self.hint(base)));
        let mut target = self
            .constructor_target(path, expected, span)?
            .ok_or_else(|| err(span, "E_TYPE", "constructor requires a struct declaration"))?;
        let a::Item::Struct(index) = self.compiler.source.declarations[target.declaration].item
        else {
            return Err(err(
                span,
                "E_TYPE",
                "constructor requires a struct declaration",
            ));
        };
        self.constrain_constructor_target(&mut target, span)?;
        let definition = self.compiler.source.structs[index].clone();
        let mut positions = Vec::new();
        let mut supplied = vec![false; definition.fields.len()];
        for (name, _) in fields {
            let index = self.compiler.field_positions[index]
                .get(name)
                .copied()
                .ok_or_else(|| err(span, "E_TYPE", &format!("unknown field {name}")))?;
            if supplied[index] {
                return Err(err(span, "E_TYPE", &format!("duplicate field {name}")));
            }
            supplied[index] = true;
            self.compiler.declared_field_access(
                &definition,
                &definition.fields[index],
                self.scope,
                span,
            )?;
            positions.push(index);
        }
        if base.is_none() && supplied.iter().any(|supplied| !supplied) {
            return Err(err(span, "E_TYPE", "struct constructor is missing fields"));
        }
        let mut coercion_substitutions = target.substitutions.clone();
        if let Some(inferred) = inferred {
            self.constrain_constructor_from_type(&mut target, inferred, span)?;
        }
        // Infer from known types before evaluating initializers. Unsuffixed
        // literals then receive the same constraints regardless of field order.
        for ((_, expression), field) in fields.iter().zip(&positions) {
            if matches!(
                self.compiler.source.exprs[*expression].kind,
                a::Expr::Method { .. }
            ) {
                continue;
            }
            if let Some(hint) = self.hint(*expression) {
                self.infer_type(
                    definition.fields[*field].ty,
                    hint,
                    &mut target.substitutions,
                )?;
            }
        }
        let mut ordered = vec![None; definition.fields.len()];
        for ((_, expression), field) in fields.iter().zip(positions) {
            let pattern = definition.fields[field].ty;
            let value = self.constructor_argument(
                *expression,
                pattern,
                &target.substitutions,
                &mut coercion_substitutions,
            )?;
            self.infer_type(pattern, value.1, &mut target.substitutions)?;
            ordered[field] = Some(value);
        }
        let base_place = if let Some(base) = base {
            let existing = match &self.compiler.source.exprs[base].kind {
                a::Expr::Path(path) => self.binding(&path.name).is_some(),
                a::Expr::Field { .. } | a::Expr::Index { .. } => true,
                a::Expr::Unary { op, .. } => op == "*",
                _ => false,
            };
            let place = if existing {
                self.place(base)?
            } else {
                k::Place::local(self.expr(base, expected)?.0)
            };
            let ty = place_type(&self.compiler.types, self.function, &place)
                .ok_or_else(|| err(span, "E_TYPE", "invalid struct update base"))?;
            self.constrain_constructor_from_type(&mut target, ty, span)?;
            Some(place)
        } else {
            None
        };
        let ty = self.finish_constructor(target, span)?;
        let k::TypeKind::Struct {
            fields: declared, ..
        } = self.compiler.types[ty].kind.clone()
        else {
            unreachable!()
        };
        if let Some(base) = base_place {
            for (index, field) in declared.iter().enumerate() {
                if ordered[index].is_none() {
                    self.compiler
                        .field_access(ty, &field.name, self.scope, span)?;
                    let mut place = base.clone();
                    place.projections.push(k::Projection::Field(index));
                    ordered[index] = Some(self.read(place, span.line)?);
                }
            }
        }
        let mut values = Vec::new();
        for (value, field) in ordered.into_iter().zip(&declared) {
            let (local, actual) = value.expect("all struct fields supplied");
            if actual != field.ty {
                return Err(err(
                    span,
                    "E_TYPE",
                    &format!("struct field {} type differs", field.name),
                ));
            }
            values.push(local);
        }
        let dst = self.temp(ty);
        self.emit(
            k::Op::Struct {
                dst,
                fields: values,
            },
            span.line,
        );
        Ok((dst, ty))
    }
    fn macro_expr(
        &mut self,
        name: &str,
        args: &[usize],
        separator: &str,
        expected: Option<usize>,
        span: a::Span,
    ) -> Result<(usize, usize)> {
        match name {
            "format" | "write" | "writeln" => self.format_macro(name, args, span),
            "assert" => {
                if args.is_empty() {
                    return Err(err(span, "E_TYPE", "assert needs a condition"));
                }
                let condition = self.expr(args[0], Some(k::BOOL))?.0;
                let dst = self.temp(k::UNIT);
                self.emit(
                    k::Op::Intrinsic {
                        dst,
                        name: "assert".to_string(),
                        args: vec![condition],
                    },
                    span.line,
                );
                Ok((dst, k::UNIT))
            }
            "assert_eq" | "assert_ne" => {
                if args.len() < 2 {
                    return Err(err(span, "E_TYPE", "assertion needs two operands"));
                }
                let condition = self
                    .equality_expr(
                        if name == "assert_eq" { "==" } else { "!=" },
                        args[0],
                        args[1],
                        span,
                    )?
                    .0;
                let dst = self.temp(k::UNIT);
                self.emit(
                    k::Op::Intrinsic {
                        dst,
                        name: "assert".to_string(),
                        args: vec![condition],
                    },
                    span.line,
                );
                Ok((dst, k::UNIT))
            }
            "panic" | "unreachable" => {
                self.emit(
                    k::Op::Trap {
                        message: "explicit panic".to_string(),
                    },
                    span.line,
                );
                Ok(self.unit(span.line))
            }
            "vec" => {
                let inner = expected.and_then(|t| match self.compiler.types[t].kind {
                    k::TypeKind::Vec { inner } => Some(inner),
                    _ => None,
                });
                if separator == ";" {
                    if args.len() != 2 {
                        return Err(err(
                            span,
                            "E_TYPE",
                            "Vec repetition requires an element and count",
                        ));
                    }
                    let value = self.expr(args[0], inner)?;
                    let count = self.expr(args[1], Some(k::U64))?;
                    return self.vector_repeat(value.0, count.0, value.1, span);
                }
                let mut values = Vec::new();
                let mut inferred = inner;
                for arg in args {
                    let value = self.expr(*arg, inferred)?;
                    inferred = Some(value.1);
                    values.push(value.0);
                }
                let inner =
                    inferred.ok_or_else(|| err(span, "E_TYPE", "empty Vec needs a type"))?;
                let ty = self
                    .compiler
                    .intern(String::new(), k::TypeKind::Vec { inner });
                let dst = self.local(ty, "$vec", true);
                self.emit(
                    k::Op::Intrinsic {
                        dst,
                        name: "vec_new".to_string(),
                        args: Vec::new(),
                    },
                    span.line,
                );
                for value in values {
                    let r = self.borrow(k::Place::local(dst), true, span.line)?.0;
                    let unit = self.temp(k::UNIT);
                    self.emit(
                        k::Op::Intrinsic {
                            dst: unit,
                            name: "vec_push".to_string(),
                            args: vec![r, value],
                        },
                        span.line,
                    );
                    self.emit(
                        k::Op::Drop {
                            place: k::Place::local(unit),
                        },
                        span.line,
                    );
                }
                Ok((dst, ty))
            }
            _ => Err(err(
                span,
                "E_UNSUPPORTED",
                &format!("macro {name} is not implemented"),
            )),
        }
    }
    fn match_expr(
        &mut self,
        value: usize,
        arms: &[a::Arm],
        context: ExpressionContext,
        span: a::Span,
    ) -> Result<(usize, usize)> {
        let expected = context.ty();
        let hint = self.hint(value).or_else(|| self.pattern_hint(arms));
        let existing = match &self.compiler.source.exprs[value].kind {
            a::Expr::Path(path) => {
                self.binding(&path.name).is_some() || self.guard_aliases.contains_key(&path.name)
            }
            a::Expr::Field { .. } | a::Expr::Index { .. } => true,
            a::Expr::Unary { op, .. } => op == "*",
            _ => false,
        };
        let original = if existing {
            self.place(value)?
        } else {
            let (local, _) = self.expr(value, hint)?;
            self.function.locals[local].mutable = true;
            k::Place::local(local)
        };
        let ty = place_type(&self.compiler.types, self.function, &original)
            .ok_or_else(|| err(span, "E_TYPE", "invalid match scrutinee place"))?;
        let mut plan = patterns::PatternPlan::default();
        let mut roots = Vec::new();
        let mut coverage = Vec::new();
        for arm in arms {
            let root = self.prepare_pattern(&mut plan, arm.pattern, ty, None)?;
            if arm.guard.is_none() {
                coverage.push(root);
            }
            roots.push(root);
        }
        if !self.patterns_exhaustive(&plan, &coverage, ty)? {
            return Err(err(span, "E_TYPE", "match is not exhaustive"));
        }
        let mut unique_places = BTreeSet::new();
        for root in &roots {
            for place in plan.tested_places(*root, original.clone()) {
                unique_places.insert(place);
            }
        }
        let mut tested_places = Vec::new();
        for place in unique_places {
            tested_places.push(place);
        }
        let start = self.live.len();
        let cache = if arms.iter().any(|arm| arm.guard.is_some()) {
            Some(self.prepare_pattern_cache(&plan, &roots, original.clone(), span.line)?)
        } else {
            None
        };
        let binding_count = self.bindings.len();
        let mut result: Option<usize> = None;
        let mut exit_state: Option<Snapshot> = None;
        let mut exhausted = false;
        let extends_scrutinee = arms.iter().any(|arm| {
            matches!(
                self.compiler.source.exprs[arm.value].kind,
                a::Expr::Diverge(_)
            )
        });
        for index in 0..arms.len() {
            let arm = &arms[index];
            let pattern = roots[index];
            let test_start = self.live.len();
            let mut choices = None;
            let condition = if let Some(guard) = arm.guard {
                let cache = cache.as_ref().unwrap();
                if plan.has_or(pattern) {
                    let (condition, selected) = self.retry_pattern_guard(
                        &plan,
                        pattern,
                        original.clone(),
                        guard,
                        cache,
                        &tested_places,
                    )?;
                    choices = Some(selected);
                    condition
                } else {
                    let condition = self.cached_pattern_condition(&plan, pattern, cache, None)?;
                    self.pattern_guard(
                        &plan,
                        pattern,
                        original.clone(),
                        guard,
                        condition,
                        &tested_places,
                    )?
                }
            } else if let Some(cache) = &cache {
                self.cached_pattern_condition(&plan, pattern, cache, None)?
            } else {
                self.pattern_condition(&plan, pattern, original.clone())?
            };
            self.emit(k::Op::If { condition }, arm.span.line);
            let failure = self.snapshot();
            self.bind_pattern_choice(
                &plan,
                pattern,
                original.clone(),
                choices.as_ref(),
                cache.is_none(),
                arm.span.line,
            )?;
            let inferred = context.inferred.or(result.map(|id| self.ty(id)));
            let value = self.expr_with_inference(arm.value, context.expected, inferred)?;
            if self.reachable && !exhausted {
                let target = if let Some(target) = result {
                    target
                } else {
                    let target = self.local(expected.unwrap_or(value.1), "$match", true);
                    result = Some(target);
                    target
                };
                self.emit(
                    k::Op::Assign {
                        place: k::Place::local(target),
                        src: value.0,
                    },
                    span.line,
                );
                self.cleanup(start, Some(target), span.line);
                let extended = extends_scrutinee && self.compiler.has_view(self.ty(target));
                if !existing && !extended && self.live.at(original.local) {
                    self.cleanup_place(original.clone(), span.line);
                }
                // An arm's new bindings are out of scope here. Preserve the
                // reassignment history of incoming locals; the match result is
                // an internal mutable slot and has no source name.
                self.written.truncate(start);
                self.written.resize(self.function.locals.len(), false);
                if let Some(exit) = &exit_state {
                    self.merge_carriers(exit);
                }
                exit_state = Some(self.snapshot());
            }
            self.bindings.truncate(binding_count);
            self.emit(k::Op::Else, span.line);
            if arm.guard.is_none() && self.pattern_irrefutable(&plan, pattern) {
                self.emit(
                    k::Op::Trap {
                        message: "unreachable arm after irrefutable pattern".to_string(),
                    },
                    span.line,
                );
                exhausted = true;
            }
            self.restore(&failure);
            self.cleanup(test_start, None, span.line);
            self.written.truncate(start);
            self.written.resize(self.function.locals.len(), false);
        }
        self.emit(
            k::Op::Trap {
                message: "unreachable match".to_string(),
            },
            span.line,
        );
        for _ in arms {
            self.emit(k::Op::End, span.line);
        }
        if let Some(exit) = exit_state {
            self.restore(&exit);
            let result = result.ok_or_else(|| err(span, "E_TYPE", "match result is missing"))?;
            Ok((result, self.ty(result)))
        } else {
            self.reachable = false;
            Ok(self.unit(span.line))
        }
    }
}
