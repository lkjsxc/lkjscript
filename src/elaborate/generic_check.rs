//! Definition-site verification through the production elaborator and K checker.
//!
//! Rigid IDs are abstract type variables, not sampled concrete instantiations.
//! Their private K storage shape supplies only affine/Copy behavior. Trait proof
//! consults the explicit assumptions below before structural rules, and source
//! cannot name or construct these IDs. Verification programs are consumed here;
//! no abstract value, assumed method, or verification artifact reaches emission.
use super::{Compiler, Instance, Result, a, err, k};
use crate::generics::TraitIdentity;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone)]
struct MethodWitness {
    declaration: usize,
    method: usize,
    substitutions: BTreeMap<String, usize>,
}

/// Builtin operations have typed contracts only in the discarded verification
/// program. Concrete instantiations still select and compile real methods.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum BuiltinMethod {
    Clone,
    CloneFrom,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    // Private comparison classification: -1, 0, 1, or 2 (unordered).
    Order,
}

impl BuiltinMethod {
    pub(super) fn named(name: &str) -> Option<Self> {
        match name {
            "clone" => Some(Self::Clone),
            "clone_from" => Some(Self::CloneFrom),
            "eq" => Some(Self::Eq),
            "ne" => Some(Self::Ne),
            "lt" => Some(Self::Lt),
            "le" => Some(Self::Le),
            "gt" => Some(Self::Gt),
            "ge" => Some(Self::Ge),
            _ => None,
        }
    }

    pub(super) fn name(self) -> &'static str {
        match self {
            Self::Clone => "clone",
            Self::CloneFrom => "clone_from",
            Self::Eq => "eq",
            Self::Ne => "ne",
            Self::Lt => "lt",
            Self::Le => "le",
            Self::Gt => "gt",
            Self::Ge => "ge",
            Self::Order => "$order",
        }
    }

    pub(super) fn required(self) -> &'static str {
        match self {
            Self::Clone | Self::CloneFrom => "Clone",
            Self::Eq | Self::Ne => "PartialEq",
            Self::Lt | Self::Le | Self::Gt | Self::Ge | Self::Order => "PartialOrd",
        }
    }

    pub(super) fn arguments(self) -> usize {
        if self == Self::Clone { 0 } else { 1 }
    }

    pub(super) fn result(self, receiver: usize) -> usize {
        match self {
            Self::Clone => receiver,
            Self::CloneFrom => k::UNIT,
            Self::Eq | Self::Ne | Self::Lt | Self::Le | Self::Gt | Self::Ge => k::BOOL,
            Self::Order => k::I32,
        }
    }
}

/// Source-bound choices survive verification; no verification type/function ID
/// is retained. Replaying a call specializes this choice, never name lookup.
#[derive(Clone)]
pub(super) enum MethodTarget {
    Builtin,
    Trait(TraitIdentity),
    Source(usize),
}

#[derive(Clone)]
pub(super) struct MethodSelection {
    pub(super) dereferences: usize,
    pub(super) target: MethodTarget,
}

pub(super) type MethodSelections = BTreeMap<a::ExprId, MethodSelection>;

#[derive(Default)]
pub(super) struct Contracts {
    pub(super) active_scope: usize,
    pub(super) rigid: BTreeMap<usize, BTreeSet<TraitIdentity>>,
    // (rigid receiver, resolved trait declaration) -> associated-type identities.
    traits: BTreeMap<(usize, usize), BTreeMap<String, usize>>,
    methods: BTreeMap<usize, MethodWitness>,
    comparisons: BTreeMap<(usize, String), usize>,
    builtin_methods: BTreeMap<(usize, BuiltinMethod), usize>,
}

#[derive(Default)]
pub(super) struct DefinitionEvidence {
    pub(super) methods: MethodSelections,
    pub(super) types: BTreeMap<usize, super::definition_types::DefinitionTypes>,
    pub(super) closures: super::closures::Evidence,
}

