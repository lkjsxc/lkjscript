//! Focused composed-flow fixtures use independent explicit expected cycles. They do not invoke
//! inference, production substitution, compiler lowering or prepared closure machinery.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;
use crate::platform::kernel::*;
use crate::platform::semantic_id::{DeclarationId, ImplementationParameterId, ModuleId};
use std::cell::Cell;

const SEED: &[u8] = b"composed-finite-independent-source";
fn package(n: u64) -> PackageId {
    PackageId::migrate(SEED, n)
}
fn reference(n: u64) -> DeclarationReference {
    DeclarationReference {
        package: package(if n == 0 { 0 } else { 1 }),
        declaration: DeclarationId::migrate(SEED, n),
    }
}
fn parameter(n: u64, ordinal: u64) -> TypeParameterId {
    TypeParameterId::migrate(SEED, 100 + n * 16 + ordinal)
}
fn method(n: u64) -> MethodId {
    MethodId::migrate(SEED, n)
}
fn witness(n: u64) -> ImplementationParameterId {
    ImplementationParameterId::migrate(SEED, n)
}

#[derive(Default)]
struct Read {
    owners: BTreeMap<(PackageId, OwnerKey), OwnerRecord>,
    types: TypeObjectInterner,
    checkpoints: Cell<usize>,
    cancel_after: Cell<Option<usize>>,
}
impl CallableClosureRead for Read {
    fn visit_callable_owners(
        &self,
        visitor: &mut dyn FnMut(DeclarationReference) -> Result<(), Diagnostic>,
    ) -> Result<(), Diagnostic> {
        for declaration in self
            .owners
            .keys()
            .filter_map(|(package, owner)| match owner {
                OwnerKey::Declaration(declaration) => Some(DeclarationReference {
                    package: *package,
                    declaration: *declaration,
                }),
                _ => None,
            })
        {
            visitor(declaration)?;
        }
        Ok(())
    }
    fn owner(
        &self,
        package: PackageId,
        owner: OwnerKey,
    ) -> Result<Option<OwnerRecord>, Diagnostic> {
        Ok(self.owners.get(&(package, owner)).cloned())
    }
    fn type_object(
        &self,
        _: PackageId,
        ty: TypeObjectDigest,
    ) -> Result<Option<TypeObject>, Diagnostic> {
        Ok(self.types.get(ty).cloned())
    }
    fn validation_checkpoint(&self) -> Result<(), Diagnostic> {
        let next = self.checkpoints.get() + 1;
        self.checkpoints.set(next);
        if self.cancel_after.get().is_some_and(|n| next >= n) {
            return Err(Diagnostic::new(
                DiagnosticClass::Cancelled,
                "test_cancelled",
                "owning source operation was cancelled",
            ));
        }
        Ok(())
    }
}
impl Read {
    fn ty(&mut self, form: TypeForm) -> TypeObjectDigest {
        self.types.intern(form).unwrap()
    }
    fn p(&mut self, owner: u64, ordinal: u64) -> TypeObjectDigest {
        self.ty(TypeForm::TypeParameter {
            parameter: parameter(owner, ordinal),
        })
    }
    fn expression(&mut self, package: PackageId, operation: ExpressionOperation) -> ExpressionId {
        let id = ExpressionId::migrate(SEED, self.owners.len() as u64);
        self.owners.insert(
            (package, OwnerKey::Expression(id)),
            OwnerRecord::Expression(ExpressionRecord::new(id, operation).unwrap()),
        );
        id
    }
    fn declaration(&mut self, n: u64, payload: DeclarationPayload) {
        let r = reference(n);
        let owner = OwnerKey::Declaration(r.declaration);
        let kind = match &payload {
            DeclarationPayload::Function(_) => OwnerKind::PureFunction,
            DeclarationPayload::OwnedImplementation(_) => OwnerKind::OwnedImplementation,
            _ => panic!("fixture declaration kind"),
        };
        self.owners.insert(
            (r.package, owner),
            OwnerRecord::Declaration(DeclarationRecord {
                header: OwnerHeader::new(owner, kind),
                module: ModuleId::migrate(SEED, n),
                name: Name::new(format!("f{n}")).unwrap(),
                visibility: DeclarationVisibility::Public,
                payload,
            }),
        );
    }
    fn function(&mut self, n: u64, arity: usize, implementations: usize, body: ExpressionId) {
        let result = self.ty(TypeForm::I64);
        let mut ps = Vec::new();
        for i in 0..implementations {
            ps.push(ImplementationParameter {
                id: witness(i as u64),
                name: Name::new(format!("w{i}")).unwrap(),
                contract: reference(900),
                self_type: result,
                type_arguments: Vec::new(),
            });
        }
        self.declaration(
            n,
            DeclarationPayload::Function(FunctionDeclaration {
                type_parameters: (0..arity).map(|i| parameter(n, i as u64)).collect(),
                implementation_parameters: ps,
                effect_parameters: Vec::new(),
                requirement_parameters: Vec::new(),
                parameters: Vec::new(),
                result,
                result_borrow: None,
                effect: FunctionEffect::Pure,
                body,
            }),
        );
    }
    fn implementation(
        &mut self,
        n: u64,
        arity: usize,
        methods: Vec<(u64, u64, Vec<TypeObjectDigest>)>,
    ) {
        let leaf = self.ty(TypeForm::OwnedI64Cell);
        self.declaration(
            n,
            DeclarationPayload::OwnedImplementation(OwnedImplementation {
                type_parameters: (0..arity).map(|i| parameter(n, i as u64)).collect(),
                implementation_parameters: Vec::new(),
                contract: reference(900),
                self_type: leaf,
                type_arguments: Vec::new(),
                methods: methods
                    .into_iter()
                    .map(|(m, f, type_arguments)| OwnedMethodImplementation {
                        method: method(m),
                        function: reference(f),
                        type_arguments,
                        implementations: Vec::new(),
                    })
                    .collect(),
            }),
        );
    }
    fn admit(&self) -> Result<CallableFlowWork, Diagnostic> {
        validate_callable_closure(self, &mut 0, 1_000_000)
    }
    fn prerequisites(&mut self, n: u64, count: usize) {
        let leaf = self.ty(TypeForm::OwnedI64Cell);
        let OwnerRecord::Declaration(record) = self
            .owners
            .get_mut(&(
                reference(n).package,
                OwnerKey::Declaration(reference(n).declaration),
            ))
            .unwrap()
        else {
            panic!("scheme")
        };
        let DeclarationPayload::OwnedImplementation(scheme) = &mut record.payload else {
            panic!("scheme")
        };
        scheme.implementation_parameters = (0..count)
            .map(|ordinal| ImplementationParameter {
                id: witness(ordinal as u64),
                name: Name::new(format!("p{ordinal}")).unwrap(),
                contract: reference(900),
                self_type: leaf,
                type_arguments: Vec::new(),
            })
            .collect();
    }
    fn mapping_prerequisites(&mut self, n: u64, operands: Vec<ImplementationOperand>) {
        let OwnerRecord::Declaration(record) = self
            .owners
            .get_mut(&(
                reference(n).package,
                OwnerKey::Declaration(reference(n).declaration),
            ))
            .unwrap()
        else {
            panic!("scheme")
        };
        let DeclarationPayload::OwnedImplementation(scheme) = &mut record.payload else {
            panic!("scheme")
        };
        scheme.methods[0].implementations = operands;
    }
}

