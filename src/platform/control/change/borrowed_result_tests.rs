//! Independent native and compact proposals retain exact borrowed-result provenance.
use super::*;
use crate::platform::change::{CanonicalBaseRead, canonical_authored_intent_bytes};
use crate::platform::execution::ExecutionControl;
use crate::platform::kernel::{DeclarationPayload, ExpressionOperation, OwnerRecord};
use crate::platform::publication::GraphRepository;

#[test]
fn borrowed_result_native_intent_matches_independent_compact_edges() {
    let base = format!("rev_{}", "82".repeat(32));
    let flat = format!(
        r#"request base={base}
create.module as=$module name=views
add.parameter as=$source function=$identity name=source type=owned-i64-cell use=borrow
create.function as=$identity module=$module name=identity visibility=public result=owned-i64-cell borrow-from=$source effect=pure body=$body
expression.local as=$body value=$source
"#
    );
    let native = format!(
        r#"request base={base}
declarations.begin
(units (module create views (as $module)
  (function create identity (as $identity) (visibility public)
    (parameter create source (as $source) (type OwnedI64Cell) (use borrow))
    (returns OwnedI64Cell (borrow-from source)) (effect pure)
    (body (local source)))))
declarations.end
"#
    );
    let flat = decode_compact_change("borrowed-flat.lkjc", flat.as_bytes()).unwrap();
    let native = decode_compact_change("borrowed-native.lkjc", native.as_bytes()).unwrap();
    let bytes = canonical_authored_intent_bytes(&native.semantic).unwrap();
    assert_eq!(&bytes[..8], b"LKJACR31");
    assert_eq!(
        bytes,
        canonical_authored_intent_bytes(&flat.semantic).unwrap()
    );
    assert_eq!(native.request_commitment, flat.request_commitment);
    assert_eq!(
        commitment_codec_identity(&bytes),
        "lkjscript-authored-change-codec-31"
    );
    assert_eq!(
        commitment_codec_identity(b"LKJACR30"),
        "lkjscript-authored-change-codec-30"
    );
}

#[test]
fn borrowed_contract_method_positions_bind_independent_intent() {
    let base = format!("rev_{}", "82".repeat(32));
    let native = format!(
        r#"request base={base}
declarations.begin
(units (module create views (as $module)
  (owned-contract create IndexRead (as $contract) (visibility public) (self Self)
    (type-parameter create Self (as $self) (constraint owned))
    (method method_85000000000000000000000000000003 at
      (parameters (I64 unrestricted) (Self borrow))
      (returns Self (borrow-from 1))))))
declarations.end
"#
    );
    let flat = format!(
        r#"request base={base}
create.module as=$module name=views
type.parameter as=@self parameter=$self
create.owned-contract as=$contract module=$module name=IndexRead visibility=public self=@self
owned.method parent=$contract index=0 as=%method id=method_85000000000000000000000000000003 name=at result=@self borrow-from=1
owned.parameter parent=%method index=0 type=i64 use=unrestricted
owned.parameter parent=%method index=1 type=@self use=borrow
add.type-parameter as=$self declaration=$contract name=Self constraint=owned
"#
    );
    let native = decode_compact_change("borrowed-method-native.lkjc", native.as_bytes()).unwrap();
    let flat = decode_compact_change("borrowed-method-flat.lkjc", flat.as_bytes()).unwrap();
    assert_eq!(
        canonical_authored_intent_bytes(&native.semantic).unwrap(),
        canonical_authored_intent_bytes(&flat.semantic).unwrap()
    );
    assert_eq!(native.request_commitment, flat.request_commitment);
}

const SOURCE: &str = r#"declarations.begin
(units (module create views (as $module)
  (function create identity (as $identity) (visibility public)
    (parameter create source (type OwnedI64Cell) (use borrow))
    (returns OwnedI64Cell (borrow-from source)) (effect pure)
    (body (local source)))
  (function create observer (as $observer) (visibility public)
    (parameter create source (type OwnedI64Cell) (use borrow))
    (returns I64) (effect pure)
    (body (borrow-call (call identity (local source))
      (binding view (type OwnedI64Cell)) (in (sequence (i64 3) (i64 5))))))))
