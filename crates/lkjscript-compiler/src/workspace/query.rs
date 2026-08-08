use std::sync::Arc;

use super::{
    CallEdge, DiagnosticHeader, EntityHeader, EntityId, EntityKind, HoleId, HoleState, NodeId,
    ReferenceEdge, RevisionId, WorkspaceError, WorkspaceSnapshot,
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
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[non_exhaustive]
pub enum LegalConstructor {
    I64Literal,
    F64Literal,
    BoolLiteral,
    UnitLiteral,
    Load(EntityId),
    Call(EntityId),
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
        Ok(NodeTypeFacts {
            revision,
            node,
            actual: Arc::clone(&header.actual_type),
            expected: header.expected_type.clone(),
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
                    if crate::Type::unify_assignable(&binding.ty, &context.expected_type) =>
                {
                    values.push(LegalConstructor::Load(entity));
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
