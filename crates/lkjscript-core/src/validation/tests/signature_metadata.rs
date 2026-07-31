use super::*;

#[test]
fn type_variable_and_copy_metadata_are_exact_and_nonoverlapping() {
    let mut arity = unit_chunk();
    arity.main.parameter_type_variables = vec![Some(0)];
    assert!(error(arity).contains("type-variable parameter metadata does not match arity"));

    let mut copy = unit_chunk();
    copy.main.parameter_copy_kinds = vec![Some(crate::StructuralKind::I64)];
    assert!(error(copy).contains("copy parameter metadata does not match arity"));

    let mut overlap = unit_chunk();
    overlap.main.return_type_variable = Some(0);
    overlap.main.return_structural = Some(crate::StructuralRepresentationId::new(0));
    assert!(error(overlap).contains("type-variable return overlaps exact metadata"));
}
