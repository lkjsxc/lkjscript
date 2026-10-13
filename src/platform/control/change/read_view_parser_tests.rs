//! Public notation controls for complete read-view coordinates and closed tagged forms.
use super::*;
use crate::platform::change::canonical_authored_intent_bytes;

const SCOPE: &str = "read_scope_96000000000000000000000000000001";
const UNUSED: &str = "read_scope_96000000000000000000000000000002";
const FIELD: &str = "field_96000000000000000000000000000003";

fn header() -> String {
    format!("request base=rev_{}\n", "96".repeat(32))
}

fn declarations() -> String {
    format!(
        "create.module as=$module name=views\ncreate.read-view as=$view module=$module name=View visibility=public scope-parameter={SCOPE} scope-name=scope\ncreate.function as=$function module=$module name=subject visibility=public result=unit effect=pure body=$body\nadd.parameter as=$source function=$function name=source type=byte-buffer use=borrow\nread-scope.parameter parent=$function index=0 id={SCOPE} name=scope\nread-scope.parameter-actual as=%scope binder-kind=declaration binder=$function parameter={SCOPE}\n"
    )
}

#[test]
fn native_seven_view_forms_match_independent_compact_requests() {
    let cases = [
        (
            "(view-create $view (types) (scope %scope) (source $source))".to_owned(),
            "expression.read-view-create as=$body declaration=$view read-scope=%scope source=$source\n".to_owned(),
        ),
        (
            "(view-borrow (local $source) (binding inner (as $binding)) (in (unit)))".to_owned(),
            "expression.read-view-borrow as=$body value=$source body=$unit\nexpression.read-view-binding parent=$body index=0 as=$binding name=inner\nexpression.unit as=$unit\n".to_owned(),
        ),
        (
            "(view-reborrow (local $source) %scope)".to_owned(),
            "expression.read-view-reborrow as=$body value=$source read-scope=%scope\n".to_owned(),
        ),
        (
            "(view-move (local $source))".to_owned(),
            "expression.read-view-move as=$body value=$source\n".to_owned(),
        ),
        (
            "(view-drop (local $source))".to_owned(),
            "expression.read-view-drop as=$body value=$source\n".to_owned(),
        ),
        (
            format!("(view-data-field (local $source) $view {FIELD})"),
            format!("expression.read-view-data-field as=$body value=$source declaration=$view field={FIELD}\n"),
        ),
        (
            format!("(borrow-view-field (local $source) $view {FIELD} (binding inner (as $binding) (type byte-buffer)) (in (unit)))"),
            format!("expression.read-view-borrow-field as=$body value=$source declaration=$view field={FIELD} body=$unit\nexpression.read-view-binding parent=$body index=0 as=$binding name=inner type=byte-buffer\nexpression.unit as=$unit\n"),
        ),
    ];
    for (native, compact) in cases {
        let prefix = format!("{}{}", header(), declarations());
        let compact =
            decode_compact_change("view-flat.lkjc", format!("{prefix}{compact}").as_bytes())
                .unwrap();
        let native = decode_compact_change(
            "view-native.lkjc",
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
fn callable_view_relation_retains_unused_formals_and_complete_sources() {
    let input = format!(
        "{}create.module as=$module name=views\ncreate.read-view as=$view module=$module name=View visibility=public scope-parameter={SCOPE} scope-name=scope\nread-scope.parameter-actual as=%scope binder-kind=declaration binder=$function parameter={SCOPE}\ntype.read-view-application as=@result declaration=$view read-scope=%scope\nexpression.read-view-create as=$body declaration=$view read-scope=%scope source=$source\ncreate.function as=$function module=$module name=subject visibility=public result=@result result-kind=view-from result-source=$source result-scope=%scope effect=pure body=$body\nadd.parameter as=$source function=$function name=source type=byte-buffer use=borrow\nread-scope.parameter parent=$function index=1 id={UNUSED} name=unused\nread-scope.parameter parent=$function index=0 id={SCOPE} name=scope\nread-scope.source parent=$function index=0 source=$source scope=%scope\n",
        header()
    );
    let decoded = decode_compact_change("view-result.lkjc", input.as_bytes()).unwrap();
    let AuthoredChange::CreateFunction {
        result,
        read_scope_parameters,
        read_scope_sources,
        body,
        ..
    } = &decoded.semantic.changes[2]
    else {
        panic!("function expected");
    };
    assert_eq!(
        read_scope_parameters
            .iter()
            .map(|p| p.id.to_string())
            .collect::<Vec<_>>(),
        [SCOPE, UNUSED]
    );
    assert_eq!(read_scope_sources.len(), 1);
    assert!(
        matches!(&result.relation, AuthoredResultRelation::ViewFrom { source: AuthoredLocalReference::Symbol { symbol }, scope: AuthoredReadScopeArgument::Parameter { parameter, .. } } if symbol == "$source" && parameter.to_string() == SCOPE)
    );
    assert!(
        matches!(&body.operation, AuthoredExpressionOperation::CreateReadView { source: AuthoredLocalReference::Symbol { symbol }, fields, .. } if symbol == "$source" && fields.is_empty())
    );
}

#[test]
fn closed_result_scope_and_field_kinds_refuse_extra_coordinates() {
    let prefix = format!("{}{}", header(), declarations());
    let base = format!("{prefix}expression.unit as=$body\n");
    for suffix in [
        "result-kind=value result-source=$source",
        "result-kind=read-from result-source=$source result-scope=%scope",
        "result-kind=view-from result-source=$source",
        "result-kind=unknown",
        "borrow-from=$source",
        "result-mode=view-from",
    ] {
        let malformed = base.replace(
            "result=unit effect=pure",
            &format!("result=unit {suffix} effect=pure"),
        );
        assert!(
            decode_compact_change("view-result-refusal.lkjc", malformed.as_bytes()).is_err(),
            "{suffix}"
        );
    }
    for extra in [
        format!(
            "read-view.field parent=$view index=0 id={FIELD} name=item type=i64 kind=ordinary read-scope=%scope\n"
        ),
        format!(
            "read-view.field parent=$view index=0 id={FIELD} name=item type=i64 kind=read-reference\n"
        ),
        format!(
            "read-view.field parent=$view index=0 id={FIELD} name=item type=i64 kind=unknown\n"
        ),
        format!(
            "read-view.field parent=$view index=1 id={FIELD} name=item type=i64 kind=ordinary\n"
        ),
        format!(
            "read-view.field parent=$view index=0 id={FIELD} name=item type=i64 kind=ordinary\nread-view.field parent=$view index=1 id={FIELD} name=again type=i64 kind=ordinary\n"
        ),
        format!(
            "read-scope.parameter-actual as=%bad binder-kind=declaration binder=$function parameter={SCOPE} member=assoc_type_96000000000000000000000000000004\n"
        ),
    ] {
        assert!(
            decode_compact_change(
                "view-field-refusal.lkjc",
                format!("{base}{extra}").as_bytes()
            )
            .is_err(),
            "{extra}"
        );
    }
}

#[test]
fn lexical_view_scope_resolves_inside_borrow_body_and_does_not_escape() {
    let prefix = format!("{}{}", header(), declarations());
    let source = format!(
        "{prefix}expression.block as=$body\n(view-borrow (local $source) (binding inner (as $binding)) (in (view-reborrow (local inner) (lexical-read-scope $function inner))))\nexpression.end\n"
    );
    let decoded = decode_compact_change("view-lexical.lkjc", source.as_bytes()).unwrap();
    let AuthoredChange::CreateFunction { body, .. } = &decoded.semantic.changes[2] else {
        panic!()
    };
    let AuthoredExpressionOperation::BorrowReadView { body, .. } = &body.operation else {
        panic!()
    };
    assert!(
        matches!(&body.operation, AuthoredExpressionOperation::ReborrowReadView { value: AuthoredLocalReference::Symbol { symbol: value }, read_scope: AuthoredReadScopeArgument::Lexical { binding: AuthoredLocalReference::Symbol { symbol: scope }, .. } } if value == "$binding" && scope == "$binding")
    );
    let escaped = format!(
        "{prefix}expression.block as=$body\n(sequence (view-borrow (local $source) (binding inner) (in (unit))) (view-reborrow (local $source) (lexical-read-scope $function inner)))\nexpression.end\n"
    );
    assert!(decode_compact_change("view-scope-escape.lkjc", escaped.as_bytes()).is_err());
}

#[test]
fn native_view_families_and_method_scopes_retain_complete_inline_inventory() {
    let member = "assoc_type_96000000000000000000000000000004";
    let method = "method_96000000000000000000000000000005";
    let input = format!(
        r#"{}declarations.begin
(units (module create views (as $module)
  (read-view create View (as $view) (visibility public)
    (scope-parameter {SCOPE} scope)
    (view-field {FIELD} item (type ByteBuffer)
      (read-reference (read-scope (declaration View) {SCOPE}))))
  (owned-contract create Read (as $contract) (visibility public) (self Self)
    (type-parameter create Self (as $self) (constraint owned))
    (associated-type {member} View (read-view-family (scope-parameter {SCOPE} scope)))
    (method {method} read (parameters (Self borrow))
      (returns (associated-read-view-member Read {member} (read-scope (contract-method Read {method}) {SCOPE}))
        (view-from 0 (read-scope (contract-method Read {method}) {SCOPE})))
      (effect pure)
      (scope-parameters (scope-parameter {SCOPE} scope) (scope-parameter {UNUSED} unused))
      (scope-sources (scope-source 0 (read-scope (contract-method Read {method}) {SCOPE})))))
  (owned-implementation create BufferRead (as $implementation) (visibility public) (contract Read) (self ByteBuffer)
    (associated-type {member} (read-view-family (scope-parameter {SCOPE} scope)
      (read-view-application View (types) (read-scope (associated-binding BufferRead {member}) {SCOPE}))))
    (method {method} maker (types) (implementations) (scopes (read-scope (associated-binding BufferRead {member}) {SCOPE}))))
  (function create maker (as $maker) (visibility public)
    (parameter create source (as $source) (type ByteBuffer) (use borrow))
    (scope-parameter {SCOPE} scope)
    (scope-parameter {UNUSED} unused)
    (scope-source source (read-scope (declaration maker) {SCOPE}))
    (returns (read-view-application View (types) (read-scope (declaration maker) {SCOPE}))
      (view-from source (read-scope (declaration maker) {SCOPE})))
    (effect pure)
    (body (view-create View (types) (scope (read-scope (declaration maker) {SCOPE}))
      (source source) (field {FIELD} (local source)))))))
declarations.end
"#,
        header()
    );
    let decoded = decode_compact_change("native-view-families.lkjc", input.as_bytes()).unwrap();
    let view = decoded
        .semantic
        .changes
        .iter()
        .find_map(|change| match change {
            AuthoredChange::CreateReadView { fields, .. } => Some(fields),
            _ => None,
        })
        .unwrap();
    assert_eq!(view.len(), 1);
    assert!(matches!(
        view[0].kind,
        AuthoredReadViewFieldKind::ReadReference { .. }
    ));
    let (methods, associated_types) = decoded
        .semantic
        .changes
        .iter()
        .find_map(|change| match change {
            AuthoredChange::CreateOwnedContract {
                methods,
                associated_types,
                ..
            } => Some((methods, associated_types)),
            _ => None,
        })
        .unwrap();
    assert!(
        matches!(&associated_types[0].bound, AssociatedTypeBound::ReadViewFamily { scope_parameter } if scope_parameter.id.to_string() == SCOPE)
    );
    assert_eq!(methods[0].read_scope_parameters.len(), 2);
    assert_eq!(methods[0].read_scope_sources[0].source, 0);
    assert!(matches!(
        methods[0].result.relation,
        AuthoredResultRelation::ViewFrom { source: 0, .. }
    ));
    let (bindings, mappings) = decoded
        .semantic
        .changes
        .iter()
        .find_map(|change| match change {
            AuthoredChange::CreateOwnedImplementation {
                associated_types,
                methods,
                ..
            } => Some((associated_types, methods)),
            _ => None,
        })
        .unwrap();
    assert!(
        matches!(&bindings[0].value, AuthoredAssociatedTypeBindingValue::ReadViewFamily { ty: AuthoredType::ReadViewApplied { arguments, .. }, .. } if arguments.is_empty())
    );
    assert_eq!(mappings[0].read_scope_arguments.len(), 1);
    let function = decoded
        .semantic
        .changes
        .iter()
        .find_map(|change| match change {
            AuthoredChange::CreateFunction {
                read_scope_parameters,
                read_scope_sources,
                ..
            } => Some((read_scope_parameters, read_scope_sources)),
            _ => None,
        })
        .unwrap();
    assert_eq!(function.0.len(), 2);
    assert_eq!(function.1.len(), 1);
}
