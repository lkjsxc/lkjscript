use super::witness::*;

mod dependency;
mod dependency_validation;
mod group;
mod group_encoding;
mod group_graph;
mod group_validation;
mod semantic;
mod semantic_encoding;
mod semantic_tags;
mod semantic_validation;

#[cfg(test)]
mod semantic_tests;

pub use dependency::*;
pub use dependency_validation::*;
pub use group::*;
pub use group_encoding::*;
pub use group_validation::*;
pub use semantic::*;
pub use semantic_encoding::*;
pub use semantic_validation::*;

pub fn canonical_executable_memory_witness_dependencies(
    dependencies: &[ExecutableMemoryWitnessDependency],
) -> Vec<u8> {
    let mut output = Encoder::new();
    output.sequence_len(dependencies.len());
    for dependency in dependencies {
        encode_dependency(&mut output, dependency);
    }
    output.finish()
}

fn encode_dependency(output: &mut Encoder, dependency: &ExecutableMemoryWitnessDependency) {
    encode_role(output, &dependency.role);
    match dependency.target {
        ExecutableMemoryWitnessTarget::LocalMember(ordinal) => {
            output.byte(0);
            output.u64(ordinal);
        }
        ExecutableMemoryWitnessTarget::ExternalMember { group, member } => {
            output.byte(1);
            output.bytes(&group);
            output.bytes(&member);
        }
    }
}

fn encode_role(output: &mut Encoder, role: &ExecutableMemoryWitnessRole) {
    match role {
        ExecutableMemoryWitnessRole::ListElement => output.byte(0),
        ExecutableMemoryWitnessRole::ProductField {
            product,
            field,
            source_order,
        } => {
            output.byte(1);
            output.bytes(product);
            output.bytes(field);
            output.u64(*source_order);
        }
        ExecutableMemoryWitnessRole::EnumVariantField {
            enumeration,
            variant,
            field,
            variant_source_order,
            field_source_order,
        } => {
            output.byte(2);
            output.bytes(enumeration);
            output.bytes(variant);
            output.bytes(field);
            output.u64(*variant_source_order);
            output.u64(*field_source_order);
        }
        ExecutableMemoryWitnessRole::TypeArgument { constructor, index } => {
            output.byte(3);
            output.bytes(constructor);
            output.u64(*index);
        }
    }
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
        let value = match u64::try_from(value) {
            Ok(value) => value,
            Err(_) => unreachable!("host usize exceeds canonical u64 length"),
        };
        self.u64(value);
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
