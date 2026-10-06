//! Independent test oracle for nominal witness inventories and structured method templates.
//! Reads source records directly; does not call production resolution or validation.
use super::*;
use crate::platform::semantic_id::TypeParameterId;
use std::collections::BTreeMap;

// The oracle follows source declarations and complete authored applications.
// An active declaration is distinct from a repeated, already admitted one; its
// original frame still admits every field before the declaration enters the cache.
struct WitnessAdmission {
    remaining: usize,
    active: BTreeSet<DeclarationReference>,
    admitted: BTreeSet<DeclarationReference>,
}

impl WitnessAdmission {
    fn new() -> Self {
        Self {
            remaining: contract::MAXIMUM_VALIDATION_WORK,
            active: BTreeSet::new(),
            admitted: BTreeSet::new(),
        }
    }

    fn checkpoint(&mut self, depth: usize) -> bool {
        if depth > contract::MAXIMUM_TYPE_DEPTH {
            return false;
        }
        let Some(remaining) = self.remaining.checked_sub(1) else {
            return false;
        };
        self.remaining = remaining;
        true
    }
}

// Test-oracle substitution owns the resulting type objects. It uses only source
// forms and canonical encoding, independently of production resolution.
struct WitnessTypes<'a> {
    source: &'a KernelSnapshot,
    derived: BTreeMap<TypeObjectDigest, TypeObject>,
    implementations: BTreeSet<(DeclarationReference, Vec<TypeObjectDigest>)>,
    remaining: usize,
}

impl<'a> WitnessTypes<'a> {
    fn new(source: &'a KernelSnapshot) -> Self {
        Self {
            source,
            derived: BTreeMap::new(),
            implementations: BTreeSet::new(),
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
        reference: DeclarationReference,
        i: &OwnedImplementation,
        arguments: &[TypeObjectDigest],
        depth: usize,
    ) -> Option<(TypeObjectDigest, Vec<TypeObjectDigest>)> {
        self.remaining = self.remaining.checked_sub(1)?;
        if depth > contract::MAXIMUM_TYPE_DEPTH || i.type_parameters.len() != arguments.len() {
            return None;
        }
        let bindings = i
            .type_parameters
            .iter()
            .copied()
            .zip(arguments.iter().copied())
            .collect();
        let application = (reference, arguments.to_vec());
        let self_type = self.apply(i.self_type, &bindings, 0)?;
        let arguments = i
            .type_arguments
            .iter()
            .map(|ty| self.apply(*ty, &bindings, 0))
            .collect::<Option<Vec<_>>>()?;
        let oracle = Oracle(self.source, None);
        self.contract(i.contract, self_type, &arguments)?;
        if !self.implementations.insert(application) {
            return Some((self_type, arguments));
        }
        for p in &i.implementation_parameters {
            let self_type = self.apply(p.self_type, &bindings, 0)?;
            let arguments = p
                .type_arguments
                .iter()
                .map(|ty| self.apply(*ty, &bindings, 0))
                .collect::<Option<Vec<_>>>()?;
            self.contract(p.contract, self_type, &arguments)?;
        }
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
            for p in &f.implementation_parameters {
                let self_type = self.apply(p.self_type, &target_bindings, 0)?;
                let arguments = p
                    .type_arguments
                    .iter()
                    .map(|ty| self.apply(*ty, &target_bindings, 0))
                    .collect::<Option<Vec<_>>>()?;
                self.contract(p.contract, self_type, &arguments)?;
            }
            for operand in &m.implementations {
                self.operand(operand, &bindings, depth + 1)?;
            }
        }
        Some((self_type, arguments))
    }

