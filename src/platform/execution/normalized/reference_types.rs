//! Independent canonical-owner derivation of composite instantiated type objects.
//! This does not inspect bytecode, compiler tables, or production preparation.

use super::reference_schema::NormalizedReferenceSchema;
use crate::platform::execution::ExecutionError;
use crate::platform::kernel::{
    DeclarationPayload, DeclarationReference, ExpressionOperation, KernelSnapshot, OwnerKey,
    OwnerRecord, PackageId, TypeForm, TypeObject, TypeObjectDigest, encode_type_object,
};
use crate::platform::semantic_id::{ExpressionId, TypeParameterId};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

type Bindings = BTreeMap<TypeParameterId, TypeObjectDigest>;
type Calls = VecDeque<(DeclarationReference, Vec<TypeObjectDigest>)>;

struct Closure<'a> {
    snapshots: BTreeMap<PackageId, &'a KernelSnapshot>,
    types: &'a mut BTreeMap<TypeObjectDigest, TypeObject>,
    visits: usize,
    allocated: usize,
    control: &'a crate::platform::execution::ExecutionControl,
}

fn allocate<T>(allocated: &mut usize, count: usize) -> Result<(), ExecutionError> {
    *allocated = count
        .checked_mul(std::mem::size_of::<T>())
        .and_then(|bytes| allocated.checked_add(bytes))
        .filter(|bytes| {
            *bytes as u64 <= super::vm::NormalizedRunPolicy::default().maximum_allocated_bytes
        })
        .ok_or_else(|| {
            ExecutionError::resource(
                "reference_instantiation_storage",
                "canonical type derivation exceeded the existing allocation limit",
            )
        })?;
    Ok(())
}

fn index_node<T>(allocated: &mut usize) -> Result<(), ExecutionError> {
    allocate::<T>(allocated, 1)?;
    allocate::<usize>(allocated, 3)
}

