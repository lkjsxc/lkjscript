use std::sync::Arc;

use super::{
    CallEdge, DiagnosticHeader, EntityHeader, EntityId, EntityKind, HoleId, HoleState, NodeId,
    ReferenceEdge, RevisionId, SemanticKind, SemanticType, UnresolvedValueReferenceId,
    UnresolvedValueReferenceState, WorkspaceError, WorkspaceSnapshot,
};

#[cfg(test)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) struct QueryMeasurement {
    pub candidates_scanned: usize,
    pub results_materialized: usize,
    pub sorted_items: usize,
    pub items_returned: usize,
    pub pages_built: usize,
}

#[cfg(test)]
thread_local! {
    static QUERY_MEASUREMENT: std::cell::RefCell<QueryMeasurement> =
        const { std::cell::RefCell::new(QueryMeasurement {
            candidates_scanned: 0,
            results_materialized: 0,
            sorted_items: 0,
            items_returned: 0,
            pages_built: 0,
        }) };
}

#[cfg(test)]
pub(super) fn reset_query_measurement() {
    QUERY_MEASUREMENT.with(|measurement| {
        *measurement.borrow_mut() = QueryMeasurement::default();
    });
}

#[cfg(test)]
pub(super) fn take_query_measurement() -> QueryMeasurement {
    QUERY_MEASUREMENT.with(|measurement| std::mem::take(&mut *measurement.borrow_mut()))
}