pub(super) fn validate_report(
    source: &a::Program,
    definitions: BTreeMap<usize, super::definition_types::DefinitionTypes>,
) -> Result<(DefinitionEvidence, Vec<a::Diagnostic>)> {
    let mut roots = Vec::new();
    for (index, function) in source.functions.iter().enumerate() {
        if function.body.is_none() || !definitions.contains_key(&index) {
            continue;
        }
        if function
            .generics
            .iter()
            .any(|parameter| !parameter.lifetime)
            || source.function_owner(a::FunctionId { index }) != a::FunctionOwner::Free
        {
            roots.push(index);
        }
    }
    let mut compiler = Compiler::new(source)?;
    compiler.definition_types = definitions;
    compiler.contracts = Some(Contracts::default());
    super::derive_check::validate(&mut compiler)?;
    if roots.is_empty() {
        return Ok((
            DefinitionEvidence {
                methods: MethodSelections::default(),
                closures: super::closures::Evidence::default(),
                types: compiler.definition_types,
            },
            Vec::new(),
        ));
    }
    let mut checked_roots = BTreeSet::new();
    let mut problems = Vec::new();
    for index in roots {
        let substitutions = compiler.definition_parameters(index)?;
        let function = compiler.instantiate(index, substitutions.clone())?;
        let instance = Instance {
            source: index,
            substitutions,
        };
        match compiler.compile_function(function, &instance) {
            Ok(body) => {
                compiler.functions[function] = body;
                checked_roots.insert(function);
            }
            Err(problem) => problems.push(problem),
        }
    }

    // User destructors retain their ordinary K obligations. Their definitions
    // were checked above; this declaration pass attaches concrete signatures to
    // the abstract nominal layouts created while checking those definitions.
    let mut next_type = 0;
    let mut next_function = 0;
    while next_type < compiler.types.len() || next_function < compiler.pending_functions.len() {
        while next_type < compiler.types.len() {
            compiler.register_drop(next_type)?;
            next_type += 1;
        }
        if let Some((index, pending)) = compiler.pending_functions.get(next_function).cloned() {
            if let super::PendingFunction::Derived(instance) = pending {
                compiler.functions[index] = compiler.compile_derived(index, &instance)?;
                checked_roots.insert(index);
            }
            next_function += 1;
        }
    }

    // A call uses a checked declaration's signature, or an identified declared
    // trait/builtin contract. It does not recursively re-check a specialization
    // of the callee. Each production definition has its own rigid root above;
    // nongeneric free functions still go through the normal concrete pipeline.
    let declared: BTreeSet<usize> = compiler.instance_names.values().copied().collect();
    checked_roots.extend(&compiler.closures.checked_functions);
    let callable_functions = compiler.assumed_callable_functions();
    let builtin_methods = compiler.validate_builtin_methods()?;
    let contracts = compiler.contracts.as_ref().unwrap();
    for (index, function) in compiler.functions.iter_mut().enumerate() {
        if checked_roots.contains(&index) {
            continue;
        }
        if let Some(witness) = contracts.methods.get(&index) {
            let receiver = witness.substitutions.get("Self").copied().ok_or_else(|| {
                err(
                    a::Span::default(),
                    "E_GENERIC",
                    "trait witness is missing its receiver type",
                )
            })?;
            if !source.traits[witness.declaration]
                .methods
                .contains(&witness.method)
                || !contracts
                    .traits
                    .contains_key(&(receiver, witness.declaration))
            {
                return Err(err(
                    a::Span::default(),
                    "E_GENERIC",
                    "call does not match its declared trait witness",
                ));
            }
        }
        let builtin = callable_functions.contains(&index)
            || builtin_methods.contains(&index)
            || contracts.comparisons.values().any(|value| *value == index);
        if !declared.contains(&index) && !builtin {
            return Err(err(
                a::Span {
                    file: function.file,
                    ..a::Span::default()
                },
                "E_GENERIC",
                "verification call has no declaration or trait contract",
            ));
        }
        function.body = vec![k::Instruction {
            column: 1,
            op: k::Op::Trap {
                message: "abstract declared contract".to_string(),
            },
            line: 1,
        }];
    }
    let mut refinement = compiler.closures.checked_functions.clone();
    refinement.extend(&compiler.return_inference);
    let (scopes, scoped_problems) = compiler.scoped_definitions(&checked_roots);
    problems.extend(scoped_problems);
    let mut program = k::Program {
        types: compiler.types,
        functions: compiler.functions,
        destructors: compiler.destructors,
    };
    problems.extend(crate::check::refine_return_contracts(
        &mut program,
        &refinement,
    ));
    problems.extend(crate::check::scoped::check_definitions(&program, &scopes));
    if let Err(errors) = crate::check::check_all(program) {
        problems.extend(errors);
    }
    Ok((
        DefinitionEvidence {
            methods: compiler.method_selections,
            closures: compiler.closures.evidence,
            types: compiler.definition_types,
        },
        problems,
    ))
}

