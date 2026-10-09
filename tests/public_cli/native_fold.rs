//! Independently imported native folds, with no runtime-only operation or timing gate.
use super::*;
use serde_json::json;

#[path = "native_fold/cases.rs"]
mod cases;
#[path = "native_fold/fixture.rs"]
mod fixture;
use fixture::Folds;

#[test]
fn native_blocked_folds_import_exactly_and_preserve_complete_results_after_source_removal() {
    let folds = Folds::new();
    for detached in [false, true] {
        if detached {
            folds.detach();
        }
        for items in cases::inputs() {
            for mode in 0..3 {
                let arguments = json!([items, {"count":3,"sum":11}, mode]);
                let (records, result) = folds.run("imported", &arguments, detached, true);
                assert_eq!(result.unwrap(), cases::expected(&items, 3, 11));
                if !detached {
                    assert_eq!(
                        compact_field(compact_record(&records, "execution"), "differential"),
                        "equal"
                    );
                }
            }
        }
        for mode in 0..2 {
            let (_, empty) = folds.run("trap-probe", &json!([[], mode]), detached, true);
            assert_eq!(empty.unwrap(), json!(29));
            let (failure, missing) = folds.run("trap-probe", &json!([[1], mode]), detached, false);
            assert!(missing.is_none());
            let diagnostic = compact_record(&failure, "diagnostic");
            assert_eq!(compact_field(diagnostic, "code"), "normalized_list_index");
            if detached {
                cases::failed_join(diagnostic);
            }
            let (_, recovered) = folds.run("trap-probe", &json!([[], mode]), detached, true);
            assert_eq!(recovered.unwrap(), json!(29));
        }
    }
    folds.verify_identity();
}

#[test]
fn native_blocked_folds_reject_wrong_contract_without_changing_accepted_consumer() {
    let folds = Folds::new();
    let before = folds.consumer.revision();
    let source = format!(
        r#"request base={before}
declarations.begin
(units
  (use std builtin)
  (use folds {} {})
  (module create rejected
    (function create bad (visibility private)
      (returns I64) (effect pure)
      (body (call folds::fold-eight (types Bool I64) (list I64 (i64 1))
        (i64 0) (function-value std::add))))))
declarations.end
"#,
        folds.package, folds.package_revision
    );
    let wrong = folds.consumer.input("wrong.lkjc", &source);
    let rejected = folds.consumer.plan(&wrong, false);
    assert_eq!(
        compact_field(compact_record(&rejected, "diagnostic"), "code"),
        "kernel_type_argument"
    );
    assert_eq!(
        compact_field(compact_record(&rejected, "diagnostic"), "class"),
        "semantic"
    );
    assert_eq!(folds.consumer.revision(), before);
    let (records, result) = folds.run(
        "imported",
        &json!([[7,8,9],{"count":3,"sum":11},1]),
        false,
        true,
    );
    assert_eq!(result.unwrap(), json!({"count":6,"sum":133}));
    assert_eq!(
        compact_field(compact_record(&records, "execution"), "differential"),
        "equal"
    );
    folds.verify_identity();
}

#[test]
fn native_blocked_folds_admit_the_complete_argument_before_invoking_a_callback() {
    let folds = Folds::new();
    for detached in [false, true] {
        if detached {
            folds.detach();
        }
        for mode in 0..2 {
            let (failure, absent) = folds.run(
                "trap-probe",
                &json!([[1, "invalid"], mode]),
                detached,
                false,
            );
            assert!(absent.is_none());
            let diagnostic = compact_record(&failure, "diagnostic");
            assert_eq!(compact_field(diagnostic, "class"), "source");
            assert_eq!(compact_field(diagnostic, "code"), "normalized_json_type");
            let (_, recovered) = folds.run("trap-probe", &json!([[], mode]), detached, true);
            assert_eq!(recovered.unwrap(), json!(29));
        }
    }
    folds.verify_identity();
}