#[cfg(test)]
#[allow(clippy::expect_used)]
fn record_query_measurement(
    candidates_scanned: usize,
    results_materialized: usize,
    sorted_items: usize,
    items_returned: usize,
    pages_built: usize,
) {
    QUERY_MEASUREMENT.with(|measurement| {
        let mut measurement = measurement.borrow_mut();
        measurement.candidates_scanned = measurement
            .candidates_scanned
            .checked_add(candidates_scanned)
            .expect("query candidate measurement overflow");
        measurement.results_materialized = measurement
            .results_materialized
            .checked_add(results_materialized)
            .expect("query materialization measurement overflow");
        measurement.sorted_items = measurement
            .sorted_items
            .checked_add(sorted_items)
            .expect("query sort measurement overflow");
        measurement.items_returned = measurement
            .items_returned
            .checked_add(items_returned)
            .expect("query return measurement overflow");
        measurement.pages_built = measurement
            .pages_built
            .checked_add(pages_built)
            .expect("query page measurement overflow");
    });
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PageRequest {
    limit: usize,
}

impl PageRequest {
    pub fn new(limit: usize) -> Result<Self, WorkspaceError> {
        if limit == 0 {
            return Err(WorkspaceError::InvalidContinuation(Arc::from(
                "query page size must be non-zero",
            )));
        }
        Ok(Self { limit })
    }

    pub const fn limit(self) -> usize {
        self.limit
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Continuation {
    namespace: super::WorkspaceNamespace,
    revision: RevisionId,
    query: [u8; 32],
    offset: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct QueryPage<T> {
    pub revision: RevisionId,
    pub items: Vec<T>,
    pub continuation: Option<Continuation>,
}

pub type EntityPage = QueryPage<EntityHeader>;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[non_exhaustive]
pub enum ValueReferenceCandidateStatus {
    RequiresCanonicalValidation,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValueReferenceCandidate {
    pub entity: EntityId,
    pub name: Arc<str>,
    pub kind: EntityKind,
    pub declared_type: SemanticType,
    pub exact_name_match: bool,
    pub status: ValueReferenceCandidateStatus,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NodeTypeFacts {
    pub revision: RevisionId,
    pub node: NodeId,
    pub actual: SemanticType,
    pub expected: Option<SemanticType>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NodeSemanticFacts {
    pub revision: RevisionId,
    pub node: NodeId,
    pub kind: super::NodeKind,
    pub actual: SemanticType,
    pub expected: Option<SemanticType>,
    pub operation: Option<crate::operation::Operation>,
    pub effects: EffectSummary,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EntityTypeFacts {
    pub revision: RevisionId,
    pub entity: EntityId,
    pub declared: Option<SemanticType>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TypeParameterBoundView {
    pub parameter: EntityId,
    pub trait_identity: super::SemanticTrait,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TypeParameterView {
    pub id: EntityId,
    pub name: Arc<str>,
    pub owner: EntityId,
    pub bounds: Vec<TypeParameterBoundView>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValueParameterView {
    pub entity: EntityId,
    pub name: Arc<str>,
    pub ty: SemanticType,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FunctionSignatureView {
    pub revision: RevisionId,
    pub function: EntityId,
    pub type_parameters: Vec<TypeParameterView>,
    pub parameters: Vec<ValueParameterView>,
    pub result: SemanticType,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EffectSummary(u16);

impl EffectSummary {
    pub const UNKNOWN: Self = Self(1 << 15);
    pub const ALLOCATES: Self = Self(1 << 0);
    pub const READS_MEMORY: Self = Self(1 << 1);
    pub const WRITES_MEMORY: Self = Self(1 << 2);
    pub const MUTATES_LOCAL: Self = Self(1 << 3);
    pub const HOST_IO: Self = Self(1 << 4);
    pub const MAY_TRAP: Self = Self(1 << 5);
    pub const MAY_EXIT: Self = Self(1 << 6);
    pub const MAY_DIVERGE: Self = Self(1 << 7);

    pub const fn bits(self) -> u16 {
        self.0
    }

    pub const fn is_known(self) -> bool {
        !self.contains(Self::UNKNOWN)
    }

    pub const fn is_pure(self) -> bool {
        self.0 == 0
    }

    pub const fn contains(self, effect: Self) -> bool {
        self.0 & effect.0 == effect.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TraitWitnessKindView {
    AutoTrait,
    Explicit(EntityId),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TraitWitnessView {
    pub parameter: EntityId,
    pub trait_identity: super::SemanticTrait,
    pub ty: SemanticType,
    pub kind: TraitWitnessKindView,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TypeArgumentView {
    pub parameter: EntityId,
    pub argument: SemanticType,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CallInstantiationView {
    pub revision: RevisionId,
    pub site: NodeId,
    pub callee: EntityId,
    pub type_arguments: Vec<TypeArgumentView>,
    pub parameters: Vec<SemanticType>,
    pub result: SemanticType,
    pub witnesses: Vec<TraitWitnessView>,
    pub effects: EffectSummary,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MatchView {
    pub revision: RevisionId,
    pub site: NodeId,
    pub scrutinee: NodeId,
    pub result: SemanticType,
    pub arms: Vec<MatchArmView>,
    pub exhaustive: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MatchArmView {
    pub pattern_root: MatchPatternLabel,
    pub patterns: Vec<MatchPatternNodeView>,
    pub body: NodeId,
    pub result: SemanticType,
}

/// Opaque cross-reference within one flat, stack-safe `MatchArmView` pattern graph.
///
/// A label is local to its returned arm view. It is not a workspace identity,
/// compiler identity, transaction input, or stable reference across queries.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct MatchPatternLabel(u64);

impl MatchPatternLabel {
    const fn new(value: u64) -> Self {
        Self(value)
    }

    pub(super) const fn projection_ordinal(self) -> u64 {
        self.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MatchPatternNodeView {
    pub label: MatchPatternLabel,
    pub ty: SemanticType,
    pub kind: MatchPatternKindView,
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum MatchPatternKindView {
    Wildcard,
    Binding {
        binding: EntityId,
    },
    Bool(bool),
    I64(i64),
    EnumVariant {
        enumeration: Option<EntityId>,
        variant: Option<EntityId>,
        fields: Vec<MatchPatternFieldView>,
    },
    Product {
        product: EntityId,
        fields: Vec<MatchPatternFieldView>,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MatchPatternFieldView {
    pub field: Option<EntityId>,
    pub pattern: MatchPatternLabel,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ConstructorStatus {
    Established,
    RequiresOwnershipValidation,
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum LegalConstructor {
    I64Literal,
    F64Literal,
    BoolLiteral,
    UnitLiteral,
    Operation(crate::operation::Operation),
    Load(EntityId),
    Move {
        binding: EntityId,
        status: ConstructorStatus,
    },
    BorrowShared {
        binding: EntityId,
        status: ConstructorStatus,
    },
    Call(EntityId),
    Product(EntityId),
    EnumVariant(EntityId),
    If,
    Sequence,
    MutableLocal,
    SetLocal(EntityId),
    While,
    Loop {
        result_type: SemanticType,
    },
    Break {
        value_type: SemanticType,
    },
    Continue,
    Return,
}

impl WorkspaceSnapshot {
    pub fn entity_page(
        &self,
        revision: RevisionId,
        request: PageRequest,
        continuation: Option<&Continuation>,
    ) -> Result<EntityPage, WorkspaceError> {
        self.check_query_revision(revision)?;
        let query = query_key(b"entities", &[])?;
        let mut values = Vec::new();
        values
            .try_reserve(self.indexes.entities.len())
            .map_err(|_| WorkspaceError::Host(Arc::from("entity page allocation failed")))?;
        values.extend(self.indexes.entities.iter().cloned());
        values.sort_by_key(|header| header.id);
        #[cfg(test)]
        record_query_measurement(
            self.indexes.entities.len(),
            values.len(),
            values.len(),
            0,
            0,
        );
        page(self, query, request, continuation, &values)
    }

    pub fn search_entities(
        &self,
        revision: RevisionId,
        text: &str,
        request: PageRequest,
        continuation: Option<&Continuation>,
    ) -> Result<EntityPage, WorkspaceError> {
        self.check_query_revision(revision)?;
        let query = query_key(b"search-entities", text.as_bytes())?;
        let mut values = Vec::new();
        values
            .try_reserve(self.indexes.entities.len())
            .map_err(|_| WorkspaceError::Host(Arc::from("entity search allocation failed")))?;
        values.extend(
            self.indexes
                .entities
                .iter()
                .filter(|header| contains_ignoring_ascii_case(&header.name, text))
                .cloned(),
        );
        values.sort_by(|left, right| {
            left.name
                .cmp(&right.name)
                .then_with(|| left.id.cmp(&right.id))
        });
        page(self, query, request, continuation, &values)
    }

    pub fn definition(
        &self,
        revision: RevisionId,
        entity: EntityId,
    ) -> Result<EntityHeader, WorkspaceError> {
        self.check_query_revision(revision)?;
        self.workspace_entity(entity).cloned()
    }

    pub fn references_to(
        &self,
        revision: RevisionId,
        entity: EntityId,
        request: PageRequest,
        continuation: Option<&Continuation>,
    ) -> Result<QueryPage<ReferenceEdge>, WorkspaceError> {
        self.check_query_revision(revision)?;
        self.workspace_entity(entity)?;
        let query = id_query_key(b"references", entity)?;
        let mut values = Vec::new();
        values
            .try_reserve(self.indexes.references.len())
            .map_err(|_| WorkspaceError::Host(Arc::from("reference query allocation failed")))?;
        values.extend(
            self.indexes
                .references
                .iter()
                .filter(|edge| edge.target == entity)
                .copied(),
        );
        values.sort_by_key(|edge| (edge.site, edge.target));
        #[cfg(test)]
        record_query_measurement(
            self.indexes.references.len(),
            values.len(),
            values.len(),
            0,
            0,
        );
        page(self, query, request, continuation, &values)
    }

    pub fn callers_of(
        &self,
        revision: RevisionId,
        entity: EntityId,
        request: PageRequest,
        continuation: Option<&Continuation>,
    ) -> Result<QueryPage<CallEdge>, WorkspaceError> {
        self.call_page(revision, entity, true, request, continuation)
    }

    pub fn callees_of(
        &self,
        revision: RevisionId,
        entity: EntityId,
        request: PageRequest,
        continuation: Option<&Continuation>,
    ) -> Result<QueryPage<CallEdge>, WorkspaceError> {
        self.call_page(revision, entity, false, request, continuation)
    }

    pub fn node_type(
        &self,
        revision: RevisionId,
        node: NodeId,
    ) -> Result<NodeTypeFacts, WorkspaceError> {
        self.check_query_revision(revision)?;
        self.workspace_node(node)?;
        let index = self
            .indexes
            .node_lookup
            .get(&node)
            .copied()
            .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("node type")))?;
        let actual = self
            .indexes
            .node_actual_types
            .get(index)
            .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("node type")))?;
        let expected = self
            .indexes
            .node_expected_types
            .get(index)
            .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("node expectation")))?;
        let context = self
            .indexes
            .node_enclosing_entities
            .get(index)
            .copied()
            .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("node type context")))?;
        Ok(NodeTypeFacts {
            revision,
            node,
            actual: super::types::view(&self.program, &self.indexes, actual, Some(context))?,
            expected: expected
                .as_ref()
                .map(|ty| super::types::view(&self.program, &self.indexes, ty, Some(context)))
                .transpose()?,
        })
    }

    pub fn node_semantics(
        &self,
        revision: RevisionId,
        node: NodeId,
    ) -> Result<NodeSemanticFacts, WorkspaceError> {
        self.check_query_revision(revision)?;
        let header = self.workspace_node(node)?;
        let index = self
            .indexes
            .node_lookup
            .get(&node)
            .copied()
            .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("node semantics")))?;
        let operation = *self
            .indexes
            .node_operations
            .get(index)
            .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("node operation")))?;
        let effects = *self
            .indexes
            .node_effects
            .get(index)
            .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("node effects")))?;
        let types = self.node_type(revision, node)?;
        Ok(NodeSemanticFacts {
            revision,
            node,
            kind: header.kind,
            actual: types.actual,
            expected: types.expected,
            operation,
            effects: EffectSummary(effects.bits()),
        })
    }

    pub fn match_view(
        &self,
        revision: RevisionId,
        site: NodeId,
    ) -> Result<MatchView, WorkspaceError> {
        self.check_query_revision(revision)?;
        let header = self.workspace_node(site)?;
        if header.kind != super::NodeKind::Match {
            return Err(WorkspaceError::WrongEntityKind {
                operation: Arc::from("match query"),
                expected: Arc::from("match node"),
                actual: super::error::SemanticKind::Node(header.kind),
            });
        }
        let node_index = self
            .indexes
            .node_lookup
            .get(&site)
            .copied()
            .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("match node")))?;
        let context = self
            .indexes
            .node_enclosing_entities
            .get(node_index)
            .copied()
            .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("match type context")))?;
        let plan_id = self
            .indexes
            .node_match_plans
            .get(node_index)
            .copied()
            .flatten()
            .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("match plan index")))?;
        let plan = self
            .program
            .match_plans
            .get(host_index(plan_id.raw(), "match plan")?)
            .filter(|item| item.id == plan_id)
            .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("match plan")))?;
        let children = self
            .indexes
            .node_children
            .get(&site)
            .map(Vec::as_slice)
            .unwrap_or_default();
        if children.len() != plan.arms.len() + 1 {
            return Err(WorkspaceError::Validation(Arc::from(
                "match node children are inconsistent with its plan",
            )));
        }
        let scrutinee = children[0];
        let mut arms = Vec::new();
        arms.try_reserve(plan.arms.len())
            .map_err(|_| WorkspaceError::Host(Arc::from("match view allocation failed")))?;
        for (index, arm) in plan.arms.iter().enumerate() {
            let (patterns, pattern_root) = pattern_view(self, &arm.pattern, context)?;
            arms.push(MatchArmView {
                pattern_root,
                patterns,
                body: children[index + 1],
                result: super::types::view(
                    &self.program,
                    &self.indexes,
                    &arm.body_type,
                    Some(context),
                )?,
            });
        }
        Ok(MatchView {
            revision,
            site,
            scrutinee,
            result: super::types::view(
                &self.program,
                &self.indexes,
                &plan.result_type,
                Some(context),
            )?,
            arms,
            exhaustive: plan.exhaustive,
        })
    }

    pub fn entity_type(
        &self,
        revision: RevisionId,
        entity: EntityId,
    ) -> Result<EntityTypeFacts, WorkspaceError> {
        self.check_query_revision(revision)?;
        self.workspace_entity(entity)?;
        let context = type_context_for_entity(self, entity)?;
        let index = self
            .indexes
            .entity_lookup
            .get(&entity)
            .copied()
            .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("entity type")))?;
        let declared = self
            .indexes
            .entity_types
            .get(index)
            .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("entity type")))?
            .as_ref()
            .map(|ty| super::types::view(&self.program, &self.indexes, ty, context))
            .transpose()?;
        Ok(EntityTypeFacts {
            revision,
            entity,
            declared,
        })
    }

    pub fn function_signature(
        &self,
        revision: RevisionId,
        function: EntityId,
    ) -> Result<FunctionSignatureView, WorkspaceError> {
        self.check_query_revision(revision)?;
        let header = self.workspace_entity(function)?;
        let (parameter_bindings, parameter_types, result, bounds, type_parameter_names) =
            if header.kind == EntityKind::Main {
                let main = self
                    .program
                    .main
                    .as_ref()
                    .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("main")))?;
                (
                    &main.params,
                    &main.param_types,
                    &main.return_type,
                    &[][..],
                    &[][..],
                )
            } else {
                if header.kind != EntityKind::Function {
                    return Err(WorkspaceError::WrongEntityKind {
                        operation: Arc::from("function signature query"),
                        expected: Arc::from("function or main"),
                        actual: super::error::SemanticKind::Entity(header.kind),
                    });
                }
                let raw = binding_address(self, function)?;
                let binding = self
                    .program
                    .bindings
                    .get(host_index(raw, "function")?)
                    .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("function")))?;
                let (type_parameter_names, signature) = match &binding.ty {
                    crate::Type::Forall { vars, body } => (vars.as_slice(), body.as_ref()),
                    other => (&[][..], other),
                };
                let crate::Type::Fn { params, ret } = signature else {
                    return Err(WorkspaceError::Validation(Arc::from(
                        "function binding lost its function signature",
                    )));
                };
                let definition = self
                    .program
                    .functions
                    .iter()
                    .find(|definition| definition.binding.raw() == raw)
                    .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("function")))?;
                (
                    &definition.params,
                    params,
                    ret.as_ref(),
                    definition.bounds.as_slice(),
                    type_parameter_names,
                )
            };

        let type_parameter_count = self
            .indexes
            .entities
            .iter()
            .filter(|entity| {
                entity.kind == EntityKind::TypeParameter && entity.owner == Some(function)
            })
            .count();
        let mut type_parameter_headers = Vec::new();
        type_parameter_headers
            .try_reserve(type_parameter_count)
            .map_err(|_| {
                WorkspaceError::Host(Arc::from("signature binder index allocation failed"))
            })?;
        for entity in &self.indexes.entities {
            if entity.kind == EntityKind::TypeParameter && entity.owner == Some(function) {
                type_parameter_headers.push(entity);
            }
        }
        type_parameter_headers.sort_unstable_by_key(|entity| {
            self.indexes
                .entity_lookup
                .get(&entity.id)
                .and_then(|index| self.indexes.entity_addresses.get(*index))
                .and_then(|address| match address {
                    super::model::EntityAddress::FunctionTypeParameter { ordinal, .. }
                    | super::model::EntityAddress::EnumTypeParameter { ordinal, .. } => {
                        Some(*ordinal)
                    }
                    _ => None,
                })
                .unwrap_or(u64::MAX)
        });
        if type_parameter_headers.len() != type_parameter_names.len()
            || type_parameter_headers
                .iter()
                .zip(type_parameter_names)
                .any(|(header, name)| header.name.as_ref() != name.as_str())
        {
            return Err(WorkspaceError::Validation(Arc::from(
                "function type-parameter identities are not canonical",
            )));
        }
        let mut type_parameters = Vec::new();
        type_parameters
            .try_reserve(type_parameter_headers.len())
            .map_err(|_| WorkspaceError::Host(Arc::from("signature binder allocation failed")))?;
        for parameter in type_parameter_headers {
            let mut parameter_bounds = Vec::new();
            parameter_bounds.try_reserve(bounds.len()).map_err(|_| {
                WorkspaceError::Host(Arc::from("signature bound allocation failed"))
            })?;
            for bound in bounds
                .iter()
                .filter(|bound| bound.parameter == parameter.name.as_ref())
            {
                parameter_bounds.push(TypeParameterBoundView {
                    parameter: parameter.id,
                    trait_identity: super::types::semantic_trait(
                        &self.program,
                        &self.indexes,
                        bound.trait_id,
                    )?,
                });
            }
            type_parameters.push(TypeParameterView {
                id: parameter.id,
                name: Arc::clone(&parameter.name),
                owner: function,
                bounds: parameter_bounds,
            });
        }

        if parameter_bindings.len() != parameter_types.len() {
            return Err(WorkspaceError::Validation(Arc::from(
                "function value-parameter metadata is not canonical",
            )));
        }
        let mut parameters = Vec::new();
        parameters.try_reserve(parameter_types.len()).map_err(|_| {
            WorkspaceError::Host(Arc::from("signature parameter allocation failed"))
        })?;
        for (binding, ty) in parameter_bindings.iter().zip(parameter_types) {
            let entity = self
                .indexes
                .address_entities
                .get(&super::model::EntityAddress::Binding(binding.raw()))
                .copied()
                .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("value parameter")))?;
            let value = self.workspace_entity(entity)?;
            parameters.push(ValueParameterView {
                entity,
                name: Arc::clone(&value.name),
                ty: super::types::view(&self.program, &self.indexes, ty, Some(function))?,
            });
        }
        Ok(FunctionSignatureView {
            revision,
            function,
            type_parameters,
            parameters,
            result: super::types::view(&self.program, &self.indexes, result, Some(function))?,
        })
    }

    pub fn call_instantiation(
        &self,
        revision: RevisionId,
        site: NodeId,
    ) -> Result<CallInstantiationView, WorkspaceError> {
        self.check_query_revision(revision)?;
        let header = self.workspace_node(site)?;
        if header.kind != super::NodeKind::Call {
            return Err(WorkspaceError::WrongEntityKind {
                operation: Arc::from("call instantiation query"),
                expected: Arc::from("call node"),
                actual: super::error::SemanticKind::Node(header.kind),
            });
        }
        let index = self
            .indexes
            .node_lookup
            .get(&site)
            .copied()
            .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("call node")))?;
        let address = *self
            .indexes
            .node_addresses
            .get(index)
            .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("call node address")))?;
        let caller = *self
            .indexes
            .node_enclosing_entities
            .get(index)
            .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("call context")))?;
        let expression = expression_at(self, address)?;
        let crate::hir::ExprKind::Call {
            callee: callee_ref,
            instantiation,
            ..
        } = &expression.kind
        else {
            return Err(WorkspaceError::Validation(Arc::from(
                "call node does not contain a call expression",
            )));
        };
        let callee = self
            .indexes
            .address_entities
            .get(&super::model::EntityAddress::Binding(
                callee_ref.binding.raw(),
            ))
            .copied()
            .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("call target")))?;
        let binding = self
            .program
            .binding(callee_ref.binding)
            .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("call target")))?;
        let definition = self
            .program
            .functions
            .iter()
            .find(|function| function.binding == callee_ref.binding)
            .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("call target")))?;
        let (variables, signature) = match &binding.ty {
            crate::Type::Forall { vars, body } => (vars.as_slice(), body.as_ref()),
            other => (&[][..], other),
        };
        let crate::Type::Fn { params, .. } = signature else {
            return Err(WorkspaceError::Validation(Arc::from(
                "call target lost its function signature",
            )));
        };
        if variables.is_empty() && instantiation.is_some() {
            return Err(WorkspaceError::Validation(Arc::from(
                "non-generic call contains instantiation metadata",
            )));
        }
        if !variables.is_empty() && instantiation.is_none() {
            return Err(WorkspaceError::Validation(Arc::from(
                "generic call is missing instantiation metadata",
            )));
        }
        let substitutions = instantiation
            .as_ref()
            .map(|value| value.substitutions.as_slice())
            .unwrap_or_default();
        if substitutions.len() != variables.len()
            || substitutions
                .iter()
                .zip(variables)
                .any(|(substitution, variable)| substitution.parameter != *variable)
        {
            return Err(WorkspaceError::Validation(Arc::from(
                "call substitutions are not canonical",
            )));
        }
        let mut type_arguments = Vec::new();
        type_arguments
            .try_reserve(substitutions.len())
            .map_err(|_| {
                WorkspaceError::Host(Arc::from("call type-argument query allocation failed"))
            })?;
        for substitution in substitutions {
            let parameter = self
                .indexes
                .type_parameter_entities
                .get(&callee)
                .and_then(|parameters| parameters.get(substitution.parameter.as_str()))
                .copied()
                .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("type parameter")))?;
            type_arguments.push(TypeArgumentView {
                parameter,
                argument: super::types::view(
                    &self.program,
                    &self.indexes,
                    &substitution.ty,
                    Some(caller),
                )?,
            });
        }
        let mut substitution_map = std::collections::HashMap::new();
        substitution_map
            .try_reserve(substitutions.len())
            .map_err(|_| {
                WorkspaceError::Host(Arc::from("call substitution map allocation failed"))
            })?;
        for substitution in substitutions {
            substitution_map.insert(substitution.parameter.as_str(), &substitution.ty);
        }
        let mut parameters = Vec::new();
        parameters.try_reserve(params.len()).map_err(|_| {
            WorkspaceError::Host(Arc::from("call parameter query allocation failed"))
        })?;
        for parameter in params {
            let instantiated = crate::generic_call::substitute_type(parameter, &substitution_map)
                .map_err(generic_query_error)?;
            parameters.push(super::types::view(
                &self.program,
                &self.indexes,
                &instantiated,
                Some(caller),
            )?);
        }
        let witness_values = instantiation
            .as_ref()
            .map(|value| value.witnesses.as_slice())
            .unwrap_or_default();
        if witness_values.len() != definition.bounds.len() {
            return Err(WorkspaceError::Validation(Arc::from(
                "call witness metadata does not match declared bounds",
            )));
        }
        let mut witnesses = Vec::new();
        witnesses
            .try_reserve(witness_values.len())
            .map_err(|_| WorkspaceError::Host(Arc::from("call witness query allocation failed")))?;
        for (bound, witness) in definition.bounds.iter().zip(witness_values) {
            let parameter = self
                .indexes
                .type_parameter_entities
                .get(&callee)
                .and_then(|parameters| parameters.get(bound.parameter.as_str()))
                .copied()
                .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("type parameter")))?;
            let kind = match witness.kind {
                crate::hir::TraitWitnessKind::AutoTrait => TraitWitnessKindView::AutoTrait,
                crate::hir::TraitWitnessKind::Explicit(implementation) => {
                    let entity = self
                        .indexes
                        .address_entities
                        .get(&super::model::EntityAddress::Implementation(
                            implementation.raw(),
                        ))
                        .copied()
                        .ok_or_else(|| {
                            WorkspaceError::StaleIdentity(Arc::from("implementation witness"))
                        })?;
                    TraitWitnessKindView::Explicit(entity)
                }
            };
            witnesses.push(TraitWitnessView {
                parameter,
                trait_identity: super::types::semantic_trait(
                    &self.program,
                    &self.indexes,
                    witness.trait_id,
                )?,
                ty: super::types::view(&self.program, &self.indexes, &witness.ty, Some(caller))?,
                kind,
            });
        }
        Ok(CallInstantiationView {
            revision,
            site,
            callee,
            type_arguments,
            parameters,
            result: super::types::view(&self.program, &self.indexes, &expression.ty, Some(caller))?,
            witnesses,
            effects: EffectSummary(expression.effects.bits()),
        })
    }

    pub fn diagnostic_page(
        &self,
        revision: RevisionId,
        request: PageRequest,
        continuation: Option<&Continuation>,
    ) -> Result<QueryPage<DiagnosticHeader>, WorkspaceError> {
        self.check_query_revision(revision)?;
        let query = query_key(b"diagnostics", &[])?;
        page(self, query, request, continuation, &self.diagnostics)
    }

    pub fn hole_context(
        &self,
        revision: RevisionId,
        hole: HoleId,
    ) -> Result<HoleState, WorkspaceError> {
        self.check_query_revision(revision)?;
        if hole.0.namespace() != self.namespace {
            return Err(WorkspaceError::ForeignNamespace(Arc::from("hole")));
        }
        self.holes
            .iter()
            .find(|record| record.state.id == hole)
            .map(|record| record.state.clone())
            .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("hole")))
    }

    pub fn unresolved_value_reference(
        &self,
        revision: RevisionId,
        reference: UnresolvedValueReferenceId,
    ) -> Result<UnresolvedValueReferenceState, WorkspaceError> {
        self.check_query_revision(revision)?;
        let header = self.workspace_node(reference.0)?;
        if header.kind != super::NodeKind::UnresolvedValueReference {
            return Err(WorkspaceError::WrongEntityKind {
                operation: Arc::from("unresolved-value-reference"),
                expected: Arc::from("unresolved value-reference node"),
                actual: SemanticKind::Node(header.kind),
            });
        }
        self.unresolved_value_references
            .iter()
            .find(|record| record.state.id == reference)
            .map(|record| record.state.clone())
            .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("unresolved value reference")))
    }

    pub fn unresolved_value_reference_candidates(
        &self,
        revision: RevisionId,
        reference: UnresolvedValueReferenceId,
        request: PageRequest,
        continuation: Option<&Continuation>,
    ) -> Result<QueryPage<ValueReferenceCandidate>, WorkspaceError> {
        let state = self.unresolved_value_reference(revision, reference)?;
        let record = self
            .unresolved_value_references
            .iter()
            .find(|record| record.state.id == reference)
            .ok_or_else(|| {
                WorkspaceError::StaleIdentity(Arc::from("unresolved value reference"))
            })?;
        let visible_entities =
            super::transaction::visible_entities_in(&self.program, &self.indexes, record.address)?;
        let mut values = Vec::new();
        values.try_reserve(visible_entities.len()).map_err(|_| {
            WorkspaceError::Host(Arc::from(
                "unresolved value-reference candidate allocation failed",
            ))
        })?;
        for entity in visible_entities.iter().copied() {
            let header = self.workspace_entity(entity)?;
            if !matches!(
                header.kind,
                EntityKind::Parameter
                    | EntityKind::ImmutableLocal
                    | EntityKind::StaticBytesLocal
                    | EntityKind::MutableLocal
            ) {
                continue;
            }
            let address = self
                .indexes
                .entity_lookup
                .get(&entity)
                .and_then(|index| self.indexes.entity_addresses.get(*index))
                .copied()
                .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("candidate entity")))?;
            let super::model::EntityAddress::Binding(raw) = address else {
                continue;
            };
            let binding =
                self.program
                    .bindings
                    .get(usize::try_from(raw).map_err(|_| {
                        WorkspaceError::StaleIdentity(Arc::from("candidate binding"))
                    })?)
                    .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("candidate binding")))?;
            if !crate::ownership::draft_parameter_load_is_supported(&binding.ty)
                || !crate::generic_call::types_assignable(&binding.ty, &record.expected_internal)
                    .map_err(generic_query_error)?
            {
                continue;
            }
            values.push(ValueReferenceCandidate {
                entity,
                name: Arc::clone(&header.name),
                kind: header.kind,
                declared_type: super::types::view(
                    &self.program,
                    &self.indexes,
                    &binding.ty,
                    Some(state.owner),
                )?,
                exact_name_match: header.name.as_ref() == state.requested_name.as_ref(),
                status: ValueReferenceCandidateStatus::RequiresCanonicalValidation,
            });
        }
        #[cfg(test)]
        let materialized = values.len();
        values.sort_by(|left, right| {
            right
                .exact_name_match
                .cmp(&left.exact_name_match)
                .then_with(|| left.name.cmp(&right.name))
                .then_with(|| left.entity.cmp(&right.entity))
        });
        #[cfg(test)]
        record_query_measurement(visible_entities.len(), materialized, materialized, 0, 0);
        let query = id_query_key(b"unresolved-value-reference-candidates", reference.0)?;
        page(self, query, request, continuation, &values)
    }

    pub fn legal_constructors(
        &self,
        revision: RevisionId,
        hole: HoleId,
        request: PageRequest,
        continuation: Option<&Continuation>,
    ) -> Result<QueryPage<LegalConstructor>, WorkspaceError> {
        let context = self.hole_context(revision, hole)?;
        let record = self
            .holes
            .iter()
            .find(|record| record.state.id == hole)
            .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("hole")))?;
        let expected_type = &record.expected_internal;
        let control = expression_root(self, record.address.root)?
            .try_control_context(record.address.preorder)
            .map_err(WorkspaceError::from_core)?
            .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("hole address")))?;
        let expected_view = super::types::view(
            &self.program,
            &self.indexes,
            expected_type,
            Some(context.owner),
        )?;
        let mut values = Vec::new();
        push_legal_constructor(&mut values, LegalConstructor::If)?;
        push_legal_constructor(&mut values, LegalConstructor::Sequence)?;
        push_legal_constructor(&mut values, LegalConstructor::MutableLocal)?;
        if !expected_type.contains_never() {
            push_legal_constructor(
                &mut values,
                LegalConstructor::Loop {
                    result_type: expected_view,
                },
            )?;
        }
        let callable_result = self.function_signature(revision, context.owner)?.result;
        if callable_result != SemanticType::Never && control.divergent_replacement_is_admissible {
            push_legal_constructor(&mut values, LegalConstructor::Return)?;
        }
        if control.divergent_replacement_is_admissible {
            if let Some(target) = control.enclosing_loop {
                push_legal_constructor(
                    &mut values,
                    LegalConstructor::Break {
                        value_type: super::types::view(
                            &self.program,
                            &self.indexes,
                            &target.result_type,
                            Some(context.owner),
                        )?,
                    },
                )?;
                push_legal_constructor(&mut values, LegalConstructor::Continue)?;
            }
        }
        if *expected_type == crate::Type::Unit {
            push_legal_constructor(&mut values, LegalConstructor::While)?;
        }
        match expected_type {
            crate::Type::I64 => {
                push_legal_constructor(&mut values, LegalConstructor::I64Literal)?;
            }
            crate::Type::F64 => {
                push_legal_constructor(&mut values, LegalConstructor::F64Literal)?;
            }
            crate::Type::Bool => {
                push_legal_constructor(&mut values, LegalConstructor::BoolLiteral)?;
            }
            crate::Type::Unit => {
                push_legal_constructor(&mut values, LegalConstructor::UnitLiteral)?;
            }
            _ => {}
        }
        for operation in super::draft::SOURCE_FREE_OPERATIONS.iter().copied() {
            if operation_matches_expected(operation, expected_type) {
                push_legal_constructor(&mut values, LegalConstructor::Operation(operation))?;
            }
        }
        for entity in context.visible_entities.iter().copied() {
            let header = self.workspace_entity(entity)?;
            let address = self
                .indexes
                .entity_lookup
                .get(&entity)
                .and_then(|index| self.indexes.entity_addresses.get(*index))
                .copied()
                .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("entity")))?;
            let super::model::EntityAddress::Binding(raw) = address else {
                continue;
            };
            let binding = self
                .program
                .bindings
                .get(
                    usize::try_from(raw)
                        .map_err(|_| WorkspaceError::StaleIdentity(Arc::from("entity")))?,
                )
                .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("entity")))?;
            match header.kind {
                EntityKind::Parameter
                | EntityKind::ImmutableLocal
                | EntityKind::StaticBytesLocal
                | EntityKind::MutableLocal => {
                    let assignable =
                        crate::generic_call::types_assignable(&binding.ty, expected_type)
                            .map_err(generic_query_error)?;
                    if crate::ownership::draft_parameter_load_is_supported(&binding.ty)
                        && assignable
                    {
                        push_legal_constructor(&mut values, LegalConstructor::Load(entity))?;
                    }
                    if matches!(
                        binding.ty,
                        crate::Type::Bytes | crate::Type::ByteVector | crate::Type::Resource(_)
                    ) && assignable
                    {
                        push_legal_constructor(
                            &mut values,
                            LegalConstructor::Move {
                                binding: entity,
                                status: ConstructorStatus::RequiresOwnershipValidation,
                            },
                        )?;
                    }
                    if binding.ty == crate::Type::ByteVector
                        && *expected_type == crate::Type::ByteSlice
                    {
                        push_legal_constructor(
                            &mut values,
                            LegalConstructor::BorrowShared {
                                binding: entity,
                                status: ConstructorStatus::RequiresOwnershipValidation,
                            },
                        )?;
                    }
                    if header.kind == EntityKind::MutableLocal
                        && *expected_type == crate::Type::Unit
                    {
                        push_legal_constructor(&mut values, LegalConstructor::SetLocal(entity))?;
                    }
                }
                EntityKind::Function => match &binding.ty {
                    crate::Type::Fn { ret, .. } => {
                        if crate::generic_call::types_assignable(ret, expected_type)
                            .map_err(generic_query_error)?
                        {
                            push_legal_constructor(&mut values, LegalConstructor::Call(entity))?;
                        }
                    }
                    crate::Type::Forall { body, .. }
                        if matches!(body.as_ref(), crate::Type::Fn { .. }) =>
                    {
                        push_legal_constructor(&mut values, LegalConstructor::Call(entity))?;
                    }
                    _ => {}
                },
                _ => {}
            }
        }
        match expected_type {
            crate::Type::Product(name) => {
                if let Some((index, _)) = self
                    .program
                    .products
                    .iter()
                    .enumerate()
                    .find(|(_, product)| product.name == *name)
                {
                    let raw = u64::try_from(index).map_err(|_| {
                        WorkspaceError::Host(Arc::from("product constructor index exceeds u64"))
                    })?;
                    if let Some(entity) = self
                        .indexes
                        .address_entities
                        .get(&super::model::EntityAddress::Product(raw))
                        .copied()
                    {
                        push_legal_constructor(&mut values, LegalConstructor::Product(entity))?;
                    }
                }
            }
            crate::Type::Enum { id, arguments, .. } if arguments.is_empty() => {
                if let Some((index, definition)) = self
                    .program
                    .enums
                    .iter()
                    .enumerate()
                    .find(|(_, definition)| definition.id == *id)
                    .filter(|(_, definition)| definition.type_parameters.is_empty())
                {
                    let raw = u64::try_from(index).map_err(|_| {
                        WorkspaceError::Host(Arc::from("enum constructor index exceeds u64"))
                    })?;
                    for (variant, _) in definition.variants.iter().enumerate() {
                        let variant = u64::try_from(variant).map_err(|_| {
                            WorkspaceError::Host(Arc::from("enum variant index exceeds u64"))
                        })?;
                        if let Some(entity) = self
                            .indexes
                            .address_entities
                            .get(&super::model::EntityAddress::EnumVariant {
                                enumeration: raw,
                                variant,
                            })
                            .copied()
                        {
                            push_legal_constructor(
                                &mut values,
                                LegalConstructor::EnumVariant(entity),
                            )?;
                        }
                    }
                }
            }
            _ => {}
        }
        #[cfg(test)]
        let materialized = values.len();
        values.sort_unstable_by(compare_legal_constructors);
        values.dedup();
        #[cfg(test)]
        record_query_measurement(
            context.visible_entities.len(),
            materialized,
            materialized,
            0,
            0,
        );
        let query = id_query_key(b"legal-constructors", hole.0)?;
        page(self, query, request, continuation, &values)
    }

    fn call_page(
        &self,
        revision: RevisionId,
        entity: EntityId,
        callers: bool,
        request: PageRequest,
        continuation: Option<&Continuation>,
    ) -> Result<QueryPage<CallEdge>, WorkspaceError> {
        self.check_query_revision(revision)?;
        self.workspace_entity(entity)?;
        let domain: &[u8] = if callers { b"callers" } else { b"callees" };
        let query = id_query_key(domain, entity)?;
        let mut values = Vec::new();
        values
            .try_reserve(self.indexes.calls.len())
            .map_err(|_| WorkspaceError::Host(Arc::from("call query allocation failed")))?;
        values.extend(
            self.indexes
                .calls
                .iter()
                .filter(|edge| {
                    if callers {
                        edge.callee == entity
                    } else {
                        edge.caller == entity
                    }
                })
                .copied(),
        );
        values.sort_by_key(|edge| (edge.caller, edge.callee, edge.site));
        page(self, query, request, continuation, &values)
    }

    pub(super) fn check_query_revision(&self, revision: RevisionId) -> Result<(), WorkspaceError> {
        if revision.namespace() != self.namespace {
            return Err(WorkspaceError::ForeignNamespace(Arc::from("revision")));
        }
        if revision != self.revision {
            return Err(WorkspaceError::StaleRevision);
        }
        Ok(())
    }

    pub(super) fn workspace_entity(&self, id: EntityId) -> Result<&EntityHeader, WorkspaceError> {
        if id.namespace() != self.namespace {
            return Err(WorkspaceError::ForeignNamespace(Arc::from("entity")));
        }
        self.indexes
            .entity(self.namespace, id)
            .map_err(|_| WorkspaceError::StaleIdentity(Arc::from("entity")))
    }

    pub(super) fn workspace_node(&self, id: NodeId) -> Result<&super::NodeHeader, WorkspaceError> {
        if id.namespace() != self.namespace {
            return Err(WorkspaceError::ForeignNamespace(Arc::from("node")));
        }
        self.indexes
            .node(self.namespace, id)
            .map_err(|_| WorkspaceError::StaleIdentity(Arc::from("node")))
    }
}

