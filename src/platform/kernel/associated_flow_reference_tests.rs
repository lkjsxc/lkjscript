//! Literal independent model fixtures, including exhaustive 1,688-schema classification.
#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]
use super::associated_flow_reference::*;
use std::collections::{BTreeMap, BTreeSet};

fn function_id(n: usize) -> Definition {
    Definition {
        package: 0,
        declaration: n,
    }
}
fn scheme_id(n: usize) -> Definition {
    Definition {
        package: 1,
        declaration: n,
    }
}
fn contract_id(n: usize) -> Definition {
    Definition {
        package: 2,
        declaration: n,
    }
}
fn parameter(scope: Definition, ordinal: usize) -> Term {
    Term::Parameter { scope, ordinal }
}
fn boxed(term: Term) -> Term {
    Term::Constructor(0, vec![term])
}
fn pair(a: Term, b: Term) -> Term {
    Term::Constructor(1, vec![a, b])
}
fn projection(scope: Definition, root: usize, contract: Definition, member: usize) -> Term {
    Term::Projection {
        scope,
        parameter: root,
        member: Member {
            contract,
            id: member,
        },
    }
}
fn out(scope: Definition, root: usize) -> Term {
    projection(scope, root, contract_id(0), 0)
}
fn path(root: usize) -> WitnessPath {
    WitnessPath {
        root,
        prerequisites: vec![],
    }
}
fn forwarded(scope: Definition, root: usize) -> Operand {
    Operand::Parameter {
        scope,
        path: path(root),
    }
}
fn selected(id: Definition, arguments: Vec<Term>) -> Witness {
    Witness::Selected {
        scheme: id,
        arguments,
        prerequisites: vec![],
    }
}
fn applied(id: Definition, arguments: Vec<Term>) -> Operand {
    Operand::Concrete {
        scheme: id,
        arguments,
        prerequisites: vec![],
    }
}
fn application(target: Definition, arguments: Vec<Term>, implementations: Vec<Operand>) -> Call {
    Call {
        target,
        arguments,
        implementations,
        kind: CallKind::Function,
        method_guard: None,
        untaken: false,
        phantom: false,
    }
}
fn unary(binding: Term, actual: Term) -> Program {
    let mut program = Program::default();
    program
        .contracts
        .insert(contract_id(0), [0].into_iter().collect());
    program.schemes.insert(
        scheme_id(0),
        Scheme {
            contract: contract_id(0),
            parameters: 1,
            prerequisite_contracts: vec![],
            bindings: [(0, binding)].into_iter().collect(),
        },
    );
    program.functions.insert(
        function_id(0),
        Function {
            parameters: 0,
            witnesses: vec![(7, contract_id(0))],
            calls: vec![application(
                function_id(0),
                vec![],
                vec![applied(scheme_id(0), vec![actual])],
            )],
        },
    );
    program.entries.push(State {
        function: function_id(0),
        arguments: vec![],
        implementations: vec![selected(scheme_id(0), vec![Term::Ground(0)])],
    });
    program
}
fn mutual(binding: Term, projection_actual: Term, relay_actual: Term) -> Program {
    let mut program = unary(binding, Term::Ground(0));
    program.functions.insert(
        function_id(0),
        Function {
            parameters: 0,
            witnesses: vec![(7, contract_id(0))],
            calls: vec![application(
                function_id(1),
                vec![projection_actual],
                vec![forwarded(function_id(0), 7)],
            )],
        },
    );
    program.functions.insert(
        function_id(1),
        Function {
            parameters: 1,
            witnesses: vec![(7, contract_id(0))],
            calls: vec![application(
                function_id(0),
                vec![],
                vec![applied(scheme_id(0), vec![relay_actual])],
            )],
        },
    );
    program
}
fn two_parameter(left: Term, right: Term, first: Term, second: Term) -> Program {
    let mut program = Program::default();
    program
        .contracts
        .insert(contract_id(0), [0, 1].into_iter().collect());
    program.schemes.insert(
        scheme_id(0),
        Scheme {
            contract: contract_id(0),
            parameters: 2,
            prerequisite_contracts: vec![],
            bindings: [(0, left), (1, right)].into_iter().collect(),
        },
    );
    program.functions.insert(
        function_id(0),
        Function {
            parameters: 0,
            witnesses: vec![(7, contract_id(0))],
            calls: vec![application(
                function_id(0),
                vec![],
                vec![applied(scheme_id(0), vec![first, second])],
            )],
        },
    );
    program.entries.push(State {
        function: function_id(0),
        arguments: vec![],
        implementations: vec![selected(
            scheme_id(0),
            vec![Term::Ground(0), Term::Ground(1)],
        )],
    });
    program
}
fn alias_permutation() -> Program {
    two_parameter(
        parameter(scheme_id(0), 0),
        parameter(scheme_id(0), 1),
        projection(function_id(0), 7, contract_id(0), 1),
        out(function_id(0), 7),
    )
}
fn prerequisite_permutation() -> Program {
    let mut program = alias_permutation();
    program
        .contracts
        .insert(contract_id(1), [0].into_iter().collect());
    program.schemes.insert(
        scheme_id(1),
        Scheme {
            contract: contract_id(1),
            parameters: 0,
            prerequisite_contracts: vec![contract_id(0)],
            bindings: [(0, Term::Ground(0))].into_iter().collect(),
        },
    );
    let mut entry = application(
        function_id(1),
        vec![],
        vec![Operand::Parameter {
            scope: function_id(0),
            path: WitnessPath {
                root: 7,
                prerequisites: vec![0],
            },
        }],
    );
    entry.kind = CallKind::SelectedMethodEntry;
    entry.method_guard = Some((7, scheme_id(1)));
    program.functions.insert(
        function_id(0),
        Function {
            parameters: 0,
            witnesses: vec![(7, contract_id(1))],
            calls: vec![entry],
        },
    );
    program.functions.insert(
        function_id(1),
        Function {
            parameters: 0,
            witnesses: vec![(11, contract_id(0))],
            calls: vec![application(
                function_id(0),
                vec![],
                vec![Operand::Concrete {
                    scheme: scheme_id(1),
                    arguments: vec![],
                    prerequisites: vec![applied(
                        scheme_id(0),
                        vec![
                            projection(function_id(1), 11, contract_id(0), 1),
                            out(function_id(1), 11),
                        ],
                    )],
                }],
            )],
        },
    );
    program.entries = vec![State {
        function: function_id(0),
        arguments: vec![],
        implementations: vec![Witness::Selected {
            scheme: scheme_id(1),
            arguments: vec![],
            prerequisites: vec![selected(
                scheme_id(0),
                vec![Term::Ground(0), Term::Ground(1)],
            )],
        }],
    }];
    program
}
fn check_finite(program: &Program, maximum_states: usize) {
    let proof = reference_classify(program, Limits::default()).unwrap();
    assert!(
        proof.finite,
        "unexpected positive diagonal {:?}",
        proof.positive_diagonal
    );
    assert_eq!(proof.positive_diagonal, None);
    let closure = program
        .concrete_closure(Limits {
            concrete_states: maximum_states,
            ..Limits::default()
        })
        .unwrap();
    assert!(!closure.is_empty());
    assert!(closure.len() <= maximum_states);
}