fn selected(
    n: u64,
    type_arguments: Vec<TypeObjectDigest>,
    implementations: Vec<ImplementationOperand>,
) -> ImplementationOperand {
    ImplementationOperand::Concrete {
        implementation: reference(n),
        type_arguments,
        implementations,
    }
}
fn lexical(n: u64, ordinal: u64) -> ImplementationOperand {
    ImplementationOperand::Parameter {
        scope: reference(n),
        parameter: witness(ordinal),
    }
}
fn implementation_call(
    read: &mut Read,
    owner: u64,
    target: u64,
    operands: Vec<ImplementationOperand>,
) -> ExpressionId {
    read.expression(
        reference(owner).package,
        ExpressionOperation::ImplementationCall {
            function: reference(target),
            type_arguments: Vec::new(),
            implementations: operands,
            effect_arguments: Vec::new(),
            requirement_arguments: Vec::new(),
            arguments: Vec::new(),
        },
    )
}
fn method_call(read: &mut Read, owner: u64, operand: ImplementationOperand) -> ExpressionId {
    read.expression(
        reference(owner).package,
        ExpressionOperation::MethodCall {
            witness: operand,
            contract: reference(900),
            method: method(0),
            arguments: Vec::new(),
        },
    )
}

#[test]
fn recursive_prerequisite_wrapping_rejects_before_exact_context_exploration() {
    let mut read = Read::default();
    let body = implementation_call(
        &mut read,
        0,
        0,
        vec![selected(1, vec![], vec![lexical(0, 0)])],
    );
    read.function(0, 0, 1, body);
    read.implementation(1, 0, vec![(0, 0, vec![])]);
    read.prerequisites(1, 1);
    read.mapping_prerequisites(1, vec![lexical(1, 0)]);
    let mut work = 0;
    let error = validate_callable_closure(&read, &mut work, 1_000).unwrap_err();
    assert_eq!(error.class, DiagnosticClass::Semantic);
    assert_eq!(error.code, "kernel_callable_recursive_witness_construction");
    assert!(error.message.contains("unsupported potential recursive"));
    assert!(!error.message.contains("expanding"));
    assert!(work < 1_000);
}