fn expression_at(
    snapshot: &WorkspaceSnapshot,
    address: super::model::NodeAddress,
) -> Result<&crate::hir::Expr, WorkspaceError> {
    expression_root(snapshot, address.root)?
        .try_at_preorder(address.preorder)
        .map_err(WorkspaceError::from_core)?
        .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("node address")))
}

fn expression_root(
    snapshot: &WorkspaceSnapshot,
    root: super::model::EntityAddress,
) -> Result<&crate::hir::Expr, WorkspaceError> {
    if root == super::model::EntityAddress::Main {
        snapshot
            .program
            .main
            .as_ref()
            .map(|main| &main.body)
            .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("main root")))
    } else {
        let super::model::EntityAddress::Binding(raw) = root else {
            return Err(WorkspaceError::StaleIdentity(Arc::from("node root")));
        };
        snapshot
            .program
            .functions
            .iter()
            .find(|function| function.binding.raw() == raw)
            .map(|function| &function.body)
            .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("function root")))
    }
}

fn binding_address(snapshot: &WorkspaceSnapshot, entity: EntityId) -> Result<u64, WorkspaceError> {
    let address = snapshot
        .indexes
        .entity_lookup
        .get(&entity)
        .and_then(|index| snapshot.indexes.entity_addresses.get(*index))
        .copied()
        .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("function")))?;
    let super::model::EntityAddress::Binding(raw) = address else {
        return Err(WorkspaceError::StaleIdentity(Arc::from("function")));
    };
    Ok(raw)
}

