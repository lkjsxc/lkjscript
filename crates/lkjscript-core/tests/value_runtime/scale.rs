use lkjscript_core::{
    InlineStructuralValue, SemanticPayload, SemanticValue, StructuralKind, StructuralValueError,
    StructuralValueLimit, StructuralValueRuntime, StructuralValueRuntimeLimits,
};

use super::support::value_type;

#[test]
fn default_flat_image_accepts_sixty_five_thousand_nodes() -> Result<(), StructuralValueError> {
    let product_type = value_type(109, 110, StructuralKind::Product)?;
    let integer_type = value_type(111, 112, StructuralKind::I64)?;
    let mut runtime = StructuralValueRuntime::new(StructuralValueRuntimeLimits::default())?;
    let owner = runtime
        .publish_owned(value_with_nodes(65_536, product_type, integer_type))
        .map_err(|failure| failure.error)?;
    assert_eq!(
        runtime.value_node(owner, product_type)?.image_node_count(),
        65_536
    );
    let copy = runtime.clone_owned(owner, product_type)?;
    assert_eq!(runtime.metrics().clone_nodes, 65_536);
    runtime.drop_owned(copy, product_type)?;
    runtime.drop_owned(owner, product_type)?;
    let domains = runtime.domain_metrics();
    assert_eq!(domains.live_domains, 0);
    assert_eq!(domains.peak_live_domains, 2);
    runtime.verify_empty()
}

#[test]
fn four_thousand_node_boundary_is_not_a_default_ceiling() -> Result<(), StructuralValueError> {
    let product_type = value_type(113, 114, StructuralKind::Product)?;
    let integer_type = value_type(115, 116, StructuralKind::I64)?;
    for nodes in [4_095, 4_096, 4_097, 16_384] {
        let mut runtime = StructuralValueRuntime::new(StructuralValueRuntimeLimits::default())?;
        let owner = runtime
            .publish_owned(value_with_nodes(nodes, product_type, integer_type))
            .map_err(|failure| failure.error)?;
        assert_eq!(
            runtime.value_node(owner, product_type)?.image_node_count(),
            nodes
        );
        runtime.drop_owned(owner, product_type)?;
        runtime.verify_empty()?;
    }
    let limits = StructuralValueRuntimeLimits {
        max_tree_nodes: 4_096,
        ..StructuralValueRuntimeLimits::default()
    };
    let mut runtime = StructuralValueRuntime::new(limits)?;
    let failure = match runtime.publish_owned(value_with_nodes(4_097, product_type, integer_type)) {
        Err(failure) => failure,
        Ok(owner) => {
            runtime.drop_owned(owner, product_type)?;
            return Err(StructuralValueError::InvariantViolation);
        }
    };
    assert_eq!(
        failure.error,
        StructuralValueError::LimitExceeded(StructuralValueLimit::TreeNodes)
    );
    assert_eq!(runtime.metrics().live_objects, 0);
    runtime.verify_empty()
}

#[test]
fn released_domain_capacity_is_live_not_cumulative() -> Result<(), StructuralValueError> {
    let integer_type = value_type(117, 118, StructuralKind::I64)?;
    let mut runtime = StructuralValueRuntime::new(StructuralValueRuntimeLimits::default())?;
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
    nodes: u32,
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
