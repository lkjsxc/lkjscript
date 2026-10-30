//! Public preparation proves that ordering preserves identities and uses final live membership.

use super::*;
use crate::platform::change::ChangeBudget;
use crate::platform::change::request::{
    AuthoredCallableResult, AuthoredChange, AuthoredChangeSet, AuthoredDeletePolicy,
    AuthoredExpression, AuthoredExpressionOperation, AuthoredFunctionEffect,
    AuthoredLoweringInputs, AuthoredOperation, AuthoredParameter, AuthoredType,
    DeclarationSelector, ModuleSelector,
};
use crate::platform::kernel::{
    ChangeDigest, DeclarationVisibility, ExternalVisibility, Idempotency, KernelSnapshot, Name,
    ParameterUse,
};
use crate::platform::publication::{GraphRepository, PublicationOptions, RepositoryView};
use std::collections::BTreeMap;

struct Fixture {
    _temporary: tempfile::TempDir,
    repository: GraphRepository,
    owners: BTreeMap<String, OwnerKey>,
    before: KernelSnapshot,
}

fn parameter(symbol: &str, name: &str, ty: AuthoredType) -> AuthoredParameter {
    AuthoredParameter {
        symbol: symbol.to_owned(),
        name: Name::new(name).unwrap(),
        ty,
        use_mode: ParameterUse::Unrestricted,
        resource_requirement: None,
    }
}

fn function(symbol: &str, parameters: Vec<AuthoredParameter>) -> AuthoredChange {
    AuthoredChange::CreateFunction {
        symbol: symbol.to_owned(),
        module: ModuleSelector::Symbol {
            symbol: "$module".to_owned(),
        },
        name: Name::new(symbol.trim_start_matches('$')).unwrap(),
        visibility: DeclarationVisibility::Private,
        type_parameters: Vec::new(),
        parameters,
        result: AuthoredCallableResult::value(AuthoredType::I64 {}),
        read_scope_parameters: Vec::new(),
        read_scope_sources: Vec::new(),
        effect: AuthoredFunctionEffect::Pure {},
        body: AuthoredExpression {
            symbol: None,
            operation: AuthoredExpressionOperation::I64 { value: 0 },
        },
    }
}

fn fixture() -> Fixture {
    let temporary = tempfile::tempdir().unwrap();
    let created = GraphRepository::create(
        &temporary.path().join("parameter-order"),
        &crate::platform::kernel::tests::witness_snapshot(),
        None,
    )
    .unwrap();
    let request = AuthoredChangeSet {
        base: created.current.head.revision,
        preconditions: Vec::new(),
        budget: ChangeBudget::default(),
        changes: vec![
            AuthoredChange::CreateModule {
                symbol: "$module".to_owned(),
                name: Name::new("parameter-order").unwrap(),
            },
            function(
                "$function",
                vec![
                    parameter("$first", "first", AuthoredType::I64 {}),
                    parameter("$second", "second", AuthoredType::I64 {}),
                ],
            ),
            function(
                "$other",
                vec![parameter("$foreign", "foreign", AuthoredType::I64 {})],
            ),
            AuthoredChange::CreateInterface {
                symbol: "$interface".to_owned(),
                module: ModuleSelector::Symbol {
                    symbol: "$module".to_owned(),
                },
                name: Name::new("Ordered").unwrap(),
                visibility: DeclarationVisibility::Private,
                operations: vec![AuthoredOperation {
                    symbol: "$operation".to_owned(),
                    name: Name::new("emit").unwrap(),
                    parameters: vec![
                        parameter("$left", "left", AuthoredType::I64 {}),
                        parameter("$right", "right", AuthoredType::I64 {}),
                    ],
                    result: AuthoredType::I64 {},
                    idempotency: Idempotency::NonIdempotent,
                    external_visibility: ExternalVisibility::None,
                }],
            },
        ],
    };
    let prepared = created
        .repository
        .prepare_authored_change(&request, PublicationOptions::default())
        .unwrap();
    created.repository.publish(&prepared.publication).unwrap();
    let before = created
        .repository
        .view_current()
        .unwrap()
        .reconstruct_full_oracle()
        .unwrap()
        .value;
    Fixture {
        _temporary: temporary,
        repository: created.repository,
        owners: prepared.allocated,
        before,
    }
}

impl Fixture {
    fn selector(&self, symbol: &str) -> OwnerSelector {
        OwnerSelector::Exact {
            owner: self.owners[symbol],
        }
    }

    fn parent(&self) -> ParameterParentSelector {
        let OwnerKey::Declaration(declaration) = self.owners["$function"] else {
            panic!("function identity")
        };
        ParameterParentSelector::Declaration {
            declaration: DeclarationSelector::Id { declaration },
        }
    }

