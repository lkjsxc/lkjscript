use super::{instruction_error, types::*, Kind, State};
use crate::{Chunk, DecodedInstruction, FunctionProto, Op, Result};

pub(super) fn apply(
    chunk: &Chunk,
    proto: &FunctionProto,
    instruction: DecodedInstruction,
    state: &mut State,
) -> Result<()> {
    let op = instruction.op();
    match op {
        Op::MakeProduct => {
            let product_index = instruction_operand(proto, instruction)?;
            let product = chunk.products.get(product_index).ok_or_else(|| {
                instruction_error(
                    proto,
                    op,
                    instruction.offset(),
                    "product metadata is missing",
                )
            })?;
            if !product.region {
                return Err(instruction_error(
                    proto,
                    op,
                    instruction.offset(),
                    "product construction requires structural or invocation-region metadata",
                ));
            }
            for field in product.region_fields.iter().rev() {
                let actual = pop(state, proto, instruction)?;
                if !region_field_matches(*field, actual) {
                    return Err(instruction_error(
                        proto,
                        op,
                        instruction.offset(),
                        "region-product construction field route mismatch",
                    ));
                }
            }
            state.stack.push(Kind::RegionProduct(product.id));
        }
        Op::LoadProductField => {
            let descriptor = product_descriptor(chunk, proto, instruction)?;
            let product = pop(state, proto, instruction)?;
            let region = chunk
                .products
                .get(descriptor.product.index())
                .is_some_and(|metadata| metadata.region);
            if !region {
                return Err(instruction_error(
                    proto,
                    op,
                    instruction.offset(),
                    "product projection requires structural or invocation-region metadata",
                ));
            }
            expect_product(product, descriptor.product, true, proto, instruction)?;
            let result = chunk
                .products
                .get(descriptor.product.index())
                .and_then(|metadata| metadata.region_fields.get(usize::from(descriptor.field)))
                .copied()
                .and_then(region_field_kind)
                .unwrap_or(Kind::Any);
            state.stack.push(result);
        }
        Op::WithProductField => {
            let descriptor = product_descriptor(chunk, proto, instruction)?;
            let replacement = pop(state, proto, instruction)?;
            let product = pop(state, proto, instruction)?;
            let region = chunk
                .products
                .get(descriptor.product.index())
                .is_some_and(|metadata| metadata.region);
            if !region {
                return Err(instruction_error(
                    proto,
                    op,
                    instruction.offset(),
                    "product update requires structural or invocation-region metadata",
                ));
            }
            expect_product(product, descriptor.product, true, proto, instruction)?;
            {
                let field = chunk
                    .products
                    .get(descriptor.product.index())
                    .and_then(|metadata| metadata.region_fields.get(usize::from(descriptor.field)))
                    .copied()
                    .ok_or_else(|| {
                        instruction_error(
                            proto,
                            op,
                            instruction.offset(),
                            "region-product replacement field route is missing",
                        )
                    })?;
                if !region_field_matches(field, replacement) {
                    return Err(instruction_error(
                        proto,
                        op,
                        instruction.offset(),
                        "region-product replacement field route mismatch",
                    ));
                }
            }
            state.stack.push(Kind::RegionProduct(descriptor.product));
        }
        Op::MakeEnum | Op::IsEnumVariant | Op::LoadEnumField => {
            return super::enums::apply(chunk, proto, instruction, state);
        }
        _ => unreachable!("opcode dispatched to wrong validation family"),
    }
    Ok(())
}

fn region_field_kind(field: crate::RegionProductFieldKind) -> Option<Kind> {
    Some(match field {
        crate::RegionProductFieldKind::Unit => Kind::Unit,
        crate::RegionProductFieldKind::Bool => Kind::Bool,
        crate::RegionProductFieldKind::I64 => Kind::I64,
        crate::RegionProductFieldKind::F64 => Kind::F64,
        crate::RegionProductFieldKind::List => Kind::List,
        crate::RegionProductFieldKind::Product(product) => Kind::RegionProduct(product),
    })
}

fn region_field_matches(field: crate::RegionProductFieldKind, actual: Kind) -> bool {
    actual == Kind::Any || region_field_kind(field) == Some(actual)
}
