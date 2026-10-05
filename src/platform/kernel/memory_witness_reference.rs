//! Independent test oracle for nominal witness inventories and structured method templates.
//! Reads source records directly; does not call production resolution or validation.
use super::*;
use crate::platform::semantic_id::TypeParameterId;
use std::collections::BTreeMap;

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
        // Ownership classification needs only the root binding. Structural
        // equality is independently checked against the original template;
        // this oracle never synthesizes a production-derived type object.
        match self.form(ty) {
            Some(TypeForm::TypeParameter { parameter }) => {
                bindings.get(parameter).copied().unwrap_or(ty)
            }
            _ => ty,
        }
    }
    pub(super) fn valid_implementation(&self, i: &OwnedImplementation) -> bool {
        let closed = Oracle(self.0, None);
        if !self.valid_contract(i.contract)
            || !closed.owned_type_in_scope(i.self_type)
            || i.type_arguments.len() >= contract::MAXIMUM_CHILDREN
            || i.type_arguments
                .iter()
                .any(|ty| !closed.owned_type_in_scope(*ty))
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
        let mut remaining = contract::MAXIMUM_VALIDATION_WORK;
        for m in &c.methods {
            let Some(mapping) = i.methods.iter().find(|v| v.method == m.id) else {
                return false;
            };
            let Some(f) = self.function(mapping.function) else {
                return false;
            };
            if f.effect != m.effect
                || !f.type_parameters.is_empty()
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
            for (id, expected) in f.parameters.iter().zip(&m.parameters) {
                let Some(p) = self.parameter(mapping.function.package, *id) else {
                    return false;
                };
                if p.parent != ParameterParent::Function(mapping.function.declaration)
                    || p.resource_requirement.is_some()
                    || p.use_mode != expected.use_mode
                    || self
                        .exact_application_type(expected.ty, p.ty, &bindings, 0, &mut remaining)
                        .is_none()
                {
                    return false;
                }
            }
            if self
                .exact_application_type(m.result, f.result, &bindings, 0, &mut remaining)
                .is_none()
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
            ImplementationOperand::Concrete { implementation } => {
                let i = self.implementation(implementation)?;
                self.valid_implementation(i).then_some((
                    i.contract,
                    i.self_type,
                    i.type_arguments.clone(),
                ))
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
                self.witness(*operand).is_some_and(|(c, ty, arguments)| {
                    c == p.contract
                        && self
                            .exact_application_type(p.self_type, ty, &bindings, 0, &mut remaining)
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
