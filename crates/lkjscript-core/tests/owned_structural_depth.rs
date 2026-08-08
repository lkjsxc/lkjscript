#![allow(clippy::expect_used, clippy::panic)]

use std::num::NonZeroU64;
use std::time::Instant;

use lkjscript_core::{
    ExecutionOutcome, InlineStructuralValue, LayoutIdentity, OwnedValue, SemanticChildren,
    SemanticPayload, SemanticTypeIdentity, SemanticValue, StaticStructuralLeaf, StructuralKind,
    StructuralType, StructuralValueRuntime,
};

const SMALL_STACK_BYTES: usize = 128 * 1024;
const ORDINARY_DEPTH: usize = 2_048;
const STRESS_DEPTH: usize = 20_000;

#[derive(Clone, Copy)]
struct Types {
    unit: StructuralType,
    boolean: StructuralType,
    integer: StructuralType,
    float: StructuralType,
    string: StructuralType,
    path: StructuralType,
    bytes: StructuralType,
    byte_vector: StructuralType,
    static_value: StructuralType,
    product: StructuralType,
    enumeration: StructuralType,
}

impl Types {
    fn new() -> Self {
        Self {
            unit: ty(1, StructuralKind::Unit),
            boolean: ty(2, StructuralKind::Bool),
            integer: ty(3, StructuralKind::I64),
            float: ty(4, StructuralKind::F64),
            string: ty(5, StructuralKind::String),
            path: ty(6, StructuralKind::Path),
            bytes: ty(7, StructuralKind::Bytes),
            byte_vector: ty(8, StructuralKind::ByteVector),
            static_value: ty(9, StructuralKind::Static),
            product: ty(10, StructuralKind::Product),
            enumeration: ty(11, StructuralKind::Enum),
        }
    }
}

fn ty(id: u64, kind: StructuralKind) -> StructuralType {
    StructuralType::new(
        LayoutIdentity::new(NonZeroU64::new(id).expect("layout identity")),
        SemanticTypeIdentity::new(NonZeroU64::new(id + 1_000).expect("semantic type identity")),
        kind,
    )
}

fn alternating_value(depth: usize, leaf: i64) -> SemanticValue {
    let types = Types::new();
    let mut base = SemanticChildren::new();
    base.push(SemanticValue::new(
        types.unit,
        SemanticPayload::Inline(InlineStructuralValue::Unit),
    ));
    base.push(SemanticValue::new(
        types.boolean,
        SemanticPayload::Inline(InlineStructuralValue::Bool(true)),
    ));
    base.push(SemanticValue::new(
        types.integer,
        SemanticPayload::Inline(InlineStructuralValue::I64(leaf)),
    ));
    base.push(SemanticValue::new(
        types.float,
        SemanticPayload::Inline(InlineStructuralValue::F64Bits(1.5_f64.to_bits())),
    ));
    base.push(SemanticValue::new(
        types.string,
        SemanticPayload::String(b"deep-text".to_vec()),
    ));
    base.push(SemanticValue::new(
        types.path,
        SemanticPayload::Path(b"/tmp/deep".to_vec()),
    ));
    base.push(SemanticValue::new(
        types.bytes,
        SemanticPayload::Bytes(vec![0, 1, 2, 255]),
    ));
    base.push(SemanticValue::new(
        types.byte_vector,
        SemanticPayload::ByteVector(vec![3, 4, 5]),
    ));
    base.push(SemanticValue::new(
        types.static_value,
        SemanticPayload::Static(StaticStructuralLeaf::Symbol(7)),
    ));
    let mut value = SemanticValue::new(types.product, SemanticPayload::Product(base));
    for level in 0..depth {
        let children = vec![value].into();
        value = if level % 2 == 0 {
            SemanticValue::new(
                types.enumeration,
                SemanticPayload::Enum {
                    tag: u64::try_from(level % 3).expect("test tag"),
                    active_payload: children,
                },
            )
        } else {
            SemanticValue::new(types.product, SemanticPayload::Product(children))
        };
    }
    value
}

