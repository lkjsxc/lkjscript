use crate::codegen::*;

pub(in crate::codegen) fn install_enum_metadata(
    chunk: &mut Chunk,
    program: &lkjscript_ir::Program,
) -> Result<()> {
    for definition in &program.enums {
        let type_parameter_count = u8::try_from(definition.type_parameters.len())
            .map_err(|_| Error::msg("enum substitution arity exceeds bytecode u8"))?;
        chunk.enums.push(BytecodeEnumMetadata {
            id: BytecodeEnumId::new(definition.id.bytes()),
            name: definition.name.clone(),
            type_parameter_count,
            layout: BytecodeLayoutId::new(definition.layout.identity.bytes()),
            variants: definition
                .variants
                .iter()
                .map(|variant| BytecodeEnumVariantMetadata {
                    id: BytecodeVariantId::new(variant.id.bytes()),
                    name: variant.name.clone(),
                    physical_tag: variant.physical_tag,
                    fields: variant
                        .fields
                        .iter()
                        .map(|field| BytecodeEnumFieldMetadata {
                            id: BytecodeVariantFieldId::new(field.id.bytes()),
                            name: field.name.clone(),
                            traced: field.traced,
                        })
                        .collect(),
                })
                .collect(),
        });
    }
    Ok(())
}

pub(in crate::codegen) fn intern_enum_construction(
    chunk: &mut Chunk,
    enum_id: lkjscript_ir::EnumId,
    variant: lkjscript_ir::VariantId,
    layout: lkjscript_ir::RuntimeLayoutId,
    substitution_arity: usize,
) -> Result<u16> {
    let descriptor = EnumConstructionRef {
        enum_id: BytecodeEnumId::new(enum_id.bytes()),
        variant: BytecodeVariantId::new(variant.bytes()),
        layout: BytecodeLayoutId::new(layout.bytes()),
        substitution_arity: u8::try_from(substitution_arity)
            .map_err(|_| Error::msg("enum substitution arity exceeds u8"))?,
    };
    intern(
        &mut chunk.enum_constructions,
        descriptor,
        "enum construction",
    )
}

pub(in crate::codegen) fn intern_enum_variant(
    chunk: &mut Chunk,
    enum_id: lkjscript_ir::EnumId,
    variant: lkjscript_ir::VariantId,
    layout: lkjscript_ir::RuntimeLayoutId,
) -> Result<u16> {
    let descriptor = EnumVariantRef {
        enum_id: BytecodeEnumId::new(enum_id.bytes()),
        variant: BytecodeVariantId::new(variant.bytes()),
        layout: BytecodeLayoutId::new(layout.bytes()),
    };
    intern(&mut chunk.enum_variants, descriptor, "enum variant")
}

pub(in crate::codegen) fn intern_enum_field(
    chunk: &mut Chunk,
    ids: (
        lkjscript_ir::EnumId,
        lkjscript_ir::VariantId,
        lkjscript_ir::VariantFieldId,
    ),
    layout: lkjscript_ir::RuntimeLayoutId,
) -> Result<u16> {
    let descriptor = EnumFieldRef {
        enum_id: BytecodeEnumId::new(ids.0.bytes()),
        variant: BytecodeVariantId::new(ids.1.bytes()),
        field: BytecodeVariantFieldId::new(ids.2.bytes()),
        layout: BytecodeLayoutId::new(layout.bytes()),
    };
    intern(&mut chunk.enum_fields, descriptor, "enum field")
}

fn intern<T: Copy + PartialEq>(table: &mut Vec<T>, descriptor: T, name: &str) -> Result<u16> {
    if let Some(index) = table.iter().position(|existing| *existing == descriptor) {
        return u16::try_from(index)
            .map_err(|_| Error::msg(format!("{name} descriptor exceeds u16")));
    }
    let index = u16::try_from(table.len())
        .map_err(|_| Error::msg(format!("too many {name} descriptors")))?;
    table.push(descriptor);
    Ok(index)
}
