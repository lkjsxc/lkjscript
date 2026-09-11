//! Small finite terms, direct substitution, and weighted transitive closure. This oracle does
//! not call production collection, SCC, substitution, preparation, or property helpers.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;
use crate::platform::kernel::{
    DeclarationRecord, DeclarationVisibility, Name, OwnerHeader, TypeParameterConstraints,
    TypeParameterRecord,
};
use crate::platform::semantic_id::ModuleId;

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum Term {
    Parameter(usize),
    Atom,
    List(Box<Term>),
    Apply(usize, Vec<Term>),
    Function(Box<Term>, Box<Term>),
    TaskFunction(Box<Term>, Box<Term>),
}

impl Term {
    fn children(&self) -> Vec<&Self> {
        match self {
            Self::Parameter(_) | Self::Atom => vec![],
            Self::List(item) => vec![item],
            Self::Apply(_, arguments) => arguments.iter().collect(),
            Self::Function(parameter, result) | Self::TaskFunction(parameter, result) => {
                vec![parameter, result]
            }
        }
    }

    fn replace(&self, arguments: &[Self]) -> Self {
        match self {
            Self::Parameter(i) => arguments[*i].clone(),
            Self::Atom => Self::Atom,
            Self::List(item) => list(item.replace(arguments)),
            Self::Apply(declaration, children) => app(
                *declaration,
                children.iter().map(|t| t.replace(arguments)).collect(),
            ),
            Self::Function(parameter, result) => {
                function(parameter.replace(arguments), result.replace(arguments))
            }
            Self::TaskFunction(parameter, result) => {
                task_function(parameter.replace(arguments), result.replace(arguments))
            }
        }
    }
}

fn p(index: usize) -> Term {
    Term::Parameter(index)
}
fn list(item: Term) -> Term {
    Term::List(Box::new(item))
}
fn app(declaration: usize, arguments: Vec<Term>) -> Term {
    Term::Apply(declaration, arguments)
}
fn function(parameter: Term, result: Term) -> Term {
    Term::Function(Box::new(parameter), Box::new(result))
}
fn task_function(parameter: Term, result: Term) -> Term {
    Term::TaskFunction(Box::new(parameter), Box::new(result))
}

type Schema = Vec<(usize, Vec<Term>)>;

// Reachability in the semiring {absent, plain, expanding}. A positive diagonal is a
// finite certificate, independent of any bounded prefix of concrete enumeration.
fn reference_admits(schema: &Schema) -> bool {
    let slots: Vec<_> = schema
        .iter()
        .enumerate()
        .flat_map(|(d, (arity, _))| (0..*arity).map(move |p| (d, p)))
        .collect();
    let mut matrix = vec![vec![0_u8; slots.len()]; slots.len()];
    for (owner, (_, members)) in schema.iter().enumerate() {
        let mut terms: Vec<_> = members.iter().collect();
        while let Some(term) = terms.pop() {
            if let Term::Apply(target, arguments) = term {
                assert_eq!(arguments.len(), schema[*target].0);
                for (position, argument) in arguments.iter().enumerate() {
                    let to = slots
                        .iter()
                        .position(|s| *s == (*target, position))
                        .unwrap();
                    let mut occurrences = vec![(argument, 1)];
                    while let Some((occurrence, weight)) = occurrences.pop() {
                        if let Term::Parameter(parameter) = occurrence {
                            let from = slots
                                .iter()
                                .position(|s| *s == (owner, *parameter))
                                .unwrap();
                            matrix[from][to] = matrix[from][to].max(weight);
                        }
                        occurrences.extend(occurrence.children().into_iter().map(|c| (c, 2)));
                    }
                }
            }
            terms.extend(term.children());
        }
    }
    for middle in 0..slots.len() {
        for from in 0..slots.len() {
            for to in 0..slots.len() {
                if matrix[from][middle] != 0 && matrix[middle][to] != 0 {
                    matrix[from][to] =
                        matrix[from][to].max(matrix[from][middle].max(matrix[middle][to]));
                }
            }
        }
    }
    (0..slots.len()).all(|i| matrix[i][i] != 2)
}

