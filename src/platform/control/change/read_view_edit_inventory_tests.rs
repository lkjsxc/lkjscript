//! Native complete read-view edits keep retained and newly authored generic identities.
use super::*;
use crate::platform::change::{CanonicalBaseRead, PrimitiveEdit, lower_authored_changes};
use crate::platform::kernel::{self as k, DeclarationPayload, OwnerRecord, TypeForm};
use crate::platform::publication::GraphRepository;
use crate::platform::semantic_id::TypeParameterId;

const SCOPE: &str = "read_scope_97000000000000000000000000000001";
const DATA: &str = "field_97000000000000000000000000000002";
const REFERENCE: &str = "field_97000000000000000000000000000003";
const APPENDED: &str = "field_97000000000000000000000000000004";

struct Fixture {
    _temporary: tempfile::TempDir,
    repository: GraphRepository,
    module: ModuleId,
    window: DeclarationId,
    item: TypeParameterId,
    unused: TypeParameterId,
}

impl Fixture {
    fn new() -> Self {
        let temporary = tempfile::tempdir().unwrap();
        let created = GraphRepository::create(
            &temporary.path().join("meaning"),
            &k::tests::witness_snapshot(),
            None,
        )
        .unwrap();
        let source = format!(
            r#"request base={}
declarations.begin
(units (module create inventory (as $module)
  (read-view create Window (as $window) (visibility public)
    (type-parameter create Item (as $item) (constraint owned))
    (type-parameter create Unused (as $unused) (constraint owned))
    (scope-parameter {SCOPE} source)
    (view-field {DATA} count (type I64) (ordinary))
    (view-field {REFERENCE} item (type Item)
      (read-reference (read-scope (declaration Window) {SCOPE}))))))
declarations.end
"#,
            created.current.head.revision
        );
        let request =
            decode_compact_change("view-inventory-create.lkjc", source.as_bytes()).unwrap();
        let prepared = created
            .repository
            .prepare_authored_change(&request.semantic, request.options)
            .unwrap_or_else(|errors| panic!("read-view inventory fixture: {errors:#?}"));
        created.repository.publish(&prepared.publication).unwrap();
        let module = match prepared.allocated["$module"] {
            OwnerKey::Module(id) => id,
            _ => panic!("module identity"),
        };
        let window = match prepared.allocated["$window"] {
            OwnerKey::Declaration(id) => id,
            _ => panic!("read-view identity"),
        };
        let parameter = |symbol| match prepared.allocated[symbol] {
            OwnerKey::TypeParameter(id) => id,
            _ => panic!("type-parameter identity"),
        };
        Self {
            _temporary: temporary,
            repository: created.repository,
            module,
            window,
            item: parameter("$item"),
            unused: parameter("$unused"),
        }
    }

    fn retained_item(&self) -> String {
        format!(
            "(type-parameter edit {} Item (as $item) (constraint owned))",
            self.item
        )
    }

    fn retained_unused(&self) -> String {
        format!(
            "(type-parameter edit {} Unused (as $unused) (constraint owned))",
            self.unused
        )
    }

    fn edit(&self, children: &str, extra_fields: &str, deletes: &str) -> String {
        let view = self.repository.view_current().unwrap();
        format!(
            r#"request base={}
{deletes}declarations.begin
(units (module edit {} inventory
  (read-view edit {} Window (as $window) (visibility public)
    {children}
    (scope-parameter {SCOPE} source)
    (view-field {DATA} count (type I64) (ordinary))
    (view-field {REFERENCE} item (type Item)
      (read-reference (read-scope (declaration Window) {SCOPE})))
    {extra_fields})))
declarations.end
"#,
            view.revision(),
            self.module,
            self.window,
        )
    }
}

fn appended_field() -> String {
    format!(
        "(view-field {APPENDED} added (type Added) (read-reference (read-scope (declaration Window) {SCOPE})))"
    )
}

