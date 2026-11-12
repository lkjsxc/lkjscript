//! Rust-compatible syntax to explicit K. Inference lives here; K trusts none of it.
mod assignment;
mod borrow_cleanup;
mod constructors;
mod definition_types;
mod derive_check;
mod derive_methods;
mod diagnostic_inference;
mod generic_check;
mod inference;
mod integers;
mod operators;
mod output;
mod patterns;
mod sequences;
mod static_lifetimes;
mod type_resolve;
mod variants;

use crate::ast as a;
use crate::check::{borrowed_projection, is_copy, place_type};
use crate::facts::Facts;
use crate::k;
use crate::names::qualified;
use std::collections::BTreeMap;

type Result<T> = std::result::Result<T, a::Diagnostic>;
fn err(span: a::Span, code: &str, message: &str) -> a::Diagnostic {
    a::Diagnostic::new(span, code, message)
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
}

fn first_problem(problems: Vec<a::Diagnostic>) -> a::Diagnostic {
    problems
        .into_iter()
        .next()
        .expect("failed analysis has a diagnostic")
}

pub fn elaborate(source: &a::Program) -> Result<k::Program> {
    elaborate_all(source).map_err(first_problem)
}

/// Resolve declarations before collecting independent definition diagnostics.
pub fn elaborate_all(source: &a::Program) -> std::result::Result<k::Program, Vec<a::Diagnostic>> {
    let resolved = crate::resolve::resolve(source).map_err(|error| vec![error])?;
    elaborate_resolved_all(&resolved)
}

/// Elaborate an already resolved source graph; the independent K gate follows.
pub fn elaborate_resolved(source: &a::Program) -> Result<k::Program> {
    elaborate_resolved_all(source).map_err(first_problem)
}

/// Recorded definition constraints and rigid generic evidence. Only this phase
/// selects source-level generic witnesses; concrete construction consumes them.
pub struct TypedDefinitions<'source> {
    compiler: Compiler<'source>,
    evidence: generic_check::DefinitionEvidence,
}

/// Definition inference is isolated, so its errors can be collected without
/// inventing signatures or continuing partially mutated K/type interning state.
pub fn infer_resolved_all(
    source: &a::Program,
) -> std::result::Result<TypedDefinitions<'_>, Vec<a::Diagnostic>> {
    static_lifetimes::annotations(source).map_err(|error| vec![error])?;
    crate::generics::validate(source).map_err(|error| vec![error])?;
    static_lifetimes::outputs(source).map_err(|error| vec![error])?;
    let compiler = Compiler::new(source).map_err(|error| vec![error])?;
    let definitions = diagnostic_inference::infer_all(source, &compiler.names)?;
    let evidence = generic_check::validate(source, definitions).map_err(|error| vec![error])?;
    Ok(TypedDefinitions { compiler, evidence })
}

/// Build concrete K from the same recorded answers used for definition checking.
pub fn lower_definitions(
    definitions: TypedDefinitions<'_>,
) -> std::result::Result<k::Program, Vec<a::Diagnostic>> {
    let source = definitions.compiler.source;
    elaborate_definitions(source, definitions.compiler, definitions.evidence)
        .map_err(|error| vec![error])
}

pub fn elaborate_resolved_all(
    source: &a::Program,
) -> std::result::Result<k::Program, Vec<a::Diagnostic>> {
    lower_definitions(infer_resolved_all(source)?)
}

