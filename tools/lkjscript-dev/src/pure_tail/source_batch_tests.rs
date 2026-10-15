//! Mutate actual source output; no synthetic successful invocation or evaluator observations.

use super::*;

fn altered_field(stdout: &str, operation: &str, name: &str, replacement: &str) -> String {
    let mut parsed = records(stdout.as_bytes()).expect("actual compact response");
    let record = parsed
        .iter_mut()
        .find(|record| record.operation == operation)
        .expect("actual owner record");
    let field = record
        .fields
        .iter_mut()
        .find(|field| field.name == name)
        .expect("actual owner field");
    field.value = replacement.to_owned();
    // Preserve all fields and record order while encoding only the deliberate mutation.
    parsed
        .iter()
        .map(|record| {
            let mut line = record.operation.clone();
            for field in &record.fields {
                line.push(' ');
                line.push_str(&field.name);
                line.push('=');
                line.push_str(&serde_json::to_string(&field.value).expect("quoted compact value"));
            }
            line.push('\n');
            line
        })
        .collect()
}

#[test]
#[ignore = "requires LKJSCRIPT_PURE_TAIL_BATCH_FIXTURE from an actual completed current pure-tail gate"]
fn actual_source_batch_rows_reject_missing_results_identity_forgery_and_live_resources() {
    let path = PathBuf::from(
        std::env::var_os("LKJSCRIPT_PURE_TAIL_BATCH_FIXTURE")
            .expect("explicit completed actual gate receipt"),
    );
    assert!(path.is_absolute());
    let receipt: Receipt = serde_json::from_slice(
        &process::read_bounded(&path, 16 * 1024 * 1024).expect("actual receipt bytes"),
    )
    .expect("current receipt");
    assert_eq!(receipt.status, "fresh passed");
    assert!(receipt.cleanup_complete);
    let root = path.parent().expect("actual evidence root");
    read(&receipt, root).expect("actual current batch evidence before any mutation");
    let evidence = receipt
        .source_batch
        .as_ref()
        .expect("actual batch evidence");
    let cases = cases().expect("independent expected input matrix");
    let smoke = command_records(&receipt, root, evidence.smoke_command_index)
        .expect("actual ordinary smoke");
    let bytes =
        process::read_bounded(&root.join(RESULT_FILE), MAXIMUM_BATCH_BYTES).expect("actual rows");
    let rows = decode_responses(&bytes).expect("complete actual ordered responses");
    inspect_rows(&cases, &rows, &smoke, evidence).expect("actual semantic and work matrix");

    assert!(
        decode_responses(&bytes[..bytes.len() - 1]).is_err(),
        "truncated final row"
    );
    let mut extra = bytes.clone();
    extra.push(b'\n');
    assert!(decode_responses(&extra).is_err(), "extra blank row");
    let mut missing = Vec::new();
    for (index, line) in bytes[..bytes.len() - 1]
        .split(|byte| *byte == b'\n')
        .enumerate()
    {
        if index == 23 {
            let original = std::str::from_utf8(line).unwrap();
            let altered = original.replace(",\"result_json\":null", "");
            assert_ne!(
                original, altered,
                "actual overflow explicitly carries its null result"
            );
            missing.extend_from_slice(altered.as_bytes());
        } else {
            missing.extend_from_slice(line);
        }
        missing.push(b'\n');
    }
    assert!(
        decode_responses(&missing).is_err(),
        "overflow still requires its explicit null result"
    );
    let mut duplicate = bytes.clone();
    let insertion = duplicate.iter().position(|byte| *byte == b'{').unwrap() + 1;
    duplicate.splice(insertion..insertion, b"\"id\":\"forged\",".iter().copied());
    assert!(
        decode_responses(&duplicate).is_err(),
        "duplicate response field"
    );

    let reject = |fault: Vec<Response>| {
        assert!(inspect_rows(&cases, &fault, &smoke, evidence).is_err());
    };
    let mut fault = rows.clone();
    fault.swap(0, 1);
    reject(fault);
    let mut fault = rows.clone();
    fault[110].result_json = Some("5".to_owned());
    reject(fault); // A final, otherwise healthy row cannot disappear after the trap.
    let mut fault = rows.clone();
    fault[23].result_json = Some("[]".to_owned());
    reject(fault);
    let mut fault = rows.clone();
    fault[23].exit_code = 4;
    reject(fault);
    let mut fault = rows.clone();
    fault[24].stdout = altered_field(&fault[24].stdout, "execution", "target", "map");
    reject(fault); // The first healthy invocation after overflow must retain its own target.
    let mut fault = rows.clone();
    fault[0].stdout = altered_field(&fault[0].stdout, "execution", "differential", "different");
    reject(fault);
    let mut fault = rows.clone();
    fault[0].stdout = altered_field(&fault[0].stdout, "authority", "revision", "rev_foreign");
    reject(fault);
    let mut fault = rows.clone();
    let actual = records(fault[0].stdout.as_bytes()).unwrap();
    let changed = (integer_field(&actual, "production-instructions").unwrap() + 1).to_string();
    fault[0].stdout = altered_field(
        &fault[0].stdout,
        "execution",
        "production-instructions",
        &changed,
    );
    reject(fault);
    for (name, field, replacement) in [
        (
            "production-observation",
            "live_operands_after",
            serde_json::json!(1),
        ),
        (
            "reference-observation",
            "live_control_frames_after",
            serde_json::json!(1),
        ),
        (
            "executor-observation",
            "dispatch_open",
            serde_json::json!(true),
        ),
        (
            "executor-observation",
            "remaining_workers",
            serde_json::json!(1),
        ),
        (
            "production-observation",
            "instructions",
            serde_json::json!(0),
        ),
    ] {
        let parsed = records(rows[0].stdout.as_bytes()).unwrap();
        let mut actual = observation(&parsed, name).unwrap();
        actual[field] = replacement;
        let mut fault = rows.clone();
        fault[0].stdout = altered_field(&fault[0].stdout, "execution", name, &actual.to_string());
        reject(fault);
    }
    let parsed = records(rows[0].stdout.as_bytes()).unwrap();
    let actual = field(&parsed, "execution", "production-observation").unwrap();
    let duplicate = format!("{{\"live_operands_after\":1,{}", &actual[1..]);
    let mut fault = rows.clone();
    fault[0].stdout = altered_field(
        &fault[0].stdout,
        "execution",
        "production-observation",
        &duplicate,
    );
    reject(fault); // A later zero cannot hide a duplicate nonquiescent observation field.
    let mut fault = rows.clone();
    fault[23].stdout = altered_field(&fault[23].stdout, "diagnostic", "notes", "[]");
    reject(fault);
    let mut fault = receipt.clone();
    fault.source_batch.as_mut().unwrap().command_index = evidence.smoke_command_index;
    assert!(
        read(&fault, root).is_err(),
        "process observation cannot stand for a batch row"
    );
    let mut fault = receipt.clone();
    let key = format!("batch-{}-110-ordered", evidence.command_index);
    fault.outcomes.get_mut(&key).unwrap()["expected"] = serde_json::json!("5");
    assert!(
        read(&fault, root).is_err(),
        "receipt outcome must bind exact actual row bytes"
    );
    read(&receipt, root).expect("immutable original remains healthy after in-memory faults");
    assert_eq!(
        process::read_bounded(&root.join(RESULT_FILE), MAXIMUM_BATCH_BYTES).unwrap(),
        bytes
    );
}