fn expected_metrics(depth: usize) -> (u64, u64, u64) {
    let depth = u64::try_from(depth).expect("test depth fits u64");
    let nodes = depth + 10;
    let fields = depth + 9;
    let aggregate_bytes = 25;
    (nodes, fields, nodes + fields + aggregate_bytes)
}

fn base_fields(mut value: &SemanticValue) -> Result<&SemanticChildren, String> {
    loop {
        match &value.payload {
            SemanticPayload::Product(fields) if fields.len() == 9 => return Ok(fields),
            SemanticPayload::Product(fields)
            | SemanticPayload::Enum {
                active_payload: fields,
                ..
            } if fields.len() == 1 => value = &fields[0],
            _ => return Err("deep structural chain lost its generated shape".into()),
        }
    }
}

fn require_rewritten_leaves(value: &OwnedValue) -> Result<(), String> {
    let fields = base_fields(
        value
            .as_structural()
            .ok_or_else(|| "owned structural tree is absent".to_string())?,
    )?;
    let valid = matches!(
        fields[0].payload,
        SemanticPayload::Inline(InlineStructuralValue::Unit)
    ) && matches!(
        fields[1].payload,
        SemanticPayload::Inline(InlineStructuralValue::Bool(true))
    ) && matches!(
        fields[2].payload,
        SemanticPayload::Inline(InlineStructuralValue::I64(41))
    ) && matches!(
        fields[3].payload,
        SemanticPayload::Inline(InlineStructuralValue::F64Bits(bits)) if bits == 1.5_f64.to_bits()
    ) && matches!(&fields[4].payload, SemanticPayload::String(bytes) if bytes == b"deep-text")
        && matches!(&fields[5].payload, SemanticPayload::Path(bytes) if bytes == b"/tmp/deep")
        && matches!(&fields[6].payload, SemanticPayload::Bytes(bytes) if bytes == &[0, 1, 2, 255])
        && matches!(&fields[7].payload, SemanticPayload::ByteVector(bytes) if bytes == &[3, 4, 5])
        && matches!(
            fields[8].payload,
            SemanticPayload::Static(StaticStructuralLeaf::Symbol(0))
        );
    if valid {
        Ok(())
    } else {
        Err("deep symbol canonicalization changed or missed a generated leaf".into())
    }
}

