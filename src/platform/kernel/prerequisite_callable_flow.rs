//! Conservative finite preflight, before exact prerequisite contexts are enumerated.
//! Contract-matched method alternatives intentionally overapproximate selected dispatch.
use super::*;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct Node {
    owner: DeclarationReference,
    method: Option<MethodId>,
}
struct CallEdge {
    from: usize,
    to: usize,
    construction: bool,
    expression: Option<ExpressionId>,
}
struct Inventory {
    nodes: Vec<Node>,
    indexes: BTreeMap<Node, usize>,
    schemes: Vec<Scheme>,
}
struct Scheme {
    owner: DeclarationReference,
    contract: DeclarationReference,
    methods: Vec<MethodId>,
}

fn node<R: CallableClosureRead + ?Sized>(
    analysis: &mut Analysis<'_, R>,
    inventory: &mut Inventory,
    key: Node,
) -> Result<(), Diagnostic> {
    analysis.tick()?;
    analysis.reserve::<Node>(1)?;
    analysis.reserve::<(Node, usize)>(1)?;
    inventory.indexes.insert(key, inventory.nodes.len());
    inventory.nodes.push(key);
    Ok(())
}
fn edge<R: CallableClosureRead + ?Sized>(
    analysis: &mut Analysis<'_, R>,
    inventory: &Inventory,
    edges: &mut Vec<CallEdge>,
    from: usize,
    target: Node,
    construction: bool,
    expression: Option<ExpressionId>,
) -> Result<(), Diagnostic> {
    analysis.tick()?;
    let to = *inventory.indexes.get(&target).ok_or_else(|| {
        semantic(
            "kernel_callable_flow_target",
            "complete callable source is missing an exact callable target",
        )
    })?;
    analysis.reserve::<CallEdge>(1)?;
    edges.push(CallEdge {
        from,
        to,
        construction,
        expression,
    });
    Ok(())
}

fn constructing<R: CallableClosureRead + ?Sized>(
    analysis: &mut Analysis<'_, R>,
    owner: DeclarationReference,
    operands: &[ImplementationOperand],
) -> Result<bool, Diagnostic> {
    analysis.reserve::<(&ImplementationOperand, bool, usize)>(operands.len())?;
    let mut pending: Vec<_> = operands.iter().map(|operand| (operand, false, 0)).collect();
    let mut result = false;
    while let Some((operand, beneath, depth)) = pending.pop() {
        analysis.tick()?;
        if depth > crate::platform::kernel::contract::MAXIMUM_TYPE_DEPTH {
            return Err(Diagnostic::new(
                DiagnosticClass::Resource,
                "kernel_callable_flow_witness_depth",
                "prerequisite operand exceeds bounded witness depth",
            ));
        }
        match operand {
            ImplementationOperand::Concrete {
                implementation,
                type_arguments,
                implementations,
            } => {
                let DeclarationPayload::OwnedImplementation(scheme) =
                    analysis.declaration(*implementation)?.payload
                else {
                    return Err(semantic(
                        "kernel_callable_flow_witness",
                        "selected witness does not name an implementation scheme",
                    ));
                };
                if scheme.type_parameters.len() != type_arguments.len()
                    || scheme.implementation_parameters.len() != implementations.len()
                {
                    return Err(semantic(
                        "kernel_callable_flow_arity",
                        "applied witness requires every ordered scheme argument and prerequisite",
                    ));
                }
                analysis.reserve::<(&ImplementationOperand, bool, usize)>(implementations.len())?;
                pending.extend(
                    implementations
                        .iter()
                        .map(|operand| (operand, true, depth + 1)),
                );
            }
            ImplementationOperand::Parameter { scope, parameter } => {
                let declaration = analysis.declaration(owner)?;
                if *scope != owner {
                    return Err(semantic(
                        "kernel_callable_flow_witness_scope",
                        "witness parameter escapes its exact callable scope",
                    ));
                }
                analysis.parameter_ordinal(&declaration.payload, *parameter)?;
                result |= beneath;
            }
        }
    }
    Ok(result)
}