#[test]
fn unused_untaken_cross_package_prerequisite_construction_still_rejects() {
    let mut read = Read::default();
    let rejected = implementation_call(
        &mut read,
        0,
        2,
        vec![selected(1, vec![], vec![lexical(0, 0)])],
    );
    let condition = read.expression(package(0), ExpressionOperation::Bool { value: true });
    let terminal = read.expression(package(0), ExpressionOperation::I64 { value: 0 });
    let body = read.expression(
        package(0),
        ExpressionOperation::If {
            condition,
            when_true: terminal,
            when_false: rejected,
        },
    );
    read.function(0, 0, 1, body);
    let body = implementation_call(&mut read, 2, 0, vec![lexical(2, 0)]);
    read.function(2, 0, 1, body);
    let terminal = read.expression(package(1), ExpressionOperation::I64 { value: 4 });
    read.function(4, 0, 0, terminal);
    read.implementation(1, 0, vec![(0, 4, vec![])]);
    read.prerequisites(1, 1);
    assert_eq!(
        read.admit().unwrap_err().code,
        "kernel_callable_recursive_witness_construction"
    );
}

#[test]
fn balanced_method_projection_cycle_has_unsupported_construction_diagnostic() {
    let mut read = Read::default();
    // f0<W> -> Wrapped<W>.method0 -> f0<W> preserves witness depth. The deliberately
    // conservative admission rule still excludes construction inside this recursive SCC.
    let body = method_call(&mut read, 0, selected(1, vec![], vec![lexical(0, 0)]));
    read.function(0, 0, 1, body);
    read.implementation(1, 0, vec![(0, 0, vec![])]);
    read.prerequisites(1, 1);
    read.mapping_prerequisites(1, vec![lexical(1, 0)]);
    assert_eq!(
        read.admit().unwrap_err().code,
        "kernel_callable_recursive_witness_construction"
    );
}

#[test]
fn prerequisite_forwarding_permutation_and_closed_reset_cycles_are_finite() {
    let mut read = Read::default();
    let body = implementation_call(&mut read, 0, 2, vec![lexical(0, 1), lexical(0, 0)]);
    read.function(0, 0, 2, body);
    let body = implementation_call(&mut read, 2, 0, vec![lexical(2, 1), lexical(2, 0)]);
    read.function(2, 0, 2, body);
    let terminal = read.expression(package(1), ExpressionOperation::I64 { value: 3 });
    read.function(4, 0, 0, terminal);
    read.implementation(3, 0, vec![(0, 4, vec![])]);
    read.implementation(6, 0, vec![(0, 4, vec![])]);
    let body = implementation_call(
        &mut read,
        5,
        0,
        vec![selected(3, vec![], vec![]), selected(6, vec![], vec![])],
    );
    read.function(5, 0, 0, body);
    read.admit().unwrap();

    let mut read = Read::default();
    let body = implementation_call(
        &mut read,
        0,
        0,
        vec![selected(1, vec![], vec![selected(3, vec![], vec![])])],
    );
    read.function(0, 0, 1, body);
    read.implementation(1, 0, vec![(0, 0, vec![])]);
    read.prerequisites(1, 1);
    read.mapping_prerequisites(1, vec![lexical(1, 0)]);
    let terminal = read.expression(package(1), ExpressionOperation::I64 { value: 3 });
    read.function(4, 0, 0, terminal);
    read.implementation(3, 0, vec![(0, 4, vec![])]);
    read.admit().unwrap();
}

fn nonrecursive_delegation() -> Read {
    let mut read = Read::default();
    let nested = selected(1, vec![], vec![selected(1, vec![], vec![lexical(0, 0)])]);
    let body = implementation_call(&mut read, 0, 2, vec![nested]);
    read.function(0, 0, 1, body);
    let body = method_call(&mut read, 2, lexical(2, 0));
    read.function(2, 0, 1, body);
    read.implementation(1, 0, vec![(0, 4, vec![])]);
    read.prerequisites(1, 1);
    read.mapping_prerequisites(1, vec![lexical(1, 0)]);
    let body = method_call(&mut read, 4, lexical(4, 0));
    read.function(4, 0, 1, body);
    let terminal = read.expression(package(1), ExpressionOperation::I64 { value: 9 });
    read.function(6, 0, 0, terminal);
    read.implementation(3, 0, vec![(0, 6, vec![])]);
    let body = implementation_call(&mut read, 5, 0, vec![selected(3, vec![], vec![])]);
    read.function(5, 0, 0, body);
    read
}

#[test]
fn symbolic_nested_delegation_outside_potential_recursive_component_is_admitted() {
    // Root f0 constructs nested wrappers before calling the reusable reader. Reader
    // method dispatch may recurse through delegation, but no recursive edge constructs.
    nonrecursive_delegation().admit().unwrap();
}