fn type_context_for_entity(
    snapshot: &WorkspaceSnapshot,
    entity: EntityId,
) -> Result<Option<EntityId>, WorkspaceError> {
    let mut current = Some(entity);
    while let Some(id) = current {
        let header = snapshot.workspace_entity(id)?;
        if matches!(
            header.kind,
            EntityKind::Function | EntityKind::Main | EntityKind::Enum
        ) {
            return Ok(Some(id));
        }
        current = header.owner;
    }
    Ok(None)
}

fn pattern_view(
    snapshot: &WorkspaceSnapshot,
    root: &crate::hir::MatchPattern,
    context: EntityId,
) -> Result<(Vec<MatchPatternNodeView>, MatchPatternLabel), WorkspaceError> {
    enum Work<'a> {
        Visit(&'a crate::hir::MatchPattern),
        Variant {
            pattern: &'a crate::hir::MatchPattern,
            enumeration: crate::hir::EnumId,
            variant: crate::hir::VariantId,
            fields: &'a [crate::hir::MatchFieldPattern],
        },
        Product {
            pattern: &'a crate::hir::MatchPattern,
            product: lkjscript_core::ProductId,
            fields: &'a [crate::hir::MatchFieldPattern],
        },
    }
    let mut work = Vec::new();
    work.try_reserve(1)
        .map_err(|_| WorkspaceError::Host(Arc::from("match pattern view allocation failed")))?;
    work.push(Work::Visit(root));
    let mut completed = Vec::new();
    let mut output = Vec::new();
    while let Some(item) = work.pop() {
        match item {
            Work::Visit(pattern) => match pattern {
                crate::hir::MatchPattern::Variant {
                    enum_id,
                    variant,
                    fields,
                    ..
                } => {
                    work.try_reserve(fields.len().checked_add(1).ok_or_else(|| {
                        WorkspaceError::Host(Arc::from("match pattern work overflow"))
                    })?)
                    .map_err(|_| {
                        WorkspaceError::Host(Arc::from("match pattern work allocation failed"))
                    })?;
                    work.push(Work::Variant {
                        pattern,
                        enumeration: *enum_id,
                        variant: *variant,
                        fields,
                    });
                    work.extend(fields.iter().rev().map(|field| Work::Visit(&field.pattern)));
                }
                crate::hir::MatchPattern::Product {
                    product, fields, ..
                } => {
                    work.try_reserve(fields.len().checked_add(1).ok_or_else(|| {
                        WorkspaceError::Host(Arc::from("match pattern work overflow"))
                    })?)
                    .map_err(|_| {
                        WorkspaceError::Host(Arc::from("match pattern work allocation failed"))
                    })?;
                    work.push(Work::Product {
                        pattern,
                        product: *product,
                        fields,
                    });
                    work.extend(fields.iter().rev().map(|field| Work::Visit(&field.pattern)));
                }
                crate::hir::MatchPattern::Wildcard { .. }
                | crate::hir::MatchPattern::Binding { .. }
                | crate::hir::MatchPattern::Bool(_)
                | crate::hir::MatchPattern::I64(_) => {
                    let kind = match pattern {
                        crate::hir::MatchPattern::Wildcard { .. } => MatchPatternKindView::Wildcard,
                        crate::hir::MatchPattern::Binding { local } => {
                            let binding = snapshot
                                .indexes
                                .address_entities
                                .get(&super::model::EntityAddress::Binding(local.binding.raw()))
                                .copied()
                                .ok_or_else(|| {
                                    WorkspaceError::StaleIdentity(Arc::from(
                                        "match pattern binding",
                                    ))
                                })?;
                            MatchPatternKindView::Binding { binding }
                        }
                        crate::hir::MatchPattern::Bool(value) => MatchPatternKindView::Bool(*value),
                        crate::hir::MatchPattern::I64(value) => MatchPatternKindView::I64(*value),
                        _ => unreachable!("aggregate patterns handled above"),
                    };
                    push_pattern_view(
                        snapshot,
                        pattern,
                        kind,
                        context,
                        &mut output,
                        &mut completed,
                    )?;
                }
            },
            Work::Variant {
                pattern,
                enumeration,
                variant,
                fields,
            } => {
                let children = take_pattern_results(&mut completed, fields.len())?;
                let (enumeration_entity, variant_entity, field_entities) =
                    enum_pattern_entities(snapshot, enumeration, variant, fields)?;
                let mut field_views = Vec::new();
                field_views.try_reserve(fields.len()).map_err(|_| {
                    WorkspaceError::Host(Arc::from("match field view allocation failed"))
                })?;
                for (field, (entity, child)) in
                    fields.iter().zip(field_entities.into_iter().zip(children))
                {
                    let _ = field;
                    field_views.push(MatchPatternFieldView {
                        field: entity,
                        pattern: child,
                    });
                }
                push_pattern_view(
                    snapshot,
                    pattern,
                    MatchPatternKindView::EnumVariant {
                        enumeration: enumeration_entity,
                        variant: variant_entity,
                        fields: field_views,
                    },
                    context,
                    &mut output,
                    &mut completed,
                )?;
            }
            Work::Product {
                pattern,
                product,
                fields,
            } => {
                let children = take_pattern_results(&mut completed, fields.len())?;
                let raw = product.raw();
                let product_entity = snapshot
                    .indexes
                    .address_entities
                    .get(&super::model::EntityAddress::Product(raw))
                    .copied()
                    .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("match product")))?;
                let mut field_views = Vec::new();
                field_views.try_reserve(fields.len()).map_err(|_| {
                    WorkspaceError::Host(Arc::from("match field view allocation failed"))
                })?;
                for (field, child) in fields.iter().zip(children) {
                    let entity = snapshot
                        .indexes
                        .address_entities
                        .get(&super::model::EntityAddress::ProductField {
                            product: raw,
                            field: field.field_index,
                        })
                        .copied();
                    field_views.push(MatchPatternFieldView {
                        field: entity,
                        pattern: child,
                    });
                }
                push_pattern_view(
                    snapshot,
                    pattern,
                    MatchPatternKindView::Product {
                        product: product_entity,
                        fields: field_views,
                    },
                    context,
                    &mut output,
                    &mut completed,
                )?;
            }
        }
    }
    let root = completed
        .pop()
        .ok_or_else(|| WorkspaceError::Validation(Arc::from("match pattern root is missing")))?;
    if completed.is_empty() {
        Ok((output, root))
    } else {
        Err(WorkspaceError::Validation(Arc::from(
            "match pattern view left disconnected results",
        )))
    }
}

