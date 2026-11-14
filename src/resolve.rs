//! Module and declaration identity. Later phases consume resolved paths, never bare-name guesses.
mod import_scopes;
mod iteration;
pub(crate) mod recovery;
use crate::ast::*;
use crate::names::qualified;
use import_scopes::ImportScopes;
use std::collections::{BTreeMap, BTreeSet};

type Result<T> = std::result::Result<T, Diagnostic>;
#[derive(Clone)]
struct Binding {
    declaration: DeclarationId,
    target: String,
    scope: ModuleId,
    public: Visibility,
    glob: bool,
    ambiguous: bool,
}
pub(crate) struct ImportFailure {
    pub import: usize,
    pub problem: Diagnostic,
}

struct Resolved {
    name: String,
    binding: Option<Binding>,
}
impl Resolved {
    fn intrinsic(name: String) -> Self {
        Self {
            name,
            binding: None,
        }
    }
    fn item(binding: Binding) -> Self {
        Self {
            name: binding.target.clone(),
            binding: Some(binding),
        }
    }
}
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Namespace {
    Type,
    Value,
}
pub struct Resolver<'a> {
    program: &'a Program,
    declarations: Vec<Declaration>,
    names: BTreeMap<(ModuleId, String, Namespace), Binding>,
    modules: BTreeMap<String, ModuleId>,
    alternatives: BTreeMap<(ModuleId, String, Namespace), Vec<Binding>>,
    generic_ranges: Vec<(Span, Vec<String>)>,
    import_scopes: ImportScopes,
    trait_names: BTreeMap<(usize, DeclarationId), BTreeSet<String>>,
    anonymous_traits: BTreeSet<(usize, DeclarationId)>,
    // Only an unresolved input needs a newly built relation. Resolved inputs
    // retain the program's authoritative table without constructing another.
    function_owners: Option<Vec<FunctionOwner>>,
}

fn first_name(path: &str) -> &str {
    match path.find("::") {
        Some(at) => &path[..at],
        None => path,
    }
}
fn last_name(path: &str) -> &str {
    match path.rfind("::") {
        Some(at) => &path[at + 2..],
        None => path,
    }
}
fn path_parts(path: &str) -> Vec<String> {
    let mut parts: Vec<String> = Vec::new();
    for part in path.split("::") {
        parts.push(part.to_string());
    }
    parts
}
fn join_parts(parts: &[String]) -> String {
    let mut result = String::new();
    for i in 0..parts.len() {
        if i > 0 {
            result.push_str("::");
        }
        result.push_str(&parts[i]);
    }
    result
}
fn within(program: &Program, mut scope: ModuleId, ancestor: ModuleId) -> bool {
    loop {
        if scope == ancestor {
            return true;
        }
        if let Some(parent) = program.scopes[scope].parent {
            scope = parent;
        } else {
            return false;
        }
    }
}
fn region(program: &Program, scope: ModuleId, visibility: Visibility) -> Option<ModuleId> {
    match visibility {
        Visibility::Public => None,
        Visibility::Crate => Some(program.scopes[scope].crate_root),
        Visibility::Parent => program.scopes[scope].parent,
        Visibility::Private => Some(scope),
        Visibility::Restricted(ancestor) => Some(ancestor),
    }
}
pub fn visible(program: &Program, owner: ModuleId, visibility: Visibility, from: ModuleId) -> bool {
    region(program, owner, visibility).is_none_or(|root| within(program, from, root))
}
pub fn module(program: &Program, name: &str) -> ModuleId {
    program
        .scopes
        .iter()
        .position(|s| s.path == name)
        .unwrap_or(0)
}
fn builtin(name: &str) -> bool {
    matches!(
        name,
        "bool"
            | "u8"
            | "u16"
            | "u32"
            | "u64"
            | "usize"
            | "i8"
            | "i16"
            | "i32"
            | "i64"
            | "isize"
            | "f64"
            | "str"
            | "char"
            | "Vec"
            | "Box"
            | "World"
            | "Stdout"
            | "Stdin"
            | "Stderr"
            | "Directory"
            | "File"
            | "Copy"
            | "Clone"
            | "Ord"
            | "Eq"
            | "PartialEq"
            | "PartialOrd"
            | "Send"
            | "Sync"
            | "Data"
            | "Drop"
            | "Fn"
            | "FnMut"
            | "FnOnce"
    )
}
fn insert_function_owner(
    owners: &mut [FunctionOwner],
    function: FunctionId,
    owner: FunctionOwner,
    span: Span,
) -> Result<()> {
    let slot = owners.get_mut(function.index).ok_or_else(|| {
        Diagnostic::new(span, "E_NAME", "method owner refers to an unknown function")
    })?;
    if *slot != FunctionOwner::Free {
        return Err(Diagnostic::new(
            span,
            "E_NAME",
            "a function cannot have more than one method owner",
        ));
    }
    *slot = owner;
    Ok(())
}

fn index_function_owners(program: &Program) -> Result<Vec<FunctionOwner>> {
    let mut owners = vec![FunctionOwner::Free; program.functions.len()];
    for (index, implementation) in program.impls.iter().enumerate() {
        for function in &implementation.methods {
            insert_function_owner(
                &mut owners,
                FunctionId { index: *function },
                FunctionOwner::Implementation(ImplId { index }),
                implementation.span,
            )?;
        }
    }
    for (index, declaration) in program.traits.iter().enumerate() {
        for function in &declaration.methods {
            insert_function_owner(
                &mut owners,
                FunctionId { index: *function },
                FunctionOwner::Trait(TraitId { index }),
                declaration.span,
            )?;
        }
    }
    Ok(owners)
}

