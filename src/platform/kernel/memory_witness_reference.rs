//! Independent test oracle for nominal witness inventories and first-order method types.
//! Reads source records directly; does not call production resolution or validation.
use super::*;
use crate::platform::semantic_id::{DeclarationId, TypeParameterId};
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
    pub(super) fn ordinary_assuming(
        &self,
        ty: TypeObjectDigest,
        assumptions: BTreeSet<TypeParameterId>,
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
                    if !bindings.contains(parameter) {
                        return false;
                    }
                }
                TypeForm::Function { .. }
                | TypeForm::TaskFunction { .. }
                | TypeForm::CapabilityResource { .. }
                | TypeForm::ByteBuffer
                | TypeForm::OwnedI64Cell
                | TypeForm::OwnedProduct { .. }
                | TypeForm::OwnedChoice { .. }
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
                                    && record.constraints != TypeParameterConstraints::Owned
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
        if !self
            .type_parameter(d.package, c.self_parameter)
            .is_some_and(|p| {
                p.declaration == d.declaration && p.constraints == TypeParameterConstraints::Owned
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
                if self.form(p.ty)
                    == Some(&TypeForm::TypeParameter {
                        parameter: c.self_parameter,
                    })
                {
                    suffix = true;
                    if p.use_mode == ParameterUse::Unrestricted
                        || (!matches!(m.effect, FunctionEffect::Pure)
                            && p.use_mode != ParameterUse::Consume)
                    {
                        return false;
                    }
                } else if suffix || p.use_mode != ParameterUse::Unrestricted || !self.ordinary(p.ty)
                {
                    return false;
                }
            }
            if self.form(m.result)
                != Some(&TypeForm::TypeParameter {
                    parameter: c.self_parameter,
                })
                && !self.ordinary(m.result)
            {
                return false;
            }
        }
        true
    }
    pub(super) fn method_type(
        &self,
        ty: TypeObjectDigest,
        parameter: TypeParameterId,
        actual: TypeObjectDigest,
    ) -> TypeObjectDigest {
        if self.form(ty) == Some(&TypeForm::TypeParameter { parameter }) {
            actual
        } else {
            ty
        }
    }
    pub(super) fn valid_implementation(&self, i: &OwnedImplementation) -> bool {
        if !self.valid_contract(i.contract)
            || !matches!(
                self.form(i.self_type),
                Some(
                    TypeForm::ByteBuffer
                        | TypeForm::OwnedI64Cell
                        | TypeForm::OwnedProduct { .. }
                        | TypeForm::OwnedChoice { .. }
                )
            )
            || i.methods.windows(2).any(|w| w[0].method >= w[1].method)
            || matches!(
                self.form(i.self_type),
                Some(TypeForm::OwnedProduct { .. } | TypeForm::OwnedChoice { .. })
            ) && !Oracle(self.0, None).product_shape(i.self_type)
        {
            return false;
        }
        let Some(c) = self.contract(i.contract) else {
            return false;
        };
        if c.methods.len() != i.methods.len() {
            return false;
        }
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
                    || p.ty != self.method_type(expected.ty, c.self_parameter, i.self_type)
                {
                    return false;
                }
            }
            if f.result != self.method_type(m.result, c.self_parameter, i.self_type) {
                return false;
            }
        }
        true
    }
    pub(super) fn valid_parameters(&self, d: DeclarationId, f: &FunctionDeclaration) -> bool {
        if f.implementation_parameters.is_empty() {
            return true;
        }
        if !f.effect_parameters.is_empty() || !f.requirement_parameters.is_empty() {
            return false;
        }
        let mut ids = BTreeSet::new();
        let mut names = BTreeSet::new();
        f.implementation_parameters.iter().all(|p| {
            let Some(TypeForm::TypeParameter { parameter }) = self.form(p.self_type) else {
                return false;
            };
            ids.insert(p.id)
                && names.insert(&p.name)
                && f.type_parameters.contains(parameter)
                && self
                    .type_parameter(self.0.root.package_id, *parameter)
                    .is_some_and(|t| {
                        t.declaration == d && t.constraints == TypeParameterConstraints::Owned
                    })
                && self.valid_contract(p.contract)
        })
    }
    pub(super) fn witness(
        &self,
        operand: ImplementationOperand,
    ) -> Option<(DeclarationReference, TypeObjectDigest)> {
        match operand {
            ImplementationOperand::Concrete { implementation } => {
                let i = self.implementation(implementation)?;
                self.valid_implementation(i)
                    .then_some((i.contract, i.self_type))
            }
            ImplementationOperand::Parameter {
                function,
                parameter,
            } => {
                if function.package != self.0.root.package_id
                    || Some(function.declaration) != self.1
                {
                    return None;
                }
                let f = self.function(function)?;
                let p = f
                    .implementation_parameters
                    .iter()
                    .find(|p| p.id == parameter)?;
                self.valid_contract(p.contract)
                    .then_some((p.contract, p.self_type))
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
        {
            return false;
        }
        f.implementation_parameters
            .iter()
            .zip(operands)
            .all(|(p, operand)| {
                let expected = match self.form(p.self_type) {
                    Some(TypeForm::TypeParameter { parameter }) => f
                        .type_parameters
                        .iter()
                        .position(|t| t == parameter)
                        .and_then(|index| types.get(index))
                        .copied(),
                    _ => None,
                };
                self.witness(*operand)
                    .is_some_and(|(c, ty)| c == p.contract && Some(ty) == expected)
            })
    }
    pub(super) fn method(
        &self,
        witness: ImplementationOperand,
        contract: DeclarationReference,
        method: crate::platform::semantic_id::MethodId,
    ) -> Option<OwnedMethod> {
        let (actual, ty) = self.witness(witness)?;
        if actual != contract {
            return None;
        }
        let c = self.contract(contract)?;
        let mut m = c.methods.iter().find(|m| m.id == method)?.clone();
        for p in &mut m.parameters {
            p.ty = self.method_type(p.ty, c.self_parameter, ty);
        }
        m.result = self.method_type(m.result, c.self_parameter, ty);
        Some(m)
    }
}