#[test]
fn associated_reference_accepts_alias_permutation_and_constant_resets() {
    let f = function_id(0);
    let alias = unary(parameter(scheme_id(0), 0), out(f, 7));
    check_finite(&alias, 1);
    check_finite(&alias_permutation(), 2);
    check_finite(&prerequisite_permutation(), 4);
    check_finite(&unary(Term::Ground(0), boxed(out(f, 7))), 2);
    check_finite(
        &unary(boxed(parameter(scheme_id(0), 0)), Term::Ground(0)),
        1,
    );
    let mut fixed = unary(boxed(parameter(scheme_id(0), 0)), Term::Ground(0));
    fixed.functions.get_mut(&f).unwrap().parameters = 1;
    fixed.functions.get_mut(&f).unwrap().calls =
        vec![application(f, vec![out(f, 7)], vec![forwarded(f, 7)])];
    fixed.entries[0].arguments = vec![Term::Ground(1)];
    check_finite(&fixed, 2);
}

#[test]
fn associated_reference_rejects_witness_mutual_and_ordinary_growth() {
    let f = function_id(0);
    let g = function_id(1);
    let s = scheme_id(0);
    let direct = unary(boxed(parameter(s, 0)), out(f, 7));
    let mixed = mutual(boxed(parameter(s, 0)), out(f, 7), parameter(g, 0));
    let outer = unary(parameter(s, 0), boxed(out(f, 7)));
    let mut ordinary = Program::default();
    ordinary.functions.insert(
        f,
        Function {
            parameters: 1,
            witnesses: vec![],
            calls: vec![application(f, vec![boxed(parameter(f, 0))], vec![])],
        },
    );
    ordinary.entries.push(State {
        function: f,
        arguments: vec![Term::Ground(0)],
        implementations: vec![],
    });
    let mut self_unchanged = mixed.clone();
    self_unchanged.schemes.get_mut(&s).unwrap().parameters = 2;
    self_unchanged
        .schemes
        .get_mut(&s)
        .unwrap()
        .bindings
        .insert(0, boxed(parameter(s, 1)));
    self_unchanged.functions.get_mut(&f).unwrap().parameters = 2;
    self_unchanged.functions.get_mut(&f).unwrap().calls = vec![application(
        g,
        vec![parameter(f, 0), out(f, 7)],
        vec![forwarded(f, 7)],
    )];
    self_unchanged.functions.get_mut(&g).unwrap().parameters = 2;
    self_unchanged.functions.get_mut(&g).unwrap().calls = vec![application(
        f,
        vec![parameter(g, 0), parameter(g, 1)],
        vec![applied(s, vec![parameter(g, 0), parameter(g, 1)])],
    )];
    self_unchanged.entries[0].arguments = vec![Term::Ground(2), Term::Ground(0)];
    self_unchanged.entries[0].implementations =
        vec![selected(s, vec![Term::Ground(2), Term::Ground(0)])];
    // Selected method entry transfers scheme arguments into ordinary parameters
    // even without any projection in that particular recursive application.
    let mut method = direct.clone();
    let mut entry = application(
        g,
        vec![Term::SelectedArgument {
            scope: f,
            path: path(7),
            ordinal: 0,
        }],
        vec![],
    );
    entry.kind = CallKind::SelectedMethodEntry;
    entry.method_guard = Some((7, s));
    method.functions.get_mut(&f).unwrap().calls = vec![entry];
    method.functions.insert(
        g,
        Function {
            parameters: 1,
            witnesses: vec![],
            calls: vec![application(
                f,
                vec![],
                vec![applied(s, vec![boxed(parameter(g, 0))])],
            )],
        },
    );
    for program in [direct, mixed, outer, ordinary, self_unchanged, method] {
        let proof = reference_classify(&program, Limits::default()).unwrap();
        assert!(!proof.finite);
        assert!(proof.positive_diagonal.is_some());
        assert!(proof.edges.iter().any(|(_, _, positive)| *positive));
    }
}