fn push_pattern_view(
    snapshot: &WorkspaceSnapshot,
    pattern: &crate::hir::MatchPattern,
    kind: MatchPatternKindView,
    context: EntityId,
    output: &mut Vec<MatchPatternNodeView>,
    completed: &mut Vec<MatchPatternLabel>,
) -> Result<(), WorkspaceError> {
    let ordinal = u64::try_from(output.len())
        .map_err(|_| WorkspaceError::Host(Arc::from("match pattern label exceeds u64")))?;
    output
        .try_reserve(1)
        .map_err(|_| WorkspaceError::Host(Arc::from("match pattern view allocation failed")))?;
    let label = MatchPatternLabel::new(ordinal);
    output.push(MatchPatternNodeView {
        label,
        ty: super::types::view(
            &snapshot.program,
            &snapshot.indexes,
            &pattern.ty(),
            Some(context),
        )?,
        kind,
    });
    completed
        .try_reserve(1)
        .map_err(|_| WorkspaceError::Host(Arc::from("match pattern result allocation failed")))?;
    completed.push(label);
    Ok(())
}

fn take_pattern_results(
    completed: &mut Vec<MatchPatternLabel>,
    count: usize,
) -> Result<Vec<MatchPatternLabel>, WorkspaceError> {
    let start = completed.len().checked_sub(count).ok_or_else(|| {
        WorkspaceError::Validation(Arc::from("match pattern child results are missing"))
    })?;
    let mut children = Vec::new();
    children
        .try_reserve(count)
        .map_err(|_| WorkspaceError::Host(Arc::from("match child result allocation failed")))?;
    children.extend_from_slice(&completed[start..]);
    completed.truncate(start);
    Ok(children)
}