fn elaborate_definitions(
    source: &a::Program,
    mut c: Compiler<'_>,
    evidence: generic_check::DefinitionEvidence,
) -> Result<k::Program> {
    c.method_selections = evidence.methods;
    c.definition_types = evidence.types;
    let empty = BTreeMap::new();
    for s in &source.structs {
        if s.generics.is_empty() {
            let _ = c.named_type(
                &a::Path {
                    declaration: c
                        .declaration_names
                        .get(&qualified(&s.module, &s.name))
                        .copied(),
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
            let _ = c.named_type(
                &a::Path {
                    declaration: c
                        .declaration_names
                        .get(&qualified(&e.module, &e.name))
                        .copied(),
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
            c.functions[index] = match pending {
                PendingFunction::Source(instance) => c.compile_function(index, &instance)?,
                PendingFunction::Derived(instance) => c.compile_derived(index, &instance)?,
            };
            next += 1;
        }
    }

    Ok(k::Program {
        types: c.types,
        functions: c.functions,
        destructors: c.destructors,
    })
}

impl<'a> Compiler<'a> {
    fn new(source: &'a a::Program) -> Result<Self> {
        let mut c = Compiler {
            source,
            names: crate::resolve::Resolver::new(source)?,
            types: k::primitives(),
            type_names: BTreeMap::new(),
            type_keys: BTreeMap::new(),
            declaration_names: source
                .declarations
                .iter()
                .enumerate()
                .map(|(id, d)| (d.name.clone(), id))
                .collect(),
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
        };
        for i in 0..c.types.len() {
            c.type_names.insert(c.types[i].name.clone(), i);
        }
        for (name, bits, signed) in [
            ("u64", 64, false),
            ("isize", 64, true),
            ("u16", 16, false),
            ("u32", 32, false),
            ("i8", 8, true),
            ("i16", 16, true),
        ] {
            let id = c.types.len();
            c.types.push(k::Type {
                name: name.to_string(),
                kind: k::TypeKind::Int { bits, signed },
            });
            c.type_names.insert(name.to_string(), id);
        }
        for node in &source.exprs {
            if let a::Expr::Path(path) = &node.kind {
                c.uses
                    .entry((node.span.file, path.name.clone()))
                    .or_default()
                    .push(node.span.start);
            }
        }
        for positions in c.uses.values_mut() {
            positions.sort_unstable();
        }
        Ok(c)
    }
}

impl Compiler<'_> {
    fn declaration_item(&self, path: &a::Path) -> Option<a::Item> {
        path.declaration
            .and_then(|id| self.source.declarations.get(id))
            .filter(|declaration| declaration.name == path.name)
            .map(|declaration| declaration.item)
    }
    fn field_access(&self, ty: usize, name: &str, scope: a::ModuleId, span: a::Span) -> Result<()> {
        if let Some((owner, _)) = self.applications.get(&ty) {
            if let Some(a::Item::Struct(index)) = self
                .declaration_names
                .get(owner)
                .map(|id| self.source.declarations[*id].item)
            {
                let definition = &self.source.structs[index];
                if let Some(field) = definition.fields.iter().find(|f| f.name == name) {
                    self.declared_field_access(definition, field, scope, span)?;
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
                let ids: Vec<_> = fields.iter().map(|f| f.ty).collect();
                let name = format!(
                    "({})",
                    ids.iter()
                        .map(|t| format!("t{t}"))
                        .collect::<Vec<_>>()
                        .join(",")
                );
                (TypeKey::Tuple(ids), name)
            }
            k::TypeKind::Vec { inner } => (TypeKey::Vec(*inner), format!("Vec<t{inner}>")),
            k::TypeKind::Box { inner } => (TypeKey::Box(*inner), format!("Box<t{inner}>")),
            k::TypeKind::World => (TypeKey::World, "World".to_string()),
            k::TypeKind::Stdout => (TypeKey::Stdout, "Stdout".to_string()),
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
        self.intern(
            format!(
                "&{}{}",
                if mutable { "mut " } else { "" },
                self.types[inner].name
            ),
            k::TypeKind::Ref { inner, mutable },
        )
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
        source: &a::Function,
        locals: &[k::Local],
        result: usize,
    ) -> Result<Option<k::ReturnSource>> {
        if !self.has_view(result) {
            return Ok(None);
        }
        if static_lifetimes::has_static(self.source, source.result) {
            return Ok(Some(k::ReturnSource::Static));
        }
        let lifetimes = self.lifetimes(source.result);
        if lifetimes.len() > 1 {
            return Err(err(
                source.span,
                "E_BORROW",
                "a returned view has one source lifetime",
            ));
        }
        let mut candidates = Vec::new();
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
                return Ok(Some(k::ReturnSource::Static));
            }
            return Ok(None);
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
        Ok(Some(k::ReturnSource::Parameter {
            index,
            projections: vec![k::Projection::Deref; dereferences],
        }))
    }
    fn instantiate(&mut self, index: usize, mut sub: BTreeMap<String, usize>) -> Result<usize> {
        self.prepare_associated(index, &mut sub)?;
        let source = self.source.functions[index].clone();
        let key = (
            index,
            sub.iter().map(|(name, ty)| (name.clone(), *ty)).collect(),
        );
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
        let return_source = self.return_source(&source, &locals, result)?;
        let id = self.functions.len();
        self.instance_names.insert(key, id);
        self.pending_functions.push((
            id,
            PendingFunction::Source(Instance {
                source: index,
                substitutions: sub,
            }),
        ));
        self.functions.push(k::Function {
            file: source.span.file,
            name,
            params: locals.len(),
            result,
            return_source,
            locals,
            body: Vec::new(),
            test: source.attrs.iter().any(|a| a.name == "test"),
        });
        Ok(id)
    }
    fn bounds(&self, parameters: &[a::GenericParam], sub: &BTreeMap<String, usize>) -> Result<()> {
        for parameter in parameters {
            if parameter.lifetime {
                continue;
            }
            let ty = *sub
                .get(&parameter.name)
                .ok_or_else(|| err(parameter.span, "E_TYPE", "cannot infer generic argument"))?;
            for id in &parameter.bounds {
                let (bound, identity) = match &self.source.types[*id].kind {
                    a::Type::Path(p) if p.args.is_empty() => (
                        p.name.as_str(),
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
                        &format!("{} does not implement {}", self.types[ty].name, bound),
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
        let impls = self.source.impls.clone();
        for im in impls {
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
                    let parameters=pattern.args.iter().filter(|p|!matches!(&self.source.types[**p].kind,a::Type::Path(path) if path.name.starts_with("'"))).copied().collect::<Vec<_>>();
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
                    let methods = im
                        .methods
                        .iter()
                        .filter(|i| self.source.functions[**i].name == "drop")
                        .copied()
                        .collect::<Vec<_>>();
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
        let definition = match self.definition_types.remove(&instance.source) {
            Some(definition) => definition,
            None => definition_types::infer(self.source, &self.names, instance.source)?,
        };
        let inferred = definition.instantiate(self, &instance.substitutions);
        self.definition_types.insert(instance.source, definition);
        let inferred = std::rc::Rc::new(inferred?);
        let source = self.source.functions[instance.source].clone();
        let mut function = self.functions[index].clone();
        let bindings = function
            .locals
            .iter()
            .enumerate()
            .map(|(i, l)| (l.name.clone(), i))
            .collect();
        let live: Facts = function.locals.iter().map(|_| true).collect();
        let references = Facts::new(live.len());
        let scope = self.names.scope(&source.module);
        let mut builder = Builder {
            source_function: instance.source,
            scope,
            file: source.span.file,
            compiler: self,
            function: &mut function,
            sub: instance.substitutions.clone(),
            bindings,
            references,
            carriers: BTreeMap::new(),
            assignment_types: BTreeMap::new(),
            inferred,
            written: live.clone(),
            live,
            loops: Vec::new(),
            reachable: true,
        };
        let result_ty = builder.function.result;
        let result = builder.block(
            source
                .body
                .ok_or_else(|| err(source.span, "E_TYPE", "function has no body"))?,
            Some(result_ty),
        )?;
        if builder.reachable {
            if result.1 != result_ty {
                return Err(err(source.span, "E_TYPE", "function result type differs"));
            }
            builder.cleanup(0, Some(result.0), source.span.line);
            builder.emit(k::Op::Return { src: result.0 }, source.span.line);
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
        self.visible
            .entry(binding.0.clone())
            .or_default()
            .push(binding.1);
        self.entries.push(binding);
    }
    fn len(&self) -> usize {
        self.entries.len()
    }
    fn get(&self, name: &str) -> Option<usize> {
        self.visible
            .get(name)
            .and_then(|locals| locals.last())
            .copied()
    }
    fn truncate(&mut self, len: usize) {
        while self.entries.len() > len {
            let (name, _) = self.entries.pop().unwrap();
            let locals = self.visible.get_mut(&name).unwrap();
            locals.pop();
            if locals.is_empty() {
                self.visible.remove(&name);
            }
        }
    }
}
impl FromIterator<(String, usize)> for Bindings {
    fn from_iter<I: IntoIterator<Item = (String, usize)>>(iter: I) -> Self {
        let mut bindings = Self::default();
        for binding in iter {
            bindings.push(binding);
        }
        bindings
    }
}

#[derive(Clone)]
struct Snapshot {
    live: Facts,
    written: Facts,
    references: Facts,
    carriers: borrow_cleanup::Dependencies,
}
struct LoopScope {
    start: usize,
    range: Option<(usize, usize, bool)>,
    exits: Vec<Snapshot>,
}
struct Builder<'a, 'b> {
    source_function: usize,
    scope: a::ModuleId,
    file: a::FileId,
    compiler: &'a mut Compiler<'b>,
    function: &'a mut k::Function,
    sub: BTreeMap<String, usize>,
    bindings: Bindings,
    references: Facts,
    carriers: borrow_cleanup::Dependencies,
    assignment_types: BTreeMap<usize, usize>,
    inferred: std::rc::Rc<definition_types::Types>,
    live: Facts,
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
    fn trait_available(&self, implementation: &a::Impl, base: usize) -> bool {
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
        if self.compiler.names.trait_in_scope(self.scope, declaration) {
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
            if self.sub.get("Self") == Some(&base) {
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
        self.written.push(false);
        self.references.push(false);
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
        self.references.set(
            local,
            live && !self.function.locals[local].name.starts_with('$')
                && matches!(
                    self.compiler.types[self.ty(local)].kind,
                    k::TypeKind::Ref { .. }
                ),
        );
    }
    fn emit(&mut self, op: k::Op, line: usize) {
        if let k::Op::Assign { place, src } = &op {
            if self.live[place.local] && place.local != *src {
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
                        let destruction = if place.projections.is_empty() {
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
            k::Op::Break => {
                let state = self.snapshot();
                if let Some(scope) = self.loops.last_mut() {
                    scope.exits.push(state);
                }
                self.reachable = false;
            }
            k::Op::Continue | k::Op::Trap { .. } => self.reachable = false,
            _ => {}
        }
        self.function.body.push(k::Instruction { op, line });
    }
    fn cleanup(&mut self, start: usize, except: Option<usize>, line: usize) {
        let locals: Vec<_> = self.live.indices_from(start).collect();
        for i in locals {
            if Some(i) != except {
                self.emit(
                    k::Op::Drop {
                        place: k::Place::local(i),
                    },
                    line,
                );
            }
        }
    }
    fn unit(&mut self, line: usize) -> (usize, usize) {
        let d = self.temp(k::UNIT);
        self.emit(k::Op::Unit { dst: d }, line);
        (d, k::UNIT)
    }
    fn snapshot(&self) -> Snapshot {
        Snapshot {
            live: self.live.clone(),
            written: self.written.clone(),
            references: self.references.clone(),
            carriers: self.carriers.clone(),
        }
    }
    fn restore(&mut self, state: &Snapshot) {
        self.carriers = state.carriers.clone();
        self.live = state.live.clone();
        self.live.resize(self.function.locals.len(), false);
        self.written = state.written.clone();
        self.written.resize(self.function.locals.len(), false);
        self.references = state.references.clone();
        self.references.resize(self.function.locals.len(), false);
        self.reachable = true;
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
        self.emit(
            k::Op::Read {
                dst,
                place,
                moving: !is_copy(&self.compiler.types, ty),
            },
            line,
        );
        Ok((dst, ty))
    }
    fn borrow(&mut self, place: k::Place, mutable: bool, line: usize) -> Result<(usize, usize)> {
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
        if let Some(ty) = self.inferred.expressions.get(&id) {
            if !self.compiler.has_view(*ty) {
                return Some(*ty);
            }
        }
        match self.compiler.source.exprs[id].kind.clone() {
            a::Expr::Path(p) => self.binding(&p.name).map(|i| self.ty(i)),
            a::Expr::Cast { ty, .. } => self.compiler.resolve(ty, &self.sub).ok(),
            a::Expr::Bool(_) => Some(k::BOOL),
            a::Expr::Float(_) => Some(k::F64),
            a::Expr::Int(s) => {
                for (name, t) in &self.compiler.type_names {
                    if s.ends_with(name.as_str()) && s.len() > name.len() {
                        return Some(*t);
                    }
                }
                None
            }
            a::Expr::Unary { op, value } if op == "*" => {
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
                match &self.compiler.types[t].kind {
                    k::TypeKind::Struct { fields, .. } => {
                        fields.iter().find(|f| f.name == name).map(|f| f.ty)
                    }
                    _ => None,
                }
            }
            a::Expr::Index { value, .. } => {
                let mut t = self.hint(value)?;
                while let k::TypeKind::Ref { inner, .. } = self.compiler.types[t].kind {
                    t = inner;
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
        let node = self.compiler.source.exprs[id].clone();
        match node.kind {
            a::Expr::Path(path) => self
                .binding(&path.name)
                .map(k::Place::local)
                .ok_or_else(|| err(node.span, "E_TYPE", &format!("unknown value {}", path.name))),
            a::Expr::Unary { op, value } if op == "*" => {
                let mut place = self.addressable(value)?;
                place.projections.push(k::Projection::Deref);
                Ok(place)
            }
            a::Expr::Field { value, name } => {
                let mut place = self.addressable(value)?;
                let mut ty = place_type(&self.compiler.types, self.function, &place)
                    .ok_or_else(|| err(node.span, "E_TYPE", "invalid field base"))?;
                while let k::TypeKind::Ref { inner, .. } | k::TypeKind::Box { inner } =
                    self.compiler.types[ty].kind
                {
                    place.projections.push(k::Projection::Deref);
                    ty = inner;
                }
                if let k::TypeKind::Struct { fields, .. } = &self.compiler.types[ty].kind {
                    let index = fields
                        .iter()
                        .position(|f| f.name == name)
                        .ok_or_else(|| err(node.span, "E_TYPE", "unknown field"))?;
                    self.compiler
                        .field_access(ty, &name, self.scope, node.span)?;
                    place.projections.push(k::Projection::Field(index));
                    Ok(place)
                } else {
                    Err(err(node.span, "E_TYPE", "field access requires a struct"))
                }
            }
            a::Expr::Index { value, index } => {
                let mut place = self.addressable(value)?;
                let mut ty = place_type(&self.compiler.types, self.function, &place)
                    .ok_or_else(|| err(node.span, "E_TYPE", "invalid index base"))?;
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
        match self.compiler.source.exprs[id].kind {
            a::Expr::Path(_)
            | a::Expr::Field { .. }
            | a::Expr::Index { .. }
            | a::Expr::Unary { .. } => self.place(id),
            _ => {
                let v = self.expr(id, None)?;
                Ok(k::Place::local(v.0))
            }
        }
    }
    fn end_unused_borrows(&mut self, start: usize, position: usize, end: usize, line: usize) {
        let mut drops = std::collections::BTreeSet::new();
        for local in self.references.indices_from(start) {
            let name = &self.function.locals[local].name;
            if self.live[local] {
                let mut used = false;
                if let Some(positions) = self.compiler.uses.get(&(self.file, name.clone())) {
                    let mut low = 0;
                    let mut high = positions.len();
                    while low < high {
                        let mid = low + (high - low) / 2;
                        if positions[mid] < position {
                            low = mid + 1;
                        } else {
                            high = mid;
                        }
                    }
                    used = low < positions.len() && positions[low] < end;
                }
                if !used {
                    drops.insert(local);
                }
            }
        }
        self.drop_unused_references(drops, line);
    }
    fn bind_name(&mut self, name: String, mutable: bool, value: (usize, usize), line: usize) {
        let dst = self.local(value.1, &name, mutable);
        self.emit(
            k::Op::Assign {
                place: k::Place::local(dst),
                src: value.0,
            },
            line,
        );
        self.bindings.push((name, dst));
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
                self.bind_name(name, mutable, val, line);
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
        if matches!(
            self.compiler.source.patterns[pattern].kind,
            a::Pattern::Bind { by_ref: true, .. }
        ) {
            return true;
        }
        let mut todo = vec![value];
        while let Some(id) = todo.pop() {
            match &self.compiler.source.exprs[id].kind {
                a::Expr::Borrow { .. } => return true,
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
        let locals: Vec<_> = self.live.indices_from(start).collect();
        for local in locals {
            if self.function.locals[local].name.starts_with('$') {
                self.emit(
                    k::Op::Drop {
                        place: k::Place::local(local),
                    },
                    line,
                );
            }
        }
    }
    fn block(&mut self, id: usize, expected: Option<usize>) -> Result<(usize, usize)> {
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
                        let v = self.expr(value, expected)?;
                        if let Some(t) = expected {
                            if t != v.1 {
                                return Err(err(span, "E_TYPE", "binding type differs"));
                            }
                        }
                        self.bind(pattern, v, span.line)?;
                        if !extends {
                            self.cleanup_temporaries(before, span.line);
                        }
                    } else {
                        let p = self.compiler.source.patterns[pattern].clone();
                        if let a::Pattern::Bind { name, mutable, .. } = p.kind {
                            let t = expected.ok_or_else(|| {
                                err(span, "E_TYPE", "uninitialized binding requires a type")
                            })?;
                            let local = self.local(t, &name, mutable);
                            self.bindings.push((name, local));
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
                let value = self.expr(tail, expected)?;
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
                self.expr(tail, expected)?
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
    fn expr(&mut self, id: usize, expected: Option<usize>) -> Result<(usize, usize)> {
        self.expr_with_inference(id, expected, None)
    }
    fn expr_with_inference(
        &mut self,
        id: usize,
        expected: Option<usize>,
        inferred: Option<usize>,
    ) -> Result<(usize, usize)> {
        // A solved type is not by itself a coercion site. In particular a bare
        // mutable reference binding moves its value; it must not acquire the
        // automatic reborrow used at an explicitly typed argument or binding.
        // Reference-bearing constructor inference stays on the existing path
        // until the typed form records each coercion separately.
        let inferred = inferred
            .or_else(|| self.inferred.expressions.get(&id).copied())
            .filter(|ty| !self.compiler.has_view(*ty));
        let reference_context = expected.is_some_and(|target| {
            matches!(self.compiler.types[target].kind, k::TypeKind::Ref { .. })
        });
        let place_expression = match &self.compiler.source.exprs[id].kind {
            a::Expr::Path(path) => self.binding(&path.name).is_some(),
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
            self.expr_inner(id, expected.or(inferred))?
        };
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
                                    self.compiler.source.exprs[id].span.line,
                                );
                                self.emit(
                                    k::Op::Drop {
                                        place: k::Place::local(value.0),
                                    },
                                    self.compiler.source.exprs[id].span.line,
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
                            let result = self.borrow(
                                place,
                                want_mut,
                                self.compiler.source.exprs[id].span.line,
                            )?;
                            self.emit(
                                k::Op::Drop {
                                    place: k::Place::local(value.0),
                                },
                                self.compiler.source.exprs[id].span.line,
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
    fn expr_inner(&mut self, id: usize, expected: Option<usize>) -> Result<(usize, usize)> {
        let node = self.compiler.source.exprs[id].clone();
        let line = node.span.line;
        match node.kind {
            a::Expr::Int(text) => self.integer(&text, expected, false, node.span),
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
            a::Expr::Bool(value) => {
                let d = self.temp(k::BOOL);
                self.emit(
                    k::Op::Int {
                        dst: d,
                        value: i64::from(value),
                    },
                    line,
                );
                Ok((d, k::BOOL))
            }
            a::Expr::Float(text) => {
                let value = text
                    .trim_end_matches("f64")
                    .replace('_', "")
                    .parse::<f64>()
                    .map_err(|_| err(node.span, "E_TYPE", "invalid float literal"))?;
                if !value.is_finite() {
                    return Err(err(
                        node.span,
                        "E_TYPE",
                        "floating-point literal exceeds f64",
                    ));
                }
                let dst = self.temp(k::F64);
                self.emit(
                    k::Op::Float {
                        dst,
                        bits: value.to_bits(),
                    },
                    line,
                );
                Ok((dst, k::F64))
            }
            a::Expr::Path(path) => {
                if self.binding(&path.name).is_some() {
                    let place = self.place(id)?;
                    return self.read(place, line);
                }
                let path = self
                    .compiler
                    .names
                    .value_path(node.scope, &path, node.span)?;
                if let Some(a::Item::Constant(index)) = self.compiler.declaration_item(&path) {
                    let constant = self.compiler.source.constants[index].clone();
                    let t = self.compiler.resolve(constant.ty, &self.sub)?;
                    return self.expr(constant.value, Some(t));
                }
                if let Some(value) =
                    self.construct_variant(&path, &[], a::VariantStyle::Unit, expected, node.span)?
                {
                    return Ok(value);
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
                let place = if matches!(
                    self.compiler.source.exprs[value].kind,
                    a::Expr::Path(_)
                        | a::Expr::Field { .. }
                        | a::Expr::Index { .. }
                        | a::Expr::Unary { .. }
                ) {
                    self.place(value)?
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
                let (src, ty) = self.expr(value, expected)?;
                let dst = self.temp(ty);
                self.emit(k::Op::Unary { dst, op, src }, line);
                Ok((dst, ty))
            }
            a::Expr::Binary { op, left, right } => {
                if op == "&&" || op == "||" {
                    return self.short_circuit(&op, left, right, line);
                }
                if op == "==" || op == "!=" {
                    return self.equality_expr(&op, left, right, node.span);
                }
                if ["==", "!=", "<", "<=", ">", ">="].contains(&op.as_str()) {
                    if self.compiler.contracts.is_some() {
                        if let Some(value) = self.generic_comparison(&op, left, right, node.span)? {
                            return Ok(value);
                        }
                    }
                    if let Some(t) = self.hint(left).or_else(|| self.hint(right)) {
                        if let k::TypeKind::Vec { inner } = self.compiler.types[t].kind {
                            return self.vector_compare(&op, left, right, inner, node.span);
                        }
                    }
                }
                let comparison = ["==", "!=", "<", "<=", ">", ">="].contains(&op.as_str());
                let operand = self
                    .hint(left)
                    .or_else(|| self.hint(right))
                    .or(if comparison { None } else { expected });
                let (left, ty) = self.expr(left, operand)?;
                let (right, rt) = self.expr(right, Some(ty))?;
                if ty != rt {
                    return Err(err(
                        node.span,
                        "E_TYPE",
                        &format!(
                            "binary operand types differ: {} and {}",
                            self.compiler.types[ty].name, self.compiler.types[rt].name
                        ),
                    ));
                }
                let result = if comparison { k::BOOL } else { ty };
                let dst = self.temp(result);
                self.emit(
                    k::Op::Binary {
                        dst,
                        op,
                        left,
                        right,
                    },
                    line,
                );
                Ok((dst, result))
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
            a::Expr::Call { callee, args } => self.call(callee, &args, expected, node.span),
            a::Expr::Method {
                receiver,
                name,
                types,
                args,
            } => self.method(id, receiver, &name, &types, &args, node.span),
            a::Expr::Struct { path, fields, base } => {
                if base.is_some() {
                    return Err(err(
                        node.span,
                        "E_UNSUPPORTED",
                        "struct update is not implemented yet",
                    ));
                }
                self.construct_struct(&path, &fields, expected, node.span)
            }
            a::Expr::Array(elements) => self.array_expr(&elements, expected, node.span),
            a::Expr::Repeat { value, count } => {
                let length = match &self.compiler.source.exprs[count].kind {
                    a::Expr::Int(text) => text.replace('_', "").parse::<usize>().map_err(|_| {
                        err(
                            node.span,
                            "E_TYPE",
                            "array repeat length must be an integer literal",
                        )
                    })?,
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
                for _ in 0..length {
                    elements.push(self.read(k::Place::local(v.0), line)?.0);
                }
                let ty = self.compiler.intern(
                    format!("[{};{}]", self.compiler.types[v.1].name, length),
                    k::TypeKind::Array {
                        inner: v.1,
                        len: length,
                    },
                );
                let dst = self.temp(ty);
                self.emit(k::Op::Array { dst, elements }, line);
                if self.live[v.0] {
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
                    let v = self.expr(*item, None)?;
                    fields.push(k::Field {
                        name: i.to_string(),
                        ty: v.1,
                    });
                    values.push(v.0);
                }
                let names = fields
                    .iter()
                    .map(|f| self.compiler.types[f.ty].name.clone())
                    .collect::<Vec<_>>()
                    .join(",");
                let copy = fields.iter().all(|f| is_copy(&self.compiler.types, f.ty));
                let ty = self
                    .compiler
                    .intern(format!("({names})"), k::TypeKind::Struct { fields, copy });
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
            a::Expr::Block(block) => self.block(block, expected),
            a::Expr::If {
                condition,
                then_block,
                otherwise,
            } => self.if_expr(condition, then_block, otherwise, expected, line),
            a::Expr::While { condition, body } => self.while_expr(Some(condition), body, line),
            a::Expr::Loop { body } => self.while_expr(None, body, line),
            a::Expr::For {
                pattern,
                iterator,
                body,
            } => self.for_expr(pattern, iterator, body, line),
            a::Expr::Match { value, arms } => self.match_expr(value, &arms, expected, node.span),
            a::Expr::Return(value) => {
                let result = if let Some(v) = value {
                    self.expr(v, Some(self.function.result))?
                } else {
                    self.unit(line)
                };
                self.cleanup(0, Some(result.0), line);
                self.emit(k::Op::Return { src: result.0 }, line);
                Ok(result)
            }
            a::Expr::Break(value) => {
                if value.is_some() {
                    return Err(err(
                        node.span,
                        "E_UNSUPPORTED",
                        "break values are not implemented yet",
                    ));
                }
                let start = self
                    .loops
                    .last()
                    .ok_or_else(|| err(node.span, "E_TYPE", "break outside loop"))?
                    .start;
                self.cleanup(start, None, line);
                self.emit(k::Op::Break, line);
                Ok(self.unit(line))
            }
            a::Expr::Continue => {
                let start = self
                    .loops
                    .last()
                    .ok_or_else(|| err(node.span, "E_TYPE", "continue outside loop"))?
                    .start;
                self.advance_range(line)?;
                self.cleanup(start, None, line);
                self.emit(k::Op::Continue, line);
                Ok(self.unit(line))
            }
            a::Expr::Macro {
                name,
                args,
                separator,
            } => self.macro_expr(&name, &args, &separator, expected, node.span),
            a::Expr::Closure { .. } => Err(err(
                node.span,
                "E_UNSUPPORTED",
                "closures are not implemented yet",
            )),
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
        let ty = self.compiler.intern(
            format!("[{};{}]", self.compiler.types[inner].name, elements.len()),
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
    fn vector_compare(
        &mut self,
        op: &str,
        left: usize,
        right: usize,
        inner: usize,
        span: a::Span,
    ) -> Result<(usize, usize)> {
        let lp = self.addressable(left)?;
        let l = self.borrow(lp, false, span.line)?.0;
        let rp = self.addressable(right)?;
        let r = self.borrow(rp, false, span.line)?.0;
        self.vector_compare_values(op, l, r, inner, span)
    }
    fn vector_compare_values(
        &mut self,
        op: &str,
        l: usize,
        r: usize,
        inner: usize,
        span: a::Span,
    ) -> Result<(usize, usize)> {
        if !matches!(
            self.compiler.types[inner].kind,
            k::TypeKind::Int { .. } | k::TypeKind::Bool
        ) {
            return Err(err(
                span,
                "E_UNSUPPORTED",
                "vector ordering currently requires integer or bool elements",
            ));
        }
        let index = self
            .compiler
            .names
            .sequence_function("__lkj_vec_compare", span)?;
        let mut sub = BTreeMap::new();
        sub.insert("T".to_string(), inner);
        let function = self.compiler.instantiate(index, sub)?;
        let comparison = self.temp(k::I32);
        self.emit(
            k::Op::Call {
                dst: comparison,
                function,
                args: vec![l, r],
            },
            span.line,
        );
        let zero = self.temp(k::I32);
        self.emit(
            k::Op::Int {
                dst: zero,
                value: 0,
            },
            span.line,
        );
        let dst = self.temp(k::BOOL);
        self.emit(
            k::Op::Binary {
                dst,
                op: op.to_string(),
                left: comparison,
                right: zero,
            },
            span.line,
        );
        Ok((dst, k::BOOL))
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
        expected: Option<usize>,
        line: usize,
    ) -> Result<(usize, usize)> {
        let (condition, _) = self.expr(condition, Some(k::BOOL))?;
        self.emit(k::Op::If { condition }, line);
        let state = self.snapshot();
        let before = self.live.len();
        let bindings = self.bindings.len();
        let first = self.block(then_block, expected)?;
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
            self.expr(
                other,
                if first_reachable {
                    Some(self.ty(result))
                } else {
                    expected
                },
            )?
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
    fn advance_range(&mut self, line: usize) -> Result<()> {
        let (start, range) = match self.loops.last() {
            Some(scope) => (scope.start, scope.range),
            None => return Ok(()),
        };
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
                self.emit(k::Op::Break, line);
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
        line: usize,
    ) -> Result<(usize, usize)> {
        let start = self.live.len();
        let entry = self.snapshot();
        self.loops.push(LoopScope {
            start,
            range: None,
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
        Ok(self.unit(line))
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
            start: first,
            range: Some((counter, finish, inclusive)),
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
        let k::TypeKind::Enum { variants, .. } = &self.compiler.types[ty].kind else {
            unreachable!()
        };
        let tag = variants
            .iter()
            .position(|variant| variant.name == name)
            .ok_or_else(|| err(span, "E_TYPE", "unknown enum variant"))?;
        Ok((ty, tag))
    }
    fn call(
        &mut self,
        callee: usize,
        args: &[usize],
        expected: Option<usize>,
        span: a::Span,
    ) -> Result<(usize, usize)> {
        let path = match self.compiler.source.exprs[callee].kind.clone() {
            a::Expr::Path(p) if self.binding(&p.name).is_some() => {
                return Err(err(
                    span,
                    "E_UNSUPPORTED",
                    "calling a local function value is not implemented",
                ));
            }
            a::Expr::Path(p) => self.compiler.names.value_path(
                self.compiler.source.exprs[callee].scope,
                &p,
                span,
            )?,
            _ => {
                return Err(err(
                    span,
                    "E_UNSUPPORTED",
                    "indirect calls are not implemented yet",
                ));
            }
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
                let t = self.compiler.intern(
                    format!("Box<{}>", self.compiler.types[value.1].name),
                    k::TypeKind::Box { inner: value.1 },
                );
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
        let fields: Vec<_> = args
            .iter()
            .enumerate()
            .map(|(index, expression)| (index.to_string(), *expression))
            .collect();
        if let Some(value) =
            self.construct_variant(&path, &fields, a::VariantStyle::Tuple, expected, span)?
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
            let ty = self.compiler.named_type(&path, &self.sub, span)?;
            let fields = match &self.compiler.types[ty].kind {
                k::TypeKind::Struct { fields, .. } => fields.clone(),
                _ => Vec::new(),
            };
            if fields.len() != args.len() {
                return Err(err(span, "E_TYPE", "tuple struct field count differs"));
            }
            let mut values = Vec::new();
            for i in 0..args.len() {
                self.compiler
                    .declared_field_access(&s, &s.fields[i], self.scope, span)?;
                values.push(self.expr(args[i], Some(fields[i].ty))?.0);
            }
            let dst = self.temp(ty);
            self.emit(
                k::Op::Struct {
                    dst,
                    fields: values,
                },
                s.span.line,
            );
            return Ok((dst, ty));
        }
        let mut initial_sub = BTreeMap::new();
        let mut associated = None;
        if let Some((owner, method)) = path.name.rsplit_once("::") {
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
                let impls = self.compiler.source.impls.clone();
                for im in impls {
                    let mut sub = BTreeMap::new();
                    if !self.infer_type_matches(im.target, t, &mut sub)? {
                        continue;
                    }
                    sub.insert("Self".to_string(), t);
                    if self.compiler.resolve(im.target, &sub).ok() == Some(t) {
                        if !self.trait_available(&im, t) {
                            continue;
                        }
                        self.compiler.bounds(&im.generics, &sub)?;
                        for candidate in im.methods {
                            let f = &self.compiler.source.functions[candidate];
                            if f.name == method && f.params.first().is_none_or(|p| p.name != "self")
                            {
                                if associated.is_some() {
                                    return Err(err(
                                        span,
                                        "E_TRAIT",
                                        "ambiguous associated function",
                                    ));
                                }
                                associated = Some(candidate);
                                initial_sub = sub.clone();
                            }
                        }
                    }
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
        if associated.is_some() {
            self.compiler.method_access(index, self.scope, span)?;
        }
        let target = self.compiler.source.functions[index].clone();
        if target.params.len() != args.len() {
            return Err(err(span, "E_TYPE", "function argument count differs"));
        }
        let mut sub = initial_sub;
        let generic: Vec<_> = target.generics.iter().filter(|g| !g.lifetime).collect();
        if !path.args.is_empty() && associated.is_none() {
            if path.args.len() != generic.len() {
                return Err(err(span, "E_TYPE", "wrong generic argument count"));
            }
            for i in 0..generic.len() {
                sub.insert(
                    generic[i].name.clone(),
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
            let parameter = self.compiler.resolve(target.params[i].ty, &sub).ok();
            let value = self.expr(args[i], parameter)?;
            self.infer_type(target.params[i].ty, value.1, &mut sub)?;
            values.push(value.0);
        }
        let function = self.compiler.instantiate(index, sub)?;
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
    fn has_declared_method(&mut self, base: usize, name: &str) -> Result<bool> {
        let implementations = self.compiler.source.impls.clone();
        for implementation in implementations {
            if !implementation
                .methods
                .iter()
                .any(|method| self.compiler.source.functions[*method].name == name)
            {
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
                && self.trait_available(&implementation, base)
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
            for implementation in self.compiler.source.impls.clone() {
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
        if let Some(value) = self.generic_method(expression, &target, base, name, _types, args)? {
            return Ok(value);
        }
        if matches!(self.compiler.types[base].kind, k::TypeKind::Str)
            && matches!(name, "len" | "is_empty" | "as_bytes" | "to_string")
        {
            if !_types.is_empty() {
                return Err(err(
                    span,
                    "E_TYPE",
                    "text methods take no generic arguments",
                ));
            }
            return self.text_method(target, name, args, span);
        }
        if matches!(
            self.compiler.types[base].kind,
            k::TypeKind::World | k::TypeKind::Stdout
        ) && matches!(name, "stdout" | "write")
        {
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
        match name {
            "as_slice" | "as_mut_slice"
                if matches!(
                    self.compiler.types[base].kind,
                    k::TypeKind::Vec { .. } | k::TypeKind::Array { .. }
                ) =>
            {
                if !args.is_empty() {
                    return Err(err(span, "E_TYPE", "slice projection takes no arguments"));
                }
                let inner = match self.compiler.types[base].kind {
                    k::TypeKind::Vec { inner } | k::TypeKind::Array { inner, .. } => inner,
                    _ => return Err(err(span, "E_TYPE", "slice projection needs a collection")),
                };
                let slice = self.compiler.intern(
                    format!("[{}]", self.compiler.types[inner].name),
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
            "len"
                if matches!(
                    self.compiler.types[base].kind,
                    k::TypeKind::Vec { .. } | k::TypeKind::Slice { .. } | k::TypeKind::Array { .. }
                ) =>
            {
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
            "push" if matches!(self.compiler.types[base].kind, k::TypeKind::Vec { .. }) => {
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
            "pop" if matches!(self.compiler.types[base].kind, k::TypeKind::Vec { .. }) => {
                if !args.is_empty() {
                    return Err(err(span, "E_TYPE", "pop takes no arguments"));
                }
                let inner = match self.compiler.types[base].kind {
                    k::TypeKind::Vec { inner } => inner,
                    _ => return Err(err(span, "E_TYPE", "pop requires Vec")),
                };
                let mut sub = BTreeMap::new();
                sub.insert("T".to_string(), inner);
                let ty = self.compiler.named_type(
                    &a::Path {
                        declaration: None,
                        name: self.compiler.names.prelude_type("Option", span)?,
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
                let mut candidates = Vec::new();
                let mut inherent_candidates = Vec::new();
                let mut unavailable_trait = false;
                let mut private_trait = false;
                let impls = self.compiler.source.impls.clone();
                for im in impls {
                    let mut sub = BTreeMap::new();
                    if !self.infer_type_matches(im.target, base, &mut sub)? {
                        continue;
                    }
                    sub.insert("Self".to_string(), base);
                    if let Ok(t) = self.compiler.resolve(im.target, &sub) {
                        if t == base {
                            if !im
                                .methods
                                .iter()
                                .any(|m| self.compiler.source.functions[*m].name == name)
                            {
                                continue;
                            }
                            if !self.trait_available(&im, base) {
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
                            self.compiler.bounds(&im.generics, &sub)?;
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
                        "method resolution requires one implementation",
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
        expected: Option<usize>,
        span: a::Span,
    ) -> Result<(usize, usize)> {
        if let Some(value) =
            self.construct_variant(path, fields, a::VariantStyle::Named, expected, span)?
        {
            return Ok(value);
        }
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
            let index = definition
                .fields
                .iter()
                .position(|field| field.name == *name)
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
        if supplied.iter().any(|supplied| !supplied) {
            return Err(err(span, "E_TYPE", "struct constructor is missing fields"));
        }
        let mut coercion_substitutions = target.substitutions.clone();
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
        let ty = self.finish_constructor(target, span)?;
        let k::TypeKind::Struct { fields, .. } = &self.compiler.types[ty].kind else {
            unreachable!()
        };
        let mut values = Vec::new();
        for (value, field) in ordered.into_iter().zip(fields) {
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
                let t = self.hint(args[0]).or_else(|| self.hint(args[1]));
                let (left, ty) = self.expr(args[0], t)?;
                let (right, _) = self.expr(args[1], Some(ty))?;
                let condition = self.temp(k::BOOL);
                self.emit(
                    k::Op::Binary {
                        dst: condition,
                        op: if name == "assert_eq" { "==" } else { "!=" }.to_string(),
                        left,
                        right,
                    },
                    span.line,
                );
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
                if separator == ";" {
                    return Err(err(
                        span,
                        "E_UNSUPPORTED",
                        "repeated Vec literals are not implemented yet",
                    ));
                }
                let inner = expected.and_then(|t| match self.compiler.types[t].kind {
                    k::TypeKind::Vec { inner } => Some(inner),
                    _ => None,
                });
                let mut values = Vec::new();
                let mut inferred = inner;
                for arg in args {
                    let value = self.expr(*arg, inferred)?;
                    inferred = Some(value.1);
                    values.push(value.0);
                }
                let inner =
                    inferred.ok_or_else(|| err(span, "E_TYPE", "empty Vec needs a type"))?;
                let ty = self.compiler.intern(
                    format!("Vec<{}>", self.compiler.types[inner].name),
                    k::TypeKind::Vec { inner },
                );
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
        expected: Option<usize>,
        span: a::Span,
    ) -> Result<(usize, usize)> {
        let hint = if let Some(hint) = self.hint(value) {
            Some(hint)
        } else {
            self.pattern_hint(arms).map(|inner| {
                if let a::Expr::Borrow { mutable, .. } = self.compiler.source.exprs[value].kind {
                    self.compiler.reference(inner, mutable)
                } else {
                    inner
                }
            })
        };
        let existing = match &self.compiler.source.exprs[value].kind {
            a::Expr::Path(path) => self.binding(&path.name).is_some(),
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
        let original_ty = place_type(&self.compiler.types, self.function, &original)
            .ok_or_else(|| err(span, "E_TYPE", "invalid match scrutinee place"))?;
        let (ty, borrowed, mutable) = match self.compiler.types[original_ty].kind {
            k::TypeKind::Ref { inner, mutable } => (inner, true, mutable),
            _ => (original_ty, false, false),
        };
        let (patterns, exhaustive) = self.match_patterns(ty, arms)?;
        if !exhaustive {
            return Err(err(span, "E_TYPE", "match is not exhaustive"));
        }
        let moving = self.match_moves(original_ty, ty, borrowed, arms, &patterns);
        let (explicit_borrow, explicit_mutable) = self.explicit_match_borrows(arms);
        if moving && explicit_borrow && existing {
            return Err(err(
                span,
                "E_UNSUPPORTED",
                "mixed affine moves and explicit ref match bindings need branch-specific place contracts",
            ));
        }
        let (scrutinee, field_base, default_base, cleanup_scrutinee) = if existing && !moving {
            let mut data = original.clone();
            if borrowed {
                data.projections.push(k::Projection::Deref);
            }
            if original.projections.is_empty() {
                (original.local, data, original.clone(), false)
            } else {
                let reference = self
                    .borrow(data, explicit_mutable || borrowed && mutable, span.line)?
                    .0;
                let data = k::Place {
                    local: reference,
                    projections: vec![k::Projection::Deref],
                };
                let default = if borrowed {
                    original.clone()
                } else {
                    data.clone()
                };
                (reference, data, default, true)
            }
        } else {
            let local = if existing {
                self.read(original, span.line)?.0
            } else {
                original.local
            };
            let mut data = k::Place::local(local);
            if borrowed {
                data.projections.push(k::Projection::Deref);
            }
            (local, data, k::Place::local(local), true)
        };
        let start = self.live.len();
        let base = self.snapshot();
        let binding_count = self.bindings.len();
        let mut result: Option<usize> = None;
        let mut exit_state: Option<Snapshot> = None;
        let mut open = 0;
        let mut default_seen = false;
        for (index, arm) in arms.iter().enumerate() {
            let pattern = self.compiler.source.patterns[arm.pattern].clone();
            let mut fields = Vec::new();
            let mut binding_default = false;
            let condition = match pattern.kind {
                a::Pattern::Wild | a::Pattern::Bind { .. } => {
                    binding_default = matches!(pattern.kind, a::Pattern::Bind { .. });
                    if index + 1 == arms.len() {
                        default_seen = true;
                        None
                    } else {
                        let condition = self.temp(k::BOOL);
                        self.emit(
                            k::Op::Int {
                                dst: condition,
                                value: 1,
                            },
                            arm.span.line,
                        );
                        Some(condition)
                    }
                }
                a::Pattern::Variant { .. } | a::Pattern::Struct { .. } => {
                    let tag = patterns[index].tag.expect("validated variant");
                    fields = patterns[index].fields.clone();
                    let c = self.temp(k::BOOL);
                    self.emit(
                        k::Op::TagTest {
                            dst: c,
                            src: scrutinee,
                            variant: tag,
                        },
                        span.line,
                    );
                    Some(c)
                }
                a::Pattern::Path(_) => {
                    let tag = patterns[index].tag.expect("validated variant");
                    let c = self.temp(k::BOOL);
                    self.emit(
                        k::Op::TagTest {
                            dst: c,
                            src: scrutinee,
                            variant: tag,
                        },
                        span.line,
                    );
                    Some(c)
                }
                a::Pattern::Literal(expr) => {
                    let l = self.read(field_base.clone(), span.line)?.0;
                    let r = self.expr(expr, Some(ty))?.0;
                    let c = self.temp(k::BOOL);
                    self.emit(
                        k::Op::Binary {
                            dst: c,
                            op: "==".to_string(),
                            left: l,
                            right: r,
                        },
                        span.line,
                    );
                    Some(c)
                }
                a::Pattern::Range { .. } => Some(self.range_condition(
                    field_base.clone(),
                    ty,
                    patterns[index].range.expect("validated range"),
                    arm.span.line,
                )?),
                _ => {
                    return Err(err(
                        pattern.span,
                        "E_UNSUPPORTED",
                        "this match pattern is not implemented yet",
                    ));
                }
            };
            if let Some(c) = condition {
                self.emit(k::Op::If { condition: c }, span.line);
                open += 1;
            }
            if binding_default {
                self.bind_match_place(arm.pattern, default_base.clone(), None, arm.span.line)?;
            }
            for (pattern, projection) in fields {
                let mut place = field_base.clone();
                place.projections.push(projection);
                self.bind_match_place(
                    pattern,
                    place,
                    if borrowed { Some(mutable) } else { None },
                    span.line,
                )?;
            }
            let v = self.expr(arm.value, expected.or(result.map(|i| self.ty(i))))?;
            if self.reachable {
                let target = if let Some(r) = result {
                    r
                } else {
                    let d = self.local(expected.unwrap_or(v.1), "$match", true);
                    result = Some(d);
                    d
                };
                self.emit(
                    k::Op::Assign {
                        place: k::Place::local(target),
                        src: v.0,
                    },
                    span.line,
                );
                self.cleanup(start, Some(target), span.line);
                if cleanup_scrutinee && self.live[scrutinee] {
                    self.emit(
                        k::Op::Drop {
                            place: k::Place::local(scrutinee),
                        },
                        span.line,
                    );
                }
                if let Some(exit) = &exit_state {
                    self.merge_carriers(exit);
                }
                exit_state = Some(self.snapshot());
            }
            self.bindings.truncate(binding_count);
            if condition.is_some() {
                self.emit(k::Op::Else, span.line);
                self.restore(&base);
            } else {
                break;
            }
        }
        if open > 0 {
            if !default_seen {
                self.emit(
                    k::Op::Trap {
                        message: "unreachable match".to_string(),
                    },
                    span.line,
                );
            }
            for _ in 0..open {
                self.emit(k::Op::End, span.line);
            }
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
