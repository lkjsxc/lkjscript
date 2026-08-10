use std::collections::{HashMap, VecDeque};
use std::fmt;

use crate::hir::{
    CoreTrait, GenericInstantiation, ImplDefinition, ImplId, ProductDefinition, ProductId,
    TraitBound, TraitDefinition, TraitId, TraitWitness, TraitWitnessKind, Type, TypeSubstitution,
};

pub(crate) struct GenericFacts<'a> {
    pub traits: &'a [TraitDefinition],
    pub products: &'a [ProductDefinition],
    pub implementations: &'a [ImplDefinition],
    pub product_names: &'a HashMap<String, ProductId>,
    pub implementation_index: &'a HashMap<(TraitId, ProductId), ImplId>,
}

#[derive(Debug)]
pub(crate) enum GenericCallError {
    NotCallable,
    UnexpectedSubstitutions,
    SubstitutionCount {
        expected: usize,
        actual: usize,
    },
    SubstitutionOrder {
        expected: String,
        actual: String,
    },
    Arity {
        expected: usize,
        actual: usize,
    },
    TypeMismatch {
        index: usize,
        expected: Box<Type>,
        actual: Box<Type>,
    },
    OwnershipUnsupported,
    ForwardingUnsupported,
    ReferenceResultUnsupported,
    UnknownTrait(TraitId),
    UnknownProduct(String),
    UnsatisfiedTrait {
        parameter: String,
        trait_id: TraitId,
        ty: Box<Type>,
    },
    MissingBoundSubstitution(String),
    InvalidFacts(String),
    Host(String),
}

impl fmt::Display for GenericCallError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotCallable => formatter.write_str("call target is not a function"),
            Self::UnexpectedSubstitutions => {
                formatter.write_str("non-generic call contains type substitutions")
            }
            Self::SubstitutionCount { expected, actual } => write!(
                formatter,
                "generic call requires {expected} type substitution(s), got {actual}"
            ),
            Self::SubstitutionOrder { expected, actual } => write!(
                formatter,
                "generic call expected substitution for {expected}, got {actual}"
            ),
            Self::Arity { expected, actual } => write!(
                formatter,
                "call requires {expected} value argument(s), got {actual}"
            ),
            Self::TypeMismatch { index, expected, actual } => write!(
                formatter,
                "call argument {index} has type {actual}, expected {expected}"
            ),
            Self::OwnershipUnsupported => formatter.write_str(
                "ownership/reference generic instantiation is unavailable in the initial ownership slice",
            ),
            Self::ForwardingUnsupported => formatter.write_str(
                "forwarding a caller type parameter through a generic call is unavailable in the current transport route",
            ),
            Self::ReferenceResultUnsupported => formatter.write_str(
                "user-call results cannot be lexical references in the initial ownership slice",
            ),
            Self::UnknownTrait(id) => write!(formatter, "bound references unknown trait {}", id.raw()),
            Self::UnknownProduct(name) => write!(formatter, "unknown product type {name}"),
            Self::UnsatisfiedTrait { trait_id, ty, .. } => write!(
                formatter,
                "type {ty} does not satisfy trait {}",
                trait_id.raw()
            ),
            Self::MissingBoundSubstitution(parameter) => {
                write!(formatter, "missing substitution for bound parameter {parameter}")
            }
            Self::InvalidFacts(message) | Self::Host(message) => formatter.write_str(message),
        }
    }
}

pub(crate) struct ExactCall {
    pub parameters: Vec<Type>,
    pub result: Type,
    pub instantiation: Option<GenericInstantiation>,
}

