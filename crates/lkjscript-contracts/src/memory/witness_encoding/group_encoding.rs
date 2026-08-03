use super::*;

const GROUP_DOMAIN: &[u8] =
    b"lkjscript.executable-memory-witness-group\0canonical-platform-contract";
const MEMBER_DOMAIN: &[u8] =
    b"lkjscript.executable-memory-witness-member\0canonical-platform-contract";

pub fn canonical_executable_memory_witness_group_descriptor(
    recursive: bool,
    members: &[ExecutableMemoryWitnessGroupMember],
) -> Vec<u8> {
    let mut output = Encoder::new();
    output.bytes(GROUP_DOMAIN);
    output.boolean(recursive);
    output.sequence_len(members.len());
    for member in members {
        output.u16(member.ordinal);
        output.bytes(&member.semantic_identity);
        encode_facts(&mut output, &member.facts, &member.dependencies);
    }
    output.finish()
}

pub fn executable_memory_witness_group_id(
    recursive: bool,
    members: &[ExecutableMemoryWitnessGroupMember],
) -> [u8; 32] {
    crate::sha256(&canonical_executable_memory_witness_group_descriptor(
        recursive, members,
    ))
}

pub fn executable_memory_witness_member_id(
    group: [u8; 32],
    ordinal: u16,
    semantic_identity: [u8; 32],
) -> [u8; 32] {
    let mut output = Encoder::new();
    output.bytes(MEMBER_DOMAIN);
    output.bytes(&group);
    output.u16(ordinal);
    output.bytes(&semantic_identity);
    crate::sha256(&output.finish())
}

pub(super) fn encode_facts(
    output: &mut Encoder,
    facts: &ExecutableMemoryWitnessFacts,
    dependencies: &[ExecutableMemoryWitnessDependency],
) {
    output.bytes(&facts.semantic_type);
    output.bytes(&facts.semantic_contract);
    output.byte(mode(facts.mode));
    for value in [
        facts.capabilities.inline,
        facts.capabilities.static_value,
        facts.capabilities.unique,
        facts.capabilities.ordinary_region,
        facts.capabilities.sealed_region,
        facts.capabilities.borrow,
        facts.capabilities.process_codec,
        facts.capabilities.list_element,
        facts.capabilities.equality,
    ] {
        output.boolean(value);
    }
    output.byte(domain(facts.domain));
    output.byte(root(facts.root));
    output.byte(copy(facts.copy));
    output.byte(drop_route(facts.drop));
    output.byte(equality(facts.equality));
    output.byte(codec(facts.codec));
    output.byte(list_element(facts.list_element));
    match facts.size {
        MemoryWitnessSize::Fixed(bytes) => {
            output.byte(0);
            output.u64(bytes);
        }
        MemoryWitnessSize::CheckedDynamic => output.byte(1),
        MemoryWitnessSize::Caller => output.byte(2),
    }
    output.u16(facts.alignment);
    output.boolean(facts.contains_borrow);
    output.boolean(facts.contains_dynamic_owner);
    output.byte(portability(facts.portability));
    output.byte(contention(facts.contention));
    output.sequence_len(facts.operations.len());
    for operation in &facts.operations {
        output.byte(operation_tag(*operation));
    }
    output.sequence_len(dependencies.len());
    for dependency in dependencies {
        encode_dependency(output, dependency);
    }
}