impl<'a> Resolver<'a> {
    pub fn new(program: &'a Program) -> Result<Self> {
        let (resolver, problems) = Self::new_report(program)?;
        if let Some(failure) = problems.into_iter().next() {
            Err(failure.problem)
        } else {
            Ok(resolver)
        }
    }
    pub(crate) fn new_report(program: &'a Program) -> Result<(Self, Vec<ImportFailure>)> {
        Self::build_report(program, false)
    }
    pub(crate) fn new_recovery(program: &'a Program) -> Result<(Self, Vec<ImportFailure>)> {
        Self::build_report(program, true)
    }
    fn build_report(
        program: &'a Program,
        unloaded_modules: bool,
    ) -> Result<(Self, Vec<ImportFailure>)> {
        if program.resolved && program.function_owners.len() != program.functions.len() {
            return Err(Diagnostic::new(
                Span::default(),
                "E_NAME",
                "resolved source is missing its function ownership relation",
            ));
        }
        let function_owners = if program.resolved {
            None
        } else {
            Some(index_function_owners(program)?)
        };
        let mut result = Resolver {
            program,
            declarations: Vec::new(),
            names: BTreeMap::new(),
            modules: BTreeMap::new(),
            alternatives: BTreeMap::new(),
            generic_ranges: Vec::new(),
            import_scopes: ImportScopes::default(),
            trait_names: BTreeMap::new(),
            anonymous_traits: BTreeSet::new(),
            function_owners,
        };
        for (id, scope) in program.scopes.iter().enumerate() {
            result.modules.insert(scope.path.clone(), id);
        }
        result.import_scopes = ImportScopes::new(program, &result.modules)?;
        for item in &program.impls {
            result.generics(item.span, &item.generics);
        }
        for (id, item) in program.functions.iter().enumerate() {
            result.generics(item.span, &item.generics);
            if result.function_owner(FunctionId { index: id }) == FunctionOwner::Free {
                result.declare(
                    &item.name,
                    &item.module,
                    Item::Function(id),
                    item.public,
                    item.span,
                    (false, true),
                )?;
            }
        }
        for (id, item) in program.structs.iter().enumerate() {
            result.generics(item.span, &item.generics);
            result.declare(
                &item.name,
                &item.module,
                Item::Struct(id),
                item.public,
                item.span,
                (true, item.tuple || item.fields.is_empty()),
            )?;
        }
        for (id, item) in program.enums.iter().enumerate() {
            result.generics(item.span, &item.generics);
            result.declare(
                &item.name,
                &item.module,
                Item::Enum(id),
                item.public,
                item.span,
                (true, false),
            )?;
        }
        for (id, item) in program.traits.iter().enumerate() {
            result.generics(item.span, &item.generics);
            result.declare(
                &item.name,
                &item.module,
                Item::Trait(id),
                item.public,
                item.span,
                (true, false),
            )?;
        }
        for (id, item) in program.aliases.iter().enumerate() {
            result.generics(item.span, &item.generics);
            result.declare(
                &item.name,
                &item.module,
                Item::Alias(id),
                item.public,
                item.span,
                (true, false),
            )?;
        }
        for (id, item) in program.constants.iter().enumerate() {
            result.declare(
                &item.name,
                &item.module,
                Item::Constant(id),
                item.public,
                item.span,
                (false, true),
            )?;
        }
        for (id, item) in program.modules.iter().enumerate() {
            if item.external && !unloaded_modules {
                return Err(Diagnostic::new(
                    item.span,
                    "E_UNSUPPORTED",
                    "external modules require a source bundle or the normal CLI",
                ));
            }
            result.declare(
                &item.name,
                &item.parent,
                Item::Module(id),
                item.public,
                item.span,
                (true, false),
            )?;
        }
        let problems = result.imports();
        Ok((result, problems))
    }
    fn function_owner(&self, function: FunctionId) -> FunctionOwner {
        match &self.function_owners {
            Some(owners) => owners[function.index],
            None => self.program.function_owner(function),
        }
    }
    fn generics(&mut self, span: Span, params: &[GenericParam]) {
        if !params.is_empty() {
            let mut names = Vec::new();
            for param in params {
                names.push(param.name.clone());
            }
            self.generic_ranges.push((span, names));
        }
    }
    fn generic(&self, span: Span, name: &str) -> bool {
        if name == "Self" || name.starts_with('\'') {
            return true;
        }
        self.generic_ranges.iter().any(|(range, names)| {
            range.file == span.file
                && range.start <= span.start
                && span.end <= range.end
                && names.iter().any(|n| n == name)
        })
    }
    fn declare(
        &mut self,
        name: &str,
        path: &str,
        item: Item,
        public: Visibility,
        span: Span,
        namespaces: (bool, bool),
    ) -> Result<()> {
        let (ty, value) = namespaces;
        let scope = *self.modules.get(&path.to_string()).unwrap();
        let declaration = self.declarations.len();
        self.declarations.push(Declaration {
            name: qualified(path, name),
            scope,
            item,
            public,
            span,
        });
        if ty && matches!(item, Item::Trait(_)) {
            let key = (scope, declaration);
            if !self.trait_names.contains_key(&key) {
                self.trait_names.insert(key, BTreeSet::new());
            }
            self.trait_names
                .get_mut(&key)
                .unwrap()
                .insert(name.to_string());
        }
        for index in 0..2 {
            let enabled = if index == 0 { ty } else { value };
            let namespace = if index == 0 {
                Namespace::Type
            } else {
                Namespace::Value
            };
            if enabled {
                let key = (scope, name.to_string(), namespace);
                if self
                    .names
                    .insert(
                        key,
                        Binding {
                            declaration,
                            target: qualified(path, name),
                            scope,
                            public,
                            glob: false,
                            ambiguous: false,
                        },
                    )
                    .is_some()
                {
                    return Err(Diagnostic::new(
                        span,
                        "E_NAME",
                        &format!("duplicate declaration: {}", qualified(path, name)),
                    ));
                }
            }
        }
        Ok(())
    }
    fn access(&self, binding: &Binding, from: ModuleId, span: Span) -> Result<()> {
        if binding.ambiguous {
            return Err(Diagnostic::new(
                span,
                "E_NAME",
                &format!("ambiguous glob import {}", last_name(&binding.target)),
            ));
        }
        if !visible(self.program, binding.scope, binding.public, from) {
            return Err(Diagnostic::new(
                span,
                "E_PRIVACY",
                &format!("{} is private", self.declarations[binding.declaration].name),
            ));
        }
        Ok(())
    }
    fn lookup(&self, scope: ModuleId, name: &str, namespace: Namespace) -> Option<Binding> {
        self.names
            .get(&(scope, name.to_string(), namespace))
            .cloned()
    }
    fn lookup_owner(
        &self,
        scope: ModuleId,
        name: &str,
        namespace: Namespace,
        span: Span,
    ) -> Option<usize> {
        let mut owner = self.import_scopes.at(scope, span);
        loop {
            if self
                .names
                .contains_key(&(owner, name.to_string(), namespace))
            {
                return Some(owner);
            }
            owner = self.import_scopes.parent(owner)?;
        }
    }
    fn lookup_lexical(
        &self,
        scope: ModuleId,
        name: &str,
        namespace: Namespace,
        span: Span,
    ) -> Option<Binding> {
        self.lookup_owner(scope, name, namespace, span)
            .and_then(|owner| {
                self.names
                    .get(&(owner, name.to_string(), namespace))
                    .cloned()
            })
    }
    fn path(
        &self,
        scope: ModuleId,
        path: &str,
        namespace: Namespace,
        span: Span,
        prelude: bool,
    ) -> Result<Option<Resolved>> {
        let parts = path_parts(path);
        if parts.is_empty() {
            return Ok(None);
        }
        let local_type = self
            .lookup_owner(scope, parts[0].as_str(), Namespace::Type, span)
            .is_some_and(|owner| owner >= self.program.scopes.len());
        if !local_type && self.generic(span, parts[0].as_str()) {
            return Ok(Some(Resolved::intrinsic(path.to_string())));
        }
        if matches!(
            path,
            "std::mem::replace"
                | "std::mem::drop"
                | "mem::replace"
                | "std::thread::scope"
                | "std::str::from_utf8"
                | "std::char::from_u32"
        ) && self
            .lookup_lexical(scope, parts[0].as_str(), Namespace::Type, span)
            .is_none()
        {
            return Ok(Some(Resolved::intrinsic(path.to_string())));
        }
        let mut at = 0;
        let mut current = scope;
        let mut rooted = false;
        if parts[0].as_str() == "crate" {
            current = self.program.scopes[scope].crate_root;
            at = 1;
            rooted = true;
        } else if parts[0].as_str() == "self" {
            at = 1;
            rooted = true;
        } else if parts[0].as_str() == "super" {
            while at < parts.len() && parts[at].as_str() == "super" {
                current = self.program.scopes[current]
                    .parent
                    .ok_or_else(|| Diagnostic::new(span, "E_NAME", "super has no parent module"))?;
                at += 1;
            }
            rooted = true;
        }
        if at == parts.len() {
            return Ok(None);
        }
        let first_namespace = if parts.len() - at > 1 {
            Namespace::Type
        } else {
            namespace
        };
        let mut binding = if rooted {
            self.lookup(current, parts[at].as_str(), first_namespace)
        } else {
            self.lookup_lexical(current, parts[at].as_str(), first_namespace, span)
        };
        if binding.is_none()
            && !rooted
            && parts[at].as_str() == "std"
            && self.modules.contains_key(&"$std".to_string())
        {
            current = *self.modules.get(&"$std".to_string()).unwrap();
            at += 1;
            rooted = true;
            if at == parts.len() {
                return Ok(None);
            }
            let remaining = join_parts(&parts[at..]);
            let intrinsic = match remaining.as_str() {
                "vec::Vec" => Some("Vec"),
                "boxed::Box" => Some("Box"),
                "mem::replace" => Some("std::mem::replace"),
                "mem::drop" => Some("std::mem::drop"),
                "str::from_utf8" => Some("std::str::from_utf8"),
                _ => None,
            };
            if let Some(name) = intrinsic {
                return Ok(Some(Resolved::intrinsic(name.to_string())));
            }
            if remaining.starts_with("vec::Vec::") {
                return Ok(Some(Resolved::intrinsic(remaining[5..].to_string())));
            }
            if remaining.starts_with("boxed::Box::") {
                return Ok(Some(Resolved::intrinsic(remaining[7..].to_string())));
            }
            binding = self.lookup(
                current,
                parts[at].as_str(),
                if parts.len() - at > 1 {
                    Namespace::Type
                } else {
                    namespace
                },
            );
        }
        if binding.is_none() && !rooted && prelude {
            if parts.len() == 1
                && namespace == Namespace::Value
                && matches!(parts[0].as_str(), "None" | "Some" | "Ok" | "Err")
            {
                let owner = if matches!(parts[0].as_str(), "None" | "Some") {
                    "Option"
                } else {
                    "Result"
                };
                if let Some(mut owner) = self.path(scope, owner, Namespace::Type, span, true)? {
                    owner.name = format!("{}::{}", owner.name, parts[0].as_str());
                    if let Some(binding) = &mut owner.binding {
                        binding.target = owner.name.clone();
                    }
                    return Ok(Some(owner));
                }
            }
            if let Some(std_scope) = self.modules.get(&"$std::prelude".to_string()) {
                binding = self.lookup(*std_scope, parts[at].as_str(), first_namespace);
            }
            if binding.is_none() && parts[0].as_str() == "String" {
                if let Some(std_scope) = self.modules.get(&"$std::string".to_string()) {
                    binding = self.lookup(*std_scope, "String", Namespace::Type);
                }
            }
            if binding.is_none()
                && (builtin(parts[0].as_str())
                    || matches!(
                        path,
                        "drop"
                            | "std::mem::replace"
                            | "mem::replace"
                            | "std::mem::drop"
                            | "scope"
                            | "std::thread::scope"
                            | "std::str::from_utf8"
                            | "std::char::from_u32"
                    ))
            {
                return Ok(Some(Resolved::intrinsic(path.to_string())));
            }
        }
        let mut binding = match binding {
            Some(b) => b,
            None => return Ok(None),
        };
        self.access(&binding, scope, span)?;
        at += 1;
        while at < parts.len() {
            let declaration = &self.declarations[binding.declaration];
            match declaration.item {
                Item::Module(index) => {
                    current = self.program.modules[index].scope;
                    let ns = if parts.len() - at > 1 {
                        Namespace::Type
                    } else {
                        namespace
                    };
                    binding = match self.lookup(current, parts[at].as_str(), ns) {
                        Some(b) => b,
                        None => return Ok(None),
                    };
                    self.access(&binding, scope, span)?;
                    at += 1;
                }
                Item::Struct(_) | Item::Enum(_) | Item::Alias(_) | Item::Trait(_) => {
                    binding.target = format!("{}::{}", binding.target, join_parts(&parts[at..]));
                    return Ok(Some(Resolved::item(binding)));
                }
                _ => return Ok(None),
            }
        }
        Ok(Some(Resolved::item(binding)))
    }
    fn resolve_path(
        &self,
        scope: ModuleId,
        path: &mut Path,
        namespace: Namespace,
        span: Span,
    ) -> Result<()> {
        if let Some(resolved) = self.path(scope, &path.name, namespace, span, true)? {
            path.name = resolved.name;
            path.declaration = resolved.binding.map(|binding| binding.declaration);
            Ok(())
        } else {
            Err(Diagnostic::new(
                span,
                "E_NAME",
                &format!("unresolved name {}", path.name),
            ))
        }
    }
    fn wider_binding(&self, incoming: &Binding, previous: &Binding) -> bool {
        match (
            region(self.program, incoming.scope, incoming.public),
            region(self.program, previous.scope, previous.public),
        ) {
            (None, Some(_)) => true,
            (Some(next), Some(old)) => next != old && within(self.program, old, next),
            _ => false,
        }
    }
    fn import_binding(
        &mut self,
        use_item: &Use,
        scope: ModuleId,
        name: &str,
        ns: Namespace,
        source: &Binding,
    ) -> Result<bool> {
        let owner = self.import_scopes.import_owner(use_item, scope);
        let key = (owner, name.to_string(), ns);
        if use_item.glob && self.names.get(&key).is_some_and(|binding| !binding.glob) {
            return Ok(false);
        }
        let target_region = region(self.program, source.scope, source.public);
        let use_region = region(self.program, scope, use_item.public);
        let export_allowed = match (target_region, use_region) {
            (None, _) => true,
            (Some(_), None) => false,
            (Some(target), Some(user)) => within(self.program, user, target),
        };
        if !export_allowed && !use_item.glob {
            return Err(Diagnostic::new(
                use_item.span,
                "E_PRIVACY",
                "re-export is wider than the imported binding's visibility",
            ));
        }
        let declaration = source.declaration;
        let target = &self.declarations[declaration];
        if name == "_" {
            return Ok(ns == Namespace::Type
                && matches!(target.item, Item::Trait(_))
                && self.anonymous_traits.insert((owner, declaration)));
        }
        if ns == Namespace::Type && matches!(target.item, Item::Trait(_)) {
            let key = (owner, declaration);
            if !self.trait_names.contains_key(&key) {
                self.trait_names.insert(key, BTreeSet::new());
            }
            self.trait_names
                .get_mut(&key)
                .unwrap()
                .insert(name.to_string());
        }
        let incoming = Binding {
            declaration,
            target: source.target.clone(),
            scope: if export_allowed { scope } else { source.scope },
            public: if export_allowed {
                use_item.public
            } else {
                source.public
            },
            glob: use_item.glob,
            ambiguous: false,
        };
        let previous: Option<Binding> = match self.names.get(&key) {
            Some(binding) => Some((*binding).clone()),
            None => None,
        };
        if let Some(previous) = previous {
            let same = previous.declaration == declaration && previous.target == source.target;
            if !previous.glob {
                if same {
                    return Ok(false);
                }
                return Err(Diagnostic::new(
                    use_item.span,
                    "E_NAME",
                    &format!("ambiguous import {name}"),
                ));
            }
            if !use_item.glob {
                self.names.insert(key.clone(), incoming);
                self.alternatives.remove(&key);
                return Ok(true);
            }
            if same {
                if self.wider_binding(&incoming, &previous) {
                    self.names.insert(
                        key,
                        Binding {
                            ambiguous: previous.ambiguous,
                            ..incoming
                        },
                    );
                    return Ok(true);
                }
                return Ok(false);
            }
            let position: Option<usize> = self.alternatives.get(&key).and_then(|bindings| {
                bindings.iter().position(|binding| {
                    binding.declaration == declaration && binding.target == source.target
                })
            });
            if let Some(position) = position {
                let wider = {
                    let previous = &self.alternatives.get(&key).unwrap()[position];
                    self.wider_binding(&incoming, previous)
                };
                if wider {
                    self.alternatives.get_mut(&key).unwrap()[position] = incoming;
                    return Ok(true);
                }
                return Ok(false);
            }
            if !self.alternatives.contains_key(&key) {
                self.alternatives.insert(key.clone(), Vec::new());
            }
            self.alternatives.get_mut(&key).unwrap().push(incoming);
            self.names.get_mut(&key).unwrap().ambiguous = true;
            return Ok(true);
        }
        self.names.insert(key, incoming);
        Ok(true)
    }
    fn module_members(
        &self,
        target: ModuleId,
        scope: ModuleId,
    ) -> Vec<(String, Namespace, Binding)> {
        let mut entries = Vec::new();
        for (key, binding) in &self.names {
            if key.0 < target {
                continue;
            }
            if key.0 > target {
                break;
            }
            if visible(self.program, binding.scope, binding.public, scope) {
                entries.push((key.1.clone(), key.2, binding.clone()));
            }
            if let Some(alternatives) = self.alternatives.get(key) {
                for binding in alternatives {
                    if visible(self.program, binding.scope, binding.public, scope) {
                        entries.push((key.1.clone(), key.2, binding.clone()));
                    }
                }
            }
        }
        entries
    }
    fn module_anchor(&self, scope: ModuleId, path: &str, span: Span) -> Result<Option<ModuleId>> {
        if path == "crate" {
            return Ok(Some(self.program.scopes[scope].crate_root));
        }
        if path == "self" {
            return Ok(Some(scope));
        }
        if path == "std"
            && self
                .lookup_lexical(scope, "std", Namespace::Type, span)
                .is_none()
        {
            return Ok(self.modules.get(&"$std".to_string()).copied());
        }
        if !path.is_empty() && path.split("::").all(|part| part == "super") {
            let mut current = scope;
            for _ in path.split("::") {
                current = self.program.scopes[current]
                    .parent
                    .ok_or_else(|| Diagnostic::new(span, "E_NAME", "super has no parent module"))?;
            }
            return Ok(Some(current));
        }
        Ok(None)
    }
    fn import_one(&mut self, index: usize) -> Result<(bool, bool)> {
        let import = self.program.uses[index].clone();
        let scope = *self.modules.get(&import.module).unwrap();
        let mut changed = false;
        if import.glob {
            let mut target = self.module_anchor(scope, &import.path, import.span)?;
            let mut enum_target: Option<(usize, Binding)> = None;
            if target.is_none() {
                let resolved =
                    self.path(scope, &import.path, Namespace::Type, import.span, false)?;
                let binding = match resolved.and_then(|resolved| resolved.binding) {
                    Some(binding) => binding,
                    None => return Ok((false, false)),
                };
                let declaration = &self.declarations[binding.declaration];
                match declaration.item {
                    Item::Module(index) => target = Some(self.program.modules[index].scope),
                    Item::Enum(index) if binding.target == declaration.name => {
                        enum_target = Some((index, binding));
                    }
                    _ => {
                        return Err(Diagnostic::new(
                            import.span,
                            "E_IMPORT",
                            "glob import requires a module or enum",
                        ));
                    }
                }
            }
            if target == Some(self.import_scopes.import_owner(&import, scope)) {
                return Err(Diagnostic::new(
                    import.span,
                    "E_IMPORT",
                    "a module cannot glob-import itself",
                ));
            }
            let entries: Vec<(String, Namespace, Binding)> = if let Some(target) = target {
                self.module_members(target, scope)
            } else if let Some((index, binding)) = enum_target {
                let mut entries = Vec::new();
                for variant in &self.program.enums[index].variants {
                    let mut binding = binding.clone();
                    binding.target = qualified(&binding.target, &variant.name);
                    for index in 0..2 {
                        let namespace = if index == 0 {
                            Namespace::Type
                        } else {
                            Namespace::Value
                        };
                        entries.push((variant.name.clone(), namespace, binding.clone()));
                    }
                }
                entries
            } else {
                return Ok((false, false));
            };
            for (name, namespace, binding) in entries {
                changed |= self.import_binding(&import, scope, &name, namespace, &binding)?;
            }
            Ok((changed, true))
        } else {
            let mut found = false;
            for index in 0..2 {
                let ns = if index == 0 {
                    Namespace::Type
                } else {
                    Namespace::Value
                };
                if let Some(binding) = self
                    .path(scope, &import.path, ns, import.span, false)?
                    .and_then(|resolved| resolved.binding)
                {
                    found = true;
                    changed |= self.import_binding(&import, scope, &import.alias, ns, &binding)?;
                }
            }
            if !found
                && self
                    .path(scope, &import.path, Namespace::Type, import.span, true)?
                    .is_some_and(|resolved| resolved.binding.is_none())
            {
                return Err(Diagnostic::new(
                    import.span,
                    "E_UNSUPPORTED",
                    "import aliases for intrinsic types or functions are not implemented",
                ));
            }
            Ok((changed, found))
        }
    }
    fn imports(&mut self) -> Vec<ImportFailure> {
        loop {
            let mut problems = BTreeMap::new();
            let mut unresolved = Vec::new();
            let mut changed = false;
            for index in 0..self.program.uses.len() {
                match self.import_one(index) {
                    Ok((updated, found)) => {
                        changed |= updated;
                        if !found {
                            unresolved.push(index);
                        }
                    }
                    Err(problem) => {
                        problems.insert(index, problem);
                    }
                }
            }
            if !changed {
                for index in unresolved {
                    let import = &self.program.uses[index];
                    problems.insert(
                        index,
                        Diagnostic::new(
                            import.span,
                            "E_IMPORT",
                            &format!("unresolved or cyclic import {}", import.path),
                        ),
                    );
                }
                let mut failures = Vec::new();
                for (import, problem) in problems {
                    failures.push(ImportFailure { import, problem });
                }
                return failures;
            }
        }
    }
}

