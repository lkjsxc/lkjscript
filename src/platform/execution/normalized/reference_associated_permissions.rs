//! Complete canonical permission obligations. A scoped member supplies only its
//! Owned bound; property derivation never turns that bound into movement or sharing.
use super::*;
use crate::platform::admission::{Meter as AdmissionMeter, Observation};
use crate::platform::execution::ExecutionControl;
use crate::platform::kernel::{FunctionEffect, OwnerKind, ParameterParent, ParameterUse};
use associated_preparation::{Proof, ReadContext, SymbolicSelection, SymbolicSelectionSource};

pub(super) type NominalConstraints = BTreeSet<(
    DeclarationReference,
    Vec<TypeObjectDigest>,
    Option<DeclarationReference>,
)>;

pub(super) struct Meter<'a> {
    pub visits: &'a mut AdmissionMeter,
    pub bytes: &'a mut AdmissionMeter,
    pub control: &'a ExecutionControl,
}

impl Meter<'_> {
    pub(super) fn tick(&mut self) -> Result<(), ExecutionError> {
        self.control.check()?;
        self.visits.charge(Observation::exact(1)).map_err(|_| {
            ExecutionError::resource(
                "reference_associated_permission_work",
                "canonical permission obligations exceeded the selected validation allowance",
            )
        })?;
        Ok(())
    }
    fn reserve<T>(&mut self, count: usize) -> Result<(), ExecutionError> {
        self.tick()?;
        allocate::<T>(self.bytes, count)
    }
    fn node<T>(&mut self) -> Result<(), ExecutionError> {
        self.tick()?;
        index_node::<T>(self.bytes)
    }
}

#[derive(Clone)]
enum Origin {
    Read(ReadContext),
    Stored(DeclarationReference),
    Mapping(DeclarationReference, crate::platform::semantic_id::MethodId),
    ContractMethod(DeclarationReference, crate::platform::semantic_id::MethodId),
    Selection(usize),
}

fn capacity() -> ExecutionError {
    ExecutionError::resource(
        "reference_associated_permission_storage",
        "canonical permission inventory size overflowed",
    )
}

struct Constraint {
    origin: usize,
    owner: OwnerKey,
    target: DeclarationReference,
    scope: Option<DeclarationReference>,
    actuals: Vec<TypeObjectDigest>,
}

struct Parallel {
    origin: usize,
    parent: ExpressionId,
    left: ExpressionId,
    right: ExpressionId,
}

pub(super) struct Queue {
    origins: Vec<Origin>,
    constraints: Vec<Constraint>,
    parallels: Vec<Parallel>,
    parents: BTreeMap<(PackageId, ExpressionId), Parent>,
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum Parent {
    Root(OwnerKey),
    Expression(ExpressionId),
}

struct Collector<'a, 's, 'm, 'c> {
    proof: &'a Proof,
    sources: &'a BTreeMap<PackageId, &'s KernelSnapshot>,
    types: &'a BTreeMap<TypeObjectDigest, TypeObject>,
    meter: &'m mut Meter<'c>,
    queue: Queue,
}

fn scope(
    context: &ReadContext,
    function: &associated_preparation::Function,
    sources: &BTreeMap<PackageId, &KernelSnapshot>,
    meter: &mut Meter<'_>,
) -> Result<Option<DeclarationReference>, ExecutionError> {
    meter.tick()?;
    match context {
        ReadContext::Symbolic(reference) => Ok(Some(*reference)),
        ReadContext::Executable((package, key @ OwnerKey::Declaration(declaration))) => {
            let reference = DeclarationReference {
                package: *package,
                declaration: *declaration,
            };
            let OwnerRecord::Declaration(owner) = indexed_owner(sources, reference, *key)? else {
                return Err(failure());
            };
            // Nominal definition roots deliberately retain their own raw formal
            // inventory. Other executable contexts carry no lexical formal authority.
            let kind = match owner.payload {
                DeclarationPayload::Record { .. } => OwnerKind::Record,
                DeclarationPayload::OwnedRecord { .. } => OwnerKind::OwnedRecord,
                DeclarationPayload::Variant { .. } => OwnerKind::Variant,
                DeclarationPayload::ReadView(_) => OwnerKind::ReadView,
                _ => return Ok(None),
            };
            if owner.header.kind != kind
                || function.occurrences.get(key) != Some(&owner.header.contract_version)
            {
                return Err(failure());
            }
            Ok(Some(reference))
        }
        ReadContext::Application(_) | ReadContext::Executable(_) => Ok(None),
    }
}

fn package(origin: &Origin, proof: &Proof) -> Result<PackageId, ExecutionError> {
    match origin {
        Origin::Read(ReadContext::Application(application)) => Ok(application.function.package),
        Origin::Read(ReadContext::Symbolic(reference))
        | Origin::Stored(reference)
        | Origin::Mapping(reference, _)
        | Origin::ContractMethod(reference, _) => Ok(reference.package),
        Origin::Read(ReadContext::Executable((package, _))) => Ok(*package),
        Origin::Selection(identity) => proof
            .selections
            .get(*identity)
            .filter(|selection| selection.identity == *identity)
            .map(|selection| selection.implementation.package)
            .ok_or_else(failure),
    }
}

impl Collector<'_, '_, '_, '_> {
    fn parent(
        &mut self,
        package: PackageId,
        expression: ExpressionId,
        parent: Parent,
    ) -> Result<(), ExecutionError> {
        self.meter.node::<((PackageId, ExpressionId), Parent)>()?;
        if self
            .queue
            .parents
            .insert((package, expression), parent)
            .is_some()
        {
            return Err(failure());
        }
        Ok(())
    }