    fn request(&self, changes: Vec<AuthoredChange>) -> AuthoredChangeSet {
        AuthoredChangeSet {
            base: self.repository.view_current().unwrap().revision(),
            preconditions: Vec::new(),
            budget: ChangeBudget::default(),
            changes,
        }
    }

    fn order(&self, parameters: Vec<OwnerSelector>) -> AuthoredChange {
        AuthoredChange::SetParameterOrder {
            parent: self.parent(),
            parameters,
        }
    }
}

fn parameter_id(owner: OwnerKey) -> ParameterId {
    let OwnerKey::Parameter(parameter) = owner else {
        panic!("parameter identity")
    };
    parameter
}

#[test]
fn parameter_order_preserves_children_and_positions_new_symbols_before_retained_members() {
    let fixture = fixture();
    let request = fixture.request(vec![
        fixture.order(vec![
            fixture.selector("$first"),
            OwnerSelector::Symbol {
                symbol: "$layout".to_owned(),
            },
            fixture.selector("$second"),
        ]),
        AuthoredChange::SetParameterOrder {
            parent: ParameterParentSelector::Operation {
                operation: fixture.selector("$operation"),
            },
            parameters: vec![fixture.selector("$right"), fixture.selector("$left")],
        },
        AuthoredChange::AddParameter {
            parent: fixture.parent(),
            parameter: parameter("$layout", "layout", AuthoredType::Bytes {}),
        },
    ]);
    let prepared = fixture
        .repository
        .prepare_authored_change(&request, PublicationOptions::default())
        .unwrap();
    fixture.repository.publish(&prepared.publication).unwrap();
    let after = fixture
        .repository
        .view_current()
        .unwrap()
        .reconstruct_full_oracle()
        .unwrap()
        .value;
    assert_eq!(
        parameter_ids(&after.owners[&fixture.owners["$function"]]).unwrap(),
        &[
            parameter_id(fixture.owners["$first"]),
            parameter_id(prepared.allocated["$layout"]),
            parameter_id(fixture.owners["$second"]),
        ]
    );
    assert_eq!(
        parameter_ids(&after.owners[&fixture.owners["$operation"]]).unwrap(),
        &[
            parameter_id(fixture.owners["$right"]),
            parameter_id(fixture.owners["$left"]),
        ]
    );
    for symbol in ["$first", "$second", "$left", "$right"] {
        let owner = fixture.owners[symbol];
        assert_eq!(after.owners[&owner], fixture.before.owners[&owner]);
    }
    assert_eq!(after.retirements, fixture.before.retirements);
}

#[test]
fn parameter_order_uses_surviving_membership_after_explicit_deletions_and_allows_empty_inventory() {
    let fixture = fixture();
    let request = fixture.request(vec![
        fixture.order(vec![
            OwnerSelector::Symbol {
                symbol: "$replacement".to_owned(),
            },
            fixture.selector("$first"),
        ]),
        AuthoredChange::DeleteOwner {
            owner: fixture.selector("$second"),
            policy: AuthoredDeletePolicy::Reject,
        },
        AuthoredChange::AddParameter {
            parent: fixture.parent(),
            parameter: parameter("$replacement", "replacement", AuthoredType::I64 {}),
        },
    ]);
    let prepared = fixture
        .repository
        .prepare_authored_change(&request, PublicationOptions::default())
        .unwrap();
    fixture.repository.publish(&prepared.publication).unwrap();
    let after = fixture
        .repository
        .view_current()
        .unwrap()
        .reconstruct_full_oracle()
        .unwrap()
        .value;
    let replacement = prepared.allocated["$replacement"];
    assert_eq!(
        parameter_ids(&after.owners[&fixture.owners["$function"]]).unwrap(),
        &[
            parameter_id(replacement),
            parameter_id(fixture.owners["$first"]),
        ]
    );
    assert!(!after.owners.contains_key(&fixture.owners["$second"]));
    assert!(after.retirements.contains_key(&fixture.owners["$second"]));
    assert_eq!(
        after.owners[&fixture.owners["$first"]],
        fixture.before.owners[&fixture.owners["$first"]]
    );
    let empty = fixture.request(vec![
        fixture.order(Vec::new()),
        AuthoredChange::DeleteOwner {
            owner: fixture.selector("$first"),
            policy: AuthoredDeletePolicy::Reject,
        },
        AuthoredChange::DeleteOwner {
            owner: OwnerSelector::Exact { owner: replacement },
            policy: AuthoredDeletePolicy::Reject,
        },
    ]);
    let prepared = fixture
        .repository
        .prepare_authored_change(&empty, PublicationOptions::default())
        .unwrap();
    fixture.repository.publish(&prepared.publication).unwrap();
    let empty = fixture
        .repository
        .view_current()
        .unwrap()
        .reconstruct_full_oracle()
        .unwrap()
        .value;
    assert!(
        parameter_ids(&empty.owners[&fixture.owners["$function"]])
            .unwrap()
            .is_empty()
    );
}