fn set_inventory(request: &NormalizedChangeRequest) -> &[AuthoredTypeParameterReference] {
    request
        .semantic
        .changes
        .iter()
        .find_map(|change| match change {
            AuthoredChange::SetReadView {
                type_parameters, ..
            } => Some(type_parameters.as_slice()),
            _ => None,
        })
        .unwrap()
}

fn assert_lowered_inventory(
    fixture: &Fixture,
    request: &NormalizedChangeRequest,
    retain_unused: bool,
) {
    let before = fixture.repository.view_current().unwrap();
    let lowered = lower_authored_changes(&before, &before, &request.semantic).unwrap();
    let OwnerKey::TypeParameter(added) = lowered.allocated["$added"] else {
        panic!("new generic identity")
    };
    let window = lowered
        .edits
        .iter()
        .find_map(|edit| match edit {
            PrimitiveEdit::InsertOwner {
                record: OwnerRecord::Declaration(record),
            }
            | PrimitiveEdit::ReplaceOwner {
                record: OwnerRecord::Declaration(record),
                ..
            } if record.header.owner == OwnerKey::Declaration(fixture.window) => {
                match &record.payload {
                    DeclarationPayload::ReadView(window) => Some(window),
                    _ => None,
                }
            }
            _ => None,
        })
        .unwrap();
    let expected = if retain_unused {
        vec![fixture.item, fixture.unused, added]
    } else {
        vec![fixture.item, added]
    };
    assert_eq!(window.type_parameters, expected);
    assert_eq!(lowered.edits.iter().any(|edit| matches!(edit, PrimitiveEdit::DeleteOwner { owner, .. } if *owner == OwnerKey::TypeParameter(fixture.unused))), !retain_unused);
    assert_eq!(window.scope_parameter.id.to_string(), SCOPE);
    assert_eq!(window.scope_parameter.name.as_str(), "source");
    assert_eq!(
        window
            .fields
            .iter()
            .map(|field| field.id.to_string())
            .collect::<Vec<_>>(),
        [DATA, REFERENCE, APPENDED]
    );
    let declaration = k::DeclarationReference {
        package: before.package(),
        declaration: fixture.window,
    };
    for (field, parameter) in [
        (&window.fields[1], fixture.item),
        (&window.fields[2], added),
    ] {
        assert_eq!(
            field.kind,
            k::ReadViewFieldKind::ReadReference {
                read_scope: k::ReadScopeArgument::Parameter(k::ReadScopeReference {
                    binder: k::ReadScopeBinder::Declaration(declaration),
                    parameter: window.scope_parameter.id,
                }),
            }
        );
        let ty = lowered
            .edits
            .iter()
            .find_map(|edit| match edit {
                PrimitiveEdit::AddTypeObject { digest, object } if *digest == field.ty => {
                    Some(object.clone())
                }
                _ => None,
            })
            .or_else(|| before.read_type_object(field.ty).unwrap().value)
            .unwrap();
        assert_eq!(ty.form, TypeForm::TypeParameter { parameter });
    }
    assert_eq!(
        fixture.repository.view_current().unwrap().revision(),
        before.revision()
    );
}

#[test]
fn read_view_complete_edit_appends_new_generic_after_retained_parameters() {
    let fixture = Fixture::new();
    let children = format!(
        "{} {} (type-parameter create Added (as $added) (constraint owned))",
        fixture.retained_item(),
        fixture.retained_unused()
    );
    let input = fixture.edit(&children, &appended_field(), "");
    let request = decode_compact_change_in_repository(
        "view-inventory-append.lkjc",
        input.as_bytes(),
        &fixture.repository,
    )
    .unwrap();
    assert_eq!(
        set_inventory(&request),
        [
            AuthoredTypeParameterReference::Id {
                parameter: fixture.item
            },
            AuthoredTypeParameterReference::Id {
                parameter: fixture.unused
            },
            AuthoredTypeParameterReference::Symbol {
                symbol: "$added".into()
            }
        ]
    );
    assert_lowered_inventory(&fixture, &request, true);
}

