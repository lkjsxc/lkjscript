use super::witness::*;

const DOMAIN: &[u8] = b"lkjscript.executable-memory-witness\0canonical-platform-contract";

pub fn canonical_executable_memory_witness(
    facts: &ExecutableMemoryWitnessFacts,
    dependencies: &[[u8; 32]],
) -> Vec<u8> {
    let mut output = Encoder::new();
    output.bytes(DOMAIN);
    output.bytes(&facts.semantic_type);
    output.bytes(&facts.semantic_contract);
    output.byte(mode(facts.mode));
    output.boolean(facts.capabilities.inline);
    output.boolean(facts.capabilities.static_value);
    output.boolean(facts.capabilities.unique);
    output.boolean(facts.capabilities.ordinary_region);
    output.boolean(facts.capabilities.sealed_region);
    output.boolean(facts.capabilities.borrow);
    output.boolean(facts.capabilities.process_codec);
    output.boolean(facts.capabilities.list_element);
    output.boolean(facts.capabilities.equality);
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
        output.bytes(dependency);
    }
    output.finish()
}

struct Encoder(Vec<u8>);

impl Encoder {
    fn new() -> Self {
        Self(Vec::new())
    }

    fn byte(&mut self, value: u8) {
        self.0.push(value);
    }

    fn boolean(&mut self, value: bool) {
        self.byte(u8::from(value));
    }

    fn u16(&mut self, value: u16) {
        self.bytes(&value.to_be_bytes());
    }

    fn u64(&mut self, value: u64) {
        self.bytes(&value.to_be_bytes());
    }

    fn sequence_len(&mut self, value: usize) {
        self.u64(u64::try_from(value).unwrap_or(u64::MAX));
    }

    fn bytes(&mut self, value: &[u8]) {
        self.0.extend_from_slice(value);
    }

    fn finish(self) -> Vec<u8> {
        self.0
    }
}

mod tags;
use tags::*;