#[test]
fn unused_same_contract_alternative_can_conservatively_reject_selected_finite_chain() {
    let mut read = nonrecursive_delegation();
    // Every actual call selects wrapper1 -> forwarding reader4 -> leaf3 -> terminal6.
    // Unused alternative7 could return to f0, making f0 -> f2 a potential recursive edge.
    read.implementation(7, 0, vec![(0, 0, vec![])]);
    read.mapping_prerequisites(7, vec![selected(3, vec![], vec![])]);
    let error = read.admit().unwrap_err();
    assert_eq!(error.code, "kernel_callable_recursive_witness_construction");
    assert!(error.message.contains("contract-matched alternatives"));
}

fn nested_type_callback(grow: bool) -> Read {
    let mut read = Read::default();
    let body = method_call(&mut read, 0, lexical(0, 0));
    read.function(0, 1, 1, body);
    let ty = read.p(1, 0);
    read.implementation(1, 1, vec![(0, 2, vec![ty])]);
    read.prerequisites(1, 1);
    read.mapping_prerequisites(1, vec![lexical(1, 0)]);
    let body = method_call(&mut read, 2, lexical(2, 0));
    read.function(2, 1, 1, body);
    let ty = read.p(3, 0);
    read.implementation(3, 1, vec![(0, 4, vec![ty])]);
    let ty = read.p(4, 0);
    let ty = if grow {
        read.ty(TypeForm::OwnedSequence { item: ty })
    } else {
        ty
    };
    let body = read.expression(
        package(1),
        ExpressionOperation::ImplementationCall {
            function: reference(0),
            type_arguments: vec![ty],
            implementations: vec![selected(1, vec![ty], vec![selected(3, vec![ty], vec![])])],
            effect_arguments: vec![],
            requirement_arguments: vec![],
            arguments: vec![],
        },
    );
    read.function(4, 1, 0, body);
    read
}

#[test]
fn nested_prerequisite_argument_provenance_preserves_independent_type_growth_proof() {
    nested_type_callback(false).admit().unwrap();
    let error = nested_type_callback(true).admit().unwrap_err();
    assert_eq!(error.code, "kernel_callable_expansion");
    assert!(error.message.contains("constructor path"));
}

#[test]
fn closed_prerequisite_method_reference_cycle_has_finite_contexts() {
    let mut read = Read::default();
    let body = method_call(&mut read, 0, lexical(0, 0));
    read.function(0, 0, 1, body);
    let body = method_call(&mut read, 2, lexical(2, 0));
    read.function(2, 0, 1, body);
    let terminal = read.expression(package(1), ExpressionOperation::I64 { value: 8 });
    read.function(8, 0, 0, terminal);
    read.implementation(6, 0, vec![(0, 8, vec![])]);
    read.implementation(1, 0, vec![(0, 0, vec![])]);
    read.prerequisites(1, 1);
    read.mapping_prerequisites(
        1,
        vec![selected(3, vec![], vec![selected(6, vec![], vec![])])],
    );
    read.implementation(3, 0, vec![(0, 2, vec![])]);
    read.prerequisites(3, 1);
    read.mapping_prerequisites(
        3,
        vec![selected(1, vec![], vec![selected(6, vec![], vec![])])],
    );
    read.admit().unwrap();
}

#[test]
fn acyclic_duplicate_prerequisites_share_selection_nodes_without_tree_expansion() {
    let mut read = Read::default();
    let terminal = read.expression(package(0), ExpressionOperation::I64 { value: 0 });
    read.function(0, 0, 1, terminal);
    // Synthetic flow-reader stress, not a fully admitted language declaration:
    // prerequisite schemes in public source require an Owned type parameter.
    // Forty tiny edges describe over a trillion logical leaf occurrences here.
    // With no type arguments, this observer has no distinct type-slot paths.
    for n in 1..=40 {
        let pair = selected(1000, vec![], vec![lexical(n, 0), lexical(n, 0)]);
        let body = implementation_call(&mut read, n, n - 1, vec![pair]);
        read.function(n, 0, 1, body);
    }
    let terminal = read.expression(package(1), ExpressionOperation::I64 { value: 1 });
    read.function(1002, 0, 2, terminal);
    read.implementation(1000, 0, vec![(0, 1002, vec![])]);
    read.prerequisites(1000, 2);
    read.mapping_prerequisites(1000, vec![lexical(1000, 0), lexical(1000, 1)]);
    let terminal = read.expression(package(1), ExpressionOperation::I64 { value: 2 });
    read.function(1003, 0, 0, terminal);
    read.implementation(1001, 0, vec![(0, 1003, vec![])]);
    let body = implementation_call(&mut read, 1004, 40, vec![selected(1001, vec![], vec![])]);
    read.function(1004, 0, 0, body);
    let mut work = 0;
    let observation = validate_callable_closure(&read, &mut work, 5_000_000).unwrap();
    assert_eq!(observation.slots, 0);
    assert!(observation.functions < 2_000);
    assert!(observation.metadata_bytes < 16 * 1024 * 1024);
    assert!(work < 5_000_000);
}

