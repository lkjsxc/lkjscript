//! Test-only occurrence authority and witness-selected substitution over canonical
//! inventories. No production associated resolver or admitted carrier is an input.
//! Resolution alone does not certify callable/method compatibility. Full call
//! boundaries additionally require complete environment and witness admission.
use super::*;
use crate::platform::semantic_id::{
    AssociatedTypeMemberId, ImplementationParameterId, TypeParameterId,
};

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(super) enum Context {
    Contract(DeclarationReference),
    Lexical(DeclarationReference),
    Forbidden,
}

pub(super) struct Invocation {
    pub parameters: Vec<(TypeObjectDigest, TypeObjectDigest, ParameterUse)>,
    pub result: TypeObjectDigest,
    pub result_borrow: Option<usize>,
    pub arguments: Vec<ExpressionId>,
    pub derived: BTreeMap<TypeObjectDigest, TypeObject>,
}

impl Oracle<'_> {
    pub(super) fn associated_operand(&self, operand: &ImplementationOperand) -> bool {
        match operand {
            ImplementationOperand::Concrete {
                implementation,
                type_arguments,
                implementations,
            } => {
                type_arguments.iter().any(|ty| self.associated_closure(*ty))
                    || self.implementation(*implementation).is_some_and(|i| {
                        !i.associated_types.is_empty()
                            || self
                                .contract(i.contract)
                                .is_some_and(|c| !c.associated_types.is_empty())
                            || std::iter::once(i.self_type)
                                .chain(i.type_arguments.iter().copied())
                                .any(|ty| self.associated_closure(ty))
                    })
                    || implementations
                        .iter()
                        .any(|operand| self.associated_operand(operand))
            }
            ImplementationOperand::Parameter { scope, parameter } => {
                let entries = self
                    .function(*scope)
                    .map(|f| f.implementation_parameters)
                    .or_else(|| {
                        self.implementation(*scope)
                            .map(|i| i.implementation_parameters.clone())
                    });
                entries.is_some_and(|entries| {
                    entries.iter().any(|p| {
                        p.id == *parameter
                            && self
                                .contract(p.contract)
                                .is_some_and(|c| !c.associated_types.is_empty())
                    })
                })
            }
        }
    }
    pub(super) fn associated_invocation_required(&self, id: ExpressionId) -> bool {
        match self.expression(id) {
            Some(
                ExpressionOperation::Call {
                    function,
                    type_arguments,
                    ..
                }
                | ExpressionOperation::ImplementationCall {
                    function,
                    type_arguments,
                    ..
                },
            ) => {
                type_arguments.iter().any(|ty| self.associated_closure(*ty))
                    || self.signature(*function).is_some_and(|s| {
                        s.0.iter()
                            .map(|p| p.ty)
                            .chain([s.1])
                            .any(|ty| self.associated_closure(ty))
                    })
                    || matches!(self.expression(id), Some(ExpressionOperation::ImplementationCall { implementations, .. }) if implementations.iter().any(|operand| self.associated_operand(operand)))
            }
            Some(ExpressionOperation::MethodCall {
                witness, contract, ..
            }) => {
                self.associated_operand(witness)
                    || self
                        .contract(*contract)
                        .is_some_and(|c| !c.associated_types.is_empty())
            }
            _ => false,
        }
    }
    pub(super) fn associated_invocation(&self, id: ExpressionId) -> Option<Invocation> {
        let caller = self.occurrence((self.0.root.package_id, OwnerKey::Expression(id)));
        let mut resolver = Resolver::new(self.0);
        let (environment, parameters, result, result_borrow, arguments) =
            match self.expression(id)? {
                ExpressionOperation::Call {
                    function,
                    type_arguments,
                    arguments,
                    effect_arguments,
                    requirement_arguments,
                }
                | ExpressionOperation::ImplementationCall {
                    function,
                    type_arguments,
                    arguments,
                    effect_arguments,
                    requirement_arguments,
                    ..
                } => {
                    let f = self.function(*function)?;
                    let operands = match self.expression(id)? {
                        ExpressionOperation::ImplementationCall {
                            implementations, ..
                        } => implementations.as_slice(),
                        _ => &[],
                    };
                    if !effect_arguments.is_empty()
                        || !requirement_arguments.is_empty()
                        || !f.effect_parameters.is_empty()
                        || !f.requirement_parameters.is_empty()
                        || f.type_parameters.len() != type_arguments.len()
                        || f.implementation_parameters.len() != operands.len()
                        || !self.valid_parameters(*function, &f)
                        || operands
                            .iter()
                            .any(|operand| self.witness(operand.clone()).is_none())
                    {
                        return None;
                    }
                    let environment = Environment {
                        ordinary: f
                            .type_parameters
                            .iter()
                            .copied()
                            .zip(type_arguments.iter().copied())
                            .collect(),
                        source: Context::Lexical(*function),
                        caller,
                        contract_operand: None,
                        witnesses: f
                            .implementation_parameters
                            .iter()
                            .map(|p| p.id)
                            .zip(operands.iter().cloned())
                            .collect(),
                    };
                    resolver.admit_application(&environment).ok()?;
                    let mut signature = self.signature(*function)?;
                    for p in &mut signature.0 {
                        if p.parent != ParameterParent::Function(function.declaration) {
                            return None;
                        }
                        resolver
                            .admit_roots(
                                &[p.ty],
                                Oracle(self.0, Some(*function))
                                    .occurrence((function.package, p.header.owner)),
                                false,
                            )
                            .ok()?;
                        p.ty = resolver.substitute(p.ty, &environment, 0).ok()?;
                    }
                    signature.1 = resolver.substitute(signature.1, &environment, 0).ok()?;
                    resolver
                        .admit_roots(
                            &signature
                                .0
                                .iter()
                                .map(|p| p.ty)
                                .chain([signature.1])
                                .collect::<Vec<_>>(),
                            caller,
                            false,
                        )
                        .ok()?;
                    let mut overlay = self.0.clone();
                    overlay.types.extend(resolver.derived.clone());
                    if !Oracle(&overlay, self.1).legal_signature(&signature) {
                        return None;
                    }
                    let source = match f.result_borrow {
                        Some(source) => Some(f.parameters.iter().position(|id| *id == source)?),
                        None => None,
                    };
                    let parameters = signature
                        .0
                        .iter()
                        .map(|p| (p.ty, p.ty, p.use_mode))
                        .collect();
                    (
                        environment,
                        parameters,
                        signature.1,
                        source,
                        arguments.clone(),
                    )
                }
                ExpressionOperation::MethodCall {
                    witness,
                    contract,
                    method,
                    arguments,
                } => {
                    // The old reference admits complete mapped method inventories;
                    // the separate algebra retains exact supplied witness authority.
                    self.witness(witness.clone())?;
                    let (actual, self_type, actuals) =
                        resolver.admit_operand(witness, caller, 0).ok()?;
                    if actual != *contract {
                        return None;
                    }
                    let c = self.contract(*contract)?;
                    if c.type_parameters.len() != actuals.len() {
                        return None;
                    }
                    let m = c.methods.iter().find(|m| m.id == *method)?;
                    let environment = Environment {
                        ordinary: std::iter::once((c.self_parameter, self_type))
                            .chain(c.type_parameters.iter().copied().zip(actuals))
                            .collect(),
                        source: Context::Contract(*contract),
                        caller,
                        contract_operand: Some(witness.clone()),
                        witnesses: BTreeMap::new(),
                    };
                    resolver.admit_application(&environment).ok()?;
                    let parameters = m
                        .parameters
                        .iter()
                        .map(|p| {
                            let ty = resolver.substitute(p.ty, &environment, 0).ok()?;
                            resolver.admit_roots(&[ty], caller, false).ok()?;
                            Some((ty, ty, p.use_mode))
                        })
                        .collect::<Option<Vec<_>>>()?;
                    let result = resolver.substitute(m.result, &environment, 0).ok()?;
                    resolver.admit_roots(&[result], caller, false).ok()?;
                    (
                        environment,
                        parameters,
                        result,
                        m.result_borrow.map(|index| index as usize),
                        arguments.clone(),
                    )
                }
                _ => return None,
            };
        // Environment remains part of every resolver cache key until all roots
        // have been derived; only canonical objects cross into flow evaluation.
        let _ = environment;
        Some(Invocation {
            parameters,
            result,
            result_borrow,
            arguments,
            derived: resolver.derived,
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(super) struct Occurrence {
    pub parameter_scope: Option<DeclarationReference>,
    pub context: Context,
    pub consumer: (PackageId, OwnerKey),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum Failure {
    Missing,
    Authority,
    Generation,
    Inventory,
    Arity,
    BindingProjection,
    Category,
    Capacity,
}
type Result<T> = std::result::Result<T, Failure>;

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(super) struct Environment {
    pub ordinary: BTreeMap<TypeParameterId, TypeObjectDigest>,
    pub source: Context,
    pub caller: Occurrence,
    pub contract_operand: Option<ImplementationOperand>,
    pub witnesses: BTreeMap<ImplementationParameterId, ImplementationOperand>,
}

#[derive(Clone, Copy)]
pub(super) struct Limits {
    pub work: usize,
    pub objects: usize,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            work: contract::MAXIMUM_VALIDATION_WORK,
            objects: contract::MAXIMUM_CHILDREN,
        }
    }
}

pub(super) struct Resolver<'a> {
    source: &'a KernelSnapshot,
    pub derived: BTreeMap<TypeObjectDigest, TypeObject>,
    remaining: usize,
    maximum_objects: usize,
    resolutions: BTreeMap<(TypeObjectDigest, Environment), TypeObjectDigest>,
    active_operands: BTreeSet<(ImplementationOperand, Occurrence)>,
}

impl<'a> Resolver<'a> {
    pub fn new(source: &'a KernelSnapshot) -> Self {
        Self::with_limits(source, Limits::default())
    }
    pub fn with_limits(source: &'a KernelSnapshot, limits: Limits) -> Self {
        Self {
            source,
            derived: BTreeMap::new(),
            remaining: limits.work,
            maximum_objects: limits.objects,
            resolutions: BTreeMap::new(),
            active_operands: BTreeSet::new(),
        }
    }
    fn step(&mut self, depth: usize) -> Result<()> {
        if depth > contract::MAXIMUM_TYPE_DEPTH {
            return Err(Failure::Capacity);
        }
        self.remaining = self.remaining.checked_sub(1).ok_or(Failure::Capacity)?;
        Ok(())
    }
    pub fn remaining(&self) -> usize {
        self.remaining
    }
    fn object(&self, ty: TypeObjectDigest) -> Result<&TypeObject> {
        self.derived
            .get(&ty)
            .or_else(|| self.source.types.get(&ty))
            .or_else(|| self.source.dependency_types.get(&ty))
            .ok_or(Failure::Missing)
    }
    fn interface(
        &self,
        package: PackageId,
        key: OwnerKey,
        property: bool,
    ) -> Result<&PackageInterfaceRecord> {
        let selection = self
            .source
            .dependencies
            .get(&package)
            .or_else(|| {
                property
                    .then(|| self.source.representation_dependencies.get(&package))
                    .flatten()
            })
            .ok_or(Failure::Authority)?;
        if selection.package != package {
            return Err(Failure::Authority);
        }
        self.source
            .dependency_interfaces
            .get(&selection.package_revision)
            .and_then(|inventory| inventory.get(&key))
            .ok_or(Failure::Missing)
    }
    fn header(&self, package: PackageId, key: OwnerKey, property: bool) -> Result<OwnerHeader> {
        let header = if package == self.source.root.package_id {
            self.source
                .owners
                .get(&key)
                .ok_or(Failure::Missing)?
                .header()
        } else {
            self.interface(package, key, property)?.header()
        };
        if header.owner != key {
            return Err(Failure::Authority);
        }
        Ok(header)
    }
    fn generation(&self, occurrence: Occurrence) -> Result<u16> {
        let (package, key) = occurrence.consumer;
        let owner = self.header(package, key, false)?.contract_version;
        let selected = if package == self.source.root.package_id {
            self.source.root.graph_contract_version
        } else {
            self.source
                .dependencies
                .get(&package)
                .ok_or(Failure::Authority)?
                .graph_contract_version
        };
        Ok(owner
            .min(selected)
            .min(self.source.root.graph_contract_version))
    }
    fn current(&self, occurrence: Occurrence) -> Result<()> {
        if self.generation(occurrence)? == contract::GRAPH_CONTRACT_VERSION {
            Ok(())
        } else {
            Err(Failure::Generation)
        }
    }
    fn declaration_occurrence(&self, scope: DeclarationReference, context: Context) -> Occurrence {
        Occurrence {
            parameter_scope: Some(scope),
            context,
            consumer: (scope.package, OwnerKey::Declaration(scope.declaration)),
        }
    }
    fn contract(&self, scope: DeclarationReference) -> Result<&OwnedContract> {
        let key = OwnerKey::Declaration(scope.declaration);
        if self.header(scope.package, key, false)?.kind != OwnerKind::OwnedContract {
            return Err(Failure::Authority);
        }
        let result = if scope.package == self.source.root.package_id {
            match self.source.owners.get(&key) {
                Some(OwnerRecord::Declaration(r)) => match &r.payload {
                    DeclarationPayload::OwnedContract(c) => Some(c),
                    _ => None,
                },
                _ => None,
            }
        } else {
            match self.interface(scope.package, key, false)? {
                PackageInterfaceRecord::Declaration(r) => match &r.payload {
                    PackageInterfaceDeclarationPayload::OwnedContract(c) => Some(c),
                    _ => None,
                },
                _ => None,
            }
        };
        result.ok_or(Failure::Authority)
    }
    fn implementation(&self, scope: DeclarationReference) -> Result<&OwnedImplementation> {
        let key = OwnerKey::Declaration(scope.declaration);
        if self.header(scope.package, key, false)?.kind != OwnerKind::OwnedImplementation {
            return Err(Failure::Authority);
        }
        let result = if scope.package == self.source.root.package_id {
            match self.source.owners.get(&key) {
                Some(OwnerRecord::Declaration(r)) => match &r.payload {
                    DeclarationPayload::OwnedImplementation(i) => Some(i),
                    _ => None,
                },
                _ => None,
            }
        } else {
            match self.interface(scope.package, key, false)? {
                PackageInterfaceRecord::Declaration(r) => match &r.payload {
                    PackageInterfaceDeclarationPayload::OwnedImplementation(i) => Some(i),
                    _ => None,
                },
                _ => None,
            }
        };
        result.ok_or(Failure::Authority)
    }
    fn lexical_inventory(
        &self,
        scope: DeclarationReference,
    ) -> Result<Vec<ImplementationParameter>> {
        let key = OwnerKey::Declaration(scope.declaration);
        let header = self.header(scope.package, key, false)?;
        self.current(self.declaration_occurrence(scope, Context::Lexical(scope)))?;
        let oracle = Oracle(self.source, Some(scope));
        let entries = match header.kind {
            OwnerKind::PureFunction | OwnerKind::TaskFunction => {
                let function = oracle.function(scope).ok_or(Failure::Authority)?;
                if matches!(function.effect, FunctionEffect::Pure)
                    != (header.kind == OwnerKind::PureFunction)
                {
                    return Err(Failure::Authority);
                }
                function.implementation_parameters
            }
            OwnerKind::OwnedImplementation => self
                .implementation(scope)?
                .implementation_parameters
                .clone(),
            _ => return Err(Failure::Authority),
        };
        if entries.len() > contract::MAXIMUM_CHILDREN {
            return Err(Failure::Capacity);
        }
        let mut ids = BTreeSet::new();
        let mut names = BTreeSet::new();
        for entry in &entries {
            if !ids.insert(entry.id) || !names.insert(&entry.name) {
                return Err(Failure::Inventory);
            }
            let target = self.contract(entry.contract)?;
            if entry.type_arguments.len() != target.type_parameters.len() {
                return Err(Failure::Arity);
            }
            let TypeForm::TypeParameter { parameter } = self.object(entry.self_type)?.form else {
                return Err(Failure::Authority);
            };
            if !oracle
                .scoped_parameter(parameter)
                .is_some_and(|p| p.constraints.has_owned())
            {
                return Err(Failure::Authority);
            }
        }
        Ok(entries)
    }
    fn member(
        &mut self,
        scope: DeclarationReference,
        member: AssociatedTypeMemberId,
    ) -> Result<()> {
        self.step(0)?;
        self.current(self.declaration_occurrence(scope, Context::Contract(scope)))?;
        let c = self.contract(scope)?;
        if c.associated_types.len() > contract::MAXIMUM_CHILDREN {
            return Err(Failure::Capacity);
        }
        let mut ids = BTreeSet::new();
        let mut names = BTreeSet::new();
        for entry in &c.associated_types {
            if !ids.insert(entry.id) || !names.insert(&entry.name) {
                return Err(Failure::Inventory);
            }
            if entry.bound != AssociatedTypeBound::Owned {
                return Err(Failure::Category);
            }
        }
        if ids.contains(&member) {
            Ok(())
        } else {
            Err(Failure::Authority)
        }
    }
    pub fn admit_form(&mut self, form: &TypeForm, occurrence: Occurrence) -> Result<()> {
        self.step(0)?;
        match form {
            TypeForm::AssociatedContractMember { contract, member } => {
                self.current(occurrence)?;
                if occurrence.context != Context::Contract(*contract) {
                    return Err(Failure::Authority);
                }
                self.member(*contract, *member)
            }
            TypeForm::AssociatedWitnessProjection {
                scope,
                parameter,
                contract,
                member,
            } => {
                self.current(occurrence)?;
                if occurrence.context != Context::Lexical(*scope) {
                    return Err(Failure::Authority);
                }
                let parameters = self.lexical_inventory(*scope)?;
                let p = parameters
                    .iter()
                    .find(|p| p.id == *parameter)
                    .ok_or(Failure::Authority)?;
                if p.contract != *contract {
                    return Err(Failure::Authority);
                }
                self.member(*contract, *member)
            }
            _ => Ok(()),
        }
    }
    fn parameter_authority(
        &self,
        parameter: TypeParameterId,
        occurrence: Occurrence,
    ) -> Result<()> {
        let scope = occurrence.parameter_scope.ok_or(Failure::Authority)?;
        let oracle = Oracle(self.source, Some(scope));
        if oracle.scoped_parameter(parameter).is_some() {
            return Ok(());
        }
        // Raw ordinary nominal formals have representation authority, rather
        // than the function/contract/scheme/OwnedRecord scope recognized by the
        // existing ownership helper. This establishes identity only; it adds no
        // Owned, transport, capture or associated-member assumption.
        if occurrence.context != Context::Forbidden {
            return Err(Failure::Authority);
        }
        let key = OwnerKey::Declaration(scope.declaration);
        let header = self.header(scope.package, key, true)?;
        let formals = if scope.package == self.source.root.package_id {
            match self.source.owners.get(&key) {
                Some(OwnerRecord::Declaration(record)) => match &record.payload {
                    DeclarationPayload::Record {
                        type_parameters, ..
                    } if header.kind == OwnerKind::Record => type_parameters,
                    DeclarationPayload::Variant {
                        type_parameters, ..
                    } if header.kind == OwnerKind::Variant => type_parameters,
                    _ => return Err(Failure::Authority),
                },
                _ => return Err(Failure::Authority),
            }
        } else {
            match self.interface(scope.package, key, true)? {
                PackageInterfaceRecord::Declaration(record)
                | PackageInterfaceRecord::RepresentationDeclaration(record) => {
                    match &record.payload {
                        PackageInterfaceDeclarationPayload::Record {
                            type_parameters, ..
                        } if header.kind == OwnerKind::Record => type_parameters,
                        PackageInterfaceDeclarationPayload::Variant {
                            type_parameters, ..
                        } if header.kind == OwnerKind::Variant => type_parameters,
                        _ => return Err(Failure::Authority),
                    }
                }
                _ => return Err(Failure::Authority),
            }
        };
        let record = oracle
            .type_parameter(scope.package, parameter)
            .ok_or(Failure::Authority)?;
        if !formals.contains(&parameter)
            || record.header.owner != OwnerKey::TypeParameter(parameter)
            || record.header.kind != OwnerKind::TypeParameter
            || record.declaration != scope.declaration
        {
            return Err(Failure::Authority);
        }
        Ok(())
    }
    pub fn admit_roots(
        &mut self,
        roots: &[TypeObjectDigest],
        occurrence: Occurrence,
        projection_free: bool,
    ) -> Result<()> {
        let mut pending: Vec<_> = roots.iter().map(|t| (*t, occurrence, 0)).collect();
        let mut seen = BTreeMap::new();
        let mut expanded = BTreeSet::new();
        while let Some((ty, here, depth)) = pending.pop() {
            self.step(depth)?;
            if seen
                .get(&(ty, here))
                .is_some_and(|previous| *previous >= depth)
            {
                continue;
            }
            seen.insert((ty, here), depth);
            let object = self.object(ty)?.clone();
            if projection_free && object.form.is_associated() {
                return Err(Failure::BindingProjection);
            }
            self.admit_form(&object.form, here)?;
            if let TypeForm::TypeParameter { parameter } = object.form {
                self.parameter_authority(parameter, here)?;
            }
            pending.extend(
                object
                    .child_types()
                    .into_iter()
                    .map(|t| (t, here, depth + 1)),
            );
            if let TypeForm::Named { declaration } | TypeForm::Applied { declaration, .. } =
                object.form
                && expanded.insert(declaration)
            {
                let oracle = Oracle(self.source, Some(declaration));
                let fields = if let Some((_, fields)) = oracle.owned_record_inventory(declaration) {
                    fields
                        .into_iter()
                        .map(|(f, t)| (OwnerKey::Field(f.field), t))
                        .collect::<Vec<_>>()
                } else {
                    let (_, types) = oracle.nominal(declaration).ok_or(Failure::Authority)?;
                    // Ordinary nominal helper verifies owner/scope inventories.
                    types
                        .into_iter()
                        .map(|t| (OwnerKey::Declaration(declaration.declaration), t))
                        .collect()
                };
                for (key, ty) in fields {
                    let header = self.header(declaration.package, key, true)?;
                    let consumer = (declaration.package, key);
                    // Representation absence is checked even when no source name
                    // may access that record. No member authority can escape it.
                    if header.owner != key {
                        return Err(Failure::Authority);
                    }
                    pending.push((
                        ty,
                        Occurrence {
                            parameter_scope: Some(declaration),
                            context: Context::Forbidden,
                            consumer,
                        },
                        depth + 1,
                    ));
                }
            }
        }
        Ok(())
    }
    pub fn admit_bindings(&mut self, scope: DeclarationReference) -> Result<()> {
        self.step(0)?;
        let implementation = self.implementation(scope)?.clone();
        let contract = self.contract(implementation.contract)?.clone();
        if !contract.associated_types.is_empty() || !implementation.associated_types.is_empty() {
            self.current(self.declaration_occurrence(scope, Context::Lexical(scope)))?;
        }
        let expected: BTreeSet<_> = contract.associated_types.iter().map(|m| m.id).collect();
        let mut actual = BTreeSet::new();
        for binding in &implementation.associated_types {
            self.member(implementation.contract, binding.member)?;
            if !actual.insert(binding.member) {
                return Err(Failure::Inventory);
            }
            let occurrence = self.declaration_occurrence(scope, Context::Lexical(scope));
            self.admit_roots(&[binding.ty], occurrence, true)?;
            if !Oracle(self.source, Some(scope)).owned_type_in_scope(binding.ty) {
                return Err(Failure::Category);
            }
        }
        if actual != expected {
            return Err(Failure::Inventory);
        }
        Ok(())
    }
    fn overlay(&self) -> KernelSnapshot {
        let mut result = self.source.clone();
        result.types.extend(self.derived.clone());
        result
    }
    pub fn owned_type(
        &mut self,
        ty: TypeObjectDigest,
        occurrence: Occurrence,
        depth: usize,
    ) -> Result<bool> {
        self.step(depth)?;
        self.admit_roots(&[ty], occurrence, false)?;
        let form = self.object(ty)?.form.clone();
        match form {
            TypeForm::AssociatedContractMember { .. }
            | TypeForm::AssociatedWitnessProjection { .. }
            | TypeForm::ByteBuffer
            | TypeForm::OwnedI64Cell => Ok(true),
            TypeForm::TypeParameter { parameter } => {
                let scope = occurrence.parameter_scope.ok_or(Failure::Authority)?;
                let oracle = Oracle(self.source, Some(scope));
                let p = oracle
                    .scoped_parameter(parameter)
                    .ok_or(Failure::Authority)?;
                Ok(p.constraints.has_owned())
            }
            TypeForm::OwnedSequence { item } => {
                self.composite_child(item, occurrence, depth + 1)?;
                Ok(true)
            }
            TypeForm::OwnedProduct { fields } | TypeForm::OwnedChoice { cases: fields } => {
                if fields.is_empty()
                    || fields.len() > contract::MAXIMUM_CHILDREN
                    || fields.windows(2).any(|f| f[0].name >= f[1].name)
                {
                    return Err(Failure::Inventory);
                }
                let mut owned = false;
                for field in fields {
                    owned |= self.composite_child(field.ty, occurrence, depth + 1)?;
                }
                if owned {
                    Ok(true)
                } else {
                    Err(Failure::Category)
                }
            }
            TypeForm::Named { declaration } | TypeForm::Applied { declaration, .. } => {
                let oracle = Oracle(self.source, Some(declaration));
                let Some((parameters, fields)) = oracle.owned_record_inventory(declaration) else {
                    return Ok(false);
                };
                oracle
                    .record_layout_acyclic(
                        declaration,
                        &mut BTreeSet::new(),
                        depth,
                        &mut self.remaining,
                    )
                    .ok_or(Failure::Category)?;
                let arguments = match self.object(ty)?.form.clone() {
                    TypeForm::Applied { arguments, .. } => arguments,
                    _ => vec![],
                };
                if parameters.len() != arguments.len() {
                    return Err(Failure::Arity);
                }
                for (formal, actual) in parameters.iter().zip(&arguments) {
                    let p = oracle
                        .type_parameter(declaration.package, *formal)
                        .ok_or(Failure::Authority)?;
                    self.constraints(*actual, p.constraints, occurrence, depth + 1)?;
                }
                let bindings = parameters.into_iter().zip(arguments).collect();
                for (_, template) in fields {
                    let field = self.replace(template, &bindings, depth + 1)?;
                    self.composite_child(field, occurrence, depth + 1)?;
                }
                Ok(true)
            }
            _ => Ok(false),
        }
    }
    fn composite_child(
        &mut self,
        ty: TypeObjectDigest,
        occurrence: Occurrence,
        depth: usize,
    ) -> Result<bool> {
        if self.owned_type(ty, occurrence, depth)? {
            return Ok(true);
        }
        let overlay = self.overlay();
        if Oracle(&overlay, occurrence.parameter_scope).ordinary(ty) {
            Ok(false)
        } else {
            Err(Failure::Category)
        }
    }
    pub fn constraints(
        &mut self,
        ty: TypeObjectDigest,
        bounds: TypeParameterConstraints,
        occurrence: Occurrence,
        depth: usize,
    ) -> Result<()> {
        self.step(depth)?;
        self.admit_roots(&[ty], occurrence, false)?;
        if bounds.has_owned() && !self.owned_type(ty, occurrence, depth + 1)? {
            return Err(Failure::Category);
        }
        // The existing independent structural transport/read/capture proofs have
        // no associated-bound shortcut. Unknown Owned members keep those refusals.
        if bounds.requires_transfer()
            || bounds.requires_share()
            || bounds.requires_capture_safe()
            || !bounds.has_owned()
        {
            let overlay = self.overlay();
            if !Oracle(&overlay, occurrence.parameter_scope).satisfies_constraints(ty, bounds) {
                return Err(Failure::Category);
            }
        }
        Ok(())
    }
    pub fn admit_operand(
        &mut self,
        operand: &ImplementationOperand,
        caller: Occurrence,
        depth: usize,
    ) -> Result<(
        DeclarationReference,
        TypeObjectDigest,
        Vec<TypeObjectDigest>,
    )> {
        self.step(depth)?;
        let key = (operand.clone(), caller);
        if !self.active_operands.insert(key.clone()) {
            return Err(Failure::Authority);
        }
        let result = self.operand_inner(operand, caller, depth);
        self.active_operands.remove(&key);
        result
    }
    fn operand_inner(
        &mut self,
        operand: &ImplementationOperand,
        caller: Occurrence,
        depth: usize,
    ) -> Result<(
        DeclarationReference,
        TypeObjectDigest,
        Vec<TypeObjectDigest>,
    )> {
        self.current(caller)?;
        match operand {
            ImplementationOperand::Parameter { scope, parameter } => {
                if caller.context != Context::Lexical(*scope) {
                    return Err(Failure::Authority);
                }
                let entries = self.lexical_inventory(*scope)?;
                let entry = entries
                    .iter()
                    .find(|p| p.id == *parameter)
                    .ok_or(Failure::Authority)?;
                self.contract_actuals(
                    entry.contract,
                    entry.self_type,
                    &entry.type_arguments,
                    caller,
                    depth + 1,
                )?;
                Ok((
                    entry.contract,
                    entry.self_type,
                    entry.type_arguments.clone(),
                ))
            }
            ImplementationOperand::Concrete {
                implementation,
                type_arguments,
                implementations,
            } => {
                let i = self.implementation(*implementation)?.clone();
                if i.type_parameters.len() != type_arguments.len()
                    || i.implementation_parameters.len() != implementations.len()
                {
                    return Err(Failure::Arity);
                }
                self.admit_bindings(*implementation)?;
                let mut formals = BTreeSet::new();
                let mut names = BTreeSet::new();
                let oracle = Oracle(self.source, Some(*implementation));
                for (id, actual) in i.type_parameters.iter().zip(type_arguments) {
                    let p = oracle.scoped_parameter(*id).ok_or(Failure::Authority)?;
                    if !formals.insert(*id) || !names.insert(&p.name) {
                        return Err(Failure::Inventory);
                    }
                    self.constraints(*actual, p.constraints, caller, depth + 1)?;
                }
                let bindings = i
                    .type_parameters
                    .iter()
                    .copied()
                    .zip(type_arguments.iter().copied())
                    .collect();
                let witnesses = i
                    .implementation_parameters
                    .iter()
                    .map(|p| p.id)
                    .zip(implementations.iter().cloned())
                    .collect();
                let environment = Environment {
                    ordinary: bindings,
                    source: Context::Lexical(*implementation),
                    caller,
                    contract_operand: None,
                    witnesses,
                };
                let mut ids = BTreeSet::new();
                let mut names = BTreeSet::new();
                for (formal, actual) in i.implementation_parameters.iter().zip(implementations) {
                    if !ids.insert(formal.id) || !names.insert(&formal.name) {
                        return Err(Failure::Inventory);
                    }
                    let (contract, self_type, arguments) =
                        self.admit_operand(actual, caller, depth + 1)?;
                    if contract != formal.contract {
                        return Err(Failure::Authority);
                    }
                    let required_self =
                        self.substitute_inner(formal.self_type, &environment, depth + 1)?;
                    let required_arguments = formal
                        .type_arguments
                        .iter()
                        .map(|t| self.substitute_inner(*t, &environment, depth + 1))
                        .collect::<Result<Vec<_>>>()?;
                    if self_type != required_self || arguments != required_arguments {
                        return Err(Failure::Authority);
                    }
                }
                let self_type = self.substitute_inner(i.self_type, &environment, depth + 1)?;
                let arguments = i
                    .type_arguments
                    .iter()
                    .map(|t| self.substitute_inner(*t, &environment, depth + 1))
                    .collect::<Result<Vec<_>>>()?;
                self.contract_actuals(i.contract, self_type, &arguments, caller, depth + 1)?;
                Ok((i.contract, self_type, arguments))
            }
        }
    }
    fn contract_actuals(
        &mut self,
        scope: DeclarationReference,
        self_type: TypeObjectDigest,
        arguments: &[TypeObjectDigest],
        caller: Occurrence,
        depth: usize,
    ) -> Result<()> {
        self.step(depth)?;
        let contract = self.contract(scope)?.clone();
        if contract.type_parameters.len() != arguments.len() {
            return Err(Failure::Arity);
        }
        let oracle = Oracle(self.source, Some(scope));
        let self_parameter = oracle
            .scoped_parameter(contract.self_parameter)
            .ok_or(Failure::Authority)?;
        if self_parameter.constraints != TypeParameterConstraints::Owned {
            return Err(Failure::Category);
        }
        self.constraints(
            self_type,
            TypeParameterConstraints::Owned,
            caller,
            depth + 1,
        )?;
        let mut ids = BTreeSet::from([contract.self_parameter]);
        for (id, ty) in contract.type_parameters.iter().zip(arguments) {
            let p = oracle.scoped_parameter(*id).ok_or(Failure::Authority)?;
            if !ids.insert(*id) {
                return Err(Failure::Inventory);
            }
            self.constraints(*ty, p.constraints, caller, depth + 1)?;
        }
        Ok(())
    }
    fn intern(&mut self, form: TypeForm) -> Result<TypeObjectDigest> {
        self.step(0)?;
        let object = TypeObject::new(form).map_err(|_| Failure::Inventory)?;
        let digest = encode_type_object(&object)
            .map_err(|_| Failure::Inventory)?
            .0;
        if !self.source.types.contains_key(&digest)
            && !self.source.dependency_types.contains_key(&digest)
        {
            if !self.derived.contains_key(&digest) && self.derived.len() >= self.maximum_objects {
                return Err(Failure::Capacity);
            }
            self.derived.entry(digest).or_insert(object);
        }
        Ok(digest)
    }
    fn children(
        &mut self,
        form: TypeForm,
        mut apply: impl FnMut(&mut Self, TypeObjectDigest) -> Result<TypeObjectDigest>,
    ) -> Result<TypeForm> {
        let mut form = form;
        match &mut form {
            TypeForm::Applied { arguments, .. } => {
                for t in arguments {
                    *t = apply(self, *t)?;
                }
            }
            TypeForm::List { item }
            | TypeForm::Option { item }
            | TypeForm::Stream { item }
            | TypeForm::OwnedSequence { item } => *item = apply(self, *item)?,
            TypeForm::Map { key, value }
            | TypeForm::Result {
                ok: key,
                error: value,
            } => {
                *key = apply(self, *key)?;
                *value = apply(self, *value)?;
            }
            TypeForm::Function { parameters, result }
            | TypeForm::TaskFunction {
                parameters, result, ..
            } => {
                for t in parameters {
                    *t = apply(self, *t)?;
                }
                *result = apply(self, *result)?;
            }
            TypeForm::StructuralRecord { fields }
            | TypeForm::OwnedProduct { fields }
            | TypeForm::OwnedChoice { cases: fields } => {
                for f in fields {
                    f.ty = apply(self, f.ty)?;
                }
            }
            _ => {}
        }
        Ok(form)
    }
    fn replace(
        &mut self,
        ty: TypeObjectDigest,
        bindings: &BTreeMap<TypeParameterId, TypeObjectDigest>,
        depth: usize,
    ) -> Result<TypeObjectDigest> {
        self.step(depth)?;
        let object = self.object(ty)?.clone();
        if let TypeForm::TypeParameter { parameter } = object.form {
            return bindings.get(&parameter).copied().ok_or(Failure::Authority);
        }
        if object.form.is_associated() {
            return Err(Failure::BindingProjection);
        }
        let form = self.children(object.form, |r, t| r.replace(t, bindings, depth + 1))?;
        self.intern(form)
    }
    fn select(
        &mut self,
        contract: DeclarationReference,
        member: AssociatedTypeMemberId,
        operand: &ImplementationOperand,
        caller: Occurrence,
        depth: usize,
    ) -> Result<TypeObjectDigest> {
        self.step(depth)?;
        self.current(caller)?;
        self.member(contract, member)?;
        if self.admit_operand(operand, caller, depth + 1)?.0 != contract {
            return Err(Failure::Authority);
        }
        match operand {
            ImplementationOperand::Parameter { scope, parameter } => {
                if caller.context != Context::Lexical(*scope) {
                    return Err(Failure::Authority);
                }
                let entries = self.lexical_inventory(*scope)?;
                let p = entries
                    .iter()
                    .find(|p| p.id == *parameter)
                    .ok_or(Failure::Authority)?;
                if p.contract != contract {
                    return Err(Failure::Authority);
                }
                self.intern(TypeForm::AssociatedWitnessProjection {
                    scope: *scope,
                    parameter: *parameter,
                    contract,
                    member,
                })
            }
            ImplementationOperand::Concrete {
                implementation,
                type_arguments,
                implementations,
            } => {
                let i = self.implementation(*implementation)?.clone();
                if i.contract != contract {
                    return Err(Failure::Authority);
                }
                if i.type_parameters.len() != type_arguments.len()
                    || i.implementation_parameters.len() != implementations.len()
                {
                    return Err(Failure::Arity);
                }
                self.admit_bindings(*implementation)?;
                // Caller actuals are admitted before any template replacement,
                // including arguments absent from the selected member's RHS.
                self.admit_roots(type_arguments, caller, false)?;
                let bindings = i
                    .type_parameters
                    .iter()
                    .copied()
                    .zip(type_arguments.iter().copied())
                    .collect();
                let binding = i
                    .associated_types
                    .iter()
                    .find(|b| b.member == member)
                    .ok_or(Failure::Inventory)?;
                self.replace(binding.ty, &bindings, depth + 1)
            }
        }
    }
    pub fn substitute(
        &mut self,
        ty: TypeObjectDigest,
        environment: &Environment,
        depth: usize,
    ) -> Result<TypeObjectDigest> {
        self.admit_environment(environment, depth, false)?;
        self.substitute_inner(ty, environment, depth)
    }
    pub fn admit_application(&mut self, environment: &Environment) -> Result<()> {
        self.admit_environment(environment, 0, true)
    }

    // A scheme definition supplies its stored templates directly. This does not
    // invent a concrete witness application or equate it with a lexical anchor.
    pub fn definition_type(
        &mut self,
        ty: TypeObjectDigest,
        scope: DeclarationReference,
        depth: usize,
    ) -> Result<TypeObjectDigest> {
        self.step(depth)?;
        self.admit_bindings(scope)?;
        let implementation = self.implementation(scope)?.clone();
        let contract = self.contract(implementation.contract)?.clone();
        self.admit_roots(
            &[ty],
            self.declaration_occurrence(
                implementation.contract,
                Context::Contract(implementation.contract),
            ),
            false,
        )?;
        let ordinary = std::iter::once((contract.self_parameter, implementation.self_type))
            .chain(
                contract
                    .type_parameters
                    .iter()
                    .copied()
                    .zip(implementation.type_arguments),
            )
            .collect::<BTreeMap<_, _>>();
        self.definition_inner(ty, scope, &ordinary, depth + 1)
    }
    fn definition_inner(
        &mut self,
        ty: TypeObjectDigest,
        scope: DeclarationReference,
        ordinary: &BTreeMap<TypeParameterId, TypeObjectDigest>,
        depth: usize,
    ) -> Result<TypeObjectDigest> {
        self.step(depth)?;
        match self.object(ty)?.form.clone() {
            TypeForm::TypeParameter { parameter } => {
                ordinary.get(&parameter).copied().ok_or(Failure::Authority)
            }
            TypeForm::AssociatedContractMember { contract, member } => {
                let implementation = self.implementation(scope)?;
                if implementation.contract != contract {
                    return Err(Failure::Authority);
                }
                implementation
                    .associated_types
                    .iter()
                    .find(|binding| binding.member == member)
                    .map(|binding| binding.ty)
                    .ok_or(Failure::Inventory)
            }
            TypeForm::AssociatedWitnessProjection { .. } => Err(Failure::Authority),
            form => {
                let form = self.children(form, |resolver, child| {
                    resolver.definition_inner(child, scope, ordinary, depth + 1)
                })?;
                self.intern(form)
            }
        }
    }
    fn admit_environment(
        &mut self,
        environment: &Environment,
        depth: usize,
        complete: bool,
    ) -> Result<()> {
        self.step(depth)?;
        let scope = match environment.source {
            Context::Contract(scope) | Context::Lexical(scope) => scope,
            Context::Forbidden => {
                if environment.ordinary.is_empty()
                    && environment.witnesses.is_empty()
                    && environment.contract_operand.is_none()
                {
                    return Ok(());
                }
                return Err(Failure::Authority);
            }
        };
        let oracle = Oracle(self.source, Some(scope));
        let (parameters, witnesses) = match environment.source {
            Context::Contract(_) => {
                let c = self.contract(scope)?;
                (
                    std::iter::once(c.self_parameter)
                        .chain(c.type_parameters.iter().copied())
                        .collect::<Vec<_>>(),
                    vec![],
                )
            }
            Context::Lexical(_) => {
                if let Some(f) = oracle.function(scope) {
                    (f.type_parameters, f.implementation_parameters)
                } else {
                    let i = self.implementation(scope)?;
                    (
                        i.type_parameters.clone(),
                        i.implementation_parameters.clone(),
                    )
                }
            }
            Context::Forbidden => unreachable!(),
        };
        let ordinary_ids = parameters.iter().copied().collect::<BTreeSet<_>>();
        let provided_ordinary = environment
            .ordinary
            .keys()
            .copied()
            .collect::<BTreeSet<_>>();
        let witness_ids = witnesses.iter().map(|p| p.id).collect::<BTreeSet<_>>();
        let provided_witnesses = environment
            .witnesses
            .keys()
            .copied()
            .collect::<BTreeSet<_>>();
        if !provided_ordinary.is_subset(&ordinary_ids)
            || !provided_witnesses.is_subset(&witness_ids)
            || complete && (ordinary_ids != provided_ordinary || witness_ids != provided_witnesses)
        {
            return Err(Failure::Arity);
        }
        for p in provided_ordinary {
            let formal = oracle.scoped_parameter(p).ok_or(Failure::Authority)?;
            self.constraints(
                environment.ordinary[&p],
                formal.constraints,
                environment.caller,
                depth + 1,
            )?;
        }
        for formal in witnesses {
            let Some(actual) = environment.witnesses.get(&formal.id) else {
                continue;
            };
            let (contract, self_type, arguments) =
                self.admit_operand(actual, environment.caller, depth + 1)?;
            if contract != formal.contract
                || complete
                    && self_type
                        != self.substitute_inner(formal.self_type, environment, depth + 1)?
            {
                return Err(Failure::Authority);
            }
            if complete {
                let expected = formal
                    .type_arguments
                    .iter()
                    .map(|t| self.substitute_inner(*t, environment, depth + 1))
                    .collect::<Result<Vec<_>>>()?;
                if arguments != expected {
                    return Err(Failure::Authority);
                }
            }
        }
        if let Context::Contract(_) = environment.source {
            let operand = environment
                .contract_operand
                .as_ref()
                .ok_or(Failure::Authority)?;
            let (actual, self_type, arguments) =
                self.admit_operand(operand, environment.caller, depth + 1)?;
            let contract = self.contract(scope)?;
            if actual != scope
                || complete
                    && (environment.ordinary[&contract.self_parameter] != self_type
                        || contract
                            .type_parameters
                            .iter()
                            .map(|p| environment.ordinary[p])
                            .collect::<Vec<_>>()
                            != arguments)
            {
                return Err(Failure::Authority);
            }
        } else if environment.contract_operand.is_some() {
            return Err(Failure::Authority);
        }
        Ok(())
    }
    fn substitute_inner(
        &mut self,
        ty: TypeObjectDigest,
        environment: &Environment,
        depth: usize,
    ) -> Result<TypeObjectDigest> {
        self.step(depth)?;
        // Complete source/caller occurrence admission precedes a cached result.
        let source_scope = match environment.source {
            Context::Contract(s) | Context::Lexical(s) => Some(s),
            Context::Forbidden => None,
        };
        let source_occurrence = match source_scope {
            Some(s) => self.declaration_occurrence(s, environment.source),
            None => environment.caller,
        };
        self.admit_roots(&[ty], source_occurrence, false)?;
        for actual in environment.ordinary.values() {
            self.admit_roots(&[*actual], environment.caller, false)?;
        }
        if let Some(result) = self.resolutions.get(&(ty, environment.clone())) {
            return Ok(*result);
        }
        let object = self.object(ty)?.clone();
        let result = match object.form {
            TypeForm::TypeParameter { parameter } => environment
                .ordinary
                .get(&parameter)
                .copied()
                .ok_or(Failure::Authority)?,
            TypeForm::AssociatedContractMember { contract, member } => {
                let operand = environment
                    .contract_operand
                    .as_ref()
                    .ok_or(Failure::Authority)?;
                self.select(contract, member, operand, environment.caller, depth + 1)?
            }
            TypeForm::AssociatedWitnessProjection {
                parameter,
                contract,
                member,
                ..
            } => {
                let operand = environment
                    .witnesses
                    .get(&parameter)
                    .ok_or(Failure::Authority)?;
                self.select(contract, member, operand, environment.caller, depth + 1)?
            }
            form => {
                let form =
                    self.children(form, |r, t| r.substitute_inner(t, environment, depth + 1))?;
                self.intern(form)?
            }
        };
        if self.resolutions.len() >= self.maximum_objects {
            return Err(Failure::Capacity);
        }
        self.resolutions.insert((ty, environment.clone()), result);
        Ok(result)
    }
}
