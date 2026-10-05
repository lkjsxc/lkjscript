//! Independent native/compact authoring oracles for reusable implementation schemes.
use super::*;
use crate::platform::change::canonical_authored_intent_bytes;

#[test]
fn generic_implementation_prerequisites_match_independent_native_and_compact_intent() {
    let base = format!("rev_{}", "83".repeat(32));
    let contract = format!("pkg_{}/decl_{}", "31".repeat(16), "32".repeat(16));
    let target = format!("pkg_{}/decl_{}", "31".repeat(16), "33".repeat(16));
    let prerequisite = "implparam_85000000000000000000000000000011";
    let method = "method_85000000000000000000000000000004";
    let flat = format!(
        r#"request base={base}
create.module as=$module name=schemes
type.parameter as=@item parameter=$item
type.owned-sequence as=@sequence item=@item
create.owned-implementation as=$scheme module=$module name=Storage visibility=public contract={contract} self=@sequence
owned.implementation-parameter parent=$scheme index=0 parameter=$item
owned.type-argument parent=$scheme index=0 type=@item
owned.witness parent=$scheme index=0 as=%prerequisite id={prerequisite} name=reader contract={contract} self=@sequence
owned.type-argument parent=%prerequisite index=0 type=@item
owned.mapping parent=$scheme index=0 as=%mapping method={method} function={target}
owned.type-argument parent=%mapping index=0 type=@item
implementation.argument parent=%mapping index=0 implementation=parameter@$scheme@{prerequisite}
add.type-parameter as=$item declaration=$scheme name=Item constraint=owned
"#
    );
    let native = format!(
        r#"request base={base}
declarations.begin
(units (module create schemes (as $module)
  (owned-implementation create Storage (as $scheme) (visibility public)
    (type-parameter create Item (as $item) (constraint owned))
    (implementation-parameter {prerequisite} reader {contract} (owned-sequence Item) (types Item))
    (contract {contract}) (self (owned-sequence Item)) (types Item)
    (method {method} {target} (types Item) (implementations parameter@Storage@{prerequisite})))))
declarations.end
"#
    );
    let compact = decode_compact_change("prerequisite-flat.lkjc", flat.as_bytes()).unwrap();
    let native = decode_compact_change("prerequisite-native.lkjc", native.as_bytes()).unwrap();
    let bytes = canonical_authored_intent_bytes(&native.semantic).unwrap();
    assert_eq!(&bytes[..8], b"LKJACR33");
    assert_eq!(
        bytes,
        canonical_authored_intent_bytes(&compact.semantic).unwrap()
    );
    assert_eq!(native.request_commitment, compact.request_commitment);
    let AuthoredChange::CreateOwnedImplementation {
        implementation_parameters,
        methods,
        ..
    } = &native.semantic.changes[1]
    else {
        panic!("expected implementation scheme");
    };
    assert_eq!(implementation_parameters.len(), 1);
    assert_eq!(implementation_parameters[0].name.as_str(), "reader");
    assert_eq!(implementation_parameters[0].type_arguments.len(), 1);
    assert_eq!(methods[0].implementations.len(), 1);
    let AuthoredImplementationOperand::Parameter { scope, parameter } =
        &methods[0].implementations[0]
    else {
        panic!("expected direct prerequisite forwarding");
    };
    assert_eq!(
        scope,
        &AuthoredDeclarationReference::Local {
            declaration: DeclarationSelector::Symbol {
                symbol: "$scheme".to_owned()
            }
        }
    );
    assert_eq!(parameter.to_string(), prerequisite);
    let changed = flat.replace(
        &format!("implementation=parameter@$scheme@{prerequisite}"),
        "implementation=parameter@$scheme@implparam_85000000000000000000000000000012",
    );
    let changed =
        decode_compact_change("changed-map-prerequisite.lkjc", changed.as_bytes()).unwrap();
    assert_ne!(changed.request_commitment, compact.request_commitment);
    let renamed = flat.replace("name=reader", "name=alternate");
    let renamed = decode_compact_change("renamed-prerequisite.lkjc", renamed.as_bytes()).unwrap();
    assert_ne!(renamed.request_commitment, compact.request_commitment);
}