#[test]
fn associated_reference_keeps_exact_roots_paths_and_unknown_anchors() {
    let f = function_id(0);
    let s = scheme_id(0);
    let contract = contract_id(0);
    let mut roots = unary(boxed(parameter(s, 0)), Term::Ground(0));
    roots.functions.get_mut(&f).unwrap().witnesses = vec![(7, contract), (11, contract)];
    roots.functions.get_mut(&f).unwrap().calls = vec![application(
        f,
        vec![],
        vec![
            applied(s, vec![out(f, 11)]),
            applied(s, vec![Term::Ground(0)]),
        ],
    )];
    roots.entries[0].implementations = vec![
        selected(s, vec![Term::Ground(1)]),
        selected(s, vec![Term::Ground(2)]),
    ];
    check_finite(&roots, 3);
    let proof = reference_classify(&roots, Limits::default()).unwrap();
    let mut roots_by_context: BTreeMap<usize, BTreeSet<usize>> = BTreeMap::new();
    for slot in &proof.slots {
        if let Slot::WitnessArgument { context, path, .. } = slot {
            roots_by_context
                .entry(*context)
                .or_default()
                .insert(path.root);
        }
    }
    assert!(
        roots_by_context
            .values()
            .any(|roots| roots == &[7, 11].into_iter().collect())
    );
    let nested = prerequisite_permutation();
    let proof = reference_classify(&nested, Limits::default()).unwrap();
    assert!(proof.slots.iter().any(
        |slot| matches!(slot, Slot::WitnessArgument { path, .. } if path.prerequisites == [0])
    ));
    let a = Witness::Unknown {
        identity: UnknownIdentity::Input {
            scope: f,
            parameter: 7,
            instance: 0,
        },
        contract,
    };
    let b = Witness::Unknown {
        identity: UnknownIdentity::Input {
            scope: Definition { package: 9, ..f },
            parameter: 7,
            instance: 0,
        },
        contract,
    };
    assert_ne!(
        roots.project(&a, Member { contract, id: 0 }).unwrap(),
        roots.project(&b, Member { contract, id: 0 }).unwrap()
    );
    let mut equal = roots.clone();
    equal
        .schemes
        .get_mut(&s)
        .unwrap()
        .bindings
        .insert(0, Term::Ground(0));
    equal
        .schemes
        .insert(scheme_id(1), equal.schemes[&s].clone());
    let a = selected(s, vec![Term::Ground(2)]);
    let b = selected(scheme_id(1), vec![Term::Ground(2)]);
    assert_eq!(
        equal.project(&a, Member { contract, id: 0 }).unwrap(),
        equal.project(&b, Member { contract, id: 0 }).unwrap()
    );
    assert_ne!(
        a, b,
        "equal normalized types do not identify witness authority"
    );
}