impl Compiler<'_> {
    fn rigid_type(&mut self, name: &str) -> usize {
        let id = self.types.len();
        self.types.push(k::Type {
            name: format!("$rigid:{name}:{id}"),
            kind: k::TypeKind::Struct {
                fields: Vec::new(),
                copy: false,
            },
        });
        self.contracts
            .as_mut()
            .unwrap()
            .rigid
            .insert(id, BTreeSet::new());
        id
    }

    pub(super) fn declare_parameters(
        &mut self,
        parameters: &[a::GenericParam],
        substitutions: &mut BTreeMap<String, usize>,
    ) -> Result<()> {
        for parameter in parameters {
            if parameter.lifetime {
                continue;
            }
            if substitutions.contains_key(&parameter.name) {
                return Err(err(
                    parameter.span,
                    "E_GENERIC",
                    "type parameter shadows an enclosing parameter",
                ));
            }
            let ty = self.rigid_type(&parameter.name);
            substitutions.insert(parameter.name.clone(), ty);
        }
        // Associated identities come from declared trait bounds. Establish all
        // of them before a callable signature resolves a sibling's I::Item,
        // regardless of the source order of generic parameter declarations.
        for callable in [false, true] {
            for parameter in parameters {
                if parameter.lifetime {
                    continue;
                }
                let ty = substitutions[&parameter.name];
                for bound in &parameter.bounds {
                    if self.is_callable_bound(*bound) == callable {
                        self.assume_bound(ty, *bound, substitutions)?;
                    }
                }
            }
        }
        Ok(())
    }

    pub(super) fn assume_builtin(&mut self, ty: usize, name: &str) {
        let assumptions = self.contracts.as_mut().unwrap().rigid.get_mut(&ty).unwrap();
        assumptions.insert(TraitIdentity::Builtin(name.to_string()));
        for parent in crate::generics::builtin_parents(name) {
            assumptions.insert(TraitIdentity::Builtin((*parent).to_string()));
        }
        if name == "Copy" {
            self.types[ty].kind = k::TypeKind::Struct {
                fields: Vec::new(),
                copy: true,
            };
        }
    }

    fn assume_bound(
        &mut self,
        ty: usize,
        first: a::TypeId,
        substitutions: &BTreeMap<String, usize>,
    ) -> Result<()> {
        let mut pending = vec![first];
        while let Some(id) = pending.pop() {
            let node = self.source.types[id].clone();
            if let a::Type::Path(path) = &node.kind {
                if self.assume_callable(ty, path, substitutions, node.span)? {
                    continue;
                }
            }
            let path = match node.kind {
                a::Type::Path(path) if path.args.is_empty() => path,
                a::Type::Path(path) if path.name.starts_with('\'') => continue,
                _ => {
                    return Err(err(
                        node.span,
                        "E_UNSUPPORTED",
                        "definition-site parameterized trait bounds are not implemented",
                    ));
                }
            };
            if path.name.starts_with('\'') {
                continue;
            }
            let identity = crate::generics::bound_identity(self.source, &path);
            let inserted = self
                .contracts
                .as_mut()
                .unwrap()
                .rigid
                .get_mut(&ty)
                .unwrap()
                .insert(identity.clone());
            if !inserted {
                continue;
            }
            let builtin = match &identity {
                TraitIdentity::Builtin(name) => name.as_str(),
                _ => "",
            };
            if !builtin.is_empty() {
                self.assume_builtin(ty, builtin);
            }
            let trait_index = match identity {
                TraitIdentity::Declared(index) => Some(index),
                _ => None,
            };
            if let Some(trait_index) = trait_index {
                let declaration = self.source.traits[trait_index].clone();
                if declaration
                    .generics
                    .iter()
                    .any(|parameter| !parameter.lifetime)
                {
                    return Err(err(
                        node.span,
                        "E_UNSUPPORTED",
                        "parameterized trait witnesses need explicit type arguments",
                    ));
                }
                pending.extend_from_slice(&declaration.bounds);
                // Associated projections are introduced lazily. A valid bound
                // such as A::Next: A must not eagerly create an infinite family.
                self.contracts
                    .as_mut()
                    .unwrap()
                    .traits
                    .entry((ty, trait_index))
                    .or_default();
            } else if !matches!(
                builtin,
                "Copy"
                    | "Clone"
                    | "Data"
                    | "Send"
                    | "Sync"
                    | "Ord"
                    | "Eq"
                    | "PartialOrd"
                    | "PartialEq"
            ) {
                return Err(err(
                    node.span,
                    "E_TRAIT",
                    "unknown declared trait constraint",
                ));
            }
        }
        Ok(())
    }

    fn definition_parameters(&mut self, index: usize) -> Result<BTreeMap<String, usize>> {
        self.contracts.as_mut().unwrap().active_scope = index;
        let function = self.source.functions[index].clone();
        let mut substitutions = BTreeMap::new();
        let owner = self.source.function_owner(a::FunctionId { index });
        if let a::FunctionOwner::Implementation(owner) = owner {
            let implementation = self.source.impls[owner.index].clone();
            self.declare_parameters(&implementation.generics, &mut substitutions)?;
            let self_ty = self.resolve(implementation.target, &substitutions)?;
            substitutions.insert("Self".to_string(), self_ty);
            let mut remaining = implementation.associated.clone();
            while !remaining.is_empty() {
                let before = remaining.len();
                let mut retry = Vec::new();
                for associated in remaining {
                    let value = associated.value.ok_or_else(|| {
                        err(
                            associated.span,
                            "E_TRAIT",
                            "implementation associated type needs a definition",
                        )
                    })?;
                    match self.resolve(value, &substitutions) {
                        Ok(ty) => {
                            substitutions.insert(format!("Self::{}", associated.name), ty);
                        }
                        Err(_) => retry.push(associated),
                    }
                }
                if retry.len() == before {
                    return Err(err(
                        implementation.span,
                        "E_TRAIT",
                        "associated types are cyclic or unresolved",
                    ));
                }
                remaining = retry;
            }
        } else if let a::FunctionOwner::Trait(owner) = owner {
            let trait_index = owner.index;
            let declaration = self.source.traits[trait_index].clone();
            self.declare_parameters(&declaration.generics, &mut substitutions)?;
            let self_ty = self.rigid_type("Self");
            substitutions.insert("Self".to_string(), self_ty);
            self.contracts
                .as_mut()
                .unwrap()
                .rigid
                .get_mut(&self_ty)
                .unwrap()
                .insert(TraitIdentity::Declared(trait_index));
            let mut associated = BTreeMap::new();
            for item in &declaration.associated {
                let ty = self.rigid_type(&format!("Self::{}", item.name));
                associated.insert(item.name.clone(), ty);
                substitutions.insert(format!("Self::{}", item.name), ty);
            }
            self.contracts
                .as_mut()
                .unwrap()
                .traits
                .insert((self_ty, trait_index), associated.clone());
            for bound in &declaration.bounds {
                self.assume_bound(self_ty, *bound, &substitutions)?;
            }
            for item in &declaration.associated {
                for bound in &item.bounds {
                    self.assume_bound(associated[&item.name], *bound, &substitutions)?;
                }
            }
        }
        self.declare_parameters(&function.generics, &mut substitutions)?;
        Ok(substitutions)
    }

    fn implementation_associated(
        &mut self,
        index: usize,
        mut environment: BTreeMap<String, usize>,
    ) -> Result<BTreeMap<String, usize>> {
        let implementation = self.source.impls[index].clone();
        self.prepare_callable_arguments(&implementation.generics, &mut environment)?;
        self.prepare_parameter_associated(&implementation.generics, &mut environment)?;
        let mut pending = implementation.associated.clone();
        if let Some(trait_type) = implementation.trait_ty {
            if let a::Type::Path(path) = &self.source.types[trait_type].kind {
                if let TraitIdentity::Declared(trait_index) =
                    crate::generics::bound_identity(self.source, path)
                {
                    for item in &self.source.traits[trait_index].associated {
                        if !pending.iter().any(|entry| entry.name == item.name) {
                            pending.push(item.clone());
                        }
                    }
                }
            }
        }
        let mut result = BTreeMap::new();
        while !pending.is_empty() {
            let before = pending.len();
            let mut retry = Vec::new();
            let mut failure = None;
            for associated in pending {
                let value = associated.value.ok_or_else(|| {
                    err(
                        associated.span,
                        "E_TRAIT",
                        "associated witness is missing its type definition",
                    )
                })?;
                match self.resolve(value, &environment) {
                    Ok(ty) => {
                        environment.insert(format!("Self::{}", associated.name), ty);
                        result.insert(associated.name, ty);
                    }
                    Err(error) => {
                        failure = Some(error);
                        retry.push(associated);
                    }
                }
            }
            if retry.len() == before {
                return Err(failure.unwrap());
            }
            pending = retry;
        }
        Ok(result)
    }

    pub(super) fn prepare_parameter_associated(
        &mut self,
        parameters: &[a::GenericParam],
        substitutions: &mut BTreeMap<String, usize>,
    ) -> Result<()> {
        for parameter in parameters {
            if parameter.lifetime {
                continue;
            }
            let Some(ty) = substitutions.get(&parameter.name).copied() else {
                continue;
            };
            if self
                .contracts
                .as_ref()
                .is_some_and(|contracts| contracts.rigid.contains_key(&ty))
            {
                continue;
            }
            for bound in &parameter.bounds {
                let path = match &self.source.types[*bound].kind {
                    a::Type::Path(path) if path.args.is_empty() => path,
                    _ => continue,
                };
                let identity = crate::generics::bound_identity(self.source, path);
                let TraitIdentity::Declared(trait_index) = identity else {
                    continue;
                };
                if self.source.traits[trait_index].associated.is_empty() {
                    continue;
                }
                let mut witness = self.implementation_witness(ty, &identity).ok_or_else(|| {
                    err(
                        parameter.span,
                        "E_TRAIT",
                        "associated projection needs its declared implementation witness",
                    )
                })?;
                witness.substitutions.insert("Self".to_string(), ty);
                let associated =
                    self.implementation_associated(witness.implementation, witness.substitutions)?;
                for (name, ty) in associated {
                    substitutions.insert(format!("{}::{name}", parameter.name), ty);
                }
            }
        }
        Ok(())
    }

    pub(super) fn prepare_associated(
        &mut self,
        index: usize,
        substitutions: &mut BTreeMap<String, usize>,
    ) -> Result<()> {
        let function = self.source.functions[index].clone();
        let owner = match self.source.function_owner(a::FunctionId { index }) {
            a::FunctionOwner::Implementation(owner) => Some(owner.index),
            _ => None,
        };
        let mut parameters = function.generics.clone();
        if let Some(owner) = owner {
            parameters.extend(self.source.impls[owner].generics.clone());
        }
        self.prepare_callable_arguments(&parameters, substitutions)?;
        self.prepare_parameter_associated(&parameters, substitutions)?;
        if let Some(owner) = owner {
            let mut environment = substitutions.clone();
            if !environment.contains_key("Self") {
                let ty = self.resolve(self.source.impls[owner].target, &environment)?;
                environment.insert("Self".to_string(), ty);
            }
            let associated = self.implementation_associated(owner, environment)?;
            for (name, ty) in associated {
                substitutions.insert(format!("Self::{name}"), ty);
            }
        }
        Ok(())
    }

    fn associated_identity(
        &mut self,
        receiver: usize,
        trait_index: usize,
        name: &str,
    ) -> Result<usize> {
        if let Some(ty) = self
            .contracts
            .as_ref()
            .unwrap()
            .traits
            .get(&(receiver, trait_index))
            .and_then(|items| items.get(name))
        {
            return Ok(*ty);
        }
        let declaration = self.source.traits[trait_index]
            .associated
            .iter()
            .find(|item| item.name == name)
            .cloned()
            .unwrap();
        let ty = self.rigid_type(&format!("t{receiver}::trait{trait_index}::{name}"));
        self.contracts
            .as_mut()
            .unwrap()
            .traits
            .entry((receiver, trait_index))
            .or_default()
            .insert(name.to_string(), ty);
        let mut substitutions = BTreeMap::new();
        substitutions.insert("Self".to_string(), receiver);
        substitutions.insert(format!("Self::{name}"), ty);
        for bound in declaration.bounds {
            self.assume_bound(ty, bound, &substitutions)?;
        }
        Ok(ty)
    }

    pub(super) fn associated_contract(
        &mut self,
        path: &a::Path,
        substitutions: &BTreeMap<String, usize>,
        span: a::Span,
    ) -> Result<Option<usize>> {
        let Some(contracts) = &self.contracts else {
            return Ok(None);
        };
        let Some((owner, name)) = path.name.rsplit_once("::") else {
            return Ok(None);
        };
        let Some(ty) = substitutions.get(owner) else {
            return Ok(None);
        };
        let mut declarations = Vec::new();
        for (receiver, trait_index) in contracts.traits.keys() {
            if receiver == ty
                && self.source.traits[*trait_index]
                    .associated
                    .iter()
                    .any(|item| item.name == name)
            {
                declarations.push(*trait_index);
            }
        }
        if declarations.len() > 1 {
            return Err(err(
                span,
                "E_TRAIT",
                "associated type name is ambiguous between declared trait bounds",
            ));
        }
        if let Some(trait_index) = declarations.first() {
            Ok(Some(self.associated_identity(*ty, *trait_index, name)?))
        } else {
            Ok(None)
        }
    }

    pub(super) fn depends_on_rigid(&self, ty: usize) -> bool {
        let Some(contracts) = &self.contracts else {
            return false;
        };
        let mut seen = BTreeSet::new();
        let mut pending = vec![ty];
        while let Some(ty) = pending.pop() {
            if !seen.insert(ty) {
                continue;
            }
            if contracts.rigid.contains_key(&ty) {
                return true;
            }
            match &self.types[ty].kind {
                k::TypeKind::Ref { inner, .. }
                | k::TypeKind::Slice { inner, .. }
                | k::TypeKind::Vec { inner }
                | k::TypeKind::Box { inner }
                | k::TypeKind::Array { inner, .. } => pending.push(*inner),
                k::TypeKind::Struct { fields, .. } => {
                    pending.extend(fields.iter().map(|field| field.ty))
                }
                k::TypeKind::Enum { variants, .. } => {
                    for variant in variants {
                        pending.extend(variant.fields.iter().map(|field| field.ty));
                    }
                }
                _ => {}
            }
        }
        false
    }

    pub(super) fn comparison_contract(
        &mut self,
        ty: usize,
        operation: &str,
        span: a::Span,
    ) -> Result<usize> {
        let required = if operation == "==" || operation == "!=" {
            "PartialEq"
        } else {
            "PartialOrd"
        };
        if !self.satisfies_bound(ty, required) {
            return Err(err(
                span,
                "E_TRAIT",
                &format!("generic comparison requires a declared {required} contract"),
            ));
        }
        let key = (ty, operation.to_string());
        if let Some(index) = self.contracts.as_ref().unwrap().comparisons.get(&key) {
            return Ok(*index);
        }
        let reference = self.reference(ty, false);
        let index = self.functions.len();
        self.functions.push(k::Function {
            file: span.file,
            name: format!("$contract:{operation}:{ty}"),
            params: 2,
            result: k::BOOL,
            return_source: None,
            locals: vec![
                k::Local {
                    name: "left".to_string(),
                    ty: reference,
                    mutable: false,
                },
                k::Local {
                    name: "right".to_string(),
                    ty: reference,
                    mutable: false,
                },
            ],
            body: Vec::new(),
            test: false,
        });
        self.contracts
            .as_mut()
            .unwrap()
            .comparisons
            .insert(key, index);
        Ok(index)
    }

    pub(super) fn builtin_method_contract(
        &mut self,
        receiver: usize,
        method: BuiltinMethod,
        span: a::Span,
    ) -> Result<usize> {
        if !self
            .contracts
            .as_ref()
            .unwrap()
            .rigid
            .contains_key(&receiver)
            || !self.satisfies_bound(receiver, method.required())
        {
            return Err(err(
                span,
                "E_TRAIT",
                &format!(
                    "generic method {} requires a declared {} contract",
                    method.name(),
                    method.required(),
                ),
            ));
        }
        let key = (receiver, method);
        if let Some(index) = self.contracts.as_ref().unwrap().builtin_methods.get(&key) {
            return Ok(*index);
        }
        let receiver_type = self.reference(receiver, method == BuiltinMethod::CloneFrom);
        let mut locals = vec![k::Local {
            name: "self".to_string(),
            ty: receiver_type,
            mutable: false,
        }];
        if method.arguments() != 0 {
            let shared = self.reference(receiver, false);
            locals.push(k::Local {
                name: "other".to_string(),
                ty: shared,
                mutable: false,
            });
        }
        let index = self.functions.len();
        self.functions.push(k::Function {
            file: span.file,
            name: format!("$contract:{}:{receiver}", method.name()),
            params: locals.len(),
            result: method.result(receiver),
            return_source: None,
            locals,
            body: Vec::new(),
            test: false,
        });
        self.contracts
            .as_mut()
            .unwrap()
            .builtin_methods
            .insert(key, index);
        Ok(index)
    }

    fn validate_builtin_methods(&mut self) -> Result<BTreeSet<usize>> {
        let methods = self.contracts.as_ref().unwrap().builtin_methods.clone();
        let mut checked = BTreeSet::new();
        for ((receiver, method), index) in &methods {
            let function = &self.functions[*index];
            let signature_matches = function.params == method.arguments() + 1
                && function.locals.len() == function.params
                && function.result == method.result(*receiver)
                && function.return_source.is_none()
                && function
                    .locals
                    .iter()
                    .enumerate()
                    .all(|(parameter, local)| {
                        matches!(self.types[local.ty].kind,
                        k::TypeKind::Ref { inner, mutable }
                        if inner == *receiver
                            && mutable == (parameter == 0 && *method == BuiltinMethod::CloneFrom))
                    });
            let file = function.file;
            if !self
                .contracts
                .as_ref()
                .unwrap()
                .rigid
                .contains_key(receiver)
                || !self.satisfies_bound(*receiver, method.required())
                || !signature_matches
            {
                return Err(err(
                    a::Span {
                        file,
                        ..a::Span::default()
                    },
                    "E_GENERIC",
                    "call does not match its typed builtin method witness",
                ));
            }
            checked.insert(*index);
        }
        Ok(checked)
    }

    pub(super) fn static_contract(
        &mut self,
        base: usize,
        name: &str,
        expression: a::ExprId,
        span: a::Span,
    ) -> Result<Option<Instance>> {
        if self.contracts.is_some() {
            let candidates: Vec<_> = self
                .declared_methods(base, name)?
                .into_iter()
                .filter(|candidate| {
                    self.source.functions[candidate.method]
                        .params
                        .first()
                        .is_none_or(|p| p.name != "self")
                })
                .collect();
            if candidates.len() > 1 {
                return Err(err(
                    span,
                    "E_TRAIT",
                    &format!(
                        "associated function {name} is ambiguous between declared trait bounds"
                    ),
                ));
            }
            if let Some(witness) = candidates.into_iter().next() {
                self.method_selections.insert(
                    expression,
                    MethodSelection {
                        dereferences: 0,
                        target: MethodTarget::Trait(TraitIdentity::Declared(witness.declaration)),
                    },
                );
                return Ok(Some(Instance {
                    source: witness.method,
                    substitutions: witness.substitutions,
                }));
            }
        } else if let Some(MethodSelection {
            target: MethodTarget::Trait(identity),
            ..
        }) = self.method_selections.get(&expression).cloned()
        {
            if !self.satisfies_identity(base, &identity) {
                return Err(err(
                    span,
                    "E_TRAIT",
                    "associated function receiver lacks its checked trait bound",
                ));
            }
            if let Some(witness) = self.implementation_witness(base, &identity) {
                if let Some(source) = self.source.impls[witness.implementation]
                    .methods
                    .iter()
                    .copied()
                    .find(|index| self.source.functions[*index].name == name)
                {
                    let mut substitutions = witness.substitutions;
                    substitutions.insert("Self".to_string(), base);
                    return Ok(Some(Instance {
                        source,
                        substitutions,
                    }));
                }
            }
            return Err(err(
                span,
                "E_UNSUPPORTED",
                &format!("selected associated function {name} has no executable implementation"),
            ));
        }
        Ok(None)
    }

    pub(super) fn record_static_contract(
        &mut self,
        called: usize,
        source: usize,
        substitutions: BTreeMap<String, usize>,
    ) {
        if let a::FunctionOwner::Trait(owner) =
            self.source.function_owner(a::FunctionId { index: source })
        {
            if let Some(contracts) = &mut self.contracts {
                contracts.methods.insert(
                    called,
                    MethodWitness {
                        declaration: owner.index,
                        method: source,
                        substitutions,
                    },
                );
            }
        }
    }

    fn declared_methods(&mut self, base: usize, name: &str) -> Result<Vec<MethodWitness>> {
        let Some(contracts) = &self.contracts else {
            return Ok(Vec::new());
        };
        let declarations: Vec<_> = contracts
            .traits
            .keys()
            .filter(|(receiver, _)| *receiver == base)
            .map(|(_, trait_index)| *trait_index)
            .collect();
        let mut candidates = Vec::new();
        for trait_index in declarations {
            let declaration = self.source.traits[trait_index].clone();
            for method in declaration.methods {
                if self.source.functions[method].name == name {
                    let mut substitutions = BTreeMap::new();
                    substitutions.insert("Self".to_string(), base);
                    for item in &declaration.associated {
                        let ty = self.associated_identity(base, trait_index, &item.name)?;
                        substitutions.insert(format!("Self::{}", item.name), ty);
                    }
                    candidates.push(MethodWitness {
                        declaration: trait_index,
                        method,
                        substitutions,
                    });
                }
            }
        }
        Ok(candidates)
    }
}