    fn parents(
        &mut self,
        package: PackageId,
        key: OwnerKey,
        owner: &OwnerRecord,
    ) -> Result<(), ExecutionError> {
        // Borrow the canonical constructor fields directly. The general children/root
        // APIs allocate vectors (and some hidden intermediate vectors); no sizing pass
        // through serialization may allocate IDs before this reservation.
        match owner {
            OwnerRecord::Declaration(declaration) => match declaration.payload {
                DeclarationPayload::Function(ref function) => {
                    self.parent(package, function.body, Parent::Root(key))?
                }
                DeclarationPayload::Constant { value, .. } => {
                    self.parent(package, value, Parent::Root(key))?
                }
                DeclarationPayload::Test {
                    actual, expected, ..
                } => {
                    self.parent(package, actual, Parent::Root(key))?;
                    self.parent(package, expected, Parent::Root(key))?;
                }
                _ => {}
            },
            OwnerRecord::Binding(binding) => {
                if let Some(value) = binding.value {
                    self.parent(package, value, Parent::Root(key))?;
                }
            }
            OwnerRecord::Port(port) => {
                if let crate::platform::kernel::PortImplementation::Expression(value) =
                    port.implementation
                {
                    self.parent(package, value, Parent::Root(key))?;
                }
            }
            _ => {}
        }
        let OwnerRecord::Expression(expression) = owner else {
            return Ok(());
        };
        let parent = Parent::Expression(expression.id);
        match &expression.operation {
            ExpressionOperation::Region { operation } => {
                for child in operation.expressions().into_iter().flatten() {
                    self.parent(package, *child, parent)?;
                }
            }
            ExpressionOperation::If {
                condition,
                when_true,
                when_false,
            } => {
                for child in [condition, when_true, when_false] {
                    self.parent(package, *child, parent)?;
                }
            }
            ExpressionOperation::Let { body, .. }
            | ExpressionOperation::BorrowReadView { body, .. }
            | ExpressionOperation::BorrowReadViewField { body, .. }
            | ExpressionOperation::Transaction { body, .. }
            | ExpressionOperation::TransactionOutcome { body, .. } => {
                self.parent(package, *body, parent)?
            }
            ExpressionOperation::Sequence { items } | ExpressionOperation::List { items, .. } => {
                for child in items {
                    self.parent(package, *child, parent)?;
                }
            }
            ExpressionOperation::Call { arguments, .. }
            | ExpressionOperation::ImplementationCall { arguments, .. }
            | ExpressionOperation::MethodCall { arguments, .. }
            | ExpressionOperation::CapabilityCall { arguments, .. } => {
                for child in arguments {
                    self.parent(package, *child, parent)?;
                }
            }
            ExpressionOperation::Invoke { callee, arguments }
            | ExpressionOperation::Bind { callee, arguments } => {
                self.parent(package, *callee, parent)?;
                for child in arguments {
                    self.parent(package, *child, parent)?;
                }
            }
            ExpressionOperation::Record { fields, .. } => {
                for field in fields {
                    self.parent(package, field.value, parent)?;
                }
            }
            ExpressionOperation::CreateReadView { fields, .. } => {
                for field in fields {
                    self.parent(package, field.value, parent)?;
                }
            }
            ExpressionOperation::Variant { payload, .. } => {
                if let Some(payload) = payload {
                    self.parent(package, *payload, parent)?;
                }
            }
            ExpressionOperation::Field { value, .. }
            | ExpressionOperation::ChooseOwned { value, .. } => {
                self.parent(package, *value, parent)?
            }
            ExpressionOperation::Map { entries, .. } => {
                for entry in entries {
                    self.parent(package, entry.key, parent)?;
                    self.parent(package, entry.value, parent)?;
                }
            }
            ExpressionOperation::Match { value, arms } => {
                self.parent(package, *value, parent)?;
                for arm in arms {
                    self.parent(package, arm.body, parent)?;
                }
            }
            ExpressionOperation::PackOwned { fields, .. } => {
                for field in fields {
                    self.parent(package, field.value, parent)?;
                }
            }
            ExpressionOperation::PackOwnedRecord { fields, .. } => {
                for field in fields {
                    self.parent(package, field.value, parent)?;
                }
            }
            ExpressionOperation::UnpackOwned { source, body, .. }
            | ExpressionOperation::UnpackOwnedRecord { source, body, .. }
            | ExpressionOperation::BorrowOwnedField { source, body, .. }
            | ExpressionOperation::BorrowOwnedRecordField { source, body, .. } => {
                self.parent(package, *source, parent)?;
                self.parent(package, *body, parent)?;
            }
            ExpressionOperation::MatchOwned { source, arms, .. }
            | ExpressionOperation::MatchBorrowedOwned { source, arms, .. } => {
                self.parent(package, *source, parent)?;
                for arm in arms {
                    self.parent(package, arm.body, parent)?;
                }
            }
            ExpressionOperation::Parallel { left, right } => {
                self.parent(package, *left, parent)?;
                self.parent(package, *right, parent)?;
            }
            ExpressionOperation::SequenceLength { source, .. }
            | ExpressionOperation::SequencePop { source, .. } => {
                self.parent(package, *source, parent)?
            }
            ExpressionOperation::SequencePush { source, value, .. } => {
                self.parent(package, *value, parent)?;
                self.parent(package, *source, parent)?;
            }
            ExpressionOperation::SequenceGet { source, index, .. } => {
                self.parent(package, *index, parent)?;
                self.parent(package, *source, parent)?;
            }
            ExpressionOperation::SequenceReplace {
                source,
                index,
                value,
                ..
            } => {
                for child in [index, value, source] {
                    self.parent(package, *child, parent)?;
                }
            }
            ExpressionOperation::BorrowOwnedItem {
                source,
                index,
                body,
                ..
            } => {
                for child in [index, source, body] {
                    self.parent(package, *child, parent)?;
                }
            }
            ExpressionOperation::BorrowCall { call, body, .. } => {
                self.parent(package, *call, parent)?;
                self.parent(package, *body, parent)?;
            }
            ExpressionOperation::Unit {}
            | ExpressionOperation::Bool { .. }
            | ExpressionOperation::I64 { .. }
            | ExpressionOperation::F64 { .. }
            | ExpressionOperation::Text { .. }
            | ExpressionOperation::StaticText { .. }
            | ExpressionOperation::Local { .. }
            | ExpressionOperation::Constant { .. }
            | ExpressionOperation::FunctionValue { .. }
            | ExpressionOperation::SequenceEmpty { .. } => {}
            ExpressionOperation::ReborrowReadView { .. }
            | ExpressionOperation::MoveReadView { .. }
            | ExpressionOperation::DropReadView { .. }
            | ExpressionOperation::ReadViewDataField { .. } => {}
        }
        Ok(())
    }
    fn origin(&mut self, origin: Origin) -> Result<usize, ExecutionError> {
        self.meter.reserve::<Origin>(1)?;
        let id = self.queue.origins.len();
        self.queue.origins.push(origin);
        Ok(id)
    }