declarations.end
"#;

#[test]
fn borrowed_result_drafts_reenter_and_literal_edits_keep_provenance() {
    let temporary = tempfile::tempdir().unwrap();
    let initial = crate::platform::kernel::tests::witness_snapshot();
    let created =
        GraphRepository::create(&temporary.path().join("meaning"), &initial, None).unwrap();
    let input = format!("request base={}\n{SOURCE}", created.current.head.revision);
    let request = decode_compact_change("borrowed-result.lkjc", input.as_bytes()).unwrap();
    let prepared = created
        .repository
        .prepare_authored_change(&request.semantic, request.options)
        .unwrap_or_else(|errors| panic!("borrowed fixture: {errors:#?}"));
    let module = prepared.allocated["$module"];
    let identity = prepared.allocated["$identity"];
    let observer = prepared.allocated["$observer"];
    created.repository.publish(&prepared.publication).unwrap();
    let before = created.repository.view_current().unwrap();
    let draft = String::from_utf8(
        render_native_draft(
            &before,
            &[module.into()],
            4 * 1_048_576,
            ExecutionControl::uncancelled(),
        )
        .unwrap(),
    )
    .unwrap();
    assert!(draft.contains("(borrow-from source)"), "{draft}");
    assert!(draft.contains("(borrow-call "), "{draft}");
    let unchanged = decode_compact_change_in_repository(
        "borrowed-noop.lkjc",
        draft.as_bytes(),
        &created.repository,
    )
    .unwrap();
    assert!(!unchanged.semantic.changes.iter().any(|c| matches!(
        c,
        AuthoredChange::ReplaceFunctionBody { .. } | AuthoredChange::SetFunctionLiterals { .. }
    )));
    let errors = created
        .repository
        .prepare_authored_change(&unchanged.semantic, unchanged.options)
        .unwrap_err();
    assert_eq!(errors[0].code, "publication_semantic_no_change");
    let OwnerRecord::Declaration(old_identity) =
        before.read_owner(identity).unwrap().value.unwrap()
    else {
        panic!()
    };
    let DeclarationPayload::Function(old_identity) = old_identity.payload else {
        panic!()
    };
    let OwnerRecord::Declaration(old_observer) =
        before.read_owner(observer).unwrap().value.unwrap()
    else {
        panic!()
    };
    let DeclarationPayload::Function(old_observer) = old_observer.payload else {
        panic!()
    };
    let old_scope = before
        .read_owner(OwnerKey::Expression(old_observer.body))
        .unwrap()
        .value
        .unwrap();
    let edited = draft.replace("(i64 3)", "(i64 4)");
    let edited = decode_compact_change_in_repository(
        "borrowed-edit.lkjc",
        edited.as_bytes(),
        &created.repository,
    )
    .unwrap();
    assert!(
        !edited
            .semantic
            .changes
            .iter()
            .any(|c| matches!(c, AuthoredChange::ReplaceFunctionBody { .. }))
    );
    let prepared = created
        .repository
        .prepare_authored_change(&edited.semantic, edited.options)
        .unwrap();
    created.repository.publish(&prepared.publication).unwrap();
    let after = created.repository.view_current().unwrap();
    let OwnerRecord::Declaration(new_identity) = after.read_owner(identity).unwrap().value.unwrap()
    else {
        panic!()
    };
    let DeclarationPayload::Function(new_identity) = new_identity.payload else {
        panic!()
    };
    assert_eq!(old_identity, new_identity);
    assert_eq!(
        old_scope,
        after
            .read_owner(OwnerKey::Expression(old_observer.body))
            .unwrap()
            .value
            .unwrap()
    );
    let OwnerRecord::Expression(scope) = old_scope else {
        panic!()
    };
    let ExpressionOperation::BorrowCall { binding, .. } = scope.operation else {
        panic!()
    };
    assert_eq!(
        before.read_owner(OwnerKey::Binding(binding)).unwrap().value,
        after.read_owner(OwnerKey::Binding(binding)).unwrap().value
    );
}