impl super::Builder<'_, '_> {
    fn builtin_method(
        &mut self,
        target: &k::Place,
        base: usize,
        method: BuiltinMethod,
        types: &[usize],
        args: &[usize],
        span: a::Span,
    ) -> Result<(usize, usize)> {
        if !types.is_empty() {
            return Err(err(
                span,
                "E_TYPE",
                "builtin methods take no generic arguments",
            ));
        }
        if args.len() != method.arguments() {
            return Err(err(
                span,
                "E_TYPE",
                "method argument count differs from builtin contract",
            ));
        }
        let called = self.compiler.builtin_method_contract(base, method, span)?;
        let receiver = self.borrow(
            target.clone(),
            method == BuiltinMethod::CloneFrom,
            span.line,
        )?;
        let mut values = vec![receiver.0];
        for argument in args {
            let expected = self.compiler.reference(base, false);
            let value = self.expr(*argument, Some(expected))?;
            if value.1 != expected {
                return Err(err(
                    span,
                    "E_TYPE",
                    "method argument type differs from builtin contract",
                ));
            }
            values.push(value.0);
        }
        let result = method.result(base);
        let dst = self.temp(result);
        self.emit(
            k::Op::Call {
                dst,
                function: called,
                args: values,
            },
            span.line,
        );
        Ok((dst, result))
    }

