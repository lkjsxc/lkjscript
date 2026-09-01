//! Implementation-disjoint finite oracle for Graph 6 affine capability flow.
//!
//! This test oracle reads only complete `KernelSnapshot` values. It deliberately does not call the
//! production affine validator or reuse its provenance, transfer, or branch-merge structures.

use super::{
    BindingKind, DeclarationPayload, DeclarationReference, ExpressionOperation, FunctionEffect,
    KernelSnapshot, LocalValueReference, OperationRecord, OperationReference, OwnerKey,
    OwnerRecord, PackageId, PackageInterfaceDeclarationPayload, PackageInterfaceRecord,
    ParameterRecord, ParameterUse, RequirementRecord, RequirementReference, TypeForm,
    TypeObjectDigest,
};
use crate::platform::publication::GraphRepository;
use crate::platform::semantic_id::{BindingId, ExpressionId, OperationId, ParameterId};
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

type Live = BTreeMap<BindingId, Slot>;

struct Reference<'a> {
    snapshot: &'a KernelSnapshot,
}

impl Reference<'_> {
    fn accepts(&self) -> bool {
        if !self.shapes_are_legal() {
            return false;
        }
        self.snapshot.owners.values().all(|owner| {
            let OwnerRecord::Declaration(declaration) = owner else {
                return true;
            };
            let DeclarationPayload::Function(function) = &declaration.payload else {
                return true;
            };
            if !matches!(function.effect, FunctionEffect::Task { .. }) {
                return true;
            }
            self.eval(function.body, &mut Live::new()).is_ok()
        })
    }

    fn shapes_are_legal(&self) -> bool {
        self.snapshot.owners.iter().all(|(key, owner)| match owner {
            OwnerRecord::Parameter(parameter) => match parameter.parent {
                super::ParameterParent::Function(_) => {
                    parameter.use_mode == ParameterUse::Unrestricted
                        && !self.contains_resource(parameter.ty, &mut BTreeSet::new())
                }
                super::ParameterParent::Operation(operation) => {
                    let direct = self.resource_type(parameter.ty, &mut BTreeSet::new());
                    match direct {
                        Some((Shape::Direct, interface)) => {
                            parameter.use_mode != ParameterUse::Unrestricted
                                && self.local_operation(operation).is_some_and(|record| {
                                    interface.package == self.snapshot.root.package_id
                                        && interface.declaration == record.declaration
                                })
                        }
                        _ => {
                            !self.contains_resource(parameter.ty, &mut BTreeSet::new())
                                && parameter.use_mode == ParameterUse::Unrestricted
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
                }
                DeclarationPayload::Function(function) => {
                    !self.contains_resource(function.result, &mut BTreeSet::new())
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
            | ExpressionOperation::Constant { .. }
            | ExpressionOperation::FunctionValue { .. } => Ok(Value::Plain),
            ExpressionOperation::Local { value } => {
                if lexical(*value).is_some_and(|binding| live.contains_key(&binding)) {
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
                            || live.insert(*binding, Slot { right, live: true }).is_some()
                        {
                            return Err(());
                        }
                        scoped.push(*binding);
                    }
                }
                let result = self.eval(*body, live)?;
                for binding in scoped {
                    live.remove(&binding);
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
            ExpressionOperation::Call { arguments, .. } => {
                for argument in arguments {
                    self.plain(*argument, live)?;
                }
                Ok(Value::Plain)
            }
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
                        if branch
                            .insert(
                                binding,
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
                        scoped = Some(binding);
                    }
                    let value = self.eval(arm.body, &mut branch)?;
                    if let Some(binding) = scoped {
                        branch.remove(&binding);
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
        let binding = lexical(value).ok_or(())?;
        let slot = live.get_mut(&binding).ok_or(())?;
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
            && let Some(binding) = lexical(value)
            && let Some(slot) = live.get_mut(&binding)
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

fn lexical(value: LocalValueReference) -> Option<BindingId> {
    match value {
        LocalValueReference::LexicalBinding(binding)
        | LocalValueReference::MatchPayload(binding) => Some(binding),
        _ => None,
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

type SnapshotMutation = (&'static str, fn(&mut KernelSnapshot));

#[test]
fn independent_reference_agrees_on_maintained_graphs_and_finite_negative_corpus() {
    let standard = maintained_snapshot("packages/standard");
    let journal = maintained_snapshot("applications/lkjournal");
    for (name, snapshot) in [("standard", &standard), ("lkjournal", &journal)] {
        assert!(production_accepts(snapshot), "production accepts {name}");
        assert!(Reference { snapshot }.accepts(), "reference accepts {name}");
    }

    let journal_mutations: [SnapshotMutation; 7] = [
        ("fabricated resource", mutate_fabricated_resource),
        ("post-consume escape", mutate_post_consume_escape),
        ("duplicate consume", mutate_duplicate_consume),
        ("resource invocation", mutate_invoke_resource),
        ("foreign requirement", mutate_foreign_requirement),
        ("branch join", mutate_branch_join),
        ("function escape", mutate_function_escape),
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