pub fn resolve(source: &Program) -> Result<Program> {
    if source.resolved {
        return Ok(source.clone());
    }
    let resolver = Resolver::new(source)?;
    let mut result = source.clone();
    resolve_nodes(&mut result, &resolver, &mut |_, problem| Err(problem))?;
    result.declarations = resolver.declarations;
    result.function_owners = resolver
        .function_owners
        .expect("unresolved source has indexed owners");
    result.resolved = true;
    Ok(result)
}

fn resolve_nodes<F: FnMut(recovery::Site, Diagnostic) -> Result<()>>(
    result: &mut Program,
    resolver: &Resolver<'_>,
    failed: &mut F,
) -> Result<()> {
    for id in 0..result.types.len() {
        let node = &mut result.types[id];
        let scope = node.scope;
        let span = node.span;
        if let Type::Path(path) = &mut node.kind {
            if let Err(problem) = resolver.resolve_path(scope, path, Namespace::Type, span) {
                failed(recovery::Site::Type(id), problem)?;
            }
        }
    }
    for id in 0..result.patterns.len() {
        let node = &mut result.patterns[id];
        let scope = node.scope;
        let span = node.span;
        let operand = match &mut node.kind {
            Pattern::Path(path) | Pattern::Variant { path, .. } => Some((path, Namespace::Value)),
            Pattern::Struct { path, .. } => Some((path, Namespace::Type)),
            _ => None,
        };
        if let Some((path, namespace)) = operand {
            if let Err(problem) = resolver.resolve_path(scope, path, namespace, span) {
                failed(recovery::Site::Pattern(id), problem)?;
            }
        }
    }
    // Locals still resolve before declaration-valued expression paths.
    for id in 0..result.exprs.len() {
        let node = &mut result.exprs[id];
        let scope = node.scope;
        let span = node.span;
        if let Expr::Struct { path, .. } = &mut node.kind {
            if let Err(problem) = resolver.resolve_path(scope, path, Namespace::Type, span) {
                failed(recovery::Site::Expression(id), problem)?;
            }
        }
    }
    if result.resolved {
        return Ok(());
    }
    iteration::expand_recover(result, resolver, &mut |id, problem| {
        failed(recovery::Site::Expression(id), problem)
    })
}