    /// Replay a trait selected by the generic body using the actual receiver's
    /// implementation evidence. Inherent methods cannot displace this choice.
    pub(super) fn witnessed_method(
        &mut self,
        target: &k::Place,
        base: usize,
        identity: &TraitIdentity,
        expression: a::ExprId,
        types: &[usize],
        args: &[usize],
    ) -> Result<(usize, usize)> {
        let span = self.compiler.source.exprs[expression].span;
        let name = match &self.compiler.source.exprs[expression].kind {
            a::Expr::Method { name, .. } => name.clone(),
            _ => {
                return Err(err(
                    span,
                    "E_GENERIC",
                    "method witness is not bound to a method expression",
                ));
            }
        };
        if !self.compiler.satisfies_identity(base, identity) {
            return Err(err(
                span,
                "E_TRAIT",
                "specialized receiver lacks its checked trait contract",
            ));
        }
        let implementation = self.compiler.implementation_witness(base, identity);
        if let Some(witness) = implementation {
            let implementation = &self.compiler.source.impls[witness.implementation];
            if let Some(index) = implementation
                .methods
                .iter()
                .find(|index| self.compiler.source.functions[**index].name == name)
                .copied()
            {
                return self.source_method(
                    target,
                    base,
                    Instance {
                        source: index,
                        substitutions: witness.substitutions,
                    },
                    types,
                    args,
                    span,
                );
            }
        }
        if let Some(method) = BuiltinMethod::named(&name) {
            if identity == &TraitIdentity::Builtin(method.required().to_string()) {
                return self.builtin_trait_method(target, base, &name, types, args, span);
            }
        }
        Err(err(
            span,
            "E_UNSUPPORTED",
            &format!("selected trait method {name} has no executable implementation"),
        ))
    }

