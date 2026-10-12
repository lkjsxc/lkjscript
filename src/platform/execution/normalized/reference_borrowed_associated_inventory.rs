//! Borrowed callable inventory over independently derived scoped signatures.
use super::*;
use crate::platform::execution::normalized::reference_types::associated_preparation::{
    Function, Proof,
};

struct SignatureView<'a> {
    parameters: &'a [(TypeObjectDigest, ParameterUse)],
    result: TypeObjectDigest,
    source: Option<u32>,
    is_task: bool,
}

fn signature(
    closure: &mut Closure<'_, '_>,
    scope: DeclarationReference,
    value: SignatureView<'_>,
) -> Result<(), ExecutionError> {
    for (ty, mode) in value.parameters {
        closure.tick()?;
        if closure.scoped_owned(*ty, Some(scope))? && *mode == ParameterUse::Unrestricted {
            return Err(reject());
        }
    }
    if let Some(source) = value.source {
        let (ty, mode) = value.parameters.get(source as usize).ok_or_else(reject)?;
        if value.is_task
            || *mode != ParameterUse::Borrow
            || !closure.scoped_owned(*ty, Some(scope))?
            || !closure.scoped_owned(value.result, Some(scope))?
        {
            return Err(reject());
        }
    }
    Ok(())
}

fn function(
    closure: &mut Closure<'_, '_>,
    scope: DeclarationReference,
    owner: &crate::platform::kernel::DeclarationRecord,
    value: &Function,
) -> Result<(), ExecutionError> {
    let (parameters, result, source, effect, formals) = match &owner.payload {
        DeclarationPayload::Function(function) => (
            &function.parameters,
            function.result,
            function.result_borrow,
            &function.effect,
            function.implementation_parameters.as_slice(),
        ),
        DeclarationPayload::External(function) => (
            &function.parameters,
            function.result,
            None,
            &FunctionEffect::Pure,
            &[][..],
        ),
        _ => return Err(reject()),
    };
    for formal in formals {
        closure.tick()?;
        for _ in 0..formal
            .type_arguments
            .len()
            .saturating_add(formal.name.as_str().len())
        {
            closure.tick()?;
        }
    }
    if parameters.len() != value.parameters.len() {
        return Err(reject());
    }
    if source.is_some() {
        for _ in parameters {
            closure.tick()?;
        }
    }
    let source_position = source
        .map(|source| {
            parameters
                .iter()
                .position(|parameter| *parameter == source)
                .ok_or_else(reject)
        })
        .transpose()?;
    let effect_matches = match effect {
        FunctionEffect::Pure => {
            value.effect.parameters.is_empty() && value.effect.requirements.is_empty()
        }
        FunctionEffect::Task {
            requirements,
            effect_parameters,
        } => {
            for _ in requirements {
                closure.tick()?;
            }
            for _ in effect_parameters {
                closure.tick()?;
            }
            requirements == &value.effect.requirements
                && effect_parameters == &value.effect.parameters
        }
    };
    if owner.header.owner != OwnerKey::Declaration(scope.declaration)
        || value.occurrences.get(&owner.header.owner) != Some(&owner.header.contract_version)
        || formals != value.implementation_templates.as_slice()
        || value.is_task != matches!(effect, FunctionEffect::Task { .. })
        || !effect_matches
        || (value.requires_associated_generation
            && owner.header.contract_version
                != crate::platform::kernel::contract::GRAPH_CONTRACT_VERSION)
        || value.roots.get(&(owner.header.owner, result)) != Some(&value.result)
        || source_position != value.result_borrow.map(|position| position as usize)
    {
        return Err(reject());
    }
    for (position, (id, (ty, mode))) in parameters.iter().zip(&value.parameters).enumerate() {
        closure.tick()?;
        let OwnerRecord::Parameter(parameter) =
            closure.owner(scope.package, OwnerKey::Parameter(*id))?
        else {
            return Err(reject());
        };
        if parameter.header.owner != OwnerKey::Parameter(*id)
            || parameter.parent != ParameterParent::Function(scope.declaration)
            || parameter.use_mode != *mode
            || value.occurrences.get(&parameter.header.owner)
                != Some(&parameter.header.contract_version)
            || value.roots.get(&(parameter.header.owner, parameter.ty)) != Some(ty)
            || (value.result_borrow == Some(position as u32)
                && parameter.resource_requirement.is_some())
            || (value.is_task
                && *mode == ParameterUse::Borrow
                && closure.scoped_owned(*ty, Some(scope))?
                && (owner.header.contract_version
                    < crate::platform::kernel::contract::SHARE_GRAPH_CONTRACT_VERSION
                    || parameter.header.contract_version
                        < crate::platform::kernel::contract::SHARE_GRAPH_CONTRACT_VERSION))
        {
            return Err(reject());
        }
    }
    signature(
        closure,
        scope,
        SignatureView {
            parameters: &value.parameters,
            result: value.result,
            source: value.result_borrow,
            is_task: value.is_task,
        },
    )
}