#[test]
fn generic_implementation_nested_witnesses_match_flat_edges_and_retain_every_identity() {
    let base = format!("rev_{}", "83".repeat(32));
    let declaration = format!("decl_{}", "44".repeat(16));
    let inner = format!("decl_{}", "45".repeat(16));
    let scope = format!("decl_{}", "46".repeat(16));
    let prerequisite = "implparam_85000000000000000000000000000011";
    let method = "method_85000000000000000000000000000004";
    let flat = format!(
        r#"request base={base}
replace.body function={declaration} body=$body
expression.method-call as=$body witness=concrete@{declaration}@%outer contract={declaration} method={method}
owned.type-argument parent=%outer index=0 type=byte-buffer
implementation.argument parent=%outer index=0 implementation=concrete@{inner}@%inner
owned.type-argument parent=%inner index=0 type=owned-i64-cell
implementation.argument parent=%inner index=0 implementation=parameter@{scope}@{prerequisite}
"#
    );
    let structural = format!(
        r#"request base={base}
replace.body function={declaration} body=$body
expression.block as=$body
(method-call (implementation {declaration} (types byte-buffer)
  (implementations (implementation {inner} (types owned-i64-cell)
    (implementations parameter@{scope}@{prerequisite})))) {declaration} {method})
expression.end
"#
    );
    let compact = decode_compact_change("nested-flat.lkjc", flat.as_bytes()).unwrap();
    let structural_request =
        decode_compact_change("nested-structural.lkjc", structural.as_bytes()).unwrap();
    assert_eq!(
        canonical_authored_intent_bytes(&compact.semantic).unwrap(),
        canonical_authored_intent_bytes(&structural_request.semantic).unwrap()
    );
    assert_eq!(
        compact.request_commitment,
        structural_request.request_commitment
    );
    let AuthoredChange::ReplaceFunctionBody { body, .. } = &compact.semantic.changes[0] else {
        panic!("expected replacement");
    };
    let AuthoredExpressionOperation::MethodCall { witness, .. } = &body.operation else {
        panic!("expected method call");
    };
    let AuthoredImplementationOperand::Concrete {
        implementations, ..
    } = witness
    else {
        panic!("expected outer concrete application");
    };
    let AuthoredImplementationOperand::Concrete {
        type_arguments,
        implementations,
        ..
    } = &implementations[0]
    else {
        panic!("expected inner concrete application");
    };
    assert_eq!(type_arguments, &[AuthoredType::OwnedI64Cell {}]);
    assert!(
        matches!(&implementations[0], AuthoredImplementationOperand::Parameter { parameter, .. } if parameter.to_string() == prerequisite)
    );
    let changed = flat.replace(&inner, &format!("decl_{}", "47".repeat(16)));
    let changed = decode_compact_change("nested-changed-id.lkjc", changed.as_bytes()).unwrap();
    assert_ne!(changed.request_commitment, compact.request_commitment);
    for invalid in [
        flat.replace(
            &format!("concrete@{inner}@%inner"),
            &format!("concrete@{inner}@%outer"),
        ),
        structural.replace(
            "(types byte-buffer)",
            "(types byte-buffer) (types byte-buffer)",
        ),
        structural.replace(
            "(types byte-buffer)",
            "(implementations) (types byte-buffer)",
        ),
    ] {
        assert!(
            decode_compact_change("nested-invalid.lkjc", invalid.as_bytes()).is_err(),
            "{invalid}"
        );
    }
}

#[test]
fn generic_implementation_compact_witness_depth_is_admitted_before_recursive_construction() {
    use std::fmt::Write;
    let declaration = format!("decl_{}", "44".repeat(16));
    let method = "method_85000000000000000000000000000004";
    let maximum = crate::platform::kernel::contract::MAXIMUM_TYPE_DEPTH;
    for depth in [maximum, maximum + 1, 4096] {
        let mut input = format!(
            "request base=rev_{}\nreplace.body function={declaration} body=$body\nexpression.method-call as=$body witness=concrete@{declaration}@%operand0 contract={declaration} method={method}\n",
            "83".repeat(32),
        );
        for index in 0..depth - 1 {
            let application = if index + 2 == depth {
                format!("concrete@{declaration}")
            } else {
                format!("concrete@{declaration}@%operand{}", index + 1)
            };
            writeln!(input, "implementation.argument parent=%operand{index} index=0 implementation={application}").unwrap();
        }
        let decoded = decode_compact_change("bounded-witness.lkjc", input.as_bytes());
        if depth == maximum {
            assert!(decoded.is_ok());
        } else {
            let errors = decoded.unwrap_err();
            assert_eq!(errors[0].code, "change_owned_operand_depth");
            assert_eq!(errors[0].class, DiagnosticClass::Resource);
        }
    }
}

