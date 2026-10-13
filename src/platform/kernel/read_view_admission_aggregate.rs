//! Declaration tables retain several precise inline contexts. This inventory
//! proof grants no context or readable permission to an authored occurrence.
use super::super::*;
use super::read_view_admission as views;
use super::{
    AssociatedTypeContext, OwnedAssociatedAdmission, OwnedTypeOccurrence, TypeParameterContext,
};
use crate::platform::diagnostic::Diagnostic;
use std::collections::BTreeSet;

pub(crate) fn validate(
    read: &(impl ExpressionRead + ?Sized),
    reference: DeclarationReference,
    record: &OwnerRecord,
    roots: &[TypeObjectDigest],
    admission: &OwnedAssociatedAdmission,
) -> Result<(), Diagnostic> {
    admission.step(read)?;
    if reference.package != read.package_id()
        || record.header().owner != OwnerKey::Declaration(reference.declaration)
        || read
            .owner(OwnerKey::Declaration(reference.declaration))?
            .as_ref()
            != Some(record)
    {
        return Err(views::reject(
            "aggregate inventory differs from its exact canonical owning declaration",
        ));
    }
    let OwnerRecord::Declaration(declaration) = record else {
        return Err(views::reject(
            "aggregate inventory requires its canonical declaration",
        ));
    };
    let generation = declaration.header.contract_version;
    let ordinary = OwnedTypeOccurrence::authored(Some(reference), generation);
    let mut pending = Vec::new();
    let mut push = |ty, occurrence| -> Result<(), Diagnostic> {
        admission.step(read)?;
        admission.reserve::<(TypeObjectDigest, OwnedTypeOccurrence)>(1)?;
        pending.push((ty, occurrence));
        Ok(())
    };
    match &declaration.payload {
        DeclarationPayload::OwnedContract(contract) => {
            super::super::owned_contract::validate_contract_at_with(
                read, reference, contract, admission,
            )?;
            for method in &contract.methods {
                admission.step(read)?;
                let occurrence = OwnedTypeOccurrence {
                    parameters: TypeParameterContext::Authored(reference),
                    associated: AssociatedTypeContext::ContractMethod {
                        contract: reference,
                        method: method.id,
                    },
                    consumer_generation: generation,
                };
                for parameter in &method.parameters {
                    push(parameter.ty, occurrence)?;
                }
                push(method.result.ty, occurrence)?;
            }
        }
        DeclarationPayload::OwnedImplementation(implementation) => {
            super::super::owned_contract::validate_implementation_at_with_admission(
                read,
                reference,
                implementation,
                admission,
            )?;
            push(implementation.self_type, ordinary)?;
            for ty in &implementation.type_arguments {
                push(*ty, ordinary)?;
            }
            for prerequisite in &implementation.implementation_parameters {
                push(prerequisite.self_type, ordinary)?;
                for ty in &prerequisite.type_arguments {
                    push(*ty, ordinary)?;
                }
            }
            for method in &implementation.methods {
                for ty in &method.type_arguments {
                    push(*ty, ordinary)?;
                }
                for operand in &method.implementations {
                    for node in operand.walk() {
                        for ty in node.type_arguments() {
                            push(*ty, ordinary)?;
                        }
                    }
                }
            }
            for binding in &implementation.associated_types {
                let occurrence = if matches!(
                    binding.value,
                    AssociatedTypeBindingValue::ReadViewFamily { .. }
                ) {
                    OwnedTypeOccurrence {
                        associated: AssociatedTypeContext::AssociatedBinding {
                            implementation: reference,
                            member: binding.member,
                        },
                        ..ordinary
                    }
                } else {
                    ordinary
                };
                push(binding.ty(), occurrence)?;
            }
        }
        _ => {
            return Err(views::reject(
                "aggregate inventory is not a contract or implementation declaration",
            ));
        }
    }
    let mut seen = BTreeSet::new();
    let mut admissible = BTreeSet::new();
    while let Some((ty, occurrence)) = pending.pop() {
        admission.step(read)?;
        if seen.contains(&(ty, occurrence)) {
            continue;
        }
        admission.reserve::<(TypeObjectDigest, OwnedTypeOccurrence)>(1)?;
        seen.insert((ty, occurrence));
        admission.validate_occurrence_at(read, &[ty], occurrence)?;
        admission.reserve::<TypeObjectDigest>(1)?;
        admissible.insert(ty);
        let object = read
            .type_object(ty)?
            .ok_or_else(|| views::reject("missing aggregate source type closure"))?;
        admission.reserve::<TypeObjectDigest>(object.child_type_count())?;
        for child in object.child_types() {
            admission.step(read)?;
            admission.reserve::<(TypeObjectDigest, OwnedTypeOccurrence)>(1)?;
            pending.push((child, occurrence));
        }
        if matches!(object.form, TypeForm::ReadViewApplied { .. }) {
            let layout = views::layout(read, ty, occurrence, admission)?;
            // Disposable nominal fields may be derived objects not in the raw
            // source reader. Admit them under exactly this originating equation.
            let overlay = super::super::owned_contract::AppliedTypeRead {
                read,
                types: &layout.types,
            };
            for field in &layout.fields {
                admission.step(&overlay)?;
                admission.validate_occurrence_at(&overlay, &[field.ty], occurrence)?;
                admission.reserve::<TypeObjectDigest>(1)?;
                admissible.insert(field.ty);
            }
            for (digest, object) in &layout.types {
                admission.step(&overlay)?;
                admission.validate_occurrence_at(&overlay, &[*digest], occurrence)?;
                admission.reserve::<TypeObjectDigest>(1)?;
                admissible.insert(*digest);
                admission.reserve::<TypeObjectDigest>(object.child_type_count())?;
                for child in object.child_types() {
                    admission.step(&overlay)?;
                    admission.reserve::<TypeObjectDigest>(1)?;
                    admissible.insert(child);
                }
            }
        }
    }
    let legacy = match declaration.payload {
        DeclarationPayload::OwnedContract(_) => {
            OwnedTypeOccurrence::contract(reference, generation)
        }
        _ => ordinary,
    };
    for ty in roots {
        admission.step(read)?;
        if views::contains(read, *ty, admission)? {
            if !admissible.contains(ty) {
                return Err(views::reject(
                    "table-only view type is not an exact canonical inline-root closure",
                ));
            }
        } else {
            admission.validate_occurrence_at(read, &[*ty], legacy)?;
        }
    }
    Ok(())
}
