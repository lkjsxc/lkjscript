//! Independent test oracle for nominal witness inventories and structured method templates.
//! Reads source records directly; does not call production resolution or validation.
use super::*;
use crate::platform::semantic_id::TypeParameterId;
use std::collections::BTreeMap;

// Test-oracle substitution owns the resulting type objects. It uses only source
// forms and canonical encoding, independently of production resolution.
struct WitnessTypes<'a> {
    source: &'a KernelSnapshot,
    derived: BTreeMap<TypeObjectDigest, TypeObject>,
    remaining: usize,
}

impl<'a> WitnessTypes<'a> {
    fn new(source: &'a KernelSnapshot) -> Self {
        Self {
            source,
            derived: BTreeMap::new(),
            remaining: contract::MAXIMUM_VALIDATION_WORK,
        }
    }
    fn apply(
        &mut self,
        ty: TypeObjectDigest,
        bindings: &BTreeMap<TypeParameterId, TypeObjectDigest>,
        depth: usize,
    ) -> Option<TypeObjectDigest> {
        self.remaining = self.remaining.checked_sub(1)?;
        if depth > contract::MAXIMUM_TYPE_DEPTH {
            return None;
        }
        let mut object = self
            .source
            .types
            .get(&ty)
            .or_else(|| self.source.dependency_types.get(&ty))
            .or_else(|| self.derived.get(&ty))?
            .clone();
        let mut child = |ty| self.apply(ty, bindings, depth + 1);
        match &mut object.form {
            TypeForm::TypeParameter { parameter } => {
                return Some(bindings.get(parameter).copied().unwrap_or(ty));
            }
            TypeForm::Applied { arguments, .. } => {
                for ty in arguments {
                    *ty = child(*ty)?;
                }
            }
            TypeForm::List { item }
            | TypeForm::Option { item }
            | TypeForm::Stream { item }
            | TypeForm::OwnedSequence { item } => *item = child(*item)?,
            TypeForm::Map { key, value }
            | TypeForm::Result {
                ok: key,
                error: value,
            } => {
                *key = child(*key)?;
                *value = child(*value)?;
            }
            TypeForm::Function { parameters, result }
            | TypeForm::TaskFunction {
                parameters, result, ..
            } => {
                for ty in parameters {
                    *ty = child(*ty)?;
                }
                *result = child(*result)?;
            }
            TypeForm::StructuralRecord { fields }
            | TypeForm::OwnedProduct { fields }
            | TypeForm::OwnedChoice { cases: fields } => {
                for field in fields {
                    field.ty = child(field.ty)?;
                }
            }
            _ => {}
        }
        let digest = encode_type_object(&object).ok()?.0;
        if !self.source.types.contains_key(&digest)
            && !self.source.dependency_types.contains_key(&digest)
        {
            self.derived.entry(digest).or_insert(object);
        }
        Some(digest)
    }

    fn implementation(
        &mut self,
        i: &OwnedImplementation,
        arguments: &[TypeObjectDigest],
    ) -> Option<(TypeObjectDigest, Vec<TypeObjectDigest>)> {
        if i.type_parameters.len() != arguments.len() {
            return None;
        }
        let bindings = i
            .type_parameters
            .iter()
            .copied()
            .zip(arguments.iter().copied())
            .collect();
        let self_type = self.apply(i.self_type, &bindings, 0)?;
        let arguments = i
            .type_arguments
            .iter()
            .map(|ty| self.apply(*ty, &bindings, 0))
            .collect::<Option<Vec<_>>>()?;
        let oracle = Oracle(self.source, None);
        self.contract(i.contract, self_type, &arguments)?;
        for m in &i.methods {
            let f = oracle.function(m.function)?;
            if f.type_parameters.len() != m.type_arguments.len() {
                return None;
            }
            let mut target_bindings = BTreeMap::new();
            for (formal, actual) in f.type_parameters.iter().zip(&m.type_arguments) {
                target_bindings.insert(*formal, self.apply(*actual, &bindings, 0)?);
            }
            for p in &f.parameters {
                self.apply(
                    oracle.parameter(m.function.package, *p)?.ty,
                    &target_bindings,
                    0,
                )?;
            }
            self.apply(f.result, &target_bindings, 0)?;
        }
        Some((self_type, arguments))
    }