impl Closure<'_> {
    fn tick(&mut self) -> Result<(), ExecutionError> {
        self.control.check()?;
        self.visits = self
            .visits
            .checked_add(1)
            .filter(|n| *n <= crate::platform::kernel::contract::MAXIMUM_VALIDATION_WORK)
            .ok_or_else(|| {
                ExecutionError::resource(
                    "reference_instantiation_work",
                    "canonical type instantiation exceeded its finite work bound",
                )
            })?;
        Ok(())
    }

    fn owner(&mut self, package: PackageId, key: OwnerKey) -> Result<&OwnerRecord, ExecutionError> {
        self.tick()?;
        self.snapshots
            .get(&package)
            .and_then(|snapshot| snapshot.owners.get(&key))
            .ok_or_else(failure)
    }

    fn identity(
        &mut self,
        ty: TypeObjectDigest,
        bindings: &Bindings,
        depth: usize,
    ) -> Result<TypeObjectDigest, ExecutionError> {
        self.tick()?;
        if depth > 256 {
            return Err(failure());
        }
        self.clone_type(ty)?;
        let object = self.types.get(&ty).cloned().ok_or_else(failure)?;
        let form = match object.form {
            TypeForm::TypeParameter { parameter } => {
                return bindings.get(&parameter).copied().ok_or_else(failure);
            }
            TypeForm::Applied {
                declaration,
                arguments,
            } => TypeForm::Applied {
                declaration,
                arguments: arguments
                    .into_iter()
                    .map(|ty| self.identity(ty, bindings, depth + 1))
                    .collect::<Result<_, _>>()?,
            },
            TypeForm::List { item } => TypeForm::List {
                item: self.identity(item, bindings, depth + 1)?,
            },
            TypeForm::Option { item } => TypeForm::Option {
                item: self.identity(item, bindings, depth + 1)?,
            },
            TypeForm::Stream { item } => TypeForm::Stream {
                item: self.identity(item, bindings, depth + 1)?,
            },
            TypeForm::Map { key, value } => TypeForm::Map {
                key: self.identity(key, bindings, depth + 1)?,
                value: self.identity(value, bindings, depth + 1)?,
            },
            TypeForm::Result { ok, error } => TypeForm::Result {
                ok: self.identity(ok, bindings, depth + 1)?,
                error: self.identity(error, bindings, depth + 1)?,
            },
            TypeForm::Function { parameters, result } => TypeForm::Function {
                parameters: parameters
                    .into_iter()
                    .map(|ty| self.identity(ty, bindings, depth + 1))
                    .collect::<Result<_, _>>()?,
                result: self.identity(result, bindings, depth + 1)?,
            },
            TypeForm::StructuralRecord { mut fields } => {
                for field in &mut fields {
                    field.ty = self.identity(field.ty, bindings, depth + 1)?;
                }
                TypeForm::StructuralRecord { fields }
            }
            form => form,
        };
        let object = TypeObject::new(form).map_err(|_| failure())?;
        let (digest, _) = encode_type_object(&object).map_err(|_| failure())?;
        self.tick()?;
        if !self.types.contains_key(&digest) {
            index_node::<(TypeObjectDigest, TypeObject)>(&mut self.allocated)?;
        }
        self.types.entry(digest).or_insert(object);
        Ok(digest)
    }

    fn body(
        &mut self,
        package: PackageId,
        root: ExpressionId,
        bindings: &Bindings,
        calls: &mut Calls,
    ) -> Result<(), ExecutionError> {
        allocate::<ExpressionId>(&mut self.allocated, 1)?;
        let mut pending = vec![root];
        while let Some(expression) = pending.pop() {
            let OwnerRecord::Expression(record) = self
                .owner(package, OwnerKey::Expression(expression))?
                .clone()
            else {
                return Err(failure());
            };
            let children = record.children();
            let nominal = match &record.operation {
                ExpressionOperation::Record {
                    nominal_type: Some(declaration),
                    type_arguments,
                    ..
                } => Some((*declaration, type_arguments)),
                ExpressionOperation::Variant {
                    case,
                    type_arguments,
                    ..
                } => {
                    let OwnerRecord::Case(owner) =
                        self.owner(case.package, OwnerKey::Case(case.case))?
                    else {
                        return Err(failure());
                    };
                    Some((
                        DeclarationReference {
                            package: case.package,
                            declaration: owner.declaration,
                        },
                        type_arguments,
                    ))
                }
                _ => None,
            };
            if let Some((declaration, arguments)) = nominal {
                allocate::<TypeObjectDigest>(&mut self.allocated, arguments.len())?;
                let arguments = arguments
                    .iter()
                    .map(|ty| self.identity(*ty, bindings, 0))
                    .collect::<Result<Vec<_>, _>>()?;
                let form = if arguments.is_empty() {
                    TypeForm::Named { declaration }
                } else {
                    TypeForm::Applied {
                        declaration,
                        arguments,
                    }
                };
                let object = TypeObject::new(form).map_err(|_| failure())?;
                let (ty, _) = encode_type_object(&object).map_err(|_| failure())?;
                self.tick()?;
                if !self.types.contains_key(&ty) {
                    index_node::<(TypeObjectDigest, TypeObject)>(&mut self.allocated)?;
                }
                self.types.entry(ty).or_insert(object);
            }
            match record.operation {
                ExpressionOperation::Let {
                    bindings: locals, ..
                } => {
                    for local in locals {
                        let OwnerRecord::Binding(binding) =
                            self.owner(package, OwnerKey::Binding(local))?
                        else {
                            return Err(failure());
                        };
                        let value = binding.value.ok_or_else(failure)?;
                        self.tick()?;
                        allocate::<ExpressionId>(&mut self.allocated, 1)?;
                        pending.push(value);
                    }
                }
                ExpressionOperation::Call {
                    function,
                    type_arguments,
                    ..
                }
                | ExpressionOperation::FunctionValue {
                    function,
                    type_arguments,
                } => {
                    allocate::<TypeObjectDigest>(&mut self.allocated, type_arguments.len())?;
                    let mut concrete = Vec::with_capacity(type_arguments.len());
                    for ty in type_arguments {
                        concrete.push(self.identity(ty, bindings, 0)?);
                    }
                    self.tick()?;
                    allocate::<(DeclarationReference, Vec<TypeObjectDigest>)>(
                        &mut self.allocated,
                        1,
                    )?;
                    calls.push_back((function, concrete));
                }
                _ => {}
            }
            for child in children {
                self.tick()?;
                allocate::<ExpressionId>(&mut self.allocated, 1)?;
                pending.push(child.expression);
            }
        }
        Ok(())
    }

    fn clone_type(&mut self, ty: TypeObjectDigest) -> Result<(), ExecutionError> {
        let object = self.types.get(&ty).ok_or_else(failure)?;
        match &object.form {
            TypeForm::StructuralRecord { fields } => {
                allocate::<crate::platform::kernel::StructuralTypeField>(
                    &mut self.allocated,
                    fields.len(),
                )?;
                for field in fields {
                    allocate::<u8>(&mut self.allocated, field.name.as_str().len())?;
                }
            }
            TypeForm::Applied { arguments, .. } => {
                allocate::<TypeObjectDigest>(&mut self.allocated, arguments.len())?
            }
            TypeForm::Function { parameters, .. } => {
                allocate::<TypeObjectDigest>(&mut self.allocated, parameters.len())?
            }
            _ => {}
        }
        Ok(())
    }
}