fn exercise_boundary(depth: usize) -> Result<(), String> {
    let value = alternating_value(depth, 41);
    let equal = alternating_value(depth, 41);
    let different = alternating_value(depth, 42);

    if !value.try_equal(&equal).map_err(|error| error.to_string())?
        || value
            .try_equal(&different)
            .map_err(|error| error.to_string())?
    {
        return Err("fallible semantic equality disagrees with deep leaves".into());
    }
    if value != equal || value == different {
        return Err("trait semantic equality disagrees with deep leaves".into());
    }

    let semantic_debug = format!("{value:?}");
    if semantic_debug.len() > 512
        || !semantic_debug.contains("SemanticValue")
        || !semantic_debug.contains("field_count")
    {
        return Err(format!(
            "semantic debug is not a bounded summary: {semantic_debug}"
        ));
    }

    let cloned = value.clone();
    if cloned != value {
        return Err("deep semantic clone changed the value".into());
    }

    let mut runtime = StructuralValueRuntime::new().map_err(|error| error.to_string())?;
    let owner = runtime
        .publish_owned(cloned)
        .map_err(|failure| failure.error.to_string())?;
    let exported = runtime
        .export_semantic(owner, value.value_type)
        .map_err(|error| error.to_string())?;
    if exported != value {
        return Err("structural image round trip changed the value".into());
    }
    runtime.verify_empty().map_err(|error| error.to_string())?;

    let owned = OwnedValue::from_structural(exported)
        .map_err(|error| error.to_string())?
        .retain_symbols(|symbol| match symbol {
            7 => Ok("retained-symbol"),
            _ => Err(lkjscript_core::Error::msg("unexpected test symbol")),
        })
        .map_err(|error| error.to_string())?;
    require_rewritten_leaves(&owned)?;
    let (nodes, fields, work) = expected_metrics(depth);
    let metrics = owned
        .structural_snapshot_metrics()
        .ok_or_else(|| "owned structural metrics are absent".to_string())?;
    if (
        metrics.nodes,
        metrics.fields,
        metrics.encode_work,
        metrics.decode_work,
    ) != (nodes, fields, work, work)
    {
        return Err(format!("unexpected structural metrics: {metrics:?}"));
    }
    if owned.snapshot_object_count() != usize::try_from(nodes).expect("test nodes fit usize") {
        return Err("owned structural inspection lost a node".into());
    }
    let owned_debug = format!("{owned:?}");
    if owned_debug.len() > 128 || !owned_debug.contains("owned-structural") {
        return Err(format!("owned debug is not concise: {owned_debug}"));
    }

    let owned_clone = owned.clone();
    if owned_clone != owned {
        return Err("deep OwnedValue clone or equality changed the value".into());
    }
    let outcome = ExecutionOutcome::Returned(owned);
    let equal_outcome = ExecutionOutcome::Returned(owned_clone);
    if outcome != equal_outcome {
        return Err("ExecutionOutcome deep equality changed the value".into());
    }
    let summary = outcome.summary();
    if summary.len() > 192 || !summary.contains("owned-structural") {
        return Err(format!("execution summary is not concise: {summary}"));
    }
    let outcome_debug = format!("{outcome:?}");
    if outcome_debug.len() > 256 || !outcome_debug.contains("owned-structural") {
        return Err(format!("execution debug is not concise: {outcome_debug}"));
    }

    drop(different);
    drop(value);
    drop(outcome);
    drop(equal_outcome);
    Ok(())
}

#[test]
fn owned_structural_boundary_is_stack_safe_on_a_small_stack() {
    std::thread::Builder::new()
        .name("owned-structural-small-stack".into())
        .stack_size(SMALL_STACK_BYTES)
        .spawn(|| exercise_boundary(ORDINARY_DEPTH))
        .expect("spawn owned structural worker")
        .join()
        .expect("owned structural worker panicked")
        .expect("owned structural boundary succeeds");
}

#[test]
fn structural_validation_work_is_exactly_linear_over_geometric_depth() {
    for depth in [1_024, 2_048, 4_096, 8_192] {
        let owned = OwnedValue::from_structural(alternating_value(depth, 41))
            .expect("validate geometric structural value");
        let (nodes, fields, work) = expected_metrics(depth);
        let metrics = owned
            .structural_snapshot_metrics()
            .expect("structural metrics");
        assert_eq!(metrics.nodes, nodes);
        assert_eq!(metrics.fields, fields);
        assert_eq!(metrics.aggregate_bytes, 25);
        assert_eq!(metrics.string_bytes, 9);
        assert_eq!(metrics.path_bytes, 9);
        assert_eq!(metrics.encode_work, work);
        assert_eq!(metrics.decode_work, work);
    }
}

#[test]
fn structural_equality_distinguishes_shape_type_tag_and_leaf_mismatches() {
    let types = Types::new();
    let original = alternating_value(8, 41);

    let mut wrong_type = original.clone();
    wrong_type.value_type.semantic_type = ty(12, wrong_type.value_type.kind).semantic_type;
    assert_ne!(original, wrong_type);

    let wrong_payload = SemanticValue::new(
        types.product,
        SemanticPayload::Enum {
            tag: 0,
            active_payload: SemanticChildren::new(),
        },
    );
    let empty_product = SemanticValue::new(
        types.product,
        SemanticPayload::Product(SemanticChildren::new()),
    );
    assert_ne!(empty_product, wrong_payload);

    let left_tag = SemanticValue::new(
        types.enumeration,
        SemanticPayload::Enum {
            tag: 1,
            active_payload: SemanticChildren::new(),
        },
    );
    let right_tag = SemanticValue::new(
        types.enumeration,
        SemanticPayload::Enum {
            tag: 2,
            active_payload: SemanticChildren::new(),
        },
    );
    assert_ne!(left_tag, right_tag);

    let one_child = SemanticValue::new(
        types.product,
        SemanticPayload::Product(
            vec![SemanticValue::new(
                types.integer,
                SemanticPayload::Inline(InlineStructuralValue::I64(1)),
            )]
            .into(),
        ),
    );
    assert_ne!(empty_product, one_child);

    let shallow_left = SemanticValue::new(types.string, SemanticPayload::String(b"left".to_vec()));
    let shallow_right =
        SemanticValue::new(types.string, SemanticPayload::String(b"right".to_vec()));
    assert_ne!(shallow_left, shallow_right);

    let deep_left = alternating_value(4_096, 41);
    let deep_right = alternating_value(4_096, 42);
    assert_ne!(deep_left, deep_right);
    assert_eq!(deep_left.try_equal(&deep_right), Ok(false));
}

