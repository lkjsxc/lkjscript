//! Independent literal flat requests are the oracle for structural input normalization.
//!
//! These fixtures intentionally keep indexed flat edges and authored binder labels. Neither
//! side is produced by the structural parser, and comparisons retain the existing codec.

use super::*;
use crate::platform::change::canonical_authored_intent_bytes;
use crate::platform::publication::{GraphRepository, PreparedAuthoredPublication};

fn decode(path: &str, input: &str) -> NormalizedChangeRequest {
    decode_compact_change(path, input.as_bytes())
        .unwrap_or_else(|errors| panic!("{path}: {errors:#?}"))
}

fn assert_intent_pair(
    flat: &str,
    block: &str,
) -> (NormalizedChangeRequest, NormalizedChangeRequest) {
    let flat = decode("independent-flat.lkjc", flat);
    let block = decode("structural.lkjc", block);
    assert_eq!(
        canonical_authored_intent_bytes(&flat.semantic).unwrap(),
        canonical_authored_intent_bytes(&block.semantic).unwrap(),
        "the unchanged authored codec must preserve labels' traversal/domain identity"
    );
    assert_eq!(flat.request_commitment, block.request_commitment);
    (flat, block)
}

fn assert_candidate_pair(
    repository: &GraphRepository,
    flat: &NormalizedChangeRequest,
    block: &NormalizedChangeRequest,
) -> (PreparedAuthoredPublication, PreparedAuthoredPublication) {
    let flat = repository
        .prepare_authored_change(&flat.semantic, flat.options.clone())
        .unwrap_or_else(|errors| panic!("flat candidate: {errors:#?}"));
    let block = repository
        .prepare_authored_change(&block.semantic, block.options.clone())
        .unwrap_or_else(|errors| panic!("block candidate: {errors:#?}"));
    assert_eq!(flat.publication.objects, block.publication.objects);
    assert_eq!(flat.publication.head_bytes, block.publication.head_bytes);
    assert_eq!(
        flat.publication.transaction_bytes,
        block.publication.transaction_bytes
    );
    assert_eq!(
        flat.publication.semantic_diff_bytes,
        block.publication.semantic_diff_bytes
    );
    assert_eq!(
        flat.publication.receipt_bytes,
        block.publication.receipt_bytes
    );
    assert_eq!(
        flat.allocated.values().copied().collect::<BTreeSet<_>>(),
        block.allocated.values().copied().collect::<BTreeSet<_>>()
    );
    assert_eq!(
        flat.logical_plan.structurally_checked,
        block.logical_plan.structurally_checked
    );
    assert_eq!(
        flat.logical_plan.semantically_checked,
        block.logical_plan.semantically_checked
    );
    (flat, block)
}

const SHADOW_FLAT: &str = "\
expression.local as=$initial value=$parameter
expression.local as=$previous value=$first
expression.call as=$next function=$identity
expression.argument parent=$next index=0 expression=$previous
expression.local as=$answer value=$second
expression.let as=$body body=$answer
expression.binding parent=$body index=0 as=$first name=value value=$initial type=i64
expression.binding parent=$body index=1 as=$second name=value value=$next
";

const SHADOW_BLOCK: &str = "\
expression.block as=$body
  (let
    (binding value (type i64) (local $parameter))
    (binding value (call $identity (local value)))
    (in (local value)))
expression.end
";

const SHADOW_DECLARATIONS: &str = "\
create.module as=$module name=structural-shadowing
create.function as=$function module=$module name=shadow visibility=private result=i64 effect=pure body=$body
add.parameter as=$parameter function=$function name=parameter type=i64
expression.local as=$identity-body value=$identity-parameter
create.function as=$identity module=$module name=identity visibility=private result=i64 effect=pure body=$identity-body
add.parameter as=$identity-parameter function=$identity name=value type=i64
";

#[test]
fn structural_call_let_later_parameter_and_same_name_rebinding_preserve_the_reviewed_candidate() {
    let temporary = tempfile::tempdir().unwrap();
    let logical = crate::platform::kernel::tests::witness_snapshot();
    let created =
        GraphRepository::create(&temporary.path().join("meaning"), &logical, None).unwrap();
    let header = format!(
        "request base={} idempotency=structural-parity intent=reviewed-shadowing\n",
        created.current.head.revision
    );
    let (flat, block) = assert_intent_pair(
        &format!("{header}{SHADOW_FLAT}{SHADOW_DECLARATIONS}"),
        &format!("{header}{SHADOW_BLOCK}{SHADOW_DECLARATIONS}"),
    );
    let (flat_candidate, block_candidate) =
        assert_candidate_pair(&created.repository, &flat, &block);
    assert_eq!(
        flat_candidate.allocated["$function"],
        block_candidate.allocated["$function"]
    );
    assert_eq!(
        flat_candidate.allocated["$parameter"],
        block_candidate.allocated["$parameter"]
    );
    assert_ne!(
        flat_candidate.allocated["$first"],
        flat_candidate.allocated["$second"]
    );

    let AuthoredChange::CreateFunction { body, .. } = &block.semantic.changes[1] else {
        panic!("subject function")
    };
    let AuthoredExpressionOperation::Let { bindings, body } = &body.operation else {
        panic!("sequential let")
    };
    assert_eq!(bindings[0].name.as_str(), "value");
    assert_eq!(bindings[1].name.as_str(), "value");
    assert_eq!(bindings[0].declared_type, Some(AuthoredType::I64 {}));
    assert_eq!(bindings[1].declared_type, None);
    assert_ne!(bindings[0].symbol, bindings[1].symbol);
    let AuthoredExpressionOperation::Call { arguments, .. } = &bindings[1].value.operation else {
        panic!("new initializer calls identity")
    };
    assert!(matches!(
        &arguments[0].operation,
        AuthoredExpressionOperation::Local { value: AuthoredLocalReference::Symbol { symbol } }
            if symbol == &bindings[0].symbol
    ));
    assert!(matches!(
        &body.operation,
        AuthoredExpressionOperation::Local { value: AuthoredLocalReference::Symbol { symbol } }
            if symbol == &bindings[1].symbol
    ));
}