    fn contract(
        &mut self,
        contract: DeclarationReference,
        self_type: TypeObjectDigest,
        arguments: &[TypeObjectDigest],
    ) -> Option<()> {
        let oracle = Oracle(self.source, None);
        let c = oracle.contract(contract)?;
        if c.type_parameters.len() != arguments.len() {
            return None;
        }
        let bindings = std::iter::once((c.self_parameter, self_type))
            .chain(
                c.type_parameters
                    .iter()
                    .copied()
                    .zip(arguments.iter().copied()),
            )
            .collect();
        for m in &c.methods {
            for p in &m.parameters {
                self.apply(p.ty, &bindings, 0)?;
            }
            self.apply(m.result, &bindings, 0)?;
        }
        Some(())
    }
}

pub(super) fn materialized_witness_types(
    source: &KernelSnapshot,
) -> Option<BTreeMap<TypeObjectDigest, TypeObject>> {
    let mut types = WitnessTypes::new(source);
    let oracle = Oracle(source, None);
    let mut declarations = Vec::new();
    for (key, owner) in &source.owners {
        if let (OwnerKey::Declaration(id), OwnerRecord::Declaration(owner)) = (key, owner)
            && let DeclarationPayload::OwnedImplementation(i) = &owner.payload
        {
            declarations.push((
                DeclarationReference {
                    package: source.root.package_id,
                    declaration: *id,
                },
                i,
            ));
        }
        if let OwnerRecord::Declaration(owner) = owner
            && let DeclarationPayload::Function(function) = &owner.payload
        {
            for parameter in &function.implementation_parameters {
                types.contract(
                    parameter.contract,
                    parameter.self_type,
                    &parameter.type_arguments,
                )?;
            }
        }
    }
    for (package, dependency) in &source.dependencies {
        let interface = source
            .dependency_interfaces
            .get(&dependency.package_revision)?;
        for (key, owner) in interface {
            if let (OwnerKey::Declaration(id), PackageInterfaceRecord::Declaration(owner)) =
                (key, owner)
                && let PackageInterfaceDeclarationPayload::OwnedImplementation(i) = &owner.payload
            {
                declarations.push((
                    DeclarationReference {
                        package: *package,
                        declaration: *id,
                    },
                    i,
                ));
            }
            if let PackageInterfaceRecord::Declaration(owner) = owner
                && let PackageInterfaceDeclarationPayload::Function(function) = &owner.payload
            {
                for parameter in &function.implementation_parameters {
                    types.contract(
                        parameter.contract,
                        parameter.self_type,
                        &parameter.type_arguments,
                    )?;
                }
            }
        }
    }
    for (_, i) in declarations {
        let mut arguments = Vec::new();
        for parameter in &i.type_parameters {
            let object = TypeObject::new(TypeForm::TypeParameter {
                parameter: *parameter,
            })
            .ok()?;
            let ty = encode_type_object(&object).ok()?.0;
            types.derived.entry(ty).or_insert(object);
            arguments.push(ty);
        }
        types.implementation(i, &arguments)?;
    }
    for owner in source.owners.values() {
        if let OwnerRecord::Expression(expression) = owner {
            let operands = match &expression.operation {
                ExpressionOperation::ImplementationCall {
                    implementations, ..
                } => implementations.as_slice(),
                ExpressionOperation::MethodCall { witness, .. } => std::slice::from_ref(witness),
                _ => &[],
            };
            for operand in operands {
                if let ImplementationOperand::Concrete {
                    implementation,
                    type_arguments,
                } = operand
                {
                    types
                        .implementation(oracle.implementation(*implementation)?, type_arguments)?;
                }
            }
        }
    }
    Some(types.derived)
}