fn method_edges<R: CallableClosureRead + ?Sized>(
    analysis: &mut Analysis<'_, R>,
    inventory: &Inventory,
    edges: &mut Vec<CallEdge>,
    from: usize,
    operand: &ImplementationOperand,
    method: MethodId,
    expression: ExpressionId,
) -> Result<(), Diagnostic> {
    let owner = inventory.nodes[from].owner;
    let construction = constructing(analysis, owner, std::slice::from_ref(operand))?;
    match operand {
        ImplementationOperand::Concrete { implementation, .. } => {
            edge(
                analysis,
                inventory,
                edges,
                from,
                Node {
                    owner: *implementation,
                    method: Some(method),
                },
                construction,
                Some(expression),
            )?;
        }
        ImplementationOperand::Parameter { parameter, .. } => {
            let declaration = analysis.declaration(owner)?;
            let ordinal = analysis.parameter_ordinal(&declaration.payload, *parameter)?;
            let contract = implementation_parameters(&declaration.payload)[ordinal].contract;
            for scheme in &inventory.schemes {
                analysis.tick()?;
                if scheme.contract != contract {
                    continue;
                }
                let mut matches = false;
                for candidate in &scheme.methods {
                    analysis.tick()?;
                    if *candidate == method {
                        matches = true;
                        break;
                    }
                }
                if matches {
                    edge(
                        analysis,
                        inventory,
                        edges,
                        from,
                        Node {
                            owner: scheme.owner,
                            method: Some(method),
                        },
                        construction,
                        Some(expression),
                    )?;
                }
            }
        }
    }
    Ok(())
}

fn scan<R: CallableClosureRead + ?Sized>(
    analysis: &mut Analysis<'_, R>,
    inventory: &Inventory,
    edges: &mut Vec<CallEdge>,
    source: usize,
) -> Result<(), Diagnostic> {
    let key = inventory.nodes[source];
    let package = key.owner.package;
    let declaration = analysis.declaration(key.owner)?;
    if let DeclarationPayload::OwnedImplementation(scheme) = &declaration.payload {
        let mapping = analysis.mapping(scheme, key.method)?;
        let construction = constructing(analysis, key.owner, &mapping.implementations)?;
        return edge(
            analysis,
            inventory,
            edges,
            source,
            Node {
                owner: mapping.function,
                method: None,
            },
            construction,
            None,
        );
    }
    let roots = declaration.expression_roots();
    analysis.reserve::<ExpressionId>(roots.len())?;
    let mut pending = roots;
    if let DeclarationPayload::Component { ports, .. } = &declaration.payload {
        for port in ports {
            analysis.tick()?;
            let Some(OwnerRecord::Port(record)) = analysis
                .read
                .owner(key.owner.package, OwnerKey::Port(*port))?
            else {
                return Err(semantic(
                    "kernel_callable_flow_port",
                    "complete callable source is missing a component port",
                ));
            };
            let roots = OwnerRecord::Port(record).expression_roots();
            analysis.reserve::<ExpressionId>(roots.len())?;
            pending.extend(roots);
        }
    }
    let mut visited = BTreeSet::new();
    while let Some(expression) = pending.pop() {
        analysis.tick()?;
        if visited.contains(&expression) {
            continue;
        }
        analysis.reserve::<ExpressionId>(1)?;
        visited.insert(expression);
        let Some(OwnerRecord::Expression(record)) = analysis
            .read
            .owner(key.owner.package, OwnerKey::Expression(expression))?
        else {
            return Err(semantic(
                "kernel_callable_flow_expression",
                "complete callable source is missing body syntax",
            ));
        };
        match &record.operation {
            ExpressionOperation::Call { function, .. }
            | ExpressionOperation::FunctionValue { function, .. } => {
                edge(
                    analysis,
                    inventory,
                    edges,
                    source,
                    Node {
                        owner: *function,
                        method: None,
                    },
                    false,
                    Some(expression),
                )?;
            }
            ExpressionOperation::ImplementationCall {
                function,
                implementations,
                ..
            } => {
                let construction = constructing(analysis, key.owner, implementations)?;
                edge(
                    analysis,
                    inventory,
                    edges,
                    source,
                    Node {
                        owner: *function,
                        method: None,
                    },
                    construction,
                    Some(expression),
                )?;
            }
            ExpressionOperation::MethodCall {
                witness, method, ..
            } => {
                method_edges(
                    analysis, inventory, edges, source, witness, *method, expression,
                )?;
            }
            _ => {}
        }
        let children = record.children();
        analysis.reserve::<ExpressionId>(children.len())?;
        pending.extend(children.into_iter().map(|c| c.expression));
        if let ExpressionOperation::Let { bindings, .. } = &record.operation {
            // Initializers belong to Binding owners rather than expression child
            // records. Their complete syntax participates in prerequisite growth,
            // even when the value is unused. Traversal grants neither a local
            // value nor prospective source-loan authority; the exact composed
            // placement/flow admission proves those independently.
            for binding in bindings.iter().rev() {
                analysis.tick()?;
                let key = OwnerKey::Binding(*binding);
                let Some(OwnerRecord::Binding(binding)) = analysis.read.owner(package, key)? else {
                    return Err(semantic(
                        "kernel_callable_flow_binding",
                        "complete callable source is missing a let initializer binding",
                    ));
                };
                if binding.header.owner != key
                    || binding.header.kind != crate::platform::kernel::OwnerKind::Binding
                    || !matches!(binding.kind, BindingKind::Let | BindingKind::ReadView)
                {
                    return Err(semantic(
                        "kernel_callable_flow_binding",
                        "let initializer has a foreign binding identity or category",
                    ));
                }
                if binding.kind == BindingKind::ReadView {
                    let Some(annotation) = binding.declared_type else {
                        return Err(associated::invalid());
                    };
                    let object = analysis
                        .read
                        .type_object(package, annotation)?
                        .ok_or_else(associated::invalid)?;
                    if binding.header.contract_version < 34 || !object.form.is_read_view() {
                        return Err(associated::invalid());
                    }
                }
                let initializer = binding.value.ok_or_else(|| {
                    semantic(
                        "kernel_callable_flow_binding",
                        "let initializer binding has no exact value root",
                    )
                })?;
                analysis.reserve::<ExpressionId>(1)?;
                pending.push(initializer);
            }
        }
    }
    Ok(())
}