struct Pair {
    name: &'static str,
    result: &'static str,
    flat: &'static str,
    block: &'static str,
}

// Additional cases distinguish nominal/structural selectors and both variant payload shapes.
const PAIRS: &[Pair] = &[
    Pair {
        name: "unit",
        result: "unit",
        flat: "expression.unit as=$body\n",
        block: "(unit)",
    },
    Pair {
        name: "bool",
        result: "bool",
        flat: "expression.bool as=$body value=true\n",
        block: "(bool true)",
    },
    Pair {
        name: "i64",
        result: "i64",
        flat: "expression.i64 as=$body value=-9223372036854775808\n",
        block: "(i64 -9223372036854775808)",
    },
    Pair {
        name: "text",
        result: "text",
        flat: "expression.text as=$body value=\"text; (expression.end) \\\"quoted\\\" \\u{1f642}\"\n",
        block: "(text \"text; (expression.end) \\\"quoted\\\" \\u{1f642}\")",
    },
    Pair {
        name: "static-text",
        result: "static-text",
        flat: "expression.static-text as=$body value=\"a\\nb\\tc\"\n",
        block: "(static-text \"a\\nb\\tc\")",
    },
    Pair {
        name: "local",
        result: "i64",
        flat: "expression.local as=$body value=$parameter\n",
        block: "(local $parameter)",
    },
    Pair {
        name: "constant",
        result: "i64",
        flat: "expression.constant as=$body declaration=$constant\n",
        block: "(constant $constant)",
    },
    Pair {
        name: "if",
        result: "i64",
        flat: "expression.bool as=$condition value=false\nexpression.i64 as=$yes value=11\nexpression.i64 as=$no value=13\nexpression.if as=$body condition=$condition when-true=$yes when-false=$no\n",
        block: "(if (bool false) (i64 11) (i64 13))",
    },
    Pair {
        name: "sequence",
        result: "i64",
        flat: "expression.unit as=$first\nexpression.i64 as=$last value=23\nexpression.sequence as=$body\nexpression.argument parent=$body index=0 expression=$first\nexpression.argument parent=$body index=1 expression=$last\n",
        block: "(sequence (unit) (i64 23))",
    },
    Pair {
        name: "call",
        result: "i64",
        flat: "expression.i64 as=$argument value=29\nexpression.call as=$body function=$identity\nexpression.argument parent=$body index=0 expression=$argument\n",
        block: "(call $identity (types) (effects) (requirements) (i64 29))",
    },
    Pair {
        name: "function-value",
        result: "@IdentitySignature",
        flat: "expression.function-value as=$body function=$identity\n",
        block: "(function-value $identity)",
    },
    Pair {
        name: "invoke",
        result: "i64",
        flat: "expression.function-value as=$callee function=$identity\nexpression.i64 as=$argument value=31\nexpression.invoke as=$body function=$callee\nexpression.argument parent=$body index=0 expression=$argument\n",
        block: "(invoke (function-value $identity) (i64 31))",
    },
    Pair {
        name: "bind",
        result: "@BoundSignature",
        flat: "expression.function-value as=$callee function=$identity\nexpression.i64 as=$argument value=37\nexpression.bind as=$body callee=$callee\nexpression.argument parent=$body index=0 expression=$argument\n",
        block: "(bind (function-value $identity) (i64 37))",
    },
    Pair {
        name: "let",
        result: "i64",
        flat: SHADOW_FLAT,
        block: "(let (binding value (type i64) (local $parameter)) (binding value (call $identity (local value))) (in (local value)))",
    },
    Pair {
        name: "nominal-record",
        result: "@I64Box",
        flat: "expression.i64 as=$value value=41\nexpression.record as=$body type=$Box\ntype.argument parent=$body index=0 type=i64\nexpression.record-field parent=$body index=0 field=$box-value value=$value\n",
        block: "(record $Box (types i64) (field $box-value (i64 41)))",
    },
    Pair {
        name: "structural-record",
        result: "@Pair",
        flat: "expression.i64 as=$left value=43\nexpression.text as=$right value=right\nexpression.record as=$body\nexpression.record-field parent=$body index=0 name=left value=$left\nexpression.record-field parent=$body index=1 name=right value=$right\n",
        block: "(record structural (field left (i64 43)) (field right (text \"right\")))",
    },
    Pair {
        name: "variant-empty",
        result: "@I64Choice",
        flat: "expression.variant as=$body case=$None\ntype.argument parent=$body index=0 type=i64\n",
        block: "(variant $None (types i64))",
    },
    Pair {
        name: "variant-payload",
        result: "@I64Choice",
        flat: "expression.i64 as=$payload value=47\nexpression.variant as=$body case=$Some payload=$payload\ntype.argument parent=$body index=0 type=i64\n",
        block: "(variant $Some (types i64) (i64 47))",
    },
    Pair {
        name: "nominal-field",
        result: "i64",
        flat: "expression.i64 as=$value value=53\nexpression.record as=$record type=$Box\ntype.argument parent=$record index=0 type=i64\nexpression.record-field parent=$record index=0 field=$box-value value=$value\nexpression.field as=$body value=$record field=$box-value\n",
        block: "(field (record $Box (types i64) (field $box-value (i64 53))) $box-value)",
    },
    Pair {
        name: "structural-field",
        result: "i64",
        flat: "expression.i64 as=$value value=59\nexpression.record as=$record\nexpression.record-field parent=$record index=0 name=value value=$value\nexpression.field as=$body value=$record name=value\n",
        block: "(field (record structural (field value (i64 59))) (name value))",
    },
    Pair {
        name: "list",
        result: "@Integers",
        flat: "expression.i64 as=$one value=61\nexpression.i64 as=$two value=67\nexpression.list as=$body item=i64\nexpression.argument parent=$body index=0 expression=$one\nexpression.argument parent=$body index=1 expression=$two\n",
        block: "(list i64 (i64 61) (i64 67))",
    },
    Pair {
        name: "map",
        result: "@Map",
        flat: "expression.text as=$key-value value=first\nexpression.record as=$key-record\nexpression.record-field parent=$key-record index=0 name=key value=$key-value\nexpression.field as=$key value=$key-record name=key\nexpression.i64 as=$value-argument value=71\nexpression.call as=$value function=$identity\nexpression.argument parent=$value index=0 expression=$value-argument\nexpression.text as=$key-two value=second\nexpression.i64 as=$value-two value=73\nexpression.map as=$body key=text value=i64\nexpression.map-entry parent=$body index=0 key=$key value=$value\nexpression.map-entry parent=$body index=1 key=$key-two value=$value-two\n",
        block: "(map text i64 (entry (field (record structural (field key (text \"first\"))) (name key)) (call $identity (i64 71))) (entry (text \"second\") (i64 73)))",
    },
    Pair {
        name: "match",
        result: "i64",
        flat: "expression.i64 as=$payload value=79\nexpression.variant as=$scrutinee case=$Some payload=$payload\ntype.argument parent=$scrutinee index=0 type=i64\nexpression.i64 as=$none-body value=0\nexpression.local as=$some-body value=$matched\nexpression.match as=$body value=$scrutinee\nexpression.match-arm parent=$body index=0 case=$None body=$none-body\nexpression.match-arm parent=$body index=1 case=$Some as=$matched name=payload type=i64 body=$some-body\n",
        block: "(match (variant $Some (types i64) (i64 79)) (arm $None (i64 0)) (arm $Some (payload payload i64) (local payload)))",
    },
    Pair {
        name: "capability-call",
        result: "@Entries",
        flat: "expression.static-text as=$space value=parity\nexpression.list as=$key item=@KeyPart\nexpression.capability-call as=$body requirement=$store operation=$get\nexpression.argument parent=$body index=0 expression=$space\nexpression.argument parent=$body index=1 expression=$key\n",
        block: "(capability-call $store $get (static-text \"parity\") (list @KeyPart))",
    },
    Pair {
        name: "transaction",
        result: "unit",
        flat: "expression.unit as=$inside\nexpression.transaction as=$body requirement=$store binding=$transaction-binding name=transaction-body body=$inside\n",
        block: "(transaction $store (binding transaction-body) (unit))",
    },
    Pair {
        name: "transaction-outcome",
        result: "@UnitOutcome",
        flat: "expression.unit as=$inside\nexpression.transaction-outcome as=$body requirement=$store binding=$transaction-binding name=transaction-body type=unit outcome=$Outcome abort-reason=$Reason committed=$Committed aborted=$Aborted condition-failed=$ConditionFailed conflict=$Conflict body=$inside\n",
        block: "(transaction-outcome $store (types unit) (outcome $Outcome $Reason $Committed $Aborted $ConditionFailed $Conflict) (binding transaction-body) (unit))",
    },
    Pair {
        name: "explicit-applications",
        result: "i64",
        flat: "expression.i64 as=$argument value=83\nexpression.call as=$body function=$generic\ntype.argument parent=$body index=0 type=i64\neffect.argument parent=$body index=0 effect=@Empty\nrequirement.argument parent=$body index=0 requirement=$store\nexpression.argument parent=$body index=0 expression=$argument\n",
        block: "(call $generic (types i64) (effects @Empty) (requirements $store) (i64 83))",
    },
    Pair {
        name: "function-value-applications",
        result: "@GenericBound",
        flat: "expression.function-value as=$body function=$generic\ntype.argument parent=$body index=0 type=i64\neffect.argument parent=$body index=0 effect=@Empty\nrequirement.argument parent=$body index=0 requirement=$store\n",
        block: "(function-value $generic (types i64) (effects @Empty) (requirements $store))",
    },
];