fn reference_closure(schema: &Schema, root: Term) -> BTreeSet<Term> {
    assert!(reference_admits(schema));
    let mut pending = vec![root];
    let mut reached = BTreeSet::new();
    while let Some(term) = pending.pop() {
        if !reached.insert(term.clone()) {
            continue;
        }
        assert!(
            reached.len() < 4096,
            "small admitted fixture exceeded its independent bound"
        );
        if let Term::Apply(declaration, arguments) = &term {
            pending.extend(schema[*declaration].1.iter().map(|m| m.replace(arguments)));
        }
        pending.extend(term.children().into_iter().cloned());
    }
    reached
}

struct Read {
    owners: BTreeMap<OwnerKey, OwnerRecord>,
    interner: TypeObjectInterner,
    control: crate::platform::execution::ExecutionControl,
}

const SEED: &[u8] = b"finite-recursive-nominal-small-terms";
fn declaration(index: usize) -> DeclarationId {
    DeclarationId::migrate(SEED, index as u64)
}
fn parameter(owner: usize, index: usize) -> TypeParameterId {
    TypeParameterId::migrate(SEED, (owner * 8 + index) as u64)
}
fn package() -> PackageId {
    PackageId::migrate(SEED, 0)
}

impl ExpressionRead for Read {
    fn validation_checkpoint(&self) -> Result<(), Diagnostic> {
        self.control
            .check()
            .map_err(|error| Diagnostic::new(DiagnosticClass::Cancelled, error.code, error.message))
    }
    fn package_id(&self) -> PackageId {
        package()
    }
    fn owner(&self, owner: OwnerKey) -> Result<Option<OwnerRecord>, Diagnostic> {
        Ok(self.owners.get(&owner).cloned())
    }
    fn type_object(&self, digest: TypeObjectDigest) -> Result<Option<TypeObject>, Diagnostic> {
        Ok(self.interner.get(digest).cloned())
    }
    fn package_interface_owner(
        &self,
        _: PackageId,
        _: OwnerKey,
    ) -> Result<Option<PackageInterfaceRecord>, Diagnostic> {
        Ok(None)
    }
    fn has_dependency(&self, _: PackageId) -> Result<bool, Diagnostic> {
        Ok(false)
    }
}

impl Read {
    fn term(&mut self, owner: usize, term: &Term) -> TypeObjectDigest {
        let form = match term {
            Term::Parameter(index) => TypeForm::TypeParameter {
                parameter: parameter(owner, *index),
            },
            Term::Atom => TypeForm::I64,
            Term::List(item) => TypeForm::List {
                item: self.term(owner, item),
            },
            Term::Apply(index, arguments) => {
                let declaration = DeclarationReference {
                    package: package(),
                    declaration: declaration(*index),
                };
                if arguments.is_empty() {
                    TypeForm::Named { declaration }
                } else {
                    TypeForm::Applied {
                        declaration,
                        arguments: arguments.iter().map(|a| self.term(owner, a)).collect(),
                    }
                }
            }
            Term::Function(parameter, result) => TypeForm::Function {
                parameters: vec![self.term(owner, parameter)],
                result: self.term(owner, result),
            },
            Term::TaskFunction(parameter, result) => TypeForm::TaskFunction {
                parameters: vec![self.term(owner, parameter)],
                result: self.term(owner, result),
                effect: super::super::EffectRow::default(),
            },
        };
        self.interner.intern(form).unwrap()
    }

