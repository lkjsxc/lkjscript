mod decoded;
mod encoder;
mod function;
mod metadata;
mod structural;
mod types;
mod witness;

use super::ValidatedChunk;
use crate::Chunk;
use encoder::Encoder;

const DOMAIN: &[u8] = b"lkjscript.validated-bytecode-identity\0canonical-binary";

/// Exact target-neutral content identity of one validated bytecode chunk.
#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ValidatedBytecodeIdentity([u8; 32]);

impl ValidatedBytecodeIdentity {
    pub const fn bytes(self) -> [u8; 32] {
        self.0
    }
}

/// Compute the canonical identity accepted only through validated bytecode authority.
pub fn validated_bytecode_identity(
    value: &ValidatedChunk,
) -> crate::Result<ValidatedBytecodeIdentity> {
    let ValidatedChunk {
        chunk,
        main_instructions,
        proto_instructions,
    } = value;
    let mut out = Encoder::new(DOMAIN);
    encode_chunk(&mut out, chunk);
    decoded::instructions(&mut out, main_instructions);
    out.sequence(proto_instructions, |out, values| {
        decoded::instructions(out, values)
    });
    out.finish().map(ValidatedBytecodeIdentity)
}

fn encode_chunk(out: &mut Encoder, value: &Chunk) {
    let Chunk {
        prepared_identity: _,
        constants,
        protos,
        main,
        memory_plan,
        memory_witness_groups,
        memory_witnesses,
        structural_types,
        structural_layouts,
        structural_representations,
        structural_destinations,
        structural_destination_fields,
        structural_aggregate_fields,
        structural_payloads,
        required_capabilities,
        global_names,
        global_prototypes,
        products,
        product_fields,
        enums,
        enum_constructions,
        enum_variants,
        enum_fields,
    } = value;
    out.sequence(constants, constant);
    out.sequence(protos, function::function);
    function::function(out, main);
    out.option(memory_plan.as_ref(), |out, value| out.fixed(&value.bytes()));
    out.sequence(memory_witness_groups, witness::group);
    out.sequence(memory_witnesses, witness::installed);
    out.sequence(structural_types, structural::structural_type);
    out.sequence(structural_layouts, structural::layout);
    out.sequence(structural_representations, structural::representation);
    out.sequence(structural_destinations, structural::destination);
    out.sequence(structural_destination_fields, structural::destination_field);
    out.sequence(structural_aggregate_fields, structural::aggregate_field);
    out.sequence(structural_payloads, structural::payload);
    out.sequence(required_capabilities, |out, value| {
        types::capability(out, *value)
    });
    out.sequence(global_names, |out, value| out.string(value));
    out.sequence(global_prototypes, |out, value| {
        out.option(value.as_ref(), |out, value| out.u32(*value))
    });
    out.sequence(products, metadata::product);
    out.sequence(product_fields, metadata::product_field);
    out.sequence(enums, metadata::enumeration);
    out.sequence(enum_constructions, metadata::enum_construction);
    out.sequence(enum_variants, metadata::enum_variant);
    out.sequence(enum_fields, metadata::enum_field);
}
fn constant(out: &mut Encoder, value: &crate::Constant) {
    match value {
        crate::Constant::I64(value) => {
            out.tag(0);
            out.i64(*value);
        }
        crate::Constant::F64(value) => {
            out.tag(1);
            out.u64(value.to_bits());
        }
        crate::Constant::Str(value) => {
            out.tag(2);
            out.string(value);
        }
        crate::Constant::StaticBytes(value) => {
            out.tag(3);
            out.bytes(value);
        }
        crate::Constant::Symbol(value) => {
            out.tag(4);
            out.string(value);
        }
        crate::Constant::Proto(value) => {
            out.tag(5);
            out.u32(*value);
        }
    }
}

#[cfg(test)]
mod tests;
