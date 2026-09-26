//! Different lexical spellings must not turn one complete owner into two partial updates.
use super::*;

const PAIR: &str = r#"declarations.begin
(units
  (module create overlap (as $module)
    (function create pair (as $function) (visibility public)
      (returns (record (left I64) (right I64))) (effect pure)
      (body (record structural (field left (i64 1)) (field right (i64 2)))))
    (function create neighbor (as $neighbor) (visibility private) (returns Text) (effect pure)
      (body (text "neighbor")))
    (constant create marker (as $marker) (visibility private) (type I64) (value (i64 99)))))
declarations.end
"#;

#[test]
fn literal_edit_rejects_overlapping_complete_native_owners_across_scopes_and_blocks() {
    let fixture = Fixture::from_source(PAIR);
    let base = fixture.view().revision();
    for separated in [false, true] {
        for right in [2, 20] {
            // Nested and root-level names differ, but the exact declaration is the same.
            // With right=20, applying base-relative patches would incorrectly produce (10,20),
            // which is neither complete proposed body. The unchanged second body is ambiguous too.
            let boundary = if separated {
                ")\ndeclarations.end\ndeclarations.begin\n(units"
            } else {
                ""
            };
            let source = format!(
                r#"request base={base}
declarations.begin
(units
  (module edit {module} overlap
    (function edit {function} pair (visibility public)
      (returns (record (left I64) (right I64))) (effect pure)
      (body (record structural (field left (i64 10)) (field right (i64 2))))))
  {boundary}
  (function edit {function} pair (in {module}) (visibility public)
    (returns (record (left I64) (right I64))) (effect pure)
    (body (record structural (field left (i64 1)) (field right (i64 {right}))))))
declarations.end
"#,
                module = fixture.module,
                function = fixture.function
            );
            let result = decode_compact_change_in_repository(
                "overlapping-edits.lkjc",
                source.as_bytes(),
                &fixture.repository,
            );
            assert!(
                result.is_err(),
                "overlapping exact owner was normalized: separate_blocks={separated} right={right}"
            );
            let errors = result.unwrap_err();
            assert!(
                errors.iter().any(|e| e.code == "change_unit_duplicate"),
                "{errors:#?}"
            );
            assert_eq!(fixture.view().revision(), base);
        }
    }
}