    fn new(schema: &Schema) -> Self {
        let mut read = Self {
            owners: BTreeMap::new(),
            interner: TypeObjectInterner::default(),
            control: crate::platform::execution::ExecutionControl::uncancelled(),
        };
        for (d, (arity, members)) in schema.iter().enumerate() {
            let mut fields = Vec::new();
            for (i, member) in members.iter().enumerate() {
                let field = FieldId::migrate(SEED, (d * 8 + i) as u64);
                let ty = read.term(d, member);
                let record = OwnerRecord::Field(FieldRecord {
                    header: OwnerHeader::new(OwnerKey::Field(field), OwnerKind::Field),
                    declaration: declaration(d),
                    name: Name::new(format!("member-{i}")).unwrap(),
                    ty,
                });
                read.owners.insert(record.owner(), record);
                fields.push(field);
            }
            let record = OwnerRecord::Declaration(DeclarationRecord {
                header: OwnerHeader::new(OwnerKey::Declaration(declaration(d)), OwnerKind::Record),
                module: ModuleId::migrate(SEED, 0),
                name: Name::new(format!("nominal-{d}")).unwrap(),
                visibility: DeclarationVisibility::Public,
                payload: DeclarationPayload::Record {
                    type_parameters: (0..*arity).map(|i| parameter(d, i)).collect(),
                    fields,
                },
            });
            read.owners.insert(record.owner(), record);
            for i in 0..*arity {
                let record = OwnerRecord::TypeParameter(TypeParameterRecord {
                    header: OwnerHeader::new(
                        OwnerKey::TypeParameter(parameter(d, i)),
                        OwnerKind::TypeParameter,
                    ),
                    declaration: declaration(d),
                    name: Name::new(format!("T{i}")).unwrap(),
                    constraints: TypeParameterConstraints::None,
                });
                read.owners.insert(record.owner(), record);
            }
        }
        read
    }
}

fn production(schema: &Schema) -> Vec<Diagnostic> {
    let read = Read::new(schema);
    let mut diagnostics = vec![];
    let outcome = validate_expression_roots_with_limits(
        &read,
        read.owners.keys().copied(),
        &mut diagnostics,
        &mut 0,
        ExpressionValidationLimits {
            maximum_steps: 1_000_000,
            maximum_diagnostics: 100,
        },
    );
    assert!(outcome.is_ok(), "{schema:?}: {outcome:?}; {diagnostics:?}");
    diagnostics
}

fn cases() -> Vec<(&'static str, Schema, bool)> {
    vec![
        (
            "tree",
            vec![(1, vec![p(0), list(app(0, vec![p(0)]))])],
            true,
        ),
        (
            "uninhabited-record",
            vec![(1, vec![app(0, vec![p(0)])])],
            true,
        ),
        (
            "mutual",
            vec![
                (1, vec![app(1, vec![p(0)])]),
                (1, vec![list(app(0, vec![p(0)]))]),
            ],
            true,
        ),
        (
            "permutation",
            vec![(2, vec![app(0, vec![p(1), p(0)])])],
            true,
        ),
        (
            "duplication-selection",
            vec![
                (1, vec![app(1, vec![p(0), p(0)])]),
                (2, vec![app(0, vec![p(1)])]),
            ],
            true,
        ),
        (
            "dropped-growth-reset",
            vec![
                (1, vec![app(1, vec![list(p(0)), Term::Atom])]),
                (2, vec![app(0, vec![p(1)])]),
            ],
            true,
        ),
        (
            "wrapper",
            vec![
                (1, vec![app(1, vec![])]),
                (0, vec![app(0, vec![Term::Atom])]),
            ],
            true,
        ),
        (
            "signature-forward",
            vec![(1, vec![function(Term::Atom, app(0, vec![p(0)]))])],
            true,
        ),
        (
            "task-signature-forward",
            vec![(1, vec![task_function(Term::Atom, app(0, vec![p(0)]))])],
            true,
        ),
        (
            "task-signature-hidden-growth",
            vec![(1, vec![task_function(app(0, vec![list(p(0))]), Term::Atom)])],
            false,
        ),
        (
            "task-signature-argument-growth",
            vec![(1, vec![app(0, vec![task_function(p(0), Term::Atom)])])],
            false,
        ),
        (
            "expanding-self",
            vec![(1, vec![app(0, vec![list(p(0))])])],
            false,
        ),
        (
            "expanding-mutual",
            vec![
                (1, vec![app(1, vec![p(0)])]),
                (1, vec![app(0, vec![list(p(0))])]),
            ],
            false,
        ),
        (
            "signature-hidden",
            vec![(1, vec![function(app(0, vec![list(p(0))]), Term::Atom)])],
            false,
        ),
        (
            "nominal-argument-phantom",
            vec![
                (1, vec![app(0, vec![app(1, vec![p(0)])])]),
                (1, vec![Term::Atom]),
            ],
            false,
        ),
        (
            "nested-application",
            vec![
                (1, vec![app(1, vec![app(0, vec![list(p(0))])])]),
                (1, vec![Term::Atom]),
            ],
            false,
        ),
    ]
}

