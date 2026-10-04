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
type Application = (
    DeclarationReference,
    Vec<TypeObjectDigest>,
    Vec<crate::platform::kernel::EffectRow>,
    Vec<crate::platform::kernel::RequirementReference>,
);
type Calls = VecDeque<Application>;
const MAXIMUM_METADATA_BYTES: usize = 256 * 1024 * 1024;

struct Closure<'a> {
    parallel_targets: BTreeSet<(
        DeclarationReference,
        Vec<TypeObjectDigest>,
        Option<DeclarationReference>,
    )>,
    constrained_applications: BTreeSet<(
        DeclarationReference,
        Vec<TypeObjectDigest>,
        Option<DeclarationReference>,
    )>,
    symbolic: bool,
    snapshots: BTreeMap<PackageId, &'a KernelSnapshot>,
    types: &'a mut BTreeMap<TypeObjectDigest, TypeObject>,
    visits: usize,
    allocated: usize,
    control: &'a crate::platform::execution::ExecutionControl,
    effects: super::reference_effects::Bindings,
    requirements: super::reference_effects::RequirementBindings,
}

fn allocate<T>(allocated: &mut usize, count: usize) -> Result<(), ExecutionError> {
    *allocated = count
        .checked_mul(std::mem::size_of::<T>())
        .and_then(|bytes| allocated.checked_add(bytes))
        .filter(|bytes| *bytes <= MAXIMUM_METADATA_BYTES)
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
    fn scoped_owned(
        &mut self,
        ty: TypeObjectDigest,
        scope: Option<DeclarationReference>,
    ) -> Result<bool, ExecutionError> {
        self.tick()?;
        match self.types.get(&ty).ok_or_else(failure)?.form {
            TypeForm::ByteBuffer
            | TypeForm::OwnedI64Cell
            | TypeForm::OwnedProduct { .. }
            | TypeForm::OwnedChoice { .. } => Ok(true),
            TypeForm::TypeParameter { parameter } => {
                let declaration = scope.ok_or_else(failure)?;
                let OwnerRecord::TypeParameter(record) =
                    self.owner(declaration.package, OwnerKey::TypeParameter(parameter))?
                else {
                    return Err(failure());
                };
                if record.declaration != declaration.declaration {
                    return Err(failure());
                }
                Ok(record.constraints.has_owned())
            }
            _ => Ok(false),
        }
    }

    fn parallel_witnesses(
        &mut self,
        function: &crate::platform::kernel::FunctionDeclaration,
        child_bindings: &Bindings,
        operands: &[crate::platform::kernel::ImplementationOperand],
        scope: Option<DeclarationReference>,
        caller_bindings: &Bindings,
    ) -> Result<(), ExecutionError> {
        for (formal, operand) in function.implementation_parameters.iter().zip(operands) {
            self.tick()?;
            let expected = self.identity(formal.self_type, child_bindings, 0)?;
            let (contract, actual) = match *operand {
                crate::platform::kernel::ImplementationOperand::Concrete { implementation } => {
                    let OwnerRecord::Declaration(owner) = self
                        .owner(
                            implementation.package,
                            OwnerKey::Declaration(implementation.declaration),
                        )?
                        .clone()
                    else {
                        return Err(failure());
                    };
                    let DeclarationPayload::OwnedImplementation(selected) = owner.payload else {
                        return Err(failure());
                    };
                    (selected.contract, selected.self_type)
                }
                crate::platform::kernel::ImplementationOperand::Parameter {
                    function,
                    parameter,
                } => {
                    if scope != Some(function) {
                        return Err(failure());
                    }
                    let OwnerRecord::Declaration(owner) = self
                        .owner(
                            function.package,
                            OwnerKey::Declaration(function.declaration),
                        )?
                        .clone()
                    else {
                        return Err(failure());
                    };
                    let DeclarationPayload::Function(parent) = owner.payload else {
                        return Err(failure());
                    };
                    let formal = parent
                        .implementation_parameters
                        .iter()
                        .find(|formal| formal.id == parameter)
                        .ok_or_else(failure)?;
                    (
                        formal.contract,
                        self.identity(formal.self_type, caller_bindings, 0)?,
                    )
                }
            };
            if contract != formal.contract || actual != expected {
                return Err(failure());
            }
        }
        Ok(())
    }

    // Independently propagate the greatest reached depth from every product root.
    // A substitution may add depth to an otherwise valid symbolic product type.
    fn product_depths(&mut self) -> Result<(), ExecutionError> {
        let mut queue = VecDeque::new();
        self.visits = self
            .visits
            .checked_add(self.types.len())
            .filter(|n| *n <= crate::platform::kernel::contract::MAXIMUM_VALIDATION_WORK)
            .ok_or_else(|| {
                ExecutionError::resource(
                    "reference_instantiation_work",
                    "product type inventory exceeded its finite work bound",
                )
            })?;
        for (ty, object) in self.types.iter() {
            self.control.check()?;
            if matches!(
                object.form,
                TypeForm::OwnedProduct { .. } | TypeForm::OwnedChoice { .. }
            ) {
                allocate::<(TypeObjectDigest, usize)>(&mut self.allocated, 1)?;
                queue.push_back((*ty, 0usize));
            }
        }
        let mut depths = BTreeMap::new();
        while let Some((ty, depth)) = queue.pop_front() {
            self.tick()?;
            if depth > crate::platform::kernel::contract::MAXIMUM_TYPE_DEPTH {
                return Err(ExecutionError::new(
                    crate::platform::execution::ExecutionFailureClass::Infrastructure,
                    "reference_product_depth",
                    "closed owned product exceeds the structural type depth bound",
                ));
            }
            if depths.get(&ty).is_some_and(|previous| *previous >= depth) {
                continue;
            }
            if !depths.contains_key(&ty) {
                index_node::<(TypeObjectDigest, usize)>(&mut self.allocated)?;
            }
            depths.insert(ty, depth);
            let object = self.types.get(&ty).ok_or_else(failure)?;
            allocate::<TypeObjectDigest>(&mut self.allocated, object.child_type_count())?;
            allocate::<(TypeObjectDigest, usize)>(&mut self.allocated, object.child_type_count())?;
            for child in object.child_types() {
                self.tick()?;
                queue.push_back((child, depth + 1));
            }
        }
        Ok(())
    }

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
            TypeForm::TaskFunction {
                parameters,
                result,
                effect,
            } => {
                let effect = if self.symbolic {
                    effect
                } else {
                    super::reference_effects::close(
                        &effect,
                        &self.effects,
                        &self.requirements,
                        |n| {
                            allocate::<(crate::platform::kernel::RequirementReference, usize)>(
                                &mut self.allocated,
                                n,
                            )?;
                            self.control.check()
                        },
                    )?
                };
                TypeForm::TaskFunction {
                    parameters: parameters
                        .into_iter()
                        .map(|ty| self.identity(ty, bindings, depth + 1))
                        .collect::<Result<_, _>>()?,
                    result: self.identity(result, bindings, depth + 1)?,
                    effect,
                }
            }
            TypeForm::OwnedChoice { mut cases } => {
                for case in &mut cases {
                    case.ty = self.identity(case.ty, bindings, depth + 1)?;
                }
                TypeForm::OwnedChoice { cases }
            }
            TypeForm::OwnedProduct { mut fields } => {
                for field in &mut fields {
                    field.ty = self.identity(field.ty, bindings, depth + 1)?;
                }
                TypeForm::OwnedProduct { fields }
            }
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
        task_context: bool,
        scope: Option<DeclarationReference>,
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
            let nominal: Option<(_, &[TypeObjectDigest])> = match &record.operation {
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
                ExpressionOperation::TransactionOutcome {
                    outcome,
                    type_argument,
                    ..
                } => Some((outcome.outcome, std::slice::from_ref(type_argument))),
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
                ExpressionOperation::Parallel { left, right } => {
                    if !task_context {
                        return Err(failure());
                    }
                    let mut fields = Vec::new();
                    allocate::<crate::platform::kernel::StructuralTypeField>(
                        &mut self.allocated,
                        2,
                    )?;
                    for (name, child) in [("left", left), ("right", right)] {
                        let OwnerRecord::Expression(child) =
                            self.owner(package, OwnerKey::Expression(child))?.clone()
                        else {
                            return Err(failure());
                        };
                        let (function, type_arguments, implementations, arguments) =
                            match child.operation {
                                ExpressionOperation::Call {
                                    function,
                                    type_arguments,
                                    effect_arguments,
                                    requirement_arguments,
                                    arguments,
                                } if effect_arguments.is_empty()
                                    && requirement_arguments.is_empty() =>
                                {
                                    (function, type_arguments, Vec::new(), arguments)
                                }
                                ExpressionOperation::ImplementationCall {
                                    function,
                                    type_arguments,
                                    effect_arguments,
                                    requirement_arguments,
                                    implementations,
                                    arguments,
                                } if effect_arguments.is_empty()
                                    && requirement_arguments.is_empty() =>
                                {
                                    (function, type_arguments, implementations, arguments)
                                }
                                _ => return Err(failure()),
                            };
                        let OwnerRecord::Declaration(declaration) = self
                            .owner(
                                function.package,
                                OwnerKey::Declaration(function.declaration),
                            )?
                            .clone()
                        else {
                            return Err(failure());
                        };
                        let DeclarationPayload::Function(signature) = declaration.payload else {
                            return Err(failure());
                        };
                        if signature.type_parameters.len() != type_arguments.len()
                            || !signature.effect_parameters.is_empty()
                            || !signature.requirement_parameters.is_empty()
                            || signature.implementation_parameters.len() != implementations.len()
                            || !matches!(&signature.effect, crate::platform::kernel::FunctionEffect::Task { requirements, effect_parameters } if requirements.is_empty() && effect_parameters.is_empty())
                            || signature.parameters.len() != arguments.len()
                        {
                            return Err(failure());
                        }
                        allocate::<TypeObjectDigest>(&mut self.allocated, type_arguments.len())?;
                        let mut arguments = Vec::new();
                        for ty in type_arguments {
                            arguments.push(self.identity(ty, bindings, 0)?);
                        }
                        allocate::<(TypeParameterId, TypeObjectDigest)>(
                            &mut self.allocated,
                            arguments.len(),
                        )?;
                        let child_bindings = signature
                            .type_parameters
                            .iter()
                            .copied()
                            .zip(arguments.iter().copied())
                            .collect();
                        self.parallel_witnesses(
                            &signature,
                            &child_bindings,
                            &implementations,
                            scope,
                            bindings,
                        )?;
                        let ty = self.identity(signature.result, &child_bindings, 0)?;
                        for parameter in &signature.parameters {
                            let OwnerRecord::Parameter(parameter) = self
                                .owner(function.package, OwnerKey::Parameter(*parameter))?
                                .clone()
                            else {
                                return Err(failure());
                            };
                            self.identity(parameter.ty, &child_bindings, 0)?;
                        }
                        index_node::<(DeclarationReference, Vec<TypeObjectDigest>)>(
                            &mut self.allocated,
                        )?;
                        self.parallel_targets.insert((
                            function,
                            arguments,
                            if self.symbolic { scope } else { None },
                        ));
                        fields.push(crate::platform::kernel::StructuralTypeField {
                            name: crate::platform::kernel::Name::new(name)
                                .map_err(|_| failure())?,
                            ty,
                        });
                    }
                    let mut owned = false;
                    for field in &fields {
                        owned |=
                            self.scoped_owned(field.ty, if self.symbolic { scope } else { None })?;
                    }
                    let object = TypeObject::new(if owned {
                        TypeForm::OwnedProduct { fields }
                    } else {
                        TypeForm::StructuralRecord { fields }
                    })
                    .map_err(|_| failure())?;
                    let (digest, _) = encode_type_object(&object).map_err(|_| failure())?;
                    index_node::<(TypeObjectDigest, TypeObject)>(&mut self.allocated)?;
                    self.types.entry(digest).or_insert(object);
                }
                ExpressionOperation::PackOwned { product_type, .. }
                | ExpressionOperation::UnpackOwned { product_type, .. }
                | ExpressionOperation::BorrowOwnedField { product_type, .. }
                | ExpressionOperation::ChooseOwned {
                    choice_type: product_type,
                    ..
                }
                | ExpressionOperation::MatchOwned {
                    choice_type: product_type,
                    ..
                }
                | ExpressionOperation::MatchBorrowedOwned {
                    choice_type: product_type,
                    ..
                } => {
                    self.identity(product_type, bindings, 0)?;
                }
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
                    effect_arguments,
                    requirement_arguments,
                    ..
                }
                | ExpressionOperation::ImplementationCall {
                    function,
                    type_arguments,
                    effect_arguments,
                    requirement_arguments,
                    ..
                }
                | ExpressionOperation::FunctionValue {
                    effect_arguments,
                    requirement_arguments,
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
                    allocate::<crate::platform::kernel::EffectRow>(
                        &mut self.allocated,
                        effect_arguments.len(),
                    )?;
                    index_node::<(
                        DeclarationReference,
                        Vec<TypeObjectDigest>,
                        Option<DeclarationReference>,
                    )>(&mut self.allocated)?;
                    allocate::<TypeObjectDigest>(&mut self.allocated, concrete.len())?;
                    self.constrained_applications.insert((
                        function,
                        concrete.clone(),
                        if self.symbolic { scope } else { None },
                    ));
                    let mut effects = Vec::with_capacity(effect_arguments.len());
                    for row in effect_arguments {
                        if self.symbolic {
                            continue;
                        }
                        self.tick()?;
                        effects.push(super::reference_effects::close(
                            &row,
                            &self.effects,
                            &self.requirements,
                            |n| {
                                allocate::<(crate::platform::kernel::RequirementReference, usize)>(
                                    &mut self.allocated,
                                    n,
                                )
                            },
                        )?);
                    }
                    allocate::<crate::platform::kernel::RequirementReference>(
                        &mut self.allocated,
                        requirement_arguments.len(),
                    )?;
                    let requirements = requirement_arguments
                        .iter()
                        .filter(|_| !self.symbolic)
                        .map(|operand| {
                            self.control.check()?;
                            super::reference_effects::resolve(*operand, &self.requirements)
                        })
                        .collect::<Result<Vec<_>, _>>()?;
                    if !self.symbolic {
                        calls.push_back((function, concrete, effects, requirements));
                    }
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
            TypeForm::StructuralRecord { fields }
            | TypeForm::OwnedProduct { fields }
            | TypeForm::OwnedChoice { cases: fields } => {
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
            TypeForm::TaskFunction {
                parameters, effect, ..
            } => {
                allocate::<TypeObjectDigest>(&mut self.allocated, parameters.len())?;
                allocate::<crate::platform::kernel::RequirementReference>(
                    &mut self.allocated,
                    effect.requirements.len(),
                )?;
                allocate::<crate::platform::kernel::EffectParameterReference>(
                    &mut self.allocated,
                    effect.parameters.len(),
                )?;
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

#[cfg(test)]
pub(super) fn check_product_substitution(
    mut types: BTreeMap<TypeObjectDigest, TypeObject>,
    ty: TypeObjectDigest,
    bindings: Bindings,
) -> Result<(), ExecutionError> {
    let control = crate::platform::execution::ExecutionControl::uncancelled();
    let mut closure = Closure {
        parallel_targets: BTreeSet::new(),
        constrained_applications: BTreeSet::new(),
        symbolic: false,
        snapshots: BTreeMap::new(),
        types: &mut types,
        visits: 0,
        allocated: 0,
        control: &control,
        effects: BTreeMap::new(),
        requirements: BTreeMap::new(),
    };
    closure.identity(ty, &bindings, 0)?;
    closure.product_depths()
}

pub(super) fn complete(
    schema: &mut NormalizedReferenceSchema,
    snapshots: &[&KernelSnapshot],
    control: &crate::platform::execution::ExecutionControl,
) -> Result<(), ExecutionError> {
    let mut closure = Closure {
        parallel_targets: BTreeSet::new(),
        constrained_applications: BTreeSet::new(),
        symbolic: false,
        snapshots: snapshots
            .iter()
            .map(|snapshot| (snapshot.root.package_id, *snapshot))
            .collect(),
        types: &mut schema.types,
        visits: 0,
        allocated: 0,
        control,
        effects: BTreeMap::new(),
        requirements: BTreeMap::new(),
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
                    DeclarationPayload::Function(function) => Some(
                        !function.type_parameters.is_empty()
                            || !function.effect_parameters.is_empty()
                            || !function.requirement_parameters.is_empty(),
                    ),
                    DeclarationPayload::External(function) => {
                        Some(!function.type_parameters.is_empty())
                    }
                    _ => None,
                };
                if let Some(generic) = generic {
                    if generic {
                        if let DeclarationPayload::Function(function) = &declaration.payload {
                            let scope = DeclarationReference {
                                package,
                                declaration: *id,
                            };
                            let mut bindings = Bindings::new();
                            for parameter in &function.type_parameters {
                                closure.tick()?;
                                let object = TypeObject::new(TypeForm::TypeParameter {
                                    parameter: *parameter,
                                })
                                .map_err(|_| failure())?;
                                let (ty, _) = encode_type_object(&object).map_err(|_| failure())?;
                                index_node::<(TypeObjectDigest, TypeObject)>(
                                    &mut closure.allocated,
                                )?;
                                closure.types.entry(ty).or_insert(object);
                                index_node::<(TypeParameterId, TypeObjectDigest)>(
                                    &mut closure.allocated,
                                )?;
                                bindings.insert(*parameter, ty);
                            }
                            closure.symbolic = true;
                            closure.identity(function.result, &bindings, 0)?;
                            for parameter in &function.parameters {
                                let OwnerRecord::Parameter(parameter) = closure
                                    .owner(package, OwnerKey::Parameter(*parameter))?
                                    .clone()
                                else {
                                    return Err(failure());
                                };
                                closure.identity(parameter.ty, &bindings, 0)?;
                            }
                            closure.body(
                                package,
                                function.body,
                                &bindings,
                                &mut calls,
                                matches!(
                                    function.effect,
                                    crate::platform::kernel::FunctionEffect::Task { .. }
                                ),
                                Some(scope),
                            )?;
                            closure.symbolic = false;
                        }
                    } else {
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
                            Vec::new(),
                            Vec::new(),
                        ));
                    }
                    continue;
                }
            }
            // Let-binding children belong to their containing function context.
            if matches!(record, OwnerRecord::Declaration(_) | OwnerRecord::Port(_)) {
                for root in record.expression_roots() {
                    let task_context = matches!(record, OwnerRecord::Port(port) if matches!(snapshot.types.get(&port.function_type).map(|t| &t.form), Some(TypeForm::TaskFunction { .. })));
                    closure.body(package, root, &empty, &mut calls, task_context, None)?;
                }
            }
        }
    }
    let mut completed = BTreeSet::new();
    while let Some((function, arguments, effects, requirements)) = calls.pop_front() {
        closure.tick()?;
        index_node::<(DeclarationReference, Vec<TypeObjectDigest>)>(&mut closure.allocated)?;
        allocate::<TypeObjectDigest>(&mut closure.allocated, arguments.len())?;
        allocate::<crate::platform::kernel::EffectRow>(&mut closure.allocated, effects.len())?;
        for row in &effects {
            allocate::<crate::platform::kernel::RequirementReference>(
                &mut closure.allocated,
                row.requirements.len(),
            )?;
        }
        allocate::<crate::platform::kernel::RequirementReference>(
            &mut closure.allocated,
            requirements.len(),
        )?;
        if !completed.insert((
            function,
            arguments.clone(),
            effects.clone(),
            requirements.clone(),
        )) {
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
        let task_context = matches!(&declaration.payload, DeclarationPayload::Function(function) if matches!(function.effect, crate::platform::kernel::FunctionEffect::Task { .. }));
        let (parameters, types, effect_parameters, requirement_parameters, result, body) =
            match declaration.payload {
                DeclarationPayload::Function(function) => (
                    function.parameters,
                    function.type_parameters,
                    function.effect_parameters,
                    function.requirement_parameters,
                    function.result,
                    Some(function.body),
                ),
                DeclarationPayload::External(function) => (
                    function.parameters,
                    function.type_parameters,
                    Vec::new(),
                    Vec::new(),
                    function.result,
                    None,
                ),
                _ => return Err(failure()),
            };
        if types.len() != arguments.len()
            || effect_parameters.len() != effects.len()
            || requirement_parameters.len() != requirements.len()
        {
            return Err(failure());
        }
        for _ in &types {
            closure.tick()?;
            index_node::<(TypeParameterId, TypeObjectDigest)>(&mut closure.allocated)?;
        }
        let bindings = types.into_iter().zip(arguments).collect();
        allocate::<(
            crate::platform::kernel::EffectParameterReference,
            crate::platform::kernel::EffectRow,
        )>(&mut closure.allocated, effects.len())?;
        closure.effects = effect_parameters
            .into_iter()
            .zip(effects)
            .map(|(parameter, row)| {
                (
                    crate::platform::kernel::EffectParameterReference {
                        package: function.package,
                        parameter,
                    },
                    row,
                )
            })
            .collect();
        allocate::<(
            crate::platform::kernel::RequirementParameterReference,
            crate::platform::kernel::RequirementReference,
        )>(&mut closure.allocated, requirements.len())?;
        closure.requirements = requirement_parameters
            .into_iter()
            .zip(requirements)
            .map(|(parameter, reference)| {
                (
                    crate::platform::kernel::RequirementParameterReference {
                        package: function.package,
                        parameter,
                    },
                    reference,
                )
            })
            .collect();
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
            closure.body(
                function.package,
                body,
                &bindings,
                &mut calls,
                task_context,
                Some(function),
            )?;
        }
    }
    closure.effects.clear();
    closure.requirements.clear();
    closure.nominals(
        &mut schema.records,
        &mut schema.variants,
        &mut schema.record_instances,
        &mut schema.variant_instances,
    )?;
    closure.product_depths()?;
    let parallel_targets = std::mem::take(&mut closure.parallel_targets);
    let constrained_applications = std::mem::take(&mut closure.constrained_applications);
    allocate::<bool>(&mut closure.allocated, schema.variants.len())?;
    schema.affine_variants.resize(schema.variants.len(), false);
    let mut visits = closure.visits;
    let mut bytes = closure.allocated;
    schema.buffer_free_types = property_types(
        schema,
        &mut visits,
        &mut bytes,
        control,
        Retention::BufferFree,
    )?;
    schema.capture_safe_types =
        property_types(schema, &mut visits, &mut bytes, control, Retention::Capture)?;
    schema.transferable_types = property_types(
        schema,
        &mut visits,
        &mut bytes,
        control,
        Retention::Transfer,
    )?;
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
    // Explicit transfer requirements are independently rechecked for every source
    // application, including unused arguments and unreachable syntax in generic bodies.
    for (target, arguments, scope) in constrained_applications {
        let OwnerRecord::Declaration(owner) =
            canonical_owner(snapshots, target, OwnerKey::Declaration(target.declaration))?
        else {
            return Err(failure());
        };
        let parameters = owner.payload.type_parameters();
        if parameters.len() != arguments.len() {
            return Err(failure());
        }
        for (parameter, actual) in parameters.iter().zip(arguments) {
            control.check()?;
            let OwnerRecord::TypeParameter(record) =
                canonical_owner(snapshots, target, OwnerKey::TypeParameter(*parameter))?
            else {
                return Err(failure());
            };
            if record.declaration != target.declaration {
                return Err(failure());
            }
            if record.constraints.requires_transfer() {
                parallel_transfer_type(
                    schema,
                    snapshots,
                    actual,
                    scope,
                    &mut visits,
                    &mut bytes,
                    control,
                )?;
                if scoped_owned(schema, snapshots, actual, scope)? != record.constraints.has_owned()
                {
                    return Err(failure());
                }
            }
        }
    }
    // Recheck canonical signatures against this independent complete data proof.
    // This includes unused targets and untaken expressions visited by body closure.
    for (target, arguments, scope) in parallel_targets {
        control.check()?;
        visits = visits
            .checked_add(1)
            .filter(|n| *n <= crate::platform::kernel::contract::MAXIMUM_VALIDATION_WORK)
            .ok_or_else(|| {
                ExecutionError::resource(
                    "reference_transfer_work",
                    "canonical transfer proof exhausted its work allowance",
                )
            })?;
        let snapshot = snapshots
            .iter()
            .find(|snapshot| snapshot.root.package_id == target.package)
            .ok_or_else(failure)?;
        let Some(OwnerRecord::Declaration(declaration)) = snapshot
            .owners
            .get(&OwnerKey::Declaration(target.declaration))
        else {
            return Err(failure());
        };
        let DeclarationPayload::Function(function) = &declaration.payload else {
            return Err(failure());
        };
        if function.type_parameters.len() != arguments.len() {
            return Err(failure());
        }
        allocate::<(TypeParameterId, TypeObjectDigest)>(&mut bytes, arguments.len())?;
        let mut bindings = BTreeMap::new();
        for (parameter, actual) in function.type_parameters.iter().zip(arguments) {
            parallel_transfer_type(
                schema,
                snapshots,
                actual,
                scope,
                &mut visits,
                &mut bytes,
                control,
            )?;
            let Some(OwnerRecord::TypeParameter(parameter_record)) =
                snapshot.owners.get(&OwnerKey::TypeParameter(*parameter))
            else {
                return Err(failure());
            };
            let owned = scoped_owned(schema, snapshots, actual, scope)?;
            if parameter_record.declaration != target.declaration
                || owned != parameter_record.constraints.has_owned()
                || bindings.insert(*parameter, actual).is_some()
            {
                return Err(failure());
            }
        }
        let result = if bindings.is_empty() {
            function.result
        } else {
            schema
                .substitute_type(function.result, &bindings, 0)
                .ok_or_else(failure)?
        };
        parallel_transfer_type(
            schema,
            snapshots,
            result,
            scope,
            &mut visits,
            &mut bytes,
            control,
        )?;
        let mut seen_owned = false;
        for parameter in &function.parameters {
            control.check()?;
            visits = visits
                .checked_add(1)
                .filter(|n| *n <= crate::platform::kernel::contract::MAXIMUM_VALIDATION_WORK)
                .ok_or_else(|| {
                    ExecutionError::resource(
                        "reference_transfer_work",
                        "canonical transfer proof exhausted its work allowance",
                    )
                })?;
            let Some(OwnerRecord::Parameter(parameter)) =
                snapshot.owners.get(&OwnerKey::Parameter(*parameter))
            else {
                return Err(failure());
            };
            let ty = if bindings.is_empty() {
                parameter.ty
            } else {
                schema
                    .substitute_type(parameter.ty, &bindings, 0)
                    .ok_or_else(failure)?
            };
            parallel_transfer_type(
                schema,
                snapshots,
                ty,
                scope,
                &mut visits,
                &mut bytes,
                control,
            )?;
            let owned = scoped_owned(schema, snapshots, ty, scope)?;
            if seen_owned && !owned {
                return Err(failure());
            }
            seen_owned |= owned;
            if parameter.parent
                != crate::platform::kernel::ParameterParent::Function(target.declaration)
                || parameter.resource_requirement.is_some()
                || if owned {
                    parameter.use_mode != crate::platform::kernel::ParameterUse::Consume
                } else {
                    parameter.use_mode != crate::platform::kernel::ParameterUse::Unrestricted
                }
            {
                return Err(failure());
            }
        }
    }
    schema.type_derivation_steps = visits as u64;
    schema.type_metadata_bytes = bytes as u64;
    Ok(())
}