type MatchEnumEntities = (Option<EntityId>, Option<EntityId>, Vec<Option<EntityId>>);

fn enum_pattern_entities(
    snapshot: &WorkspaceSnapshot,
    enumeration: crate::hir::EnumId,
    variant: crate::hir::VariantId,
    fields: &[crate::hir::MatchFieldPattern],
) -> Result<MatchEnumEntities, WorkspaceError> {
    let enum_index = snapshot
        .indexes
        .enum_identity_indices
        .get(&enumeration)
        .copied()
        .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("match enum")))?;
    let (variant_enum_index, variant_index) = snapshot
        .indexes
        .variant_identity_indices
        .get(&(enumeration, variant))
        .copied()
        .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("match enum variant")))?;
    if variant_enum_index != enum_index {
        return Err(WorkspaceError::Validation(Arc::from(
            "match variant query index is inconsistent",
        )));
    }
    let enumeration = u64::try_from(enum_index)
        .map_err(|_| WorkspaceError::Host(Arc::from("match enum index exceeds u64")))?;
    let variant = u64::try_from(variant_index)
        .map_err(|_| WorkspaceError::Host(Arc::from("match variant index exceeds u64")))?;
    let enumeration_entity = snapshot
        .indexes
        .address_entities
        .get(&super::model::EntityAddress::Enum(enumeration))
        .copied();
    let variant_entity = snapshot
        .indexes
        .address_entities
        .get(&super::model::EntityAddress::EnumVariant {
            enumeration,
            variant,
        })
        .copied();
    let mut field_entities = Vec::new();
    field_entities
        .try_reserve(fields.len())
        .map_err(|_| WorkspaceError::Host(Arc::from("match field allocation failed")))?;
    field_entities.extend(fields.iter().map(|field| {
        snapshot
            .indexes
            .address_entities
            .get(&super::model::EntityAddress::EnumField {
                enumeration,
                variant,
                field: field.field_index,
            })
            .copied()
    }));
    Ok((enumeration_entity, variant_entity, field_entities))
}

