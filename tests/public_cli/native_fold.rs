//! Independently imported native folds, with no runtime-only operation or timing gate.
use super::*;
use serde_json::json;

#[path = "native_fold/cases.rs"]
mod cases;
#[path = "native_fold/fixture.rs"]
mod fixture;
#[path = "native_fold/history.rs"]
mod history;
use fixture::Folds;

#[test]
fn native_blocked_folds_import_exactly_and_preserve_complete_results_after_source_removal() {
    let folds = Folds::new();
    let mut batch = Vec::new();
    for items in cases::inputs() {
        for mode in 0..3 {
            batch.push(BatchCase::success(
                format!("imported-{}-{mode}", items.len()),
                &json!([items, {"count":3,"sum":11}, mode]),
                cases::expected(&items, 3, 11),
            ));
        }
    }
    for mode in 0..2 {
        let mut empty =
            BatchCase::success(format!("trap-empty-{mode}"), &json!([[], mode]), json!(29));
        empty.target = Some("trap-probe".to_owned());
        batch.push(empty);
        // Failure after successful prefixes covers scalar, full-block and remainder paths.
        for prefix in [0, 3, 7, 8, 16] {
            let mut items = vec![1_i64; prefix];
            items.push(-1);
            let mut failure = BatchCase::failure(
                format!("trap-{mode}-{prefix}"),
                &json!([items, mode]),
                "semantic",
                "normalized_list_index",
            );
            failure.target = Some("trap-probe".to_owned());
            batch.push(failure);
            let mut recovered = BatchCase::success(
                format!("recovery-{mode}-{prefix}"),
                &json!([[1, 2, 3], mode]),
                json!(35),
            );
            recovered.target = Some("trap-probe".to_owned());
            batch.push(recovered);
        }
    }
    assert_eq!(batch.len(), 76, "every original row in each fold lane");
    folds.source_batch("attached-folds", "imported", &batch);
    folds.detach();
    folds.batch("detached-folds", "imported", &batch);
    let (_, smoke) = folds.run(
        "imported",
        &json!([[7,8,9],{"count":3,"sum":11},1]),
        true,
        true,
    );
    assert_eq!(smoke.unwrap(), json!({"count":6,"sum":133}));
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
    let mut batch = Vec::new();
    for mode in 0..2 {
        // The first item would trap, but complete raw admission must reject the last item first.
        batch.push(BatchCase::failure(
            format!("invalid-tail-{mode}"),
            &json!([[-1, "invalid"], mode]),
            "source",
            "normalized_json_type",
        ));
        batch.push(BatchCase::success(
            format!("after-invalid-tail-{mode}"),
            &json!([[1, 2, 3], mode]),
            json!(35),
        ));
    }
    assert_eq!(
        batch.len(),
        4,
        "every original admission/recovery row in each lane"
    );
    folds.source_batch("attached-admission", "trap-probe", &batch);
    folds.detach();
    folds.batch("detached-admission", "trap-probe", &batch);
    let (_, smoke) = folds.run("trap-probe", &json!([[1, 2, 3], 1]), true, true);
    assert_eq!(smoke.unwrap(), json!(35));
    folds.verify_identity();
}