#[derive(Clone, Copy, Eq, Ord, PartialEq, PartialOrd)]
struct TransferContext {
    declaration: Option<DeclarationReference>,
    nominal_formals: bool,
}

fn canonical_owner<'a>(
    snapshots: &[&'a KernelSnapshot],
    declaration: DeclarationReference,
    key: OwnerKey,
) -> Result<&'a OwnerRecord, ExecutionError> {
    snapshots
        .iter()
        .find(|snapshot| snapshot.root.package_id == declaration.package)
        .and_then(|snapshot| snapshot.owners.get(&key))
        .ok_or_else(failure)
}

fn scoped_owned(
    schema: &NormalizedReferenceSchema,
    snapshots: &[&KernelSnapshot],
    ty: TypeObjectDigest,
    scope: Option<DeclarationReference>,
) -> Result<bool, ExecutionError> {
    match &schema.types.get(&ty).ok_or_else(failure)?.form {
        TypeForm::ByteBuffer
        | TypeForm::OwnedI64Cell
        | TypeForm::OwnedProduct { .. }
        | TypeForm::OwnedChoice { .. } => Ok(true),
        TypeForm::TypeParameter { parameter } => {
            let declaration = scope.ok_or_else(failure)?;
            let OwnerRecord::TypeParameter(record) =
                canonical_owner(snapshots, declaration, OwnerKey::TypeParameter(*parameter))?
            else {
                return Err(failure());
            };
            if record.declaration != declaration.declaration {
                return Err(failure());
            }
            Ok(record.constraints.has_owned())
        }
        _ => Ok(false),
    }
}