fn failure() -> ExecutionError {
    ExecutionError::new(
        crate::platform::execution::ExecutionFailureClass::Infrastructure,
        "reference_instantiation_scope",
        "canonical instantiation has a missing owner, type, or exact substitution",
    )
}

pub(super) fn complete(
    schema: &mut NormalizedReferenceSchema,
    snapshots: &[&KernelSnapshot],
    control: &crate::platform::execution::ExecutionControl,
) -> Result<(), ExecutionError> {
    let mut closure = Closure {
        snapshots: snapshots
            .iter()
            .map(|snapshot| (snapshot.root.package_id, *snapshot))
            .collect(),
        types: &mut schema.types,
        visits: 0,
        allocated: 0,
        control,
    };
    let mut calls = VecDeque::new();
    let empty = Bindings::new();
    for snapshot in snapshots {
        let package = snapshot.root.package_id;
        for (key, record) in &snapshot.owners {
            closure.tick()?;
            if let (OwnerKey::Declaration(id), OwnerRecord::Declaration(declaration)) =
                (key, record)
            {
                let generic = match &declaration.payload {
                    DeclarationPayload::Function(function) => {
                        Some(!function.type_parameters.is_empty())
                    }
                    DeclarationPayload::External(function) => {
                        Some(!function.type_parameters.is_empty())
                    }
                    _ => None,
                };
                if let Some(generic) = generic {
                    if !generic {
                        allocate::<(DeclarationReference, Vec<TypeObjectDigest>)>(
                            &mut closure.allocated,
                            1,
                        )?;
                        calls.push_back((
                            DeclarationReference {
                                package,
                                declaration: *id,
                            },
                            Vec::new(),
                        ));
                    }
                    continue;
                }
            }
            // Let-binding children belong to their containing function context.
            if matches!(record, OwnerRecord::Declaration(_) | OwnerRecord::Port(_)) {
                for root in record.expression_roots() {
                    closure.body(package, root, &empty, &mut calls)?;
                }
            }
        }
    }
    let mut completed = BTreeSet::new();
    while let Some((function, arguments)) = calls.pop_front() {
        closure.tick()?;
        index_node::<(DeclarationReference, Vec<TypeObjectDigest>)>(&mut closure.allocated)?;
        allocate::<TypeObjectDigest>(&mut closure.allocated, arguments.len())?;
        if !completed.insert((function, arguments.clone())) {
            continue;
        }
        let OwnerRecord::Declaration(declaration) = closure
            .owner(
                function.package,
                OwnerKey::Declaration(function.declaration),
            )?
            .clone()
        else {
            return Err(failure());
        };
        let (parameters, types, result, body) = match declaration.payload {
            DeclarationPayload::Function(function) => (
                function.parameters,
                function.type_parameters,
                function.result,
                Some(function.body),
            ),
            DeclarationPayload::External(function) => (
                function.parameters,
                function.type_parameters,
                function.result,
                None,
            ),
            _ => return Err(failure()),
        };
        if types.len() != arguments.len() {
            return Err(failure());
        }
        for _ in &types {
            closure.tick()?;
            index_node::<(TypeParameterId, TypeObjectDigest)>(&mut closure.allocated)?;
        }
        let bindings = types.into_iter().zip(arguments).collect();
        closure.identity(result, &bindings, 0)?;
        for parameter in parameters {
            let OwnerRecord::Parameter(parameter) = closure
                .owner(function.package, OwnerKey::Parameter(parameter))?
                .clone()
            else {
                return Err(failure());
            };
            closure.identity(parameter.ty, &bindings, 0)?;
        }
        if let Some(body) = body {
            closure.body(function.package, body, &bindings, &mut calls)?;
        }
    }
    closure.nominals(
        &mut schema.records,
        &mut schema.variants,
        &mut schema.record_instances,
        &mut schema.variant_instances,
    )?;
    allocate::<bool>(&mut closure.allocated, schema.variants.len())?;
    schema.affine_variants.resize(schema.variants.len(), false);
    let mut visits = closure.visits;
    let mut bytes = closure.allocated;
    schema.capture_safe_types =
        property_types(schema, &mut visits, &mut bytes, control, Retention::Capture)?;
    schema.ordinary_types = property_types(
        schema,
        &mut visits,
        &mut bytes,
        control,
        Retention::Ordinary,
    )?;
    schema.comparable_types = property_types(
        schema,
        &mut visits,
        &mut bytes,
        control,
        Retention::Comparable,
    )?;
    schema.application_free_types = property_types(
        schema,
        &mut visits,
        &mut bytes,
        control,
        Retention::NoApplication,
    )?;
    schema.type_derivation_steps = visits as u64;
    schema.type_metadata_bytes = bytes as u64;
    Ok(())
}

