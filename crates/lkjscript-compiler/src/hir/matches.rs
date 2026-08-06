use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MatchPlanId(u32);

impl MatchPlanId {
    pub(crate) const fn new(raw: u32) -> Self {
        Self(raw)
    }
    pub const fn raw(self) -> u32 {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchLocal {
    pub binding: BindingId,
    pub place: PlaceId,
    pub slot: usize,
    pub ty: Type,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchFieldPattern {
    pub name: String,
    pub field_index: u64,
    pub projection: Option<MatchLocal>,
    pub pattern: MatchPattern,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MatchPattern {
    Wildcard {
        ty: Type,
    },
    Binding {
        local: MatchLocal,
    },
    Bool(bool),
    I64(i64),
    Variant {
        ty: Type,
        enum_id: EnumId,
        variant: VariantId,
        layout: RuntimeLayoutId,
        fields: Vec<MatchFieldPattern>,
    },
    Product {
        ty: Type,
        product: ProductId,
        fields: Vec<MatchFieldPattern>,
    },
}

impl MatchPattern {
    pub fn ty(&self) -> Type {
        match self {
            Self::Wildcard { ty } | Self::Variant { ty, .. } | Self::Product { ty, .. } => {
                ty.clone()
            }
            Self::Binding { local } => local.ty.clone(),
            Self::Bool(_) => Type::Bool,
            Self::I64(_) => Type::I64,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MatchTestKind {
    Bool(bool),
    I64(i64),
    Variant {
        enum_id: EnumId,
        variant: VariantId,
        layout: RuntimeLayoutId,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchTest {
    pub arm: u16,
    pub path: Vec<u16>,
    pub kind: MatchTestKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchProjection {
    pub arm: u16,
    pub path: Vec<u16>,
    pub local: MatchLocal,
    pub active_variant: Option<VariantId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchBindingAssignment {
    pub arm: u16,
    pub path: Vec<u16>,
    pub local: MatchLocal,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MatchEdgeTarget {
    Arm(u64),
    Default,
    Unreachable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchPlanCharges {
    pub patterns: u64,
    pub arms: u64,
    pub rows: u64,
    pub columns: u64,
    pub specialization_work: u64,
    pub witness_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannedMatchArm {
    pub id: u16,
    pub pattern: MatchPattern,
    pub body_type: Type,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchPlan {
    pub(crate) id: MatchPlanId,
    pub(crate) origin: SourceId,
    pub(crate) scrutinee: MatchLocal,
    pub(crate) result_type: Type,
    pub(crate) arms: Vec<PlannedMatchArm>,
    pub(crate) tests: Vec<MatchTest>,
    pub(crate) projections: Vec<MatchProjection>,
    pub(crate) bindings: Vec<MatchBindingAssignment>,
    pub(crate) edges: Vec<MatchEdgeTarget>,
    pub(crate) exhaustive: bool,
    pub(crate) witness: Option<String>,
    pub(crate) charges: MatchPlanCharges,
}