fn admit<R: CallableClosureRead + ?Sized>(
    analysis: &mut Analysis<'_, R>,
    inventory: &Inventory,
    edges: &[CallEdge],
) -> Result<(), Diagnostic> {
    let count = inventory.nodes.len();
    analysis.reserve::<Vec<usize>>(
        count
            .checked_mul(2)
            .ok_or_else(super::super::storage_overflow)?,
    )?;
    analysis.reserve::<usize>(
        count
            .checked_mul(10)
            .ok_or_else(super::super::storage_overflow)?,
    )?;
    analysis.reserve::<usize>(
        edges
            .len()
            .checked_mul(2)
            .ok_or_else(super::super::storage_overflow)?,
    )?;
    let mut outgoing = vec![Vec::new(); count];
    let mut incoming = vec![Vec::new(); count];
    for edge in edges {
        analysis.tick()?;
        outgoing[edge.from].push(edge.to);
        incoming[edge.to].push(edge.from);
    }
    let mut seen = vec![false; count];
    let mut finish = Vec::new();
    for root in 0..count {
        analysis.tick()?;
        if seen[root] {
            continue;
        }
        seen[root] = true;
        let mut stack = vec![(root, 0)];
        while let Some((node, next)) = stack.last_mut() {
            analysis.tick()?;
            if let Some(target) = outgoing[*node].get(*next) {
                *next += 1;
                if !seen[*target] {
                    seen[*target] = true;
                    stack.push((*target, 0));
                }
            } else {
                finish.push(*node);
                stack.pop();
            }
        }
    }
    let mut components = vec![usize::MAX; count];
    for root in finish.into_iter().rev() {
        analysis.tick()?;
        if components[root] != usize::MAX {
            continue;
        }
        components[root] = root;
        let mut pending = vec![root];
        while let Some(node) = pending.pop() {
            analysis.tick()?;
            for source in &incoming[node] {
                analysis.tick()?;
                if components[*source] == usize::MAX {
                    components[*source] = root;
                    pending.push(*source);
                }
            }
        }
    }
    for edge in edges {
        analysis.tick()?;
        if edge.construction && components[edge.from] == components[edge.to] {
            let from = inventory.nodes[edge.from];
            let to = inventory.nodes[edge.to];
            return Err(semantic(
                "kernel_callable_recursive_witness_construction",
                format!(
                    "unsupported potential recursive prerequisite witness construction: {}::{} method {:?} -> {}::{} method {:?}, expression {:?}; contract-matched alternatives share a recursive component; forward or project lexical prerequisites, or reset to closed witnesses",
                    from.owner.package,
                    from.owner.declaration,
                    from.method,
                    to.owner.package,
                    to.owner.declaration,
                    to.method,
                    edge.expression
                ),
            ));
        }
    }
    Ok(())
}

