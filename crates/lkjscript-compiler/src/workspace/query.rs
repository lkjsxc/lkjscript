use std::sync::Arc;

use super::{
    CallEdge, DiagnosticHeader, EntityHeader, EntityId, EntityKind, HoleId, HoleState, NodeId,
    ReferenceEdge, RevisionId, SemanticTypeView, WorkspaceError, WorkspaceSnapshot,
};

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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NodeTypeFacts {
    pub revision: RevisionId,
    pub node: NodeId,
    pub actual: Arc<str>,
    pub expected: Option<Arc<str>>,
    pub actual_semantic: SemanticTypeView,
    pub expected_semantic: Option<SemanticTypeView>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EntityTypeFacts {
    pub revision: RevisionId,
    pub entity: EntityId,
    pub declared: Option<SemanticTypeView>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FunctionSignatureView {
    pub revision: RevisionId,
    pub function: EntityId,
    pub parameters: Vec<SemanticTypeView>,
    pub result: SemanticTypeView,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MatchView {
    pub revision: RevisionId,
    pub site: NodeId,
    pub scrutinee: NodeId,
    pub result: SemanticTypeView,
    pub arms: Vec<MatchArmView>,
    pub exhaustive: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MatchArmView {
    pub id: u64,
    pub pattern_root: u64,
    pub patterns: Vec<MatchPatternNodeView>,
    pub body: NodeId,
    pub result: SemanticTypeView,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MatchPatternNodeView {
    /// Review-local dense pattern label; not a workspace edit identity.
    pub id: u64,
    pub ty: SemanticTypeView,
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
    pub pattern: u64,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ConstructorStatus {
    Established,
    RequiresOwnershipValidation,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[non_exhaustive]
pub enum LegalConstructor {
    I64Literal,
    F64Literal,
    BoolLiteral,
    UnitLiteral,
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
        let header = self.workspace_node(node)?;
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
        Ok(NodeTypeFacts {
            revision,
            node,
            actual: Arc::clone(&header.actual_type),
            expected: header.expected_type.clone(),
            actual_semantic: super::types::view(&self.program, &self.indexes, actual)?,
            expected_semantic: expected
                .as_ref()
                .map(|ty| super::types::view(&self.program, &self.indexes, ty))
                .transpose()?,
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
                actual: Arc::from(format!("{:?}", header.kind)),
            });
        }
        let node_index = self
            .indexes
            .node_lookup
            .get(&site)
            .copied()
            .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("match node")))?;
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
            let (patterns, pattern_root) = pattern_view(self, &arm.pattern)?;
            arms.push(MatchArmView {
                id: arm.id,
                pattern_root,
                patterns,
                body: children[index + 1],
                result: super::types::view(&self.program, &self.indexes, &arm.body_type)?,
            });
        }
        Ok(MatchView {
            revision,
            site,
            scrutinee,
            result: super::types::view(&self.program, &self.indexes, &plan.result_type)?,
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
            .map(|ty| super::types::view(&self.program, &self.indexes, ty))
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
        let (parameters, result) = if header.kind == EntityKind::Main {
            let main = self
                .program
                .main
                .as_ref()
                .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("main")))?;
            (&main.param_types, &main.return_type)
        } else {
            if header.kind != EntityKind::Function {
                return Err(WorkspaceError::WrongEntityKind {
                    operation: Arc::from("function signature query"),
                    expected: Arc::from("function or main"),
                    actual: Arc::from(format!("{:?}", header.kind)),
                });
            }
            let address = self
                .indexes
                .entity_lookup
                .get(&function)
                .and_then(|index| self.indexes.entity_addresses.get(*index))
                .copied()
                .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("function")))?;
            let super::model::EntityAddress::Binding(raw) = address else {
                return Err(WorkspaceError::StaleIdentity(Arc::from("function")));
            };
            let binding = self
                .program
                .bindings
                .get(
                    usize::try_from(raw)
                        .map_err(|_| WorkspaceError::StaleIdentity(Arc::from("function")))?,
                )
                .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("function")))?;
            let crate::Type::Fn { params, ret } = &binding.ty else {
                return Err(WorkspaceError::unsupported(
                    "function-signature",
                    "generic signatures are explicit unsupported query results in this vertical",
                ));
            };
            (params, ret.as_ref())
        };
        let mut parameter_views = Vec::new();
        parameter_views
            .try_reserve(parameters.len())
            .map_err(|_| WorkspaceError::Host(Arc::from("signature view allocation failed")))?;
        for parameter in parameters {
            parameter_views.push(super::types::view(&self.program, &self.indexes, parameter)?);
        }
        Ok(FunctionSignatureView {
            revision,
            function,
            parameters: parameter_views,
            result: super::types::view(&self.program, &self.indexes, result)?,
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
        page(
            self,
            query,
            request,
            continuation,
            &self.indexes.diagnostics,
        )
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

    pub fn legal_constructors(
        &self,
        revision: RevisionId,
        hole: HoleId,
        request: PageRequest,
        continuation: Option<&Continuation>,
    ) -> Result<QueryPage<LegalConstructor>, WorkspaceError> {
        let context = self.hole_context(revision, hole)?;
        let mut values = vec![LegalConstructor::If];
        match &context.expected_type {
            crate::Type::I64 => values.push(LegalConstructor::I64Literal),
            crate::Type::F64 => values.push(LegalConstructor::F64Literal),
            crate::Type::Bool => values.push(LegalConstructor::BoolLiteral),
            crate::Type::Unit => values.push(LegalConstructor::UnitLiteral),
            _ => {}
        }
        values
            .try_reserve(context.visible_entities.len())
            .map_err(|_| WorkspaceError::Host(Arc::from("legal constructor allocation failed")))?;
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
                | EntityKind::StaticBytesLocal => {
                    if crate::ownership::draft_parameter_load_is_supported(&binding.ty)
                        && crate::Type::unify_assignable(&binding.ty, &context.expected_type)
                    {
                        values.push(LegalConstructor::Load(entity));
                    }
                    if matches!(
                        binding.ty,
                        crate::Type::Bytes | crate::Type::ByteVector | crate::Type::Resource(_)
                    ) && crate::Type::unify_assignable(&binding.ty, &context.expected_type)
                    {
                        values.push(LegalConstructor::Move {
                            binding: entity,
                            status: ConstructorStatus::RequiresOwnershipValidation,
                        });
                    }
                    if binding.ty == crate::Type::ByteVector
                        && context.expected_type == crate::Type::ByteSlice
                    {
                        values.push(LegalConstructor::BorrowShared {
                            binding: entity,
                            status: ConstructorStatus::RequiresOwnershipValidation,
                        });
                    }
                }
                EntityKind::Function => {
                    if let crate::Type::Fn { ret, .. } = &binding.ty {
                        if crate::Type::unify_assignable(ret, &context.expected_type) {
                            values.push(LegalConstructor::Call(entity));
                        }
                    }
                }
                _ => {}
            }
        }
        match &context.expected_type {
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
                        values.push(LegalConstructor::Product(entity));
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
                            values.push(LegalConstructor::EnumVariant(entity));
                        }
                    }
                }
            }
            _ => {}
        }
        values.sort();
        values.dedup();
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

fn pattern_view(
    snapshot: &WorkspaceSnapshot,
    root: &crate::hir::MatchPattern,
) -> Result<(Vec<MatchPatternNodeView>, u64), WorkspaceError> {
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
                    push_pattern_view(snapshot, pattern, kind, &mut output, &mut completed)?;
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
    output: &mut Vec<MatchPatternNodeView>,
    completed: &mut Vec<u64>,
) -> Result<(), WorkspaceError> {
    let id = u64::try_from(output.len())
        .map_err(|_| WorkspaceError::Host(Arc::from("match pattern label exceeds u64")))?;
    output
        .try_reserve(1)
        .map_err(|_| WorkspaceError::Host(Arc::from("match pattern view allocation failed")))?;
    output.push(MatchPatternNodeView {
        id,
        ty: super::types::view(&snapshot.program, &snapshot.indexes, &pattern.ty())?,
        kind,
    });
    completed
        .try_reserve(1)
        .map_err(|_| WorkspaceError::Host(Arc::from("match pattern result allocation failed")))?;
    completed.push(id);
    Ok(())
}

fn take_pattern_results(
    completed: &mut Vec<u64>,
    count: usize,
) -> Result<Vec<u64>, WorkspaceError> {
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
