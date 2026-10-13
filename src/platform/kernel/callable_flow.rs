//! Package-local ordinary preflight from explicit, simultaneous parameter flow.
//! Associated projections and supplier callbacks require the complete composed
//! admission exported below. Success here alone is not a callable-closure proof.
//! Read-scope coordinates contribute finite inventory work, never ordinary type
//! constructor edges or value/custody permission.
//! No runtime branch, variance, nominal layout or preparation frontier changes this relation.

#[cfg(test)]
#[path = "callable_flow_tests.rs"]
mod tests;

#[path = "composed_callable_flow.rs"]
mod composed;
pub(crate) use composed::{
    CallableClosureRead, validate_callable_closure, validate_snapshot_callable_closure,
};
#[cfg(test)]
pub(crate) use composed::{
    validate_snapshot_callable_closure_eager, validate_snapshot_callable_closure_with_limits,
};

use super::{
    DeclarationPayload, DeclarationReference, ExpressionOperation, ExpressionRead, OwnerKey,
    OwnerRecord, TypeForm, TypeObjectDigest,
};
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::semantic_id::{ExpressionId, TypeParameterId};
use std::collections::{BTreeMap, BTreeSet};

const MAXIMUM_ANALYSIS_BYTES: usize = 64 * 1_048_576;
const MAXIMUM_DIAGNOSTIC_PATH: usize = 8;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct Slot {
    function: DeclarationReference,
    ordinal: usize,
    parameter: TypeParameterId,
}

struct Edge {
    from: usize,
    to: usize,
    expression: ExpressionId,
    argument: usize,
    argument_type: TypeObjectDigest,
    path: Vec<usize>,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct CallableFlowWork {
    pub functions: usize,
    pub applications: usize,
    pub slots: usize,
    pub edges: usize,
    pub metadata_bytes: usize,
}

struct Flow<'a, R> {
    read: &'a R,
    work: &'a mut usize,
    maximum_work: usize,
    observation: CallableFlowWork,
    slots: Vec<Slot>,
    indexes: BTreeMap<Slot, usize>,
    edges: Vec<Edge>,
}

