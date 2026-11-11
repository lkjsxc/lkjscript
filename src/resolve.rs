//! Module and declaration identity. Later phases consume resolved paths, never bare-name guesses.
use crate::ast::*;
use std::collections::{BTreeMap, BTreeSet};

type Result<T> = std::result::Result<T, Diagnostic>;
#[derive(Clone)]
struct Binding {
    declaration: DeclarationId,
    target: String,
    scope: ModuleId,
    public: Visibility,
}
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Namespace {
    Type,
    Value,
}
pub struct Resolver<'a> {
    program: &'a Program,
    declarations: Vec<Declaration>,
    names: BTreeMap<(ModuleId, String, Namespace), Binding>,
    modules: BTreeMap<String, ModuleId>,
    generic_ranges: Vec<(Span, Vec<String>)>,
}

pub fn qualified(module: &str, name: &str) -> String {
    if module.is_empty() {
        name.to_string()
    } else {
        format!("{module}::{name}")
    }
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
impl<'a> Resolver<'a> {
    pub fn new(program: &'a Program) -> Result<Self> {
        let mut result = Resolver {
            program,
            declarations: Vec::new(),
            names: BTreeMap::new(),
            modules: BTreeMap::new(),
            generic_ranges: Vec::new(),
        };
        for (id, scope) in program.scopes.iter().enumerate() {
            result.modules.insert(scope.path.clone(), id);
        }
        let mut methods = BTreeSet::new();
        for item in &program.traits {
            methods.extend(item.methods.iter().copied());
        }
        for item in &program.impls {
            methods.extend(item.methods.iter().copied());
            result.generics(item.span, &item.generics);
        }
        for (id, item) in program.functions.iter().enumerate() {
            result.generics(item.span, &item.generics);
            if !methods.contains(&id) {
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
            if item.external {
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
        result.imports()?;
        Ok(result)
    }
    fn generics(&mut self, span: Span, params: &[GenericParam]) {
        if !params.is_empty() {
            self.generic_ranges
                .push((span, params.iter().map(|p| p.name.clone()).collect()));
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
        let scope = self.modules[path];
        let declaration = self.declarations.len();
        self.declarations.push(Declaration {
            name: qualified(path, name),
            scope,
            item,
            public,
            span,
        });
        for (enabled, namespace) in [(ty, Namespace::Type), (value, Namespace::Value)] {
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
    fn path(
        &self,
        scope: ModuleId,
        path: &str,
        namespace: Namespace,
        span: Span,
        prelude: bool,
    ) -> Result<Option<(String, Option<DeclarationId>)>> {
        let parts: Vec<_> = path.split("::").collect();
        if parts.is_empty() {
            return Ok(None);
        }
        if self.generic(span, parts[0]) {
            return Ok(Some((path.to_string(), None)));
        }
        if matches!(
            path,
            "std::mem::replace" | "std::mem::drop" | "mem::replace" | "std::thread::scope"
        ) && !self
            .names
            .contains_key(&(scope, parts[0].to_string(), Namespace::Type))
        {
            return Ok(Some((path.to_string(), None)));
        }
        let mut at = 0;
        let mut current = scope;
        let mut rooted = false;
        if parts[0] == "crate" {
            current = self.program.scopes[scope].crate_root;
            at = 1;
            rooted = true;
        } else if parts[0] == "self" {
            at = 1;
            rooted = true;
        } else if parts[0] == "super" {
            while at < parts.len() && parts[at] == "super" {
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
        let mut binding = self.lookup(current, parts[at], first_namespace);
        if binding.is_none() && !rooted && parts[at] == "std" && self.modules.contains_key("$std") {
            current = self.modules["$std"];
            at += 1;
            rooted = true;
            if at == parts.len() {
                return Ok(None);
            }
            let remaining = parts[at..].join("::");
            let intrinsic = match remaining.as_str() {
                "vec::Vec" => Some("Vec"),
                "boxed::Box" => Some("Box"),
                "mem::replace" => Some("std::mem::replace"),
                "mem::drop" => Some("std::mem::drop"),
                _ => None,
            };
            if let Some(name) = intrinsic {
                return Ok(Some((name.to_string(), None)));
            }
            if remaining.starts_with("vec::Vec::") {
                return Ok(Some((remaining.replacen("vec::", "", 1), None)));
            }
            if remaining.starts_with("boxed::Box::") {
                return Ok(Some((remaining.replacen("boxed::", "", 1), None)));
            }
            binding = self.lookup(
                current,
                parts[at],
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
                && matches!(parts[0], "None" | "Some" | "Ok" | "Err")
            {
                let owner = if matches!(parts[0], "None" | "Some") {
                    "Option"
                } else {
                    "Result"
                };
                if let Some((owner, id)) = self.path(scope, owner, Namespace::Type, span, true)? {
                    return Ok(Some((format!("{owner}::{}", parts[0]), id)));
                }
            }
            if let Some(std_scope) = self.modules.get("$std::prelude") {
                binding = self.lookup(*std_scope, parts[at], first_namespace);
            }
            if binding.is_none() && parts[0] == "String" {
                if let Some(std_scope) = self.modules.get("$std::string") {
                    binding = self.lookup(*std_scope, "String", Namespace::Type);
                }
            }
            if binding.is_none() && parts[0] == "__lkj_vec_compare" {
                if let Some(std_scope) = self.modules.get("$std::sequence") {
                    binding = self.lookup(*std_scope, parts[0], Namespace::Value);
                }
            }
            if binding.is_none()
                && (builtin(parts[0])
                    || matches!(
                        path,
                        "drop"
                            | "std::mem::replace"
                            | "mem::replace"
                            | "std::mem::drop"
                            | "scope"
                            | "std::thread::scope"
                    ))
            {
                return Ok(Some((path.to_string(), None)));
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
                    binding = match self.lookup(current, parts[at], ns) {
                        Some(b) => b,
                        None => return Ok(None),
                    };
                    self.access(&binding, scope, span)?;
                    at += 1;
                }
                Item::Struct(_) | Item::Enum(_) | Item::Alias(_) | Item::Trait(_) => {
                    return Ok(Some((
                        format!("{}::{}", binding.target, parts[at..].join("::")),
                        Some(binding.declaration),
                    )));
                }
                _ => return Ok(None),
            }
        }
        Ok(Some((binding.target.clone(), Some(binding.declaration))))
    }
    fn resolve_path(
        &self,
        scope: ModuleId,
        path: &mut Path,
        namespace: Namespace,
        span: Span,
    ) -> Result<()> {
        if let Some((name, declaration)) = self.path(scope, &path.name, namespace, span, true)? {
            path.name = name;
            path.declaration = declaration;
            Ok(())
        } else {
            Err(Diagnostic::new(
                span,
                "E_NAME",
                &format!("unresolved name {}", path.name),
            ))
        }
    }
    fn import_binding(
        &mut self,
        use_item: &Use,
        scope: ModuleId,
        name: &str,
        ns: Namespace,
        declaration: DeclarationId,
        target_name: &str,
    ) -> Result<bool> {
        let target = &self.declarations[declaration];
        let target_region = region(self.program, target.scope, target.public);
        let use_region = region(self.program, scope, use_item.public);
        let export_allowed = match (target_region, use_region) {
            (None, _) => true,
            (Some(_), None) => false,
            (Some(target), Some(user)) => within(self.program, user, target),
        };
        if !export_allowed {
            return Err(Diagnostic::new(
                use_item.span,
                "E_PRIVACY",
                "re-export is wider than the declaration's visibility",
            ));
        }
        let key = (scope, name.to_string(), ns);
        if let Some(previous) = self.names.get(&key) {
            if previous.declaration == declaration && previous.target == target_name {
                return Ok(false);
            }
            return Err(Diagnostic::new(
                use_item.span,
                "E_NAME",
                &format!("ambiguous import {name}"),
            ));
        }
        self.names.insert(
            key,
            Binding {
                declaration,
                target: target_name.to_string(),
                scope,
                public: use_item.public,
            },
        );
        Ok(true)
    }
    fn imports(&mut self) -> Result<()> {
        loop {
            let mut next = Vec::new();
            let mut changed = false;
            for index in 0..self.program.uses.len() {
                let import = self.program.uses[index].clone();
                let scope = self.modules[&import.module];
                if import.glob {
                    let resolved =
                        self.path(scope, &import.path, Namespace::Type, import.span, false)?;
                    let id = match resolved {
                        Some((_, Some(id))) => id,
                        _ => {
                            next.push(index);
                            continue;
                        }
                    };
                    let target = match self.declarations[id].item {
                        Item::Module(m) => self.program.modules[m].scope,
                        _ => {
                            return Err(Diagnostic::new(
                                import.span,
                                "E_UNSUPPORTED",
                                "glob imports currently require a module",
                            ));
                        }
                    };
                    let entries: Vec<_> = self
                        .names
                        .iter()
                        .filter(|((s, _, _), b)| {
                            *s == target && visible(self.program, b.scope, b.public, scope)
                        })
                        .map(|((_, n, ns), b)| (n.clone(), *ns, b.declaration, b.target.clone()))
                        .collect();
                    for (name, ns, id, target) in entries {
                        changed |= self.import_binding(&import, scope, &name, ns, id, &target)?;
                    }
                } else {
                    let mut found = false;
                    for ns in [Namespace::Type, Namespace::Value] {
                        if let Some((target, Some(id))) =
                            self.path(scope, &import.path, ns, import.span, false)?
                        {
                            found = true;
                            changed |= self.import_binding(
                                &import,
                                scope,
                                &import.alias,
                                ns,
                                id,
                                &target,
                            )?;
                        }
                    }
                    if !found {
                        // Compiler intrinsics have a fixed identity, never an arbitrary bare-name fallback.
                        if self
                            .path(scope, &import.path, Namespace::Type, import.span, true)?
                            .is_some_and(|(_, id)| id.is_none())
                        {
                            return Err(Diagnostic::new(
                                import.span,
                                "E_UNSUPPORTED",
                                "import aliases for intrinsic types or functions are not implemented",
                            ));
                        }
                        next.push(index);
                    }
                }
            }
            if !changed && next.is_empty() {
                break;
            }
            if !changed {
                let import = &self.program.uses[next[0]];
                return Err(Diagnostic::new(
                    import.span,
                    "E_IMPORT",
                    &format!("unresolved or cyclic import {}", import.path),
                ));
            }
        }
        Ok(())
    }
}

pub fn resolve(source: &Program) -> Result<Program> {
    if source.resolved {
        return Ok(source.clone());
    }
    let resolver = Resolver::new(source)?;
    let mut result = source.clone();
    for node in &mut result.types {
        if let Type::Path(path) = &mut node.kind {
            resolver.resolve_path(node.scope, path, Namespace::Type, node.span)?;
        }
    }
    for node in &mut result.patterns {
        match &mut node.kind {
            Pattern::Path(path) | Pattern::Variant { path, .. } => {
                resolver.resolve_path(node.scope, path, Namespace::Value, node.span)?
            }
            Pattern::Struct { path, .. } => {
                resolver.resolve_path(node.scope, path, Namespace::Type, node.span)?
            }
            _ => {}
        }
    }
    // Lexical locals are handled by the elaborator's block stack before item lookup.
    // Resolve declaration-valued expression paths there, in the same module table.
    for node in &mut result.exprs {
        if let Expr::Struct { path, .. } = &mut node.kind {
            resolver.resolve_path(node.scope, path, Namespace::Type, node.span)?;
        }
    }
    result.declarations = resolver.declarations;
    result.resolved = true;
    Ok(result)
}

impl Resolver<'_> {
    pub fn value_path(&self, scope: ModuleId, path: &Path, span: Span) -> Result<Path> {
        let mut path = path.clone();
        self.resolve_path(scope, &mut path, Namespace::Value, span)?;
        Ok(path)
    }
    pub fn scope(&self, path: &str) -> ModuleId {
        self.modules[path]
    }
    pub fn type_name(&self, scope: ModuleId, name: &str, span: Span) -> Result<String> {
        self.path(scope, name, Namespace::Type, span, true)?
            .map(|(name, _)| name)
            .ok_or_else(|| Diagnostic::new(span, "E_NAME", &format!("unresolved type {name}")))
    }
    pub fn prelude_type(&self, name: &str, span: Span) -> Result<String> {
        let scope = self.modules.get("$std::prelude").copied().unwrap_or(0);
        self.type_name(scope, name, span)
    }
    pub fn sequence_function(&self, span: Span) -> Result<String> {
        let scope = self.modules.get("$std::sequence").copied().unwrap_or(0);
        self.value_path(
            scope,
            &Path {
                declaration: None,
                name: "__lkj_vec_compare".to_string(),
                args: Vec::new(),
            },
            span,
        )
        .map(|p| p.name)
    }
    pub fn trait_in_scope(&self, scope: ModuleId, declaration: DeclarationId) -> bool {
        self.names.iter().any(|((owner, _, namespace), binding)| {
            *owner == scope
                && *namespace == Namespace::Type
                && binding.declaration == declaration
                && visible(self.program, binding.scope, binding.public, scope)
        })
    }
    pub fn declaration_visible(&self, scope: ModuleId, declaration: DeclarationId) -> bool {
        let item = &self.declarations[declaration];
        visible(self.program, item.scope, item.public, scope)
    }
}