#[test]
fn generic_implementation_native_matches_complete_independent_compact_application() {
    let base = format!("rev_{}", "82".repeat(32));
    let contract = format!("pkg_{}/decl_{}", "31".repeat(16), "32".repeat(16));
    let target = format!("pkg_{}/decl_{}", "31".repeat(16), "33".repeat(16));
    let flat = format!(
        r#"request base={base}
create.module as=$module name=schemes
type.parameter as=@item parameter=$item
type.parameter as=@unused parameter=$unused
type.owned-sequence as=@sequence item=@item
create.owned-implementation as=$scheme module=$module name=Storage visibility=public contract={contract} self=@sequence
owned.implementation-parameter parent=$scheme index=0 parameter=$item
owned.implementation-parameter parent=$scheme index=1 parameter=$unused
owned.type-argument parent=$scheme index=0 type=@item
owned.mapping parent=$scheme index=0 as=%mapping method=method_85000000000000000000000000000004 function={target}
owned.type-argument parent=%mapping index=0 type=@sequence
owned.type-argument parent=%mapping index=1 type=@unused
add.type-parameter as=$item declaration=$scheme name=Item constraint=owned
add.type-parameter as=$unused declaration=$scheme name=Unused constraint=owned
"#
    );
    let native = format!(
        r#"request base={base}
declarations.begin
(units (module create schemes (as $module)
  (owned-implementation create Storage (as $scheme) (visibility public)
    (type-parameter create Item (as $item) (constraint owned))
    (type-parameter create Unused (as $unused) (constraint owned))
    (contract {contract}) (self (owned-sequence Item)) (types Item)
    (method method_85000000000000000000000000000004 {target} (types (owned-sequence Item) Unused)))))
declarations.end
"#
    );
    for invalid in [
        flat.replace("index=1 parameter=$unused", "index=2 parameter=$unused"),
        flat.replace("index=1 parameter=$unused", "index=0 parameter=$unused"),
        flat.replace(" as=%mapping", ""),
        flat.replace("parent=%mapping", "parent=%unknown"),
        format!(
            "{flat}owned.mapping parent=$scheme index=1 as=%mapping method=method_85000000000000000000000000000005 function={target}\n"
        ),
    ] {
        assert!(
            decode_compact_change("bad-generic-edges.lkjc", invalid.as_bytes()).is_err(),
            "{invalid}"
        );
    }
    let swapped = flat
        .replace("index=0 type=@sequence", "index=1 type=@sequence")
        .replace("index=1 type=@unused", "index=0 type=@unused");
    let swapped =
        decode_compact_change("swapped-generic-mapping.lkjc", swapped.as_bytes()).unwrap();
    let flat = decode_compact_change("generic-flat.lkjc", flat.as_bytes()).unwrap();
    let native = decode_compact_change("generic-native.lkjc", native.as_bytes()).unwrap();
    let bytes = canonical_authored_intent_bytes(&native.semantic).unwrap();
    assert_eq!(&bytes[..8], b"LKJACR32");
    assert_eq!(
        bytes,
        canonical_authored_intent_bytes(&flat.semantic).unwrap()
    );
    assert_eq!(native.request_commitment, flat.request_commitment);
    assert_ne!(swapped.request_commitment, flat.request_commitment);
    let AuthoredChange::CreateOwnedImplementation {
        type_parameters,
        methods,
        ..
    } = &native.semantic.changes[1]
    else {
        panic!("expected scheme")
    };
    assert_eq!(type_parameters.len(), 2);
    assert_eq!(methods[0].type_arguments.len(), 2);
}