/// Traverse the complete canonical obligation graph. Revisited vertices only suppress
/// duplicate work; success is returned after every reachable argument, field and case
/// has been inspected. Nominal bodies use their own finite formal context, while every
/// actual (including phantoms) remains an obligation in the enclosing context.
fn parallel_transfer_type(
    schema: &NormalizedReferenceSchema,
    snapshots: &[&KernelSnapshot],
    ty: TypeObjectDigest,
    scope: Option<DeclarationReference>,
    visits: &mut usize,
    allocated: &mut usize,
    control: &crate::platform::execution::ExecutionControl,
) -> Result<(), ExecutionError> {
    type Vertex = (TypeObjectDigest, TransferContext, bool);
    control.check()?;
    let mut pending = VecDeque::new();
    allocate::<Vertex>(allocated, 1)?;
    pending.push_back((
        ty,
        TransferContext {
            declaration: scope,
            nominal_formals: false,
        },
        true,
    ));
    let mut seen = BTreeSet::new();
    while let Some(vertex) = pending.pop_front() {
        control.check()?;
        *visits = visits
            .checked_add(1)
            .filter(|n| *n <= crate::platform::kernel::contract::MAXIMUM_VALIDATION_WORK)
            .ok_or_else(|| {
                ExecutionError::resource(
                    "reference_transfer_work",
                    "canonical transfer proof exhausted its work allowance",
                )
            })?;
        if seen.contains(&vertex) {
            continue;
        }
        index_node::<Vertex>(allocated)?;
        seen.insert(vertex);
        let (ty, context, owners_allowed) = vertex;
        let form = &schema.types.get(&ty).ok_or_else(failure)?.form;
        let mut edges = Vec::new();
        match form {
            TypeForm::Unit
            | TypeForm::Bool
            | TypeForm::I64
            | TypeForm::F64
            | TypeForm::Bytes
            | TypeForm::Text
            | TypeForm::StaticText => {}
            TypeForm::ByteBuffer | TypeForm::OwnedI64Cell if owners_allowed => {}
            TypeForm::OwnedProduct { fields } | TypeForm::OwnedChoice { cases: fields }
                if owners_allowed =>
            {
                let mut statically_owned = false;
                for field in fields {
                    statically_owned |=
                        scoped_owned(schema, snapshots, field.ty, context.declaration)?;
                }
                if !statically_owned {
                    return Err(failure());
                }
                allocate::<Vertex>(allocated, fields.len())?;
                for field in fields {
                    edges.push((field.ty, context, true));
                }
            }
            TypeForm::TypeParameter { parameter } => {
                let declaration = context.declaration.ok_or_else(failure)?;
                let OwnerRecord::TypeParameter(record) =
                    canonical_owner(snapshots, declaration, OwnerKey::TypeParameter(*parameter))?
                else {
                    return Err(failure());
                };
                let OwnerRecord::Declaration(owner) = canonical_owner(
                    snapshots,
                    declaration,
                    OwnerKey::Declaration(declaration.declaration),
                )?
                else {
                    return Err(failure());
                };
                if record.declaration != declaration.declaration
                    || !owner.payload.type_parameters().contains(parameter)
                    || (!owners_allowed && record.constraints.has_owned())
                    || if context.nominal_formals {
                        record.constraints.has_owned()
                            || !matches!(
                                owner.payload,
                                DeclarationPayload::Record { .. }
                                    | DeclarationPayload::Variant { .. }
                            )
                    } else {
                        !record.constraints.requires_transfer()
                            || !matches!(owner.payload, DeclarationPayload::Function(_))
                    }
                {
                    return Err(failure());
                }
            }
            TypeForm::List { item } | TypeForm::Option { item } => {
                allocate::<Vertex>(allocated, 1)?;
                edges.push((*item, context, false));
            }
            TypeForm::Map { key, value }
            | TypeForm::Result {
                ok: key,
                error: value,
            } => {
                allocate::<Vertex>(allocated, 2)?;
                edges.push((*key, context, false));
                edges.push((*value, context, false));
            }
            TypeForm::StructuralRecord { fields } => {
                allocate::<Vertex>(allocated, fields.len())?;
                for field in fields {
                    edges.push((field.ty, context, false));
                }
            }
            TypeForm::Named { declaration } | TypeForm::Applied { declaration, .. } => {
                let arguments: &[TypeObjectDigest] = match form {
                    TypeForm::Applied { arguments, .. } => arguments,
                    _ => &[],
                };
                let OwnerRecord::Declaration(owner) = canonical_owner(
                    snapshots,
                    *declaration,
                    OwnerKey::Declaration(declaration.declaration),
                )?
                else {
                    return Err(failure());
                };
                let (parameters, fields, cases) = match &owner.payload {
                    DeclarationPayload::Record {
                        type_parameters,
                        fields,
                    } => (type_parameters, fields.as_slice(), &[][..]),
                    DeclarationPayload::Variant {
                        type_parameters,
                        cases,
                    } => (type_parameters, &[][..], cases.as_slice()),
                    _ => return Err(failure()),
                };
                if arguments.len() != parameters.len() {
                    return Err(failure());
                }
                allocate::<Vertex>(allocated, arguments.len() + fields.len() + cases.len())?;
                for (parameter, actual) in parameters.iter().zip(arguments) {
                    let OwnerRecord::TypeParameter(record) = canonical_owner(
                        snapshots,
                        *declaration,
                        OwnerKey::TypeParameter(*parameter),
                    )?
                    else {
                        return Err(failure());
                    };
                    if record.declaration != declaration.declaration
                        || record.constraints.has_owned()
                        || record.constraints.requires_transfer()
                    {
                        return Err(failure());
                    }
                    edges.push((*actual, context, false));
                }
                let nested = TransferContext {
                    declaration: Some(*declaration),
                    nominal_formals: true,
                };
                for field in fields {
                    let OwnerRecord::Field(record) =
                        canonical_owner(snapshots, *declaration, OwnerKey::Field(*field))?
                    else {
                        return Err(failure());
                    };
                    if record.declaration != declaration.declaration {
                        return Err(failure());
                    }
                    edges.push((record.ty, nested, false));
                }
                for case in cases {
                    let OwnerRecord::Case(record) =
                        canonical_owner(snapshots, *declaration, OwnerKey::Case(*case))?
                    else {
                        return Err(failure());
                    };
                    if record.declaration != declaration.declaration {
                        return Err(failure());
                    }
                    if let Some(payload) = record.payload {
                        edges.push((payload, nested, false));
                    }
                }
            }
            _ => return Err(failure()),
        }
        allocate::<Vertex>(allocated, edges.len())?;
        pending.extend(edges);
    }
    Ok(())
}