const FORM_DECLARATIONS: &str = "\
reference.package as=$std source=builtin
reference.owner as=$DataStore package=$std class=declaration name=DataStore
reference.owner as=$DataEntry package=$std class=declaration name=DataEntry
reference.owner as=$DataKeyPart package=$std class=declaration name=DataKeyPart
reference.owner as=$get package=$std class=operation parent=$DataStore name=get
reference.owner as=$transaction package=$std class=operation parent=$DataStore name=transaction
reference.owner as=$Outcome package=$std class=declaration name=TransactionOutcome
reference.owner as=$Reason package=$std class=declaration name=TransactionAbortReason
reference.owner as=$Committed package=$std class=case parent=$Outcome name=Committed
reference.owner as=$Aborted package=$std class=case parent=$Outcome name=Aborted
reference.owner as=$ConditionFailed package=$std class=case parent=$Reason name=ConditionFailed
reference.owner as=$Conflict package=$std class=case parent=$Reason name=Conflict
type.application as=@UnitOutcome declaration=$Outcome
type.argument parent=@UnitOutcome index=0 type=unit
create.module as=$module name=structural-forms
expression.local as=$identity-body value=$identity-parameter
create.function as=$identity module=$module name=identity visibility=private result=i64 effect=pure body=$identity-body
add.parameter as=$identity-parameter function=$identity name=value type=i64
expression.i64 as=$constant-value value=17
create.constant as=$constant module=$module name=constant visibility=private type=i64 value=$constant-value
create.record as=$Box module=$module name=Box visibility=private
add.type-parameter as=$BoxT declaration=$Box name=T
type.parameter as=@BoxT parameter=$BoxT
add.field as=$box-value record=$Box name=value type=@BoxT
type.application as=@I64Box declaration=$Box
type.argument parent=@I64Box index=0 type=i64
create.variant as=$Choice module=$module name=Choice visibility=private
add.type-parameter as=$ChoiceT declaration=$Choice name=T
type.parameter as=@ChoiceT parameter=$ChoiceT
add.case as=$None variant=$Choice name=None
add.case as=$Some variant=$Choice name=Some payload=@ChoiceT
type.application as=@I64Choice declaration=$Choice
type.argument parent=@I64Choice index=0 type=i64
type.structural-record as=@Pair
type.field parent=@Pair index=0 name=left type=i64
type.field parent=@Pair index=1 name=right type=text
type.function as=@IdentitySignature result=i64
type.argument parent=@IdentitySignature index=0 type=i64
type.function as=@BoundSignature result=i64
type.list as=@Integers item=i64
type.map as=@Map key=text value=i64
type.named as=@KeyPart declaration=$DataKeyPart
type.named as=@Entry declaration=$DataEntry
type.list as=@Entries item=@Entry
create.component as=$component module=$module name=Component visibility=private
add.port as=$store-port component=$component name=identity type=@IdentitySignature function=$identity
add.requirement as=$store component=$component name=store interface=$DataStore
requirement.operation parent=$store index=0 operation=$get
requirement.operation parent=$store index=1 operation=$transaction
effect.row as=@Empty
effect.row as=@StoreEffect
effect.requirement parent=@StoreEffect index=0 requirement=$store
type.task-function as=@GenericBound result=i64 effect=@StoreEffect
type.argument parent=@GenericBound index=0 type=i64
type.parameter as=@GenericT parameter=$GenericT
expression.local as=$generic-body value=$generic-parameter
create.function as=$generic module=$module name=generic visibility=private result=@GenericT effect=task body=$generic-body
add.type-parameter as=$GenericT declaration=$generic name=T constraint=capture-safe
add.effect-parameter as=$GenericE declaration=$generic name=E
add.requirement-parameter as=$GenericR declaration=$generic name=R interface=$DataStore
requirement-parameter.operation parent=$GenericR index=0 operation=$get
requirement-parameter.operation parent=$GenericR index=1 operation=$transaction
effect.parameter parent=$generic index=0 parameter=$GenericE
effect.requirement parent=$generic index=0 requirement=parameter:$GenericR
add.parameter as=$generic-parameter function=$generic name=value type=@GenericT
";