#[test]
fn associated_reference_substitution_is_simultaneous_and_capture_free() {
    let caller = function_id(0);
    let callee = function_id(1);
    let s = scheme_id(0);
    let contract = contract_id(0);
    let mut program = unary(boxed(parameter(s, 0)), Term::Ground(0));
    program.functions.insert(
        caller,
        Function {
            parameters: 2,
            witnesses: vec![(7, contract), (11, contract)],
            calls: vec![],
        },
    );
    program.functions.insert(
        callee,
        Function {
            parameters: 2,
            witnesses: vec![(7, contract)],
            calls: vec![],
        },
    );
    let swapped = program
        .rebind(
            &pair(parameter(callee, 0), parameter(callee, 1)),
            callee,
            caller,
            &[parameter(caller, 1), parameter(caller, 0)],
            &[forwarded(caller, 11)],
        )
        .unwrap();
    assert_eq!(swapped, pair(parameter(caller, 1), parameter(caller, 0)));
    assert_eq!(
        program
            .rebind(
                &out(callee, 7),
                callee,
                caller,
                &[parameter(caller, 0), parameter(caller, 1)],
                &[forwarded(caller, 11)]
            )
            .unwrap(),
        out(caller, 11)
    );
    assert_eq!(
        program
            .rebind(
                &out(callee, 7),
                callee,
                caller,
                &[parameter(caller, 0), parameter(caller, 1)],
                &[applied(s, vec![out(caller, 7)])]
            )
            .unwrap(),
        boxed(out(caller, 7))
    );
    assert_eq!(
        program.rebind(
            &out(function_id(9), 7),
            callee,
            caller,
            &[parameter(caller, 0), parameter(caller, 1)],
            &[forwarded(caller, 11)]
        ),
        Err(Refusal::Scope)
    );
    let zero = unary(boxed(parameter(s, 0)), out(caller, 7));
    let env = Environment {
        scope: caller,
        declaration: zero.function(caller).unwrap(),
        arguments: &[],
        implementations: &zero.entries[0].implementations,
    };
    assert_eq!(
        zero.evaluate_type(&out(caller, 7), &env).unwrap(),
        boxed(Term::Ground(0))
    );
}