    /// A source method selected while validating a known receiver remains the
    /// same declaration after substitution, even if another method then applies.
    pub(super) fn fixed_source_method(
        &mut self,
        target: &k::Place,
        base: usize,
        index: usize,
        types: &[usize],
        args: &[usize],
        span: a::Span,
    ) -> Result<(usize, usize)> {
        let implementation = match self.compiler.source.function_owner(a::FunctionId { index }) {
            a::FunctionOwner::Implementation(owner) => {
                Some(self.compiler.source.impls[owner.index].clone())
            }
            _ => None,
        }
        .ok_or_else(|| {
            err(
                span,
                "E_GENERIC",
                "checked source method has no implementation",
            )
        })?;
        let mut substitutions = BTreeMap::new();
        self.infer_type(implementation.target, base, &mut substitutions)?;
        substitutions.insert("Self".to_string(), base);
        if self
            .compiler
            .resolve(implementation.target, &substitutions)?
            != base
        {
            return Err(err(
                span,
                "E_GENERIC",
                "specialized receiver differs from checked method target",
            ));
        }
        self.compiler
            .bounds(&implementation.generics, &substitutions)?;
        self.source_method(
            target,
            base,
            Instance {
                source: index,
                substitutions,
            },
            types,
            args,
            span,
        )
    }

