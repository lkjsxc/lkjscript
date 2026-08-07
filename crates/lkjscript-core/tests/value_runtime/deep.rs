use lkjscript_core::{
    InlineStructuralValue, SemanticPayload, SemanticValue, StructuralKind, StructuralNodeView,
    StructuralValueError, StructuralValueRuntime,
};

use super::support::value_type;

const DEPTH: u16 = 12_000;

#[test]
fn deep_image_conversion_clone_export_and_release_are_iterative() -> Result<(), StructuralValueError>
{
    let product_type = value_type(101, 102, StructuralKind::Product)?;
    let integer_type = value_type(103, 104, StructuralKind::I64)?;
    let mut runtime = StructuralValueRuntime::new()?;

    let semantic = deep_value(product_type, integer_type);
    let owner = runtime
        .publish_owned(semantic)
        .map_err(|failure| failure.error)?;
    let root = runtime.value_node(owner, product_type)?;
    let mut node = root;
    for _ in 0..DEPTH {
        let StructuralNodeView::Product(fields) = node.payload() else {
            return Err(StructuralValueError::InvariantViolation);
        };
        assert_eq!(fields.len(), 1);
        let child = node
            .child(0)
            .ok_or(StructuralValueError::InvariantViolation)?;
        assert!(child.id() > node.id());
        node = child;
    }
    assert_eq!(
        node.payload(),
        StructuralNodeView::Inline(InlineStructuralValue::I64(7))
    );

    let copy = runtime.clone_owned(owner, product_type)?;
    let observed = runtime.export_semantic(copy, product_type)?;
    assert_eq!(consume_depth(observed)?, DEPTH);
    runtime.drop_owned(owner, product_type)?;

    let metrics = runtime.metrics();
    assert_eq!(metrics.clone_nodes, u64::from(DEPTH) + 1);
    assert_eq!(metrics.release_work, u64::from(DEPTH) + 1);
    runtime.verify_empty()
}

#[test]
fn deep_semantic_clone_and_drop_are_iterative() -> Result<(), StructuralValueError> {
    let product_type = value_type(105, 106, StructuralKind::Product)?;
    let integer_type = value_type(107, 108, StructuralKind::I64)?;
    let value = deep_value(product_type, integer_type);
    let cloned = value.clone();
    drop(value);
    assert_eq!(consume_depth(cloned)?, DEPTH);
    Ok(())
}

fn deep_value(
    product_type: lkjscript_core::StructuralType,
    integer_type: lkjscript_core::StructuralType,
) -> SemanticValue {
    let mut value = SemanticValue::new(
        integer_type,
        SemanticPayload::Inline(InlineStructuralValue::I64(7)),
    );
    for _ in 0..DEPTH {
        value = SemanticValue::new(product_type, SemanticPayload::Product(vec![value].into()));
    }
    value
}

fn consume_depth(mut value: SemanticValue) -> Result<u16, StructuralValueError> {
    let mut depth = 0_u16;
    loop {
        match value.payload {
            SemanticPayload::Product(mut fields) if fields.len() == 1 => {
                value = fields
                    .pop()
                    .ok_or(StructuralValueError::InvariantViolation)?;
                depth = depth
                    .checked_add(1)
                    .ok_or(StructuralValueError::ArithmeticOverflow)?;
            }
            SemanticPayload::Inline(InlineStructuralValue::I64(7)) => return Ok(depth),
            _ => return Err(StructuralValueError::InvariantViolation),
        }
    }
}