fn compare_legal_constructors(
    left: &LegalConstructor,
    right: &LegalConstructor,
) -> std::cmp::Ordering {
    let rank = |constructor: &LegalConstructor| match constructor {
        LegalConstructor::I64Literal => 0_u8,
        LegalConstructor::F64Literal => 1,
        LegalConstructor::BoolLiteral => 2,
        LegalConstructor::UnitLiteral => 3,
        LegalConstructor::Operation(_) => 4,
        LegalConstructor::Load(_) => 5,
        LegalConstructor::Move { .. } => 6,
        LegalConstructor::BorrowShared { .. } => 7,
        LegalConstructor::Call(_) => 8,
        LegalConstructor::Product(_) => 9,
        LegalConstructor::EnumVariant(_) => 10,
        LegalConstructor::If => 11,
        LegalConstructor::Sequence => 12,
        LegalConstructor::MutableLocal => 13,
        LegalConstructor::SetLocal(_) => 14,
        LegalConstructor::While => 15,
        LegalConstructor::Loop { .. } => 16,
        LegalConstructor::Break { .. } => 17,
        LegalConstructor::Continue => 18,
        LegalConstructor::Return => 19,
    };
    rank(left)
        .cmp(&rank(right))
        .then_with(|| match (left, right) {
            (LegalConstructor::Operation(left), LegalConstructor::Operation(right)) => {
                left.cmp(right)
            }
            (LegalConstructor::Load(left), LegalConstructor::Load(right))
            | (LegalConstructor::Call(left), LegalConstructor::Call(right))
            | (LegalConstructor::Product(left), LegalConstructor::Product(right))
            | (LegalConstructor::EnumVariant(left), LegalConstructor::EnumVariant(right))
            | (LegalConstructor::SetLocal(left), LegalConstructor::SetLocal(right)) => {
                left.cmp(right)
            }
            (
                LegalConstructor::Move {
                    binding: left_binding,
                    status: left_status,
                },
                LegalConstructor::Move {
                    binding: right_binding,
                    status: right_status,
                },
            )
            | (
                LegalConstructor::BorrowShared {
                    binding: left_binding,
                    status: left_status,
                },
                LegalConstructor::BorrowShared {
                    binding: right_binding,
                    status: right_status,
                },
            ) => (left_binding, left_status).cmp(&(right_binding, right_status)),
            _ => std::cmp::Ordering::Equal,
        })
}