pub(super) fn validate<R: CallableClosureRead + ?Sized>(
    analysis: &mut Analysis<'_, R>,
) -> Result<(), Diagnostic> {
    let mut inventory = Inventory {
        nodes: Vec::new(),
        indexes: BTreeMap::new(),
        schemes: Vec::new(),
    };
    analysis.read.visit_callable_owners(&mut |owner| {
        let declaration = analysis.declaration(owner)?;
        if let DeclarationPayload::OwnedImplementation(scheme) = declaration.payload {
            for mapping in &scheme.methods {
                node(
                    analysis,
                    &mut inventory,
                    Node {
                        owner,
                        method: Some(mapping.method),
                    },
                )?;
            }
            analysis.reserve::<Scheme>(1)?;
            analysis.reserve::<MethodId>(scheme.methods.len())?;
            inventory.schemes.push(Scheme {
                owner,
                contract: scheme.contract,
                methods: scheme.methods.iter().map(|m| m.method).collect(),
            });
        } else {
            node(
                analysis,
                &mut inventory,
                Node {
                    owner,
                    method: None,
                },
            )?;
        }
        Ok(())
    })?;
    let mut edges = Vec::new();
    for source in 0..inventory.nodes.len() {
        scan(analysis, &inventory, &mut edges, source)?;
    }
    admit(analysis, &inventory, &edges)
}

#[cfg(test)]
mod initializer_tests {
    use super::*;
    use crate::platform::kernel::*;
    use crate::platform::semantic_id::{
        DeclarationId, FieldId, ModuleId, ParameterId, ReadScopeParameterId,
    };

    const SEED: &[u8] = b"prerequisite-hidden-initializer";

    #[derive(Default)]
    struct Read {
        owners: BTreeMap<(PackageId, OwnerKey), OwnerRecord>,
        types: TypeObjectInterner,
    }