impl Closure<'_> {
    fn has_unresolved_type(
        &mut self,
        ty: TypeObjectDigest,
        memo: &mut BTreeMap<TypeObjectDigest, bool>,
        depth: usize,
    ) -> Result<bool, ExecutionError> {
        self.tick()?;
        if depth > crate::platform::kernel::contract::MAXIMUM_TYPE_DEPTH {
            return Err(failure());
        }
        if let Some(answer) = memo.get(&ty) {
            return Ok(*answer);
        }
        self.clone_type(ty)?;
        let object = self.types.get(&ty).cloned().ok_or_else(failure)?;
        let mut unresolved = matches!(object.form, TypeForm::TypeParameter { .. })
            || matches!(&object.form, TypeForm::TaskFunction { effect, .. } if !effect.parameters.is_empty());
        allocate::<TypeObjectDigest>(&mut self.allocated, object.child_type_count())?;
        for child in object.child_types() {
            unresolved |= self.has_unresolved_type(child, memo, depth + 1)?;
        }
        index_node::<(TypeObjectDigest, bool)>(&mut self.allocated)?;
        memo.insert(ty, unresolved);
        Ok(unresolved)
    }

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
        let mut unresolved = BTreeMap::new();
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
            if self.has_unresolved_type(ty, &mut unresolved, 0)? {
                continue;
            }
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
    BufferFree,
    Capture,
    Transfer,
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
    let callables = !matches!(retention, Retention::Comparable | Retention::Transfer);
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
        if (retention == Retention::BufferFree
            && !matches!(
                object.form,
                TypeForm::ByteBuffer
                    | TypeForm::OwnedI64Cell
                    | TypeForm::OwnedProduct { .. }
                    | TypeForm::OwnedChoice { .. }
            ))
            || retention == Retention::NoApplication
            || retention != Retention::BufferFree
                && !matches!(
                    object.form,
                    TypeForm::ByteBuffer | TypeForm::OwnedI64Cell
                        | TypeForm::OwnedProduct { .. } | TypeForm::OwnedChoice { .. }
                        if retention != Retention::Transfer
                )
                && !matches!(
                    object.form,
                    TypeForm::Stream { .. }
                        | TypeForm::CapabilityResource { .. }
                        | TypeForm::TypeParameter { .. }
                )
                && (secrets || !matches!(object.form, TypeForm::Secret))
                && (callables
                    || !matches!(
                        object.form,
                        TypeForm::Function { .. } | TypeForm::TaskFunction { .. }
                    ))
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
                Ok(safe.contains(&child)
                    && (retention != Retention::Transfer
                        || !matches!(
                            schema.types[&child].form,
                            TypeForm::ByteBuffer
                                | TypeForm::OwnedI64Cell
                                | TypeForm::OwnedProduct { .. }
                                | TypeForm::OwnedChoice { .. }
                        )))
            };
            let accepted = match &object.form {
                TypeForm::ByteBuffer
                | TypeForm::OwnedI64Cell
                | TypeForm::OwnedProduct { .. }
                | TypeForm::OwnedChoice { .. }
                    if retention == Retention::BufferFree =>
                {
                    false
                }
                _ if retention == Retention::BufferFree => {
                    let mut accepted = true;
                    for ty in object.child_types() {
                        accepted &= retained(ty)?;
                    }
                    accepted
                }
                TypeForm::Applied { .. } if retention == Retention::NoApplication => false,
                TypeForm::ByteBuffer
                | TypeForm::OwnedI64Cell
                | TypeForm::OwnedProduct { .. }
                | TypeForm::OwnedChoice { .. }
                | TypeForm::Secret
                | TypeForm::Stream { .. }
                | TypeForm::CapabilityResource { .. }
                | TypeForm::TypeParameter { .. }
                    if retention == Retention::NoApplication =>
                {
                    true
                }
                TypeForm::ByteBuffer | TypeForm::OwnedI64Cell
                    if retention == Retention::Transfer =>
                {
                    true
                }
                TypeForm::OwnedProduct { fields } | TypeForm::OwnedChoice { cases: fields }
                    if retention == Retention::Transfer =>
                {
                    let mut accepted = fields.iter().any(|field| {
                        matches!(
                            schema.types.get(&field.ty).map(|ty| &ty.form),
                            Some(
                                TypeForm::ByteBuffer
                                    | TypeForm::OwnedI64Cell
                                    | TypeForm::OwnedProduct { .. }
                                    | TypeForm::OwnedChoice { .. }
                            )
                        )
                    });
                    for field in fields {
                        tick(visits)?;
                        if !schema.types.contains_key(&field.ty) {
                            return Err(failure());
                        }
                        accepted &= safe.contains(&field.ty);
                    }
                    accepted
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
                                accepted &= safe.contains(&payload)
                                    && (retention != Retention::Transfer
                                        || !matches!(
                                            schema.types[&payload].form,
                                            TypeForm::ByteBuffer
                                                | TypeForm::OwnedI64Cell
                                                | TypeForm::OwnedProduct { .. }
                                                | TypeForm::OwnedChoice { .. }
                                        ));
                            }
                        }
                    } else if matches!(object.form, TypeForm::Applied { .. }) {
                        accepted = false;
                    } else {
                        return Err(failure());
                    }
                    accepted
                }
                TypeForm::Unit
                | TypeForm::Bool
                | TypeForm::I64
                | TypeForm::F64
                | TypeForm::Bytes
                | TypeForm::Text
                | TypeForm::StaticText => true,
                TypeForm::Function { .. } | TypeForm::TaskFunction { .. } => callables,
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