#[test]
fn structural_all_expression_forms_preserve_canonical_intent_and_candidate_objects() {
    let temporary = tempfile::tempdir().unwrap();
    let logical = crate::platform::kernel::tests::witness_snapshot();
    let created =
        GraphRepository::create(&temporary.path().join("meaning"), &logical, None).unwrap();
    let standard = crate::platform::builtin_standard::BuiltinStandard::load().unwrap();
    let transport = standard.transport();
    created
        .repository
        .stage_package_transport(standard.package_transport, &transport.container)
        .unwrap();
    let header = format!(
        "request base={} intent=all-forms\n",
        created.current.head.revision
    );
    let dependency = format!(
        "add.dependency package={} semantic-revision={} package-revision={}\n",
        standard.package, standard.semantic_revision, standard.package_revision
    );
    for pair in PAIRS {
        let tail = format!(
            "{FORM_DECLARATIONS}create.function as=$subject module=$module name=subject visibility=private result={} effect=task body=$body\nadd.parameter as=$parameter function=$subject name=parameter type=i64\neffect.requirement parent=$subject index=0 requirement=$store\n{dependency}",
            pair.result
        );
        let flat_source = format!("{header}{}{tail}", pair.flat);
        let block_source = format!(
            "{header}expression.block as=$body\n  {}\nexpression.end\n{tail}",
            pair.block
        );
        let (flat, block) = assert_intent_pair(&flat_source, &block_source);
        let (flat, block) = assert_candidate_pair(&created.repository, &flat, &block);
        assert_eq!(
            flat.allocated["$body"], block.allocated["$body"],
            "{}",
            pair.name
        );
    }
}

