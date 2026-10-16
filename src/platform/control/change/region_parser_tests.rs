//! Compact and native region forms have one authored semantic encoding.
use super::*;
use crate::platform::change::canonical_authored_intent_bytes;

fn prefix() -> String {
    format!(
        "request base=rev_{}\ncreate.module as=$module name=regions\ncreate.function as=$function module=$module name=subject visibility=public result=i64 effect=pure body=$body\nadd.parameter as=$cap function=$function name=cap type=i64\n",
        "96".repeat(32)
    )
}

#[test]
fn region_native_forms_match_independent_compact_requests() {
    let cases = [
        (
            "(region-with (binding arena (as $arena) (type i64)) (i64 512) (in (i64 7)))",
            "expression.region-with as=$body binding=$arena name=arena type=i64 capacity=$capacity body=$value\nexpression.i64 as=$capacity value=512\nexpression.i64 as=$value value=7\n",
        ),
        (
            "(region-alloc (type i64) $cap (i64 7))",
            "expression.region-alloc as=$body type=i64 capability=$cap value=$value\nexpression.i64 as=$value value=7\n",
        ),
        (
            "(region-read (type i64) $cap (i64 7))",
            "expression.region-read as=$body type=i64 capability=$cap pointer=$value\nexpression.i64 as=$value value=7\n",
        ),
        (
            "(region-write (type i64) $cap (i64 7) (i64 9))",
            "expression.region-write as=$body type=i64 capability=$cap pointer=$value value=$next\nexpression.i64 as=$value value=7\nexpression.i64 as=$next value=9\n",
        ),
        (
            "(region-freeze (type i64) $cap)",
            "expression.region-freeze as=$body type=i64 capability=$cap\n",
        ),
        (
            "(region-resolve (type i64) $cap (i64 7))",
            "expression.region-resolve as=$body type=i64 capability=$cap offset=$value\nexpression.i64 as=$value value=7\n",
        ),
        (
            "(region-offset (i64 7))",
            "expression.region-offset as=$body pointer=$value\nexpression.i64 as=$value value=7\n",
        ),
        (
            "(region-mmap-save $cap pkg_96000000000000000000000000000001/req_96000000000000000000000000000002 pkg_96000000000000000000000000000001/op_96000000000000000000000000000003 (i64 1) (i64 2) (i64 3))",
            "expression.region-mmap-save as=$body capability=$cap requirement=pkg_96000000000000000000000000000001/req_96000000000000000000000000000002 operation=pkg_96000000000000000000000000000001/op_96000000000000000000000000000003 space=$space key=$key expected=$expected\nexpression.i64 as=$space value=1\nexpression.i64 as=$key value=2\nexpression.i64 as=$expected value=3\n",
        ),
        (
            "(region-mmap-load (type i64) $cap pkg_96000000000000000000000000000001/req_96000000000000000000000000000002 pkg_96000000000000000000000000000001/op_96000000000000000000000000000003 (i64 1) (i64 2) (i64 3))",
            "expression.region-mmap-load as=$body type=i64 capability=$cap requirement=pkg_96000000000000000000000000000001/req_96000000000000000000000000000002 operation=pkg_96000000000000000000000000000001/op_96000000000000000000000000000003 space=$space key=$key digest=$digest\nexpression.i64 as=$space value=1\nexpression.i64 as=$key value=2\nexpression.i64 as=$digest value=3\n",
        ),
    ];
    for (native, compact) in cases {
        let prefix = prefix();
        let compact =
            decode_compact_change("region-flat.lkjc", format!("{prefix}{compact}").as_bytes())
                .unwrap();
        let native = decode_compact_change(
            "region-native.lkjc",
            format!("{prefix}expression.block as=$body\n{native}\nexpression.end\n").as_bytes(),
        )
        .unwrap();
        assert_eq!(
            canonical_authored_intent_bytes(&native.semantic).unwrap(),
            canonical_authored_intent_bytes(&compact.semantic).unwrap()
        );
        assert_eq!(native.request_commitment, compact.request_commitment);
    }
}

#[test]
fn region_compact_preflight_admits_all_operands_before_decoding() {
    for fields in ["capacity=$body body=$unit", "capacity=$unit body=$body"] {
        let source = format!(
            "{}expression.region-with as=$body binding=$arena name=arena type=i64 {fields}\nexpression.unit as=$unit\n",
            prefix()
        );
        let error = decode_compact_change("region-cycle.lkjc", source.as_bytes()).unwrap_err();
        assert_eq!(error[0].code, "change_expression_cycle");
    }
    let source = format!(
        "{}expression.region-write as=$body type=i64 capability=$cap pointer=$missing value=$value\nexpression.i64 as=$value value=7\n",
        prefix()
    );
    assert_eq!(
        decode_compact_change("region-undefined.lkjc", source.as_bytes()).unwrap_err()[0].code,
        "change_expression_undefined"
    );
}