#[test]
fn read_view_complete_edit_deletes_unused_generic_and_retains_new_template_binding() {
    let fixture = Fixture::new();
    let children = format!(
        "{} (type-parameter create Added (as $added) (constraint owned))",
        fixture.retained_item()
    );
    let deletes = format!("delete.owner owner={} policy=reject\n", fixture.unused);
    let input = fixture.edit(&children, &appended_field(), &deletes);
    let request = decode_compact_change_in_repository(
        "view-inventory-delete-append.lkjc",
        input.as_bytes(),
        &fixture.repository,
    )
    .unwrap();
    assert_eq!(
        set_inventory(&request),
        [
            AuthoredTypeParameterReference::Id {
                parameter: fixture.item
            },
            AuthoredTypeParameterReference::Symbol {
                symbol: "$added".into()
            }
        ]
    );
    assert_lowered_inventory(&fixture, &request, false);
}

#[test]
fn read_view_complete_edit_refuses_reorder_insertion_and_unaccounted_omission() {
    let fixture = Fixture::new();
    let added = "(type-parameter create Added (as $added) (constraint owned))";
    for (children, code) in [
        (
            format!("{} {}", fixture.retained_unused(), fixture.retained_item()),
            "change_unit_order",
        ),
        (
            format!(
                "{added} {} {}",
                fixture.retained_item(),
                fixture.retained_unused()
            ),
            "change_unit_order",
        ),
        (fixture.retained_item(), "change_unit_children"),
    ] {
        let before = fixture.repository.view_current().unwrap().revision();
        let input = fixture.edit(&children, "", "");
        let errors = decode_compact_change_in_repository(
            "view-inventory-refusal.lkjc",
            input.as_bytes(),
            &fixture.repository,
        )
        .unwrap_err();
        assert_eq!(errors[0].class, DiagnosticClass::Source);
        assert_eq!(errors[0].code, code);
        assert_eq!(
            fixture.repository.view_current().unwrap().revision(),
            before
        );
    }
}

fn delete_unused_and_append_request(fixture: &Fixture) -> NormalizedChangeRequest {
    let children = format!(
        "{} (type-parameter create Added (as $added) (constraint owned))",
        fixture.retained_item()
    );
    let deletes = format!("delete.owner owner={} policy=reject\n", fixture.unused);
    let input = fixture.edit(&children, &appended_field(), &deletes);
    decode_compact_change_in_repository(
        "view-inventory-delete-publication.lkjc",
        input.as_bytes(),
        &fixture.repository,
    )
    .unwrap()
}

#[test]
fn read_view_complete_edit_publishes_exact_parameter_retirement_and_new_binding() {
    let fixture = Fixture::new();
    let request = delete_unused_and_append_request(&fixture);
    let before = fixture.repository.view_current().unwrap().revision();
    let prepared = fixture
        .repository
        .prepare_authored_change(&request.semantic, request.options)
        .expect("complete view inventory and explicit unused-parameter deletion prepare together");
    let OwnerKey::TypeParameter(added) = prepared.allocated["$added"] else {
        panic!("new generic identity")
    };
    assert_eq!(
        fixture.repository.view_current().unwrap().revision(),
        before
    );
    fixture.repository.publish(&prepared.publication).unwrap();
    let accepted = fixture
        .repository
        .view_current()
        .unwrap()
        .reconstruct_full_oracle()
        .unwrap()
        .value;
    k::validate_full(&accepted).unwrap();
    assert!(
        accepted
            .owners
            .contains_key(&OwnerKey::TypeParameter(fixture.item))
    );
    assert!(
        accepted
            .owners
            .contains_key(&OwnerKey::TypeParameter(added))
    );
    assert!(
        !accepted
            .owners
            .contains_key(&OwnerKey::TypeParameter(fixture.unused))
    );
    let retirement = &accepted.retirements[&OwnerKey::TypeParameter(fixture.unused)];
    assert_eq!(
        retirement.last_parent,
        Some(OwnerKey::Declaration(fixture.window))
    );
    assert_eq!(retirement.last_live_revision, before);
    let OwnerRecord::Declaration(declaration) =
        &accepted.owners[&OwnerKey::Declaration(fixture.window)]
    else {
        panic!("retained view")
    };
    let DeclarationPayload::ReadView(view) = &declaration.payload else {
        panic!("retained view category")
    };
    assert_eq!(view.type_parameters, [fixture.item, added]);
    let added_type = &accepted.types[&view.fields[2].ty];
    assert_eq!(
        added_type.form,
        TypeForm::TypeParameter { parameter: added }
    );
}

