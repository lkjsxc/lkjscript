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
}

impl Closure<'_> {
    fn tick(&mut self) -> Result<(), ExecutionError> {
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
        let object = self.types.get(&ty).cloned().ok_or_else(failure)?;
        let form = match object.form {
            TypeForm::TypeParameter { parameter } => {
                return bindings.get(&parameter).copied().ok_or_else(failure);
            }
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
        let mut pending = vec![root];
        while let Some(expression) = pending.pop() {
            let OwnerRecord::Expression(record) = self
                .owner(package, OwnerKey::Expression(expression))?
                .clone()
            else {
                return Err(failure());
            };
            let children = record.children();
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
                    let mut concrete = Vec::with_capacity(type_arguments.len());
                    for ty in type_arguments {
                        concrete.push(self.identity(ty, bindings, 0)?);
                    }
                    self.tick()?;
                    calls.push_back((function, concrete));
                }
                _ => {}
            }
            for child in children {
                self.tick()?;
                pending.push(child.expression);
            }
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
) -> Result<(), ExecutionError> {
    let mut closure = Closure {
        snapshots: snapshots
            .iter()
            .map(|snapshot| (snapshot.root.package_id, *snapshot))
            .collect(),
        types: &mut schema.types,
        visits: 0,
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
    Ok(())
}
