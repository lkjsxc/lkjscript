use super::{encoder::Encoder, types};
use crate::*;
use lkjscript_contracts::{
    ExecutableMemoryWitnessDependency, ExecutableMemoryWitnessFacts, ExecutableMemoryWitnessRole,
    ExecutableMemoryWitnessTarget,
};

pub(super) fn installed(out: &mut Encoder, value: &InstalledMemoryWitness) {
    let InstalledMemoryWitness {
        id,
        group,
        ordinal,
        facts,
        dependencies,
        value_kind,
    } = value;
    out.fixed(&id.bytes());
    out.fixed(&group.bytes());
    out.u64(*ordinal);
    facts_value(out, id.bytes(), *ordinal, facts, dependencies);
    witness_kind(out, *value_kind);
}
fn facts_value(
    out: &mut Encoder,
    id: [u8; 32],
    ordinal: u64,
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
        Err(_) => out.fail("validated bytecode semantic witness is invalid"),
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
pub(super) fn group(out: &mut Encoder, value: &InstalledMemoryWitnessGroup) {
    let InstalledMemoryWitnessGroup {
        id,
        recursive,
        members,
    } = value;
    out.fixed(&id.bytes());
    out.bool(*recursive);
    out.sequence(members, |out, member| {
        let InstalledMemoryWitnessGroupMember {
            witness,
            ordinal,
            semantic_identity,
        } = member;
        out.fixed(&witness.bytes());
        out.u64(*ordinal);
        out.fixed(semantic_identity);
    });
}
fn witness_kind(out: &mut Encoder, value: MemoryWitnessValueKind) {
    match value {
        MemoryWitnessValueKind::Unit => out.tag(0),
        MemoryWitnessValueKind::Bool => out.tag(1),
        MemoryWitnessValueKind::I64 => out.tag(2),
        MemoryWitnessValueKind::F64 => out.tag(3),
        MemoryWitnessValueKind::List => out.tag(4),
        MemoryWitnessValueKind::Structural(id) => {
            out.tag(5);
            out.u64(id.raw());
        }
        MemoryWitnessValueKind::Unsupported => out.tag(6),
    }
}
pub(super) fn parameter(out: &mut Encoder, value: &MemoryWitnessParameter) {
    let MemoryWitnessParameter {
        parameter,
        operations,
    } = value;
    out.u64(*parameter);
    out.sequence(operations, |out, value| {
        types::witness_operation(out, *value)
    });
}
pub(super) fn call_site(out: &mut Encoder, value: &CallWitnessSite) {
    let CallWitnessSite {
        offset,
        callee,
        bindings,
    } = value;
    out.u64(*offset);
    out.u64(*callee);
    out.sequence(bindings, |out, value| {
        let MemoryWitnessBinding { parameter, witness } = value;
        out.u64(*parameter);
        out.u64(*witness);
    });
}