    fn constraint(
        &mut self,
        origin: usize,
        owner: OwnerKey,
        target: DeclarationReference,
        actuals: &[TypeObjectDigest],
        scope: Option<DeclarationReference>,
    ) -> Result<(), ExecutionError> {
        self.meter.reserve::<Constraint>(1)?;
        self.meter.reserve::<TypeObjectDigest>(actuals.len())?;
        self.queue.constraints.push(Constraint {
            origin,
            owner,
            target,
            scope,
            actuals: actuals.to_vec(),
        });
        for actual in actuals {
            self.types(origin, owner, *actual, scope)?;
        }
        Ok(())
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "contract obligations retain exact Self, arguments, source occurrence and scope"
    )]
    fn contract(
        &mut self,
        origin: usize,
        owner: OwnerKey,
        contract: DeclarationReference,
        self_type: TypeObjectDigest,
        actuals: &[TypeObjectDigest],
        scope: Option<DeclarationReference>,
    ) -> Result<(), ExecutionError> {
        let count = actuals.len().checked_add(1).ok_or_else(capacity)?;
        self.meter.reserve::<TypeObjectDigest>(count)?;
        let mut arguments = Vec::new();
        arguments.try_reserve_exact(count).map_err(|_| capacity())?;
        arguments.push(self_type);
        arguments.extend_from_slice(actuals);
        self.constraint(origin, owner, contract, &arguments, scope)
    }

