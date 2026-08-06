use super::{encoder::Encoder, function, memory, metadata};
use crate::Program;

pub(super) fn encode_program(out: &mut Encoder, value: &Program) {
    let Program {
        prepared_identity: _,
        memory: memory_value,
        region_products,
        sources,
        products,
        enums,
        traits,
        implementations,
        functions,
        main,
    } = value;
    memory::structural_memory(out, memory_value);
    out.sequence(region_products, |out, value| {
        let crate::RegionProductMetadata { product, identity } = value;
        out.u64(product.raw());
        out.fixed(&identity.bytes());
    });
    out.sequence(sources, metadata::source);
    out.sequence(products, metadata::product);
    out.sequence(enums, metadata::enumeration);
    out.sequence(traits, metadata::trait_value);
    out.sequence(implementations, metadata::implementation);
    out.sequence(functions, function::function);
    out.u32(main.raw());
}