#[test]
fn read_view_complete_edit_deletion_refuses_duplicate_membership_and_new_live_reference() {
    let fixture = Fixture::new();
    let before = fixture.repository.view_current().unwrap();
    for (duplicate, expected) in [
        (true, "change_delete_parent_membership"),
        (false, "change_delete_live_reference"),
    ] {
        let mut request = delete_unused_and_append_request(&fixture);
        for change in &mut request.semantic.changes {
            if let AuthoredChange::SetReadView {
                type_parameters,
                fields,
                ..
            } = change
            {
                if duplicate {
                    for _ in 0..2 {
                        type_parameters.push(AuthoredTypeParameterReference::Id {
                            parameter: fixture.unused,
                        });
                    }
                } else {
                    fields[1].ty = AuthoredType::TypeParameter {
                        parameter: AuthoredTypeParameterReference::Id {
                            parameter: fixture.unused,
                        },
                    };
                }
            }
        }
        let diagnostic = lower_authored_changes(&before, &before, &request.semantic).expect_err(
            "complete replacement cannot hide duplicate membership or a live field use",
        );
        assert_eq!(diagnostic.class, DiagnosticClass::Semantic);
        assert_eq!(diagnostic.code, expected);
        assert_eq!(
            fixture.repository.view_current().unwrap().revision(),
            before.revision()
        );
    }
}

#[test]
fn read_view_complete_edit_deletion_rechecks_parent_witness_and_admits_original_scan() {
    let fixture = Fixture::new();
    let before = fixture.repository.view_current().unwrap();
    let snapshot = before.reconstruct_full_oracle().unwrap().value;
    let mut witness = crate::platform::witness::rebuild_full_witness(&snapshot).unwrap();
    witness
        .entries
        .ownership
        .get_mut(&OwnerKey::TypeParameter(fixture.unused))
        .unwrap()
        .parent =
        crate::platform::witness::OwnershipParent::Owner(OwnerKey::Module(fixture.module));
    let request = delete_unused_and_append_request(&fixture);
    let diagnostic = lower_authored_changes(&before, &witness, &request.semantic).expect_err(
        "already detached candidate membership cannot replace the exact accepted parent proof",
    );
    assert_eq!(diagnostic.class, DiagnosticClass::Corrupt);
    assert_eq!(diagnostic.code, "change_delete_ownership_disagreement");
    assert_eq!(
        fixture.repository.view_current().unwrap().revision(),
        before.revision()
    );

    let mut request = request;
    request.semantic.budget.impact.maximum_ownership_steps = Some(0);
    let diagnostic = lower_authored_changes(&before, &before, &request.semantic).expect_err(
        "accepted read-view membership inspection must obey the ownership work admission",
    );
    assert_eq!(diagnostic.class, DiagnosticClass::Resource);
    assert_eq!(diagnostic.code, "change_budget_impact_ownership_steps");
    assert_eq!(
        fixture.repository.view_current().unwrap().revision(),
        before.revision()
    );
}