    pub(super) fn source_method(
        &mut self,
        target: &k::Place,
        base: usize,
        instance: Instance,
        types: &[usize],
        args: &[usize],
        span: a::Span,
    ) -> Result<(usize, usize)> {
        let index = instance.source;
        let mut substitutions = instance.substitutions;
        self.compiler.method_access(index, self.scope, span)?;
        let definition = self.compiler.source.functions[index].clone();
        substitutions.insert("Self".to_string(), base);
        let parameters: Vec<_> = definition
            .generics
            .iter()
            .filter(|parameter| !parameter.lifetime)
            .collect();
        if !types.is_empty() {
            if types.len() != parameters.len() {
                return Err(err(span, "E_TYPE", "wrong generic method argument count"));
            }
            for (parameter, argument) in parameters.iter().zip(types) {
                let ty = self.compiler.resolve(*argument, &self.sub)?;
                substitutions.insert(parameter.name.clone(), ty);
            }
        }
        if definition.params.is_empty() {
            return Err(err(span, "E_TYPE", "method has no receiver"));
        }
        if definition.params.len() != args.len() + 1 {
            return Err(err(span, "E_TYPE", "method argument count differs"));
        }
        let first = self
            .compiler
            .resolve(definition.params[0].ty, &substitutions)?;
        let receiver = if first == base {
            // A by-value Self may itself be a reference. Reborrowing it would
            // change the declared receiver from Self to &Self (or &mut Self).
            self.read(target.clone(), span.line)?
        } else if let k::TypeKind::Ref { inner, mutable } = self.compiler.types[first].kind {
            if inner != base {
                return Err(err(
                    span,
                    "E_TYPE",
                    "method receiver differs from its implementation target",
                ));
            }
            self.borrow(target.clone(), mutable, span.line)?
        } else {
            return Err(err(
                span,
                "E_TYPE",
                "method receiver differs from its implementation target",
            ));
        };
        let mut values = vec![receiver.0];
        for (parameter, argument) in definition.params[1..].iter().zip(args) {
            let expected = self.compiler.resolve(parameter.ty, &substitutions).ok();
            let value = self.expr(*argument, expected)?;
            self.infer_type(parameter.ty, value.1, &mut substitutions)?;
            values.push(value.0);
        }
        let called = self
            .compiler
            .instantiate(index, substitutions)
            .map_err(|problem| super::at_use_site(problem, span))?;
        let result = self.compiler.functions[called].result;
        let dst = self.temp(result);
        self.emit(
            k::Op::Call {
                dst,
                function: called,
                args: values,
            },
            span.line,
        );
        Ok((dst, result))
    }