#[cfg(test)]
mod transfer_proof_tests {
    use super::*;
    use crate::platform::execution::{ExecutionControl, ExecutionFailureClass};
    use crate::platform::kernel::TypeParameterConstraints;

    fn source(input: &str) -> KernelSnapshot {
        super::super::tests::byte_buffer_tests::author_only(input).unwrap()
    }

    fn declaration(snapshot: &KernelSnapshot, name: &str) -> DeclarationReference {
        snapshot
            .owners
            .iter()
            .find_map(|(key, record)| {
                let (OwnerKey::Declaration(id), OwnerRecord::Declaration(owner)) = (key, record)
                else {
                    return None;
                };
                (owner.name.as_str() == name).then_some(DeclarationReference {
                    package: snapshot.root.package_id,
                    declaration: *id,
                })
            })
            .unwrap()
    }

    fn schema(snapshot: &KernelSnapshot) -> NormalizedReferenceSchema {
        NormalizedReferenceSchema {
            types: snapshot
                .types
                .iter()
                .chain(&snapshot.dependency_types)
                .map(|(digest, object)| (*digest, object.clone()))
                .collect(),
            ..NormalizedReferenceSchema::default()
        }
    }

    fn intern(schema: &mut NormalizedReferenceSchema, form: TypeForm) -> TypeObjectDigest {
        let object = TypeObject::new(form).unwrap();
        let (digest, _) = encode_type_object(&object).unwrap();
        schema.types.insert(digest, object);
        digest
    }