    fn types(
        &mut self,
        origin: usize,
        owner: OwnerKey,
        root: TypeObjectDigest,
        scope: Option<DeclarationReference>,
    ) -> Result<(), ExecutionError> {
        self.meter.reserve::<(TypeObjectDigest, usize)>(1)?;
        let mut pending = vec![(root, 0usize)];
        let mut seen = BTreeMap::new();
        while let Some((ty, depth)) = pending.pop() {
            self.meter.tick()?;
            if depth > crate::platform::kernel::contract::MAXIMUM_TYPE_DEPTH {
                return Err(failure());
            }
            if seen.get(&ty).is_some_and(|previous| *previous >= depth) {
                continue;
            }
            if !seen.contains_key(&ty) {
                self.meter.node::<(TypeObjectDigest, usize)>()?;
            }
            seen.insert(ty, depth);
            let object = self.types.get(&ty).ok_or_else(failure)?;
            if let TypeForm::Applied {
                declaration,
                arguments,
            }
            | TypeForm::ReadViewApplied {
                declaration,
                arguments,
                ..
            } = &object.form
            {
                self.meter.reserve::<Constraint>(1)?;
                self.meter.reserve::<TypeObjectDigest>(arguments.len())?;
                self.queue.constraints.push(Constraint {
                    origin,
                    owner,
                    target: *declaration,
                    scope,
                    actuals: arguments.clone(),
                });
            }
            self.meter
                .reserve::<TypeObjectDigest>(object.child_type_count())?;
            let children = object.child_types();
            self.meter
                .reserve::<(TypeObjectDigest, usize)>(children.len())?;
            pending.extend(children.into_iter().map(|child| (child, depth + 1)));
        }
        Ok(())
    }

    fn symbolic_selection(
        &mut self,
        origin: usize,
        owner: OwnerKey,
        selected: &SymbolicSelection,
        scope: Option<DeclarationReference>,
        depth: usize,
    ) -> Result<(), ExecutionError> {
        self.meter.tick()?;
        if depth > crate::platform::kernel::contract::MAXIMUM_TYPE_DEPTH {
            return Err(failure());
        }
        self.contract(
            origin,
            owner,
            selected.contract,
            selected.self_type,
            &selected.arguments,
            scope,
        )?;
        if let SymbolicSelectionSource::Concrete {
            implementation,
            actuals,
            prerequisites,
            members,
        } = &selected.source
        {
            self.constraint(origin, owner, *implementation, actuals, scope)?;
            for member in members {
                self.types(origin, owner, member.ty, scope)?;
            }
            for child in prerequisites {
                self.symbolic_selection(origin, owner, child, scope, depth + 1)?;
            }
        }
        Ok(())
    }

    fn target(
        &mut self,
        origin: usize,
        owner: OwnerKey,
        target: &associated_preparation::SymbolicTarget,
        scope: Option<DeclarationReference>,
    ) -> Result<(), ExecutionError> {
        if let Some(function) = target.function {
            self.constraint(origin, owner, function, &target.types, scope)?;
        }
        for selected in &target.witnesses {
            self.symbolic_selection(origin, owner, selected, scope, 0)?;
        }
        self.types(origin, owner, target.signature.result.ty, scope)?;
        for (ty, _) in &target.signature.parameters {
            self.types(origin, owner, *ty, scope)?;
        }
        Ok(())
    }

    fn read(&mut self, context: ReadContext) -> Result<(), ExecutionError> {
        let proof = self.proof;
        let function = proof.read_function(&context, self.meter.control)?;
        let source_scope = scope(&context, function, self.sources, self.meter)?;
        let origin = self.origin(Origin::Read(context))?;
        let package = package(&self.queue.origins[origin], self.proof)?;
        if let Origin::Read(ReadContext::Application(application)) = &self.queue.origins[origin] {
            let target = application.function;
            self.meter
                .reserve::<TypeObjectDigest>(application.types.len())?;
            let actuals = application.types.clone();
            self.constraint(
                origin,
                OwnerKey::Declaration(target.declaration),
                target,
                &actuals,
                None,
            )?;
        }
        for ((owner, _), ty) in &function.roots {
            self.types(origin, *owner, *ty, source_scope)?;
        }
        for (expression, target) in &function.targets {
            self.constraint(
                origin,
                OwnerKey::Expression(*expression),
                target.function,
                &target.types,
                source_scope,
            )?;
        }
        for (expression, target) in &function.symbolic_targets {
            self.target(
                origin,
                OwnerKey::Expression(*expression),
                target,
                source_scope,
            )?;
        }
        for (owner, generation) in &function.occurrences {
            self.meter.tick()?;
            let source = self.sources.get(&package).ok_or_else(failure)?;
            let record = source.owners.get(owner).ok_or_else(failure)?;
            if record.header().owner != *owner || record.header().contract_version != *generation {
                return Err(failure());
            }
            if let OwnerRecord::Expression(expression) = record
                && let ExpressionOperation::Parallel { left, right } = expression.operation
            {
                self.meter.reserve::<Parallel>(1)?;
                self.queue.parallels.push(Parallel {
                    origin,
                    parent: expression.id,
                    left,
                    right,
                });
            }
        }
        Ok(())
    }
}