impl Oracle<'_> {
    pub(super) fn form(&self, ty: TypeObjectDigest) -> Option<&TypeForm> {
        self.0
            .types
            .get(&ty)
            .or_else(|| self.0.dependency_types.get(&ty))
            .map(|t| &t.form)
    }
    pub(super) fn type_parameter(
        &self,
        package: PackageId,
        id: TypeParameterId,
    ) -> Option<&TypeParameterRecord> {
        if package == self.0.root.package_id {
            match self.0.owners.get(&OwnerKey::TypeParameter(id))? {
                OwnerRecord::TypeParameter(p) => Some(p),
                _ => None,
            }
        } else {
            match self.foreign(package, OwnerKey::TypeParameter(id))? {
                PackageInterfaceRecord::TypeParameter(p) => Some(p),
                _ => None,
            }
        }
    }
    pub(super) fn scoped_parameter(&self, id: TypeParameterId) -> Option<&TypeParameterRecord> {
        let declaration = self.1?;
        let p = self.type_parameter(declaration.package, id)?;
        if p.header.owner != OwnerKey::TypeParameter(id) || p.declaration != declaration.declaration
        {
            return None;
        }
        let listed = if let Some(function) = self.function(declaration) {
            function.type_parameters.contains(&id)
        } else if let Some(contract) = self.contract(declaration) {
            (contract.self_parameter == id || contract.type_parameters.contains(&id))
                && p.constraints == TypeParameterConstraints::Owned
        } else if let Some(implementation) = self.implementation(declaration) {
            implementation.type_parameters.contains(&id)
                && p.constraints == TypeParameterConstraints::Owned
        } else {
            false
        };
        listed.then_some(p)
    }
    pub(super) fn function(&self, d: DeclarationReference) -> Option<PackageFunctionSignature> {
        if d.package == self.0.root.package_id {
            let OwnerRecord::Declaration(d) =
                self.0.owners.get(&OwnerKey::Declaration(d.declaration))?
            else {
                return None;
            };
            let DeclarationPayload::Function(f) = &d.payload else {
                return None;
            };
            Some(PackageFunctionSignature {
                implementation_parameters: f.implementation_parameters.clone(),
                type_parameters: f.type_parameters.clone(),
                effect_parameters: f.effect_parameters.clone(),
                requirement_parameters: f.requirement_parameters.clone(),
                parameters: f.parameters.clone(),
                result: f.result,
                result_borrow: f.result_borrow,
                effect: f.effect.clone(),
            })
        } else {
            let PackageInterfaceRecord::Declaration(d) =
                self.foreign(d.package, OwnerKey::Declaration(d.declaration))?
            else {
                return None;
            };
            let PackageInterfaceDeclarationPayload::Function(f) = &d.payload else {
                return None;
            };
            Some(f.clone())
        }
    }
    pub(super) fn contract(&self, d: DeclarationReference) -> Option<&OwnedContract> {
        if d.package == self.0.root.package_id {
            let OwnerRecord::Declaration(d) =
                self.0.owners.get(&OwnerKey::Declaration(d.declaration))?
            else {
                return None;
            };
            let DeclarationPayload::OwnedContract(c) = &d.payload else {
                return None;
            };
            Some(c)
        } else {
            let PackageInterfaceRecord::Declaration(d) =
                self.foreign(d.package, OwnerKey::Declaration(d.declaration))?
            else {
                return None;
            };
            let PackageInterfaceDeclarationPayload::OwnedContract(c) = &d.payload else {
                return None;
            };
            Some(c)
        }
    }
    pub(super) fn implementation(&self, d: DeclarationReference) -> Option<&OwnedImplementation> {
        if d.package == self.0.root.package_id {
            let OwnerRecord::Declaration(d) =
                self.0.owners.get(&OwnerKey::Declaration(d.declaration))?
            else {
                return None;
            };
            let DeclarationPayload::OwnedImplementation(i) = &d.payload else {
                return None;
            };
            Some(i)
        } else {
            let PackageInterfaceRecord::Declaration(d) =
                self.foreign(d.package, OwnerKey::Declaration(d.declaration))?
            else {
                return None;
            };
            let PackageInterfaceDeclarationPayload::OwnedImplementation(i) = &d.payload else {
                return None;
            };
            Some(i)
        }
    }
    pub(super) fn nominal(
        &self,
        d: DeclarationReference,
    ) -> Option<(Vec<TypeParameterId>, Vec<TypeObjectDigest>)> {
        let (parameters, fields, cases) = if d.package == self.0.root.package_id {
            let OwnerRecord::Declaration(owner) =
                self.0.owners.get(&OwnerKey::Declaration(d.declaration))?
            else {
                return None;
            };
            match &owner.payload {
                DeclarationPayload::Record {
                    type_parameters,
                    fields,
                } => (type_parameters.clone(), fields.clone(), vec![]),
                DeclarationPayload::Variant {
                    type_parameters,
                    cases,
                } => (type_parameters.clone(), vec![], cases.clone()),
                _ => return None,
            }
        } else {
            let PackageInterfaceRecord::Declaration(owner) =
                self.foreign(d.package, OwnerKey::Declaration(d.declaration))?
            else {
                return None;
            };
            match &owner.payload {
                PackageInterfaceDeclarationPayload::Record {
                    type_parameters,
                    fields,
                } => (type_parameters.clone(), fields.clone(), vec![]),
                PackageInterfaceDeclarationPayload::Variant {
                    type_parameters,
                    cases,
                } => (type_parameters.clone(), vec![], cases.clone()),
                _ => return None,
            }
        };
        let mut types = vec![];
        for f in fields {
            let field = if d.package == self.0.root.package_id {
                match self.0.owners.get(&OwnerKey::Field(f))? {
                    OwnerRecord::Field(f) => f,
                    _ => return None,
                }
            } else {
                match self.foreign(d.package, OwnerKey::Field(f))? {
                    PackageInterfaceRecord::Field(f) => f,
                    _ => return None,
                }
            };
            if field.declaration != d.declaration {
                return None;
            }
            types.push(field.ty);
        }
        for c in cases {
            let case = if d.package == self.0.root.package_id {
                match self.0.owners.get(&OwnerKey::Case(c))? {
                    OwnerRecord::Case(c) => c,
                    _ => return None,
                }
            } else {
                match self.foreign(d.package, OwnerKey::Case(c))? {
                    PackageInterfaceRecord::Case(c) => c,
                    _ => return None,
                }
            };
            if case.declaration != d.declaration {
                return None;
            }
            types.extend(case.payload);
        }
        Some((parameters, types))
    }
    pub(super) fn ordinary(&self, ty: TypeObjectDigest) -> bool {
        self.ordinary_assuming(ty, BTreeSet::new())
    }
    pub(super) fn capture_safe(&self, ty: TypeObjectDigest) -> bool {
        self.data_assuming(ty, BTreeSet::new(), true)
    }
    pub(super) fn ordinary_assuming(
        &self,
        ty: TypeObjectDigest,
        assumptions: BTreeSet<TypeParameterId>,
    ) -> bool {
        self.data_assuming(ty, assumptions, false)
    }
    fn data_assuming(
        &self,
        ty: TypeObjectDigest,
        assumptions: BTreeSet<TypeParameterId>,
        capture: bool,
    ) -> bool {
        // A nominal's formals stand for arbitrary ordinary types; its actuals
        // must independently prove that assumption, even when unused in fields.
        let mut pending = vec![(ty, assumptions)];
        let mut seen = BTreeSet::new();
        while let Some((ty, bindings)) = pending.pop() {
            if !seen.insert((ty, bindings.clone())) {
                continue;
            }
            // Finite test oracle; exhaustion is not acceptance of a recursive expansion.
            if seen.len() > contract::MAXIMUM_CHILDREN {
                return false;
            }
            let Some(form) = self.form(ty) else {
                return false;
            };
            match form {
                TypeForm::TypeParameter { parameter } => {
                    if !bindings.contains(parameter)
                        && !self.scoped_parameter(*parameter).is_some_and(|p| {
                            if capture {
                                p.constraints.proves_capture_safe()
                            } else {
                                !p.constraints.has_owned() && p.constraints.requires_transfer()
                            }
                        })
                    {
                        return false;
                    }
                }
                TypeForm::Function { .. } | TypeForm::TaskFunction { .. } if capture => {}
                TypeForm::Function { .. }
                | TypeForm::TaskFunction { .. }
                | TypeForm::CapabilityResource { .. }
                | TypeForm::ByteBuffer
                | TypeForm::OwnedI64Cell
                | TypeForm::OwnedProduct { .. }
                | TypeForm::OwnedChoice { .. }
                | TypeForm::OwnedSequence { .. }
                | TypeForm::Secret
                | TypeForm::Stream { .. } => return false,
                TypeForm::Named { declaration } | TypeForm::Applied { declaration, .. } => {
                    let Some((parameters, fields)) = self.nominal(*declaration) else {
                        return false;
                    };
                    let arguments = match form {
                        TypeForm::Applied { arguments, .. } => arguments.as_slice(),
                        _ => &[],
                    };
                    if parameters.len() != arguments.len() {
                        return false;
                    }
                    let mut nested = BTreeSet::new();
                    for p in parameters {
                        if !self
                            .type_parameter(declaration.package, p)
                            .is_some_and(|record| {
                                record.declaration == declaration.declaration
                                    && !record.constraints.has_owned()
                                    && !record.constraints.requires_transfer()
                            })
                        {
                            return false;
                        }
                        nested.insert(p);
                    }
                    pending.extend(fields.into_iter().map(|t| (t, nested.clone())));
                    pending.extend(arguments.iter().map(|t| (*t, bindings.clone())));
                }
                TypeForm::Unit
                | TypeForm::Bool
                | TypeForm::I64
                | TypeForm::F64
                | TypeForm::Bytes
                | TypeForm::Text
                | TypeForm::StaticText
                | TypeForm::StructuralRecord { .. }
                | TypeForm::List { .. }
                | TypeForm::Map { .. }
                | TypeForm::Option { .. }
                | TypeForm::Result { .. } => {
                    let object = self
                        .0
                        .types
                        .get(&ty)
                        .or_else(|| self.0.dependency_types.get(&ty));
                    let Some(object) = object else {
                        return false;
                    };
                    pending.extend(
                        object
                            .child_types()
                            .into_iter()
                            .map(|t| (t, bindings.clone())),
                    );
                }
            }
        }
        true
    }
    pub(super) fn valid_contract(&self, d: DeclarationReference) -> bool {
        let Some(c) = self.contract(d) else {
            return false;
        };
        let closed = Oracle(self.0, None);
        let scoped = Oracle(self.0, Some(d));
        let mut parameters = BTreeSet::new();
        let mut parameter_names = BTreeSet::new();
        if c.type_parameters.len() >= contract::MAXIMUM_CHILDREN
            || !std::iter::once(c.self_parameter)
                .chain(c.type_parameters.iter().copied())
                .all(|id| {
                    parameters.insert(id)
                        && scoped.scoped_parameter(id).is_some_and(|p| {
                            p.constraints == TypeParameterConstraints::Owned
                                && (id == c.self_parameter || p.header.contract_version >= 26)
                                && parameter_names.insert(&p.name)
                        })
                })
            || c.methods.is_empty()
            || c.methods.len() > crate::platform::kernel::contract::MAXIMUM_CHILDREN
        {
            return false;
        }
        let mut ids = BTreeSet::new();
        let mut names = BTreeSet::new();
        for m in &c.methods {
            if !ids.insert(m.id)
                || !names.insert(&m.name)
                || m.parameters.len() > crate::platform::kernel::contract::MAXIMUM_CHILDREN
                || m.effect.row().validate().is_err()
                || !m.effect.row().is_closed()
            {
                return false;
            }
            let mut suffix = false;
            for p in &m.parameters {
                if scoped.owned_type_in_scope(p.ty) {
                    suffix = true;
                    if p.use_mode == ParameterUse::Unrestricted
                        || (!matches!(m.effect, FunctionEffect::Pure)
                            && p.use_mode != ParameterUse::Consume)
                    {
                        return false;
                    }
                } else if suffix
                    || p.use_mode != ParameterUse::Unrestricted
                    || !closed.ordinary(p.ty)
                {
                    return false;
                }
            }
            if !scoped.owned_type_in_scope(m.result) && !closed.ordinary(m.result) {
                return false;
            }
            if let Some(source) = m.result_borrow
                && (!matches!(m.effect, FunctionEffect::Pure)
                    || !scoped.owned_type_in_scope(m.result)
                    || m.parameters.get(source as usize).is_none_or(|p| {
                        p.use_mode != ParameterUse::Borrow || !scoped.owned_type_in_scope(p.ty)
                    }))
            {
                return false;
            }
        }
        true
    }
    pub(super) fn contract_generation(&self, c: &OwnedContract, generation: u16) -> bool {
        if generation < 27 && c.methods.iter().any(|m| m.result_borrow.is_some()) {
            return false;
        }
        if generation >= 26 {
            return true;
        }
        let closed = Oracle(self.0, None);
        c.type_parameters.is_empty()
            && c.methods.iter().all(|method| {
                method
                    .parameters
                    .iter()
                    .map(|parameter| parameter.ty)
                    .chain([method.result])
                    .all(|ty| {
                        self.form(ty)
                            == Some(&TypeForm::TypeParameter {
                                parameter: c.self_parameter,
                            })
                            || closed.ordinary(ty)
                    })
            })
    }
    pub(super) fn method_type(
        &self,
        ty: TypeObjectDigest,
        bindings: &BTreeMap<TypeParameterId, TypeObjectDigest>,
    ) -> TypeObjectDigest {
        // Complete witness applications were independently materialized before
        // flow admission; no production-derived object or classification is used.
        WitnessTypes::new(self.0)
            .apply(ty, bindings, 0)
            .unwrap_or(ty)
    }
    pub(super) fn valid_implementation(&self, i: &OwnedImplementation) -> bool {
        let reference = self.0.owners.iter().find_map(|(key, owner)| match (key, owner) {
            (OwnerKey::Declaration(id), OwnerRecord::Declaration(owner))
                if matches!(&owner.payload, DeclarationPayload::OwnedImplementation(candidate) if candidate == i) =>
                    Some(DeclarationReference { package: self.0.root.package_id, declaration: *id }),
            _ => None,
        }).or_else(|| self.0.dependencies.iter().find_map(|(package, dependency)| {
            self.0.dependency_interfaces.get(&dependency.package_revision)?.iter().find_map(|(key, owner)| match (key, owner) {
                (OwnerKey::Declaration(id), PackageInterfaceRecord::Declaration(owner))
                    if matches!(&owner.payload, PackageInterfaceDeclarationPayload::OwnedImplementation(candidate) if candidate == i) =>
                        Some(DeclarationReference { package: *package, declaration: *id }),
                _ => None,
            })
        }));
        let Some(reference) = reference else {
            return false;
        };
        let scoped = Oracle(self.0, Some(reference));
        let mut parameter_ids = BTreeSet::new();
        let mut parameter_names = BTreeSet::new();
        if !self.valid_contract(i.contract)
            || i.type_parameters.len() > contract::MAXIMUM_CHILDREN
            || i.type_parameters.iter().any(|id| {
                !parameter_ids.insert(*id)
                    || scoped.scoped_parameter(*id).is_none_or(|p| {
                        p.constraints != TypeParameterConstraints::Owned
                            || !parameter_names.insert(&p.name)
                    })
            })
            || !scoped.owned_type_in_scope(i.self_type)
            || i.type_arguments.len() >= contract::MAXIMUM_CHILDREN
            || i.type_arguments
                .iter()
                .any(|ty| !scoped.owned_type_in_scope(*ty))
            || i.methods.windows(2).any(|w| w[0].method >= w[1].method)
        {
            return false;
        }
        let Some(c) = self.contract(i.contract) else {
            return false;
        };
        if c.methods.len() != i.methods.len() || c.type_parameters.len() != i.type_arguments.len() {
            return false;
        }
        let bindings: BTreeMap<_, _> = std::iter::once((c.self_parameter, i.self_type))
            .chain(
                c.type_parameters
                    .iter()
                    .copied()
                    .zip(i.type_arguments.iter().copied()),
            )
            .collect();
        let mut derived = WitnessTypes::new(self.0);
        for m in &c.methods {
            let Some(mapping) = i.methods.iter().find(|v| v.method == m.id) else {
                return false;
            };
            let Some(f) = self.function(mapping.function) else {
                return false;
            };
            if f.effect != m.effect
                || f.type_parameters.len() != mapping.type_arguments.len()
                || !f.effect_parameters.is_empty()
                || !f.requirement_parameters.is_empty()
                || !f.implementation_parameters.is_empty()
                || f.parameters.len() != m.parameters.len()
                || f.result_borrow
                    != m.result_borrow
                        .and_then(|index| f.parameters.get(index as usize).copied())
            {
                return false;
            }
            let target_scope = Oracle(self.0, Some(mapping.function));
            let mut target_ids = BTreeSet::new();
            let mut target_names = BTreeSet::new();
            let mut target_bindings = BTreeMap::new();
            for (parameter, actual) in f.type_parameters.iter().zip(&mapping.type_arguments) {
                if !target_ids.insert(*parameter)
                    || target_scope.scoped_parameter(*parameter).is_none_or(|p| {
                        p.constraints != TypeParameterConstraints::Owned
                            || !target_names.insert(&p.name)
                    })
                    || !scoped.owned_type_in_scope(*actual)
                    || target_bindings.insert(*parameter, *actual).is_some()
                {
                    return false;
                }
            }
            for (id, expected) in f.parameters.iter().zip(&m.parameters) {
                let Some(p) = self.parameter(mapping.function.package, *id) else {
                    return false;
                };
                if p.parent != ParameterParent::Function(mapping.function.declaration)
                    || p.resource_requirement.is_some()
                    || p.use_mode != expected.use_mode
                    || (!target_scope.owned_type_in_scope(p.ty)
                        && !Oracle(self.0, None).ordinary(p.ty))
                    || derived
                        .apply(expected.ty, &bindings, 0)
                        .is_none_or(|expected| {
                            derived.apply(p.ty, &target_bindings, 0) != Some(expected)
                        })
                {
                    return false;
                }
            }
            if (!target_scope.owned_type_in_scope(f.result)
                && !Oracle(self.0, None).ordinary(f.result))
                || derived
                    .apply(m.result, &bindings, 0)
                    .is_none_or(|expected| {
                        derived.apply(f.result, &target_bindings, 0) != Some(expected)
                    })
            {
                return false;
            }
        }
        true
    }
    pub(super) fn valid_parameters(
        &self,
        d: DeclarationReference,
        f: &PackageFunctionSignature,
    ) -> bool {
        if let Some(source) = f.result_borrow {
            let scoped = Oracle(self.0, Some(d));
            if !matches!(f.effect, FunctionEffect::Pure)
                || !scoped.owned_type_in_scope(f.result)
                || !f.parameters.contains(&source)
                || self.parameter(d.package, source).is_none_or(|p| {
                    p.header.owner != OwnerKey::Parameter(source)
                        || p.parent != ParameterParent::Function(d.declaration)
                        || p.use_mode != ParameterUse::Borrow
                        || !scoped.owned_type_in_scope(p.ty)
                })
            {
                return false;
            }
        }
        if f.type_parameters.iter().any(|id| {
            self.type_parameter(d.package, *id).is_none_or(|p| {
                p.declaration != d.declaration
                    || p.header.owner != OwnerKey::TypeParameter(*id)
                    || (p.constraints.requires_transfer() && p.header.contract_version < 22)
            })
        }) {
            return false;
        }
        if f.implementation_parameters.is_empty() {
            return true;
        }
        let mut ids = BTreeSet::new();
        let mut names = BTreeSet::new();
        let scoped = Oracle(self.0, Some(d));
        f.implementation_parameters.iter().all(|p| {
            let Some(TypeForm::TypeParameter { parameter }) = self.form(p.self_type) else {
                return false;
            };
            ids.insert(p.id)
                && names.insert(&p.name)
                && f.type_parameters.contains(parameter)
                && self
                    .type_parameter(d.package, *parameter)
                    .is_some_and(|t| t.declaration == d.declaration && t.constraints.has_owned())
                && self.valid_contract(p.contract)
                && self
                    .contract(p.contract)
                    .is_some_and(|c| c.type_parameters.len() == p.type_arguments.len())
                && p.type_arguments
                    .iter()
                    .all(|ty| scoped.owned_type_in_scope(*ty))
        })
    }
    pub(super) fn witness(
        &self,
        operand: ImplementationOperand,
    ) -> Option<(
        DeclarationReference,
        TypeObjectDigest,
        Vec<TypeObjectDigest>,
    )> {
        match operand {
            ImplementationOperand::Concrete {
                implementation,
                type_arguments,
            } => {
                let i = self.implementation(implementation)?;
                if !self.valid_implementation(i)
                    || i.type_parameters.len() != type_arguments.len()
                    || type_arguments
                        .iter()
                        .any(|ty| !self.owned_type_in_scope(*ty))
                {
                    return None;
                }
                let mut derived = WitnessTypes::new(self.0);
                let (self_type, arguments) = derived.implementation(i, &type_arguments)?;
                Some((i.contract, self_type, arguments))
            }
            ImplementationOperand::Parameter {
                function,
                parameter,
            } => {
                if function.package != self.0.root.package_id || Some(function) != self.1 {
                    return None;
                }
                let f = self.function(function)?;
                if !self.valid_parameters(function, &f) {
                    return None;
                }
                let p = f
                    .implementation_parameters
                    .iter()
                    .find(|p| p.id == parameter)?;
                self.valid_contract(p.contract).then_some((
                    p.contract,
                    p.self_type,
                    p.type_arguments.clone(),
                ))
            }
        }
    }
    pub(super) fn witnesses(
        &self,
        function: DeclarationReference,
        types: &[TypeObjectDigest],
        operands: &[ImplementationOperand],
    ) -> bool {
        let Some(f) = self.function(function) else {
            return false;
        };
        if f.implementation_parameters.len() != operands.len()
            || f.type_parameters.len() != types.len()
            || !self.valid_parameters(function, &f)
        {
            return false;
        }
        let bindings: BTreeMap<_, _> = f
            .type_parameters
            .iter()
            .copied()
            .zip(types.iter().copied())
            .collect();
        let mut remaining = contract::MAXIMUM_VALIDATION_WORK;
        f.implementation_parameters
            .iter()
            .zip(operands)
            .all(|(p, operand)| {
                self.witness(operand.clone())
                    .is_some_and(|(c, ty, arguments)| {
                        c == p.contract
                            && self
                                .exact_application_type(
                                    p.self_type,
                                    ty,
                                    &bindings,
                                    0,
                                    &mut remaining,
                                )
                                .is_some()
                            && p.type_arguments.len() == arguments.len()
                            && p.type_arguments
                                .iter()
                                .zip(&arguments)
                                .all(|(template, actual)| {
                                    self.exact_application_type(
                                        *template,
                                        *actual,
                                        &bindings,
                                        0,
                                        &mut remaining,
                                    )
                                    .is_some()
                                })
                    })
            })
    }
    pub(super) fn method(
        &self,
        witness: ImplementationOperand,
        contract: DeclarationReference,
        method: crate::platform::semantic_id::MethodId,
    ) -> Option<(OwnedMethod, BTreeMap<TypeParameterId, TypeObjectDigest>)> {
        let (actual, ty, arguments) = self.witness(witness)?;
        if actual != contract {
            return None;
        }
        let c = self.contract(contract)?;
        if c.type_parameters.len() != arguments.len() {
            return None;
        }
        let bindings = std::iter::once((c.self_parameter, ty))
            .chain(c.type_parameters.iter().copied().zip(arguments))
            .collect();
        Some((c.methods.iter().find(|m| m.id == method)?.clone(), bindings))
    }
}