#[test]
fn generic_implementation_structural_method_witness_matches_independent_compact_edges() {
    let base = format!("rev_{}", "82".repeat(32));
    let declaration = format!("decl_{}", "44".repeat(16));
    let method = "method_85000000000000000000000000000004";
    let flat = format!(
        r#"request base={base}
replace.body function={declaration} body=$body
expression.method-call as=$body witness=concrete@{declaration}@%application contract={declaration} method={method}
owned.type-argument parent=%application index=0 type=byte-buffer
owned.type-argument parent=%application index=1 type=owned-i64-cell
"#
    );
    let structural = format!(
        r#"request base={base}
replace.body function={declaration} body=$body
expression.block as=$body
(method-call (implementation {declaration} (types byte-buffer owned-i64-cell)) {declaration} {method})
expression.end
"#
    );
    let flat = decode_compact_change("applied-flat.lkjc", flat.as_bytes()).unwrap();
    let structural = decode_compact_change("applied-block.lkjc", structural.as_bytes()).unwrap();
    assert_eq!(
        canonical_authored_intent_bytes(&flat.semantic).unwrap(),
        canonical_authored_intent_bytes(&structural.semantic).unwrap()
    );
    assert_eq!(flat.request_commitment, structural.request_commitment);
    for invalid in [
        "(implementation)",
        &format!("(implementation {declaration} (effects pure))"),
        &format!("(implementation {declaration} (types byte-buffer) (types owned-i64-cell))"),
    ] {
        let invalid = structural_with_witness(&base, &declaration, method, invalid);
        assert!(decode_compact_change("bad-applied-witness.lkjc", invalid.as_bytes()).is_err());
    }
}

fn structural_with_witness(base: &str, declaration: &str, method: &str, witness: &str) -> String {
    format!(
        "request base={base}\nreplace.body function={declaration} body=$body\nexpression.block as=$body\n(method-call {witness} {declaration} {method})\nexpression.end\n"
    )
}

#[test]
fn generic_implementation_drafts_preserve_complete_applications_and_owner_identity() {
    use crate::platform::change::CanonicalBaseRead;
    use crate::platform::execution::ExecutionControl;
    use crate::platform::publication::GraphRepository;

    let temporary = tempfile::tempdir().unwrap();
    let initial = crate::platform::kernel::tests::witness_snapshot();
    let created =
        GraphRepository::create(&temporary.path().join("meaning"), &initial, None).unwrap();
    let source = r#"declarations.begin
(units (module create schemes (as $module)
  (owned-contract create Marker (visibility public)
    (self Self) (type-parameter create Self (constraint owned))
    (method method_85000000000000000000000000000004 identity
      (parameters (Self consume)) (returns Self)))
  (function create identity (visibility public) (effect pure)
    (type-parameter create A (constraint owned))
    (type-parameter create B (constraint owned))
    (parameter create owner (type A) (use consume))
    (returns A) (body (local owner)))
  (owned-implementation create Storage (as $scheme) (visibility public)
    (type-parameter create Item (as $item) (constraint owned))
    (type-parameter create Unused (as $unused) (constraint owned))
    (contract Marker) (self (owned-sequence Item))
    (method method_85000000000000000000000000000004 identity (types (owned-sequence Item) Unused)))
  (function create use (visibility public) (effect pure)
    (type-parameter create Storage (constraint owned))
    (type-parameter create Unused (constraint owned))
    (implementation-parameter implparam_85000000000000000000000000000001 ops Marker Storage)
    (parameter create owner (type Storage) (use consume))
    (returns Storage)
    (body (method-call parameter@use@implparam_85000000000000000000000000000001 Marker method_85000000000000000000000000000004 (local owner))))
  (function create run (visibility public) (effect pure)
    (parameter create owner (type (owned-sequence ByteBuffer)) (use consume))
    (returns (owned-sequence ByteBuffer))
    (body (if (bool true)
      (implementation-call use (types (owned-sequence ByteBuffer) OwnedI64Cell)
        (implementations (implementation Storage (types ByteBuffer OwnedI64Cell))) (local owner))
      (method-call (implementation Storage (types ByteBuffer OwnedI64Cell)) Marker method_85000000000000000000000000000004 (local owner)))))))
declarations.end"#;
    let input = format!("request base={}\n{source}\n", created.current.head.revision);
    let request = decode_compact_change("generic-create.lkjc", input.as_bytes()).unwrap();
    let prepared = created
        .repository
        .prepare_authored_change(&request.semantic, request.options)
        .unwrap_or_else(|errors| panic!("{errors:#?}"));
    let module = prepared.allocated["$module"];
    let scheme = prepared.allocated["$scheme"];
    let item = prepared.allocated["$item"];
    let unused = prepared.allocated["$unused"];
    created.repository.publish(&prepared.publication).unwrap();
    let view = created.repository.view_current().unwrap();
    let before_scheme = view.read_owner(scheme).unwrap().value.unwrap();
    let before_item = view.read_owner(item).unwrap().value.unwrap();
    let before_unused = view.read_owner(unused).unwrap().value.unwrap();
    let draft = render_native_draft(
        &view,
        &[module.into()],
        4 * 1_048_576,
        ExecutionControl::uncancelled(),
    )
    .unwrap();
    let text = std::str::from_utf8(&draft).unwrap();
    assert!(text.contains("(implementation "), "{text}");
    assert!(text.contains("(types ByteBuffer OwnedI64Cell)"), "{text}");
    let reentered =
        decode_compact_change_in_repository("generic-reentry.lkjc", &draft, &created.repository)
            .unwrap();
    let errors = created
        .repository
        .prepare_authored_change(&reentered.semantic, reentered.options)
        .unwrap_err();
    assert_eq!(errors.len(), 1, "{errors:#?}");
    assert_eq!(
        errors[0].code, "publication_semantic_no_change",
        "{errors:#?}"
    );

    let edited = text.replace("(bool true)", "(bool false)");
    assert_ne!(edited, text);
    let edited = decode_compact_change_in_repository(
        "generic-edit.lkjc",
        edited.as_bytes(),
        &created.repository,
    )
    .unwrap();
    let prepared = created
        .repository
        .prepare_authored_change(&edited.semantic, edited.options)
        .unwrap_or_else(|errors| panic!("{errors:#?}"));
    created.repository.publish(&prepared.publication).unwrap();
    let after = created.repository.view_current().unwrap();
    assert_eq!(
        after.read_owner(scheme).unwrap().value.unwrap(),
        before_scheme
    );
    assert_eq!(after.read_owner(item).unwrap().value.unwrap(), before_item);
    assert_eq!(
        after.read_owner(unused).unwrap().value.unwrap(),
        before_unused
    );
}