fn duplicate_shape_provenance(select_growing: bool) -> Read {
    let mut read = Read::default();
    let body = method_call(&mut read, 0, lexical(0, 0));
    read.function(0, 1, 1, body);
    let ty = read.p(1, 0);
    read.implementation(1, 1, vec![(0, 2, vec![ty])]);
    read.prerequisites(1, 2);
    read.mapping_prerequisites(1, vec![lexical(1, 0), lexical(1, 1)]);
    let body = method_call(&mut read, 2, lexical(2, u64::from(select_growing)));
    read.function(2, 1, 2, body);
    let ty = read.p(3, 0);
    read.implementation(3, 1, vec![(0, 4, vec![ty])]);
    let ty = read.p(4, 0);
    let nested = read.ty(TypeForm::OwnedSequence { item: ty });
    let closed = read.ty(TypeForm::OwnedI64Cell);
    let body = read.expression(
        package(1),
        ExpressionOperation::ImplementationCall {
            function: reference(0),
            type_arguments: vec![ty],
            implementations: vec![selected(
                1,
                vec![ty],
                vec![
                    selected(3, vec![closed], vec![]),
                    selected(3, vec![nested], vec![]),
                ],
            )],
            effect_arguments: vec![],
            requirement_arguments: vec![],
            arguments: vec![],
        },
    );
    read.function(4, 1, 0, body);
    read
}

#[test]
fn equal_interned_selection_shapes_keep_distinct_type_argument_paths() {
    // Both prerequisites intern to the same leaf scheme shape. Their applications
    // have different arguments; merging path slots would invent a growing cycle.
    duplicate_shape_provenance(false).admit().unwrap();
    assert_eq!(
        duplicate_shape_provenance(true).admit().unwrap_err().code,
        "kernel_callable_expansion"
    );
}

#[test]
fn exponential_distinct_type_provenance_paths_exhaust_proof_capacity_as_resource() {
    let mut read = Read::default();
    let terminal = read.expression(package(0), ExpressionOperation::I64 { value: 0 });
    read.function(0, 1, 1, terminal);
    for n in 1..=24 {
        let ty = read.p(n, 0);
        let body = read.expression(
            reference(n).package,
            ExpressionOperation::ImplementationCall {
                function: reference(n - 1),
                type_arguments: vec![ty],
                implementations: vec![selected(1000, vec![ty], vec![lexical(n, 0), lexical(n, 0)])],
                effect_arguments: vec![],
                requirement_arguments: vec![],
                arguments: vec![],
            },
        );
        read.function(n, 1, 1, body);
    }
    let terminal = read.expression(package(1), ExpressionOperation::I64 { value: 2 });
    read.function(1002, 1, 2, terminal);
    let ty = read.p(1000, 0);
    read.implementation(1000, 1, vec![(0, 1002, vec![ty])]);
    read.prerequisites(1000, 2);
    read.mapping_prerequisites(1000, vec![lexical(1000, 0), lexical(1000, 1)]);
    let mut work = 0;
    let error = validate_callable_closure(&read, &mut work, 1_000_000).unwrap_err();
    assert_eq!(error.class, DiagnosticClass::Resource);
    assert!(matches!(
        error.code.as_str(),
        "kernel_callable_flow_work" | "kernel_callable_flow_storage"
    ));
    assert!(work <= 1_000_000);
}

// Supplier f0<T,W> calls W.method0. Consumer scheme f1<T> maps method0 to consumer
// f2<T>, whose untaken body calls f0 with the scheme applied again. Dependencies may
// be acyclic (consumer -> supplier) although the resulting callable relation is cyclic.
fn callback(grow: bool, reset: bool) -> Read {
    let mut read = Read::default();
    let call = read.expression(
        package(0),
        ExpressionOperation::MethodCall {
            witness: ImplementationOperand::Parameter {
                scope: reference(0),
                parameter: witness(0),
            },
            contract: reference(900),
            method: method(0),
            arguments: Vec::new(),
        },
    );
    read.function(0, 1, 1, call);
    let p1 = read.p(1, 0);
    read.implementation(1, 1, vec![(0, 2, vec![p1])]);
    let p2 = read.p(2, 0);
    let ty = if reset {
        read.ty(TypeForm::OwnedI64Cell)
    } else if grow {
        read.ty(TypeForm::OwnedSequence { item: p2 })
    } else {
        p2
    };
    let unused = read.expression(
        package(1),
        ExpressionOperation::ImplementationCall {
            function: reference(0),
            type_arguments: vec![ty],
            implementations: vec![ImplementationOperand::Concrete {
                implementation: reference(1),
                type_arguments: vec![ty],
                implementations: Vec::new(),
            }],
            effect_arguments: Vec::new(),
            requirement_arguments: Vec::new(),
            arguments: Vec::new(),
        },
    );
    let condition = read.expression(package(1), ExpressionOperation::Bool { value: true });
    let result = read.expression(package(1), ExpressionOperation::I64 { value: 1 });
    let body = read.expression(
        package(1),
        ExpressionOperation::If {
            condition,
            when_true: result,
            when_false: unused,
        },
    );
    read.function(2, 1, 0, body);
    read
}