fn operands(
    closure: &mut Closure<'_, '_>,
    values: &[crate::platform::kernel::ImplementationOperand],
    depth: usize,
) -> Result<(), ExecutionError> {
    if depth > crate::platform::kernel::contract::MAXIMUM_TYPE_DEPTH
        || values.len() > crate::platform::kernel::contract::MAXIMUM_CHILDREN
    {
        return Err(reject());
    }
    for value in values {
        closure.tick()?;
        if let crate::platform::kernel::ImplementationOperand::Concrete {
            type_arguments,
            implementations,
            ..
        } = value
        {
            for _ in type_arguments {
                closure.tick()?;
            }
            operands(closure, implementations, depth + 1)?;
        }
    }
    Ok(())
}

pub(super) fn inventory(
    closure: &mut Closure<'_, '_>,
    proof: &Proof,
) -> Result<(), ExecutionError> {
    allocate::<&crate::platform::kernel::KernelSnapshot>(
        &mut closure.allocated,
        closure.snapshots.len(),
    )?;
    let snapshots: Vec<_> = closure.snapshots.values().copied().collect();
    for snapshot in snapshots {
        for (key, record) in &snapshot.owners {
            closure.tick()?;
            let (OwnerKey::Declaration(id), OwnerRecord::Declaration(owner)) = (key, record) else {
                continue;
            };
            let scope = DeclarationReference {
                package: snapshot.root.package_id,
                declaration: *id,
            };
            match &owner.payload {
                DeclarationPayload::Function(_) | DeclarationPayload::External(_) => {
                    function(
                        closure,
                        scope,
                        owner,
                        proof.symbolic.get(&scope).ok_or_else(reject)?,
                    )?;
                }
                DeclarationPayload::OwnedContract(contract) => {
                    for _ in &contract.type_parameters {
                        closure.tick()?;
                    }
                    for member in &contract.associated_types {
                        closure.tick()?;
                        for _ in member.name.as_str().bytes() {
                            closure.tick()?;
                        }
                    }
                    for method in &contract.methods {
                        closure.tick()?;
                        for _ in method.name.as_str().bytes() {
                            closure.tick()?;
                        }
                        for _ in &method.parameters {
                            closure.tick()?;
                        }
                        if let FunctionEffect::Task {
                            requirements,
                            effect_parameters,
                        } = &method.effect
                        {
                            for _ in effect_parameters {
                                closure.tick()?;
                            }
                            for _ in requirements {
                                closure.tick()?;
                            }
                        }
                    }
                    if contract != proof.contract_template(scope, closure.control)? {
                        return Err(reject());
                    }
                    for method in &contract.methods {
                        closure.tick()?;
                        let value = proof
                            .contract_methods
                            .get(&(scope, method.id))
                            .ok_or_else(reject)?;
                        if value.occurrences.get(key) != Some(&owner.header.contract_version)
                            || value.is_task != matches!(method.effect, FunctionEffect::Task { .. })
                            || value.result_borrow != method.result_borrow
                        {
                            return Err(reject());
                        }
                        signature(
                            closure,
                            scope,
                            SignatureView {
                                parameters: &value.parameters,
                                result: value.result,
                                source: value.result_borrow,
                                is_task: value.is_task,
                            },
                        )?;
                    }
                }
                DeclarationPayload::OwnedImplementation(implementation) => {
                    let contract =
                        proof.contract_template(implementation.contract, closure.control)?;
                    if implementation.methods.len() != contract.methods.len() {
                        return Err(reject());
                    }
                    for (index, mapping) in implementation.methods.iter().enumerate() {
                        closure.tick()?;
                        for _ in &mapping.type_arguments {
                            closure.tick()?;
                        }
                        operands(closure, &mapping.implementations, 0)?;
                        if (index > 0 && implementation.methods[index - 1].method >= mapping.method)
                            || mapping
                                != proof.mapping_template_at(
                                    scope,
                                    mapping.method,
                                    closure.control,
                                )?
                        {
                            return Err(reject());
                        }
                        let value = &proof
                            .symbolic_mapping(scope, mapping.method, closure.control)?
                            .signature;
                        signature(
                            closure,
                            scope,
                            SignatureView {
                                parameters: &value.parameters,
                                result: value.result,
                                source: value.result_borrow,
                                is_task: value.is_task,
                            },
                        )?;
                    }
                }
                _ => {}
            }
        }
    }
    Ok(())
}
