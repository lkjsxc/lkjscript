//! Independent algebra controls after canonical scope validation. These literal
//! type/witness models are not an admission proof for an arbitrary finite prefix.
use crate::platform::kernel::associated_flow_reference::*;

fn definition(package: usize, declaration: usize) -> Definition {
    Definition {
        package,
        declaration,
    }
}
fn witness_only(binding: Term, actual: Term) -> Program {
    let c = definition(2, 0);
    let s = definition(1, 0);
    let f = definition(0, 0);
    Program {
        contracts: [(c, [0].into_iter().collect())].into_iter().collect(),
        schemes: [(
            s,
            Scheme {
                contract: c,
                parameters: 1,
                prerequisite_contracts: vec![],
                bindings: [(0, binding)].into_iter().collect(),
            },
        )]
        .into_iter()
        .collect(),
        functions: [(
            f,
            Function {
                parameters: 0,
                witnesses: vec![(7, c)],
                calls: vec![Call {
                    target: f,
                    arguments: vec![],
                    implementations: vec![Operand::Concrete {
                        scheme: s,
                        arguments: vec![actual],
                        prerequisites: vec![],
                    }],
                    kind: CallKind::Function,
                    method_guard: None,
                    untaken: true,
                    phantom: true,
                }],
            },
        )]
        .into_iter()
        .collect(),
        entries: vec![State {
            function: f,
            arguments: vec![],
            implementations: vec![Witness::Selected {
                scheme: s,
                arguments: vec![Term::Ground(0)],
                prerequisites: vec![],
            }],
        }],
    }
}

#[test]
fn view_family_growth_keeps_phantom_witness_actuals_and_constant_reset_distinct() {
    let s = definition(1, 0);
    let f = definition(0, 0);
    let c = definition(2, 0);
    let formal = Term::Parameter {
        scope: s,
        ordinal: 0,
    };
    let out = Term::Projection {
        scope: f,
        parameter: 7,
        member: Member { contract: c, id: 0 },
    };
    // A stored nominal family constructor contributes depth. Scope application
    // itself contributes none; it was independently admitted by the raw adapter.
    let grow = witness_only(Term::Constructor(9, vec![formal.clone()]), out.clone());
    let proof = reference_classify(&grow, Limits::default()).unwrap();
    assert!(!proof.finite);
    assert!(proof.positive_diagonal.is_some());
    // Exact bounded closure exhaustion is evidence about these initial states,
    // never substituted for the finite weighted positive-cycle certificate.
    assert!(matches!(
        grow.concrete_closure(Limits {
            concrete_states: 8,
            ..Limits::default()
        }),
        Err(Refusal::Capacity(_))
    ));
    let reset = witness_only(Term::Constructor(9, vec![Term::Ground(0)]), out);
    let proof = reference_classify(&reset, Limits::default()).unwrap();
    assert!(proof.finite);
    assert!(proof.positive_diagonal.is_none());
    assert_eq!(reset.concrete_closure(Limits::default()).unwrap().len(), 2);
    let reflected = witness_only(
        Term::Constructor(9, vec![formal]),
        Term::SelectedArgument {
            scope: f,
            path: WitnessPath {
                root: 7,
                prerequisites: vec![],
            },
            ordinal: 0,
        },
    );
    // Authored calls cannot reflect an arbitrary selected scheme argument.
    assert_eq!(
        reference_classify(&reflected, Limits::default()).unwrap_err(),
        Refusal::UnauthorizedDerivedPath
    );
    // Forwarding the complete witness is legal and adds no constructor weight,
    // even though its unused member binding contains a nominal constructor.
    let mut alias = reflected.clone();
    alias.functions.get_mut(&f).unwrap().calls[0].implementations = vec![Operand::Parameter {
        scope: f,
        path: WitnessPath {
            root: 7,
            prerequisites: vec![],
        },
    }];
    let proof = reference_classify(&alias, Limits::default()).unwrap();
    assert!(proof.finite);
    assert!(proof.positive_diagonal.is_none());
    assert_eq!(alias.concrete_closure(Limits::default()).unwrap().len(), 1);
    // Only the guarded selected-method entry transfers all declared scheme
    // arguments into ordinary parameters; its target then forwards that actual.
    let g = definition(0, 1);
    let mut guarded = alias.clone();
    guarded.functions.get_mut(&f).unwrap().calls = vec![Call {
        target: g,
        arguments: vec![Term::SelectedArgument {
            scope: f,
            path: WitnessPath {
                root: 7,
                prerequisites: vec![],
            },
            ordinal: 0,
        }],
        implementations: vec![],
        kind: CallKind::SelectedMethodEntry,
        method_guard: Some((7, s)),
        untaken: true,
        phantom: true,
    }];
    guarded.functions.insert(
        g,
        Function {
            parameters: 1,
            witnesses: vec![],
            calls: vec![Call {
                target: f,
                arguments: vec![],
                implementations: vec![Operand::Concrete {
                    scheme: s,
                    arguments: vec![Term::Parameter {
                        scope: g,
                        ordinal: 0,
                    }],
                    prerequisites: vec![],
                }],
                kind: CallKind::Function,
                method_guard: None,
                untaken: true,
                phantom: true,
            }],
        },
    );
    let proof = reference_classify(&guarded, Limits::default()).unwrap();
    assert!(proof.finite);
    assert!(proof.positive_diagonal.is_none());
    assert_eq!(
        guarded.concrete_closure(Limits::default()).unwrap().len(),
        2
    );
    let mut unguarded = guarded.clone();
    let entry = &mut unguarded.functions.get_mut(&f).unwrap().calls[0];
    entry.kind = CallKind::Function;
    entry.method_guard = None;
    assert_eq!(
        reference_classify(&unguarded, Limits::default()).unwrap_err(),
        Refusal::UnauthorizedDerivedPath
    );
}