#[test]
fn generic_implementation_unused_parameter_deletion_detaches_the_exact_parent() {
    use crate::platform::change::CanonicalBaseRead;
    use crate::platform::kernel::{DeclarationPayload, OwnerRecord};
    use crate::platform::publication::GraphRepository;

    let temporary = tempfile::tempdir().unwrap();
    let created = GraphRepository::create(
        &temporary.path().join("meaning"),
        &crate::platform::kernel::tests::witness_snapshot(),
        None,
    )
    .unwrap();
    let source = r#"declarations.begin
(units (module create phantom
  (owned-contract create Marker (visibility public)
    (self Self) (type-parameter create Self (constraint owned))
    (method method_85000000000000000000000000000004 identity
      (parameters (Self consume)) (returns Self)))
  (function create identity (visibility public) (effect pure)
    (parameter create owner (type OwnedI64Cell) (use consume))
    (returns OwnedI64Cell) (body (local owner)))
  (owned-implementation create Storage (as $scheme) (visibility public)
    (type-parameter create Unused (as $unused) (constraint owned))
    (contract Marker) (self OwnedI64Cell)
    (method method_85000000000000000000000000000004 identity))))
declarations.end"#;
    let request = decode_compact_change(
        "phantom-create.lkjc",
        format!("request base={}\n{source}\n", created.current.head.revision).as_bytes(),
    )
    .unwrap();
    let prepared = created
        .repository
        .prepare_authored_change(&request.semantic, request.options)
        .unwrap_or_else(|errors| panic!("{errors:#?}"));
    let scheme = prepared.allocated["$scheme"];
    let unused = prepared.allocated["$unused"];
    created.repository.publish(&prepared.publication).unwrap();
    let base = created.repository.current().unwrap().head.revision;
    let request = decode_compact_change(
        "phantom-delete.lkjc",
        format!("request base={base}\ndelete.owner owner={unused} policy=reject\n").as_bytes(),
    )
    .unwrap();
    let prepared = created
        .repository
        .prepare_authored_change(&request.semantic, request.options)
        .unwrap_or_else(|errors| panic!("{errors:#?}"));
    created.repository.publish(&prepared.publication).unwrap();
    let view = created.repository.view_current().unwrap();
    assert!(view.read_owner(unused).unwrap().value.is_none());
    let OwnerRecord::Declaration(scheme) = view.read_owner(scheme).unwrap().value.unwrap() else {
        panic!("expected declaration")
    };
    let DeclarationPayload::OwnedImplementation(scheme) = scheme.payload else {
        panic!("expected scheme")
    };
    assert!(scheme.type_parameters.is_empty());
}
