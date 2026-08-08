#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum ParameterOwnerKind {
    Resource,
    Unique,
    BorrowedUnique,
    Structural,
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum OwnerIdentity {
    None,
    Parameter {
        index: usize,
        kind: ParameterOwnerKind,
    },
    Instruction {
        offset: usize,
        sequence: u8,
    },
    Merged,
}

impl OwnerIdentity {
    pub(super) const fn parameter(index: usize, kind: ParameterOwnerKind) -> Self {
        Self::Parameter { index, kind }
    }

    pub(super) const fn instruction(offset: usize, sequence: u8) -> Self {
        Self::Instruction { offset, sequence }
    }

    pub(super) const fn is_none(self) -> bool {
        matches!(self, Self::None)
    }

    pub(super) const fn is_borrowed_parameter(self) -> bool {
        matches!(
            self,
            Self::Parameter {
                kind: ParameterOwnerKind::BorrowedUnique,
                ..
            }
        )
    }
}

impl std::fmt::Display for OwnerIdentity {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::None => formatter.write_str("none"),
            Self::Parameter { index, kind } => write!(formatter, "parameter {kind:?}:{index}"),
            Self::Instruction { offset, sequence } => {
                write!(formatter, "instruction {offset}:{sequence}")
            }
            Self::Merged => formatter.write_str("merged"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Kind {
    Any,
    Unit,
    Bool,
    I64,
    F64,
    Str,
    Symbol,
    Proto(u64),
    Closure(u64),
    List,
    StaticBytes,
    Bytes(OwnerIdentity),
    BytesBorrow {
        owner: OwnerIdentity,
        used: bool,
    },
    ByteVector(OwnerIdentity),
    ByteSlice {
        owner: OwnerIdentity,
        mutable: bool,
        used: bool,
    },
    Path,
    Capability(crate::CapabilityKind),
    Resource {
        kind: crate::ResourceKind,
        owner: OwnerIdentity,
    },
    ResourceResult {
        kind: crate::ResourceKind,
        owner: OwnerIdentity,
    },
    Product(ProductId),
    RegionProduct(ProductId),
    Enum(EnumId, Option<VariantId>),
    StructuralOwner {
        representation: crate::StructuralRepresentationId,
        owner: OwnerIdentity,
        active_variant: Option<crate::VariantId>,
    },
    StructuralOwnerRef {
        representation: crate::StructuralRepresentationId,
        owner: OwnerIdentity,
        active_variant: Option<crate::VariantId>,
    },
    StructuralView {
        representation: crate::StructuralRepresentationId,
        owner: OwnerIdentity,
        mutable: bool,
        used: bool,
    },
    StructuralDestination {
        destination: crate::StructuralDestinationId,
        identity: OwnerIdentity,
    },
}

impl std::fmt::Display for Kind {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            Self::Any => "any",
            Self::Unit => "unit",
            Self::Bool => "bool",
            Self::I64 => "i64",
            Self::F64 => "f64",
            Self::Str => "string",
            Self::Symbol => "symbol",
            Self::Proto(_) => "function-prototype",
            Self::Closure(_) => "function",
            Self::List => "list",
            Self::StaticBytes => "static bytes",
            Self::Bytes(_) => "dynamic bytes",
            Self::BytesBorrow { .. } => "borrowed dynamic bytes",
            Self::ByteVector(_) => "byte-vector",
            Self::ByteSlice { mutable: false, .. } => "byte-slice",
            Self::ByteSlice { mutable: true, .. } => "byte-slice-mut",
            Self::Path => "path",
            Self::Capability(_) => "capability",
            Self::Resource { .. } => "resource",
            Self::ResourceResult { .. } => "result resource",
            Self::Product(_) => "product",
            Self::RegionProduct(_) => "region-product",
            Self::Enum(_, _) => "enum",
            Self::StructuralOwner { .. } => "structural-owner",
            Self::StructuralOwnerRef { .. } => "structural-owner-reference",
            Self::StructuralView { .. } => "structural-view",
            Self::StructuralDestination { .. } => "structural-destination",
        };
        formatter.write_str(name)?;
        match self {
            Self::Proto(id) | Self::Closure(id) => write!(formatter, " {id}"),
            Self::Capability(kind) => write!(formatter, " {}", kind.as_str()),
            Self::Resource { kind, owner } | Self::ResourceResult { kind, owner } => {
                write!(formatter, " {} owner {owner}", kind.as_str())
            }
            Self::Bytes(owner)
            | Self::BytesBorrow { owner, .. }
            | Self::ByteVector(owner)
            | Self::ByteSlice { owner, .. } => {
                write!(formatter, " owner {owner}")
            }
            Self::Product(id) | Self::RegionProduct(id) => {
                write!(formatter, " {}", id.raw())
            }
            Self::Enum(_, Some(_)) => formatter.write_str(" variant"),
            Self::Enum(_, None) => Ok(()),
            Self::StructuralOwner {
                representation,
                owner,
                ..
            }
            | Self::StructuralOwnerRef {
                representation,
                owner,
                ..
            }
            | Self::StructuralView {
                representation,
                owner,
                ..
            } => write!(
                formatter,
                " representation {} owner {owner}",
                representation.raw()
            ),
            Self::StructuralDestination {
                destination,
                identity,
            } => write!(
                formatter,
                " metadata {} identity {identity}",
                destination.raw()
            ),
            _ => Ok(()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum UniquePlaceState {
    Inactive,
    Active {
        owner: Option<OwnerIdentity>,
        transferred: Option<OwnerIdentity>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct StructuralDestinationState {
    pub(super) destination: crate::StructuralDestinationId,
    pub(super) initialized: Vec<bool>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct CleanupRequirement {
    active_unique_owners: usize,
    borrowed_locals: usize,
    structural_destinations: usize,
}

impl CleanupRequirement {
    const fn required(self) -> bool {
        self.active_unique_owners != 0
            || self.borrowed_locals != 0
            || self.structural_destinations != 0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct State {
    pub(super) stack: Vec<Kind>,
    pub(super) locals: Vec<Option<Kind>>,
    pub(super) globals: Vec<Option<Kind>>,
    pub(super) unique_places: Vec<UniquePlaceState>,
    pub(super) structural_destinations:
        std::collections::BTreeMap<OwnerIdentity, StructuralDestinationState>,
    cleanup: CleanupRequirement,
}

impl State {
    pub(super) fn new(
        proto: &crate::FunctionProto,
        stack: Vec<Kind>,
        locals: Vec<Option<Kind>>,
        globals: Vec<Option<Kind>>,
        unique_places: Vec<UniquePlaceState>,
    ) -> Self {
        let mut state = Self {
            stack,
            locals,
            globals,
            unique_places,
            structural_destinations: std::collections::BTreeMap::new(),
            cleanup: CleanupRequirement::default(),
        };
        state.refresh_cleanup_requirement(proto);
        state
    }

    pub(super) const fn cleanup_required(&self) -> bool {
        self.cleanup.required()
    }

    pub(super) fn set_local(
        &mut self,
        proto: &crate::FunctionProto,
        index: usize,
        value: Option<Kind>,
    ) {
        let before = local_requires_cleanup(proto, index, self.locals[index]);
        let after = local_requires_cleanup(proto, index, value);
        update_count(&mut self.cleanup.borrowed_locals, before, after);
        self.locals[index] = value;
    }

    pub(super) fn set_unique_place(&mut self, index: usize, value: UniquePlaceState) {
        let before = place_requires_cleanup(self.unique_places[index]);
        let after = place_requires_cleanup(value);
        update_count(&mut self.cleanup.active_unique_owners, before, after);
        self.unique_places[index] = value;
    }

    pub(super) fn clear_unique_owner(&mut self, owner: OwnerIdentity) {
        for index in 0..self.unique_places.len() {
            if matches!(
                self.unique_places[index],
                UniquePlaceState::Active {
                    owner: Some(actual),
                    ..
                } if actual == owner
            ) {
                self.set_unique_place(
                    index,
                    UniquePlaceState::Active {
                        owner: None,
                        transferred: None,
                    },
                );
            }
        }
    }

    pub(super) fn insert_structural_destination(
        &mut self,
        identity: OwnerIdentity,
        destination: StructuralDestinationState,
    ) -> Option<StructuralDestinationState> {
        let previous = self.structural_destinations.insert(identity, destination);
        if previous.is_none() {
            self.cleanup.structural_destinations += 1;
        }
        previous
    }

    pub(super) fn remove_structural_destination(
        &mut self,
        identity: &OwnerIdentity,
    ) -> Option<StructuralDestinationState> {
        let removed = self.structural_destinations.remove(identity);
        if removed.is_some() {
            debug_assert!(self.cleanup.structural_destinations != 0);
            self.cleanup.structural_destinations -= 1;
        }
        removed
    }

    pub(super) fn refresh_cleanup_requirement(&mut self, proto: &crate::FunctionProto) {
        self.cleanup = self.calculated_cleanup_requirement(proto);
    }

    #[cfg(any(debug_assertions, test))]
    pub(super) fn cleanup_requirement_is_consistent(
        &self,
        proto: &crate::FunctionProto,
    ) -> bool {
        self.cleanup == self.calculated_cleanup_requirement(proto)
    }

    fn calculated_cleanup_requirement(
        &self,
        proto: &crate::FunctionProto,
    ) -> CleanupRequirement {
        CleanupRequirement {
            active_unique_owners: self
                .unique_places
                .iter()
                .filter(|place| place_requires_cleanup(**place))
                .count(),
            borrowed_locals: self
                .locals
                .iter()
                .copied()
                .enumerate()
                .filter(|(index, kind)| local_requires_cleanup(proto, *index, *kind))
                .count(),
            structural_destinations: self.structural_destinations.len(),
        }
    }
}

fn update_count(count: &mut usize, before: bool, after: bool) {
    match (before, after) {
        (false, true) => *count += 1,
        (true, false) => {
            debug_assert!(*count != 0);
            *count -= 1;
        }
        (false, false) | (true, true) => {}
    }
}

fn place_requires_cleanup(place: UniquePlaceState) -> bool {
    matches!(
        place,
        UniquePlaceState::Active {
            owner: Some(_),
            ..
        }
    )
}

fn local_requires_cleanup(
    proto: &crate::FunctionProto,
    index: usize,
    kind: Option<Kind>,
) -> bool {
    kind.is_some_and(|kind| {
        matches!(
            kind,
            Kind::BytesBorrow { .. }
                | Kind::ByteSlice { .. }
                | Kind::StructuralView { .. }
                | Kind::StructuralDestination { .. }
        ) && !borrowed_parameter(proto, index)
    })
}

fn borrowed_parameter(proto: &crate::FunctionProto, index: usize) -> bool {
    index < proto.arity
        && matches!(
            proto.parameter_uniques.get(index).copied().flatten(),
            Some(crate::UniqueValueKind::ByteSlice | crate::UniqueValueKind::ByteSliceMut)
        )
}

#[cfg(test)]
mod cleanup_requirement_tests {
    use super::*;

    #[test]
    fn local_place_and_destination_transitions_update_the_summary_exactly() {
        let mut chunk = crate::Chunk::new();
        chunk.main.locals = 1;
        chunk.main.unique_places = 1;
        let owner = OwnerIdentity::instruction(7, 1);
        let mut state = State::new(
            &chunk.main,
            Vec::new(),
            vec![None],
            Vec::new(),
            vec![UniquePlaceState::Inactive],
        );
        assert!(!state.cleanup_required());

        state.set_local(
            &chunk.main,
            0,
            Some(Kind::BytesBorrow { owner, used: false }),
        );
        assert!(state.cleanup_required());
        state.set_local(&chunk.main, 0, None);
        assert!(!state.cleanup_required());

        state.set_unique_place(
            0,
            UniquePlaceState::Active {
                owner: Some(owner),
                transferred: None,
            },
        );
        assert!(state.cleanup_required());
        state.set_unique_place(0, UniquePlaceState::Inactive);
        assert!(!state.cleanup_required());

        let destination = StructuralDestinationState {
            destination: crate::StructuralDestinationId::new(0),
            initialized: vec![false],
        };
        assert!(
            state
                .insert_structural_destination(owner, destination)
                .is_none()
        );
        assert!(state.cleanup_required());
        assert!(state.remove_structural_destination(&owner).is_some());
        assert!(!state.cleanup_required());
        assert!(state.cleanup_requirement_is_consistent(&chunk.main));
    }
}