#[test]
fn associated_reference_checks_unused_meaning_and_capacity_separately() {
    let f = function_id(0);
    let s = scheme_id(0);
    let mut phantom = unary(boxed(parameter(s, 0)), out(f, 7));
    let call = &mut phantom.functions.get_mut(&f).unwrap().calls[0];
    call.kind = CallKind::FunctionValue;
    call.untaken = true;
    call.phantom = true;
    let mut mapping = application(f, vec![], vec![forwarded(f, 7)]);
    mapping.kind = CallKind::UnusedMapping;
    phantom.functions.get_mut(&f).unwrap().calls.push(mapping);
    assert!(
        phantom.functions[&f]
            .calls
            .iter()
            .any(|c| c.untaken && c.phantom)
    );
    let proof = reference_classify(&phantom, Limits::default()).unwrap();
    assert!(!proof.finite);
    let exact = Limits {
        contexts: Some(proof.contexts as u64),
        slots: Some(proof.slots.len() as u64),
        ..Limits::default()
    };
    assert!(!reference_classify(&phantom, exact).unwrap().finite);
    assert!(matches!(
        reference_classify(
            &phantom,
            Limits {
                slots: Some(proof.slots.len() as u64 - 1),
                ..exact
            }
        ),
        Err(Refusal::Capacity("reference slots"))
    ));
    assert!(matches!(
        reference_classify(
            &phantom,
            Limits {
                contexts: Some(proof.contexts as u64 - 1),
                ..exact
            }
        ),
        Err(Refusal::Capacity("reference contexts"))
    ));
    assert_eq!(
        phantom.concrete_closure(Limits {
            concrete_states: 3,
            ..Limits::default()
        }),
        Err(Refusal::Capacity("concrete states"))
    );
    assert!(
        !reference_classify(&phantom, Limits::default())
            .unwrap()
            .finite,
        "fresh analysis recovery"
    );
    let mut foreign = phantom.clone();
    let Operand::Concrete { arguments, .. } =
        &mut foreign.functions.get_mut(&f).unwrap().calls[0].implementations[0]
    else {
        panic!("fixture concrete");
    };
    arguments[0] = out(function_id(9), 7);
    assert_eq!(foreign.admit(), Err(Refusal::Scope));
    let mut unused = phantom.clone();
    unused.contracts.get_mut(&contract_id(0)).unwrap().insert(1);
    unused
        .schemes
        .get_mut(&s)
        .unwrap()
        .bindings
        .insert(1, out(f, 7));
    assert_eq!(unused.admit(), Err(Refusal::ProjectionInBinding));
    let mut wrong_contract = phantom.clone();
    wrong_contract
        .contracts
        .insert(contract_id(1), [0].into_iter().collect());
    let Operand::Concrete { arguments, .. } =
        &mut wrong_contract.functions.get_mut(&f).unwrap().calls[0].implementations[0]
    else {
        panic!("fixture concrete");
    };
    arguments[0] = projection(f, 7, contract_id(1), 0);
    assert_eq!(wrong_contract.admit(), Err(Refusal::ForeignContract));
    let mut wrap = phantom;
    wrap.schemes.insert(
        scheme_id(1),
        Scheme {
            contract: contract_id(0),
            parameters: 0,
            prerequisite_contracts: vec![contract_id(0)],
            bindings: [(0, Term::Ground(0))].into_iter().collect(),
        },
    );
    wrap.functions.get_mut(&f).unwrap().calls = vec![application(
        f,
        vec![],
        vec![Operand::Concrete {
            scheme: scheme_id(1),
            arguments: vec![],
            prerequisites: vec![forwarded(f, 7)],
        }],
    )];
    let mut foreign_prerequisite = wrap.clone();
    foreign_prerequisite
        .contracts
        .insert(contract_id(1), [0].into_iter().collect());
    foreign_prerequisite
        .schemes
        .get_mut(&scheme_id(1))
        .unwrap()
        .prerequisite_contracts[0] = contract_id(1);
    assert_eq!(foreign_prerequisite.admit(), Err(Refusal::ForeignContract));
    assert_eq!(
        wrap.admit(),
        Err(Refusal::UnsupportedRecursiveWitnessConstruction)
    );
}

