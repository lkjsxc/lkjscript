use lkjscript_core::{
    InlineStructuralValue, SemanticPayload, SemanticValue, StructuralKind, StructuralValueError,
    StructuralValueRuntime,
};

use super::support::value_type;

#[test]
fn flat_image_crosses_former_node_and_field_boundaries() -> Result<(), StructuralValueError> {
    let product_type = value_type(109, 110, StructuralKind::Product)?;
    let integer_type = value_type(111, 112, StructuralKind::I64)?;
    let mut runtime = StructuralValueRuntime::new()?;
    let owner = runtime
        .publish_owned(value_with_nodes(65_537, product_type, integer_type))
        .map_err(|failure| failure.error)?;
    assert_eq!(
        runtime.value_node(owner, product_type)?.image_node_count(),
        65_537
    );
    let copy = runtime.clone_owned(owner, product_type)?;
    assert_eq!(runtime.metrics().clone_nodes, 65_537);
    runtime.drop_owned(copy, product_type)?;
    runtime.drop_owned(owner, product_type)?;
    let domains = runtime.domain_metrics();
    assert_eq!(domains.live_domains, 0);
    assert_eq!(domains.peak_live_domains, 2);
    runtime.verify_empty()
}

#[test]
fn payload_crosses_former_one_megabyte_boundary() -> Result<(), StructuralValueError> {
    let bytes_type = value_type(113, 114, StructuralKind::ByteVector)?;
    let mut runtime = StructuralValueRuntime::new()?;
    let payload = vec![7; 1_000_001];
    let owner = runtime
        .publish_owned(SemanticValue::new(
            bytes_type,
            SemanticPayload::ByteVector(payload.clone()),
        ))
        .map_err(|failure| failure.error)?;
    let exported = runtime.export_semantic(owner, bytes_type)?;
    assert!(matches!(
        exported.payload,
        SemanticPayload::ByteVector(bytes) if bytes == payload
    ));
    runtime.verify_empty()
}

#[test]
fn released_domain_capacity_is_live_not_cumulative() -> Result<(), StructuralValueError> {
    let integer_type = value_type(117, 118, StructuralKind::I64)?;
    let mut runtime = StructuralValueRuntime::new()?;
    for value in 0..8_192_i64 {
        let owner = runtime
            .publish_owned(SemanticValue::new(
                integer_type,
                SemanticPayload::Inline(InlineStructuralValue::I64(value)),
            ))
            .map_err(|failure| failure.error)?;
        runtime.drop_owned(owner, integer_type)?;
    }
    let domains = runtime.domain_metrics();
    assert_eq!(domains.domains_created, 8_192);
    assert_eq!(domains.domains_released, 8_192);
    assert_eq!(domains.slots_reused, 8_191);
    assert_eq!(domains.live_domains, 0);
    assert_eq!(domains.peak_live_domains, 1);
    runtime.verify_empty()
}

fn value_with_nodes(
    nodes: u64,
    product_type: lkjscript_core::StructuralType,
    integer_type: lkjscript_core::StructuralType,
) -> SemanticValue {
    assert!(nodes > 1);
    let mut remaining = nodes - 1;
    let mut fields = Vec::new();
    while remaining > 0 {
        if remaining == 1 {
            fields.push(integer(integer_type));
            remaining = 0;
        } else {
            let group_nodes = remaining.min(1_025);
            let leaves = (1..group_nodes)
                .map(|_| integer(integer_type))
                .collect::<Vec<_>>();
            fields.push(SemanticValue::new(
                product_type,
                SemanticPayload::Product(leaves.into()),
            ));
            remaining -= group_nodes;
        }
    }
    SemanticValue::new(product_type, SemanticPayload::Product(fields.into()))
}

fn integer(integer_type: lkjscript_core::StructuralType) -> SemanticValue {
    SemanticValue::new(
        integer_type,
        SemanticPayload::Inline(InlineStructuralValue::I64(7)),
    )
}
