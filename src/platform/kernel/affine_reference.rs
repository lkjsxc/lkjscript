//! Implementation-disjoint finite oracle for Graph 9 affine capability flow.
//!
//! This test oracle reads only complete `KernelSnapshot` values. It deliberately does not call the
//! production affine validator or reuse its provenance, transfer, or branch-merge structures.

use super::{
    BindingKind, DeclarationPayload, DeclarationReference, DeclarationVisibility,
    ExpressionOperation, FunctionEffect, KernelSnapshot, LocalValueReference, OperationRecord,
    OperationReference, OwnerKey, OwnerRecord, PackageId, PackageInterfaceDeclarationPayload,
    PackageInterfaceRecord, ParameterRecord, ParameterUse, RequirementRecord, RequirementReference,
    TypeForm, TypeObjectDigest,
};
use crate::platform::publication::GraphRepository;
use crate::platform::semantic_id::{DeclarationId, ExpressionId, OperationId, ParameterId};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Shape {
    Direct,
    Variant,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Right {
    shape: Shape,
    requirement: RequirementReference,
    interface: DeclarationReference,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Slot {
    right: Right,
    live: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Value {
    Plain,
    Affine(Right),
}

type Live = BTreeMap<LocalValueReference, Slot>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ResourceSignature {
    parameter: ParameterId,
    parameter_count: usize,
    right: Right,
}

struct Reference<'a> {
    snapshot: &'a KernelSnapshot,
}

impl Reference<'_> {
    fn accepts(&self) -> bool {
        if !self.shapes_are_legal() || !self.resource_calls_are_acyclic() {
            return false;
        }
        self.snapshot.owners.iter().all(|(owner, record)| {
            let (OwnerKey::Declaration(declaration_id), OwnerRecord::Declaration(declaration)) =
                (owner, record)
            else {
                return true;
            };
            let DeclarationPayload::Function(function) = &declaration.payload else {
                return true;
            };
            if !matches!(function.effect, FunctionEffect::Task { .. }) {
                return true;
            }
            let mut live = Live::new();
            if let Some(signature) = self
                .resource_signature(DeclarationReference {
                    package: self.snapshot.root.package_id,
                    declaration: *declaration_id,
                })
                .ok()
                .flatten()
            {
                live.insert(
                    LocalValueReference::FunctionParameter(signature.parameter),
                    Slot {
                        right: signature.right,
                        live: true,
                    },
                );
            }
            self.eval(function.body, &mut live).is_ok()
        })
    }

    fn shapes_are_legal(&self) -> bool {
        self.snapshot.owners.iter().all(|(key, owner)| match owner {
            OwnerRecord::Parameter(parameter) => match parameter.parent {
                super::ParameterParent::Function(declaration) => {
                    let shape = self.resource_type(parameter.ty, &mut BTreeSet::new());
                    match shape {
                        Some((Shape::Direct, _)) => {
                            parameter.use_mode == ParameterUse::Consume
                                && parameter.resource_requirement.is_some()
                                && self
                                    .resource_signature(DeclarationReference {
                                        package: self.snapshot.root.package_id,
                                        declaration,
                                    })
                                    .is_ok()
                        }
                        Some((Shape::Variant, _)) => false,
                        None => {
                            !self.contains_resource(parameter.ty, &mut BTreeSet::new())
                                && parameter.use_mode == ParameterUse::Unrestricted
                                && parameter.resource_requirement.is_none()
                        }
                    }
                }
                super::ParameterParent::Operation(operation) => {
                    let direct = self.resource_type(parameter.ty, &mut BTreeSet::new());
                    match direct {
                        Some((Shape::Direct, interface)) => {
                            parameter.use_mode != ParameterUse::Unrestricted
                                && parameter.resource_requirement.is_none()
                                && self.local_operation(operation).is_some_and(|record| {
                                    interface.package == self.snapshot.root.package_id
                                        && interface.declaration == record.declaration
                                })
                        }
                        _ => {
                            !self.contains_resource(parameter.ty, &mut BTreeSet::new())
                                && parameter.use_mode == ParameterUse::Unrestricted
                                && parameter.resource_requirement.is_none()
                        }
                    }
                }
            },
            OwnerRecord::Field(field) => !self.contains_resource(field.ty, &mut BTreeSet::new()),
            OwnerRecord::Case(case) => case.payload.is_none_or(|payload| {
                !self.contains_resource(payload, &mut BTreeSet::new())
                    || matches!(
                        self.resource_type(payload, &mut BTreeSet::new()),
                        Some((Shape::Direct, _))
                    )
            }),
            OwnerRecord::Operation(operation) => {
                let resource = self.resource_type(operation.result, &mut BTreeSet::new());
                match resource {
                    Some((_, interface)) => {
                        interface.package == self.snapshot.root.package_id
                            && interface.declaration == operation.declaration
                    }
                    None => !self.contains_resource(operation.result, &mut BTreeSet::new()),
                }
            }
            OwnerRecord::Declaration(declaration) => match &declaration.payload {
                DeclarationPayload::External(external) => {
                    !self.contains_resource(external.result, &mut BTreeSet::new())
                        && external.parameters.iter().all(|parameter| {
                            self.parameter(self.snapshot.root.package_id, *parameter)
                                .is_some_and(|parameter| {
                                    !self.contains_resource(parameter.ty, &mut BTreeSet::new())
                                        && parameter.use_mode == ParameterUse::Unrestricted
                                        && parameter.resource_requirement.is_none()
                                })
                        })
                }
                DeclarationPayload::Function(function) => {
                    !self.contains_resource(function.result, &mut BTreeSet::new())
                        && matches!(key, OwnerKey::Declaration(declaration) if self
                            .resource_signature(DeclarationReference {
                                package: self.snapshot.root.package_id,
                                declaration: *declaration,
                            })
                            .is_ok())
                }
                DeclarationPayload::Constant { ty, .. } => {
                    !self.contains_resource(*ty, &mut BTreeSet::new())
                }
                DeclarationPayload::Variant { cases } => {
                    cases
                        .iter()
                        .filter_map(
                            |case| match self.snapshot.owners.get(&OwnerKey::Case(*case)) {
                                Some(OwnerRecord::Case(case)) => case.payload,
                                _ => None,
                            },
                        )
                        .filter(|payload| {
                            matches!(
                                self.resource_type(*payload, &mut BTreeSet::new()),
                                Some((Shape::Direct, _))
                            )
                        })
                        .count()
                        <= 1
                }
                _ => true,
            },
            OwnerRecord::Port(port) => {
                !self.contains_resource(port.function_type, &mut BTreeSet::new())
            }
            _ => {
                let _ = key;
                true
            }
        })
    }

    fn eval(&self, expression: ExpressionId, live: &mut Live) -> Result<Value, ()> {
        let operation = match self.snapshot.owners.get(&OwnerKey::Expression(expression)) {
            Some(OwnerRecord::Expression(record)) => &record.operation,
            _ => return Err(()),
        };
        match operation {
            ExpressionOperation::Unit { .. }
            | ExpressionOperation::Bool { .. }
            | ExpressionOperation::I64 { .. }
            | ExpressionOperation::Text { .. }
            | ExpressionOperation::StaticText { .. }
            | ExpressionOperation::Constant { .. } => Ok(Value::Plain),
            ExpressionOperation::FunctionValue { function, .. } => {
                if self.resource_signature(*function)?.is_some() {
                    Err(())
                } else {
                    Ok(Value::Plain)
                }
            }
            ExpressionOperation::Local { value } => {
                if resource_owner(*value).is_some_and(|owner| live.contains_key(&owner)) {
                    Err(())
                } else {
                    Ok(Value::Plain)
                }
            }
            ExpressionOperation::If {
                condition,
                when_true,
                when_false,
            } => {
                self.plain(*condition, live)?;
                let before = live.clone();
                let mut left = before.clone();
                let left_value = self.eval(*when_true, &mut left)?;
                let mut right = before;
                let right_value = self.eval(*when_false, &mut right)?;
                if left != right || left_value != right_value {
                    return Err(());
                }
                *live = left;
                Ok(left_value)
            }
            ExpressionOperation::Let { bindings, body } => {
                let mut scoped = Vec::new();
                for binding in bindings {
                    let Some(OwnerRecord::Binding(record)) =
                        self.snapshot.owners.get(&OwnerKey::Binding(*binding))
                    else {
                        return Err(());
                    };
                    if record.kind != BindingKind::Let {
                        return Err(());
                    }
                    let value = self.eval(record.value.ok_or(())?, live)?;
                    if let Value::Affine(right) = value {
                        let declared = record.declared_type.ok_or(())?;
                        if self.resource_type(declared, &mut BTreeSet::new())
                            != Some((right.shape, right.interface))
                            || live
                                .insert(
                                    LocalValueReference::LexicalBinding(*binding),
                                    Slot { right, live: true },
                                )
                                .is_some()
                        {
                            return Err(());
                        }
                        scoped.push(LocalValueReference::LexicalBinding(*binding));
                    }
                }
                let result = self.eval(*body, live)?;
                for owner in scoped {
                    live.remove(&owner);
                }
                Ok(result)
            }
            ExpressionOperation::Sequence { items } => {
                let mut result = Value::Plain;
                for item in items {
                    result = self.eval(*item, live)?;
                }
                Ok(result)
            }
            ExpressionOperation::Call {
                function,
                arguments,
                ..
            } => self.function_call(*function, arguments, live),
            ExpressionOperation::Invoke { callee, arguments } => {
                self.plain(*callee, live)?;
                for argument in arguments {
                    self.plain(*argument, live)?;
                }
                Ok(Value::Plain)
            }
            ExpressionOperation::Record { fields, .. } => {
                for field in fields {
                    self.plain(field.value, live)?;
                }
                Ok(Value::Plain)
            }
            ExpressionOperation::Variant { case, payload } => {
                let payload_type = self.case_payload(case.package, case.case);
                let resource =
                    payload_type.and_then(|ty| self.resource_type(ty, &mut BTreeSet::new()));
                match (payload, resource) {
                    (Some(payload), Some((Shape::Direct, interface))) => {
                        let right = self.take(*payload, live, ParameterUse::Consume)?;
                        if right.interface != interface {
                            return Err(());
                        }
                        Ok(Value::Affine(Right {
                            shape: Shape::Variant,
                            ..right
                        }))
                    }
                    (Some(payload), None) => {
                        self.plain(*payload, live)?;
                        Ok(Value::Plain)
                    }
                    (None, None) => Ok(Value::Plain),
                    _ => Err(()),
                }
            }
            ExpressionOperation::Field { value, .. } => {
                self.plain(*value, live)?;
                Ok(Value::Plain)
            }
            ExpressionOperation::List { items, .. } => {
                for item in items {
                    self.plain(*item, live)?;
                }
                Ok(Value::Plain)
            }
            ExpressionOperation::Map { entries, .. } => {
                for entry in entries {
                    self.plain(entry.key, live)?;
                    self.plain(entry.value, live)?;
                }
                Ok(Value::Plain)
            }
            ExpressionOperation::Match { value, arms } => {
                let matched = self.match_value(*value, live)?;
                let before = live.clone();
                let mut states = Vec::new();
                let mut values = Vec::new();
                for arm in arms {
                    let mut branch = before.clone();
                    let resource = self
                        .case_payload(arm.case.package, arm.case.case)
                        .and_then(|ty| self.resource_type(ty, &mut BTreeSet::new()));
                    let mut scoped = None;
                    if let Some((Shape::Direct, interface)) = resource {
                        let outer = matched.ok_or(())?;
                        if outer.interface != interface {
                            return Err(());
                        }
                        let binding = arm.payload_binding.ok_or(())?;
                        let owner = LocalValueReference::MatchPayload(binding);
                        if branch
                            .insert(
                                owner,
                                Slot {
                                    right: Right {
                                        shape: Shape::Direct,
                                        ..outer
                                    },
                                    live: true,
                                },
                            )
                            .is_some()
                        {
                            return Err(());
                        }
                        scoped = Some(owner);
                    }
                    let value = self.eval(arm.body, &mut branch)?;
                    if let Some(owner) = scoped {
                        branch.remove(&owner);
                    }
                    states.push(branch);
                    values.push(value);
                }
                if states.windows(2).any(|pair| pair[0] != pair[1])
                    || values.windows(2).any(|pair| pair[0] != pair[1])
                {
                    return Err(());
                }
                *live = states.into_iter().next().unwrap_or(before);
                Ok(values.into_iter().next().unwrap_or(Value::Plain))
            }
            ExpressionOperation::CapabilityCall {
                requirement,
                operation,
                arguments,
            } => self.capability(*requirement, *operation, arguments, live),
            ExpressionOperation::Transaction { body, .. } => self.eval(*body, live),
        }
    }

    fn capability(
        &self,
        requirement: RequirementReference,
        operation: OperationReference,
        arguments: &[ExpressionId],
        live: &mut Live,
    ) -> Result<Value, ()> {
        let requirement_record = self.requirement(requirement).ok_or(())?;
        let operation_record = self.operation(operation).ok_or(())?;
        if requirement_record.interface.package != operation.package
            || requirement_record.interface.declaration != operation_record.declaration
            || operation_record.parameters.len() != arguments.len()
        {
            return Err(());
        }
        for (argument, parameter) in arguments.iter().zip(&operation_record.parameters) {
            let parameter = self.parameter(operation.package, *parameter).ok_or(())?;
            match parameter.use_mode {
                ParameterUse::Unrestricted => self.plain(*argument, live)?,
                mode @ (ParameterUse::Borrow | ParameterUse::Consume) => {
                    let (Shape::Direct, interface) = self
                        .resource_type(parameter.ty, &mut BTreeSet::new())
                        .ok_or(())?
                    else {
                        return Err(());
                    };
                    let right = self.take(*argument, live, mode)?;
                    if right.requirement != requirement
                        || right.interface != interface
                        || interface != requirement_record.interface
                    {
                        return Err(());
                    }
                }
            }
        }
        let Some((shape, interface)) =
            self.resource_type(operation_record.result, &mut BTreeSet::new())
        else {
            return Ok(Value::Plain);
        };
        if interface != requirement_record.interface {
            return Err(());
        }
        Ok(Value::Affine(Right {
            shape,
            requirement,
            interface,
        }))
    }

    fn function_call(
        &self,
        function: DeclarationReference,
        arguments: &[ExpressionId],
        live: &mut Live,
    ) -> Result<Value, ()> {
        let Some(signature) = self.resource_signature(function)? else {
            for argument in arguments {
                self.plain(*argument, live)?;
            }
            return Ok(Value::Plain);
        };
        if function.package != self.snapshot.root.package_id
            || arguments.len() != signature.parameter_count
        {
            return Err(());
        }
        let (resource, ordinary) = arguments.split_last().ok_or(())?;
        for argument in ordinary {
            self.plain(*argument, live)?;
        }
        let right = self.take(*resource, live, ParameterUse::Consume)?;
        if right != signature.right {
            return Err(());
        }
        Ok(Value::Plain)
    }

    fn resource_signature(
        &self,
        reference: DeclarationReference,
    ) -> Result<Option<ResourceSignature>, ()> {
        if reference.package != self.snapshot.root.package_id {
            let Some(PackageInterfaceRecord::Declaration(declaration)) = self.foreign_owner(
                reference.package,
                OwnerKey::Declaration(reference.declaration),
            ) else {
                return Ok(None);
            };
            let parameters = match &declaration.payload {
                PackageInterfaceDeclarationPayload::Function(signature) => &signature.parameters,
                PackageInterfaceDeclarationPayload::External(signature) => &signature.parameters,
                _ => return Ok(None),
            };
            if parameters.iter().any(|parameter| {
                self.parameter(reference.package, *parameter)
                    .is_none_or(|parameter| {
                        self.contains_resource(parameter.ty, &mut BTreeSet::new())
                            || parameter.use_mode != ParameterUse::Unrestricted
                            || parameter.resource_requirement.is_some()
                    })
            }) {
                return Err(());
            }
            return Ok(None);
        }

        let Some(OwnerRecord::Declaration(declaration)) = self
            .snapshot
            .owners
            .get(&OwnerKey::Declaration(reference.declaration))
        else {
            return Err(());
        };
        let function = match &declaration.payload {
            DeclarationPayload::Function(function) => function,
            DeclarationPayload::External(signature) => {
                if signature.parameters.iter().any(|parameter| {
                    self.parameter(reference.package, *parameter)
                        .is_none_or(|parameter| {
                            self.contains_resource(parameter.ty, &mut BTreeSet::new())
                                || parameter.use_mode != ParameterUse::Unrestricted
                                || parameter.resource_requirement.is_some()
                        })
                }) {
                    return Err(());
                }
                return Ok(None);
            }
            _ => return Ok(None),
        };

        let mut resource = None;
        for (index, parameter) in function.parameters.iter().copied().enumerate() {
            let record = self.parameter(reference.package, parameter).ok_or(())?;
            if record.parent != super::ParameterParent::Function(reference.declaration) {
                return Err(());
            }
            match self.resource_type(record.ty, &mut BTreeSet::new()) {
                Some((Shape::Direct, interface)) => {
                    if resource.is_some()
                        || index.saturating_add(1) != function.parameters.len()
                        || record.use_mode != ParameterUse::Consume
                    {
                        return Err(());
                    }
                    let requirement = record.resource_requirement.ok_or(())?;
                    resource = Some(ResourceSignature {
                        parameter,
                        parameter_count: function.parameters.len(),
                        right: Right {
                            shape: Shape::Direct,
                            requirement,
                            interface,
                        },
                    });
                }
                Some((Shape::Variant, _)) => return Err(()),
                None => {
                    if self.contains_resource(record.ty, &mut BTreeSet::new())
                        || record.use_mode != ParameterUse::Unrestricted
                        || record.resource_requirement.is_some()
                    {
                        return Err(());
                    }
                }
            }
        }
        let Some(resource) = resource else {
            return Ok(None);
        };
        if declaration.visibility != DeclarationVisibility::Private
            || !function.type_parameters.is_empty()
            || self.contains_resource(function.result, &mut BTreeSet::new())
        {
            return Err(());
        }
        let FunctionEffect::Task { requirements } = &function.effect else {
            return Err(());
        };
        if resource.right.requirement.package != self.snapshot.root.package_id
            || !requirements.contains(&resource.right.requirement)
            || self
                .requirement(resource.right.requirement)
                .is_none_or(|requirement| requirement.interface != resource.right.interface)
        {
            return Err(());
        }
        Ok(Some(resource))
    }

    fn resource_calls_are_acyclic(&self) -> bool {
        let mut nodes = BTreeSet::new();
        for (owner, record) in &self.snapshot.owners {
            let (OwnerKey::Declaration(declaration), OwnerRecord::Declaration(record)) =
                (owner, record)
            else {
                continue;
            };
            if !matches!(record.payload, DeclarationPayload::Function(_)) {
                continue;
            }
            match self.resource_signature(DeclarationReference {
                package: self.snapshot.root.package_id,
                declaration: *declaration,
            }) {
                Ok(Some(_)) => {
                    nodes.insert(*declaration);
                }
                Ok(None) => {}
                Err(()) => return false,
            }
        }
        let mut edges = BTreeMap::<DeclarationId, BTreeSet<DeclarationId>>::new();
        let mut incoming = nodes
            .iter()
            .copied()
            .map(|node| (node, 0_usize))
            .collect::<BTreeMap<_, _>>();
        for node in &nodes {
            let Some(OwnerRecord::Declaration(record)) =
                self.snapshot.owners.get(&OwnerKey::Declaration(*node))
            else {
                return false;
            };
            let DeclarationPayload::Function(function) = &record.payload else {
                return false;
            };
            let Some(callees) = self.resource_callees(function.body) else {
                return false;
            };
            for callee in callees {
                if !nodes.contains(&callee) {
                    return false;
                }
                if edges.entry(*node).or_default().insert(callee) {
                    let Some(count) = incoming.get_mut(&callee) else {
                        return false;
                    };
                    *count = count.saturating_add(1);
                }
            }
        }
        let mut ready = incoming
            .iter()
            .filter_map(|(node, count)| (*count == 0).then_some(*node))
            .collect::<BTreeSet<_>>();
        let mut visited = 0_usize;
        while let Some(node) = ready.pop_first() {
            visited = visited.saturating_add(1);
            for callee in edges.get(&node).into_iter().flatten() {
                let Some(count) = incoming.get_mut(callee) else {
                    return false;
                };
                *count = count.saturating_sub(1);
                if *count == 0 {
                    ready.insert(*callee);
                }
            }
        }
        visited == nodes.len()
    }

    fn resource_callees(&self, body: ExpressionId) -> Option<BTreeSet<DeclarationId>> {
        let mut pending = vec![body];
        let mut visited = BTreeSet::new();
        let mut callees = BTreeSet::new();
        while let Some(expression) = pending.pop() {
            if !visited.insert(expression) {
                continue;
            }
            let Some(OwnerRecord::Expression(record)) =
                self.snapshot.owners.get(&OwnerKey::Expression(expression))
            else {
                return None;
            };
            if let ExpressionOperation::Call { function, .. } = &record.operation {
                match self.resource_signature(*function) {
                    Ok(Some(_)) if function.package == self.snapshot.root.package_id => {
                        callees.insert(function.declaration);
                    }
                    Ok(Some(_)) | Err(()) => return None,
                    Ok(None) => {}
                }
            }
            pending.extend(record.children().into_iter().map(|child| child.expression));
        }
        Some(callees)
    }

    fn plain(&self, expression: ExpressionId, live: &mut Live) -> Result<(), ()> {
        if self.eval(expression, live)? == Value::Plain {
            Ok(())
        } else {
            Err(())
        }
    }

    fn take(
        &self,
        expression: ExpressionId,
        live: &mut Live,
        mode: ParameterUse,
    ) -> Result<Right, ()> {
        let Some(OwnerRecord::Expression(record)) =
            self.snapshot.owners.get(&OwnerKey::Expression(expression))
        else {
            return Err(());
        };
        let ExpressionOperation::Local { value } = record.operation else {
            return Err(());
        };
        let owner = resource_owner(value).ok_or(())?;
        let slot = live.get_mut(&owner).ok_or(())?;
        if !slot.live {
            return Err(());
        }
        let right = slot.right;
        if right.shape != Shape::Direct {
            return Err(());
        }
        if mode == ParameterUse::Consume {
            slot.live = false;
        }
        Ok(right)
    }

    fn match_value(&self, expression: ExpressionId, live: &mut Live) -> Result<Option<Right>, ()> {
        if let Some(OwnerRecord::Expression(record)) =
            self.snapshot.owners.get(&OwnerKey::Expression(expression))
            && let ExpressionOperation::Local { value } = record.operation
            && let Some(owner) = resource_owner(value)
            && let Some(slot) = live.get_mut(&owner)
        {
            if !slot.live {
                return Err(());
            }
            slot.live = false;
            return (slot.right.shape == Shape::Variant)
                .then_some(Some(slot.right))
                .ok_or(());
        }
        match self.eval(expression, live)? {
            Value::Plain => Ok(None),
            Value::Affine(right) if right.shape == Shape::Variant => Ok(Some(right)),
            Value::Affine(_) => Err(()),
        }
    }

    fn resource_type(
        &self,
        digest: TypeObjectDigest,
        active: &mut BTreeSet<TypeObjectDigest>,
    ) -> Option<(Shape, DeclarationReference)> {
        if !active.insert(digest) {
            return None;
        }
        let result = match &self.type_object(digest)?.form {
            TypeForm::CapabilityResource { interface } => Some((Shape::Direct, *interface)),
            TypeForm::Named { declaration } => {
                let mut found = None;
                for payload in self.variant_payloads(*declaration) {
                    if let Some((Shape::Direct, interface)) = self.resource_type(payload, active) {
                        if found.is_some_and(|previous| previous != interface) {
                            return None;
                        }
                        found = Some(interface);
                    }
                }
                found.map(|interface| (Shape::Variant, interface))
            }
            _ => None,
        };
        active.remove(&digest);
        result
    }

    fn contains_resource(
        &self,
        digest: TypeObjectDigest,
        active: &mut BTreeSet<TypeObjectDigest>,
    ) -> bool {
        if !active.insert(digest) {
            return false;
        }
        let Some(object) = self.type_object(digest) else {
            return false;
        };
        let result = match &object.form {
            TypeForm::CapabilityResource { .. } => true,
            TypeForm::Named { declaration } => self
                .named_member_types(*declaration)
                .into_iter()
                .any(|member| self.contains_resource(member, active)),
            TypeForm::StructuralRecord { fields } => fields
                .iter()
                .any(|field| self.contains_resource(field.ty, active)),
            TypeForm::List { item } | TypeForm::Option { item } | TypeForm::Stream { item } => {
                self.contains_resource(*item, active)
            }
            TypeForm::Map { key, value }
            | TypeForm::Result {
                ok: key,
                error: value,
            } => self.contains_resource(*key, active) || self.contains_resource(*value, active),
            TypeForm::Function { parameters, result } => {
                self.contains_resource(*result, active)
                    || parameters
                        .iter()
                        .any(|parameter| self.contains_resource(*parameter, active))
            }
            _ => false,
        };
        active.remove(&digest);
        result
    }

    fn type_object(&self, digest: TypeObjectDigest) -> Option<&super::TypeObject> {
        self.snapshot
            .types
            .get(&digest)
            .or_else(|| self.snapshot.dependency_types.get(&digest))
    }

    fn local_operation(&self, operation: OperationId) -> Option<&OperationRecord> {
        match self.snapshot.owners.get(&OwnerKey::Operation(operation))? {
            OwnerRecord::Operation(record) => Some(record),
            _ => None,
        }
    }

    fn operation(&self, reference: OperationReference) -> Option<OperationRecord> {
        if reference.package == self.snapshot.root.package_id {
            return self.local_operation(reference.operation).cloned();
        }
        match self.foreign_owner(reference.package, OwnerKey::Operation(reference.operation))? {
            PackageInterfaceRecord::Operation(record) => Some(record.clone()),
            _ => None,
        }
    }

    fn parameter(&self, package: PackageId, parameter: ParameterId) -> Option<ParameterRecord> {
        if package == self.snapshot.root.package_id {
            return match self.snapshot.owners.get(&OwnerKey::Parameter(parameter))? {
                OwnerRecord::Parameter(record) => Some(record.clone()),
                _ => None,
            };
        }
        match self.foreign_owner(package, OwnerKey::Parameter(parameter))? {
            PackageInterfaceRecord::Parameter(record) => Some(record.clone()),
            _ => None,
        }
    }

    fn requirement(&self, reference: RequirementReference) -> Option<RequirementRecord> {
        if reference.package == self.snapshot.root.package_id {
            return match self
                .snapshot
                .owners
                .get(&OwnerKey::Requirement(reference.requirement))?
            {
                OwnerRecord::Requirement(record) => Some(record.clone()),
                _ => None,
            };
        }
        match self.foreign_owner(
            reference.package,
            OwnerKey::Requirement(reference.requirement),
        )? {
            PackageInterfaceRecord::Requirement(record) => Some(record.clone()),
            _ => None,
        }
    }

    fn case_payload(
        &self,
        package: PackageId,
        case: crate::platform::semantic_id::CaseId,
    ) -> Option<TypeObjectDigest> {
        if package == self.snapshot.root.package_id {
            return match self.snapshot.owners.get(&OwnerKey::Case(case))? {
                OwnerRecord::Case(record) => record.payload,
                _ => None,
            };
        }
        match self.foreign_owner(package, OwnerKey::Case(case))? {
            PackageInterfaceRecord::Case(record) => record.payload,
            _ => None,
        }
    }

    fn variant_payloads(&self, declaration: DeclarationReference) -> Vec<TypeObjectDigest> {
        if declaration.package == self.snapshot.root.package_id {
            let Some(OwnerRecord::Declaration(record)) = self
                .snapshot
                .owners
                .get(&OwnerKey::Declaration(declaration.declaration))
            else {
                return Vec::new();
            };
            let DeclarationPayload::Variant { cases } = &record.payload else {
                return Vec::new();
            };
            return cases
                .iter()
                .filter_map(|case| self.case_payload(declaration.package, *case))
                .collect();
        }
        let Some(PackageInterfaceRecord::Declaration(record)) = self.foreign_owner(
            declaration.package,
            OwnerKey::Declaration(declaration.declaration),
        ) else {
            return Vec::new();
        };
        let PackageInterfaceDeclarationPayload::Variant { cases } = &record.payload else {
            return Vec::new();
        };
        cases
            .iter()
            .filter_map(|case| self.case_payload(declaration.package, *case))
            .collect()
    }

    fn named_member_types(&self, declaration: DeclarationReference) -> Vec<TypeObjectDigest> {
        if declaration.package == self.snapshot.root.package_id {
            let Some(OwnerRecord::Declaration(record)) = self
                .snapshot
                .owners
                .get(&OwnerKey::Declaration(declaration.declaration))
            else {
                return Vec::new();
            };
            return match &record.payload {
                DeclarationPayload::Record { fields } => fields
                    .iter()
                    .filter_map(
                        |field| match self.snapshot.owners.get(&OwnerKey::Field(*field)) {
                            Some(OwnerRecord::Field(record)) => Some(record.ty),
                            _ => None,
                        },
                    )
                    .collect(),
                DeclarationPayload::Variant { cases } => cases
                    .iter()
                    .filter_map(|case| self.case_payload(declaration.package, *case))
                    .collect(),
                _ => Vec::new(),
            };
        }
        let Some(PackageInterfaceRecord::Declaration(record)) = self.foreign_owner(
            declaration.package,
            OwnerKey::Declaration(declaration.declaration),
        ) else {
            return Vec::new();
        };
        match &record.payload {
            PackageInterfaceDeclarationPayload::Record { fields } => fields
                .iter()
                .filter_map(|field| {
                    match self.foreign_owner(declaration.package, OwnerKey::Field(*field)) {
                        Some(PackageInterfaceRecord::Field(record)) => Some(record.ty),
                        _ => None,
                    }
                })
                .collect(),
            PackageInterfaceDeclarationPayload::Variant { cases } => cases
                .iter()
                .filter_map(|case| self.case_payload(declaration.package, *case))
                .collect(),
            _ => Vec::new(),
        }
    }

    fn foreign_owner(
        &self,
        package: PackageId,
        owner: OwnerKey,
    ) -> Option<&PackageInterfaceRecord> {
        let dependency = self.snapshot.dependencies.get(&package)?;
        self.snapshot
            .dependency_interfaces
            .get(&dependency.package_revision)?
            .get(&owner)
    }
}

fn resource_owner(value: LocalValueReference) -> Option<LocalValueReference> {
    match value {
        LocalValueReference::FunctionParameter(_)
        | LocalValueReference::LexicalBinding(_)
        | LocalValueReference::MatchPayload(_) => Some(value),
        LocalValueReference::OperationParameter(_) | LocalValueReference::TransactionBinding(_) => {
            None
        }
    }
}

fn maintained_snapshot(relative: &str) -> KernelSnapshot {
    let project = Path::new(env!("CARGO_MANIFEST_DIR")).join(relative);
    GraphRepository::open(&project)
        .expect("open maintained affine-oracle project")
        .view_current()
        .expect("read maintained affine-oracle HEAD")
        .reconstruct_full_oracle()
        .expect("reconstruct maintained affine-oracle snapshot")
        .value
}

fn production_accepts(snapshot: &KernelSnapshot) -> bool {
    let mut diagnostics = Vec::new();
    let mut work = 0;
    let result = super::validate_affine_roots_with_limits(
        snapshot,
        snapshot.owners.keys().copied(),
        &mut diagnostics,
        &mut work,
        super::ExpressionValidationLimits {
            maximum_steps: 1_000_000,
            maximum_diagnostics: 1_000,
        },
    );
    result.is_ok() && diagnostics.is_empty()
}

fn operation_name(snapshot: &KernelSnapshot, reference: OperationReference) -> Option<String> {
    Reference { snapshot }
        .operation(reference)
        .map(|record| record.name.as_str().to_owned())
}

fn capability_call(
    snapshot: &KernelSnapshot,
    names: &[&str],
) -> Option<(
    ExpressionId,
    RequirementReference,
    OperationReference,
    Vec<ExpressionId>,
)> {
    snapshot.owners.iter().find_map(|(owner, record)| {
        let (OwnerKey::Expression(expression), OwnerRecord::Expression(record)) = (owner, record)
        else {
            return None;
        };
        let ExpressionOperation::CapabilityCall {
            requirement,
            operation,
            arguments,
        } = &record.operation
        else {
            return None;
        };
        names
            .contains(&operation_name(snapshot, *operation)?.as_str())
            .then_some((*expression, *requirement, *operation, arguments.clone()))
    })
}

fn mutate_fabricated_resource(snapshot: &mut KernelSnapshot) {
    let (_, _, _, arguments) =
        capability_call(snapshot, &["complete", "fail"]).expect("maintained worker terminal call");
    let Some(OwnerRecord::Expression(argument)) =
        snapshot.owners.get_mut(&OwnerKey::Expression(arguments[0]))
    else {
        panic!("maintained terminal resource argument");
    };
    argument.operation = ExpressionOperation::Unit {};
}

fn mutate_post_consume_escape(snapshot: &mut KernelSnapshot) {
    let (call, _, _, arguments) =
        capability_call(snapshot, &["complete"]).expect("maintained worker completion call");
    let Some(OwnerRecord::Expression(record)) =
        snapshot.owners.get_mut(&OwnerKey::Expression(call))
    else {
        panic!("maintained completion expression");
    };
    let ExpressionOperation::CapabilityCall {
        arguments: call_arguments,
        ..
    } = &mut record.operation
    else {
        panic!("maintained completion operation");
    };
    call_arguments[2] = arguments[0];
}

fn mutate_duplicate_consume(snapshot: &mut KernelSnapshot) {
    let (call, _, _, _) =
        capability_call(snapshot, &["complete"]).expect("maintained worker completion call");
    let clone_id = ExpressionId::migrate(b"affine-reference-duplicate-consume", 0);
    let Some(OwnerRecord::Expression(record)) = snapshot.owners.get(&OwnerKey::Expression(call))
    else {
        panic!("maintained completion expression");
    };
    let clone = super::ExpressionRecord::new(clone_id, record.operation.clone())
        .expect("duplicate-consume mutation expression");
    assert!(
        snapshot
            .owners
            .insert(
                OwnerKey::Expression(clone_id),
                OwnerRecord::Expression(clone)
            )
            .is_none()
    );
    let Some(OwnerRecord::Expression(record)) =
        snapshot.owners.get_mut(&OwnerKey::Expression(call))
    else {
        panic!("maintained completion expression");
    };
    record.operation = ExpressionOperation::Sequence {
        items: vec![clone_id, clone_id],
    };
}

fn mutate_invoke_resource(snapshot: &mut KernelSnapshot) {
    let (call, _, _, arguments) =
        capability_call(snapshot, &["complete", "fail"]).expect("maintained worker terminal call");
    let Some(OwnerRecord::Expression(record)) =
        snapshot.owners.get_mut(&OwnerKey::Expression(call))
    else {
        panic!("maintained terminal expression");
    };
    record.operation = ExpressionOperation::Invoke {
        callee: arguments[0],
        arguments: Vec::new(),
    };
}

fn mutate_foreign_requirement(snapshot: &mut KernelSnapshot) {
    let (call, requirement, _, _) =
        capability_call(snapshot, &["complete", "fail"]).expect("maintained worker terminal call");
    let interface = Reference { snapshot }
        .requirement(requirement)
        .expect("terminal requirement")
        .interface;
    let replacement = snapshot.owners.iter().find_map(|(owner, record)| {
        let (OwnerKey::Requirement(candidate), OwnerRecord::Requirement(record)) = (owner, record)
        else {
            return None;
        };
        let reference = RequirementReference {
            package: snapshot.root.package_id,
            requirement: *candidate,
        };
        (reference != requirement && record.interface == interface).then_some(reference)
    });
    let replacement = replacement.expect("second exact durable-queue requirement");
    let Some(OwnerRecord::Expression(record)) =
        snapshot.owners.get_mut(&OwnerKey::Expression(call))
    else {
        panic!("maintained terminal expression");
    };
    let ExpressionOperation::CapabilityCall { requirement, .. } = &mut record.operation else {
        panic!("maintained terminal operation");
    };
    *requirement = replacement;
}

fn mutate_branch_join(snapshot: &mut KernelSnapshot) {
    let (branch, _, _, arguments) =
        capability_call(snapshot, &["complete"]).expect("maintained worker completion call");
    let clone_id = ExpressionId::migrate(b"affine-reference-branch-consume", 0);
    let Some(OwnerRecord::Expression(record)) = snapshot.owners.get(&OwnerKey::Expression(branch))
    else {
        panic!("completion branch expression");
    };
    let clone = super::ExpressionRecord::new(clone_id, record.operation.clone())
        .expect("branch-consume mutation expression");
    assert!(
        snapshot
            .owners
            .insert(
                OwnerKey::Expression(clone_id),
                OwnerRecord::Expression(clone)
            )
            .is_none()
    );
    let Some(OwnerRecord::Expression(record)) =
        snapshot.owners.get_mut(&OwnerKey::Expression(branch))
    else {
        panic!("completion branch expression");
    };
    record.operation = ExpressionOperation::If {
        condition: arguments[1],
        when_true: clone_id,
        when_false: arguments[2],
    };
}

fn mutate_operation_use(snapshot: &mut KernelSnapshot) {
    let operation = snapshot
        .owners
        .iter()
        .find_map(|(owner, record)| match (owner, record) {
            (OwnerKey::Operation(operation), OwnerRecord::Operation(record))
                if record.name.as_str() == "complete" =>
            {
                Some((*operation, record.parameters[0]))
            }
            _ => None,
        });
    let (_, parameter) = operation.expect("standard complete operation");
    let Some(OwnerRecord::Parameter(record)) =
        snapshot.owners.get_mut(&OwnerKey::Parameter(parameter))
    else {
        panic!("standard complete lease parameter");
    };
    record.use_mode = ParameterUse::Unrestricted;
}

fn mutate_record_containment(snapshot: &mut KernelSnapshot) {
    let resource = snapshot.types.iter().find_map(|(digest, object)| {
        matches!(object.form, TypeForm::CapabilityResource { .. }).then_some(*digest)
    });
    let resource = resource.expect("standard resource type");
    let field = snapshot
        .owners
        .iter()
        .find_map(|(owner, record)| match (owner, record) {
            (OwnerKey::Field(field), OwnerRecord::Field(_)) => Some(*field),
            _ => None,
        });
    let Some(OwnerRecord::Field(record)) = snapshot
        .owners
        .get_mut(&OwnerKey::Field(field.expect("standard record field")))
    else {
        panic!("standard record field owner");
    };
    record.ty = resource;
}

fn mutate_function_escape(snapshot: &mut KernelSnapshot) {
    let resource = snapshot
        .dependency_types
        .iter()
        .find_map(|(digest, object)| {
            matches!(object.form, TypeForm::CapabilityResource { .. }).then_some(*digest)
        })
        .expect("dependency queue resource type");
    let function = snapshot
        .owners
        .iter()
        .find_map(|(owner, record)| match (owner, record) {
            (OwnerKey::Declaration(declaration), OwnerRecord::Declaration(record))
                if matches!(
                    record.payload,
                    DeclarationPayload::Function(super::FunctionDeclaration {
                        effect: FunctionEffect::Task { .. },
                        ..
                    })
                ) =>
            {
                Some(*declaration)
            }
            _ => None,
        });
    let Some(OwnerRecord::Declaration(record)) = snapshot.owners.get_mut(&OwnerKey::Declaration(
        function.expect("maintained task function"),
    )) else {
        panic!("maintained task declaration");
    };
    let DeclarationPayload::Function(function) = &mut record.payload else {
        panic!("maintained task payload");
    };
    function.result = resource;
}

fn maintained_resource_helper(
    snapshot: &KernelSnapshot,
) -> (DeclarationId, ParameterId, RequirementReference) {
    snapshot
        .owners
        .iter()
        .find_map(|(owner, record)| {
            let (OwnerKey::Declaration(declaration), OwnerRecord::Declaration(record)) =
                (owner, record)
            else {
                return None;
            };
            let DeclarationPayload::Function(function) = &record.payload else {
                return None;
            };
            if record.name.as_str() != "process-lease" {
                return None;
            }
            let parameter = *function.parameters.last()?;
            let requirement = match snapshot.owners.get(&OwnerKey::Parameter(parameter)) {
                Some(OwnerRecord::Parameter(parameter)) => parameter.resource_requirement?,
                _ => return None,
            };
            Some((*declaration, parameter, requirement))
        })
        .expect("maintained resource helper")
}

fn mutate_missing_function_binding(snapshot: &mut KernelSnapshot) {
    let (_, parameter, _) = maintained_resource_helper(snapshot);
    let Some(OwnerRecord::Parameter(record)) =
        snapshot.owners.get_mut(&OwnerKey::Parameter(parameter))
    else {
        panic!("maintained resource helper parameter");
    };
    record.resource_requirement = None;
}

fn mutate_wrong_function_binding(snapshot: &mut KernelSnapshot) {
    let (_, parameter, requirement) = maintained_resource_helper(snapshot);
    let replacement = snapshot
        .owners
        .iter()
        .find_map(|(owner, record)| {
            let (OwnerKey::Requirement(candidate), OwnerRecord::Requirement(_)) = (owner, record)
            else {
                return None;
            };
            let reference = RequirementReference {
                package: snapshot.root.package_id,
                requirement: *candidate,
            };
            (reference != requirement).then_some(reference)
        })
        .expect("foreign maintained requirement");
    let Some(OwnerRecord::Parameter(record)) =
        snapshot.owners.get_mut(&OwnerKey::Parameter(parameter))
    else {
        panic!("maintained resource helper parameter");
    };
    record.resource_requirement = Some(replacement);
}

fn mutate_public_resource_helper(snapshot: &mut KernelSnapshot) {
    let (helper, _, _) = maintained_resource_helper(snapshot);
    let Some(OwnerRecord::Declaration(record)) =
        snapshot.owners.get_mut(&OwnerKey::Declaration(helper))
    else {
        panic!("maintained resource helper declaration");
    };
    record.visibility = DeclarationVisibility::Public;
}

fn mutate_resource_self_recursion(snapshot: &mut KernelSnapshot) {
    let (helper, parameter, _) = maintained_resource_helper(snapshot);
    let ordinary = ExpressionId::migrate(b"affine-reference-self-recursion-ordinary", 0);
    let local = ExpressionId::migrate(b"affine-reference-self-recursion-local", 0);
    let call = ExpressionId::migrate(b"affine-reference-self-recursion-call", 0);
    let ordinary_record =
        super::ExpressionRecord::new(ordinary, ExpressionOperation::I64 { value: 0 })
            .expect("self-recursion ordinary argument");
    let local_record = super::ExpressionRecord::new(
        local,
        ExpressionOperation::Local {
            value: LocalValueReference::FunctionParameter(parameter),
        },
    )
    .expect("self-recursion local");
    let call_record = super::ExpressionRecord::new(
        call,
        ExpressionOperation::Call {
            function: DeclarationReference {
                package: snapshot.root.package_id,
                declaration: helper,
            },
            type_arguments: Vec::new(),
            arguments: vec![ordinary, local],
        },
    )
    .expect("self-recursion call");
    assert!(
        snapshot
            .owners
            .insert(
                OwnerKey::Expression(ordinary),
                OwnerRecord::Expression(ordinary_record)
            )
            .is_none()
    );
    assert!(
        snapshot
            .owners
            .insert(
                OwnerKey::Expression(local),
                OwnerRecord::Expression(local_record)
            )
            .is_none()
    );
    assert!(
        snapshot
            .owners
            .insert(
                OwnerKey::Expression(call),
                OwnerRecord::Expression(call_record)
            )
            .is_none()
    );
    let Some(OwnerRecord::Declaration(record)) =
        snapshot.owners.get_mut(&OwnerKey::Declaration(helper))
    else {
        panic!("maintained resource helper declaration");
    };
    let DeclarationPayload::Function(function) = &mut record.payload else {
        panic!("maintained resource helper function");
    };
    function.body = call;
}

fn mutate_resource_function_value(snapshot: &mut KernelSnapshot) {
    let (helper, _, _) = maintained_resource_helper(snapshot);
    let call = snapshot.owners.iter().find_map(|(owner, record)| {
        let (OwnerKey::Expression(expression), OwnerRecord::Expression(record)) = (owner, record)
        else {
            return None;
        };
        matches!(
            record.operation,
            ExpressionOperation::Call {
                function: DeclarationReference { declaration, .. },
                ..
            } if declaration == helper
        )
        .then_some(*expression)
    });
    let Some(OwnerRecord::Expression(record)) = snapshot.owners.get_mut(&OwnerKey::Expression(
        call.expect("maintained resource handoff call"),
    )) else {
        panic!("maintained resource handoff expression");
    };
    record.operation = ExpressionOperation::FunctionValue {
        function: DeclarationReference {
            package: snapshot.root.package_id,
            declaration: helper,
        },
        type_arguments: Vec::new(),
    };
}

fn mutate_duplicate_handoff(snapshot: &mut KernelSnapshot) {
    let (helper, _, _) = maintained_resource_helper(snapshot);
    let call = snapshot.owners.iter().find_map(|(owner, record)| {
        let (OwnerKey::Expression(expression), OwnerRecord::Expression(record)) = (owner, record)
        else {
            return None;
        };
        matches!(
            record.operation,
            ExpressionOperation::Call {
                function: DeclarationReference { declaration, .. },
                ..
            } if declaration == helper
        )
        .then_some(*expression)
    });
    let call = call.expect("maintained resource handoff call");
    let clone_id = ExpressionId::migrate(b"affine-reference-duplicate-handoff", 0);
    let Some(OwnerRecord::Expression(record)) = snapshot.owners.get(&OwnerKey::Expression(call))
    else {
        panic!("maintained resource handoff expression");
    };
    let clone = super::ExpressionRecord::new(clone_id, record.operation.clone())
        .expect("duplicate-handoff mutation expression");
    assert!(
        snapshot
            .owners
            .insert(
                OwnerKey::Expression(clone_id),
                OwnerRecord::Expression(clone)
            )
            .is_none()
    );
    let Some(OwnerRecord::Expression(record)) =
        snapshot.owners.get_mut(&OwnerKey::Expression(call))
    else {
        panic!("maintained resource handoff expression");
    };
    record.operation = ExpressionOperation::Sequence {
        items: vec![clone_id, clone_id],
    };
}

type SnapshotMutation = (&'static str, fn(&mut KernelSnapshot));

#[test]
fn independent_reference_agrees_on_maintained_graphs_and_finite_negative_corpus() {
    let standard = maintained_snapshot("packages/standard");
    let journal = maintained_snapshot("applications/lkjournal");
    for (name, snapshot) in [("standard", &standard), ("lkjournal", &journal)] {
        assert!(production_accepts(snapshot), "production accepts {name}");
        assert!(Reference { snapshot }.accepts(), "reference accepts {name}");
    }

    let journal_mutations: [SnapshotMutation; 13] = [
        ("fabricated resource", mutate_fabricated_resource),
        ("post-consume escape", mutate_post_consume_escape),
        ("duplicate consume", mutate_duplicate_consume),
        ("resource invocation", mutate_invoke_resource),
        ("foreign requirement", mutate_foreign_requirement),
        ("branch join", mutate_branch_join),
        ("function escape", mutate_function_escape),
        ("missing function binding", mutate_missing_function_binding),
        ("wrong function binding", mutate_wrong_function_binding),
        ("public resource helper", mutate_public_resource_helper),
        ("resource self recursion", mutate_resource_self_recursion),
        ("resource function value", mutate_resource_function_value),
        ("duplicate handoff", mutate_duplicate_handoff),
    ];
    for (name, mutate) in journal_mutations {
        let mut candidate = journal.clone();
        mutate(&mut candidate);
        assert!(!production_accepts(&candidate), "production rejects {name}");
        assert!(
            !Reference {
                snapshot: &candidate
            }
            .accepts(),
            "reference rejects {name}"
        );
    }

    let standard_mutations: [SnapshotMutation; 2] = [
        ("unrestricted resource parameter", mutate_operation_use),
        ("resource record containment", mutate_record_containment),
    ];
    for (name, mutate) in standard_mutations {
        let mut candidate = standard.clone();
        mutate(&mut candidate);
        assert!(!production_accepts(&candidate), "production rejects {name}");
        assert!(
            !Reference {
                snapshot: &candidate
            }
            .accepts(),
            "reference rejects {name}"
        );
    }
}