impl Closure<'_> {
    // This derivation reads canonical declaration and member owners directly. It neither copies
    // production instance tables nor uses compiled parameter, member, or constraint layouts.
    fn nominals(
        &mut self,
        records: &mut Vec<super::prepare::NormalizedRecordLayout>,
        variants: &mut Vec<super::prepare::NormalizedVariantLayout>,
        record_instances: &mut BTreeMap<TypeObjectDigest, usize>,
        variant_instances: &mut BTreeMap<TypeObjectDigest, usize>,
    ) -> Result<(), ExecutionError> {
        for (index, record) in records.iter().enumerate() {
            self.tick()?;
            if record.type_parameters.is_empty() {
                let object = TypeObject::new(TypeForm::Named {
                    declaration: record.declaration,
                })
                .map_err(|_| failure())?;
                let (ty, _) = encode_type_object(&object).map_err(|_| failure())?;
                index_node::<(TypeObjectDigest, usize)>(&mut self.allocated)?;
                record_instances.insert(ty, index);
                if !self.types.contains_key(&ty) {
                    index_node::<(TypeObjectDigest, TypeObject)>(&mut self.allocated)?;
                }
                self.types.entry(ty).or_insert(object);
            }
        }
        for (index, variant) in variants.iter().enumerate() {
            self.tick()?;
            if variant.type_parameters.is_empty() {
                let object = TypeObject::new(TypeForm::Named {
                    declaration: variant.declaration,
                })
                .map_err(|_| failure())?;
                let (ty, _) = encode_type_object(&object).map_err(|_| failure())?;
                index_node::<(TypeObjectDigest, usize)>(&mut self.allocated)?;
                variant_instances.insert(ty, index);
                if !self.types.contains_key(&ty) {
                    index_node::<(TypeObjectDigest, TypeObject)>(&mut self.allocated)?;
                }
                self.types.entry(ty).or_insert(object);
            }
        }
        for _ in self.types.keys() {
            allocate::<(TypeObjectDigest, usize)>(&mut self.allocated, 1)?;
        }
        let mut queue = self
            .types
            .keys()
            .map(|ty| (*ty, 0usize))
            .collect::<VecDeque<_>>();
        let mut inspected = BTreeSet::new();
        let mut additions_record = BTreeMap::new();
        let mut additions_variant = BTreeMap::new();
        while let Some((ty, depth)) = queue.pop_front() {
            self.tick()?;
            if depth > crate::platform::kernel::contract::MAXIMUM_TYPE_DEPTH {
                return Err(failure());
            }
            if inspected.contains(&ty) {
                continue;
            }
            index_node::<TypeObjectDigest>(&mut self.allocated)?;
            inspected.insert(ty);
            self.clone_type(ty)?;
            let object = self.types.get(&ty).cloned().ok_or_else(failure)?;
            for child in object.child_types() {
                self.tick()?;
                allocate::<(TypeObjectDigest, usize)>(&mut self.allocated, 1)?;
                queue.push_back((child, depth + 1));
            }
            let TypeForm::Applied {
                declaration,
                arguments,
            } = object.form
            else {
                continue;
            };
            let OwnerRecord::Declaration(owner) = self
                .owner(
                    declaration.package,
                    OwnerKey::Declaration(declaration.declaration),
                )?
                .clone()
            else {
                return Err(failure());
            };
            allocate::<TypeParameterId>(
                &mut self.allocated,
                owner.payload.type_parameters().len(),
            )?;
            let parameters = owner.payload.type_parameters().to_vec();
            if parameters.len() != arguments.len() || parameters.is_empty() {
                return Err(failure());
            }
            let mut constraints = Vec::new();
            let mut bindings = BTreeMap::new();
            for (parameter, argument) in parameters.iter().zip(&arguments) {
                let OwnerRecord::TypeParameter(record) =
                    self.owner(declaration.package, OwnerKey::TypeParameter(*parameter))?
                else {
                    return Err(failure());
                };
                if record.declaration != declaration.declaration {
                    return Err(failure());
                }
                let constraint = record.constraints;
                allocate::<crate::platform::kernel::TypeParameterConstraints>(
                    &mut self.allocated,
                    1,
                )?;
                index_node::<(TypeParameterId, TypeObjectDigest)>(&mut self.allocated)?;
                constraints.push(constraint);
                bindings.insert(*parameter, *argument);
            }
            match owner.payload {
                DeclarationPayload::Record { fields, .. } => {
                    let mut layout = Vec::new();
                    for field in fields {
                        let OwnerRecord::Field(member) = self
                            .owner(declaration.package, OwnerKey::Field(field))?
                            .clone()
                        else {
                            return Err(failure());
                        };
                        if member.declaration != declaration.declaration {
                            return Err(failure());
                        }
                        allocate::<u8>(&mut self.allocated, member.name.as_str().len())?;
                        allocate::<super::prepare::NormalizedRecordField>(&mut self.allocated, 1)?;
                        allocate::<(TypeObjectDigest, usize)>(&mut self.allocated, 1)?;
                        let field_type = self.identity(member.ty, &bindings, 0)?;
                        queue.push_back((field_type, depth + 1));
                        layout.push(super::prepare::NormalizedRecordField {
                            reference: crate::platform::kernel::FieldReference {
                                package: declaration.package,
                                field,
                            },
                            name: member.name,
                            ty: field_type,
                        });
                    }
                    index_node::<(TypeObjectDigest, super::prepare::NormalizedRecordLayout)>(
                        &mut self.allocated,
                    )?;
                    additions_record.insert(
                        ty,
                        super::prepare::NormalizedRecordLayout {
                            declaration,
                            type_parameters: parameters.into(),
                            type_parameter_constraints: constraints.into(),
                            arguments: arguments.into(),
                            fields: layout.into(),
                        },
                    );
                }
                DeclarationPayload::Variant { cases, .. } => {
                    let mut layout = Vec::new();
                    for case in cases {
                        let OwnerRecord::Case(member) = self
                            .owner(declaration.package, OwnerKey::Case(case))?
                            .clone()
                        else {
                            return Err(failure());
                        };
                        if member.declaration != declaration.declaration {
                            return Err(failure());
                        }
                        allocate::<u8>(&mut self.allocated, member.name.as_str().len())?;
                        allocate::<super::prepare::NormalizedVariantCase>(&mut self.allocated, 1)?;
                        let payload = member
                            .payload
                            .map(|ty| self.identity(ty, &bindings, 0))
                            .transpose()?;
                        if let Some(ty) = payload {
                            allocate::<(TypeObjectDigest, usize)>(&mut self.allocated, 1)?;
                            queue.push_back((ty, depth + 1));
                        }
                        layout.push(super::prepare::NormalizedVariantCase {
                            reference: crate::platform::kernel::CaseReference {
                                package: declaration.package,
                                case,
                            },
                            name: member.name,
                            payload,
                        });
                    }
                    index_node::<(TypeObjectDigest, super::prepare::NormalizedVariantLayout)>(
                        &mut self.allocated,
                    )?;
                    additions_variant.insert(
                        ty,
                        super::prepare::NormalizedVariantLayout {
                            declaration,
                            type_parameters: parameters.into(),
                            type_parameter_constraints: constraints.into(),
                            arguments: arguments.into(),
                            cases: layout.into(),
                        },
                    );
                }
                _ => return Err(failure()),
            }
        }
        for (ty, layout) in additions_record {
            self.tick()?;
            index_node::<(TypeObjectDigest, usize)>(&mut self.allocated)?;
            allocate::<super::prepare::NormalizedRecordLayout>(&mut self.allocated, 1)?;
            record_instances.insert(ty, records.len());
            records.push(layout);
        }
        for (ty, layout) in additions_variant {
            self.tick()?;
            index_node::<(TypeObjectDigest, usize)>(&mut self.allocated)?;
            allocate::<super::prepare::NormalizedVariantLayout>(&mut self.allocated, 1)?;
            variant_instances.insert(ty, variants.len());
            variants.push(layout);
        }
        Ok(())
    }
}

