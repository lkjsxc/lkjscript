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

#[derive(Default)]
pub(super) struct Contracts {
    pub(super) rigid: BTreeMap<usize, BTreeSet<TraitIdentity>>,
    // (rigid receiver, resolved trait declaration) -> associated-type identities.
    traits: BTreeMap<(usize, usize), BTreeMap<String, usize>>,
    methods: BTreeMap<usize, MethodWitness>,
    comparisons: BTreeMap<(usize, String), usize>,
}

pub(super) fn validate(source: &a::Program) -> Result<()> {
    let mut roots = Vec::new();
    for (index, function) in source.functions.iter().enumerate() {
        if function.body.is_none() {
            continue;
        }
        if function
            .generics
            .iter()
            .any(|parameter| !parameter.lifetime)
            || source
                .impls
                .iter()
                .any(|item| item.methods.contains(&index))
            || source
                .traits
                .iter()
                .any(|item| item.methods.contains(&index))
        {
            roots.push(index);
        }
    }
    let mut compiler = Compiler::new(source)?;
    compiler.contracts = Some(Contracts::default());
    super::derive_check::validate(&mut compiler)?;
    if roots.is_empty() {
        return Ok(());
    }
    let mut checked_roots = BTreeSet::new();
    for index in roots {
        let substitutions = compiler.definition_parameters(index)?;
        let function = compiler.instantiate(index, substitutions.clone())?;
        let instance = Instance {
            source: index,
            substitutions,
        };
        compiler.functions[function] = compiler.compile_function(function, &instance)?;
        checked_roots.insert(function);
    }

    // User destructors retain their ordinary K obligations. Their definitions
    // were checked above; this declaration pass attaches concrete signatures to
    // the abstract nominal layouts created while checking those definitions.
    let mut next = 0;
    while next < compiler.types.len() {
        compiler.register_drop(next)?;
        next += 1;
    }

    // A call uses a checked declaration's signature, or an identified declared
    // trait/builtin contract. It does not recursively re-check a specialization
    // of the callee. Each production definition has its own rigid root above;
    // nongeneric free functions still go through the normal concrete pipeline.
    let declared: BTreeSet<usize> = compiler.instance_names.values().copied().collect();
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
        let builtin = contracts.comparisons.values().any(|value| *value == index);
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
            op: k::Op::Trap {
                message: "abstract declared contract".to_string(),
            },
            line: 1,
        }];
    }
    let program = k::Program {
        types: compiler.types,
        functions: compiler.functions,
        destructors: compiler.destructors,
    };
    let _checked_definition_meaning = crate::check::check(program)?;
    Ok(())
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
        for parameter in parameters {
            if parameter.lifetime {
                continue;
            }
            let ty = substitutions[&parameter.name];
            for bound in &parameter.bounds {
                self.assume_bound(ty, *bound, substitutions)?;
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
        _substitutions: &BTreeMap<String, usize>,
    ) -> Result<()> {
        let mut pending = vec![first];
        while let Some(id) = pending.pop() {
            let node = self.source.types[id].clone();
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
        let function = self.source.functions[index].clone();
        let mut substitutions = BTreeMap::new();
        if let Some(implementation) = self
            .source
            .impls
            .iter()
            .find(|item| item.methods.contains(&index))
            .cloned()
        {
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
        } else if let Some((trait_index, declaration)) = self
            .source
            .traits
            .iter()
            .enumerate()
            .find(|(_, item)| item.methods.contains(&index))
            .map(|(index, item)| (index, item.clone()))
        {
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

    pub(super) fn prepare_associated(
        &mut self,
        index: usize,
        substitutions: &mut BTreeMap<String, usize>,
    ) -> Result<()> {
        let function = self.source.functions[index].clone();
        let owner = self
            .source
            .impls
            .iter()
            .position(|item| item.methods.contains(&index));
        let mut parameters = function.generics.clone();
        if let Some(owner) = owner {
            parameters.extend(self.source.impls[owner].generics.clone());
        }
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
            for bound in parameter.bounds {
                let path = match &self.source.types[bound].kind {
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
                let empty = BTreeMap::new();
                let rigid = self
                    .contracts
                    .as_ref()
                    .map(|contracts| &contracts.rigid)
                    .unwrap_or(&empty);
                let mut witness = crate::generics::implementation_witness(
                    self.source,
                    &self.types,
                    &self.applications,
                    ty,
                    &identity,
                    rigid,
                )
                .ok_or_else(|| {
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

    pub(super) fn satisfies_bound(&self, ty: usize, bound: &str) -> bool {
        self.satisfies_identity(ty, &TraitIdentity::Builtin(bound.to_string()))
    }

    pub(super) fn satisfies_identity(&self, ty: usize, bound: &TraitIdentity) -> bool {
        if let Some(contracts) = &self.contracts {
            crate::generics::satisfies_under(
                self.source,
                &self.types,
                &self.applications,
                ty,
                bound,
                &contracts.rigid,
            )
        } else {
            crate::generics::satisfies_resolved(
                self.source,
                &self.types,
                &self.applications,
                ty,
                bound,
            )
        }
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

    fn depends_on_rigid(&self, ty: usize) -> bool {
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

    fn comparison_contract(&mut self, ty: usize, operation: &str, span: a::Span) -> Result<usize> {
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
    fn generic_comparison_place(&mut self, expression: usize) -> Result<k::Place> {
        if matches!(&self.compiler.source.exprs[expression].kind,a::Expr::Unary{op,..} if op=="*") {
            self.place(expression)
        } else {
            self.addressable(expression)
        }
    }

    pub(super) fn generic_comparison(
        &mut self,
        operation: &str,
        left: usize,
        right: usize,
        span: a::Span,
    ) -> Result<Option<(usize, usize)>> {
        let Some(ty) = self.hint(left).or_else(|| self.hint(right)) else {
            return Ok(None);
        };
        if !self.compiler.depends_on_rigid(ty) {
            return Ok(None);
        }
        let contract = self.compiler.comparison_contract(ty, operation, span)?;
        let lp = self.generic_comparison_place(left)?;
        let lt = super::place_type(&self.compiler.types, self.function, &lp)
            .ok_or_else(|| err(span, "E_TYPE", "invalid comparison operand"))?;
        let left = self.borrow(lp, false, span.line)?.0;
        let rp = self.generic_comparison_place(right)?;
        let rt = super::place_type(&self.compiler.types, self.function, &rp)
            .ok_or_else(|| err(span, "E_TYPE", "invalid comparison operand"))?;
        if lt != ty || rt != ty {
            return Err(err(span, "E_TYPE", "comparison operand types differ"));
        }
        let right = self.borrow(rp, false, span.line)?.0;
        let dst = self.temp(k::BOOL);
        self.emit(
            k::Op::Call {
                dst,
                function: contract,
                args: vec![left, right],
            },
            span.line,
        );
        Ok(Some((dst, k::BOOL)))
    }

    pub(super) fn generic_method(
        &mut self,
        target: &k::Place,
        base: usize,
        name: &str,
        types: &[usize],
        args: &[usize],
        span: a::Span,
    ) -> Result<Option<(usize, usize)>> {
        let candidates = self.compiler.declared_methods(base, name)?;
        if candidates.is_empty() {
            return Ok(None);
        }
        if candidates.len() != 1 {
            return Err(err(
                span,
                "E_TRAIT",
                "method is ambiguous between declared trait bounds",
            ));
        }
        let witness = candidates[0].clone();
        let trait_index = witness.declaration;
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