fn push_legal_constructor(
    values: &mut Vec<LegalConstructor>,
    value: LegalConstructor,
) -> Result<(), WorkspaceError> {
    values
        .try_reserve(1)
        .map_err(|_| WorkspaceError::Host(Arc::from("legal constructor allocation failed")))?;
    values.push(value);
    Ok(())
}

fn operation_matches_expected(
    operation: crate::operation::Operation,
    expected: &crate::Type,
) -> bool {
    match operation {
        crate::operation::Operation::Add => matches!(expected, crate::Type::I64 | crate::Type::F64),
        crate::operation::Operation::Less => *expected == crate::Type::Bool,
        crate::operation::Operation::ByteVectorNew | crate::operation::Operation::ThawBytes => {
            *expected == crate::Type::ByteVector
        }
        crate::operation::Operation::ByteSliceLength
        | crate::operation::Operation::ByteSliceByteAt
        | crate::operation::Operation::BytesLength => *expected == crate::Type::I64,
        _ => false,
    }
}

fn generic_query_error(error: crate::generic_call::GenericCallError) -> WorkspaceError {
    match error {
        crate::generic_call::GenericCallError::Host(message) => {
            WorkspaceError::Host(Arc::from(message))
        }
        other => WorkspaceError::Validation(Arc::from(other.to_string())),
    }
}

fn host_index(raw: u64, subject: &str) -> Result<usize, WorkspaceError> {
    usize::try_from(raw).map_err(|_| WorkspaceError::StaleIdentity(Arc::from(subject.to_owned())))
}

fn page<T: Clone>(
    snapshot: &WorkspaceSnapshot,
    query: [u8; 32],
    request: PageRequest,
    continuation: Option<&Continuation>,
    values: &[T],
) -> Result<QueryPage<T>, WorkspaceError> {
    let start = match continuation {
        None => 0,
        Some(cursor) => {
            if cursor.namespace != snapshot.namespace {
                return Err(WorkspaceError::InvalidContinuation(Arc::from(
                    "continuation belongs to a different workspace namespace",
                )));
            }
            if cursor.revision != snapshot.revision {
                return Err(WorkspaceError::InvalidContinuation(Arc::from(
                    "continuation revision is stale",
                )));
            }
            if cursor.query != query {
                return Err(WorkspaceError::InvalidContinuation(Arc::from(
                    "continuation belongs to a different query",
                )));
            }
            usize::try_from(cursor.offset).map_err(|_| {
                WorkspaceError::InvalidContinuation(Arc::from(
                    "continuation offset is not host-addressable",
                ))
            })?
        }
    };
    if start > values.len() {
        return Err(WorkspaceError::InvalidContinuation(Arc::from(
            "continuation offset exceeds query result",
        )));
    }
    let end = start
        .checked_add(request.limit())
        .map_or(values.len(), |candidate| candidate.min(values.len()));
    let mut items = Vec::new();
    items
        .try_reserve(end - start)
        .map_err(|_| WorkspaceError::Host(Arc::from("query page allocation failed")))?;
    items.extend_from_slice(&values[start..end]);
    #[cfg(test)]
    record_query_measurement(0, 0, 0, items.len(), 1);
    let continuation = if end < values.len() {
        Some(Continuation {
            namespace: snapshot.namespace,
            revision: snapshot.revision,
            query,
            offset: u64::try_from(end)
                .map_err(|_| WorkspaceError::Host(Arc::from("query continuation exceeds u64")))?,
        })
    } else {
        None
    };
    Ok(QueryPage {
        revision: snapshot.revision,
        items,
        continuation,
    })
}

fn query_key(domain: &[u8], value: &[u8]) -> Result<[u8; 32], WorkspaceError> {
    let capacity = domain
        .len()
        .checked_add(value.len())
        .ok_or_else(|| WorkspaceError::Host(Arc::from("query identity size overflow")))?;
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(capacity)
        .map_err(|_| WorkspaceError::Host(Arc::from("query identity allocation failed")))?;
    bytes.extend_from_slice(domain);
    bytes.extend_from_slice(value);
    Ok(lkjscript_core::sha256(&bytes))
}

fn id_query_key(
    domain: &[u8],
    id: impl WorkspaceQueryId + Copy,
) -> Result<[u8; 32], WorkspaceError> {
    let capacity = domain
        .len()
        .checked_add(48)
        .ok_or_else(|| WorkspaceError::Host(Arc::from("identity query size overflow")))?;
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(capacity)
        .map_err(|_| WorkspaceError::Host(Arc::from("identity query allocation failed")))?;
    bytes.extend_from_slice(domain);
    bytes.extend_from_slice(&id.namespace().bytes());
    bytes.extend_from_slice(&id.slot().to_be_bytes());
    bytes.extend_from_slice(&id.generation().to_be_bytes());
    Ok(lkjscript_core::sha256(&bytes))
}

fn contains_ignoring_ascii_case(value: &str, needle: &str) -> bool {
    if needle.is_empty() {
        return true;
    }
    value
        .as_bytes()
        .windows(needle.len())
        .any(|window| window.eq_ignore_ascii_case(needle.as_bytes()))
}

trait WorkspaceQueryId {
    fn namespace(self) -> super::WorkspaceNamespace;
    fn slot(self) -> u64;
    fn generation(self) -> u64;
}

impl WorkspaceQueryId for EntityId {
    fn namespace(self) -> super::WorkspaceNamespace {
        self.namespace()
    }
    fn slot(self) -> u64 {
        self.slot()
    }
    fn generation(self) -> u64 {
        self.generation()
    }
}

impl WorkspaceQueryId for NodeId {
    fn namespace(self) -> super::WorkspaceNamespace {
        self.namespace()
    }
    fn slot(self) -> u64 {
        self.slot()
    }
    fn generation(self) -> u64 {
        self.generation()
    }
}