pub(crate) fn resolve_exact(
    callable: &Type,
    substitutions: Vec<TypeSubstitution>,
    arguments: &[Type],
    bounds: &[TraitBound],
    facts: &GenericFacts<'_>,
) -> Result<ExactCall, GenericCallError> {
    let (variables, signature, generic) = match callable {
        Type::Forall { vars, body } => (vars.as_slice(), body.as_ref(), true),
        other => (&[][..], other, false),
    };
    let Type::Fn { params, ret } = signature else {
        return Err(GenericCallError::NotCallable);
    };
    if params.len() != arguments.len() {
        return Err(GenericCallError::Arity {
            expected: params.len(),
            actual: arguments.len(),
        });
    }
    if !generic && !substitutions.is_empty() {
        return Err(GenericCallError::UnexpectedSubstitutions);
    }
    if generic && substitutions.len() != variables.len() {
        return Err(GenericCallError::SubstitutionCount {
            expected: variables.len(),
            actual: substitutions.len(),
        });
    }
    for (expected, substitution) in variables.iter().zip(&substitutions) {
        if substitution.parameter != *expected {
            return Err(GenericCallError::SubstitutionOrder {
                expected: expected.clone(),
                actual: substitution.parameter.clone(),
            });
        }
    }
    if generic {
        if contains_ownership_type(callable)? {
            return Err(GenericCallError::OwnershipUnsupported);
        }
        for substitution in &substitutions {
            if contains_ownership_type(&substitution.ty)? {
                return Err(GenericCallError::OwnershipUnsupported);
            }
        }
    }
    for substitution in &substitutions {
        if contains_type_parameter(&substitution.ty)? {
            return Err(GenericCallError::ForwardingUnsupported);
        }
    }
    let mut map = HashMap::new();
    map.try_reserve(substitutions.len())
        .map_err(|_| GenericCallError::Host("generic substitution map allocation failed".into()))?;
    for substitution in &substitutions {
        map.insert(substitution.parameter.as_str(), &substitution.ty);
    }
    let mut parameters = Vec::new();
    parameters
        .try_reserve(params.len())
        .map_err(|_| GenericCallError::Host("generic parameter allocation failed".into()))?;
    for parameter in params {
        parameters.push(substitute_type(parameter, &map)?);
    }
    let result = substitute_type(ret, &map)?;
    for (index, (actual, expected)) in arguments.iter().zip(&parameters).enumerate() {
        if !types_assignable(actual, expected)? {
            return Err(GenericCallError::TypeMismatch {
                index,
                expected: Box::new(expected.clone()),
                actual: Box::new(actual.clone()),
            });
        }
    }
    if contains_reference_type(&result)? {
        return Err(GenericCallError::ReferenceResultUnsupported);
    }
    let mut witnesses = Vec::new();
    witnesses
        .try_reserve(bounds.len())
        .map_err(|_| GenericCallError::Host("generic witness allocation failed".into()))?;
    for bound in bounds {
        let ty = substitutions
            .iter()
            .find(|substitution| substitution.parameter == bound.parameter)
            .map(|substitution| &substitution.ty)
            .ok_or_else(|| GenericCallError::MissingBoundSubstitution(bound.parameter.clone()))?;
        witnesses.push(solve_trait_bound(
            &bound.parameter,
            bound.trait_id,
            ty,
            facts,
        )?);
    }
    drop(map);
    Ok(ExactCall {
        parameters,
        result,
        instantiation: generic.then_some(GenericInstantiation {
            substitutions,
            witnesses,
        }),
    })
}