#[test]
fn downstream_witness_closes_foreign_expanding_cycle_even_in_untaken_body() {
    let error = callback(true, false).admit().unwrap_err();
    assert_eq!(error.class, DiagnosticClass::Semantic);
    assert_eq!(error.code, "kernel_callable_expansion");
    assert!(error.message.contains("slot") && error.message.contains("constructor path"));
    assert!(
        error.message.contains(&package(0).to_string())
            || error.message.contains(&package(1).to_string())
    );
}

#[test]
fn plain_cross_package_cycle_and_closed_reset_remain_finite() {
    let plain = callback(false, false).admit().unwrap();
    assert!(plain.edges >= 3 && plain.slots >= 3);
    // A constructor before a closed reset is acyclic parameter flow, not recursive growth.
    callback(true, true).admit().unwrap();
}

#[test]
fn unused_method_mapping_growth_is_checked_without_invocation() {
    let mut read = callback(false, false);
    let p1 = read.p(1, 0);
    let nested = read.ty(TypeForm::OwnedSequence { item: p1 });
    let OwnerRecord::Declaration(record) = read
        .owners
        .get_mut(&(package(1), OwnerKey::Declaration(reference(1).declaration)))
        .unwrap()
    else {
        panic!("scheme")
    };
    let DeclarationPayload::OwnedImplementation(i) = &mut record.payload else {
        panic!("scheme")
    };
    i.methods[0].type_arguments = vec![nested];
    assert_eq!(read.admit().unwrap_err().code, "kernel_callable_expansion");
}

#[test]
fn distinct_methods_do_not_invent_a_recursive_dispatch() {
    let mut read = callback(true, false);
    let terminal = read.expression(package(1), ExpressionOperation::I64 { value: 4 });
    read.function(3, 1, 0, terminal);
    let p1 = read.p(1, 0);
    // f0 invokes method0 -> terminal f3. Unused method1 -> f2 -> f0 -> method0 is
    // a finite chain, even though f2 constructs a larger scheme application.
    read.implementation(1, 1, vec![(0, 3, vec![p1]), (1, 2, vec![p1])]);
    read.admit().unwrap();
}

#[test]
fn parameter_permutation_across_scheme_and_function_slots_is_finite() {
    let mut read = Read::default();
    let call = read.expression(
        package(0),
        ExpressionOperation::MethodCall {
            witness: ImplementationOperand::Parameter {
                scope: reference(0),
                parameter: witness(0),
            },
            contract: reference(900),
            method: method(0),
            arguments: Vec::new(),
        },
    );
    read.function(0, 2, 1, call);
    let a = read.p(1, 0);
    let b = read.p(1, 1);
    read.implementation(1, 2, vec![(0, 2, vec![b, a])]);
    let a = read.p(2, 0);
    let b = read.p(2, 1);
    let call = read.expression(
        package(1),
        ExpressionOperation::ImplementationCall {
            function: reference(0),
            type_arguments: vec![a, b],
            implementations: vec![ImplementationOperand::Concrete {
                implementation: reference(1),
                type_arguments: vec![b, a],
                implementations: Vec::new(),
            }],
            effect_arguments: Vec::new(),
            requirement_arguments: Vec::new(),
            arguments: Vec::new(),
        },
    );
    read.function(2, 2, 0, call);
    read.admit().unwrap();
}

#[test]
fn explicit_forwarding_of_scheme_arguments_preserves_growth_cycle() {
    let mut read = callback(true, false);
    // Insert f3<T,W> between the supplier and its selected method, forwarding both
    // the ordinary argument and the exact applied witness rather than reconstructing it.
    let ty = read.p(0, 0);
    let body = read.expression(
        package(0),
        ExpressionOperation::ImplementationCall {
            function: reference(3),
            type_arguments: vec![ty],
            implementations: vec![ImplementationOperand::Parameter {
                scope: reference(0),
                parameter: witness(0),
            }],
            effect_arguments: Vec::new(),
            requirement_arguments: Vec::new(),
            arguments: Vec::new(),
        },
    );
    read.function(0, 1, 1, body);
    let body = read.expression(
        package(1),
        ExpressionOperation::MethodCall {
            witness: ImplementationOperand::Parameter {
                scope: reference(3),
                parameter: witness(0),
            },
            contract: reference(900),
            method: method(0),
            arguments: Vec::new(),
        },
    );
    read.function(3, 1, 1, body);
    assert_eq!(read.admit().unwrap_err().code, "kernel_callable_expansion");
}