impl Resolver<'_> {
    /// A parameter path keeps its lexical binding even when another module has
    /// a declaration with the same spelling. Resolved items are never parameters.
    pub(crate) fn generic_path(&self, path: &Path, span: Span) -> bool {
        let root = first_name(&path.name);
        path.declaration.is_none() && !root.starts_with('\'') && self.generic(span, root)
    }

    pub(crate) fn is_type_parameter(&self, path: &Path, span: Span) -> bool {
        !path.name.contains("::") && self.generic_path(path, span)
    }

    /// A failed import can change only lookups in its lexical domain. Known
    /// strong bindings outrank an unresolved glob; explicit failed aliases do
    /// not acquire an invented declaration.
    fn lookup_sites(
        &self,
        scope: ModuleId,
        name: &str,
        namespace: Namespace,
        span: Span,
    ) -> Vec<(ModuleId, String, Namespace)> {
        let mut parts: Vec<String> = Vec::new();
        for part in name.split("::") {
            if !part.is_empty() {
                parts.push(part.to_string());
            }
        }
        if parts.is_empty() {
            return Vec::new();
        }
        let first = parts[0].as_str();
        let mut current = scope;
        let mut at = 0;
        let rooted = match first {
            "crate" => {
                current = self.program.scopes[scope].crate_root;
                at = 1;
                true
            }
            "self" => {
                at = 1;
                true
            }
            "super" => {
                while at < parts.len() && parts[at] == "super" {
                    let Some(parent) = self.program.scopes[current].parent else {
                        return Vec::new();
                    };
                    current = parent;
                    at += 1;
                }
                true
            }
            _ => false,
        };
        if at == parts.len() {
            return Vec::new();
        }
        let first_namespace = if at + 1 < parts.len() {
            Namespace::Type
        } else {
            namespace
        };
        let mut result = vec![(current, parts[at].clone(), first_namespace)];
        let mut binding = if rooted {
            self.lookup(current, &parts[at], first_namespace)
        } else {
            self.path(scope, first, first_namespace, span, true)
                .ok()
                .flatten()
                .and_then(|resolved| resolved.binding)
        };
        if !rooted
            && first == "std"
            && self
                .lookup_lexical(scope, first, Namespace::Type, span)
                .is_none()
        {
            if let Some(library) = self.modules.get(&"$std".to_string()).copied() {
                current = library;
                at += 1;
                if at == parts.len() {
                    return result;
                }
                let namespace = if at + 1 < parts.len() {
                    Namespace::Type
                } else {
                    namespace
                };
                result.push((current, parts[at].clone(), namespace));
                binding = self.lookup(current, &parts[at], namespace);
            }
        }
        while at + 1 < parts.len() {
            let Some(binding_value) = binding else {
                break;
            };
            let Item::Module(module) = self.declarations[binding_value.declaration].item else {
                break;
            };
            current = self.program.modules[module].scope;
            at += 1;
            let namespace = if at + 1 < parts.len() {
                Namespace::Type
            } else {
                namespace
            };
            result.push((current, parts[at].clone(), namespace));
            binding = self.lookup(current, &parts[at], namespace);
        }
        result
    }
    /// Resolved module-glob edges for diagnostic availability propagation.
    /// This reads existing identities and never installs a missing binding.
    pub(crate) fn recovery_globs(&self) -> Vec<(ModuleId, usize)> {
        let mut edges = Vec::new();
        for import in &self.program.uses {
            if !import.glob {
                continue;
            }
            let scope = *self.modules.get(&import.module).unwrap();
            let target = self
                .module_anchor(scope, &import.path, import.span)
                .ok()
                .flatten()
                .or_else(|| {
                    let resolved = self
                        .path(scope, &import.path, Namespace::Type, import.span, false)
                        .ok()
                        .flatten()?;
                    let declaration = resolved.binding?.declaration;
                    if let Item::Module(module) = self.declarations[declaration].item {
                        Some(self.program.modules[module].scope)
                    } else {
                        None
                    }
                });
            if let Some(target) = target {
                edges.push((target, self.import_scopes.import_owner(import, scope)));
            }
        }
        edges
    }
    /// Whether an unavailable declaration could participate in this lookup.
    /// Used only by diagnostic reports; it installs no binding.
    pub(crate) fn recovery_name_visible(
        &self,
        lookup: (ModuleId, &str, Namespace, Span),
        owner: ModuleId,
        declared: Option<&str>,
        glob: bool,
    ) -> bool {
        if self.generic(lookup.3, first_name(lookup.1)) && owner < self.program.scopes.len() {
            return false;
        }
        for (scope, name, namespace) in self.lookup_sites(lookup.0, lookup.1, lookup.2, lookup.3) {
            if declared.is_some_and(|declared| declared != name.as_str()) {
                continue;
            }
            let mut at = self.import_scopes.at(scope, lookup.3);
            loop {
                let strong = self
                    .names
                    .get(&(at, name.to_string(), namespace))
                    .is_some_and(|binding| !binding.glob);
                if at == owner {
                    if !glob || !strong {
                        return true;
                    }
                    break;
                }
                if strong {
                    break;
                }
                match self.import_scopes.parent(at) {
                    Some(parent) => at = parent,
                    None => {
                        if self.modules.get(&"$std::prelude".to_string()).copied() == Some(owner) {
                            return true;
                        }
                        break;
                    }
                }
            }
        }
        false
    }
    pub(crate) fn uncertain_trait_import(
        &self,
        imports: &[usize],
        scope: ModuleId,
        span: Span,
    ) -> Option<usize> {
        let mut owners = BTreeSet::new();
        let mut at = self.import_scopes.at(scope, span);
        loop {
            owners.insert(at);
            let Some(parent) = self.import_scopes.parent(at) else {
                break;
            };
            at = parent;
        }
        if let Some(prelude) = self.modules.get(&"$std::prelude".to_string()) {
            owners.insert(*prelude);
        }
        for id in imports {
            let import = &self.program.uses[*id];
            let module = *self.modules.get(&import.module).unwrap();
            if owners.contains(&self.import_scopes.import_owner(import, module)) {
                return Some(*id);
            }
        }
        None
    }
    pub(crate) fn uncertain_import(
        &self,
        imports: &[usize],
        scope: ModuleId,
        name: &str,
        namespace: Namespace,
        span: Span,
    ) -> Option<usize> {
        let generic = self.generic(span, first_name(name));
        let lookups = self.lookup_sites(scope, name, namespace, span);
        for (scope, name, namespace) in lookups {
            for id in imports {
                let import = &self.program.uses[*id];
                if !import.glob && import.alias != name {
                    continue;
                }
                let module = *self.modules.get(&import.module).unwrap();
                let owner = self.import_scopes.import_owner(import, module);
                if generic && owner < self.program.scopes.len() {
                    continue;
                }
                let mut at = self.import_scopes.at(scope, span);
                let prelude = self.modules.get(&"$std::prelude".to_string()).copied();
                loop {
                    let strong = self
                        .names
                        .get(&(at, name.to_string(), namespace))
                        .is_some_and(|binding| !binding.glob);
                    if at == owner {
                        if !import.glob || !strong {
                            return Some(*id);
                        }
                        break;
                    }
                    if strong {
                        break;
                    }
                    match self.import_scopes.parent(at) {
                        Some(parent) => at = parent,
                        None => {
                            if prelude == Some(owner) {
                                return Some(*id);
                            }
                            break;
                        }
                    }
                }
            }
        }
        None
    }

    pub fn value_path(&self, scope: ModuleId, path: &Path, span: Span) -> Result<Path> {
        // Resolved expression paths include compiler-generated source sugar.
        // Their declaration identity must not be looked up again by spelling.
        if let Some(declaration) = path.declaration {
            if self.declarations.get(declaration).is_none() {
                return Err(Diagnostic::new(
                    span,
                    "E_NAME",
                    "resolved value declaration is missing",
                ));
            }
            return Ok(path.clone());
        }
        let mut path = path.clone();
        self.resolve_path(scope, &mut path, Namespace::Value, span)?;
        Ok(path)
    }
    pub fn scope(&self, path: &str) -> ModuleId {
        *self.modules.get(&path.to_string()).unwrap()
    }
    pub fn type_name(&self, scope: ModuleId, name: &str, span: Span) -> Result<String> {
        self.path(scope, name, Namespace::Type, span, true)?
            .map(|resolved| resolved.name)
            .ok_or_else(|| Diagnostic::new(span, "E_NAME", &format!("unresolved type {name}")))
    }
    pub fn prelude_type(&self, name: &str, span: Span) -> Result<String> {
        let scope = self
            .modules
            .get(&"$std::prelude".to_string())
            .copied()
            .unwrap_or(0);
        self.type_name(scope, name, span)
    }
    pub fn library_type(&self, module: &str, name: &str, span: Span) -> Result<Path> {
        let module = qualified("$std", module);
        let scope = match self.modules.get(&module) {
            Some(scope) => *scope,
            None => {
                return Err(Diagnostic::new(
                    span,
                    "E_NAME",
                    &format!("source library module {module} is missing"),
                ));
            }
        };
        let mut path = Path {
            declaration: None,
            name: name.to_string(),
            args: Vec::new(),
        };
        self.resolve_path(scope, &mut path, Namespace::Type, span)?;
        Ok(path)
    }
    pub fn library_function(&self, module: &str, name: &str, span: Span) -> Result<usize> {
        let module = format!("$std::{module}");
        let scope = match self.modules.get(&module) {
            Some(scope) => *scope,
            None => {
                return Err(Diagnostic::new(
                    span,
                    "E_UNSUPPORTED",
                    &format!("source library module {module} is not loaded"),
                ));
            }
        };
        let path = self.value_path(
            scope,
            &Path {
                declaration: None,
                name: name.to_string(),
                args: Vec::new(),
            },
            span,
        )?;
        let declaration = path.declaration.ok_or_else(|| {
            Diagnostic::new(span, "E_NAME", "source library helper is not a declaration")
        })?;
        match self.declarations[declaration].item {
            Item::Function(index) => Ok(index),
            _ => Err(Diagnostic::new(
                span,
                "E_TYPE",
                "source library helper is not a function",
            )),
        }
    }
    pub fn sequence_function(&self, name: &str, span: Span) -> Result<usize> {
        self.library_function("sequence", name, span)
    }
    pub fn trait_in_scope(&self, scope: ModuleId, declaration: DeclarationId, span: Span) -> bool {
        let mut owner = self.import_scopes.at(scope, span);
        loop {
            if self.anonymous_traits.contains(&(owner, declaration)) {
                return true;
            }
            if let Some(names) = self.trait_names.get(&(owner, declaration)) {
                for name in names {
                    if self.lookup_owner(scope, name, Namespace::Type, span) != Some(owner) {
                        continue;
                    }
                    let key = (owner, name.clone(), Namespace::Type);
                    if self.names.get(&key).is_some_and(|binding| {
                        binding.declaration == declaration
                            && visible(self.program, binding.scope, binding.public, scope)
                    }) || self.alternatives.get(&key).is_some_and(|bindings| {
                        bindings.iter().any(|binding| {
                            binding.declaration == declaration
                                && visible(self.program, binding.scope, binding.public, scope)
                        })
                    }) {
                        return true;
                    }
                }
            }
            let Some(parent) = self.import_scopes.parent(owner) else {
                break;
            };
            owner = parent;
        }
        let Some(prelude) = self.modules.get(&"$std::prelude".to_string()) else {
            return false;
        };
        if let Some(names) = self.trait_names.get(&(*prelude, declaration)) {
            for name in names {
                if let Ok(Some(resolved)) = self.path(scope, name, Namespace::Type, span, true) {
                    if let Some(binding) = resolved.binding {
                        if binding.declaration == declaration {
                            return true;
                        }
                    }
                }
            }
        }
        false
    }
    pub fn declaration_visible(&self, scope: ModuleId, declaration: DeclarationId) -> bool {
        let item = &self.declarations[declaration];
        visible(self.program, item.scope, item.public, scope)
    }
}

/// Diagnostic inventory over the same declaration/import resolver. No resolved
/// Program or checked authority escapes when an import or use is unresolved.
/// Method bodies still require valid signatures; missing imports can account
/// for later unresolved type uses, so the census reports them as dependencies.
pub fn resolution_problems(source: &Program) -> Vec<Diagnostic> {
    match recovery::resolve(source) {
        Ok(report) => {
            let mut problems = Vec::new();
            for failure in report.failures {
                problems.push(failure.problem);
            }
            problems
        }
        Err(problem) => vec![problem],
    }
}