fn standalone(body: &str) -> String {
    format!(
        "request base={}\nexpression.block as=$body\n{body}\nexpression.end\n{SHADOW_DECLARATIONS}",
        RevisionId::from_digest([7; 32])
    )
}

#[test]
fn transaction_outcome_rejects_wrong_nominal_authority_body_type_and_missing_operation_before_acceptance()
 {
    let temporary = tempfile::tempdir().unwrap();
    let logical = crate::platform::kernel::tests::witness_snapshot();
    let created =
        GraphRepository::create(&temporary.path().join("meaning"), &logical, None).unwrap();
    let standard = crate::platform::builtin_standard::BuiltinStandard::load().unwrap();
    created
        .repository
        .stage_package_transport(standard.package_transport, &standard.transport().container)
        .unwrap();
    let pair = PAIRS
        .iter()
        .find(|pair| pair.name == "transaction-outcome")
        .unwrap();
    let source = format!(
        "request base={} intent=outcome-authority\n{}{FORM_DECLARATIONS}create.function as=$subject module=$module name=subject visibility=private result=@UnitOutcome effect=task body=$body\neffect.requirement parent=$subject index=0 requirement=$store\nadd.dependency package={} semantic-revision={} package-revision={}\n",
        created.current.head.revision,
        pair.flat,
        standard.package,
        standard.semantic_revision,
        standard.package_revision,
    );
    for (request, code) in [
        (
            source.replace(
                "name=transaction-body type=unit",
                "name=transaction-body type=text",
            ),
            "kernel_type_transaction_outcome_payload",
        ),
        (
            source.replace("committed=$Committed", "committed=$Aborted"),
            "kernel_transaction_outcome_identity",
        ),
        (
            source.replace("outcome=$Outcome", "outcome=$Choice"),
            "kernel_transaction_outcome_identity",
        ),
        (
            source.replace(
                "requirement.operation parent=$store index=1 operation=$transaction\n",
                "",
            ),
            "kernel_type_transaction_requirement",
        ),
    ] {
        let decoded = decode("outcome-invalid.lkjc", &request);
        let errors = created
            .repository
            .prepare_authored_change(&decoded.semantic, decoded.options)
            .unwrap_err();
        assert!(
            errors.iter().any(|error| error.code == code),
            "{code}: {errors:#?}"
        );
        assert_eq!(
            created
                .repository
                .view_current()
                .unwrap()
                .current()
                .head
                .revision,
            created.current.head.revision
        );
    }
}

#[test]
fn structural_comments_spacing_empty_applications_and_public_label_spelling_are_not_meaning() {
    let original = standalone("(call $identity (local $parameter))");
    let formatted = standalone(
        "; before\n (call\t$identity (types) (effects) (requirements) ; clauses\n (local $parameter) ; argument\n ) ; after",
    );
    let (original, _) = assert_intent_pair(&original, &formatted);
    let renamed = formatted
        .replace("$body", "$__structural_0")
        .replace("$identity", "$__structural_1");
    let renamed = decode("renamed.lkjc", &renamed);
    assert_eq!(original.request_commitment, renamed.request_commitment);
}

#[test]
fn structural_dense_quoted_literals_match_flat_and_multiline_inputs_without_sharing_occurrences() {
    const OCCURRENCES: usize = 2_048;
    let header = format!("request base={}\n", RevisionId::from_digest([7; 32]));
    let declarations = "create.module as=$module name=dense-strings\ncreate.function as=$function module=$module name=strings visibility=private result=text effect=pure body=$body\n";
    // Only workload repetition is generated. Flat record construction is independent of the
    // structural adapter, and the decoded literal is specified separately below.
    let flat_literal = r#""🙂; expression.end ( \"quoted\" )""#;
    let block_literal = r#"(text "\u{1f642}; expression.end ( \"quoted\" )")"#;
    let mut flat = format!("{header}expression.sequence as=$body\n");
    for index in 0..OCCURRENCES {
        flat.push_str(&format!("expression.text as=$item-{index} value={flat_literal}\nexpression.argument parent=$body index={index} expression=$item-{index}\n"));
    }
    flat.push_str(declarations);
    let dense = format!(
        "{header}expression.block as=$body\n(sequence {})\nexpression.end\n{declarations}",
        format!("{block_literal} ").repeat(OCCURRENCES)
    );
    let multiline = format!(
        "{header}expression.block as=$body\n(sequence\n{})\nexpression.end\n{declarations}",
        format!("  {block_literal}\n").repeat(OCCURRENCES)
    );
    let (flat, dense) = assert_intent_pair(&flat, &dense);
    let multiline = decode("many-lines.lkjc", &multiline);
    assert_eq!(flat.request_commitment, multiline.request_commitment);
    let AuthoredChange::CreateFunction { body, .. } = &dense.semantic.changes[1] else {
        panic!("dense string function")
    };
    let AuthoredExpressionOperation::Sequence { items } = &body.operation else {
        panic!("dense string sequence")
    };
    assert_eq!(items.len(), OCCURRENCES);
    let mut symbols = BTreeSet::new();
    for item in items {
        assert!(symbols.insert(item.symbol.as_ref().expect("owned occurrence")));
        assert!(
            matches!(&item.operation, AuthoredExpressionOperation::Text { value } if value == "🙂; expression.end ( \"quoted\" )")
        );
    }
}

