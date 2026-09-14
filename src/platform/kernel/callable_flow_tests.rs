//! Independent finite term substitution and weighted reachability, without the production
//! extractor, SCC helper, type substitution, compiler, or preparation machinery.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;
use crate::platform::kernel::*;
use crate::platform::semantic_id::{DeclarationId, ModuleId};

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum Term {
    Parameter(usize),
    Ground(u8),
    Constructor(u8, Vec<Term>),
}

impl Term {
    fn substitute(&self, original: &[Self]) -> Self {
        match self {
            Self::Parameter(i) => original[*i].clone(),
            Self::Ground(_) => self.clone(),
            Self::Constructor(tag, children) => Self::Constructor(
                *tag,
                children.iter().map(|c| c.substitute(original)).collect(),
            ),
        }
    }
    fn occurrences(&self, nested: bool, output: &mut Vec<(usize, u8)>) {
        match self {
            Self::Parameter(i) => output.push((*i, if nested { 2 } else { 1 })),
            Self::Ground(_) => {}
            Self::Constructor(_, children) => {
                for child in children {
                    child.occurrences(true, output);
                }
            }
        }
    }
}

#[derive(Clone, Debug)]
struct Application {
    target: usize,
    arguments: Vec<Term>,
}
type Schema = Vec<(usize, Vec<Application>)>;
fn p(i: usize) -> Term {
    Term::Parameter(i)
}
fn ground(i: u8) -> Term {
    Term::Ground(i)
}
fn list(t: Term) -> Term {
    Term::Constructor(0, vec![t])
}
fn app(target: usize, arguments: Vec<Term>) -> Application {
    Application { target, arguments }
}

// A positive diagonal is a finite growth certificate, rather than a concrete-depth cutoff.
fn reference_admits(schema: &Schema) -> bool {
    let mut offsets = Vec::new();
    let mut total = 0;
    for (arity, _) in schema {
        offsets.push(total);
        total += arity;
    }
    let mut paths = vec![vec![0_u8; total]; total];
    for (caller, (_, applications)) in schema.iter().enumerate() {
        for application in applications {
            assert_eq!(schema[application.target].0, application.arguments.len());
            for (slot, argument) in application.arguments.iter().enumerate() {
                let mut occurrences = Vec::new();
                argument.occurrences(false, &mut occurrences);
                for (parameter, weight) in occurrences {
                    let edge =
                        &mut paths[offsets[caller] + parameter][offsets[application.target] + slot];
                    *edge = (*edge).max(weight);
                }
            }
        }
    }
    for middle in 0..total {
        for from in 0..total {
            for to in 0..total {
                if paths[from][middle] > 0 && paths[middle][to] > 0 {
                    paths[from][to] =
                        paths[from][to].max(paths[from][middle].max(paths[middle][to]));
                }
            }
        }
    }
    (0..total).all(|i| paths[i][i] != 2)
}

fn concrete_closure(schema: &Schema, start: (usize, Vec<Term>)) -> BTreeSet<(usize, Vec<Term>)> {
    assert!(reference_admits(schema));
    let mut visited = BTreeSet::new();
    let mut pending = vec![start];
    while let Some((function, arguments)) = pending.pop() {
        if !visited.insert((function, arguments.clone())) {
            continue;
        }
        assert!(
            visited.len() < 4096,
            "small independent finite oracle storage"
        );
        pending.extend(schema[function].1.iter().map(|application| {
            (
                application.target,
                application
                    .arguments
                    .iter()
                    .map(|t| t.substitute(&arguments))
                    .collect(),
            )
        }));
    }
    visited
}

const SEED: &[u8] = b"finite-callable-independent-schema";
fn package() -> PackageId {
    PackageId::migrate(SEED, 0)
}
fn declaration(i: usize) -> DeclarationId {
    DeclarationId::migrate(SEED, i as u64)
}
fn parameter(f: usize, p: usize) -> TypeParameterId {
    TypeParameterId::migrate(SEED, (f * 32 + p) as u64)
}
fn target(i: usize) -> DeclarationReference {
    DeclarationReference {
        package: package(),
        declaration: declaration(i),
    }
}

