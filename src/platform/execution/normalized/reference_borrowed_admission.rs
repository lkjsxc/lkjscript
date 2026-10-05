//! Independent canonical-source admission for source-tied read results.
//! Invocation permission belongs to one traversal occurrence, never an expression ID.

use super::{Bindings, Closure, allocate, index_node};
use crate::platform::execution::{ExecutionError, ExecutionFailureClass};
use crate::platform::kernel::{
    BindingKind, DeclarationPayload, DeclarationReference, ExpressionOperation, FunctionEffect,
    LocalValueReference, OwnerKey, OwnerRecord, PackageId, ParameterParent, ParameterUse, TypeForm,
    TypeObjectDigest,
};
use crate::platform::semantic_id::{BindingId, ExpressionId};
use std::collections::BTreeMap;

fn reject() -> ExecutionError {
    ExecutionError::new(
        ExecutionFailureClass::Infrastructure,
        "reference_borrowed_result_contract",
        "canonical read result violates its exact source, lexical custody, or callable contract",
    )
}

#[derive(Clone, Copy)]
enum Demand {
    Data,
    Owner,
    Read {
        root: LocalValueReference,
        ty: TypeObjectDigest,
    },
}

#[derive(Clone, Copy, Eq, PartialEq)]
struct Local {
    ty: TypeObjectDigest,
    live: bool,
    owning: bool,
    root: LocalValueReference,
    parent: Option<LocalValueReference>,
    guards: usize,
}

type Locals = BTreeMap<LocalValueReference, Local>;

struct Signature {
    parameters: Vec<(TypeObjectDigest, ParameterUse)>,
    result: TypeObjectDigest,
    source: Option<usize>,
}

struct Loan {
    source: LocalValueReference,
    binding: BindingId,
    ty: TypeObjectDigest,
}