#[test]
fn missing_supplier_body_and_erased_phantom_arguments_reject() {
    let mut read = callback(false, false);
    read.owners
        .remove(&(package(0), OwnerKey::Declaration(reference(0).declaration)));
    assert_eq!(
        read.admit().unwrap_err().code,
        "kernel_callable_flow_target"
    );
    let mut read = callback(false, false);
    let (_, record) = read
        .owners
        .iter_mut()
        .find(|(_, r)| {
            matches!(
                r,
                OwnerRecord::Expression(ExpressionRecord {
                    operation: ExpressionOperation::ImplementationCall { .. },
                    ..
                })
            )
        })
        .unwrap();
    let OwnerRecord::Expression(record) = record else {
        panic!("expression")
    };
    let ExpressionOperation::ImplementationCall {
        implementations, ..
    } = &mut record.operation
    else {
        panic!("call")
    };
    let ImplementationOperand::Concrete { type_arguments, .. } = &mut implementations[0] else {
        panic!("witness")
    };
    type_arguments.clear();
    assert_eq!(read.admit().unwrap_err().code, "kernel_callable_flow_arity");
}

#[test]
fn work_exhaustion_and_owning_cancellation_are_separate_from_invalid_meaning() {
    let read = callback(false, false);
    let error = validate_callable_closure(&read, &mut 0, 0).unwrap_err();
    assert_eq!(error.class, DiagnosticClass::Resource);
    assert_eq!(error.code, "kernel_callable_flow_work");
    read.cancel_after.set(Some(8));
    let error = read.admit().unwrap_err();
    assert_eq!(error.class, DiagnosticClass::Cancelled);
    assert_eq!(error.code, "test_cancelled");
}

#[derive(Clone, Debug)]
enum Term {
    Parameter(usize),
    Closed,
    Nested(Vec<Term>),
}
impl Term {
    fn occurrences(&self, nested: bool, result: &mut Vec<(usize, u8)>) {
        match self {
            Self::Parameter(i) => result.push((*i, if nested { 2 } else { 1 })),
            Self::Closed => {}
            Self::Nested(children) => {
                for child in children {
                    child.occurrences(true, result);
                }
            }
        }
    }
    fn intern(&self, read: &mut Read, declaration: u64) -> TypeObjectDigest {
        match self {
            Self::Parameter(i) => read.p(declaration, *i as u64),
            Self::Closed => read.ty(TypeForm::OwnedI64Cell),
            Self::Nested(children) if children.len() == 1 => {
                let item = children[0].intern(read, declaration);
                read.ty(TypeForm::OwnedSequence { item })
            }
            Self::Nested(children) => {
                let fields = children
                    .iter()
                    .enumerate()
                    .map(|(i, child)| StructuralTypeField {
                        name: Name::new(format!("f{i}")).unwrap(),
                        ty: child.intern(read, declaration),
                    })
                    .collect();
                read.ty(TypeForm::OwnedProduct { fields })
            }
        }
    }
}

// Independent weighted transitive closure. Slots 0,1 are the scheme's parameters and 2,3
// are method function parameters. A weight-2 diagonal proves constructor growth on a cycle.
// This oracle has no production context walker, extraction, substitution or SCC dependency.
fn reference_finite(method_arguments: &[Term], next_arguments: &[Term]) -> bool {
    let mut paths = [[0_u8; 4]; 4];
    for (source, target, arguments) in [(0, 2, method_arguments), (2, 0, next_arguments)] {
        for (ordinal, argument) in arguments.iter().enumerate() {
            let mut occurrences = Vec::new();
            argument.occurrences(false, &mut occurrences);
            for (parameter, weight) in occurrences {
                paths[source + parameter][target + ordinal] =
                    paths[source + parameter][target + ordinal].max(weight);
            }
        }
    }
    for middle in 0..4 {
        for source in 0..4 {
            for target in 0..4 {
                if paths[source][middle] != 0 && paths[middle][target] != 0 {
                    paths[source][target] =
                        paths[source][target].max(paths[source][middle].max(paths[middle][target]));
                }
            }
        }
    }
    (0..4).all(|slot| paths[slot][slot] != 2)
}

fn schema_callback(method_arguments: &[Term], next_arguments: &[Term]) -> Read {
    let mut read = Read::default();
    let call = read.expression(
        package(0),
        ExpressionOperation::MethodCall {
            witness: ImplementationOperand::Parameter {
                scope: reference(0),
                parameter: witness(0),
            },
            contract: reference(900),
            method: method(0),
            arguments: Vec::new(),
        },
    );
    read.function(0, 2, 1, call);
    let arguments = method_arguments
        .iter()
        .map(|t| t.intern(&mut read, 1))
        .collect();
    read.implementation(1, 2, vec![(0, 2, arguments)]);
    let arguments = next_arguments
        .iter()
        .map(|t| t.intern(&mut read, 2))
        .collect();
    let ground = read.ty(TypeForm::OwnedI64Cell);
    let call = read.expression(
        package(1),
        ExpressionOperation::ImplementationCall {
            function: reference(0),
            type_arguments: vec![ground, ground],
            implementations: vec![ImplementationOperand::Concrete {
                implementation: reference(1),
                type_arguments: arguments,
                implementations: Vec::new(),
            }],
            effect_arguments: Vec::new(),
            requirement_arguments: Vec::new(),
            arguments: Vec::new(),
        },
    );
    read.function(2, 2, 0, call);
    read
}