    fn operand(
        &mut self,
        operand: &ImplementationOperand,
        bindings: &BTreeMap<TypeParameterId, TypeObjectDigest>,
        depth: usize,
    ) -> Option<()> {
        self.remaining = self.remaining.checked_sub(1)?;
        if depth > contract::MAXIMUM_TYPE_DEPTH {
            return None;
        }
        if let ImplementationOperand::Concrete {
            implementation,
            type_arguments,
            implementations,
        } = operand
        {
            let arguments = type_arguments
                .iter()
                .map(|ty| self.apply(*ty, bindings, 0))
                .collect::<Option<Vec<_>>>()?;
            let oracle = Oracle(self.source, None);
            self.implementation(
                *implementation,
                oracle.implementation(*implementation)?,
                &arguments,
                depth + 1,
            )?;
            for operand in implementations {
                self.operand(operand, bindings, depth + 1)?;
            }
        }
        Some(())
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
    for (reference, i) in declarations {
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
        types.implementation(reference, i, &arguments, 0)?;
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
                types.operand(operand, &BTreeMap::new(), 0)?;
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
                && p.constraints.has_owned()
                && (p.constraints == TypeParameterConstraints::Owned
                    || (p.header.contract_version >= 30
                        && self
                            .declaration_generation(declaration)
                            .is_some_and(|v| v >= 30)))
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
                    if p.use_mode == ParameterUse::Unrestricted {
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
        if generation < 30
            && c.methods.iter().any(|m| {
                !matches!(m.effect, FunctionEffect::Pure)
                    && m.parameters
                        .iter()
                        .any(|p| p.use_mode == ParameterUse::Borrow)
            })
        {
            return false;
        }
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
    pub(super) fn valid_implementation_at(&self, reference: DeclarationReference) -> bool {
        self.admit_implementation(reference, &mut WitnessAdmission::new(), 0)
    }

    pub(super) fn operand_generation(
        &self,
        operand: &ImplementationOperand,
        generation: u16,
    ) -> bool {
        let mut pending = vec![(operand, 0)];
        let mut remaining = contract::MAXIMUM_VALIDATION_WORK;
        while let Some((operand, depth)) = pending.pop() {
            if depth > contract::MAXIMUM_TYPE_DEPTH || remaining == 0 {
                return false;
            }
            remaining -= 1;
            match operand {
                ImplementationOperand::Concrete {
                    type_arguments,
                    implementations,
                    ..
                } => {
                    if (generation < 28 && !type_arguments.is_empty())
                        || (generation < 29 && !implementations.is_empty())
                    {
                        return false;
                    }
                    pending.extend(implementations.iter().map(|operand| (operand, depth + 1)));
                }
                ImplementationOperand::Parameter { scope, .. } => {
                    if generation < 29 && self.function(*scope).is_none() {
                        return false;
                    }
                }
            }
        }
        true
    }

    fn admit_implementation(
        &self,
        reference: DeclarationReference,
        admission: &mut WitnessAdmission,
        depth: usize,
    ) -> bool {
        if !admission.checkpoint(depth) {
            return false;
        }
        if admission.admitted.contains(&reference) {
            return true;
        }
        if !admission.active.insert(reference) {
            return true;
        }
        let accepted = self
            .implementation(reference)
            .is_some_and(|i| self.implementation_template(reference, i, admission, depth + 1));
        admission.active.remove(&reference);
        if accepted {
            admission.admitted.insert(reference);
        }
        accepted
    }

    fn implementation_template(
        &self,
        reference: DeclarationReference,
        i: &OwnedImplementation,
        admission: &mut WitnessAdmission,
        depth: usize,
    ) -> bool {
        if !admission.checkpoint(depth) {
            return false;
        }
        let scoped = Oracle(self.0, Some(reference));
        let mut parameter_ids = BTreeSet::new();
        let mut parameter_names = BTreeSet::new();
        if !self.valid_contract(i.contract)
            || i.type_parameters.len() > contract::MAXIMUM_CHILDREN
            || i.type_parameters.iter().any(|id| {
                !parameter_ids.insert(*id)
                    || scoped.scoped_parameter(*id).is_none_or(|p| {
                        !p.constraints.has_owned() || !parameter_names.insert(&p.name)
                    })
            })
            || !scoped.owned_type_in_scope(i.self_type)
            || i.type_arguments.len() >= contract::MAXIMUM_CHILDREN
            || i.type_arguments
                .iter()
                .any(|ty| !scoped.owned_type_in_scope(*ty))
            || !scoped.valid_prerequisites(
                reference,
                &i.type_parameters,
                &i.implementation_parameters,
                true,
            )
            || i.methods.windows(2).any(|w| w[0].method >= w[1].method)
        {
            return false;
        }
        let Some(c) = self.contract(i.contract) else {
            return false;
        };
        if self
            .declaration_generation(reference)
            .is_none_or(|v| v < 30)
            && c.methods.iter().any(|m| {
                !matches!(m.effect, FunctionEffect::Pure)
                    && m.parameters
                        .iter()
                        .any(|p| p.use_mode == ParameterUse::Borrow)
            })
        {
            return false;
        }
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
            if self
                .declaration_generation(reference)
                .is_none_or(|v| v < 30)
                && f.type_parameters.iter().any(|id| {
                    self.type_parameter(mapping.function.package, *id)
                        .is_none_or(|p| p.constraints != TypeParameterConstraints::Owned)
                })
            {
                return false;
            }
            if f.effect != m.effect
                || f.type_parameters.len() != mapping.type_arguments.len()
                || !f.effect_parameters.is_empty()
                || !f.requirement_parameters.is_empty()
                || f.implementation_parameters.len() != mapping.implementations.len()
                || !self.valid_parameters(mapping.function, &f)
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
                        !p.constraints.has_owned()
                            || !target_names.insert(&p.name)
                            || !scoped.satisfies_constraints(*actual, p.constraints)
                    })
                    || !scoped.owned_type_in_scope(*actual)
                    || target_bindings.insert(*parameter, *actual).is_some()
                {
                    return false;
                }
            }
            if mapping
                .implementations
                .iter()
                .any(|operand| !scoped.mapping_operand(operand, reference, admission, depth + 1))
                || !scoped.matching_witnesses(
                    &f.implementation_parameters,
                    &target_bindings,
                    &mapping.implementations,
                    admission,
                    depth + 1,
                )
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
        // Unused mapped targets remain part of admission. Reconstruct their
        // versioned memory signature even when no expression invokes them.
        if self.signature(d).is_none() {
            return false;
        }
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
                    || (p.constraints.requires_share()
                        && (p.header.contract_version < 30
                            || self.declaration_generation(d).is_none_or(|v| v < 30)))
            })
        }) {
            return false;
        }
        self.valid_prerequisites(d, &f.type_parameters, &f.implementation_parameters, false)
    }

    fn valid_prerequisites(
        &self,
        d: DeclarationReference,
        type_parameters: &[TypeParameterId],
        parameters: &[ImplementationParameter],
        _exact_owned: bool,
    ) -> bool {
        if parameters.len() > contract::MAXIMUM_CHILDREN {
            return false;
        }
        let mut ids = BTreeSet::new();
        let mut names = BTreeSet::new();
        let scoped = Oracle(self.0, Some(d));
        parameters.iter().all(|p| {
            let Some(TypeForm::TypeParameter { parameter }) = self.form(p.self_type) else {
                return false;
            };
            ids.insert(p.id)
                && names.insert(&p.name)
                && type_parameters.contains(parameter)
                && scoped
                    .scoped_parameter(*parameter)
                    .is_some_and(|t| t.constraints.has_owned())
                && self.valid_contract(p.contract)
                && self
                    .contract(p.contract)
                    .is_some_and(|c| c.type_parameters.len() == p.type_arguments.len())
                && p.type_arguments
                    .iter()
                    .all(|ty| scoped.owned_type_in_scope(*ty))
        })
    }

    fn mapping_operand(
        &self,
        operand: &ImplementationOperand,
        declaration: DeclarationReference,
        admission: &mut WitnessAdmission,
        depth: usize,
    ) -> bool {
        match operand {
            ImplementationOperand::Parameter { scope, .. } => *scope == declaration,
            ImplementationOperand::Concrete { .. } => {
                Self::closed_operand(operand, admission, depth)
            }
        }
    }

    fn closed_operand(
        operand: &ImplementationOperand,
        admission: &mut WitnessAdmission,
        depth: usize,
    ) -> bool {
        if !admission.checkpoint(depth) {
            return false;
        }
        match operand {
            ImplementationOperand::Parameter { .. } => false,
            ImplementationOperand::Concrete {
                implementations, ..
            } => implementations
                .iter()
                .all(|operand| Self::closed_operand(operand, admission, depth + 1)),
        }
    }

    pub(super) fn witness(
        &self,
        operand: ImplementationOperand,
    ) -> Option<(
        DeclarationReference,
        TypeObjectDigest,
        Vec<TypeObjectDigest>,
    )> {
        self.admit_witness(&operand, &mut WitnessAdmission::new(), 0)
    }

    fn admit_witness(
        &self,
        operand: &ImplementationOperand,
        admission: &mut WitnessAdmission,
        depth: usize,
    ) -> Option<(
        DeclarationReference,
        TypeObjectDigest,
        Vec<TypeObjectDigest>,
    )> {
        if !admission.checkpoint(depth) {
            return None;
        }
        match operand {
            ImplementationOperand::Concrete {
                implementation,
                type_arguments,
                implementations,
            } => {
                let i = self.implementation(*implementation)?;
                if !self.admit_implementation(*implementation, admission, depth + 1)
                    || i.type_parameters.len() != type_arguments.len()
                    || i.implementation_parameters.len() != implementations.len()
                    || i.type_parameters
                        .iter()
                        .zip(type_arguments)
                        .any(|(id, ty)| {
                            self.type_parameter(implementation.package, *id)
                                .is_none_or(|p| !self.satisfies_constraints(*ty, p.constraints))
                        })
                {
                    return None;
                }
                let bindings = i
                    .type_parameters
                    .iter()
                    .copied()
                    .zip(type_arguments.iter().copied())
                    .collect();
                if !self.matching_witnesses(
                    &i.implementation_parameters,
                    &bindings,
                    implementations,
                    admission,
                    depth + 1,
                ) {
                    return None;
                }
                let mut derived = WitnessTypes::new(self.0);
                derived.remaining = admission.remaining;
                let self_type = derived.apply(i.self_type, &bindings, 0)?;
                let arguments = i
                    .type_arguments
                    .iter()
                    .map(|ty| derived.apply(*ty, &bindings, 0))
                    .collect::<Option<Vec<_>>>()?;
                admission.remaining = derived.remaining;
                Some((i.contract, self_type, arguments))
            }
            ImplementationOperand::Parameter { scope, parameter } => {
                if Some(*scope) != self.1 {
                    return None;
                }
                let parameters = if let Some(f) = self.function(*scope) {
                    if !self.valid_parameters(*scope, &f) {
                        return None;
                    }
                    f.implementation_parameters
                } else {
                    let i = self.implementation(*scope)?;
                    if !self.valid_prerequisites(
                        *scope,
                        &i.type_parameters,
                        &i.implementation_parameters,
                        true,
                    ) {
                        return None;
                    }
                    i.implementation_parameters.clone()
                };
                let p = parameters.iter().find(|p| p.id == *parameter)?;
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
        self.matching_witnesses(
            &f.implementation_parameters,
            &bindings,
            operands,
            &mut WitnessAdmission::new(),
            0,
        )
    }

    fn matching_witnesses(
        &self,
        parameters: &[ImplementationParameter],
        bindings: &BTreeMap<TypeParameterId, TypeObjectDigest>,
        operands: &[ImplementationOperand],
        admission: &mut WitnessAdmission,
        depth: usize,
    ) -> bool {
        parameters.len() == operands.len()
            && parameters.iter().zip(operands).all(|(p, operand)| {
                self.admit_witness(operand, admission, depth)
                    .is_some_and(|(c, ty, arguments)| {
                        c == p.contract
                            && self
                                .exact_application_type(
                                    p.self_type,
                                    ty,
                                    bindings,
                                    0,
                                    &mut admission.remaining,
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
                                        bindings,
                                        0,
                                        &mut admission.remaining,
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

#[cfg(test)]
mod prerequisite_tests {
    use super::*;

    const SOURCE: &str = r#"declarations.begin
(units (module create prerequisites
  (external create make (visibility private) (implementation core.cell.create)
    (parameter create value (type I64)) (returns OwnedI64Cell))
  (external create read (visibility private) (implementation core.cell.read)
    (parameter create value (type OwnedI64Cell) (use borrow)) (returns I64))
  (owned-contract create Reader (visibility public)
    (self Self) (type-parameter create Self (constraint owned))
    (method method_b9000000000000000000000000000001 read
      (parameters (Self borrow)) (returns I64)))
  (owned-contract create Transfer (visibility public)
    (self Self) (type-parameter create Self (constraint owned))
    (method method_b9000000000000000000000000000002 move
      (parameters (Self consume)) (returns Self)))
  (function create identity (visibility public) (effect pure)
    (type-parameter create T (constraint owned))
    (parameter create value (type T) (use consume)) (returns T)
    (body (local value)))
  (owned-implementation create Identity (visibility public)
    (type-parameter create T (constraint owned)) (contract Transfer) (self T)
    (method method_b9000000000000000000000000000002 identity (types T)))
  (function create cell-read (visibility public) (effect pure)
    (parameter create value (type OwnedI64Cell) (use borrow)) (returns I64)
    (body (call read (local value))))
  (owned-implementation create CellReader (visibility public)
    (contract Reader) (self OwnedI64Cell)
    (method method_b9000000000000000000000000000001 cell-read))
  (function create scalar-read (visibility public) (effect pure)
    (type-parameter create T (constraint owned))
    (implementation-parameter implparam_b9000000000000000000000000000001 reader Reader T)
    (implementation-parameter implparam_b9000000000000000000000000000002 transfer Transfer T)
    (parameter create value (type T) (use borrow)) (returns I64)
    (body (method-call parameter@scalar-read@implparam_b9000000000000000000000000000001
      Reader method_b9000000000000000000000000000001 (local value))))
  (owned-implementation create DelegatingReader (visibility public)
    (type-parameter create T (constraint owned))
    (implementation-parameter implparam_b9000000000000000000000000000003 reader Reader T)
    (implementation-parameter implparam_b9000000000000000000000000000004 transfer Transfer T)
    (contract Reader) (self T)
    (method method_b9000000000000000000000000000001 scalar-read (types T)
      (implementations
        parameter@DelegatingReader@implparam_b9000000000000000000000000000003
        parameter@DelegatingReader@implparam_b9000000000000000000000000000004)))
  (function create main (visibility public) (effect pure) (returns I64)
    (body (let (binding value (type OwnedI64Cell) (call make (i64 17)))
      (in (method-call
        (implementation DelegatingReader (types OwnedI64Cell)
          (implementations
            (implementation DelegatingReader (types OwnedI64Cell)
              (implementations concrete@CellReader (implementation Identity (types OwnedI64Cell))))
            (implementation Identity (types OwnedI64Cell))))
        Reader method_b9000000000000000000000000000001 (local value))))))))
declarations.end
"#;

    fn source() -> KernelSnapshot {
        crate::platform::execution::normalized::tests::byte_buffer_tests::author_only(SOURCE)
            .unwrap()
    }

    fn declaration(snapshot: &KernelSnapshot, name: &str) -> DeclarationReference {
        snapshot
            .owners
            .iter()
            .find_map(|(key, owner)| match (key, owner) {
                (OwnerKey::Declaration(id), OwnerRecord::Declaration(owner))
                    if owner.name.as_str() == name =>
                {
                    Some(DeclarationReference {
                        package: snapshot.root.package_id,
                        declaration: *id,
                    })
                }
                _ => None,
            })
            .unwrap()
    }

    fn implementation(snapshot: &mut KernelSnapshot) -> &mut OwnedImplementation {
        let declaration = declaration(snapshot, "DelegatingReader");
        let OwnerRecord::Declaration(owner) = snapshot
            .owners
            .get_mut(&OwnerKey::Declaration(declaration.declaration))
            .unwrap()
        else {
            unreachable!()
        };
        let DeclarationPayload::OwnedImplementation(implementation) = &mut owner.payload else {
            unreachable!()
        };
        implementation
    }

    fn application(snapshot: &mut KernelSnapshot) -> &mut ImplementationOperand {
        snapshot
            .owners
            .values_mut()
            .find_map(|owner| match owner {
                OwnerRecord::Expression(expression) => match &mut expression.operation {
                    ExpressionOperation::MethodCall { witness, .. }
                        if matches!(witness, ImplementationOperand::Concrete { .. }) =>
                    {
                        Some(witness)
                    }
                    _ => None,
                },
                _ => None,
            })
            .unwrap()
    }

    #[test]
    fn oracle_admits_nested_prerequisites_and_unused_ordered_mapping() {
        assert!(accepts(&source()));
    }

    #[test]
    fn oracle_rejects_forged_prerequisite_scopes_types_order_and_nested_maps() {
        let baseline = source();
        assert!(accepts(&baseline));

        let mut changed = baseline.clone();
        let foreign = declaration(&changed, "scalar-read");
        let ImplementationOperand::Parameter { scope, .. } =
            &mut implementation(&mut changed).methods[0].implementations[0]
        else {
            unreachable!()
        };
        *scope = foreign;
        assert!(
            !accepts(&changed),
            "a matching type grants no foreign lexical witness"
        );

        let mut changed = baseline.clone();
        let item = implementation(&mut changed).implementation_parameters[1].self_type;
        let object = TypeObject::new(TypeForm::OwnedSequence { item }).unwrap();
        let ty = encode_type_object(&object).unwrap().0;
        changed.types.insert(ty, object);
        implementation(&mut changed).implementation_parameters[1].self_type = ty;
        assert!(
            !accepts(&changed),
            "unused prerequisite Self must be an exact scheme parameter"
        );

        let mut changed = baseline.clone();
        let ImplementationOperand::Concrete {
            implementations, ..
        } = application(&mut changed)
        else {
            unreachable!()
        };
        implementations.swap(0, 1);
        assert!(
            !accepts(&changed),
            "ordered prerequisite contracts must match exactly"
        );

        let mut changed = baseline.clone();
        let ImplementationOperand::Concrete {
            implementations, ..
        } = application(&mut changed)
        else {
            unreachable!()
        };
        let ImplementationOperand::Concrete {
            implementations, ..
        } = &mut implementations[0]
        else {
            unreachable!()
        };
        implementations.pop();
        assert!(
            !accepts(&changed),
            "unused nested prerequisites remain required"
        );

        let mut changed = baseline.clone();
        let reference = declaration(&changed, "DelegatingReader");
        let scheme = implementation(&mut changed);
        scheme.methods[0].implementations[0] = ImplementationOperand::Concrete {
            implementation: reference,
            type_arguments: vec![scheme.implementation_parameters[0].self_type],
            implementations: scheme.methods[0].implementations.clone(),
        };
        assert!(
            !accepts(&changed),
            "method-map constructors cannot hide lexical prerequisites"
        );

        let mut changed = baseline;
        changed.root.graph_contract_version = 28;
        assert!(
            !accepts(&changed),
            "graph generation must admit prerequisite source fields"
        );
    }
}
