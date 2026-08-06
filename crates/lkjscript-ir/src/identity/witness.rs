use super::{encoder::Encoder, types::ty};
use crate::MemoryWitnessDescriptor;
use lkjscript_contracts::{
    ExecutableMemoryWitnessDependency, ExecutableMemoryWitnessFacts, ExecutableMemoryWitnessRole,
    ExecutableMemoryWitnessTarget,
};

pub(super) fn descriptor(out: &mut Encoder, value: &MemoryWitnessDescriptor) {
    let MemoryWitnessDescriptor {
        id,
        group,
        ordinal,
        facts,
        ty: value_ty,
        dependencies,
        representation,
    } = value;
    out.fixed(&id.bytes());
    out.fixed(&group.bytes());
    out.u16(*ordinal);
    facts_value(out, id.bytes(), *ordinal, facts, dependencies);
    ty(out, value_ty);
    out.option(representation.as_ref(), |out, value| out.u64(value.raw()));
}

pub(super) fn facts_value(
    out: &mut Encoder,
    id: [u8; 32],
    ordinal: u16,
    facts: &ExecutableMemoryWitnessFacts,
    dependencies: &[ExecutableMemoryWitnessDependency],
) {
    let ExecutableMemoryWitnessFacts {
        semantic_type,
        semantic_contract,
        semantic,
        mode,
        capabilities,
        domain,
        root,
        copy,
        drop,
        equality,
        codec,
        list_element,
        size,
        alignment,
        contains_borrow,
        contains_dynamic_owner,
        portability,
        contention,
        operations,
    } = facts;
    let _ = (
        semantic_type,
        semantic_contract,
        mode,
        capabilities,
        domain,
        root,
        copy,
        drop,
        equality,
        codec,
        list_element,
        size,
        alignment,
        contains_borrow,
        contains_dynamic_owner,
        portability,
        contention,
        operations,
    );
    for dependency in dependencies {
        dependency_shape(dependency);
    }
    match lkjscript_contracts::canonical_semantic_descriptor(semantic) {
        Ok(bytes) => out.bytes(&bytes),
        Err(_) => out.fail("verified SSA semantic witness is invalid"),
    }
    let member = lkjscript_contracts::ExecutableMemoryWitnessGroupMember {
        id,
        ordinal,
        semantic_identity: facts.semantic_type,
        facts: facts.clone(),
        dependencies: dependencies.to_vec(),
    };
    out.bytes(
        &lkjscript_contracts::canonical_executable_memory_witness_group_descriptor(
            false,
            &[member],
        ),
    );
}

fn dependency_shape(value: &ExecutableMemoryWitnessDependency) {
    let ExecutableMemoryWitnessDependency { role, target } = value;
    match role {
        ExecutableMemoryWitnessRole::ListElement => {}
        ExecutableMemoryWitnessRole::ProductField {
            product,
            field,
            source_order,
        } => {
            let _ = (product, field, source_order);
        }
        ExecutableMemoryWitnessRole::EnumVariantField {
            enumeration,
            variant,
            field,
            variant_source_order,
            field_source_order,
        } => {
            let _ = (
                enumeration,
                variant,
                field,
                variant_source_order,
                field_source_order,
            );
        }
        ExecutableMemoryWitnessRole::TypeArgument { constructor, index } => {
            let _ = (constructor, index);
        }
    }
    match target {
        ExecutableMemoryWitnessTarget::LocalMember(ordinal) => {
            let _ = ordinal;
        }
        ExecutableMemoryWitnessTarget::ExternalMember { group, member } => {
            let _ = (group, member);
        }
    }
}

pub(super) fn group(out: &mut Encoder, value: &crate::MemoryWitnessGroupDescriptor) {
    let crate::MemoryWitnessGroupDescriptor {
        id,
        recursive,
        members,
    } = value;
    out.fixed(&id.bytes());
    out.bool(*recursive);
    out.sequence(members, |out, member| {
        let crate::MemoryWitnessGroupMember {
            witness,
            ordinal,
            semantic_identity,
        } = member;
        out.fixed(&witness.bytes());
        out.u16(*ordinal);
        out.fixed(semantic_identity);
    });
}