    impl CallableClosureRead for Read {
        fn visit_callable_owners(
            &self,
            visitor: &mut dyn FnMut(DeclarationReference) -> Result<(), Diagnostic>,
        ) -> Result<(), Diagnostic> {
            for (package, key) in self.owners.keys() {
                if let OwnerKey::Declaration(declaration) = key {
                    visitor(DeclarationReference {
                        package: *package,
                        declaration: *declaration,
                    })?;
                }
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
    }

    impl Read {
        fn declaration(
            &mut self,
            reference: DeclarationReference,
            kind: OwnerKind,
            payload: DeclarationPayload,
        ) {
            let owner = OwnerKey::Declaration(reference.declaration);
            self.owners.insert(
                (reference.package, owner),
                OwnerRecord::Declaration(DeclarationRecord {
                    header: OwnerHeader::new(owner, kind),
                    module: ModuleId::migrate(SEED, 0),
                    name: Name::new(format!("d{}", self.owners.len())).unwrap(),
                    visibility: DeclarationVisibility::Private,
                    payload,
                }),
            );
        }
        fn expression(
            &mut self,
            package: PackageId,
            ordinal: u64,
            operation: ExpressionOperation,
        ) -> ExpressionId {
            let expression = ExpressionId::migrate(SEED, ordinal);
            self.owners.insert(
                (package, OwnerKey::Expression(expression)),
                OwnerRecord::Expression(ExpressionRecord::new(expression, operation).unwrap()),
            );
            expression
        }
        fn preflight(&self) -> Result<(), Diagnostic> {
            let mut work = 0;
            let mut analysis = Analysis {
                read: self,
                work: &mut work,
                maximum_work: contract::MAXIMUM_VALIDATION_WORK,
                maximum_metadata_bytes: MAXIMUM_ANALYSIS_BYTES,
                observation: CallableFlowWork::default(),
                contexts: vec![],
                indexes: BTreeMap::new(),
                selections: vec![],
                selection_indexes: BTreeMap::new(),
                pending: vec![],
                calls: vec![],
                slot_owners: vec![],
                edges: vec![],
                associated: associated::State::default(),
                completed_view_queries: BTreeMap::new(),
                diagnostic_context: WorkContext {
                    phase: "fixture initialization",
                    owner: None,
                    context: None,
                    expression: None,
                    query: None,
                },
            };
            validate(&mut analysis)
        }
    }

    fn reference(ordinal: u64) -> DeclarationReference {
        DeclarationReference {
            package: PackageId::migrate(SEED, 0),
            declaration: DeclarationId::migrate(SEED, ordinal),
        }
    }

    fn witness(
        ordinal: u64,
        contract: DeclarationReference,
        ty: TypeObjectDigest,
    ) -> ImplementationParameter {
        ImplementationParameter {
            id: ImplementationParameterId::migrate(SEED, ordinal),
            name: Name::new("w").unwrap(),
            contract,
            self_type: ty,
            type_arguments: vec![],
        }
    }

    fn fixture(view_initializer: bool, closed_reset: bool) -> (Read, ExpressionId) {
        let mut read = Read::default();
        let function = reference(0);
        let implementation = reference(1);
        let contract = reference(2);
        let closed = reference(3);
        let view = reference(4);
        let unit = read.types.intern(TypeForm::Unit).unwrap();
        let buffer = read.types.intern(TypeForm::ByteBuffer).unwrap();
        let method = MethodId::migrate(SEED, 0);
        let self_parameter = TypeParameterId::migrate(SEED, 0);
        let self_type = read
            .types
            .intern(TypeForm::TypeParameter {
                parameter: self_parameter,
            })
            .unwrap();
        read.owners.insert(
            (contract.package, OwnerKey::TypeParameter(self_parameter)),
            OwnerRecord::TypeParameter(TypeParameterRecord {
                header: OwnerHeader::new(
                    OwnerKey::TypeParameter(self_parameter),
                    OwnerKind::TypeParameter,
                ),
                declaration: contract.declaration,
                name: Name::new("Self").unwrap(),
                constraints: TypeParameterConstraints::Owned,
            }),
        );
        read.declaration(
            contract,
            OwnerKind::OwnedContract,
            DeclarationPayload::OwnedContract(OwnedContract {
                self_parameter,
                type_parameters: vec![],
                associated_types: vec![],
                methods: vec![OwnedMethod {
                    id: method,
                    name: Name::new("read").unwrap(),
                    parameters: vec![OwnedMethodParameter {
                        ty: self_type,
                        use_mode: ParameterUse::Borrow,
                    }],
                    result: CallableResult {
                        ty: unit,
                        relation: ResultRelation::Value,
                    },
                    read_scope_parameters: vec![],
                    read_scope_sources: vec![],
                    effect: FunctionEffect::Pure,
                }],
            }),
        );
        let formal = witness(0, contract, buffer);
        let prerequisite = witness(1, contract, buffer);
        let parameter = ParameterId::migrate(SEED, 0);
        let argument = read.expression(
            function.package,
            0,
            ExpressionOperation::Local {
                value: LocalValueReference::FunctionParameter(parameter),
            },
        );
        let operand = ImplementationOperand::Concrete {
            implementation,
            type_arguments: vec![],
            implementations: vec![if closed_reset {
                ImplementationOperand::Concrete {
                    implementation: closed,
                    type_arguments: vec![],
                    implementations: vec![],
                }
            } else {
                ImplementationOperand::Parameter {
                    scope: function,
                    parameter: formal.id,
                }
            }],
        };
        let call = read.expression(
            function.package,
            1,
            ExpressionOperation::ImplementationCall {
                function,
                arguments: vec![argument],
                type_arguments: vec![],
                implementations: vec![operand.clone()],
                effect_arguments: vec![],
                requirement_arguments: vec![],
                read_scope_arguments: vec![],
            },
        );
        let binding = BindingId::migrate(SEED, 0);
        let scope = ReadScopeArgument::Lexical {
            callable: function,
            binding,
        };
        let (initializer, annotation, kind) = if view_initializer {
            let field = FieldId::migrate(SEED, 0);
            read.declaration(
                view,
                OwnerKind::ReadView,
                DeclarationPayload::ReadView(ReadViewDeclaration {
                    type_parameters: vec![],
                    scope_parameter: ReadScopeParameter {
                        id: ReadScopeParameterId::migrate(SEED, 0),
                        name: Name::new("s").unwrap(),
                    },
                    fields: vec![ReadViewField {
                        id: field,
                        name: Name::new("metadata").unwrap(),
                        ty: unit,
                        kind: ReadViewFieldKind::Ordinary,
                    }],
                }),
            );
            let annotation = read
                .types
                .intern(TypeForm::ReadViewApplied {
                    declaration: view,
                    arguments: vec![],
                    read_scope: scope,
                })
                .unwrap();
            let initializer = read.expression(
                function.package,
                2,
                ExpressionOperation::CreateReadView {
                    declaration: view,
                    type_arguments: vec![],
                    read_scope: scope,
                    source: LocalValueReference::FunctionParameter(parameter),
                    fields: vec![ReadViewFieldInitializer { field, value: call }],
                },
            );
            (initializer, annotation, BindingKind::ReadView)
        } else {
            (call, unit, BindingKind::Let)
        };
        let body = read.expression(function.package, 3, ExpressionOperation::Unit {});
        let root = read.expression(
            function.package,
            4,
            ExpressionOperation::Let {
                bindings: vec![binding],
                body,
            },
        );
        read.owners.insert(
            (function.package, OwnerKey::Binding(binding)),
            OwnerRecord::Binding(BindingRecord {
                header: OwnerHeader::new(OwnerKey::Binding(binding), OwnerKind::Binding),
                name: Name::new("unused").unwrap(),
                kind,
                value: Some(initializer),
                declared_type: Some(annotation),
            }),
        );
        read.owners.insert(
            (function.package, OwnerKey::Parameter(parameter)),
            OwnerRecord::Parameter(ParameterRecord {
                header: OwnerHeader::new(OwnerKey::Parameter(parameter), OwnerKind::Parameter),
                parent: ParameterParent::Function(function.declaration),
                name: Name::new("source").unwrap(),
                ty: buffer,
                use_mode: ParameterUse::Borrow,
                resource_requirement: None,
            }),
        );
        read.declaration(
            function,
            OwnerKind::PureFunction,
            DeclarationPayload::Function(FunctionDeclaration {
                type_parameters: vec![],
                implementation_parameters: vec![formal],
                effect_parameters: vec![],
                requirement_parameters: vec![],
                parameters: vec![parameter],
                result: CallableResult {
                    ty: unit,
                    relation: ResultRelation::Value,
                },
                read_scope_parameters: vec![],
                read_scope_sources: vec![],
                effect: FunctionEffect::Pure,
                body: root,
            }),
        );
        read.declaration(
            implementation,
            OwnerKind::OwnedImplementation,
            DeclarationPayload::OwnedImplementation(OwnedImplementation {
                type_parameters: vec![],
                implementation_parameters: vec![prerequisite.clone()],
                contract,
                self_type: buffer,
                type_arguments: vec![],
                associated_types: vec![],
                methods: vec![OwnedMethodImplementation {
                    method,
                    function,
                    type_arguments: vec![],
                    implementations: vec![ImplementationOperand::Parameter {
                        scope: implementation,
                        parameter: prerequisite.id,
                    }],
                    read_scope_arguments: vec![],
                }],
            }),
        );
        if closed_reset {
            read.declaration(
                closed,
                OwnerKind::OwnedImplementation,
                DeclarationPayload::OwnedImplementation(OwnedImplementation {
                    type_parameters: vec![],
                    implementation_parameters: vec![],
                    contract,
                    self_type: buffer,
                    type_arguments: vec![],
                    associated_types: vec![],
                    methods: vec![OwnedMethodImplementation {
                        method,
                        function,
                        type_arguments: vec![],
                        implementations: vec![operand],
                        read_scope_arguments: vec![],
                    }],
                }),
            );
        }
        (read, call)
    }

    #[test]
    fn unused_ordinary_initializer_cannot_hide_recursive_prerequisite_construction() {
        let (read, _) = fixture(false, false);
        assert_eq!(
            read.preflight().unwrap_err().code,
            "kernel_callable_recursive_witness_construction"
        );
    }

    #[test]
    fn unused_view_initializer_field_cannot_hide_recursive_prerequisite_construction() {
        let (read, _) = fixture(true, false);
        assert_eq!(
            read.preflight().unwrap_err().code,
            "kernel_callable_recursive_witness_construction"
        );
    }

    #[test]
    fn closed_witness_reset_remains_finite_inside_unused_initializers() {
        for view in [false, true] {
            fixture(view, true).0.preflight().unwrap();
        }
    }

    #[test]
    fn unused_initializer_dependency_requires_exact_callable_source() {
        let (mut read, call) = fixture(true, true);
        let OwnerRecord::Expression(record) = read
            .owners
            .get_mut(&(reference(0).package, OwnerKey::Expression(call)))
            .unwrap()
        else {
            panic!("call");
        };
        let ExpressionOperation::ImplementationCall { function, .. } = &mut record.operation else {
            panic!("call");
        };
        *function = reference(99);
        assert_eq!(
            read.preflight().unwrap_err().code,
            "kernel_callable_flow_target"
        );
    }
}