    fn prove(
        schema: &NormalizedReferenceSchema,
        snapshot: &KernelSnapshot,
        ty: TypeObjectDigest,
        scope: Option<DeclarationReference>,
    ) -> Result<(), ExecutionError> {
        parallel_transfer_type(
            schema,
            &[snapshot],
            ty,
            scope,
            &mut 0,
            &mut 0,
            &ExecutionControl::uncancelled(),
        )
    }

    const PARAMETERS: &str = r#"
declarations.begin
(units (module create proof
  (function create keep (visibility public)
    (type-parameter create T (constraint transferable))
    (parameter create value (type T)) (returns T) (effect pure) (body (local value)))
  (function create foreign (visibility public)
    (type-parameter create U (constraint transferable))
    (parameter create value (type U)) (returns U) (effect pure) (body (local value)))))
declarations.end
"#;

    #[test]
    fn canonical_transfer_parameters_require_exact_scope_and_explicit_bounds() {
        let mut snapshot = source(PARAMETERS);
        let scope = declaration(&snapshot, "keep");
        let foreign = declaration(&snapshot, "foreign");
        let OwnerRecord::Declaration(owner) =
            &snapshot.owners[&OwnerKey::Declaration(scope.declaration)]
        else {
            panic!("function owner");
        };
        let DeclarationPayload::Function(function) = &owner.payload else {
            panic!("function");
        };
        let ty = function.result;
        let parameter = function.type_parameters[0];
        let schema = schema(&snapshot);
        assert!(prove(&schema, &snapshot, ty, Some(scope)).is_ok());
        assert!(prove(&schema, &snapshot, ty, None).is_err());
        assert!(prove(&schema, &snapshot, ty, Some(foreign)).is_err());
        for constraints in [
            TypeParameterConstraints::None,
            TypeParameterConstraints::CaptureSafe,
            TypeParameterConstraints::Owned,
        ] {
            let OwnerRecord::TypeParameter(record) = snapshot
                .owners
                .get_mut(&OwnerKey::TypeParameter(parameter))
                .unwrap()
            else {
                panic!("type parameter");
            };
            record.constraints = constraints;
            assert!(
                prove(&schema, &snapshot, ty, Some(scope)).is_err(),
                "{constraints:?}"
            );
        }
        for constraints in [
            TypeParameterConstraints::Transferable,
            TypeParameterConstraints::CaptureSafeTransferable,
            TypeParameterConstraints::OwnedTransferable,
        ] {
            let OwnerRecord::TypeParameter(record) = snapshot
                .owners
                .get_mut(&OwnerKey::TypeParameter(parameter))
                .unwrap()
            else {
                panic!("type parameter");
            };
            record.constraints = constraints;
            assert!(
                prove(&schema, &snapshot, ty, Some(scope)).is_ok(),
                "{constraints:?}"
            );
        }
    }