#[test]
fn structural_real_names_annotations_literals_and_argument_order_remain_committed() {
    let source =
        standalone("(let (binding value (type i64) (i64 3)) (in (call $identity (local value))))");
    let original = decode("original.lkjc", &source);
    for different in [
        source.replace("(i64 3)", "(i64 4)"),
        source
            .replace("binding value", "binding renamed")
            .replace("local value", "local renamed"),
        source.replace("(type i64) ", ""),
        source.replace("(type i64)", "(type text)"),
    ] {
        let different = decode("different.lkjc", &different);
        assert_ne!(original.request_commitment, different.request_commitment);
    }
    let ordered = decode("ordered.lkjc", &standalone("(sequence (i64 3) (i64 4))"));
    let reversed = decode("reversed.lkjc", &standalone("(sequence (i64 4) (i64 3))"));
    assert_ne!(ordered.request_commitment, reversed.request_commitment);
    let ordered = decode(
        "ordered-fields.lkjc",
        &standalone("(record structural (field a (i64 3)) (field b (i64 4)))"),
    );
    let reversed = decode(
        "reversed-fields.lkjc",
        &standalone("(record structural (field b (i64 4)) (field a (i64 3)))"),
    );
    assert_ne!(ordered.request_commitment, reversed.request_commitment);
    let ordered = decode(
        "ordered-map.lkjc",
        &standalone("(map i64 i64 (entry (i64 1) (i64 2)) (entry (i64 3) (i64 4)))"),
    );
    let reversed = decode(
        "reversed-map.lkjc",
        &standalone("(map i64 i64 (entry (i64 3) (i64 4)) (entry (i64 1) (i64 2)))"),
    );
    assert_ne!(ordered.request_commitment, reversed.request_commitment);
}

#[test]
fn structural_nested_shadowing_restores_scope_and_all_occurrences_are_distinct() {
    let source = standalone(
        "(let (binding value (i64 3)) (in (sequence (let (binding value (local value)) (in (local value))) (local value) (local value))))",
    );
    let decoded = decode("nested.lkjc", &source);
    let AuthoredChange::CreateFunction { body, .. } = &decoded.semantic.changes[1] else {
        panic!("function")
    };
    let AuthoredExpressionOperation::Let { bindings, body } = &body.operation else {
        panic!("outer let")
    };
    let outer_symbol = &bindings[0].symbol;
    let AuthoredExpressionOperation::Sequence { items } = &body.operation else {
        panic!("sequence")
    };
    let AuthoredExpressionOperation::Let { bindings, body } = &items[0].operation else {
        panic!("inner let")
    };
    assert_ne!(outer_symbol, &bindings[0].symbol);
    for expression in [&bindings[0].value, &items[1], &items[2]] {
        assert!(
            matches!(&expression.operation, AuthoredExpressionOperation::Local { value: AuthoredLocalReference::Symbol { symbol } } if symbol == outer_symbol)
        );
    }
    assert!(
        matches!(&body.operation, AuthoredExpressionOperation::Local { value: AuthoredLocalReference::Symbol { symbol } } if symbol == &bindings[0].symbol)
    );
    assert!(items[1].symbol.is_some() && items[2].symbol.is_some());
    assert_ne!(items[1].symbol, items[2].symbol);
}

#[test]
fn structural_lexical_names_never_fall_back_to_public_or_exact_references() {
    for body in [
        "(local parameter)",
        "(local body)",
        "(local value)",
        "(let (binding value (local value)) (in (local value)))",
        "(let (binding first (local second)) (binding second (i64 1)) (in (local first)))",
        "(sequence (let (binding value (i64 3)) (in (local value))) (local value))",
    ] {
        let input = standalone(body);
        let errors = decode_compact_change("scope.lkjc", input.as_bytes()).unwrap_err();
        assert!(
            errors.iter().any(|error| error
                .location
                .as_ref()
                .is_some_and(|location| location.path == "scope.lkjc" && location.line >= 3)),
            "{body}: {errors:#?}"
        );
    }
    let exact = ParameterId::migrate(b"structural-exact-local", 0);
    let bare = standalone(&format!("(local {exact})"));
    assert!(decode_compact_change("bare-exact.lkjc", bare.as_bytes()).is_err());
    let explicit = decode(
        "explicit-exact.lkjc",
        &standalone(&format!("(local (exact {exact}))")),
    );
    let flat = format!(
        "request base={}\nexpression.local as=$body value={exact}\n{SHADOW_DECLARATIONS}",
        RevisionId::from_digest([7; 32])
    );
    let flat = decode("flat-exact.lkjc", &flat);
    assert_eq!(flat.request_commitment, explicit.request_commitment);
    let lexical = standalone(&format!(
        "(let (binding {exact} (i64 3)) (in (local {exact})))"
    ));
    assert!(decode_compact_change("exact-spelled-name.lkjc", lexical.as_bytes()).is_ok());
}