pub(crate) fn substitute_type(
    root: &Type,
    substitutions: &HashMap<&str, &Type>,
) -> Result<Type, GenericCallError> {
    enum Work<'a> {
        Visit(&'a Type, bool),
        Enter(&'a [String]),
        Exit(&'a [String]),
        Enum(crate::hir::EnumId, &'a str, usize),
        List,
        Function(usize),
        Forall(&'a [String]),
    }

    let mut work = Vec::new();
    work.try_reserve(1)
        .map_err(|_| GenericCallError::Host("type substitution work allocation failed".into()))?;
    work.push(Work::Visit(root, true));
    let mut completed = Vec::new();
    let mut bound = HashMap::<&str, usize>::new();
    while let Some(item) = work.pop() {
        match item {
            Work::Visit(ty, apply) => {
                completed.try_reserve(1).map_err(|_| {
                    GenericCallError::Host("type substitution result allocation failed".into())
                })?;
                match ty {
                    Type::Never => completed.push(Type::Never),
                    Type::Unit => completed.push(Type::Unit),
                    Type::Bool => completed.push(Type::Bool),
                    Type::I64 => completed.push(Type::I64),
                    Type::F64 => completed.push(Type::F64),
                    Type::Str => completed.push(Type::Str),
                    Type::Bytes => completed.push(Type::Bytes),
                    Type::ByteVector => completed.push(Type::ByteVector),
                    Type::ByteSlice => completed.push(Type::ByteSlice),
                    Type::ByteSliceMut => completed.push(Type::ByteSliceMut),
                    Type::Path => completed.push(Type::Path),
                    Type::Capability(kind) => completed.push(Type::Capability(*kind)),
                    Type::Symbol => completed.push(Type::Symbol),
                    Type::Resource(kind) => completed.push(Type::Resource(*kind)),
                    Type::Product(name) => {
                        completed.push(Type::Product(clone_type_string(name)?));
                    }
                    Type::Param(name)
                        if apply
                            && !bound.contains_key(name.as_str())
                            && substitutions.contains_key(name.as_str()) =>
                    {
                        let substitution =
                            substitutions.get(name.as_str()).copied().ok_or_else(|| {
                                GenericCallError::InvalidFacts(
                                    "type substitution disappeared during lookup".into(),
                                )
                            })?;
                        work.try_reserve(1).map_err(|_| {
                            GenericCallError::Host(
                                "type substitution work allocation failed".into(),
                            )
                        })?;
                        work.push(Work::Visit(substitution, false));
                    }
                    Type::Param(name) => {
                        completed.push(Type::Param(clone_type_string(name)?));
                    }
                    Type::Enum {
                        id,
                        name,
                        arguments,
                    } => {
                        let additional = arguments.len().checked_add(1).ok_or_else(|| {
                            GenericCallError::Host("type substitution child count overflow".into())
                        })?;
                        work.try_reserve(additional).map_err(|_| {
                            GenericCallError::Host(
                                "type substitution work allocation failed".into(),
                            )
                        })?;
                        work.push(Work::Enum(*id, name, arguments.len()));
                        work.extend(
                            arguments
                                .iter()
                                .rev()
                                .map(|argument| Work::Visit(argument, apply)),
                        );
                    }
                    Type::List(inner) => {
                        work.try_reserve(2).map_err(|_| {
                            GenericCallError::Host(
                                "type substitution work allocation failed".into(),
                            )
                        })?;
                        work.push(Work::List);
                        work.push(Work::Visit(inner, apply));
                    }
                    Type::Fn { params, ret } => {
                        let additional = params.len().checked_add(2).ok_or_else(|| {
                            GenericCallError::Host("type substitution child count overflow".into())
                        })?;
                        work.try_reserve(additional).map_err(|_| {
                            GenericCallError::Host(
                                "type substitution work allocation failed".into(),
                            )
                        })?;
                        work.push(Work::Function(params.len()));
                        work.push(Work::Visit(ret, apply));
                        work.extend(
                            params
                                .iter()
                                .rev()
                                .map(|parameter| Work::Visit(parameter, apply)),
                        );
                    }
                    Type::Forall { vars, body } if apply => {
                        work.try_reserve(4).map_err(|_| {
                            GenericCallError::Host(
                                "type substitution work allocation failed".into(),
                            )
                        })?;
                        work.push(Work::Forall(vars));
                        work.push(Work::Exit(vars));
                        work.push(Work::Visit(body, true));
                        work.push(Work::Enter(vars));
                    }
                    Type::Forall { vars, body } => {
                        work.try_reserve(2).map_err(|_| {
                            GenericCallError::Host(
                                "type substitution work allocation failed".into(),
                            )
                        })?;
                        work.push(Work::Forall(vars));
                        work.push(Work::Visit(body, false));
                    }
                }
            }
            Work::Enter(variables) => {
                bound.try_reserve(variables.len()).map_err(|_| {
                    GenericCallError::Host("type substitution scope allocation failed".into())
                })?;
                for variable in variables {
                    let count = bound.entry(variable.as_str()).or_insert(0);
                    *count = count.checked_add(1).ok_or_else(|| {
                        GenericCallError::Host("type substitution scope count overflow".into())
                    })?;
                }
            }
            Work::Exit(variables) => {
                for variable in variables {
                    let remove = {
                        let count = bound.get_mut(variable.as_str()).ok_or_else(|| {
                            GenericCallError::InvalidFacts(
                                "type substitution scope exit is unbalanced".into(),
                            )
                        })?;
                        *count = count.checked_sub(1).ok_or_else(|| {
                            GenericCallError::InvalidFacts(
                                "type substitution scope count underflow".into(),
                            )
                        })?;
                        *count == 0
                    };
                    if remove {
                        bound.remove(variable.as_str());
                    }
                }
            }
            Work::Enum(id, name, count) => {
                let split = completed.len().checked_sub(count).ok_or_else(|| {
                    GenericCallError::InvalidFacts(
                        "type substitution enum children are incomplete".into(),
                    )
                })?;
                let arguments = completed.split_off(split);
                completed.push(Type::Enum {
                    id,
                    name: clone_type_string(name)?,
                    arguments,
                });
            }
            Work::List => {
                let inner = completed.pop().ok_or_else(|| {
                    GenericCallError::InvalidFacts(
                        "type substitution list child is incomplete".into(),
                    )
                })?;
                completed.push(Type::List(Box::new(inner)));
            }
            Work::Function(count) => {
                let result = completed.pop().ok_or_else(|| {
                    GenericCallError::InvalidFacts(
                        "type substitution function result is incomplete".into(),
                    )
                })?;
                let split = completed.len().checked_sub(count).ok_or_else(|| {
                    GenericCallError::InvalidFacts(
                        "type substitution function parameters are incomplete".into(),
                    )
                })?;
                let params = completed.split_off(split);
                completed.push(Type::Fn {
                    params,
                    ret: Box::new(result),
                });
            }
            Work::Forall(variables) => {
                let body = completed.pop().ok_or_else(|| {
                    GenericCallError::InvalidFacts(
                        "type substitution universal body is incomplete".into(),
                    )
                })?;
                let mut vars = Vec::new();
                vars.try_reserve(variables.len()).map_err(|_| {
                    GenericCallError::Host("type substitution binder allocation failed".into())
                })?;
                for variable in variables {
                    vars.push(clone_type_string(variable)?);
                }
                completed.push(Type::Forall {
                    vars,
                    body: Box::new(body),
                });
            }
        }
    }
    let result = completed.pop().ok_or_else(|| {
        GenericCallError::InvalidFacts("type substitution omitted its root".into())
    })?;
    if completed.is_empty() {
        Ok(result)
    } else {
        Err(GenericCallError::InvalidFacts(
            "type substitution left disconnected results".into(),
        ))
    }
}

fn clone_type_string(value: &str) -> Result<String, GenericCallError> {
    let mut cloned = String::new();
    cloned
        .try_reserve(value.len())
        .map_err(|_| GenericCallError::Host("type substitution string allocation failed".into()))?;
    cloned.push_str(value);
    Ok(cloned)
}

fn solve_trait_bound(
    parameter: &str,
    trait_id: TraitId,
    ty: &Type,
    facts: &GenericFacts<'_>,
) -> Result<TraitWitness, GenericCallError> {
    let definition = facts
        .traits
        .get(trait_id.index().unwrap_or(usize::MAX))
        .filter(|definition| definition.id == trait_id)
        .ok_or(GenericCallError::UnknownTrait(trait_id))?;
    let kind = if let Some(core) = definition.core.filter(|core| core.is_auto()) {
        if !auto_trait_holds(core, ty, facts)? {
            return Err(GenericCallError::UnsatisfiedTrait {
                parameter: parameter.to_owned(),
                trait_id,
                ty: Box::new(ty.clone()),
            });
        }
        TraitWitnessKind::AutoTrait
    } else {
        let Type::Product(name) = ty else {
            return Err(GenericCallError::UnsatisfiedTrait {
                parameter: parameter.to_owned(),
                trait_id,
                ty: Box::new(ty.clone()),
            });
        };
        let product = facts
            .product_names
            .get(name)
            .copied()
            .ok_or_else(|| GenericCallError::UnknownProduct(name.clone()))?;
        let implementation = facts
            .implementation_index
            .get(&(trait_id, product))
            .copied()
            .ok_or_else(|| GenericCallError::UnsatisfiedTrait {
                parameter: parameter.to_owned(),
                trait_id,
                ty: Box::new(ty.clone()),
            })?;
        let stored = facts
            .implementations
            .get(implementation.index().unwrap_or(usize::MAX))
            .filter(|stored| {
                stored.id == implementation
                    && stored.trait_id == trait_id
                    && stored.product == product
            })
            .ok_or_else(|| {
                GenericCallError::InvalidFacts("implementation index is stale".into())
            })?;
        TraitWitnessKind::Explicit(stored.id)
    };
    Ok(TraitWitness {
        trait_id,
        ty: ty.clone(),
        kind,
    })
}

fn auto_trait_holds<'a>(
    core: CoreTrait,
    ty: &'a Type,
    facts: &'a GenericFacts<'_>,
) -> Result<bool, GenericCallError> {
    let mut subjects = Vec::new();
    subjects
        .try_reserve(1)
        .map_err(|_| GenericCallError::Host("trait subject allocation failed".into()))?;
    subjects.push(ty);
    let mut indexes = HashMap::new();
    indexes
        .try_reserve(1)
        .map_err(|_| GenericCallError::Host("trait index allocation failed".into()))?;
    indexes.insert(std::ptr::from_ref(ty), 0_usize);
    let mut obligations = Vec::new();
    let mut cursor = 0_usize;
    while cursor < subjects.len() {
        let subject = subjects[cursor];
        let mut children = Vec::new();
        let intrinsic = auto_trait_dependencies(core, subject, facts, &mut children)?;
        let mut dependencies = Vec::new();
        dependencies
            .try_reserve(children.len())
            .map_err(|_| GenericCallError::Host("trait dependency allocation failed".into()))?;
        for child in children {
            let pointer = std::ptr::from_ref(child);
            let dependency = if let Some(index) = indexes.get(&pointer) {
                *index
            } else {
                subjects.try_reserve(1).map_err(|_| {
                    GenericCallError::Host("trait subject allocation failed".into())
                })?;
                indexes
                    .try_reserve(1)
                    .map_err(|_| GenericCallError::Host("trait index allocation failed".into()))?;
                let index = subjects.len();
                subjects.push(child);
                indexes.insert(pointer, index);
                index
            };
            dependencies.push(dependency);
        }
        obligations
            .try_reserve(1)
            .map_err(|_| GenericCallError::Host("trait obligation allocation failed".into()))?;
        obligations.push((intrinsic, dependencies));
        cursor = cursor
            .checked_add(1)
            .ok_or_else(|| GenericCallError::Host("trait obligation index overflow".into()))?;
    }

    let mut dependent_counts = Vec::new();
    dependent_counts
        .try_reserve(obligations.len())
        .map_err(|_| GenericCallError::Host("trait dependent-count allocation failed".into()))?;
    dependent_counts.resize(obligations.len(), 0_usize);
    for (_, dependencies) in &obligations {
        for dependency in dependencies {
            dependent_counts[*dependency] = dependent_counts[*dependency]
                .checked_add(1)
                .ok_or_else(|| GenericCallError::Host("trait dependent count overflow".into()))?;
        }
    }
    let mut dependents = Vec::new();
    dependents
        .try_reserve(obligations.len())
        .map_err(|_| GenericCallError::Host("trait dependent allocation failed".into()))?;
    for count in dependent_counts {
        let mut values = Vec::new();
        values
            .try_reserve(count)
            .map_err(|_| GenericCallError::Host("trait dependent allocation failed".into()))?;
        dependents.push(values);
    }
    for (index, (_, dependencies)) in obligations.iter().enumerate() {
        for dependency in dependencies {
            dependents[*dependency].push(index);
        }
    }

    let mut values = Vec::new();
    values
        .try_reserve(obligations.len())
        .map_err(|_| GenericCallError::Host("trait result allocation failed".into()))?;
    let mut failed = VecDeque::new();
    failed
        .try_reserve(obligations.len())
        .map_err(|_| GenericCallError::Host("trait work allocation failed".into()))?;
    for (index, (intrinsic, _)) in obligations.iter().enumerate() {
        values.push(*intrinsic);
        if !intrinsic {
            failed.push_back(index);
        }
    }
    while let Some(index) = failed.pop_front() {
        for dependent in &dependents[index] {
            if values[*dependent] {
                values[*dependent] = false;
                failed.push_back(*dependent);
            }
        }
    }
    values
        .first()
        .copied()
        .ok_or_else(|| GenericCallError::InvalidFacts("trait solver omitted its root".into()))
}

fn auto_trait_dependencies<'a>(
    core: CoreTrait,
    ty: &'a Type,
    facts: &'a GenericFacts<'_>,
    dependencies: &mut Vec<&'a Type>,
) -> Result<bool, GenericCallError> {
    match core {
        CoreTrait::Copy => match ty {
            Type::Unit
            | Type::Bool
            | Type::I64
            | Type::F64
            | Type::Capability(_)
            | Type::Str
            | Type::Path
            | Type::Symbol
            | Type::ByteSlice => Ok(true),
            Type::Never
            | Type::Bytes
            | Type::ByteVector
            | Type::ByteSliceMut
            | Type::Resource(_)
            | Type::Fn { .. }
            | Type::Forall { .. }
            | Type::Param(_) => Ok(false),
            Type::List(inner) => {
                dependencies.try_reserve(1).map_err(|_| {
                    GenericCallError::Host("trait dependency allocation failed".into())
                })?;
                dependencies.push(inner);
                Ok(true)
            }
            Type::Enum { id, arguments, .. }
                if matches!(
                    id.bytes(),
                    lkjscript_core::OPTION_ID | lkjscript_core::RESULT_ID
                ) =>
            {
                dependencies.try_reserve(arguments.len()).map_err(|_| {
                    GenericCallError::Host("trait dependency allocation failed".into())
                })?;
                dependencies.extend(arguments);
                Ok(true)
            }
            Type::Enum { .. } => Ok(false),
            Type::Product(name) => {
                let product = facts
                    .product_names
                    .get(name)
                    .and_then(|id| id.index())
                    .and_then(|index| facts.products.get(index))
                    .ok_or_else(|| GenericCallError::UnknownProduct(name.clone()))?;
                dependencies
                    .try_reserve(product.fields.len())
                    .map_err(|_| {
                        GenericCallError::Host("trait dependency allocation failed".into())
                    })?;
                dependencies.extend(product.fields.iter().map(|field| &field.ty));
                Ok(true)
            }
        },
        CoreTrait::Send | CoreTrait::Sync => Ok(matches!(
            ty,
            Type::Unit | Type::Bool | Type::I64 | Type::F64
        )),
        CoreTrait::Clone | CoreTrait::Drop => Ok(false),
    }
}

fn contains_type_parameter(root: &Type) -> Result<bool, GenericCallError> {
    visit_type(root, |ty| matches!(ty, Type::Param(_)))
}

fn contains_reference_type(root: &Type) -> Result<bool, GenericCallError> {
    visit_type(root, |ty| {
        matches!(ty, Type::ByteSlice | Type::ByteSliceMut)
    })
}

fn contains_ownership_type(root: &Type) -> Result<bool, GenericCallError> {
    visit_type(root, |ty| {
        matches!(
            ty,
            Type::Bytes
                | Type::ByteVector
                | Type::ByteSlice
                | Type::ByteSliceMut
                | Type::Resource(_)
        )
    })
}

fn visit_type(
    root: &Type,
    mut predicate: impl FnMut(&Type) -> bool,
) -> Result<bool, GenericCallError> {
    let mut pending = Vec::new();
    pending
        .try_reserve(1)
        .map_err(|_| GenericCallError::Host("type traversal allocation failed".into()))?;
    pending.push(root);
    while let Some(ty) = pending.pop() {
        if predicate(ty) {
            return Ok(true);
        }
        match ty {
            Type::Enum { arguments, .. } => {
                pending.try_reserve(arguments.len()).map_err(|_| {
                    GenericCallError::Host("type traversal allocation failed".into())
                })?;
                pending.extend(arguments);
            }
            Type::List(inner) => {
                pending.try_reserve(1).map_err(|_| {
                    GenericCallError::Host("type traversal allocation failed".into())
                })?;
                pending.push(inner);
            }
            Type::Fn { params, ret } => {
                let additional = params
                    .len()
                    .checked_add(1)
                    .ok_or_else(|| GenericCallError::Host("type child count overflow".into()))?;
                pending.try_reserve(additional).map_err(|_| {
                    GenericCallError::Host("type traversal allocation failed".into())
                })?;
                pending.push(ret);
                pending.extend(params);
            }
            Type::Forall { body, .. } => {
                pending.try_reserve(1).map_err(|_| {
                    GenericCallError::Host("type traversal allocation failed".into())
                })?;
                pending.push(body);
            }
            _ => {}
        }
    }
    Ok(false)
}

pub(crate) fn types_assignable(got: &Type, expected: &Type) -> Result<bool, GenericCallError> {
    let mut pending = Vec::new();
    pending
        .try_reserve(1)
        .map_err(|_| GenericCallError::Host("type comparison allocation failed".into()))?;
    pending.push((got, expected));
    while let Some((got, expected)) = pending.pop() {
        match (got, expected) {
            (Type::Never, Type::Never)
            | (Type::Unit, Type::Unit)
            | (Type::Bool, Type::Bool)
            | (Type::I64, Type::I64)
            | (Type::F64, Type::F64)
            | (Type::Str, Type::Str)
            | (Type::Bytes, Type::Bytes)
            | (Type::ByteVector, Type::ByteVector)
            | (Type::ByteSlice, Type::ByteSlice)
            | (Type::ByteSliceMut, Type::ByteSliceMut)
            | (Type::Path, Type::Path)
            | (Type::Symbol, Type::Symbol) => {}
            (Type::Capability(got), Type::Capability(expected)) if got == expected => {}
            (Type::Resource(got), Type::Resource(expected)) if got == expected => {}
            (Type::Product(got), Type::Product(expected)) if got == expected => {}
            (Type::Param(got), Type::Param(expected)) if got == expected => {}
            (
                Type::Enum {
                    id: got_id,
                    arguments: got_arguments,
                    ..
                },
                Type::Enum {
                    id: expected_id,
                    arguments: expected_arguments,
                    ..
                },
            ) if got_id == expected_id && got_arguments.len() == expected_arguments.len() => {
                pending.try_reserve(got_arguments.len()).map_err(|_| {
                    GenericCallError::Host("type comparison allocation failed".into())
                })?;
                pending.extend(got_arguments.iter().zip(expected_arguments));
            }
            (Type::List(got), Type::List(expected)) => {
                pending.try_reserve(1).map_err(|_| {
                    GenericCallError::Host("type comparison allocation failed".into())
                })?;
                pending.push((got, expected));
            }
            (
                Type::Fn {
                    params: got_parameters,
                    ret: got_result,
                },
                Type::Fn {
                    params: expected_parameters,
                    ret: expected_result,
                },
            ) if got_parameters.len() == expected_parameters.len() => {
                let additional = got_parameters
                    .len()
                    .checked_add(1)
                    .ok_or_else(|| GenericCallError::Host("type child count overflow".into()))?;
                pending.try_reserve(additional).map_err(|_| {
                    GenericCallError::Host("type comparison allocation failed".into())
                })?;
                pending.push((got_result, expected_result));
                pending.extend(got_parameters.iter().zip(expected_parameters));
            }
            (
                Type::Forall {
                    vars: got_variables,
                    body: got_body,
                },
                Type::Forall {
                    vars: expected_variables,
                    body: expected_body,
                },
            ) if got_variables == expected_variables => {
                pending.try_reserve(1).map_err(|_| {
                    GenericCallError::Host("type comparison allocation failed".into())
                })?;
                pending.push((got_body, expected_body));
            }
            _ => return Ok(false),
        }
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::*;

    fn nested_list(mut ty: Type, depth: usize) -> Type {
        for _ in 0..depth {
            ty = Type::List(Box::new(ty));
        }
        ty
    }

    #[test]
    fn exact_substitution_comparison_and_drop_are_stack_safe() {
        std::thread::Builder::new()
            .name("generic-call-deep-types".to_owned())
            .stack_size(128 * 1024)
            .spawn(|| {
                let depth = 1_024;
                let parameter = nested_list(Type::Param("t".to_owned()), depth);
                let concrete = nested_list(Type::I64, depth);
                let callable = Type::Forall {
                    vars: vec!["t".to_owned()],
                    body: Box::new(Type::Fn {
                        params: vec![parameter],
                        ret: Box::new(Type::Param("t".to_owned())),
                    }),
                };
                let product_names = HashMap::new();
                let implementation_index = HashMap::new();
                let facts = GenericFacts {
                    traits: &[],
                    products: &[],
                    implementations: &[],
                    product_names: &product_names,
                    implementation_index: &implementation_index,
                };
                let exact = resolve_exact(
                    &callable,
                    vec![TypeSubstitution {
                        parameter: "t".to_owned(),
                        ty: Type::I64,
                    }],
                    std::slice::from_ref(&concrete),
                    &[],
                    &facts,
                )
                .expect("resolve deep exact call");
                assert_eq!(exact.parameters, [concrete]);
                assert_eq!(exact.result, Type::I64);
            })
            .expect("spawn deep generic call")
            .join()
            .expect("deep generic call completes");
    }
}