    const RECURSIVE: &str = r#"
declarations.begin
(units (module create recursive-proof
  (record create Left (visibility public) (type-parameter create L)
    (field create value (type L)) (field create right (type (option (Right L)))))
  (record create Right (visibility public) (type-parameter create R)
    (field create left (type (option (Left R)))))
  (variant create Node (visibility public)
    (case create loop (payload Node)) (case create inactive (payload I64)))
  (record create Phantom (visibility public) (type-parameter create P)
    (field create value (type I64)))))
declarations.end
"#;

    #[test]
    fn canonical_transfer_recursion_finishes_and_rejects_phantoms_and_inactive_cases() {
        let mut snapshot = source(RECURSIVE);
        let mut schema = schema(&snapshot);
        let scalar = intern(&mut schema, TypeForm::I64);
        let left = intern(
            &mut schema,
            TypeForm::Applied {
                declaration: declaration(&snapshot, "Left"),
                arguments: vec![scalar],
            },
        );
        let node = intern(
            &mut schema,
            TypeForm::Named {
                declaration: declaration(&snapshot, "Node"),
            },
        );
        assert!(prove(&schema, &snapshot, left, None).is_ok());
        assert!(prove(&schema, &snapshot, node, None).is_ok());
        let callable = intern(
            &mut schema,
            TypeForm::Function {
                parameters: vec![],
                result: scalar,
            },
        );
        let phantom = intern(
            &mut schema,
            TypeForm::Applied {
                declaration: declaration(&snapshot, "Phantom"),
                arguments: vec![callable],
            },
        );
        assert!(prove(&schema, &snapshot, phantom, None).is_err());
        let secret = intern(&mut schema, TypeForm::Secret);
        for owner in snapshot.owners.values_mut() {
            if let OwnerRecord::Case(case) = owner
                && case.name.as_str() == "inactive"
            {
                case.payload = Some(secret);
            }
        }
        assert!(prove(&schema, &snapshot, node, None).is_err());
        let right = declaration(&snapshot, "Right");
        for owner in snapshot.owners.values_mut() {
            if let OwnerRecord::Field(field) = owner
                && field.declaration == right.declaration
            {
                field.ty = callable;
            }
        }
        assert!(prove(&schema, &snapshot, left, None).is_err());
    }