pub(super) fn collect(
    proof: &Proof,
    sources: &BTreeMap<PackageId, &KernelSnapshot>,
    types: &BTreeMap<TypeObjectDigest, TypeObject>,
    meter: &mut Meter<'_>,
) -> Result<Queue, ExecutionError> {
    meter.tick()?;
    let mut collector = Collector {
        proof,
        sources,
        types,
        meter,
        queue: Queue {
            origins: Vec::new(),
            constraints: Vec::new(),
            parallels: Vec::new(),
            parents: BTreeMap::new(),
        },
    };
    for (package, source) in sources {
        for (key, owner) in &source.owners {
            collector.meter.tick()?;
            collector.parents(*package, *key, owner)?;
        }
    }
    for application in proof.applications.keys() {
        // Reserve the full application before cloning its immutable queue context.
        collector
            .meter
            .reserve::<TypeObjectDigest>(application.types.len())?;
        collector
            .meter
            .reserve::<usize>(application.witnesses.len())?;
        collector
            .meter
            .reserve::<crate::platform::kernel::EffectRow>(application.effects.len())?;
        for row in &application.effects {
            collector
                .meter
                .reserve::<crate::platform::kernel::EffectParameterReference>(
                    row.parameters.len(),
                )?;
            collector
                .meter
                .reserve::<crate::platform::kernel::RequirementOperand>(row.requirements.len())?;
        }
        collector
            .meter
            .reserve::<crate::platform::kernel::RequirementReference>(
                application.requirements.len(),
            )?;
        collector
            .meter
            .reserve::<crate::platform::kernel::ReadScopeArgument>(
                application.read_scope_arguments.len(),
            )?;
        collector.read(ReadContext::Application(application.clone()))?;
    }
    for reference in proof.symbolic.keys() {
        collector.read(ReadContext::Symbolic(*reference))?;
    }
    for executable in proof.executables.keys() {
        collector.read(ReadContext::Executable(*executable))?;
    }
    for selected in &proof.selections {
        let origin = collector.origin(Origin::Selection(selected.identity))?;
        let owner = OwnerKey::Declaration(selected.implementation.declaration);
        collector.constraint(
            origin,
            owner,
            selected.implementation,
            &selected.actuals,
            None,
        )?;
        collector.contract(
            origin,
            owner,
            selected.contract,
            selected.self_type,
            &selected.contract_actuals,
            None,
        )?;
        for member in &selected.members {
            collector.types(origin, owner, member.ty, None)?;
        }
    }
    // Raw owner inventories cover unused bindings, private fields, inactive cases
    // and every phantom prerequisite even when no concrete entry selects a scheme.
    for (package, source) in sources {
        for (key, owner) in &source.owners {
            collector.meter.tick()?;
            let (OwnerKey::Declaration(id), OwnerRecord::Declaration(declaration)) = (key, owner)
            else {
                continue;
            };
            let reference = DeclarationReference {
                package: *package,
                declaration: *id,
            };
            let origin = collector.origin(Origin::Stored(reference))?;
            let scope = Some(reference);
            match &declaration.payload {
                DeclarationPayload::OwnedImplementation(implementation) => {
                    collector.contract(
                        origin,
                        *key,
                        implementation.contract,
                        implementation.self_type,
                        &implementation.type_arguments,
                        scope,
                    )?;
                    for member in &implementation.associated_types {
                        collector.types(origin, *key, member.ty(), scope)?;
                    }
                    for prerequisite in &implementation.implementation_parameters {
                        collector.contract(
                            origin,
                            *key,
                            prerequisite.contract,
                            prerequisite.self_type,
                            &prerequisite.type_arguments,
                            scope,
                        )?;
                    }
                    for mapping in &implementation.methods {
                        let target = proof.symbolic_mapping(
                            reference,
                            mapping.method,
                            &mapping.read_scope_arguments,
                            collector.meter.control,
                        )?;
                        let mapping_origin =
                            collector.origin(Origin::Mapping(reference, mapping.method))?;
                        collector.target(mapping_origin, *key, target, scope)?;
                    }
                }
                DeclarationPayload::Function(function) => {
                    for prerequisite in &function.implementation_parameters {
                        collector.contract(
                            origin,
                            *key,
                            prerequisite.contract,
                            prerequisite.self_type,
                            &prerequisite.type_arguments,
                            scope,
                        )?;
                    }
                }
                DeclarationPayload::ReadView(view) => {
                    for field in &view.fields {
                        collector.meter.tick()?;
                        collector.types(origin, *key, field.ty, scope)?;
                    }
                }
                DeclarationPayload::Record { fields, .. }
                | DeclarationPayload::OwnedRecord { fields, .. } => {
                    for field in fields {
                        collector.meter.tick()?;
                        let OwnerRecord::Field(field) = source
                            .owners
                            .get(&OwnerKey::Field(*field))
                            .ok_or_else(failure)?
                        else {
                            return Err(failure());
                        };
                        if field.declaration != *id {
                            return Err(failure());
                        }
                        collector.types(origin, field.header.owner, field.ty, scope)?;
                    }
                }
                DeclarationPayload::Variant { cases, .. } => {
                    for case in cases {
                        collector.meter.tick()?;
                        let OwnerRecord::Case(case) = source
                            .owners
                            .get(&OwnerKey::Case(*case))
                            .ok_or_else(failure)?
                        else {
                            return Err(failure());
                        };
                        if case.declaration != *id {
                            return Err(failure());
                        }
                        if let Some(payload) = case.payload {
                            collector.types(origin, case.header.owner, payload, scope)?;
                        }
                    }
                }
                DeclarationPayload::OwnedContract(_) => {
                    for ((contract, id), method) in &proof.contract_methods {
                        collector.meter.tick()?;
                        if *contract == reference {
                            let method_origin =
                                collector.origin(Origin::ContractMethod(reference, *id))?;
                            for ((owner, _), ty) in &method.roots {
                                collector.types(method_origin, *owner, *ty, scope)?;
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }
    Ok(collector.queue)
}

fn constraint(
    schema: &NormalizedReferenceSchema,
    sources: &BTreeMap<PackageId, &KernelSnapshot>,
    target: DeclarationReference,
    actuals: &[TypeObjectDigest],
    scope: Option<DeclarationReference>,
    meter: &mut Meter<'_>,
) -> Result<(), ExecutionError> {
    meter.tick()?;
    let OwnerRecord::Declaration(owner) =
        indexed_owner(sources, target, OwnerKey::Declaration(target.declaration))?
    else {
        return Err(failure());
    };
    let parameters = owner.payload.type_parameters();
    if parameters.len() != actuals.len() {
        return Err(failure());
    }
    for (parameter, actual) in parameters.iter().zip(actuals) {
        meter.tick()?;
        let OwnerRecord::TypeParameter(record) =
            indexed_owner(sources, target, OwnerKey::TypeParameter(*parameter))?
        else {
            return Err(failure());
        };
        if schema.is_read_view_type(*actual) {
            return Err(failure());
        }
        if record.declaration != target.declaration
            || indexed_owned(schema, sources, *actual, scope, meter)?
                != record.constraints.has_owned()
        {
            return Err(failure());
        }
        if record.constraints.requires_capture_safe() {
            capture(schema, sources, *actual, scope, meter)?;
        }
        if record.constraints.requires_transfer() {
            parallel_carrier_type_indexed(
                schema,
                sources,
                *actual,
                scope,
                meter.visits,
                meter.bytes,
                meter.control,
                false,
            )?;
        }
        if record.constraints.requires_share() {
            parallel_carrier_type_indexed(
                schema,
                sources,
                *actual,
                scope,
                meter.visits,
                meter.bytes,
                meter.control,
                true,
            )?;
        }
    }
    Ok(())
}

fn capture(
    schema: &NormalizedReferenceSchema,
    sources: &BTreeMap<PackageId, &KernelSnapshot>,
    root: TypeObjectDigest,
    scope: Option<DeclarationReference>,
    meter: &mut Meter<'_>,
) -> Result<(), ExecutionError> {
    meter.reserve::<TypeObjectDigest>(1)?;
    let mut pending = vec![root];
    let mut seen = BTreeSet::new();
    while let Some(ty) = pending.pop() {
        meter.tick()?;
        if !seen.contains(&ty) {
            meter.node::<TypeObjectDigest>()?;
            seen.insert(ty);
        } else {
            continue;
        }
        if schema.capture_safe_types.contains(&ty) {
            continue;
        }
        let object = schema.types.get(&ty).ok_or_else(failure)?;
        match &object.form {
            TypeForm::TypeParameter { parameter } => {
                let scope = scope.ok_or_else(failure)?;
                let OwnerRecord::TypeParameter(record) =
                    indexed_owner(sources, scope, OwnerKey::TypeParameter(*parameter))?
                else {
                    return Err(failure());
                };
                if record.declaration != scope.declaration
                    || !record.constraints.proves_capture_safe()
                {
                    return Err(failure());
                }
                indexed_owned(schema, sources, ty, Some(scope), meter)?;
            }
            TypeForm::Function { .. } | TypeForm::TaskFunction { .. } => {}
            TypeForm::List { .. }
            | TypeForm::Option { .. }
            | TypeForm::Map { .. }
            | TypeForm::Result { .. }
            | TypeForm::StructuralRecord { .. } => {
                meter.reserve::<TypeObjectDigest>(object.child_type_count())?;
                let children = object.child_types();
                meter.reserve::<TypeObjectDigest>(children.len())?;
                pending.extend(children);
            }
            TypeForm::Named { .. } | TypeForm::Applied { .. } => {
                meter.reserve::<TypeObjectDigest>(object.child_type_count())?;
                let children = object.child_types();
                meter.reserve::<TypeObjectDigest>(children.len())?;
                pending.extend(children);
                if let Some(index) = schema.record_instances.get(&ty) {
                    let layout = schema.records.get(*index).ok_or_else(failure)?;
                    if layout.owned {
                        return Err(failure());
                    }
                    meter.reserve::<TypeObjectDigest>(layout.fields.len())?;
                    pending.extend(layout.fields.iter().map(|field| field.ty));
                } else if let Some(index) = schema.variant_instances.get(&ty) {
                    let layout = schema.variants.get(*index).ok_or_else(failure)?;
                    meter.reserve::<TypeObjectDigest>(layout.cases.len())?;
                    pending.extend(layout.cases.iter().filter_map(|case| case.payload));
                } else {
                    return Err(failure());
                }
            }
            TypeForm::ReadViewApplied { .. }
            | TypeForm::AssociatedReadViewMember { .. }
            | TypeForm::AssociatedReadViewProjection { .. } => return Err(failure()),
            _ => return Err(failure()),
        }
    }
    Ok(())
}

pub(super) fn discharge(
    schema: &NormalizedReferenceSchema,
    proof: &Proof,
    sources: &BTreeMap<PackageId, &KernelSnapshot>,
    queue: Queue,
    nominal: NominalConstraints,
    meter: &mut Meter<'_>,
) -> Result<(), ExecutionError> {
    meter.tick()?;
    for obligation in queue.constraints {
        meter.tick()?;
        let origin = queue.origins.get(obligation.origin).ok_or_else(failure)?;
        let package = package(origin, proof)?;
        let source = sources.get(&package).ok_or_else(failure)?;
        let owner = source.owners.get(&obligation.owner).ok_or_else(failure)?;
        if owner.header().owner != obligation.owner {
            return Err(failure());
        }
        if let Origin::Read(context) = origin {
            let function = proof.read_function(context, meter.control)?;
            if function.occurrences.get(&obligation.owner) != Some(&owner.header().contract_version)
            {
                return Err(failure());
            }
        } else if let Origin::Mapping(reference, method) = origin {
            let template = proof.mapping_template_at(*reference, *method, meter.control)?;
            proof.symbolic_mapping(
                *reference,
                *method,
                &template.read_scope_arguments,
                meter.control,
            )?;
            if obligation.owner != OwnerKey::Declaration(reference.declaration) {
                return Err(failure());
            }
        } else if let Origin::ContractMethod(reference, method) = origin {
            let signature = proof
                .contract_methods
                .get(&(*reference, *method))
                .ok_or_else(failure)?;
            if signature.occurrences.get(&obligation.owner)
                != Some(&owner.header().contract_version)
            {
                return Err(failure());
            }
        }
        constraint(
            schema,
            sources,
            obligation.target,
            &obligation.actuals,
            obligation.scope,
            meter,
        )
        .map_err(|error| at_stage(error, "associated permission constraint"))?;
    }
    for (target, actuals, scope) in nominal {
        constraint(schema, sources, target, &actuals, scope, meter)
            .map_err(|error| at_stage(error, "associated nominal permission constraint"))?;
    }
    for parallel in queue.parallels {
        meter.tick()?;
        let Origin::Read(context) = queue.origins.get(parallel.origin).ok_or_else(failure)? else {
            return Err(failure());
        };
        let function = proof.read_function(context, meter.control)?;
        let package = package(&queue.origins[parallel.origin], proof)?;
        let source = sources.get(&package).ok_or_else(failure)?;
        let parent_key = OwnerKey::Expression(parallel.parent);
        let OwnerRecord::Expression(parent) = source.owners.get(&parent_key).ok_or_else(failure)?
        else {
            return Err(failure());
        };
        if function.occurrences.get(&parent_key) != Some(&parent.contract_version)
            || !matches!(parent.operation, ExpressionOperation::Parallel {left,right} if left==parallel.left && right==parallel.right)
            || parent.contract_version != crate::platform::kernel::contract::GRAPH_CONTRACT_VERSION
        {
            return Err(failure());
        }
        for child in [parallel.left, parallel.right] {
            meter.tick()?;
            let key = OwnerKey::Expression(child);
            let OwnerRecord::Expression(expression) =
                source.owners.get(&key).ok_or_else(failure)?
            else {
                return Err(failure());
            };
            if function.occurrences.get(&key) != Some(&expression.contract_version)
                || queue.parents.get(&(package, child))
                    != Some(&Parent::Expression(parallel.parent))
            {
                return Err(failure());
            }
            let (target, arguments, type_arguments) = match &expression.operation {
                ExpressionOperation::Call {
                    function,
                    arguments,
                    type_arguments,
                    ..
                }
                | ExpressionOperation::ImplementationCall {
                    function,
                    arguments,
                    type_arguments,
                    ..
                } => (*function, arguments, type_arguments),
                _ => return Err(failure()),
            };
            let OwnerRecord::Declaration(declaration) =
                indexed_owner(sources, target, OwnerKey::Declaration(target.declaration))?
            else {
                return Err(failure());
            };
            let DeclarationPayload::Function(raw) = &declaration.payload else {
                return Err(failure());
            };
            if !raw.effect_parameters.is_empty()
                || !raw.requirement_parameters.is_empty()
                || !matches!(&raw.effect, FunctionEffect::Task {requirements,effect_parameters} if requirements.is_empty() && effect_parameters.is_empty())
                || raw.parameters.len() != arguments.len()
            {
                return Err(failure());
            }
            let (parameters, result, borrowed, is_task, mut associated, mut read_view, effect) =
                if let Some(application) = function.targets.get(&child) {
                    if application.function != target {
                        return Err(failure());
                    }
                    let signature = proof.application(application, meter.control)?;
                    (
                        &signature.parameters,
                        signature.result.ty,
                        signature.result.relation.source().copied(),
                        signature.is_task,
                        signature.requires_associated_generation,
                        signature.requires_read_view_generation,
                        &signature.effect,
                    )
                } else {
                    let target_descriptor =
                        function.symbolic_targets.get(&child).ok_or_else(failure)?;
                    if target_descriptor.function != Some(target) {
                        return Err(failure());
                    }
                    let signature = &target_descriptor.signature;
                    (
                        &signature.parameters,
                        signature.result.ty,
                        signature.result.relation.source().copied(),
                        signature.is_task,
                        signature.requires_associated_generation,
                        signature.requires_read_view_generation,
                        &signature.effect,
                    )
                };
            for ty in type_arguments {
                meter.tick()?;
                associated |= proof.raw_templates.contains(ty);
                read_view |= proof.raw_read_view_templates.contains(ty);
            }
            associated |= function.associated_targets.contains(&child);
            read_view |= function.read_view_targets.contains(&child)
                || !expression.operation.read_scope_arguments().is_empty();
            associated |= proof.raw_templates.contains(&result);
            read_view |= proof.raw_read_view_templates.contains(&result);
            for (ty, _) in parameters {
                meter.tick()?;
                associated |= proof.raw_templates.contains(ty);
                read_view |= proof.raw_read_view_templates.contains(ty);
            }
            if !is_task
                || borrowed.is_some()
                || !effect.parameters.is_empty()
                || !effect.requirements.is_empty()
                || parameters.len() != raw.parameters.len()
                || (associated || read_view)
                    && (!associated_generation_admitted(parent.contract_version, read_view)
                        || !associated_generation_admitted(expression.contract_version, read_view)
                        || !associated_generation_admitted(
                            source.root.graph_contract_version,
                            read_view,
                        )
                        || parent.contract_version != source.root.graph_contract_version
                        || expression.contract_version != source.root.graph_contract_version)
            {
                return Err(failure());
            }
            let scope = scope(context, function, sources, meter)?;
            parallel_carrier_type_indexed(
                schema,
                sources,
                result,
                scope,
                meter.visits,
                meter.bytes,
                meter.control,
                false,
            )?;
            let mut seen_owned = false;
            for (id, (ty, use_mode)) in raw.parameters.iter().zip(parameters) {
                meter.tick()?;
                let OwnerRecord::Parameter(parameter) =
                    indexed_owner(sources, target, OwnerKey::Parameter(*id))?
                else {
                    return Err(failure());
                };
                let owned = indexed_owned(schema, sources, *ty, scope, meter)?;
                let view = schema.is_read_view_type(*ty);
                if parameter.parent != ParameterParent::Function(target.declaration)
                    || parameter.resource_requirement.is_some()
                    || parameter.use_mode != *use_mode
                    || seen_owned && !owned
                    || if owned || view {
                        !matches!(use_mode, ParameterUse::Consume | ParameterUse::Borrow)
                    } else {
                        *use_mode != ParameterUse::Unrestricted
                    }
                {
                    return Err(failure());
                }
                seen_owned |= owned;
                let share = *use_mode == ParameterUse::Borrow;
                if share
                    && (declaration.header.contract_version
                        != crate::platform::kernel::contract::GRAPH_CONTRACT_VERSION
                        || parameter.header.contract_version
                            != crate::platform::kernel::contract::GRAPH_CONTRACT_VERSION)
                {
                    return Err(failure());
                }
                parallel_carrier_type_indexed(
                    schema,
                    sources,
                    *ty,
                    scope,
                    meter.visits,
                    meter.bytes,
                    meter.control,
                    share,
                )?;
            }
        }
    }
    Ok(())
}