#[test]
fn finite_recursive_nominals_small_term_boundary() {
    for (name, schema, admitted) in cases() {
        assert_eq!(reference_admits(&schema), admitted, "reference {name}");
        let diagnostics = production(&schema);
        if admitted {
            assert!(diagnostics.is_empty(), "{name}: {diagnostics:?}");
            for (d, (arity, _)) in schema.iter().enumerate() {
                reference_closure(&schema, app(d, vec![Term::Atom; *arity]));
            }
        } else {
            assert!(
                diagnostics
                    .iter()
                    .any(|d| d.code == "kernel_type_nominal_expansion"),
                "{name}: {diagnostics:?}"
            );
            let witness = diagnostics
                .iter()
                .find(|d| d.code == "kernel_type_nominal_expansion")
                .unwrap();
            for expected in ["expanding", "argument", "member", "typeparam_"] {
                assert!(
                    witness.message.contains(expected),
                    "missing {expected}: {witness:?}"
                );
            }
        }
    }
}

#[test]
fn finite_recursive_nominals_independent_enumeration() {
    let schema = vec![(2, vec![app(0, vec![p(1), p(0)])])];
    let root = app(0, vec![Term::Atom, list(Term::Atom)]);
    let closure = reference_closure(&schema, root.clone());
    assert_eq!(
        closure
            .iter()
            .filter(|t| matches!(t, Term::Apply(..)))
            .cloned()
            .collect::<BTreeSet<_>>(),
        BTreeSet::from([root, app(0, vec![list(Term::Atom), Term::Atom])])
    );
    for left in [p(0), list(p(0)), Term::Atom] {
        for right in [p(0), list(p(0)), Term::Atom] {
            let schema = vec![
                (1, vec![app(1, vec![left.clone()])]),
                (1, vec![app(0, vec![right])]),
            ];
            let admitted = reference_admits(&schema);
            assert_eq!(production(&schema).is_empty(), admitted, "{schema:?}");
            if admitted {
                reference_closure(&schema, app(0, vec![Term::Atom]));
            }
        }
    }
}

#[test]
fn finite_recursive_nominals_bounded_analysis_cancellation_and_recovery() {
    let schema = cases()[4].1.clone();
    let mut read = Read::new(&schema);
    let mut diagnostics = Vec::new();
    let mut work = 0;
    let outcome = validate_expression_roots_with_limits(
        &read,
        read.owners.keys().copied(),
        &mut diagnostics,
        &mut work,
        ExpressionValidationLimits {
            maximum_steps: 5,
            maximum_diagnostics: 100,
        },
    );
    assert_eq!(outcome, Err(ExpressionValidationExhaustion::Steps));
    assert_eq!(work, 5);
    read.control = crate::platform::execution::ExecutionControl::cancel_after_checks(40);
    diagnostics.clear();
    work = 0;
    validate_expression_roots_with_limits(
        &read,
        read.owners.keys().copied(),
        &mut diagnostics,
        &mut work,
        ExpressionValidationLimits {
            maximum_steps: 1_000_000,
            maximum_diagnostics: 100,
        },
    )
    .unwrap();
    assert!(
        diagnostics
            .iter()
            .any(|d| d.class == DiagnosticClass::Cancelled)
    );
    assert!(work <= 40);
    assert!(production(&schema).is_empty());
}