pub(super) fn inventory(closure: &mut Closure<'_>) -> Result<(), ExecutionError> {
    // Check the complete metadata inventory. An unused implementation can still
    // counterfeit a result relationship that a later consumer would trust.
    allocate::<&crate::platform::kernel::KernelSnapshot>(
        &mut closure.allocated,
        closure.snapshots.len(),
    )?;
    let snapshots: Vec<_> = closure.snapshots.values().copied().collect();
    for snapshot in snapshots {
        for (key, owner) in &snapshot.owners {
            closure.tick()?;
            let (OwnerKey::Declaration(id), OwnerRecord::Declaration(owner)) = (key, owner) else {
                continue;
            };
            let reference = DeclarationReference {
                package: snapshot.root.package_id,
                declaration: *id,
            };
            match &owner.payload {
                DeclarationPayload::OwnedContract(contract) => {
                    for method in &contract.methods {
                        closure.tick()?;
                        if let Some(position) = method.result_borrow {
                            let parameter = method
                                .parameters
                                .get(position as usize)
                                .ok_or_else(reject)?;
                            if method.effect != FunctionEffect::Pure
                                || parameter.use_mode != ParameterUse::Borrow
                                || !closure.scoped_owned(parameter.ty, Some(reference))?
                                || !closure.scoped_owned(method.result, Some(reference))?
                            {
                                return Err(reject());
                            }
                        }
                    }
                }
                DeclarationPayload::OwnedImplementation(implementation) => {
                    let OwnerRecord::Declaration(owner) = closure.owner(
                        implementation.contract.package,
                        OwnerKey::Declaration(implementation.contract.declaration),
                    )?
                    else {
                        return Err(reject());
                    };
                    let DeclarationPayload::OwnedContract(contract) = &owner.payload else {
                        return Err(reject());
                    };
                    if contract.type_parameters.len() != implementation.type_arguments.len()
                        || contract.methods.len() != implementation.methods.len()
                    {
                        return Err(reject());
                    }
                    let mut types = Bindings::new();
                    index_node::<(
                        crate::platform::semantic_id::TypeParameterId,
                        TypeObjectDigest,
                    )>(&mut closure.allocated)?;
                    types.insert(contract.self_parameter, implementation.self_type);
                    for (parameter, ty) in contract
                        .type_parameters
                        .iter()
                        .zip(&implementation.type_arguments)
                    {
                        index_node::<(
                            crate::platform::semantic_id::TypeParameterId,
                            TypeObjectDigest,
                        )>(&mut closure.allocated)?;
                        if types.insert(*parameter, *ty).is_some() {
                            return Err(reject());
                        }
                    }
                    for method in &contract.methods {
                        closure.tick()?;
                        let mut mappings = implementation
                            .methods
                            .iter()
                            .filter(|mapping| mapping.method == method.id);
                        let target = mappings.next().ok_or_else(reject)?.function;
                        if mappings.next().is_some() {
                            return Err(reject());
                        }
                        let OwnerRecord::Declaration(owner) = closure
                            .owner(target.package, OwnerKey::Declaration(target.declaration))?
                        else {
                            return Err(reject());
                        };
                        let DeclarationPayload::Function(function) = &owner.payload else {
                            return Err(reject());
                        };
                        if function.effect != method.effect
                            || !function.type_parameters.is_empty()
                            || !function.effect_parameters.is_empty()
                            || !function.requirement_parameters.is_empty()
                            || !function.implementation_parameters.is_empty()
                            || function.parameters.len() != method.parameters.len()
                            || function.result_borrow
                                != method.result_borrow.and_then(|position| {
                                    function.parameters.get(position as usize).copied()
                                })
                            || function.result != closure.identity(method.result, &types, 0)?
                        {
                            return Err(reject());
                        }
                        for (actual, expected) in function.parameters.iter().zip(&method.parameters)
                        {
                            let OwnerRecord::Parameter(actual) =
                                closure.owner(target.package, OwnerKey::Parameter(*actual))?
                            else {
                                return Err(reject());
                            };
                            if actual.parent != ParameterParent::Function(target.declaration)
                                || actual.use_mode != expected.use_mode
                                || actual.resource_requirement.is_some()
                                || actual.ty != closure.identity(expected.ty, &types, 0)?
                            {
                                return Err(reject());
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }
    Ok(())
}

pub(super) fn body(
    closure: &mut Closure<'_>,
    package: PackageId,
    root: ExpressionId,
    types: &Bindings,
    scope: Option<DeclarationReference>,
) -> Result<(), ExecutionError> {
    let mut checker = Checker {
        closure,
        package,
        types,
        scope,
    };
    let returns_read = if let Some(scope) = scope {
        let OwnerRecord::Declaration(owner) = checker
            .closure
            .owner(scope.package, OwnerKey::Declaration(scope.declaration))?
        else {
            return Err(reject());
        };
        matches!(&owner.payload, DeclarationPayload::Function(function) if function.result_borrow.is_some())
    } else {
        false
    };
    if !returns_read && !checker.mentions_read(root)? {
        return Ok(());
    }
    let mut locals = Locals::new();
    let demand = if let Some(scope) = scope {
        let OwnerRecord::Declaration(owner) = checker
            .closure
            .owner(scope.package, OwnerKey::Declaration(scope.declaration))?
        else {
            return Err(reject());
        };
        let DeclarationPayload::Function(function) = &owner.payload else {
            return Err(reject());
        };
        let result = checker.closure.identity(function.result, types, 0)?;
        for id in &function.parameters {
            let OwnerRecord::Parameter(parameter) = checker
                .closure
                .owner(scope.package, OwnerKey::Parameter(*id))?
            else {
                return Err(reject());
            };
            if parameter.parent != ParameterParent::Function(scope.declaration)
                || (function.result_borrow == Some(*id) && parameter.resource_requirement.is_some())
            {
                return Err(reject());
            }
            let ty = checker.closure.identity(parameter.ty, types, 0)?;
            if checker.owned(ty)? {
                if parameter.use_mode == ParameterUse::Unrestricted {
                    return Err(reject());
                }
                let key = LocalValueReference::FunctionParameter(*id);
                checker.insert(
                    &mut locals,
                    key,
                    Local {
                        ty,
                        live: true,
                        owning: parameter.use_mode == ParameterUse::Consume,
                        root: key,
                        parent: None,
                        guards: 0,
                    },
                )?;
            }
        }
        if let Some(source) = function.result_borrow {
            let key = LocalValueReference::FunctionParameter(source);
            let source_local = locals.get(&key).ok_or_else(reject)?;
            if function.effect != FunctionEffect::Pure
                || !function.parameters.contains(&source)
                || source_local.owning
                || !checker.owned(result)?
            {
                return Err(reject());
            }
            Demand::Read {
                root: key,
                ty: result,
            }
        } else if checker.owned(result)? {
            Demand::Owner
        } else {
            Demand::Data
        }
    } else {
        // Test declarations can compare owned values; existing admission owns
        // their result mode, while this pass checks every nested invocation.
        Demand::Owner
    };
    checker.flow(root, &mut locals, demand, 0)?;
    Ok(())
}

struct Checker<'a, 'b> {
    closure: &'a mut Closure<'b>,
    package: PackageId,
    types: &'a Bindings,
    scope: Option<DeclarationReference>,
}

impl Checker<'_, '_> {
    fn mentions_read(&mut self, root: ExpressionId) -> Result<bool, ExecutionError> {
        allocate::<ExpressionId>(&mut self.closure.allocated, 1)?;
        let mut pending = vec![root];
        while let Some(id) = pending.pop() {
            self.closure.tick()?;
            let operation = self.operation(id)?.clone();
            match &operation {
                ExpressionOperation::BorrowCall { .. } => return Ok(true),
                ExpressionOperation::Call { function, .. }
                | ExpressionOperation::ImplementationCall { function, .. }
                | ExpressionOperation::FunctionValue { function, .. } => {
                    let OwnerRecord::Declaration(owner) = self.closure.owner(
                        function.package,
                        OwnerKey::Declaration(function.declaration),
                    )?
                    else {
                        return Err(reject());
                    };
                    if matches!(&owner.payload, DeclarationPayload::Function(function) if function.result_borrow.is_some())
                    {
                        return Ok(true);
                    }
                }
                ExpressionOperation::MethodCall {
                    contract, method, ..
                } => {
                    let OwnerRecord::Declaration(owner) = self.closure.owner(
                        contract.package,
                        OwnerKey::Declaration(contract.declaration),
                    )?
                    else {
                        return Err(reject());
                    };
                    let DeclarationPayload::OwnedContract(contract) = &owner.payload else {
                        return Err(reject());
                    };
                    if contract
                        .methods
                        .iter()
                        .find(|candidate| candidate.id == *method)
                        .ok_or_else(reject)?
                        .result_borrow
                        .is_some()
                    {
                        return Ok(true);
                    }
                }
                ExpressionOperation::Let { bindings, .. } => {
                    for binding in bindings {
                        let OwnerRecord::Binding(binding) = self
                            .closure
                            .owner(self.package, OwnerKey::Binding(*binding))?
                        else {
                            return Err(reject());
                        };
                        allocate::<ExpressionId>(&mut self.closure.allocated, 1)?;
                        pending.push(binding.value.ok_or_else(reject)?);
                    }
                }
                _ => {}
            }
            let record = crate::platform::kernel::ExpressionRecord {
                contract_version: crate::platform::kernel::contract::GRAPH_CONTRACT_VERSION,
                id,
                operation,
            };
            let children = self.children(&record)?;
            allocate::<ExpressionId>(&mut self.closure.allocated, children.len())?;
            pending.extend(children.into_iter().map(|child| child.expression));
        }
        Ok(false)
    }

    fn owned(&mut self, ty: TypeObjectDigest) -> Result<bool, ExecutionError> {
        self.closure.scoped_owned(ty, self.scope)
    }

    fn children(
        &mut self,
        record: &crate::platform::kernel::ExpressionRecord,
    ) -> Result<Vec<crate::platform::kernel::ExpressionChild>, ExecutionError> {
        let count = match &record.operation {
            ExpressionOperation::Unit { .. }
            | ExpressionOperation::Bool { .. }
            | ExpressionOperation::I64 { .. }
            | ExpressionOperation::F64 { .. }
            | ExpressionOperation::Text { .. }
            | ExpressionOperation::StaticText { .. }
            | ExpressionOperation::Local { .. }
            | ExpressionOperation::Constant { .. }
            | ExpressionOperation::FunctionValue { .. }
            | ExpressionOperation::SequenceEmpty { .. } => 0,
            ExpressionOperation::Let { .. }
            | ExpressionOperation::Field { .. }
            | ExpressionOperation::Transaction { .. }
            | ExpressionOperation::TransactionOutcome { .. }
            | ExpressionOperation::ChooseOwned { .. }
            | ExpressionOperation::SequenceLength { .. }
            | ExpressionOperation::SequencePop { .. } => 1,
            ExpressionOperation::BorrowCall { .. }
            | ExpressionOperation::UnpackOwned { .. }
            | ExpressionOperation::BorrowOwnedField { .. }
            | ExpressionOperation::Parallel { .. }
            | ExpressionOperation::SequencePush { .. } => 2,
            ExpressionOperation::If { .. } | ExpressionOperation::BorrowOwnedItem { .. } => 3,
            ExpressionOperation::Sequence { items } | ExpressionOperation::List { items, .. } => {
                items.len()
            }
            ExpressionOperation::Call { arguments, .. }
            | ExpressionOperation::ImplementationCall { arguments, .. }
            | ExpressionOperation::MethodCall { arguments, .. }
            | ExpressionOperation::CapabilityCall { arguments, .. } => arguments.len(),
            ExpressionOperation::Invoke { arguments, .. }
            | ExpressionOperation::Bind { arguments, .. } => {
                arguments.len().checked_add(1).ok_or_else(reject)?
            }
            ExpressionOperation::Record { fields, .. } => {
                // Canonical record child discovery also retains an intermediate
                // ordered value-ID vector; reserve it before the factory runs.
                allocate::<ExpressionId>(&mut self.closure.allocated, fields.len())?;
                fields.len()
            }
            ExpressionOperation::PackOwned { fields, .. } => fields.len(),
            ExpressionOperation::Variant { payload, .. } => usize::from(payload.is_some()),
            ExpressionOperation::Map { entries, .. } => {
                entries.len().checked_mul(2).ok_or_else(reject)?
            }
            ExpressionOperation::Match { arms, .. } => {
                arms.len().checked_add(1).ok_or_else(reject)?
            }
            ExpressionOperation::MatchOwned { arms, .. }
            | ExpressionOperation::MatchBorrowedOwned { arms, .. } => {
                arms.len().checked_add(1).ok_or_else(reject)?
            }
        };
        allocate::<crate::platform::kernel::ExpressionChild>(&mut self.closure.allocated, count)?;
        Ok(record.children())
    }

    fn operation(&mut self, id: ExpressionId) -> Result<&ExpressionOperation, ExecutionError> {
        let OwnerRecord::Expression(owner) =
            self.closure.owner(self.package, OwnerKey::Expression(id))?
        else {
            return Err(reject());
        };
        Ok(&owner.operation)
    }

    fn insert(
        &mut self,
        locals: &mut Locals,
        key: LocalValueReference,
        value: Local,
    ) -> Result<(), ExecutionError> {
        index_node::<(LocalValueReference, Local)>(&mut self.closure.allocated)?;
        if locals.insert(key, value).is_some() {
            return Err(reject());
        }
        Ok(())
    }

    fn branch(&mut self, locals: &Locals) -> Result<Locals, ExecutionError> {
        allocate::<(LocalValueReference, Local)>(&mut self.closure.allocated, locals.len())?;
        allocate::<usize>(&mut self.closure.allocated, locals.len().saturating_mul(3))?;
        Ok(locals.clone())
    }

    fn join(&mut self, left: &mut Locals, right: &Locals) -> Result<(), ExecutionError> {
        if left.len() != right.len() {
            return Err(reject());
        }
        for (key, local) in left {
            self.closure.tick()?;
            let other = right.get(key).ok_or_else(reject)?;
            if local.ty != other.ty
                || local.owning != other.owning
                || local.root != other.root
                || local.parent != other.parent
                || local.guards != other.guards
                || (!local.owning && local.live != other.live)
            {
                return Err(reject());
            }
            local.live &= other.live;
        }
        Ok(())
    }

    fn local(
        &mut self,
        expression: ExpressionId,
        locals: &Locals,
        ty: TypeObjectDigest,
    ) -> Result<LocalValueReference, ExecutionError> {
        let ExpressionOperation::Local { value } = self.operation(expression)? else {
            return Err(reject());
        };
        let value = *value;
        let local = locals.get(&value).ok_or_else(reject)?;
        if !local.live || local.ty != ty {
            return Err(reject());
        }
        Ok(value)
    }

    fn protect(
        &mut self,
        locals: &mut Locals,
        mut source: LocalValueReference,
    ) -> Result<Vec<LocalValueReference>, ExecutionError> {
        let mut ancestors = Vec::new();
        loop {
            self.closure.tick()?;
            if ancestors.len() > crate::platform::kernel::contract::MAXIMUM_EXPRESSION_DEPTH {
                return Err(reject());
            }
            let local = locals.get_mut(&source).ok_or_else(reject)?;
            if !local.live {
                return Err(reject());
            }
            local.guards = local.guards.checked_add(1).ok_or_else(reject)?;
            allocate::<LocalValueReference>(&mut self.closure.allocated, 1)?;
            ancestors.push(source);
            match local.parent {
                Some(parent) => source = parent,
                None => break,
            }
        }
        Ok(ancestors)
    }

    fn release(
        &mut self,
        locals: &mut Locals,
        ancestors: Vec<LocalValueReference>,
    ) -> Result<(), ExecutionError> {
        for key in ancestors {
            self.closure.tick()?;
            let local = locals.get_mut(&key).ok_or_else(reject)?;
            local.guards = local.guards.checked_sub(1).ok_or_else(reject)?;
        }
        Ok(())
    }

    fn bind(
        &mut self,
        id: BindingId,
        expected_kind: BindingKind,
        expected: Option<TypeObjectDigest>,
        locals: &mut Locals,
        source: Option<LocalValueReference>,
    ) -> Result<(), ExecutionError> {
        let OwnerRecord::Binding(binding) =
            self.closure.owner(self.package, OwnerKey::Binding(id))?
        else {
            return Err(reject());
        };
        if binding.kind != expected_kind
            || (expected_kind != BindingKind::Let && binding.value.is_some())
        {
            return Err(reject());
        }
        let declared = binding
            .declared_type
            .map(|ty| self.closure.identity(ty, self.types, 0))
            .transpose()?;
        if expected_kind == BindingKind::OwnedBorrow && declared != expected {
            return Err(reject());
        }
        if expected_kind == BindingKind::Let
            && expected.is_none()
            && declared
                .map(|ty| self.owned(ty))
                .transpose()?
                .unwrap_or(false)
        {
            return Err(reject());
        }
        if let (Some(expected), Some(declared)) = (expected, declared)
            && expected != declared
        {
            return Err(reject());
        }
        if let Some(ty) = expected.or(declared) {
            if self.owned(ty)? {
                let key = LocalValueReference::LexicalBinding(id);
                let root = source
                    .map(|source| {
                        locals
                            .get(&source)
                            .map(|local| local.root)
                            .ok_or_else(reject)
                    })
                    .transpose()?
                    .unwrap_or(key);
                self.insert(
                    locals,
                    key,
                    Local {
                        ty,
                        live: true,
                        owning: source.is_none(),
                        root,
                        parent: source,
                        guards: 0,
                    },
                )?;
            } else if source.is_some() && expected_kind != BindingKind::OwnedBorrow {
                return Err(reject());
            }
        } else if source.is_some() {
            return Err(reject());
        }
        Ok(())
    }

    fn unbind(&mut self, id: BindingId, locals: &mut Locals) -> Result<(), ExecutionError> {
        if let Some(local) = locals.remove(&LocalValueReference::LexicalBinding(id))
            && local.guards != 0
        {
            return Err(reject());
        }
        Ok(())
    }

    fn scoped(
        &mut self,
        loan: Loan,
        body: ExpressionId,
        locals: &mut Locals,
        demand: Demand,
        depth: usize,
    ) -> Result<Option<TypeObjectDigest>, ExecutionError> {
        let Loan {
            source,
            binding,
            ty,
        } = loan;
        let ancestors = self.protect(locals, source)?;
        self.bind(
            binding,
            BindingKind::OwnedBorrow,
            Some(ty),
            locals,
            Some(source),
        )?;
        let result = self.flow(body, locals, demand, depth + 1)?;
        self.unbind(binding, locals)?;
        self.release(locals, ancestors)?;
        Ok(result)
    }

    fn signature(&mut self, call: &ExpressionOperation) -> Result<Signature, ExecutionError> {
        match call {
            ExpressionOperation::Call {
                function,
                type_arguments,
                ..
            }
            | ExpressionOperation::ImplementationCall {
                function,
                type_arguments,
                ..
            }
            | ExpressionOperation::FunctionValue {
                function,
                type_arguments,
                ..
            } => {
                let OwnerRecord::Declaration(owner) = self.closure.owner(
                    function.package,
                    OwnerKey::Declaration(function.declaration),
                )?
                else {
                    return Err(reject());
                };
                let (parameters, formal_types, result, source) = match &owner.payload {
                    DeclarationPayload::Function(signature) => (
                        &signature.parameters,
                        &signature.type_parameters,
                        signature.result,
                        signature.result_borrow,
                    ),
                    DeclarationPayload::External(signature) => (
                        &signature.parameters,
                        &signature.type_parameters,
                        signature.result,
                        None,
                    ),
                    _ => return Err(reject()),
                };
                if formal_types.len() != type_arguments.len() {
                    return Err(reject());
                }
                let mut substitutions = Bindings::new();
                for (formal, actual) in formal_types.iter().zip(type_arguments) {
                    let actual = self.closure.identity(*actual, self.types, 0)?;
                    index_node::<(
                        crate::platform::semantic_id::TypeParameterId,
                        TypeObjectDigest,
                    )>(&mut self.closure.allocated)?;
                    if substitutions.insert(*formal, actual).is_some() {
                        return Err(reject());
                    }
                }
                let source = source
                    .map(|source| {
                        parameters
                            .iter()
                            .position(|id| *id == source)
                            .ok_or_else(reject)
                    })
                    .transpose()?;
                let mut instantiated = Vec::new();
                allocate::<(TypeObjectDigest, ParameterUse)>(
                    &mut self.closure.allocated,
                    parameters.len(),
                )?;
                for parameter in parameters {
                    let OwnerRecord::Parameter(parameter) = self
                        .closure
                        .owner(function.package, OwnerKey::Parameter(*parameter))?
                    else {
                        return Err(reject());
                    };
                    instantiated.push((
                        self.closure.identity(parameter.ty, &substitutions, 0)?,
                        parameter.use_mode,
                    ));
                }
                Ok(Signature {
                    parameters: instantiated,
                    result: self.closure.identity(result, &substitutions, 0)?,
                    source,
                })
            }
            ExpressionOperation::MethodCall {
                witness,
                contract,
                method,
                ..
            } => {
                let (selected, self_type, arguments) = self
                    .closure
                    .witness_application(*witness, self.scope, self.types)?;
                if selected != *contract {
                    return Err(reject());
                }
                let OwnerRecord::Declaration(owner) = self.closure.owner(
                    contract.package,
                    OwnerKey::Declaration(contract.declaration),
                )?
                else {
                    return Err(reject());
                };
                let DeclarationPayload::OwnedContract(contract) = &owner.payload else {
                    return Err(reject());
                };
                if contract.type_parameters.len() != arguments.len() {
                    return Err(reject());
                }
                let mut substitutions = Bindings::new();
                index_node::<(
                    crate::platform::semantic_id::TypeParameterId,
                    TypeObjectDigest,
                )>(&mut self.closure.allocated)?;
                substitutions.insert(contract.self_parameter, self_type);
                for (formal, actual) in contract.type_parameters.iter().zip(arguments) {
                    index_node::<(
                        crate::platform::semantic_id::TypeParameterId,
                        TypeObjectDigest,
                    )>(&mut self.closure.allocated)?;
                    if substitutions.insert(*formal, actual).is_some() {
                        return Err(reject());
                    }
                }
                let method = contract
                    .methods
                    .iter()
                    .find(|candidate| candidate.id == *method)
                    .ok_or_else(reject)?;
                allocate::<(TypeObjectDigest, ParameterUse)>(
                    &mut self.closure.allocated,
                    method.parameters.len(),
                )?;
                let parameters = method
                    .parameters
                    .iter()
                    .map(|parameter| {
                        Ok((
                            self.closure.identity(parameter.ty, &substitutions, 0)?,
                            parameter.use_mode,
                        ))
                    })
                    .collect::<Result<_, ExecutionError>>()?;
                Ok(Signature {
                    parameters,
                    result: self.closure.identity(method.result, &substitutions, 0)?,
                    source: method.result_borrow.map(|position| position as usize),
                })
            }
            _ => Err(reject()),
        }
    }

    fn arguments(
        &mut self,
        signature: &Signature,
        arguments: &[ExpressionId],
        locals: &mut Locals,
        depth: usize,
    ) -> Result<Option<LocalValueReference>, ExecutionError> {
        if signature.parameters.len() != arguments.len() {
            return Err(reject());
        }
        let mut retained = Vec::new();
        let mut selected = None;
        for (position, ((ty, mode), argument)) in
            signature.parameters.iter().zip(arguments).enumerate()
        {
            self.closure.tick()?;
            if self.owned(*ty)? {
                if *mode == ParameterUse::Borrow {
                    let local = self.local(*argument, locals, *ty)?;
                    let guards = self.protect(locals, local)?;
                    allocate::<Vec<LocalValueReference>>(&mut self.closure.allocated, 1)?;
                    retained.push(guards);
                    if signature.source == Some(position) {
                        selected = Some(local);
                    }
                } else if *mode == ParameterUse::Consume {
                    let actual = self.flow(*argument, locals, Demand::Owner, depth + 1)?;
                    if actual != Some(*ty) {
                        return Err(reject());
                    }
                } else {
                    return Err(reject());
                }
            } else {
                self.flow(*argument, locals, Demand::Data, depth + 1)?;
            }
        }
        for guards in retained.into_iter().rev() {
            self.release(locals, guards)?;
        }
        if signature.source.is_some() && selected.is_none() {
            return Err(reject());
        }
        Ok(selected)
    }

    fn flow(
        &mut self,
        id: ExpressionId,
        locals: &mut Locals,
        demand: Demand,
        depth: usize,
    ) -> Result<Option<TypeObjectDigest>, ExecutionError> {
        self.closure.tick()?;
        if depth > crate::platform::kernel::contract::MAXIMUM_EXPRESSION_DEPTH {
            return Err(reject());
        }
        let operation = self.operation(id)?.clone();
        if matches!(demand, Demand::Read { .. })
            && !matches!(
                operation,
                ExpressionOperation::Local { .. }
                    | ExpressionOperation::If { .. }
                    | ExpressionOperation::Let { .. }
                    | ExpressionOperation::Sequence { .. }
                    | ExpressionOperation::Match { .. }
                    | ExpressionOperation::MatchOwned { .. }
                    | ExpressionOperation::UnpackOwned { .. }
                    | ExpressionOperation::BorrowOwnedField { .. }
                    | ExpressionOperation::BorrowOwnedItem { .. }
                    | ExpressionOperation::MatchBorrowedOwned { .. }
                    | ExpressionOperation::BorrowCall { .. }
            )
        {
            return Err(reject());
        }
        let result = match operation {
            ExpressionOperation::Local { value } => {
                if let Some(local) = locals.get_mut(&value) {
                    if !local.live {
                        return Err(reject());
                    }
                    match demand {
                        Demand::Data => return Err(reject()),
                        Demand::Owner => {
                            if !local.owning || local.guards != 0 {
                                return Err(reject());
                            }
                            local.live = false;
                        }
                        Demand::Read { root, ty } => {
                            if local.owning || local.root != root || local.ty != ty {
                                return Err(reject());
                            }
                        }
                    }
                    Some(local.ty)
                } else {
                    if matches!(demand, Demand::Read { .. }) {
                        return Err(reject());
                    }
                    None
                }
            }
            ExpressionOperation::If {
                condition,
                when_true,
                when_false,
            } => {
                self.flow(condition, locals, Demand::Data, depth + 1)?;
                let mut left = self.branch(locals)?;
                let mut right = self.branch(locals)?;
                let first = self.flow(when_true, &mut left, demand, depth + 1)?;
                let second = self.flow(when_false, &mut right, demand, depth + 1)?;
                if first != second {
                    return Err(reject());
                }
                self.join(&mut left, &right)?;
                *locals = left;
                first
            }
            ExpressionOperation::Sequence { items } => {
                if let Some((last, preceding)) = items.split_last() {
                    for item in preceding {
                        self.flow(*item, locals, Demand::Data, depth + 1)?;
                    }
                    self.flow(*last, locals, demand, depth + 1)?
                } else {
                    if matches!(demand, Demand::Read { .. }) {
                        return Err(reject());
                    }
                    None
                }
            }
            ExpressionOperation::Let { bindings, body } => {
                for binding in &bindings {
                    let OwnerRecord::Binding(owner) = self
                        .closure
                        .owner(self.package, OwnerKey::Binding(*binding))?
                    else {
                        return Err(reject());
                    };
                    let value = owner.value.ok_or_else(reject)?;
                    let actual = self.flow(value, locals, Demand::Owner, depth + 1)?;
                    self.bind(*binding, BindingKind::Let, actual, locals, None)?;
                }
                let result = self.flow(body, locals, demand, depth + 1)?;
                for binding in bindings.into_iter().rev() {
                    self.unbind(binding, locals)?;
                }
                result
            }
            ExpressionOperation::BorrowCall {
                call,
                binding,
                body,
            } => {
                let invocation = self.operation(call)?.clone();
                let signature = self.signature(&invocation)?;
                if signature.source.is_none() || !self.owned(signature.result)? {
                    return Err(reject());
                }
                let arguments = match invocation {
                    ExpressionOperation::Call { arguments, .. }
                    | ExpressionOperation::ImplementationCall { arguments, .. }
                    | ExpressionOperation::MethodCall { arguments, .. } => arguments,
                    _ => return Err(reject()),
                };
                let source = self
                    .arguments(&signature, &arguments, locals, depth + 1)?
                    .ok_or_else(reject)?;
                self.scoped(
                    Loan {
                        source,
                        binding,
                        ty: signature.result,
                    },
                    body,
                    locals,
                    demand,
                    depth,
                )?
            }
            ExpressionOperation::BorrowOwnedField {
                product_type,
                source,
                field,
                binding,
                body,
            } => {
                let product = self.closure.identity(product_type, self.types, 0)?;
                let TypeForm::OwnedProduct { fields } =
                    &self.closure.types.get(&product).ok_or_else(reject)?.form
                else {
                    return Err(reject());
                };
                let ty = fields
                    .iter()
                    .find(|candidate| candidate.name == field)
                    .ok_or_else(reject)?
                    .ty;
                if !self.owned(ty)? {
                    return Err(reject());
                }
                let source = self.local(source, locals, product)?;
                self.scoped(
                    Loan {
                        source,
                        binding,
                        ty,
                    },
                    body,
                    locals,
                    demand,
                    depth,
                )?
            }
            ExpressionOperation::BorrowOwnedItem {
                sequence_type,
                source,
                index,
                binding,
                body,
            } => {
                self.flow(index, locals, Demand::Data, depth + 1)?;
                let sequence = self.closure.identity(sequence_type, self.types, 0)?;
                let TypeForm::OwnedSequence { item } =
                    self.closure.types.get(&sequence).ok_or_else(reject)?.form
                else {
                    return Err(reject());
                };
                let source = self.local(source, locals, sequence)?;
                self.scoped(
                    Loan {
                        source,
                        binding,
                        ty: item,
                    },
                    body,
                    locals,
                    demand,
                    depth,
                )?
            }
            ExpressionOperation::MatchBorrowedOwned {
                choice_type,
                source,
                arms,
            } => {
                let choice = self.closure.identity(choice_type, self.types, 0)?;
                let TypeForm::OwnedChoice { cases } = self
                    .closure
                    .types
                    .get(&choice)
                    .ok_or_else(reject)?
                    .form
                    .clone()
                else {
                    return Err(reject());
                };
                let source = self.local(source, locals, choice)?;
                let mut joined: Option<(Locals, Option<TypeObjectDigest>)> = None;
                for arm in arms {
                    let mut branch = self.branch(locals)?;
                    let ty = cases
                        .iter()
                        .find(|case| case.name == arm.name)
                        .ok_or_else(reject)?
                        .ty;
                    let result = self.scoped(
                        Loan {
                            source,
                            binding: arm.binding,
                            ty,
                        },
                        arm.body,
                        &mut branch,
                        demand,
                        depth,
                    )?;
                    if let Some((joined, previous)) = &mut joined {
                        if *previous != result {
                            return Err(reject());
                        }
                        self.join(joined, &branch)?;
                    } else {
                        joined = Some((branch, result));
                    }
                }
                let (joined, result) = joined.ok_or_else(reject)?;
                *locals = joined;
                result
            }
            ExpressionOperation::UnpackOwned {
                product_type,
                source,
                fields,
                body,
            } => {
                let product = self.closure.identity(product_type, self.types, 0)?;
                if self.flow(source, locals, Demand::Owner, depth + 1)? != Some(product) {
                    return Err(reject());
                }
                let TypeForm::OwnedProduct { fields: types } = self
                    .closure
                    .types
                    .get(&product)
                    .ok_or_else(reject)?
                    .form
                    .clone()
                else {
                    return Err(reject());
                };
                for field in &fields {
                    let ty = types
                        .iter()
                        .find(|candidate| candidate.name == field.name)
                        .ok_or_else(reject)?
                        .ty;
                    self.bind(
                        field.binding,
                        BindingKind::OwnedUnpack,
                        Some(ty),
                        locals,
                        None,
                    )?;
                }
                let result = self.flow(body, locals, demand, depth + 1)?;
                for field in fields {
                    self.unbind(field.binding, locals)?;
                }
                result
            }
            ExpressionOperation::MatchOwned {
                choice_type,
                source,
                arms,
            } => {
                let choice = self.closure.identity(choice_type, self.types, 0)?;
                if self.flow(source, locals, Demand::Owner, depth + 1)? != Some(choice) {
                    return Err(reject());
                }
                let TypeForm::OwnedChoice { cases } = self
                    .closure
                    .types
                    .get(&choice)
                    .ok_or_else(reject)?
                    .form
                    .clone()
                else {
                    return Err(reject());
                };
                let mut joined: Option<(Locals, Option<TypeObjectDigest>)> = None;
                for arm in arms {
                    let mut branch = self.branch(locals)?;
                    let ty = cases
                        .iter()
                        .find(|case| case.name == arm.name)
                        .ok_or_else(reject)?
                        .ty;
                    self.bind(
                        arm.binding,
                        BindingKind::OwnedChoicePayload,
                        Some(ty),
                        &mut branch,
                        None,
                    )?;
                    let result = self.flow(arm.body, &mut branch, demand, depth + 1)?;
                    self.unbind(arm.binding, &mut branch)?;
                    if let Some((joined, previous)) = &mut joined {
                        if *previous != result {
                            return Err(reject());
                        }
                        self.join(joined, &branch)?;
                    } else {
                        joined = Some((branch, result));
                    }
                }
                let (joined, result) = joined.ok_or_else(reject)?;
                *locals = joined;
                result
            }
            ExpressionOperation::Match { value, arms } => {
                self.flow(value, locals, Demand::Data, depth + 1)?;
                let mut joined: Option<(Locals, Option<TypeObjectDigest>)> = None;
                for arm in arms {
                    let mut branch = self.branch(locals)?;
                    let result = self.flow(arm.body, &mut branch, demand, depth + 1)?;
                    if let Some((joined, previous)) = &mut joined {
                        if *previous != result {
                            return Err(reject());
                        }
                        self.join(joined, &branch)?;
                    } else {
                        joined = Some((branch, result));
                    }
                }
                let (joined, result) = joined.ok_or_else(reject)?;
                *locals = joined;
                result
            }
            ref call @ (ExpressionOperation::Call { .. }
            | ExpressionOperation::ImplementationCall { .. }
            | ExpressionOperation::MethodCall { .. }) => {
                let signature = self.signature(call)?;
                if signature.source.is_some() {
                    return Err(reject());
                }
                let arguments = match call {
                    ExpressionOperation::Call { arguments, .. }
                    | ExpressionOperation::ImplementationCall { arguments, .. }
                    | ExpressionOperation::MethodCall { arguments, .. } => arguments,
                    _ => unreachable!(),
                };
                self.arguments(&signature, arguments, locals, depth)?;
                self.owned(signature.result)?.then_some(signature.result)
            }
            ref call @ ExpressionOperation::FunctionValue { .. } => {
                if self.signature(call)?.source.is_some() {
                    return Err(reject());
                }
                None
            }
            ExpressionOperation::SequenceEmpty { sequence_type } => {
                Some(self.closure.identity(sequence_type, self.types, 0)?)
            }
            ExpressionOperation::SequenceLength {
                sequence_type,
                source,
            } => {
                let ty = self.closure.identity(sequence_type, self.types, 0)?;
                self.local(source, locals, ty)?;
                None
            }
            ExpressionOperation::SequencePush {
                sequence_type,
                value,
                source,
            } => {
                let sequence = self.closure.identity(sequence_type, self.types, 0)?;
                let TypeForm::OwnedSequence { item } =
                    self.closure.types.get(&sequence).ok_or_else(reject)?.form
                else {
                    return Err(reject());
                };
                if self.flow(value, locals, Demand::Owner, depth + 1)? != Some(item)
                    || self.flow(source, locals, Demand::Owner, depth + 1)? != Some(sequence)
                {
                    return Err(reject());
                }
                Some(sequence)
            }
            ExpressionOperation::SequencePop {
                sequence_type,
                result_type,
                source,
            } => {
                let sequence = self.closure.identity(sequence_type, self.types, 0)?;
                if self.flow(source, locals, Demand::Owner, depth + 1)? != Some(sequence) {
                    return Err(reject());
                }
                Some(self.closure.identity(result_type, self.types, 0)?)
            }
            ExpressionOperation::PackOwned {
                product_type,
                fields,
            } => {
                for field in fields {
                    self.flow(field.value, locals, Demand::Owner, depth + 1)?;
                }
                Some(self.closure.identity(product_type, self.types, 0)?)
            }
            ExpressionOperation::ChooseOwned {
                choice_type, value, ..
            } => {
                self.flow(value, locals, Demand::Owner, depth + 1)?;
                Some(self.closure.identity(choice_type, self.types, 0)?)
            }
            ExpressionOperation::Parallel { left, right } => {
                self.flow(left, locals, Demand::Owner, depth + 1)?;
                self.flow(right, locals, Demand::Owner, depth + 1)?;
                let left = self.operation(left)?.clone();
                let right = self.operation(right)?.clone();
                let left = self.signature(&left)?.result;
                let right = self.signature(&right)?.result;
                let owned = self.owned(left)? || self.owned(right)?;
                allocate::<crate::platform::kernel::StructuralTypeField>(
                    &mut self.closure.allocated,
                    2,
                )?;
                allocate::<u8>(&mut self.closure.allocated, "left".len() + "right".len())?;
                let fields = [("left", left), ("right", right)]
                    .into_iter()
                    .map(|(name, ty)| {
                        Ok(crate::platform::kernel::StructuralTypeField {
                            name: crate::platform::kernel::Name::new(name).map_err(|_| reject())?,
                            ty,
                        })
                    })
                    .collect::<Result<Vec<_>, ExecutionError>>()?;
                let object = crate::platform::kernel::TypeObject::new(if owned {
                    TypeForm::OwnedProduct { fields }
                } else {
                    TypeForm::StructuralRecord { fields }
                })
                .map_err(|_| reject())?;
                let (ty, _) =
                    crate::platform::kernel::encode_type_object(&object).map_err(|_| reject())?;
                index_node::<(TypeObjectDigest, crate::platform::kernel::TypeObject)>(
                    &mut self.closure.allocated,
                )?;
                self.closure.types.entry(ty).or_insert(object);
                owned.then_some(ty)
            }
            ExpressionOperation::CapabilityCall {
                operation,
                arguments,
                ..
            } => {
                let package = operation.package;
                let OwnerRecord::Operation(operation) = self
                    .closure
                    .owner(operation.package, OwnerKey::Operation(operation.operation))?
                else {
                    return Err(reject());
                };
                if operation.parameters.len() != arguments.len() {
                    return Err(reject());
                }
                let mut parameters = Vec::new();
                allocate::<(TypeObjectDigest, ParameterUse)>(
                    &mut self.closure.allocated,
                    operation.parameters.len(),
                )?;
                for parameter in &operation.parameters {
                    let OwnerRecord::Parameter(parameter) = self
                        .closure
                        .owner(package, OwnerKey::Parameter(*parameter))?
                    else {
                        return Err(reject());
                    };
                    parameters.push((parameter.ty, parameter.use_mode));
                }
                self.arguments(
                    &Signature {
                        parameters,
                        result: operation.result,
                        source: None,
                    },
                    &arguments,
                    locals,
                    depth,
                )?;
                self.owned(operation.result)?.then_some(operation.result)
            }
            ExpressionOperation::Field { value, selector } => {
                let source = match self.operation(value)? {
                    ExpressionOperation::Local { value } => Some(*value),
                    _ => None,
                };
                if let Some(local) = source.and_then(|source| locals.get(&source)).copied() {
                    if !local.live {
                        return Err(reject());
                    }
                    let (
                        TypeForm::OwnedProduct { fields },
                        crate::platform::kernel::FieldSelector::Structural(name),
                    ) = (
                        &self.closure.types.get(&local.ty).ok_or_else(reject)?.form,
                        selector,
                    )
                    else {
                        return Err(reject());
                    };
                    let field = fields
                        .iter()
                        .find(|field| field.name == name)
                        .ok_or_else(reject)?
                        .ty;
                    if self.owned(field)? {
                        return Err(reject());
                    }
                } else {
                    self.flow(value, locals, Demand::Data, depth + 1)?;
                }
                None
            }
            ExpressionOperation::Transaction { body, .. }
            | ExpressionOperation::TransactionOutcome { body, .. } => {
                self.flow(body, locals, demand, depth + 1)?
            }
            other => {
                let record = crate::platform::kernel::ExpressionRecord {
                    contract_version: crate::platform::kernel::contract::GRAPH_CONTRACT_VERSION,
                    id,
                    operation: other,
                };
                for child in self.children(&record)? {
                    self.flow(child.expression, locals, Demand::Data, depth + 1)?;
                }
                None
            }
        };
        if matches!(demand, Demand::Data) && result.is_some() {
            return Err(reject());
        }
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::execution::ExecutionControl;
    use crate::platform::kernel::{FunctionDeclaration, KernelSnapshot};

    const SOURCE: &str = r#"declarations.begin
(units (module create independent-read-results
  (external create read (visibility public) (implementation core.cell.read)
    (parameter create value (type OwnedI64Cell) (use borrow)) (returns I64))
  (function create discard (visibility private) (effect pure)
    (parameter create value (type OwnedI64Cell) (use consume))
    (returns I64) (body (i64 0)))
  (function create first (visibility public) (effect pure)
    (parameter create a (type OwnedI64Cell) (use borrow))
    (parameter create b (type OwnedI64Cell) (use borrow))
    (returns OwnedI64Cell (borrow-from a)) (body (local a)))
  (function create branch (visibility public) (effect pure)
    (parameter create flag (type Bool))
    (parameter create a (type OwnedI64Cell) (use borrow))
    (parameter create b (type OwnedI64Cell) (use borrow))
    (returns OwnedI64Cell (borrow-from a))
    (body (if (local flag) (local a) (local a))))
  (function create project (visibility public) (effect pure)
    (parameter create pair (type (owned-product (field left OwnedI64Cell) (field right OwnedI64Cell))) (use borrow))
    (returns OwnedI64Cell (borrow-from pair))
    (body (borrow-owned-field (type (owned-product (field left OwnedI64Cell) (field right OwnedI64Cell)))
      (local pair) (field left (binding view (type OwnedI64Cell))) (in (local view)))))
  (function create relay (visibility public) (effect pure)
    (parameter create pair (type (owned-product (field left OwnedI64Cell) (field right OwnedI64Cell))) (use borrow))
    (returns OwnedI64Cell (borrow-from pair))
    (body (borrow-call (call project (local pair))
      (binding view (type OwnedI64Cell)) (in (local view)))))
  (function create caller (visibility public) (effect pure)
    (parameter create value (type OwnedI64Cell) (use consume))
    (returns OwnedI64Cell)
    (body (sequence
      (borrow-call (call first (local value) (local value))
        (binding view (type OwnedI64Cell)) (in (call read (local view))))
      (local value))))
  (owned-contract create Reader (visibility public)
    (self Self) (type-parameter create Self (constraint owned))
    (type-parameter create Item (constraint owned))
    (method method_a8000000000000000000000000000001 at
      (parameters (I64 unrestricted) (Self borrow))
      (returns Item (borrow-from 1))))
  (function create at (visibility public) (effect pure)
    (parameter create index (type I64))
    (parameter create pair (type (owned-product (field left OwnedI64Cell) (field right OwnedI64Cell))) (use borrow))
    (returns OwnedI64Cell (borrow-from pair))
    (body (borrow-call (call project (local pair))
      (binding view (type OwnedI64Cell)) (in (local view)))))
  (owned-implementation create PairReader (visibility public)
    (contract Reader) (self (owned-product (field left OwnedI64Cell) (field right OwnedI64Cell)))
    (types OwnedI64Cell) (method method_a8000000000000000000000000000001 at))))
declarations.end
"#;

    fn source() -> KernelSnapshot {
        super::super::super::tests::byte_buffer_tests::author_only(SOURCE).unwrap()
    }

    fn reference(snapshot: &KernelSnapshot, name: &str) -> DeclarationReference {
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

    fn function(snapshot: &KernelSnapshot, name: &str) -> FunctionDeclaration {
        let OwnerRecord::Declaration(owner) =
            &snapshot.owners[&OwnerKey::Declaration(reference(snapshot, name).declaration)]
        else {
            unreachable!();
        };
        let DeclarationPayload::Function(function) = &owner.payload else {
            unreachable!();
        };
        function.clone()
    }

    fn operation(snapshot: &mut KernelSnapshot, id: ExpressionId) -> &mut ExpressionOperation {
        let OwnerRecord::Expression(expression) =
            snapshot.owners.get_mut(&OwnerKey::Expression(id)).unwrap()
        else {
            unreachable!();
        };
        &mut expression.operation
    }

    fn expression(
        snapshot: &mut KernelSnapshot,
        seed: &[u8],
        operation: ExpressionOperation,
    ) -> ExpressionId {
        let id = ExpressionId::migrate(seed, 0);
        snapshot.owners.insert(
            OwnerKey::Expression(id),
            OwnerRecord::Expression(
                crate::platform::kernel::ExpressionRecord::new(id, operation).unwrap(),
            ),
        );
        id
    }

    fn admit(snapshot: &KernelSnapshot) -> Result<(), ExecutionError> {
        // Deliberately bypass production source validation and all bytecode:
        // this entrypoint must find the forgery using canonical metadata alone.
        let mut schema = super::super::NormalizedReferenceSchema {
            types: snapshot
                .types
                .iter()
                .chain(&snapshot.dependency_types)
                .map(|(id, ty)| (*id, ty.clone()))
                .collect(),
            ..super::super::NormalizedReferenceSchema::default()
        };
        super::super::complete(&mut schema, &[snapshot], &ExecutionControl::uncancelled())
    }

    #[test]
    fn independent_read_admission_accepts_projection_forwarding_and_aliased_inputs() {
        admit(&source()).unwrap();
    }

    #[test]
    fn independent_read_admission_accepts_shared_runtime_custody_fixture() {
        let source = super::super::super::tests::byte_buffer_tests::author_only(
            super::super::super::vm::borrow_result_tests::SOURCE,
        )
        .unwrap();
        admit(&source).unwrap();
    }

    #[test]
    fn independent_read_admission_preserves_legacy_owned_metadata_reads() {
        let source = super::super::super::tests::byte_buffer_tests::author_only(include_str!(
            "../../../../tests/fixtures/owned-products-read.lkjc"
        ))
        .unwrap();
        admit(&source).unwrap();
    }

    #[test]
    fn independent_read_admission_rejects_wrong_activation_parameter_despite_aliasing() {
        let mut snapshot = source();
        let first = function(&snapshot, "first");
        *operation(&mut snapshot, first.body) = ExpressionOperation::Local {
            value: LocalValueReference::FunctionParameter(first.parameters[1]),
        };
        assert_eq!(
            admit(&snapshot).unwrap_err().code,
            "reference_borrowed_result_contract"
        );
    }

    #[test]
    fn independent_read_admission_checks_untaken_terminal_paths() {
        let mut snapshot = source();
        let branch = function(&snapshot, "branch");
        let ExpressionOperation::If { when_false, .. } = *operation(&mut snapshot, branch.body)
        else {
            unreachable!();
        };
        *operation(&mut snapshot, when_false) = ExpressionOperation::Local {
            value: LocalValueReference::FunctionParameter(branch.parameters[2]),
        };
        assert_eq!(
            admit(&snapshot).unwrap_err().code,
            "reference_borrowed_result_contract"
        );
    }

    #[test]
    fn independent_read_admission_rejects_source_consumption_and_owning_view_forgery() {
        let baseline = source();
        let caller = function(&baseline, "caller");
        let mut snapshot = baseline.clone();
        let ExpressionOperation::Sequence { items } = operation(&mut snapshot, caller.body).clone()
        else {
            unreachable!();
        };
        let scope = items[0];
        let ExpressionOperation::BorrowCall {
            call,
            binding,
            body,
        } = *operation(&mut snapshot, scope)
        else {
            unreachable!();
        };
        let ExpressionOperation::Call { arguments, .. } = operation(&mut snapshot, call).clone()
        else {
            unreachable!();
        };
        let discard = reference(&snapshot, "discard");
        let consume = expression(
            &mut snapshot,
            b"reference-read-consume-source",
            ExpressionOperation::Call {
                function: discard,
                type_arguments: vec![],
                effect_arguments: vec![],
                requirement_arguments: vec![],
                arguments: vec![arguments[0]],
            },
        );
        let inner = expression(
            &mut snapshot,
            b"reference-read-consume-body",
            ExpressionOperation::Sequence {
                items: vec![consume, body],
            },
        );
        *operation(&mut snapshot, scope) = ExpressionOperation::BorrowCall {
            call,
            binding,
            body: inner,
        };
        assert_eq!(
            admit(&snapshot).unwrap_err().code,
            "reference_borrowed_result_contract"
        );

        let mut snapshot = baseline;
        let OwnerRecord::Binding(view) = snapshot
            .owners
            .get_mut(&OwnerKey::Binding(binding))
            .unwrap()
        else {
            unreachable!();
        };
        view.kind = BindingKind::OwnedUnpack;
        assert_eq!(
            admit(&snapshot).unwrap_err().code,
            "reference_borrowed_result_contract"
        );
    }

    #[test]
    fn independent_read_admission_invocation_permission_is_per_occurrence() {
        let mut snapshot = source();
        let caller = function(&snapshot, "caller");
        let ExpressionOperation::Sequence { mut items } =
            operation(&mut snapshot, caller.body).clone()
        else {
            unreachable!();
        };
        let ExpressionOperation::BorrowCall { call, .. } = *operation(&mut snapshot, items[0])
        else {
            unreachable!();
        };
        // The exact expression identity is legal inside the wrapper and illegal
        // in this additional ordinary occurrence in the same source body.
        items.insert(1, call);
        *operation(&mut snapshot, caller.body) = ExpressionOperation::Sequence { items };
        assert_eq!(
            admit(&snapshot).unwrap_err().code,
            "reference_borrowed_result_contract"
        );
    }

    #[test]
    fn independent_read_admission_rejects_first_class_borrowed_functions() {
        let mut snapshot = source();
        let caller = function(&snapshot, "caller");
        let first = reference(&snapshot, "first");
        let value = expression(
            &mut snapshot,
            b"reference-read-first-class",
            ExpressionOperation::FunctionValue {
                function: first,
                type_arguments: vec![],
                requirement_arguments: vec![],
                effect_arguments: vec![],
            },
        );
        let ExpressionOperation::Sequence { mut items } =
            operation(&mut snapshot, caller.body).clone()
        else {
            unreachable!();
        };
        items.insert(0, value);
        *operation(&mut snapshot, caller.body) = ExpressionOperation::Sequence { items };
        assert_eq!(
            admit(&snapshot).unwrap_err().code,
            "reference_borrowed_result_contract"
        );
    }

    #[test]
    fn independent_read_admission_checks_unused_method_sources_and_exact_mapping() {
        let baseline = source();
        let mut snapshot = baseline.clone();
        let reader = reference(&snapshot, "Reader");
        let OwnerRecord::Declaration(owner) = snapshot
            .owners
            .get_mut(&OwnerKey::Declaration(reader.declaration))
            .unwrap()
        else {
            unreachable!();
        };
        let DeclarationPayload::OwnedContract(contract) = &mut owner.payload else {
            unreachable!();
        };
        contract.methods[0].result_borrow = Some(0);
        assert_eq!(
            admit(&snapshot).unwrap_err().code,
            "reference_borrowed_result_contract"
        );

        let mut snapshot = baseline;
        let at = reference(&snapshot, "at");
        let OwnerRecord::Declaration(owner) = snapshot
            .owners
            .get_mut(&OwnerKey::Declaration(at.declaration))
            .unwrap()
        else {
            unreachable!();
        };
        let DeclarationPayload::Function(function) = &mut owner.payload else {
            unreachable!();
        };
        function.result_borrow = None;
        assert_eq!(
            admit(&snapshot).unwrap_err().code,
            "reference_borrowed_result_contract"
        );
    }
}