#[test]
fn parameter_order_rejects_incomplete_duplicate_foreign_and_wrong_kind_inventories() {
    let fixture = fixture();
    let head = fixture.repository.current().unwrap().head;
    let normal = || {
        fixture.order(vec![
            fixture.selector("$second"),
            fixture.selector("$first"),
        ])
    };
    let cases = [
        (
            vec![fixture.order(vec![fixture.selector("$first")])],
            "change_parameter_order_membership",
        ),
        (
            vec![fixture.order(vec![fixture.selector("$first"), fixture.selector("$first")])],
            "change_parameter_order_duplicate",
        ),
        (
            vec![fixture.order(vec![
                fixture.selector("$first"),
                fixture.selector("$foreign"),
            ])],
            "change_parameter_order_foreign_parent",
        ),
        (
            vec![fixture.order(vec![
                fixture.selector("$first"),
                fixture.selector("$module"),
            ])],
            "change_parameter_order_kind",
        ),
        (
            vec![normal(), normal()],
            "change_parameter_order_duplicate_parent",
        ),
        (
            vec![AuthoredChange::SetParameterOrder {
                parent: ParameterParentSelector::Operation {
                    operation: fixture.selector("$module"),
                },
                parameters: Vec::new(),
            }],
            "change_parameter_order_parent",
        ),
        (
            vec![
                fixture.order(vec![fixture.selector("$second")]),
                AuthoredChange::DeleteOwner {
                    owner: fixture.selector("$second"),
                    policy: AuthoredDeletePolicy::Reject,
                },
            ],
            "change_authored_owner_deleted",
        ),
    ];
    for (changes, expected) in cases {
        let errors = fixture
            .repository
            .prepare_authored_change(&fixture.request(changes), PublicationOptions::default())
            .unwrap_err();
        assert!(
            errors.iter().any(|error| error.code == expected),
            "{expected}: {errors:?}"
        );
        assert_eq!(fixture.repository.current().unwrap().head, head);
    }
}

fn lowerer<'a>(
    view: &'a RepositoryView,
    budget: ChangeBudget,
) -> AuthoredLowerer<'a, RepositoryView, RepositoryView> {
    AuthoredLowerer::new(
        view,
        view,
        AuthoredLoweringInputs {
            intent_base: view.revision(),
            allocation_seed: [3; 32],
            deletion_change: ChangeDigest::of(b"parameter order admission"),
            allocated: BTreeMap::new(),
            definitions: BTreeMap::new(),
            allocations: Vec::new(),
            budget,
        },
    )
    .unwrap()
}

#[test]
fn parameter_order_precharges_exact_optional_work_and_never_fails_on_unmetered_saturation() {
    let fixture = fixture();
    let view = fixture.repository.view_current().unwrap();
    let parent = fixture.parent();
    let order = vec![fixture.selector("$second"), fixture.selector("$first")];
    for maximum in [None, Some(0), Some(4), Some(5), Some(u64::MAX)] {
        let mut budget = ChangeBudget::default();
        budget.impact.maximum_ownership_steps = maximum;
        let mut candidate = lowerer(&view, budget);
        let result = lower(&mut candidate, [(&parent, order.as_slice())]);
        if matches!(maximum, Some(0 | 4)) {
            assert_eq!(
                result.unwrap_err().code,
                "change_budget_impact_ownership_steps"
            );
            assert!(candidate.owner_edits.is_empty());
            assert_eq!(
                candidate.work.ownership_steps,
                if maximum == Some(0) { 0 } else { 1 }
            );
        } else {
            result.unwrap();
            assert_eq!(candidate.work.ownership_steps, 5);
            assert_eq!(
                candidate.owner_edits,
                BTreeSet::from([fixture.owners["$function"]])
            );
        }
    }
    for maximum in [None, Some(u64::MAX)] {
        let mut budget = ChangeBudget::default();
        budget.impact.maximum_ownership_steps = maximum;
        let mut candidate = lowerer(&view, budget);
        candidate.work.ownership_steps = u64::MAX;
        let result = lower(&mut candidate, [(&parent, order.as_slice())]);
        if maximum.is_none() {
            result.unwrap();
            assert_eq!(candidate.work.ownership_steps, u64::MAX);
            assert!(
                candidate
                    .work
                    .saturation
                    .contains(BudgetDimension::ImpactOwnershipSteps)
            );
        } else {
            assert_eq!(
                result.unwrap_err().code,
                "change_budget_impact_ownership_steps"
            );
            assert!(candidate.owners.is_empty());
            assert!(
                !candidate
                    .work
                    .saturation
                    .contains(BudgetDimension::ImpactOwnershipSteps)
            );
        }
    }
}