#[test]
fn exhaustive_composed_permutations_resets_phantoms_and_nested_occurrences_match_independent_oracle()
 {
    let terms = [
        Term::Parameter(0),
        Term::Parameter(1),
        Term::Closed,
        Term::Nested(vec![Term::Parameter(0)]),
        Term::Nested(vec![Term::Parameter(1)]),
        Term::Nested(vec![Term::Parameter(0), Term::Parameter(1)]),
    ];
    let mut accepted = 0;
    let mut rejected = 0;
    for a in &terms {
        for b in &terms {
            for c in &terms {
                for d in &terms {
                    let method_arguments = [a.clone(), b.clone()];
                    let next_arguments = [c.clone(), d.clone()];
                    let expected = reference_finite(&method_arguments, &next_arguments);
                    let result = schema_callback(&method_arguments, &next_arguments).admit();
                    match result {
                        Ok(_) => {
                            assert!(
                                expected,
                                "missed composed growth {method_arguments:?} {next_arguments:?}"
                            );
                            accepted += 1;
                        }
                        Err(error) => {
                            assert!(
                                !expected,
                                "rejected finite composed flow {method_arguments:?} {next_arguments:?}: {error}"
                            );
                            assert_eq!(error.code, "kernel_callable_expansion");
                            rejected += 1;
                        }
                    }
                }
            }
        }
    }
    assert!(accepted > 100 && rejected > 100);
    assert_eq!(accepted + rejected, 1296);
}

fn selected_chain(close_cycle: bool) -> Read {
    let mut read = Read::default();
    let call = read.expression(
        package(0),
        ExpressionOperation::MethodCall {
            witness: ImplementationOperand::Parameter {
                scope: reference(0),
                parameter: witness(0),
            },
            contract: reference(900),
            method: method(0),
            arguments: Vec::new(),
        },
    );
    read.function(0, 1, 1, call);
    let p = read.p(1, 0);
    read.implementation(1, 1, vec![(0, 2, vec![p])]);
    let p = read.p(4, 0);
    read.implementation(4, 1, vec![(0, 5, vec![p])]);
    let p = read.p(2, 0);
    let nested = read.ty(TypeForm::OwnedSequence { item: p });
    let call = read.expression(
        package(1),
        ExpressionOperation::ImplementationCall {
            function: reference(0),
            type_arguments: vec![nested],
            implementations: vec![ImplementationOperand::Concrete {
                implementation: reference(4),
                type_arguments: vec![nested],
                implementations: Vec::new(),
            }],
            effect_arguments: Vec::new(),
            requirement_arguments: Vec::new(),
            arguments: Vec::new(),
        },
    );
    read.function(2, 1, 0, call);
    let body = if close_cycle {
        let p = read.p(5, 0);
        read.expression(
            package(1),
            ExpressionOperation::ImplementationCall {
                function: reference(0),
                type_arguments: vec![p],
                implementations: vec![ImplementationOperand::Concrete {
                    implementation: reference(1),
                    type_arguments: vec![p],
                    implementations: Vec::new(),
                }],
                effect_arguments: Vec::new(),
                requirement_arguments: Vec::new(),
                arguments: Vec::new(),
            },
        )
    } else {
        read.expression(package(1), ExpressionOperation::I64 { value: 0 })
    };
    read.function(5, 1, 0, body);
    let ground = read.ty(TypeForm::OwnedI64Cell);
    let call = read.expression(
        package(1),
        ExpressionOperation::ImplementationCall {
            function: reference(0),
            type_arguments: vec![ground],
            implementations: vec![ImplementationOperand::Concrete {
                implementation: reference(1),
                type_arguments: vec![ground],
                implementations: Vec::new(),
            }],
            effect_arguments: Vec::new(),
            requirement_arguments: Vec::new(),
            arguments: Vec::new(),
        },
    );
    read.function(6, 0, 0, call);
    read
}

#[test]
fn exact_selected_declaration_vectors_separate_finite_chain_from_expanding_cycle() {
    // F<I<A>> -> G<A> -> F<J<Sequence<A>>> -> H<Sequence<A>> terminates. Merging
    // the two F witness contexts would invent G -> F -> I -> G and falsely reject.
    selected_chain(false).admit().unwrap();
    // H forwarding back to F<I<Sequence<A>>> closes the actual growing relation.
    assert_eq!(
        selected_chain(true).admit().unwrap_err().code,
        "kernel_callable_expansion"
    );
}