#[test]
fn structural_debug_summaries_bound_large_leaf_payloads() {
    let types = Types::new();
    let value = SemanticValue::new(types.string, SemanticPayload::String(vec![b'x'; 65_537]));
    let semantic_debug = format!("{value:?}");
    assert!(semantic_debug.len() < 256, "{semantic_debug}");
    assert!(semantic_debug.contains("byte_count: 65537"));
    let owned = OwnedValue::from_structural(value).expect("large valid string");
    assert_eq!(owned.as_str().map(str::len), Some(65_537));
    let owned_debug = format!("{owned:?}");
    assert_eq!(owned_debug, "#<owned-string-or-symbol:65537>");
    let summary = ExecutionOutcome::Returned(owned).summary();
    assert_eq!(summary, "Returned(#<owned-string-or-symbol:65537>)");
}

#[test]
fn malformed_structural_leaves_fail_without_partial_owned_publication() {
    let types = Types::new();
    let invalid_utf8 = SemanticValue::new(
        types.string,
        SemanticPayload::String(vec![0xf0, 0x28, 0x8c, 0x28]),
    );
    let invalid_path = SemanticValue::new(types.path, SemanticPayload::Path(b"relative".to_vec()));
    let wrong_payload =
        SemanticValue::new(types.string, SemanticPayload::Path(b"/wrong-kind".to_vec()));

    let utf8 = OwnedValue::from_structural(invalid_utf8)
        .expect_err("invalid UTF-8 must fail")
        .to_string();
    assert!(utf8.contains("not UTF-8"), "{utf8}");
    let path = OwnedValue::from_structural(invalid_path)
        .expect_err("invalid path must fail")
        .to_string();
    assert!(path.contains("path is invalid"), "{path}");
    let payload = OwnedValue::from_structural(wrong_payload)
        .expect_err("wrong payload must fail")
        .to_string();
    assert!(payload.contains("type and payload disagree"), "{payload}");
}

#[test]
#[ignore = "opt-in 20,000-level owned structural small-stack stress geometry"]
fn twenty_thousand_level_owned_structural_boundary_stress() {
    std::thread::Builder::new()
        .name("owned-structural-stress-small-stack".into())
        .stack_size(SMALL_STACK_BYTES)
        .spawn(|| exercise_boundary(STRESS_DEPTH))
        .expect("spawn owned structural stress worker")
        .join()
        .expect("owned structural stress worker panicked")
        .expect("owned structural stress boundary succeeds");
}