#[test]
fn structural_match_payload_and_transaction_names_exist_only_in_their_own_bodies() {
    for body in [
        "(sequence (match (variant $Some (types i64) (i64 79)) (arm $None (i64 0)) (arm $Some (payload payload i64) (local payload))) (local payload))",
        "(match (variant $Some (types i64) (i64 79)) (arm $Some (payload payload i64) (local payload)) (arm $None (local payload)))",
        "(sequence (transaction $store (binding tx) (unit)) (local tx))",
        "(transaction $store (binding tx) (let (binding value (unit)) (in (local absent))))",
    ] {
        let input = format!(
            "request base={}\nexpression.block as=$body\n{body}\nexpression.end\n{FORM_DECLARATIONS}create.function as=$subject module=$module name=subject visibility=private result=unit effect=task body=$body\neffect.requirement parent=$subject index=0 requirement=$store\n",
            RevisionId::from_digest([7; 32])
        );
        let errors = decode_compact_change("scoped-binders.lkjc", input.as_bytes()).unwrap_err();
        assert!(
            errors.iter().any(|error| error
                .location
                .as_ref()
                .is_some_and(
                    |location| location.path == "scoped-binders.lkjc" && location.line == 3
                )),
            "{body}: {errors:#?}"
        );
    }
}

#[test]
fn structural_requirement_parameter_application_and_explicit_empty_clauses_match_flat_intent() {
    let base = RevisionId::from_digest([7; 32]);
    let declarations = format!(
        "{FORM_DECLARATIONS}create.function as=$subject module=$module name=subject visibility=private result=i64 effect=task body=$body\nadd.requirement-parameter as=$SubjectR declaration=$subject name=R interface=$DataStore\nrequirement-parameter.operation parent=$SubjectR index=0 operation=$get\nrequirement-parameter.operation parent=$SubjectR index=1 operation=$transaction\neffect.requirement parent=$subject index=0 requirement=parameter:$SubjectR\n"
    );
    let flat = format!(
        "request base={base}\nexpression.i64 as=$value value=3\nexpression.call as=$body function=$generic\ntype.argument parent=$body index=0 type=i64\neffect.argument parent=$body index=0 effect=@Empty\nrequirement.argument parent=$body index=0 requirement=parameter:$SubjectR\nexpression.argument parent=$body index=0 expression=$value\n{declarations}"
    );
    let block = format!(
        "request base={base}\nexpression.block as=$body\n(call $generic (types i64) (effects @Empty) (requirements parameter:$SubjectR) (i64 3))\nexpression.end\n{declarations}"
    );
    let (_, original) = assert_intent_pair(&flat, &block);
    let concrete = decode(
        "concrete-requirement.lkjc",
        &block.replace(
            "(requirements parameter:$SubjectR)",
            "(requirements $store)",
        ),
    );
    assert_ne!(original.request_commitment, concrete.request_commitment);
    let changed_type = decode(
        "changed-application.lkjc",
        &block.replace("(types i64)", "(types text)"),
    );
    assert_ne!(original.request_commitment, changed_type.request_commitment);
}

#[test]
fn structural_roots_can_be_owned_by_flat_parents_but_cannot_be_extended_or_shared() {
    let header = format!("request base={}\n", RevisionId::from_digest([7; 32]));
    let tail = SHADOW_DECLARATIONS;
    let flat = format!(
        "{header}expression.i64 as=$child value=5\nexpression.sequence as=$body\nexpression.argument parent=$body index=0 expression=$child\n{tail}"
    );
    let mixed = format!(
        "{header}expression.block as=$child\n(i64 5)\nexpression.end\nexpression.sequence as=$body\nexpression.argument parent=$body index=0 expression=$child\n{tail}"
    );
    assert_intent_pair(&flat, &mixed);
    let extensions = [
        "expression.argument parent=$body index=0 expression=$extra\nexpression.i64 as=$extra value=1\n",
        "type.argument parent=$body index=0 type=i64\n",
        "effect.argument parent=$body index=0 effect=@Empty\neffect.row as=@Empty\n",
        "requirement.argument parent=$body index=0 requirement=$unresolved\n",
        "expression.binding parent=$body index=0 as=$binding name=value value=$extra\nexpression.i64 as=$extra value=1\n",
        "expression.record-field parent=$body index=0 name=value value=$extra\nexpression.i64 as=$extra value=1\n",
        "expression.match-arm parent=$body index=0 case=$case body=$extra\nexpression.i64 as=$extra value=1\n",
    ];
    for extension in extensions {
        let input = format!(
            "{header}expression.block as=$body\n(i64 5)\nexpression.end\n{extension}{tail}"
        );
        assert!(
            decode_compact_change("extend.lkjc", input.as_bytes()).is_err(),
            "{extension}"
        );
    }
    let shared = format!(
        "{mixed}create.function as=$other module=$module name=other visibility=private result=i64 effect=pure body=$child\n"
    );
    assert!(decode_compact_change("shared.lkjc", shared.as_bytes()).is_err());
    let unused = format!(
        "{header}expression.block as=$unused\n(unit)\nexpression.end\ncreate.module as=$module name=module\n"
    );
    assert!(decode_compact_change("unused.lkjc", unused.as_bytes()).is_err());
    let spliced = standalone("(sequence $outside)") + "expression.i64 as=$outside value=1\n";
    assert!(decode_compact_change("splice.lkjc", spliced.as_bytes()).is_err());
}

