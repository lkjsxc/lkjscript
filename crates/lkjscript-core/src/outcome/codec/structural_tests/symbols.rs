#![allow(clippy::expect_used)]

use super::*;

#[test]
fn structural_symbol_traversal_and_equality_are_canonical() {
    fn symbols(zeta: u64, alpha: u64) -> SemanticValue {
        value(
            40,
            StructuralKind::Product,
            SemanticPayload::Product(
                vec![
                    value(
                        41,
                        StructuralKind::Static,
                        SemanticPayload::Static(StaticStructuralLeaf::Symbol(zeta)),
                    ),
                    value(
                        42,
                        StructuralKind::Static,
                        SemanticPayload::Static(StaticStructuralLeaf::Symbol(alpha)),
                    ),
                    value(
                        43,
                        StructuralKind::Static,
                        SemanticPayload::Static(StaticStructuralLeaf::Symbol(zeta)),
                    ),
                ]
                .into(),
            ),
        )
    }
    fn text(index: u64, zeta: u64, alpha: u64) -> crate::Result<&'static str> {
        if index == zeta {
            Ok("zeta")
        } else if index == alpha {
            Ok("alpha")
        } else {
            Err(crate::Error::msg("unexpected structural symbol"))
        }
    }

    let mut first_order = Vec::new();
    let first = OwnedValue::from_structural(symbols(7, 2), StructuralSnapshotLimits::DEFAULT)
        .and_then(|owned| {
            owned.retain_symbols(|index| {
                first_order.push(index);
                text(index, 7, 2)
            })
        })
        .expect("first structural symbols");
    let mut second_order = Vec::new();
    let second = OwnedValue::from_structural(symbols(9, 3), StructuralSnapshotLimits::DEFAULT)
        .and_then(|owned| {
            owned.retain_symbols(|index| {
                second_order.push(index);
                text(index, 9, 3)
            })
        })
        .expect("second structural symbols");
    assert_eq!(first_order, vec![7, 2, 7]);
    assert_eq!(second_order, vec![9, 3, 9]);
    assert_eq!(first, second);

    let outcome = ExecutionOutcome::Returned(first);
    let wire = encode_execution_outcome(&outcome, 2 * 1024 * 1024).expect("encode symbols");
    assert_eq!(
        decode_execution_outcome(&wire, 2 * 1024 * 1024).expect("decode symbols"),
        outcome
    );

    let symbol = OwnedValue::from_structural(
        value(
            44,
            StructuralKind::Static,
            SemanticPayload::Static(StaticStructuralLeaf::Symbol(12)),
        ),
        StructuralSnapshotLimits::DEFAULT,
    )
    .and_then(|owned| owned.retain_symbols(|_| Ok("standalone-symbol")))
    .expect("standalone structural symbol");
    assert_eq!(symbol.as_str(), Some("standalone-symbol"));
    assert_eq!(format!("{symbol:?}"), "\"standalone-symbol\"");
}