#[test]
#[ignore = "release characterization selected by LKJSCRIPT_STRUCTURAL_DEPTH and LKJSCRIPT_STRUCTURAL_OPERATION"]
fn owned_structural_scale_sample() {
    let depth = std::env::var("LKJSCRIPT_STRUCTURAL_DEPTH")
        .expect("LKJSCRIPT_STRUCTURAL_DEPTH selects the generated geometry")
        .parse::<usize>()
        .expect("structural depth is a usize");
    let operation = std::env::var("LKJSCRIPT_STRUCTURAL_OPERATION")
        .expect("LKJSCRIPT_STRUCTURAL_OPERATION selects the measured operation");
    std::thread::Builder::new()
        .name(format!("owned-structural-{operation}"))
        .stack_size(SMALL_STACK_BYTES)
        .spawn(move || {
            let (elapsed, detail) = match operation.as_str() {
                "construction" => {
                    let started = Instant::now();
                    let value = alternating_value(depth, 41);
                    let elapsed = started.elapsed();
                    std::hint::black_box(&value);
                    drop(value);
                    (elapsed, format!("nodes={}", expected_metrics(depth).0))
                }
                "validation" => {
                    let value = alternating_value(depth, 41);
                    let started = Instant::now();
                    let owned = OwnedValue::from_structural(value).expect("validation");
                    let elapsed = started.elapsed();
                    let detail = format!(
                        "work={}",
                        owned
                            .structural_snapshot_metrics()
                            .expect("metrics")
                            .encode_work
                    );
                    drop(owned);
                    (elapsed, detail)
                }
                "image-roundtrip" => {
                    let value = alternating_value(depth, 41);
                    let value_type = value.value_type;
                    let mut runtime = StructuralValueRuntime::new().expect("structural runtime");
                    let started = Instant::now();
                    let owner = runtime
                        .publish_owned(value)
                        .expect("structural image publication");
                    let exported = runtime
                        .export_semantic(owner, value_type)
                        .expect("structural image export");
                    let elapsed = started.elapsed();
                    runtime.verify_empty().expect("empty structural runtime");
                    drop(exported);
                    (elapsed, format!("nodes={}", expected_metrics(depth).0))
                }
                "clone" => {
                    let value = alternating_value(depth, 41);
                    let started = Instant::now();
                    let cloned = value.clone();
                    let elapsed = started.elapsed();
                    std::hint::black_box(&cloned);
                    drop(cloned);
                    drop(value);
                    (elapsed, format!("nodes={}", expected_metrics(depth).0))
                }
                "try-equal" => {
                    let left = alternating_value(depth, 41);
                    let right = alternating_value(depth, 41);
                    let started = Instant::now();
                    assert!(left.try_equal(&right).expect("fallible equality"));
                    let elapsed = started.elapsed();
                    drop(left);
                    drop(right);
                    (elapsed, format!("nodes={}", expected_metrics(depth).0))
                }
                "partial-eq" => {
                    let left = alternating_value(depth, 41);
                    let right = alternating_value(depth, 41);
                    let started = Instant::now();
                    assert_eq!(left, right);
                    let elapsed = started.elapsed();
                    drop(left);
                    drop(right);
                    (elapsed, format!("nodes={}", expected_metrics(depth).0))
                }
                "debug" => {
                    let value = alternating_value(depth, 41);
                    let started = Instant::now();
                    let rendered = format!("{value:?}");
                    let elapsed = started.elapsed();
                    let detail = format!("output_bytes={}", rendered.len());
                    drop(rendered);
                    drop(value);
                    (elapsed, detail)
                }
                "symbols" => {
                    let owned = OwnedValue::from_structural(alternating_value(depth, 41))
                        .expect("validation");
                    let started = Instant::now();
                    let owned = owned
                        .retain_symbols(|_| Ok("retained-symbol"))
                        .expect("symbol retention");
                    let elapsed = started.elapsed();
                    std::hint::black_box(&owned);
                    drop(owned);
                    (elapsed, format!("nodes={}", expected_metrics(depth).0))
                }
                "destruction" => {
                    let value = alternating_value(depth, 41);
                    let started = Instant::now();
                    drop(value);
                    (
                        started.elapsed(),
                        format!("nodes={}", expected_metrics(depth).0),
                    )
                }
                other => panic!("unknown structural operation {other}"),
            };
            eprintln!(
                "LKJSCRIPT_STRUCTURAL_SCALE operation={} depth={} elapsed_ns={} {}",
                operation,
                depth,
                elapsed.as_nanos(),
                detail
            );
        })
        .expect("spawn structural scale worker")
        .join()
        .expect("structural scale worker panicked");
}