#[test]
fn structural_private_symbols_cannot_be_addressed_even_when_their_actual_spelling_is_known() {
    let input = standalone("(let (binding value (i64 3)) (in (local value)))");
    let decoded = decode("private.lkjc", &input);
    let AuthoredChange::CreateFunction { body, .. } = &decoded.semantic.changes[1] else {
        panic!("function")
    };
    let AuthoredExpressionOperation::Let { bindings, .. } = &body.operation else {
        panic!("let")
    };
    let private_binding = &bindings[0].symbol;
    let private_expression = bindings[0].value.symbol.as_ref().unwrap();
    for extra in [
        format!(
            "expression.local as=$attack value={private_binding}\ncreate.function as=$attacker module=$module name=attacker visibility=private result=i64 effect=pure body=$attack\n"
        ),
        format!(
            "create.function as=$attacker module=$module name=attacker visibility=private result=i64 effect=pure body={private_expression}\n"
        ),
        format!(
            "expression.argument parent={private_expression} index=0 expression=$extra\nexpression.unit as=$extra\n"
        ),
    ] {
        assert!(
            decode_compact_change("private-attack.lkjc", format!("{input}{extra}").as_bytes())
                .is_err(),
            "{extra}"
        );
    }
    // Generated spellings remain legal public labels. A real public definition must resolve
    // independently; its presence can make the private allocator choose another spelling.
    let public = format!(
        "{input}expression.i64 as={private_expression} value=7\ncreate.function as=$other module=$module name=other visibility=private result=i64 effect=pure body={private_expression}\n"
    );
    assert!(decode_compact_change("public-lookalike.lkjc", public.as_bytes()).is_ok());
}

#[test]
fn structural_framing_and_forms_reject_malformed_input_at_original_locations() {
    for body in [
        "",
        "(unit) (unit)",
        "(unit) trailing",
        "unit",
        "(unit",
        "(unit extra)",
        "(unknown)",
        "(bool True)",
        "(i64 9223372036854775808)",
        "(text bare)",
        "(text \"bad\\q\")",
        "(text \"bad\\u{d800}\")",
        "(text \"physical\nnewline\")",
        "(if (bool true) (unit))",
        "(local)",
        "(call)",
        "(invoke)",
        "(bind)",
        "(let (binding value (i64 1)))",
        "(let (binding value (type i64) (type i64) (i64 1)) (in (local value)))",
        "(let (in (i64 1)) (binding value (i64 1)))",
        "(record structural (types i64))",
        "(record structural (field value))",
        "(variant $Some (i64 1) (i64 2))",
        "(list)",
        "(map i64 i64 (entry (i64 1)))",
        "(match (unit) (arm))",
        "(transaction $store (binding tx) (unit) (unit))",
        "(transaction-outcome $store (types) (outcome $Outcome $Reason $Committed $Aborted $ConditionFailed $Conflict) (binding tx) (unit))",
        "(transaction-outcome $store (types unit) (outcome $Outcome $Reason $Committed $Aborted $ConditionFailed) (binding tx) (unit))",
        "(transaction-outcome $store (types unit) (outcome $Outcome $Reason $Committed $Aborted $ConditionFailed $Conflict) (binding tx))",
        "(capability-call $store)",
        "(call $identity (types) (types))",
        "(call $identity (effects) (types))",
        "(call $identity (requirements) (effects))",
        "(call $identity (i64 1) (types))",
        "(function-value $identity (i64 1))",
        "(record $Box (effects @Empty))",
    ] {
        let source = standalone(body);
        let errors = decode_compact_change("syntax.lkjc", source.as_bytes()).unwrap_err();
        assert!(
            errors.iter().any(|error| error
                .location
                .as_ref()
                .is_some_and(|location| location.path == "syntax.lkjc" && location.line >= 2)),
            "{body}: {errors:#?}"
        );
    }
    let valid = standalone("(i64 3)");
    for source in [
        valid.replace(
            "expression.block as=$body",
            "expression.block as=$body extra=value",
        ),
        valid.replace("expression.end\n", ""),
        valid.replace("expression.end", "expression.end extra=value"),
        valid.replace("(i64 3)", "expression.block as=$nested\n(i64 3)"),
        valid.replace("(i64 3)", "(i64 3)\nexpression.end\nexpression.end"),
        format!("{valid}expression.block as=$body\n(i64 4)\nexpression.end\n"),
    ] {
        assert!(decode_compact_change("framing.lkjc", source.as_bytes()).is_err());
    }
    let malformed = standalone("(text \"valid\")").replace("valid", "TOKEN");
    let offset = malformed.find("TOKEN").unwrap();
    let mut malformed = malformed.into_bytes();
    malformed[offset] = 0xff;
    assert!(decode_compact_change("utf8.lkjc", &malformed).is_err());
    assert!(
        parse_records(
            "response",
            b"expression.block as=$body\n(unit)\nexpression.end\n"
        )
        .is_err()
    );
}

#[test]
fn structural_unbound_local_diagnostic_points_to_the_user_token() {
    let source = standalone("(let\n  (binding value (i64 3))\n  (in (local missing)))");
    let offset = source.find("missing").unwrap();
    let errors = decode_compact_change("located.lkjc", source.as_bytes()).unwrap_err();
    let location = errors[0]
        .location
        .as_ref()
        .expect("original source location");
    assert_eq!(location.path, "located.lkjc");
    assert_eq!(location.byte_offset, offset);
    assert_eq!(location.line, 5);
    assert_eq!(location.column, 14);
    assert!(!errors[0].message.contains("__structural"));
}