#[test]
fn associated_reference_small_grammars_exhaust_every_classified_finite_instance() {
    // Exact bounded grammar from the independently executed ignored prototype.
    // Exhausting these initial-state closures does not prove general soundness;
    // growing classifications come from weighted diagonals, never a finite prefix.
    let f = function_id(0);
    let g = function_id(1);
    let s = scheme_id(0);
    let bindings = [
        parameter(s, 0),
        Term::Ground(0),
        Term::Ground(1),
        boxed(parameter(s, 0)),
        boxed(Term::Ground(0)),
        pair(parameter(s, 0), parameter(s, 0)),
        pair(parameter(s, 0), Term::Ground(0)),
    ];
    let actuals = |owner| {
        [
            out(owner, 7),
            boxed(out(owner, 7)),
            Term::Ground(0),
            Term::Ground(1),
            boxed(Term::Ground(0)),
            pair(out(owner, 7), Term::Ground(0)),
            pair(out(owner, 7), out(owner, 7)),
        ]
    };
    let relays = [
        parameter(g, 0),
        Term::Ground(0),
        Term::Ground(1),
        boxed(parameter(g, 0)),
        boxed(Term::Ground(0)),
        pair(parameter(g, 0), parameter(g, 0)),
        pair(parameter(g, 0), Term::Ground(0)),
    ];
    let mut counts = (0, 0, 0);
    let mut check = |mut program: Program| {
        // The unary grammar starts from a third ground symbol, Cell;
        // constants 0/1 name A/B, matching the retained prototype's alphabet.
        let unary_grammar = program.functions.len() == 1;
        if let Witness::Selected { arguments, .. } = &mut program.entries[0].implementations[0]
            && unary_grammar
            && arguments.len() == 1
        {
            arguments[0] = Term::Ground(2);
        }
        counts.0 += 1;
        let proof = reference_classify(&program, Limits::default()).unwrap();
        if proof.finite {
            counts.1 += 1;
            assert!(
                !program
                    .concrete_closure(Limits::default())
                    .unwrap()
                    .is_empty()
            );
        } else {
            counts.2 += 1;
            assert!(proof.positive_diagonal.is_some());
        }
    };
    for binding in &bindings {
        for actual in actuals(f) {
            check(unary(binding.clone(), actual));
        }
    }
    for binding in &bindings {
        for actual in actuals(f) {
            for relay in &relays {
                check(mutual(binding.clone(), actual.clone(), relay.clone()));
            }
        }
    }
    let two_bindings = [
        parameter(s, 0),
        parameter(s, 1),
        Term::Ground(0),
        boxed(parameter(s, 0)),
        boxed(parameter(s, 1)),
        pair(parameter(s, 0), parameter(s, 1)),
    ];
    let two_actuals = [
        out(f, 7),
        projection(f, 7, contract_id(0), 1),
        Term::Ground(0),
        Term::Ground(1),
        boxed(out(f, 7)),
        boxed(projection(f, 7, contract_id(0), 1)),
    ];
    for left in &two_bindings {
        for right in &two_bindings {
            for first in &two_actuals {
                for second in &two_actuals {
                    check(two_parameter(
                        left.clone(),
                        right.clone(),
                        first.clone(),
                        second.clone(),
                    ));
                }
            }
        }
    }
    assert_eq!(counts, (1688, 958, 730));
}

