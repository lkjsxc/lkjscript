use super::*;

pub(crate) const SOURCE: &str = r#"declarations.begin
(units (module create nominal-record-projection
  (owned-record create Packet (visibility private)
    (field create first (type I64)) (field create second (type I64))
    (field create payload (type ByteBuffer)))
  (function create make (visibility public) (effect pure)
    (parameter create bytes (type ByteBuffer) (use consume)) (returns Packet)
    (body (pack-owned (type Packet)
      (field second (i64 2)) (field payload (local bytes)) (field first (i64 1)))))
  (function create unpack (visibility public) (effect pure)
    (parameter create packet (type Packet) (use consume)) (returns I64)
    (body (unpack-owned (type Packet) (local packet)
      (field second (binding b (type I64))) (field first (binding a (type I64)))
      (field payload (binding bytes (type ByteBuffer)))
      (in (local a)))))
  (function create inspect (visibility public) (effect pure)
    (parameter create packet (type Packet) (use borrow)) (returns I64)
    (body (borrow-owned-field (type Packet) (local packet)
      (field payload (binding view (type ByteBuffer))) (in (i64 9)))))))
declarations.end"#;

pub(crate) fn authored_snapshot() -> KernelSnapshot {
    crate::platform::execution::normalized::tests::byte_buffer_tests::author_only(SOURCE)
        .expect("native authored nominal record fixture")
}

pub(crate) fn function(snapshot: &KernelSnapshot, name: &str) -> DeclarationId {
    snapshot
        .owners
        .iter()
        .find_map(|(owner, record)| match (owner, record) {
            (OwnerKey::Declaration(id), OwnerRecord::Declaration(record))
                if record.name.as_str() == name =>
            {
                Some(*id)
            }
            _ => None,
        })
        .expect("authored nominal record function")
}

#[test]
fn nominal_record_oracle_preserves_order_binding_identity_and_exact_field_relations() {
    let snapshot = authored_snapshot();
    let temporary = tempfile::tempdir().unwrap();
    let project = temporary.path().join("meaning");
    GraphRepository::create(&project, &snapshot, None).unwrap();
    let before = std::fs::read(project.join("HEAD")).unwrap();
    for (name, expected_forms, expected_roles, relation_kind) in [
        (
            "make",
            vec!["pack_owned_record", "i64", "local", "i64"],
            vec![
                "function_body",
                "owned_product_field",
                "owned_product_field",
                "owned_product_field",
            ],
            "nominal_field_construction",
        ),
        (
            "unpack",
            vec![
                "unpack_owned_record",
                "local",
                "binding:owned_unpack",
                "binding:owned_unpack",
                "binding:owned_unpack",
                "local",
            ],
            vec![
                "function_body",
                "owned_product_source",
                "owned_product_binding",
                "owned_product_binding",
                "owned_product_binding",
                "owned_product_body",
            ],
            "nominal_field_access",
        ),
        (
            "inspect",
            vec![
                "borrow_owned_record_field",
                "local",
                "binding:owned_borrow",
                "i64",
            ],
            vec![
                "function_body",
                "owned_product_source",
                "owned_product_binding",
                "owned_product_body",
            ],
            "nominal_field_access",
        ),
    ] {
        let function = function(&snapshot, name);
        let oracle = function_definition_oracle(&project, &function.to_string()).unwrap();
        assert_eq!(
            oracle
                .body_preorder
                .iter()
                .map(|owner| owner.form.as_str())
                .collect::<Vec<_>>(),
            expected_forms
        );
        assert_eq!(
            oracle
                .body_preorder
                .iter()
                .map(|owner| owner.role.as_str())
                .collect::<Vec<_>>(),
            expected_roles
        );
        let root = oracle.body_root.parse::<ExpressionId>().unwrap();
        let OwnerRecord::Expression(expression) = &snapshot.owners[&OwnerKey::Expression(root)]
        else {
            panic!("expression");
        };
        let expected_fields = match &expression.operation {
            ExpressionOperation::PackOwnedRecord { fields, .. } => {
                fields.iter().map(|field| field.field).collect::<Vec<_>>()
            }
            ExpressionOperation::UnpackOwnedRecord { fields, .. } => {
                fields.iter().map(|field| field.field).collect::<Vec<_>>()
            }
            ExpressionOperation::BorrowOwnedRecordField { field, .. } => vec![*field],
            _ => panic!("nominal representation operation"),
        };
        let field_targets = oracle
            .relations
            .iter()
            .filter(|relation| relation.kind == relation_kind)
            .map(|relation| relation.target.clone())
            .collect::<BTreeSet<_>>();
        assert_eq!(
            field_targets,
            expected_fields
                .iter()
                .map(|field| format!("owner:{}/{}", field.package, field.field))
                .collect()
        );
        for field in expected_fields {
            assert_eq!(field.package, snapshot.root.package_id);
            assert!(matches!(
                snapshot.owners[&OwnerKey::Field(field.field)],
                OwnerRecord::Field(_)
            ));
        }
        if name == "inspect" {
            assert_eq!(
                function_extraction_oracle(
                    &project,
                    &function.to_string(),
                    &oracle.body_preorder[1].owner
                )
                .unwrap_err()
                .code,
                "contributor_extraction_owned_borrow"
            );
        } else {
            assert_eq!(
                function_extraction_oracle(
                    &project,
                    &function.to_string(),
                    &oracle.body_preorder[1].owner,
                )
                .unwrap_err()
                .code,
                "contributor_extraction_owned_product"
            );
        }
    }
    assert_eq!(std::fs::read(project.join("HEAD")).unwrap(), before);
}