    pub(super) fn generic_method(
        &mut self,
        expression: a::ExprId,
        target: &k::Place,
        base: usize,
        name: &str,
        types: &[usize],
        args: &[usize],
    ) -> Result<Option<(usize, usize)>> {
        let span = self.compiler.source.exprs[expression].span;
        let candidates = self.compiler.declared_methods(base, name)?;
        let builtin = BuiltinMethod::named(name).filter(|_| {
            self.compiler
                .contracts
                .as_ref()
                .is_some_and(|contracts| contracts.rigid.contains_key(&base))
        });
        let proven =
            builtin.filter(|method| self.compiler.satisfies_bound(base, method.required()));
        if candidates.len() + usize::from(proven.is_some()) > 1 {
            return Err(err(
                span,
                "E_TRAIT",
                "method is ambiguous between builtin and declared trait contracts",
            ));
        }
        if let Some(method) = proven {
            // A blanket source implementation can supply another method even
            // without an explicit bound. Do not silently choose the builtin.
            if self.has_declared_method(base, name, span)? {
                return Err(err(
                    span,
                    "E_TRAIT",
                    "method is ambiguous between trait contracts",
                ));
            }
            self.compiler
                .method_selections
                .get_mut(&expression)
                .unwrap()
                .target =
                MethodTarget::Trait(TraitIdentity::Builtin(method.required().to_string()));
            return self
                .builtin_method(target, base, method, types, args, span)
                .map(Some);
        }
        if candidates.is_empty() {
            if let Some(method) = builtin {
                // Preserve ordinary source implementation lookup, including
                // blanket implementations of a distinct same-spelled method.
                if !self.has_declared_method(base, name, span)? {
                    return Err(err(
                        span,
                        "E_TRAIT",
                        &format!(
                            "generic method {} requires a declared {} contract",
                            method.name(),
                            method.required(),
                        ),
                    ));
                }
            }
            return Ok(None);
        }
        let witness = candidates[0].clone();
        let trait_index = witness.declaration;
        self.compiler
            .method_selections
            .get_mut(&expression)
            .unwrap()
            .target = MethodTarget::Trait(TraitIdentity::Declared(trait_index));
        let index = witness.method;
        let mut substitutions = witness.substitutions;
        let function = self.compiler.source.functions[index].clone();
        let parameters: Vec<_> = function
            .generics
            .iter()
            .filter(|parameter| !parameter.lifetime)
            .collect();
        if !types.is_empty() {
            if types.len() != parameters.len() {
                return Err(err(span, "E_TYPE", "wrong generic method argument count"));
            }
            for (parameter, argument) in parameters.iter().zip(types) {
                let ty = self.compiler.resolve(*argument, &self.sub)?;
                substitutions.insert(parameter.name.clone(), ty);
            }
        }
        if function.params.len() != args.len() + 1 {
            return Err(err(
                span,
                "E_TYPE",
                "method argument count differs from declared contract",
            ));
        }
        let first = self
            .compiler
            .resolve(function.params[0].ty, &substitutions)?;
        let receiver = match self.compiler.types[first].kind {
            k::TypeKind::Ref { mutable, .. } => self.borrow(target.clone(), mutable, span.line)?,
            _ => self.read(target.clone(), span.line)?,
        };
        let mut values = vec![receiver.0];
        for (parameter, argument) in function.params[1..].iter().zip(args) {
            let expected = self.compiler.resolve(parameter.ty, &substitutions).ok();
            let value = self.expr(*argument, expected)?;
            self.infer_type(parameter.ty, value.1, &mut substitutions)?;
            values.push(value.0);
        }
        let called = self.compiler.instantiate(index, substitutions.clone())?;
        self.compiler.contracts.as_mut().unwrap().methods.insert(
            called,
            MethodWitness {
                declaration: trait_index,
                method: index,
                substitutions,
            },
        );
        let result = self.compiler.functions[called].result;
        let dst = self.temp(result);
        self.emit(
            k::Op::Call {
                dst,
                function: called,
                args: values,
            },
            span.line,
        );
        Ok(Some((dst, result)))
    }
}