struct Read {
    owners: BTreeMap<OwnerKey, OwnerRecord>,
    types: TypeObjectInterner,
    control: crate::platform::execution::ExecutionControl,
}
impl ExpressionRead for Read {
    fn package_id(&self) -> PackageId {
        package()
    }
    fn owner(&self, key: OwnerKey) -> Result<Option<OwnerRecord>, Diagnostic> {
        Ok(self.owners.get(&key).cloned())
    }
    fn type_object(&self, key: TypeObjectDigest) -> Result<Option<TypeObject>, Diagnostic> {
        Ok(self.types.get(key).cloned())
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
    fn validation_checkpoint(&self) -> Result<(), Diagnostic> {
        self.control
            .check()
            .map_err(|e| Diagnostic::new(DiagnosticClass::Cancelled, e.code, e.message))
    }
}
impl Read {
    fn term(&mut self, caller: usize, term: &Term) -> TypeObjectDigest {
        let form = match term {
            Term::Parameter(i) => TypeForm::TypeParameter {
                parameter: parameter(caller, *i),
            },
            Term::Ground(0) => TypeForm::Text,
            Term::Ground(_) => TypeForm::I64,
            Term::Constructor(tag, children) => {
                let c: Vec<_> = children.iter().map(|t| self.term(caller, t)).collect();
                let atom = self.types.intern(TypeForm::I64).unwrap();
                match tag {
                    0 => TypeForm::List { item: c[0] },
                    1 => TypeForm::Map {
                        key: c[0],
                        value: *c.get(1).unwrap_or(&atom),
                    },
                    2 => TypeForm::Option { item: c[0] },
                    3 => TypeForm::Result {
                        ok: c[0],
                        error: *c.get(1).unwrap_or(&atom),
                    },
                    4 => TypeForm::Stream { item: c[0] },
                    5 => TypeForm::StructuralRecord {
                        fields: c
                            .iter()
                            .enumerate()
                            .map(|(i, ty)| StructuralTypeField {
                                name: Name::new(format!("f{i}")).unwrap(),
                                ty: *ty,
                            })
                            .collect(),
                    },
                    6 => TypeForm::Applied {
                        declaration: target(10_000),
                        arguments: c,
                    },
                    7 => TypeForm::Function {
                        parameters: c,
                        result: atom,
                    },
                    8 => TypeForm::Function {
                        parameters: vec![atom],
                        result: c[0],
                    },
                    9 => TypeForm::TaskFunction {
                        parameters: c,
                        result: atom,
                        effect: EffectRow::default(),
                    },
                    10 => TypeForm::TaskFunction {
                        parameters: vec![atom],
                        result: c[0],
                        effect: EffectRow::default(),
                    },
                    _ => panic!("unknown oracle constructor"),
                }
            }
        };
        self.types.intern(form).unwrap()
    }
    fn expression(&mut self, operation: ExpressionOperation) -> ExpressionId {
        let id = ExpressionId::migrate(SEED, self.owners.len() as u64);
        let record = ExpressionRecord::new(id, operation).unwrap();
        self.owners
            .insert(OwnerKey::Expression(id), OwnerRecord::Expression(record));
        id
    }
    fn new(schema: &Schema, named: bool) -> Self {
        let mut read = Self {
            owners: BTreeMap::new(),
            types: TypeObjectInterner::default(),
            control: crate::platform::execution::ExecutionControl::uncancelled(),
        };
        for (f, (arity, applications)) in schema.iter().enumerate() {
            let mut expressions = Vec::new();
            for application in applications {
                let type_arguments = application
                    .arguments
                    .iter()
                    .map(|t| read.term(f, t))
                    .collect();
                let operation = if named {
                    ExpressionOperation::FunctionValue {
                        requirement_arguments: Vec::new(),
                        function: target(application.target),
                        type_arguments,
                        effect_arguments: vec![],
                    }
                } else {
                    ExpressionOperation::Call {
                        requirement_arguments: Vec::new(),
                        function: target(application.target),
                        type_arguments,
                        effect_arguments: vec![],
                        arguments: vec![],
                    }
                };
                expressions.push(read.expression(operation));
            }
            expressions.push(read.expression(ExpressionOperation::I64 { value: 7 }));
            let unused = read.expression(ExpressionOperation::Sequence { items: expressions });
            let stop = read.expression(ExpressionOperation::Bool { value: true });
            let seven = read.expression(ExpressionOperation::I64 { value: 7 });
            let body = read.expression(ExpressionOperation::If {
                condition: stop,
                when_true: seven,
                when_false: unused,
            });
            let owner = OwnerKey::Declaration(declaration(f));
            let result = read.types.intern(TypeForm::I64).unwrap();
            read.owners.insert(
                owner,
                OwnerRecord::Declaration(DeclarationRecord {
                    header: OwnerHeader::new(owner, OwnerKind::PureFunction),
                    module: ModuleId::migrate(SEED, 0),
                    name: Name::new(format!("f{f}")).unwrap(),
                    visibility: DeclarationVisibility::Private,
                    payload: DeclarationPayload::Function(FunctionDeclaration {
                        requirement_parameters: Vec::new(),
                        effect_parameters: vec![],
                        type_parameters: (0..*arity).map(|p| parameter(f, p)).collect(),
                        parameters: vec![],
                        result,
                        effect: FunctionEffect::Pure,
                        body,
                    }),
                }),
            );
            for p in 0..*arity {
                let owner = OwnerKey::TypeParameter(parameter(f, p));
                read.owners.insert(
                    owner,
                    OwnerRecord::TypeParameter(TypeParameterRecord {
                        header: OwnerHeader::new(owner, OwnerKind::TypeParameter),
                        declaration: declaration(f),
                        name: Name::new(format!("T{p}")).unwrap(),
                        constraints: TypeParameterConstraints::None,
                    }),
                );
            }
        }
        read
    }
    fn admit(&self, maximum: usize) -> Result<CallableFlowWork, Diagnostic> {
        validate_callable_flow(self, self.owners.keys().copied(), &mut 0, maximum)
    }
}

#[test]
fn finite_callable_small_terms_and_simultaneous_instances() {
    let finite = [
        (
            vec![(2, vec![app(0, vec![p(1), p(0)])])],
            (0, vec![ground(0), ground(1)]),
            2,
        ),
        (
            vec![(2, vec![app(0, vec![p(0), p(0)])])],
            (0, vec![ground(0), ground(1)]),
            2,
        ),
        (
            vec![(2, vec![app(0, vec![ground(1), list(p(0))])])],
            (0, vec![ground(0), ground(1)]),
            3,
        ),
        (
            vec![
                (1, vec![app(1, vec![list(p(0))])]),
                (1, vec![app(0, vec![ground(1)])]),
            ],
            (0, vec![ground(0)]),
            4,
        ),
        (
            vec![
                (1, vec![app(1, vec![p(0), p(0)])]),
                (2, vec![app(0, vec![p(1)])]),
            ],
            (0, vec![ground(0)]),
            2,
        ),
    ];
    for (schema, start, instances) in finite {
        assert_eq!(concrete_closure(&schema, start).len(), instances);
        for named in [false, true] {
            assert!(
                Read::new(&schema, named).admit(100_000).is_ok(),
                "{schema:?}"
            );
        }
    }
    let terms = [p(0), p(1), ground(0), list(p(0)), list(p(1))];
    for a in &terms {
        for b in &terms {
            for c in &terms {
                for d in &terms {
                    let schema = vec![
                        (2, vec![app(1, vec![a.clone(), b.clone()])]),
                        (2, vec![app(0, vec![c.clone(), d.clone()])]),
                    ];
                    let expected = reference_admits(&schema);
                    for named in [false, true] {
                        let actual = Read::new(&schema, named).admit(100_000);
                        assert_eq!(actual.is_ok(), expected, "{schema:?}: {actual:?}");
                        if let Err(error) = actual {
                            assert_eq!(error.code, "kernel_callable_expansion");
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn finite_callable_scc_ladder_records_separate_analysis_cost() {
    for count in [1_usize, 8, 32, 128] {
        let schema: Schema = (0..count)
            .map(|index| (2, vec![app((index + 1) % count, vec![p(1), p(0)])]))
            .collect();
        let instances = concrete_closure(&schema, (0, vec![ground(0), ground(1)])).len();
        assert_eq!(instances, if count == 1 { 2 } else { count });
        let read = Read::new(&schema, false);
        let mut work = 0;
        let start = std::time::Instant::now();
        let observed =
            validate_callable_flow(&read, read.owners.keys().copied(), &mut work, 100_000).unwrap();
        let elapsed = start.elapsed().as_nanos();
        assert_eq!(observed.functions, count);
        assert_eq!(observed.applications, count);
        assert_eq!(observed.slots, 2 * count);
        assert_eq!(observed.edges, 2 * count);
        println!(
            "finite-scc-ladder functions={count} instances={instances} slots={} edges={} analysis-work={work} metadata-bytes={} analysis-nanoseconds={elapsed}",
            observed.slots, observed.edges, observed.metadata_bytes
        );
    }
}

#[test]
fn finite_callable_every_constructor_and_shared_occurrence() {
    for tag in 0..=10 {
        let nested = Term::Constructor(tag, vec![p(0)]);
        let schema = vec![(1, vec![app(0, vec![nested])])];
        assert!(!reference_admits(&schema));
        for named in [false, true] {
            let error = Read::new(&schema, named).admit(100_000).unwrap_err();
            assert_eq!(error.class, DiagnosticClass::Semantic);
            assert_eq!(error.code, "kernel_callable_expansion");
            assert!(
                error.message.contains("expression")
                    && error.message.contains("argument")
                    && error.message.contains("constructor")
            );
        }
    }
    // A shared T leaf appears first as a whole argument and also under a constructor.
    let schema = vec![(
        2,
        vec![app(0, vec![p(0), Term::Constructor(1, vec![p(1), p(0)])])],
    )];
    assert_eq!(
        Read::new(&schema, true).admit(100_000).unwrap_err().code,
        "kernel_callable_expansion"
    );
}

#[test]
fn finite_callable_long_cycle_is_semantic_and_work_cancellation_stay_distinct() {
    let size = 513;
    let schema: Schema = (0..size)
        .map(|i| {
            (
                1,
                vec![app(
                    (i + 1) % size,
                    vec![if i == 0 { list(p(0)) } else { p(0) }],
                )],
            )
        })
        .collect();
    let read = Read::new(&schema, false);
    let error = read.admit(1_000_000).unwrap_err();
    assert_eq!(error.class, DiagnosticClass::Semantic);
    assert_eq!(error.code, "kernel_callable_expansion");
    assert!(error.message.contains("513") && error.message.contains("omitted"));
    assert!(error.message.len() < 4096);
    let error = read.admit(1).unwrap_err();
    assert_eq!(error.class, DiagnosticClass::Resource);
    assert_eq!(error.code, "kernel_callable_flow_work");
    let mut cancelled = Read::new(&schema, false);
    cancelled.control = crate::platform::execution::ExecutionControl::cancel_after_checks(10);
    assert_eq!(
        cancelled.admit(1_000_000).unwrap_err().class,
        DiagnosticClass::Cancelled
    );
    assert_eq!(
        read.admit(1_000_000).unwrap_err().class,
        DiagnosticClass::Semantic
    );
}