    #[test]
    fn canonical_transfer_capacity_and_cancellation_are_distinct_from_rejection() {
        let snapshot = source(PARAMETERS);
        let scope = declaration(&snapshot, "keep");
        let OwnerRecord::Declaration(owner) =
            &snapshot.owners[&OwnerKey::Declaration(scope.declaration)]
        else {
            panic!("function owner");
        };
        let DeclarationPayload::Function(function) = &owner.payload else {
            panic!("function");
        };
        let schema = schema(&snapshot);
        let mut work = crate::platform::kernel::contract::MAXIMUM_VALIDATION_WORK;
        let error = parallel_transfer_type(
            &schema,
            &[&snapshot],
            function.result,
            Some(scope),
            &mut work,
            &mut 0,
            &ExecutionControl::uncancelled(),
        )
        .unwrap_err();
        assert_eq!(error.class, ExecutionFailureClass::Resource);
        let control = ExecutionControl::uncancelled();
        control.cancel();
        let error = parallel_transfer_type(
            &schema,
            &[&snapshot],
            function.result,
            Some(scope),
            &mut 0,
            &mut 0,
            &control,
        )
        .unwrap_err();
        assert_eq!(error.class, ExecutionFailureClass::Cancelled);
        let mut metadata = MAXIMUM_METADATA_BYTES;
        let error = parallel_transfer_type(
            &schema,
            &[&snapshot],
            function.result,
            Some(scope),
            &mut 0,
            &mut metadata,
            &ExecutionControl::uncancelled(),
        )
        .unwrap_err();
        assert_eq!(error.class, ExecutionFailureClass::Resource);
    }

    #[test]
    fn canonical_schema_retains_generic_pairs_without_a_concrete_caller() {
        let snapshot = source(
            r#"
declarations.begin
(units (module create pair-proof
  (function create keep (visibility public) (effect (task))
    (type-parameter create T (constraint transferable))
    (parameter create value (type T)) (returns T) (body (local value)))
  (function create group (visibility public) (effect (task))
    (type-parameter create T (constraint transferable))
    (parameter create value (type T)) (returns (record (left T) (right T)))
    (body (parallel (call keep (types T) (local value))
      (call keep (types T) (local value)))))))
declarations.end
"#,
        );
        let schema = NormalizedReferenceSchema::reconstruct([&snapshot]).unwrap();
        let group = declaration(&snapshot, "group");
        let OwnerRecord::Declaration(owner) =
            &snapshot.owners[&OwnerKey::Declaration(group.declaration)]
        else {
            panic!("function owner");
        };
        let DeclarationPayload::Function(function) = &owner.payload else {
            panic!("function");
        };
        assert!(schema.types.contains_key(&function.result));
    }
    #[test]
    fn canonical_concrete_transfer_property_preserves_owners_and_complete_alternatives() {
        use crate::platform::kernel::{Name, StructuralTypeField};
        let mut schema = NormalizedReferenceSchema::default();
        let integer = intern(&mut schema, TypeForm::I64);
        let buffer = intern(&mut schema, TypeForm::ByteBuffer);
        let cell = intern(&mut schema, TypeForm::OwnedI64Cell);
        let callback = intern(
            &mut schema,
            TypeForm::Function {
                parameters: vec![],
                result: integer,
            },
        );
        let fields = |left, right| {
            vec![
                StructuralTypeField {
                    name: Name::new("left").unwrap(),
                    ty: left,
                },
                StructuralTypeField {
                    name: Name::new("right").unwrap(),
                    ty: right,
                },
            ]
        };
        let mixed = intern(
            &mut schema,
            TypeForm::OwnedProduct {
                fields: fields(buffer, integer),
            },
        );
        let ordinary_wrapper = intern(
            &mut schema,
            TypeForm::OwnedProduct {
                fields: fields(integer, integer),
            },
        );
        let list_owner = intern(&mut schema, TypeForm::List { item: cell });
        let choice = intern(
            &mut schema,
            TypeForm::OwnedChoice {
                cases: fields(mixed, callback),
            },
        );
        let safe = property_types(
            &schema,
            &mut 0,
            &mut 0,
            &ExecutionControl::uncancelled(),
            Retention::Transfer,
        )
        .unwrap();
        for ty in [integer, buffer, cell, mixed] {
            assert!(safe.contains(&ty));
        }
        for ty in [callback, ordinary_wrapper, list_owner, choice] {
            assert!(!safe.contains(&ty));
        }
    }
}