#[test]
fn associated_reference_many_zero_slots_do_not_spend_quadratic_storage() {
    let function = function_id(0);
    let contract = contract_id(0);
    let parameters = 2_200;
    // One associated member sits beside a large ordinary alias relation. The
    // previous per-slot visited vectors alone exceeded 64 MiB at this width.
    for (first, second, finite, positive_edges) in [
        (parameter(function, 0), parameter(function, 1), true, 0),
        (boxed(Term::Ground(0)), parameter(function, 1), true, 0),
        (Term::Ground(0), boxed(parameter(function, 0)), true, 1),
        (
            parameter(function, 1),
            boxed(parameter(function, 0)),
            false,
            1,
        ),
    ] {
        let mut arguments: Vec<_> = (0..parameters)
            .map(|ordinal| parameter(function, ordinal))
            .collect();
        arguments[0] = first;
        arguments[1] = second;
        let program = Program {
            contracts: BTreeMap::from([(contract, BTreeSet::from([0]))]),
            functions: BTreeMap::from([(
                function,
                Function {
                    parameters,
                    witnesses: vec![(7, contract)],
                    calls: vec![application(
                        function,
                        arguments,
                        vec![forwarded(function, 7)],
                    )],
                },
            )]),
            ..Program::default()
        };
        let checkpoint = || Ok(());
        let budget = Budget::new(&checkpoint, Some(10_000_000), Some(64 * 1_048_576));
        let proof = reference_classify_with_budget(
            &program,
            Limits {
                contexts: Some(4),
                slots: Some(parameters as u64 + 1),
                ..Limits::default()
            },
            &budget,
        )
        .expect("many unrelated aliases must fit the unchanged reference allowance");
        assert_eq!(proof.slots.len(), parameters + 1);
        assert_eq!(
            proof
                .edges
                .iter()
                .filter(|(_, _, positive)| *positive)
                .count(),
            positive_edges
        );
        assert_eq!(proof.finite, finite);
        assert_eq!(proof.positive_diagonal.is_none(), finite);
        assert!(!budget.bytes().saturated);
        assert!(budget.bytes().consumed < 64 * 1_048_576);
    }
}

#[test]
fn associated_reference_default_context_and_slot_admission_crosses_former_limits() {
    let functions = (0..257)
        .map(|ordinal| {
            (
                function_id(ordinal),
                Function {
                    parameters: 2,
                    witnesses: Vec::new(),
                    calls: Vec::new(),
                },
            )
        })
        .collect();
    let program = Program {
        functions,
        ..Program::default()
    };
    let proof = reference_classify(&program, Limits::default()).unwrap();
    assert!(proof.finite && proof.positive_diagonal.is_none());
    assert_eq!(proof.contexts, 257);
    assert_eq!(proof.slots.len(), 514);
    assert!(proof.edges.is_empty());
    for (contexts, slots) in [(Some(257), Some(514)), (Some(u64::MAX), Some(u64::MAX))] {
        let retry = reference_classify(
            &program,
            Limits {
                contexts,
                slots,
                ..Limits::default()
            },
        )
        .unwrap();
        assert_eq!(
            (retry.contexts, retry.slots, retry.edges),
            (proof.contexts, proof.slots.clone(), proof.edges.clone())
        );
    }
    for (contexts, slots, kind) in [
        (Some(0), None, "reference contexts"),
        (Some(256), None, "reference contexts"),
        (None, Some(0), "reference slots"),
        (None, Some(513), "reference slots"),
    ] {
        assert_eq!(
            reference_classify(
                &program,
                Limits {
                    contexts,
                    slots,
                    ..Limits::default()
                }
            )
            .unwrap_err(),
            Refusal::Capacity(kind)
        );
    }
}

#[test]
fn associated_reference_unbounded_budget_crosses_former_work_and_modeled_storage() {
    let checkpoint = || Ok(());
    let budget = Budget::new(&checkpoint, None, None);
    for _ in 0..=super::contract::MAXIMUM_VALIDATION_WORK {
        budget.tick().unwrap();
    }
    let modeled = 64 * 1_048_576 + 1;
    budget.reserve_bytes(modeled).unwrap();
    assert_eq!(
        budget.consumed(),
        crate::platform::admission::Observation::exact(
            super::contract::MAXIMUM_VALIDATION_WORK as u64 + 2
        )
    );
    assert_eq!(
        budget.bytes(),
        crate::platform::admission::Observation::from_usize(modeled)
    );
    for limit in [0, modeled as u64 - 1] {
        let bounded = Budget::new(&checkpoint, None, Some(limit));
        assert_eq!(
            bounded.reserve_bytes(modeled),
            Err(Refusal::Capacity("reference storage"))
        );
        assert_eq!(
            bounded.bytes(),
            crate::platform::admission::Observation::default()
        );
    }
    for limit in [modeled as u64, u64::MAX] {
        let bounded = Budget::new(&checkpoint, Some(1), Some(limit));
        bounded.reserve_bytes(modeled).unwrap();
        assert_eq!(bounded.bytes(), budget.bytes());
        assert_eq!(bounded.tick(), Err(Refusal::Capacity("reference work")));
    }
}