// Independent greatest fixed point over canonical stored edges. Function signatures have
// no stored edges. Cycles admitted by canonical validation retain their ordinary meaning.
#[derive(Clone, Copy, Eq, PartialEq)]
enum Retention {
    Capture,
    Ordinary,
    Comparable,
    NoApplication,
}

fn property_types(
    schema: &NormalizedReferenceSchema,
    visits: &mut usize,
    allocated: &mut usize,
    control: &crate::platform::execution::ExecutionControl,
    retention: Retention,
) -> Result<BTreeSet<TypeObjectDigest>, ExecutionError> {
    let callables = retention != Retention::Comparable;
    let secrets = retention == Retention::Ordinary;
    fn tick(visits: &mut usize) -> Result<(), ExecutionError> {
        *visits = visits
            .checked_add(1)
            .filter(|n| *n <= crate::platform::kernel::contract::MAXIMUM_VALIDATION_WORK)
            .ok_or_else(|| {
                ExecutionError::resource(
                    "reference_constraint_work",
                    "canonical capture proof exceeded validation work",
                )
            })?;
        Ok(())
    }
    let mut safe = BTreeSet::new();
    for (ty, object) in &schema.types {
        control.check()?;
        tick(visits)?;
        if retention == Retention::NoApplication
            || !matches!(
                object.form,
                TypeForm::Stream { .. }
                    | TypeForm::CapabilityResource { .. }
                    | TypeForm::TypeParameter { .. }
            ) && (secrets || !matches!(object.form, TypeForm::Secret))
                && (callables || !matches!(object.form, TypeForm::Function { .. }))
        {
            tick(visits)?;
            index_node::<TypeObjectDigest>(allocated)?;
            safe.insert(*ty);
        }
    }
    loop {
        let before = safe.len();
        for (ty, object) in &schema.types {
            control.check()?;
            tick(visits)?;
            if !safe.contains(ty) {
                continue;
            }
            let mut retained = |child: TypeObjectDigest| -> Result<bool, ExecutionError> {
                tick(visits)?;
                if !schema.types.contains_key(&child) {
                    return Err(failure());
                }
                Ok(safe.contains(&child))
            };
            let accepted = match &object.form {
                TypeForm::Applied { .. } if retention == Retention::NoApplication => false,
                TypeForm::Secret
                | TypeForm::Stream { .. }
                | TypeForm::CapabilityResource { .. }
                | TypeForm::TypeParameter { .. }
                    if retention == Retention::NoApplication =>
                {
                    true
                }
                TypeForm::List { item } | TypeForm::Option { item } => retained(*item)?,
                TypeForm::Map { key, value }
                | TypeForm::Result {
                    ok: key,
                    error: value,
                } => retained(*key)? & retained(*value)?,
                TypeForm::StructuralRecord { fields } => {
                    let mut accepted = true;
                    for field in fields {
                        accepted &= retained(field.ty)?;
                    }
                    accepted
                }
                TypeForm::Named { .. } | TypeForm::Applied { .. } => {
                    let mut accepted = true;
                    if let Some(index) = schema.record_instances.get(ty).copied() {
                        for argument in schema.records[index].arguments.iter() {
                            accepted &= retained(*argument)?;
                        }
                        for field in schema.records[index].fields.iter() {
                            accepted &= retained(field.ty)?;
                        }
                    } else if let Some(index) = schema.variant_instances.get(ty).copied() {
                        for argument in schema.variants[index].arguments.iter() {
                            accepted &= retained(*argument)?;
                        }
                        for case in schema.variants[index].cases.iter() {
                            tick(visits)?;
                            if let Some(payload) = case.payload {
                                tick(visits)?;
                                if !schema.types.contains_key(&payload) {
                                    return Err(failure());
                                }
                                accepted &= safe.contains(&payload);
                            }
                        }
                    } else {
                        return Err(failure());
                    }
                    accepted
                }
                TypeForm::Unit
                | TypeForm::Bool
                | TypeForm::I64
                | TypeForm::Bytes
                | TypeForm::Text
                | TypeForm::StaticText => true,
                TypeForm::Function { .. } => callables,
                TypeForm::Secret => secrets,
                _ => false,
            };
            if !accepted {
                safe.remove(ty);
            }
        }
        if safe.len() == before {
            return Ok(safe);
        }
    }
}