#[test]
fn borrow_call_rejects_non_invocations_and_lexical_escape() {
    let base = format!("rev_{}", "82".repeat(32));
    for body in [
        "(borrow-call (unit) (binding view (type OwnedI64Cell)) (in (unit)))",
        "(borrow-call (call $identity) (binding view) (in (unit)))",
        "(sequence (borrow-call (call $identity) (binding view (type OwnedI64Cell)) (in (unit))) (local view))",
        "(borrow-call (call $identity (local view)) (binding view (type OwnedI64Cell)) (in (unit)))",
    ] {
        let input = format!(
            "request base={base}\ncreate.module as=$module name=views\ncreate.function as=$identity module=$module name=identity visibility=public result=unit effect=pure body=$body\nexpression.block as=$body\n{body}\nexpression.end\n"
        );
        assert!(
            decode_compact_change("borrowed-invalid.lkjc", input.as_bytes()).is_err(),
            "{body}"
        );
    }
}

#[test]
fn borrow_call_uses_existing_invocation_grammar_and_shadows_after_invocation() {
    let base = format!("rev_{}", "82".repeat(32));
    let function = format!("pkg_{}/decl_{}", "31".repeat(16), "32".repeat(16));
    let contract = format!("pkg_{}/decl_{}", "31".repeat(16), "33".repeat(16));
    for (invocation, flat_invocation) in [
        (
            format!("(call {function} (local $source))"),
            format!("expression.call as=$call function={function}\n"),
        ),
        (
            format!(
                "(implementation-call {function} (implementations concrete@{contract}) (local $source))"
            ),
            format!(
                "expression.implementation-call as=$call function={function}\nimplementation.argument parent=$call index=0 implementation=concrete@{contract}\n"
            ),
        ),
        (
            format!(
                "(method-call concrete@{contract} {contract} method_85000000000000000000000000000003 (local $source))"
            ),
            format!(
                "expression.method-call as=$call witness=concrete@{contract} contract={contract} method=method_85000000000000000000000000000003\n"
            ),
        ),
    ] {
        let declarations = "create.module as=$module name=views\ncreate.function as=$identity module=$module name=identity visibility=public result=owned-i64-cell effect=pure body=$body\nadd.parameter as=$source function=$identity name=source type=owned-i64-cell use=borrow\n";
        let flat = format!(
            "request base={base}\n{declarations}expression.borrow-call as=$body call=$call body=$read\nexpression.call-binding parent=$body index=0 as=$view name=source type=owned-i64-cell\n{flat_invocation}expression.argument parent=$call index=0 expression=$argument\nexpression.local as=$argument value=$source\nexpression.local as=$read value=$view\n"
        );
        let structural = format!(
            "request base={base}\n{declarations}expression.block as=$body\n(borrow-call {invocation} (binding source (as $view) (type owned-i64-cell)) (in (local source)))\nexpression.end\n"
        );
        let flat = decode_compact_change("borrow-call-flat.lkjc", flat.as_bytes()).unwrap();
        let structural =
            decode_compact_change("borrow-call-structural.lkjc", structural.as_bytes()).unwrap();
        assert_eq!(
            canonical_authored_intent_bytes(&flat.semantic).unwrap(),
            canonical_authored_intent_bytes(&structural.semantic).unwrap()
        );
        assert_eq!(flat.request_commitment, structural.request_commitment);
        let AuthoredChange::CreateFunction { body, .. } = &structural.semantic.changes[1] else {
            panic!()
        };
        let AuthoredExpressionOperation::BorrowCall {
            call,
            binding,
            body,
        } = &body.operation
        else {
            panic!()
        };
        assert_eq!(binding.symbol, "$view");
        assert!(
            matches!(&body.operation, AuthoredExpressionOperation::Local { value: AuthoredLocalReference::Symbol { symbol } } if symbol == "$view")
        );
        let (AuthoredExpressionOperation::Call { arguments, .. }
        | AuthoredExpressionOperation::ImplementationCall { arguments, .. }
        | AuthoredExpressionOperation::MethodCall { arguments, .. }) = &call.operation
        else {
            panic!()
        };
        assert!(
            matches!(&arguments[0].operation, AuthoredExpressionOperation::Local { value: AuthoredLocalReference::Symbol { symbol } } if symbol == "$source")
        );
    }
}