impl<R: ExpressionRead> Flow<'_, R> {
    fn tick(&mut self) -> Result<(), Diagnostic> {
        self.read.validation_checkpoint()?;
        *self.work = self
            .work
            .checked_add(1)
            .filter(|next| *next <= self.maximum_work)
            .ok_or_else(|| {
                Diagnostic::new(
                    DiagnosticClass::Resource,
                    "kernel_callable_flow_work",
                    "callable parameter-flow analysis exhausted its work admission",
                )
            })?;
        Ok(())
    }

    fn reserve<T>(&mut self, count: usize) -> Result<(), Diagnostic> {
        self.observation.metadata_bytes = count
            .checked_mul(std::mem::size_of::<T>() + 4 * std::mem::size_of::<usize>())
            .and_then(|size| self.observation.metadata_bytes.checked_add(size))
            .filter(|size| *size <= MAXIMUM_ANALYSIS_BYTES)
            .ok_or_else(|| {
                Diagnostic::new(
                    DiagnosticClass::Resource,
                    "kernel_callable_flow_storage",
                    "callable parameter-flow analysis exceeded its metadata admission",
                )
            })?;
        Ok(())
    }

    fn slot(&mut self, slot: Slot) -> Result<usize, Diagnostic> {
        if let Some(index) = self.indexes.get(&slot) {
            return Ok(*index);
        }
        self.reserve::<(Slot, usize)>(2)?;
        let index = self.slots.len();
        self.slots.push(slot);
        self.indexes.insert(slot, index);
        Ok(index)
    }

    fn scope_arguments(
        &mut self,
        arguments: &[super::ReadScopeArgument],
    ) -> Result<(), Diagnostic> {
        self.reserve::<super::ReadScopeArgument>(arguments.len())?;
        // Equal symbolic actuals do not merge their ordered source positions. Their exact
        // binder, containment and invocation authority are admitted independently.
        for _ in arguments {
            self.tick()?;
        }
        Ok(())
    }

    fn scope_inventory<S>(
        &mut self,
        parameters: &[super::ReadScopeParameter],
        sources: &[super::ReadScopeSource<S>],
    ) -> Result<(), Diagnostic> {
        self.reserve::<super::ReadScopeParameter>(parameters.len())?;
        self.reserve::<super::ReadScopeSource<S>>(sources.len())?;
        for _ in parameters {
            self.tick()?;
        }
        for _ in sources {
            self.tick()?;
        }
        Ok(())
    }

    fn declaration_scopes(&mut self, payload: &DeclarationPayload) -> Result<(), Diagnostic> {
        match payload {
            DeclarationPayload::Function(function) => {
                self.scope_inventory(
                    &function.read_scope_parameters,
                    &function.read_scope_sources,
                )?;
                if let super::ResultRelation::ViewFrom { scope, .. } = function.result.relation {
                    self.scope_arguments(std::slice::from_ref(&scope))?;
                }
            }
            DeclarationPayload::OwnedContract(contract) => {
                for member in &contract.associated_types {
                    self.tick()?;
                    if let super::AssociatedTypeBound::ReadViewFamily { scope_parameter } =
                        &member.bound
                    {
                        self.scope_inventory::<u32>(std::slice::from_ref(scope_parameter), &[])?;
                    }
                }
                for method in &contract.methods {
                    self.tick()?;
                    self.scope_inventory(
                        &method.read_scope_parameters,
                        &method.read_scope_sources,
                    )?;
                    if let super::ResultRelation::ViewFrom { scope, .. } = method.result.relation {
                        self.scope_arguments(std::slice::from_ref(&scope))?;
                    }
                }
            }
            DeclarationPayload::OwnedImplementation(implementation) => {
                for binding in &implementation.associated_types {
                    self.tick()?;
                    if let super::AssociatedTypeBindingValue::ReadViewFamily {
                        scope_parameter,
                        ..
                    } = &binding.value
                    {
                        self.scope_inventory::<u32>(std::slice::from_ref(scope_parameter), &[])?;
                    }
                }
                for mapping in &implementation.methods {
                    self.tick()?;
                    self.scope_arguments(&mapping.read_scope_arguments)?;
                }
            }
            DeclarationPayload::ReadView(view) => {
                self.scope_inventory::<u32>(std::slice::from_ref(&view.scope_parameter), &[])?;
                for field in &view.fields {
                    self.tick()?;
                    if let super::ReadViewFieldKind::ReadReference { read_scope } = field.kind {
                        self.scope_arguments(std::slice::from_ref(&read_scope))?;
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn application(
        &mut self,
        caller: DeclarationReference,
        parameters: &[TypeParameterId],
        expression: ExpressionId,
        target: DeclarationReference,
        arguments: &[TypeObjectDigest],
        read_scope_arguments: &[super::ReadScopeArgument],
    ) -> Result<(), Diagnostic> {
        self.tick()?;
        self.observation.applications += 1;
        self.scope_arguments(read_scope_arguments)?;
        // This declaration-local pass has no complete supplier bodies or downstream witness
        // selections. The mandatory composed source-closure pass at publication, transport and
        // artifact admission closes foreign calls and selected method callbacks independently.
        if target.package != self.read.package_id() {
            return Ok(());
        }
        let Some(OwnerRecord::Declaration(declaration)) =
            self.read.owner(OwnerKey::Declaration(target.declaration))?
        else {
            return Err(semantic(
                "kernel_callable_flow_target",
                "callable application target is missing",
            ));
        };
        let (target_parameters, scope_arity) = match &declaration.payload {
            DeclarationPayload::Function(function) => (
                &function.type_parameters,
                function.read_scope_parameters.len(),
            ),
            DeclarationPayload::External(function) => (&function.type_parameters, 0),
            _ => {
                return Err(semantic(
                    "kernel_callable_flow_target",
                    "application target is not callable",
                ));
            }
        };
        if target_parameters.len() != arguments.len() {
            return Err(semantic(
                "kernel_callable_flow_arity",
                "callable application requires every ordered ordinary type argument",
            ));
        }
        if scope_arity != read_scope_arguments.len() {
            return Err(semantic(
                "kernel_callable_flow_arity",
                "callable application requires every ordered read-scope argument, including unused formals",
            ));
        }
        for (argument, (ty, target_parameter)) in
            arguments.iter().zip(target_parameters).enumerate()
        {
            self.reserve::<(TypeObjectDigest, Vec<usize>)>(1)?;
            let mut pending = vec![(*ty, Vec::new())];
            let mut visited = BTreeSet::new();
            while let Some((digest, path)) = pending.pop() {
                self.tick()?;
                // The same subterm at the entire argument and underneath a constructor has
                // different flow meaning. Keep both facts, retaining one exact occurrence path.
                let occurrence = (digest, !path.is_empty());
                if visited.contains(&occurrence) {
                    continue;
                }
                self.reserve::<(TypeObjectDigest, bool)>(1)?;
                visited.insert(occurrence);
                let object = self.read.type_object(digest)?.ok_or_else(|| {
                    semantic(
                        "kernel_callable_flow_type",
                        "callable application type object is missing",
                    )
                })?;
                match &object.form {
                    TypeForm::ReadViewApplied { read_scope, .. }
                    | TypeForm::AssociatedReadViewMember { read_scope, .. }
                    | TypeForm::AssociatedReadViewProjection { read_scope, .. } => {
                        self.scope_arguments(std::slice::from_ref(read_scope))?;
                    }
                    _ => {}
                }
                if let TypeForm::TypeParameter { parameter } = object.form {
                    let ordinal = parameters
                        .iter()
                        .position(|value| *value == parameter)
                        .ok_or_else(|| {
                            semantic(
                                "kernel_type_parameter_scope",
                                "callable argument contains a foreign caller parameter",
                            )
                        })?;
                    let from = self.slot(Slot {
                        function: caller,
                        ordinal,
                        parameter,
                    })?;
                    let to = self.slot(Slot {
                        function: target,
                        ordinal: argument,
                        parameter: *target_parameter,
                    })?;
                    self.reserve::<Edge>(1)?;
                    self.edges.push(Edge {
                        from,
                        to,
                        expression,
                        argument,
                        argument_type: *ty,
                        path,
                    });
                } else if object.form.is_associated() {
                    // A zero-child associated form has no ordinary dependency to
                    // add here. Its exact member/binding/path relation is admitted
                    // by the mandatory complete-source composed pass, including
                    // scope substitution for a ReadViewFamily projection.
                } else {
                    for (index, child) in object.child_types().into_iter().enumerate() {
                        self.tick()?;
                        if path.len() >= super::contract::MAXIMUM_TYPE_DEPTH {
                            return Err(Diagnostic::new(
                                DiagnosticClass::Resource,
                                "kernel_callable_flow_type_depth",
                                "callable argument exceeds the structural type-depth contract",
                            ));
                        }
                        self.reserve::<usize>(path.len() + 1)?;
                        self.reserve::<(TypeObjectDigest, Vec<usize>)>(1)?;
                        let mut child_path = path.clone();
                        child_path.push(index);
                        pending.push((child, child_path));
                    }
                }
            }
        }
        Ok(())
    }

    fn scan(&mut self, roots: impl IntoIterator<Item = OwnerKey>) -> Result<(), Diagnostic> {
        let mut pending = BTreeSet::new();
        for root in roots {
            self.tick()?;
            if let OwnerKey::Declaration(_) = root {
                self.reserve::<OwnerKey>(1)?;
                pending.insert(root);
            }
        }
        let mut visited = BTreeSet::new();
        while let Some(owner) = pending.pop_first() {
            self.tick()?;
            if visited.contains(&owner) {
                continue;
            }
            self.reserve::<OwnerKey>(1)?;
            visited.insert(owner);
            let OwnerKey::Declaration(id) = owner else {
                continue;
            };
            let Some(OwnerRecord::Declaration(declaration)) = self.read.owner(owner)? else {
                continue;
            };
            // Charge complete inline scope inventories before skipping a declaration without
            // body syntax. Unused methods, bindings, fields and formals remain relevant.
            self.declaration_scopes(&declaration.payload)?;
            let parameters = declaration.payload.type_parameters().to_vec();
            let mut syntax = declaration.expression_roots();
            if syntax.is_empty() {
                continue;
            }
            self.observation.functions += usize::from(matches!(
                declaration.payload,
                DeclarationPayload::Function(_)
            ));
            self.reserve::<ExpressionId>(syntax.len())?;
            let caller = DeclarationReference {
                package: self.read.package_id(),
                declaration: id,
            };
            let mut expressions = BTreeSet::new();
            while let Some(expression) = syntax.pop() {
                self.tick()?;
                if expressions.contains(&expression) {
                    continue;
                }
                self.reserve::<ExpressionId>(1)?;
                expressions.insert(expression);
                let Some(OwnerRecord::Expression(record)) =
                    self.read.owner(OwnerKey::Expression(expression))?
                else {
                    return Err(semantic(
                        "kernel_callable_flow_expression",
                        "callable body expression is missing",
                    ));
                };
                match &record.operation {
                    ExpressionOperation::ImplementationCall {
                        function,
                        type_arguments,
                        read_scope_arguments,
                        implementations,
                        requirement_arguments,
                        ..
                    } => {
                        self.reserve::<super::RequirementOperand>(requirement_arguments.len())?;
                        for _ in requirement_arguments {
                            self.tick()?;
                        }
                        self.reserve::<super::ImplementationOperand>(implementations.len())?;
                        for operand in implementations {
                            self.tick()?;
                            if let super::ImplementationOperand::Concrete {
                                implementation, ..
                            } = operand
                            {
                                self.implementation_targets(*implementation, &mut pending)?;
                            }
                        }
                        self.application(
                            caller,
                            &parameters,
                            expression,
                            *function,
                            type_arguments,
                            read_scope_arguments,
                        )?;
                        if function.package == self.read.package_id() {
                            self.reserve::<OwnerKey>(1)?;
                            pending.insert(OwnerKey::Declaration(function.declaration));
                        }
                    }
                    ExpressionOperation::MethodCall {
                        witness,
                        read_scope_arguments,
                        ..
                    } => {
                        self.tick()?;
                        self.scope_arguments(read_scope_arguments)?;
                        if let super::ImplementationOperand::Concrete { implementation, .. } =
                            witness
                        {
                            self.implementation_targets(*implementation, &mut pending)?;
                        }
                    }
                    ExpressionOperation::Call {
                        function,
                        type_arguments,
                        read_scope_arguments,
                        requirement_arguments,
                        ..
                    }
                    | ExpressionOperation::FunctionValue {
                        function,
                        type_arguments,
                        read_scope_arguments,
                        requirement_arguments,
                        ..
                    } => {
                        // Rank-one requirement arguments only select an exact atom or a caller
                        // formal. Their permutations add finite application keys, never type
                        // constructor edges. Still admit and charge every explicit argument.
                        self.reserve::<super::RequirementOperand>(requirement_arguments.len())?;
                        for _ in requirement_arguments {
                            self.tick()?;
                        }
                        self.application(
                            caller,
                            &parameters,
                            expression,
                            *function,
                            type_arguments,
                            read_scope_arguments,
                        )?;
                        if function.package == self.read.package_id() {
                            self.reserve::<OwnerKey>(1)?;
                            pending.insert(OwnerKey::Declaration(function.declaration));
                        }
                    }
                    ExpressionOperation::CreateReadView { read_scope, .. }
                    | ExpressionOperation::ReborrowReadView { read_scope, .. } => {
                        self.scope_arguments(std::slice::from_ref(read_scope))?;
                    }
                    _ => {}
                }
                let children = record.children();
                self.reserve::<ExpressionId>(children.len())?;
                syntax.extend(children.into_iter().map(|child| child.expression));
            }
        }
        Ok(())
    }

    fn implementation_targets(
        &mut self,
        reference: DeclarationReference,
        pending: &mut BTreeSet<OwnerKey>,
    ) -> Result<(), Diagnostic> {
        self.tick()?;
        let methods = if reference.package == self.read.package_id() {
            match self
                .read
                .owner(OwnerKey::Declaration(reference.declaration))?
            {
                Some(OwnerRecord::Declaration(d)) => match d.payload {
                    DeclarationPayload::OwnedImplementation(i) => i.methods,
                    _ => {
                        return Err(semantic(
                            "kernel_owned_contract",
                            "witness selects a foreign owner kind",
                        ));
                    }
                },
                _ => {
                    return Err(semantic(
                        "kernel_owned_contract",
                        "missing selected implementation",
                    ));
                }
            }
        } else {
            match self.read.package_interface_owner(
                reference.package,
                OwnerKey::Declaration(reference.declaration),
            )? {
                Some(super::PackageInterfaceRecord::Declaration(d)) => match d.payload {
                    super::PackageInterfaceDeclarationPayload::OwnedImplementation(i) => i.methods,
                    _ => {
                        return Err(semantic(
                            "kernel_owned_contract",
                            "witness selects a foreign imported kind",
                        ));
                    }
                },
                _ => {
                    return Err(semantic(
                        "kernel_owned_contract",
                        "missing imported implementation",
                    ));
                }
            }
        };
        for mapping in methods {
            self.tick()?;
            self.scope_arguments(&mapping.read_scope_arguments)?;
            if mapping.function.package == self.read.package_id() {
                self.reserve::<OwnerKey>(1)?;
                pending.insert(OwnerKey::Declaration(mapping.function.declaration));
            }
        }
        Ok(())
    }

    fn admit(&mut self) -> Result<(), Diagnostic> {
        let count = self.slots.len();
        self.reserve::<Vec<usize>>(count.checked_mul(2).ok_or_else(storage_overflow)?)?;
        self.reserve::<usize>(count.checked_mul(10).ok_or_else(storage_overflow)?)?;
        self.reserve::<usize>(
            self.edges
                .len()
                .checked_mul(2)
                .ok_or_else(storage_overflow)?,
        )?;
        let mut outgoing = vec![Vec::new(); count];
        let mut incoming = vec![Vec::new(); count];
        for index in 0..self.edges.len() {
            self.tick()?;
            let edge = &self.edges[index];
            outgoing[edge.from].push(edge.to);
            incoming[edge.to].push(edge.from);
        }
        let mut seen = vec![false; count];
        let mut finish = Vec::new();
        for root in 0..count {
            self.tick()?;
            if seen[root] {
                continue;
            }
            seen[root] = true;
            let mut stack = vec![(root, 0)];
            while let Some((node, next)) = stack.last_mut() {
                self.tick()?;
                if let Some(target) = outgoing[*node].get(*next) {
                    *next += 1;
                    if !seen[*target] {
                        seen[*target] = true;
                        stack.push((*target, 0));
                    }
                } else {
                    finish.push(*node);
                    stack.pop();
                }
            }
        }
        let mut component = vec![usize::MAX; count];
        let mut sizes = vec![0; count];
        for root in finish.into_iter().rev() {
            self.tick()?;
            if component[root] != usize::MAX {
                continue;
            }
            component[root] = root;
            let mut pending = vec![root];
            while let Some(node) = pending.pop() {
                self.tick()?;
                sizes[root] += 1;
                for source in &incoming[node] {
                    self.tick()?;
                    if component[*source] == usize::MAX {
                        component[*source] = root;
                        pending.push(*source);
                    }
                }
            }
        }
        for index in 0..self.edges.len() {
            self.tick()?;
            let edge = &self.edges[index];
            if !edge.path.is_empty() && component[edge.from] == component[edge.to] {
                // The SCC decision is complete. Rendering cannot change semantic rejection into
                // resource exhaustion, even for a cycle much longer than the witness bound.
                let from = self.slots[edge.from];
                let to = self.slots[edge.to];
                let path = &edge.path[..edge.path.len().min(MAXIMUM_DIAGNOSTIC_PATH)];
                let omitted = if edge.path.len() > path.len() {
                    " (remaining constructor path omitted)"
                } else {
                    ""
                };
                return Err(semantic(
                    "kernel_callable_expansion",
                    format!(
                        "expanding callable parameter cycle: {}::{} slot {} ({}) -> {}::{} slot {} ({}); expression {}; argument {} type {}; constructor path {:?}{}; same strongly connected component ({} slots), return-path witness omitted; forward parameters without nesting on the cycle or reset to a closed type",
                        from.function.package,
                        from.function.declaration,
                        from.ordinal,
                        from.parameter,
                        to.function.package,
                        to.function.declaration,
                        to.ordinal,
                        to.parameter,
                        edge.expression,
                        edge.argument,
                        edge.argument_type,
                        path,
                        omitted,
                        sizes[component[edge.from]],
                    ),
                ));
            }
        }
        Ok(())
    }
}

fn semantic(code: &str, message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(DiagnosticClass::Semantic, code, message)
}

fn storage_overflow() -> Diagnostic {
    Diagnostic::new(
        DiagnosticClass::Resource,
        "kernel_callable_flow_storage",
        "callable flow metadata size overflowed",
    )
}

/// Ordinary package-local preflight only; exposing boundaries must also run the
/// complete-source composed admission for callbacks and associated outputs.
pub(crate) fn validate_callable_flow<R: ExpressionRead>(
    read: &R,
    roots: impl IntoIterator<Item = OwnerKey>,
    work: &mut usize,
    maximum_work: usize,
) -> Result<CallableFlowWork, Diagnostic> {
    let mut flow = Flow {
        read,
        work,
        maximum_work,
        observation: CallableFlowWork::default(),
        slots: Vec::new(),
        indexes: BTreeMap::new(),
        edges: Vec::new(),
    };
    flow.scan(roots)?;
    flow.admit()?;
    flow.observation.slots = flow.slots.len();
    flow.observation.edges = flow.edges.len();
    Ok(flow.observation)
}

#[cfg(test)]
pub(super) mod read_view_tests {
    use super::*;
    use crate::platform::kernel::*;
    use crate::platform::semantic_id::{
        BindingId, DeclarationId, FieldId, ModuleId, ParameterId, ReadScopeParameterId,
    };

    const SEED: &[u8] = b"ordinary-read-view-flow-preflight";

    pub(super) struct Read {
        pub(super) owners: BTreeMap<OwnerKey, OwnerRecord>,
        pub(super) types: TypeObjectInterner,
        next_expression: u64,
        pub(super) parameter_type: TypeObjectDigest,
    }

    pub(super) fn function() -> DeclarationReference {
        DeclarationReference {
            package: PackageId::migrate(SEED, 0),
            declaration: DeclarationId::migrate(SEED, 0),
        }
    }

    pub(super) fn scope() -> ReadScopeArgument {
        ReadScopeArgument::Parameter(ReadScopeReference {
            binder: ReadScopeBinder::Declaration(function()),
            parameter: ReadScopeParameterId::migrate(SEED, 0),
        })
    }

    impl ExpressionRead for Read {
        fn package_id(&self) -> PackageId {
            function().package
        }

        fn owner(&self, owner: OwnerKey) -> Result<Option<OwnerRecord>, Diagnostic> {
            Ok(self.owners.get(&owner).cloned())
        }

        fn type_object(&self, ty: TypeObjectDigest) -> Result<Option<TypeObject>, Diagnostic> {
            Ok(self.types.get(ty).cloned())
        }

        fn package_interface_owner(
            &self,
            _: PackageId,
            _: OwnerKey,
        ) -> Result<Option<PackageInterfaceRecord>, Diagnostic> {
            Ok(None)
        }

        fn has_dependency(&self, _: PackageId) -> Result<bool, Diagnostic> {
            Ok(false)
        }
    }

    impl Read {
        pub(super) fn new() -> Self {
            let mut types = TypeObjectInterner::default();
            let parameter = TypeParameterId::migrate(SEED, 0);
            let parameter_type = types
                .intern(TypeForm::TypeParameter { parameter })
                .expect("parameter type");
            let result = types.intern(TypeForm::I64).expect("result type");
            let mut read = Self {
                owners: BTreeMap::new(),
                types,
                next_expression: 0,
                parameter_type,
            };
            let body = read.expression(ExpressionOperation::I64 { value: 7 });
            let parameter_owner = OwnerKey::TypeParameter(parameter);
            read.owners.insert(
                parameter_owner,
                OwnerRecord::TypeParameter(TypeParameterRecord {
                    header: OwnerHeader::new(parameter_owner, OwnerKind::TypeParameter),
                    declaration: function().declaration,
                    name: Name::new("T").expect("parameter name"),
                    constraints: TypeParameterConstraints::None,
                }),
            );
            let owner = OwnerKey::Declaration(function().declaration);
            read.owners.insert(
                owner,
                OwnerRecord::Declaration(DeclarationRecord {
                    header: OwnerHeader::new(owner, OwnerKind::PureFunction),
                    module: ModuleId::migrate(SEED, 0),
                    name: Name::new("preflight").expect("name"),
                    visibility: DeclarationVisibility::Private,
                    payload: DeclarationPayload::Function(FunctionDeclaration {
                        implementation_parameters: Vec::new(),
                        requirement_parameters: Vec::new(),
                        effect_parameters: Vec::new(),
                        type_parameters: vec![parameter],
                        parameters: Vec::new(),
                        result: CallableResult {
                            ty: result,
                            relation: ResultRelation::Value,
                        },
                        read_scope_parameters: vec![ReadScopeParameter {
                            id: ReadScopeParameterId::migrate(SEED, 0),
                            name: Name::new("source").expect("scope name"),
                        }],
                        read_scope_sources: Vec::new(),
                        effect: FunctionEffect::Pure,
                        body,
                    }),
                }),
            );
            read
        }

        pub(super) fn expression(&mut self, operation: ExpressionOperation) -> ExpressionId {
            let id = ExpressionId::migrate(SEED, self.next_expression);
            self.next_expression += 1;
            let record = ExpressionRecord::new(id, operation).expect("expression");
            self.owners
                .insert(OwnerKey::Expression(id), OwnerRecord::Expression(record));
            id
        }

        pub(super) fn body(&mut self, body: ExpressionId) {
            let Some(OwnerRecord::Declaration(declaration)) = self
                .owners
                .get_mut(&OwnerKey::Declaration(function().declaration))
            else {
                panic!("function");
            };
            let DeclarationPayload::Function(target) = &mut declaration.payload else {
                panic!("function payload");
            };
            target.body = body;
        }

        pub(super) fn call(
            &mut self,
            kind: u8,
            ty: TypeObjectDigest,
            scopes: Vec<ReadScopeArgument>,
        ) -> ExpressionId {
            self.expression(match kind {
                0 => ExpressionOperation::Call {
                    requirement_arguments: Vec::new(),
                    effect_arguments: Vec::new(),
                    function: function(),
                    type_arguments: vec![ty],
                    read_scope_arguments: scopes,
                    arguments: Vec::new(),
                },
                1 => ExpressionOperation::FunctionValue {
                    requirement_arguments: Vec::new(),
                    effect_arguments: Vec::new(),
                    function: function(),
                    type_arguments: vec![ty],
                    read_scope_arguments: scopes,
                },
                _ => ExpressionOperation::ImplementationCall {
                    requirement_arguments: Vec::new(),
                    effect_arguments: Vec::new(),
                    function: function(),
                    type_arguments: vec![ty],
                    read_scope_arguments: scopes,
                    implementations: Vec::new(),
                    arguments: Vec::new(),
                },
            })
        }

        fn admit(&self) -> Result<CallableFlowWork, Diagnostic> {
            validate_callable_flow(self, self.owners.keys().copied(), &mut 0, 100_000)
        }
    }

    // These fixtures isolate ordinary parameter flow. Success in this pass does not admit
    // a first-class scoped callable, a view as an ordinary actual, or a source/custody claim.
    #[test]
    fn preflight_keeps_unused_scope_formals_in_named_application_arity() {
        for kind in 0..3 {
            let mut read = Read::new();
            let body = read.call(kind, read.parameter_type, Vec::new());
            read.body(body);
            let error = read
                .admit()
                .expect_err("unused formal still requires an actual");
            assert_eq!(error.code, "kernel_callable_flow_arity");
            assert!(error.message.contains("read-scope"));

            if kind != 1 {
                let body = read.call(kind, read.parameter_type, vec![scope()]);
                read.body(body);
                assert!(read.admit().is_ok());
            }
        }
    }

    #[test]
    fn preflight_follows_phantom_read_view_type_actuals() {
        let mut read = Read::new();
        let view = read
            .types
            .intern(TypeForm::ReadViewApplied {
                declaration: DeclarationReference {
                    package: function().package,
                    declaration: DeclarationId::migrate(SEED, 1),
                },
                arguments: vec![read.parameter_type],
                read_scope: scope(),
            })
            .expect("phantom view application");
        let body = read.call(0, view, vec![scope()]);
        read.body(body);
        assert_eq!(
            read.admit().expect_err("constructor cycle").code,
            "kernel_callable_expansion"
        );
    }

    #[test]
    fn preflight_accounts_for_unused_view_inventory_without_body_syntax() {
        let mut read = Read::new();
        let declaration = DeclarationId::migrate(SEED, 1);
        let owner = OwnerKey::Declaration(declaration);
        let parameter = ReadScopeParameterId::migrate(SEED, 1);
        let read_scope = ReadScopeArgument::Parameter(ReadScopeReference {
            binder: ReadScopeBinder::Declaration(DeclarationReference {
                package: function().package,
                declaration,
            }),
            parameter,
        });
        let ty = read.types.intern(TypeForm::OwnedI64Cell).expect("referent");
        read.owners.insert(
            owner,
            OwnerRecord::Declaration(DeclarationRecord {
                header: OwnerHeader::new(owner, OwnerKind::ReadView),
                module: ModuleId::migrate(SEED, 0),
                name: Name::new("unused").expect("view name"),
                visibility: DeclarationVisibility::Private,
                payload: DeclarationPayload::ReadView(ReadViewDeclaration {
                    type_parameters: Vec::new(),
                    scope_parameter: ReadScopeParameter {
                        id: parameter,
                        name: Name::new("source").expect("scope name"),
                    },
                    fields: (0..128)
                        .map(|ordinal| ReadViewField {
                            id: FieldId::migrate(SEED, ordinal),
                            name: Name::new(format!("field{ordinal}")).expect("field name"),
                            ty,
                            kind: ReadViewFieldKind::ReadReference { read_scope },
                        })
                        .collect(),
                }),
            }),
        );
        let error = validate_callable_flow(&read, read.owners.keys().copied(), &mut 0, 64)
            .expect_err("unused inline fields remain within work admission");
        assert_eq!(error.class, DiagnosticClass::Resource);
        assert_eq!(error.code, "kernel_callable_flow_work");
    }

    #[test]
    fn preflight_reads_inactive_view_fields_and_scoped_bodies() {
        for kind in 0..3 {
            let mut read = Read::new();
            let list = read
                .types
                .intern(TypeForm::List {
                    item: read.parameter_type,
                })
                .expect("list");
            let call = read.call(0, list, vec![scope()]);
            let value = LocalValueReference::FunctionParameter(ParameterId::migrate(SEED, 0));
            let binding = BindingId::migrate(SEED, 0);
            let field = ReadViewFieldReference {
                declaration: DeclarationReference {
                    package: function().package,
                    declaration: DeclarationId::migrate(SEED, 1),
                },
                field: FieldId::migrate(SEED, 0),
            };
            let first_field = read.expression(ExpressionOperation::I64 { value: 7 });
            let view_syntax = read.expression(match kind {
                0 => ExpressionOperation::CreateReadView {
                    declaration: field.declaration,
                    type_arguments: Vec::new(),
                    read_scope: scope(),
                    source: value,
                    fields: vec![
                        ReadViewFieldInitializer {
                            field: field.field,
                            value: first_field,
                        },
                        ReadViewFieldInitializer {
                            field: FieldId::migrate(SEED, 1),
                            value: call,
                        },
                    ],
                },
                1 => ExpressionOperation::BorrowReadView {
                    value,
                    binding,
                    body: call,
                },
                _ => ExpressionOperation::BorrowReadViewField {
                    value,
                    field,
                    binding,
                    body: call,
                },
            });
            let condition = read.expression(ExpressionOperation::Bool { value: true });
            let body = read.expression(ExpressionOperation::If {
                condition,
                when_true: first_field,
                when_false: view_syntax,
            });
            read.body(body);
            assert_eq!(
                read.admit().expect_err("inactive syntax is complete").code,
                "kernel_callable_expansion"
            );
        }
    }
}
