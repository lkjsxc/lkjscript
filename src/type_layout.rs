use crate::error::{ErrorCode, LkError, Result};
use crate::graph::Snapshot;
use crate::ids::NodeId;
use crate::schema::{Node, SemanticType};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LayoutFailure {
    ByteSizeOverflow,
    CellCountOverflow,
    InvalidDependency,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(
    tag = "kind",
    content = "data",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum DerivedLayout {
    Representable(ValueLayout),
    Unrepresentable(LayoutFailure),
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ValueLayout {
    pub size: u64,
    pub align: u64,
    pub cells: u64,
    pub shape: LayoutShape,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(
    tag = "kind",
    content = "data",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum LayoutShape {
    Primitive,
    Product {
        fields: Vec<FieldLayout>,
    },
    Sum {
        discriminant_bytes: u8,
        payload_offset: u64,
        variants: Vec<VariantLayout>,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FieldLayout {
    pub field: NodeId,
    pub offset: u64,
    pub cells: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct VariantLayout {
    pub variant: NodeId,
    pub discriminant: u64,
    pub payload_size: u64,
    pub payload_align: u64,
    pub payload_cells: u64,
}

pub fn primitive_layout(ty: SemanticType) -> Option<ValueLayout> {
    let descriptor = ty.primitive_descriptor()?;
    Some(ValueLayout {
        size: descriptor.physical_slot_size,
        align: descriptor.physical_slot_align,
        cells: descriptor.cells,
        shape: LayoutShape::Primitive,
    })
}

pub(crate) fn managed_handle_layout() -> ValueLayout {
    ValueLayout {
        size: 8,
        align: 8,
        cells: 1,
        shape: LayoutShape::Primitive,
    }
}

pub(crate) fn validate_acyclic(snapshot: &Snapshot) -> Result<()> {
    let (_, remaining) = dependency_order(snapshot)?;
    if remaining.is_empty() {
        return Ok(());
    }
    let participants = exact_cycle_participants(snapshot, &remaining)?;
    let target = participants.first().copied().ok_or_else(|| {
        LkError::new(
            ErrorCode::ByValueTypeCycle,
            "cyclic nominal declaration component unexpectedly became empty",
        )
    })?;
    Err(LkError::new(
        ErrorCode::ByValueTypeCycle,
        "nominal declarations contain a by-value type cycle",
    )
    .for_node(target)
    .with_related(participants))
}

pub fn derive_layouts(snapshot: &Snapshot) -> Result<BTreeMap<NodeId, DerivedLayout>> {
    let (order, remaining) = dependency_order(snapshot)?;
    if !remaining.is_empty() {
        let participants = exact_cycle_participants(snapshot, &remaining)?;
        let target = participants.first().copied().ok_or_else(|| {
            LkError::new(
                ErrorCode::ByValueTypeCycle,
                "cyclic nominal declaration component unexpectedly became empty",
            )
        })?;
        return Err(LkError::new(
            ErrorCode::ByValueTypeCycle,
            "cannot derive layout for a cyclic nominal declaration graph",
        )
        .for_node(target)
        .with_related(participants));
    }
    let mut layouts = BTreeMap::new();
    for declaration in order {
        let layout = derive_declaration(snapshot, declaration, &layouts)?;
        layouts.insert(declaration, layout);
    }
    Ok(layouts)
}

pub fn layout_of(
    _snapshot: &Snapshot,
    ty: SemanticType,
    layouts: &BTreeMap<NodeId, DerivedLayout>,
) -> Result<DerivedLayout> {
    if let Some(layout) = primitive_layout(ty) {
        return Ok(DerivedLayout::Representable(layout));
    }
    let SemanticType::Nominal(target) = ty else {
        unreachable!()
    };
    layouts.get(&target).cloned().ok_or_else(|| {
        LkError::new(
            ErrorCode::WrongKind,
            "nominal layout target is not a declaration",
        )
        .for_node(target)
    })
}

fn dependency_order(snapshot: &Snapshot) -> Result<(Vec<NodeId>, BTreeSet<NodeId>)> {
    let declarations = snapshot
        .nodes
        .iter()
        .filter_map(|(id, node)| {
            matches!(
                node,
                Node::ProductType { .. } | Node::SumType { .. } | Node::SequenceType { .. }
            )
            .then_some(*id)
        })
        .collect::<BTreeSet<_>>();
    let mut pending = BTreeMap::<NodeId, BTreeSet<NodeId>>::new();
    let mut dependents = BTreeMap::<NodeId, BTreeSet<NodeId>>::new();
    for declaration in &declarations {
        let dependencies = declaration_dependencies(snapshot, *declaration)?;
        for dependency in &dependencies {
            if !declarations.contains(dependency) {
                return Err(LkError::new(
                    ErrorCode::WrongKind,
                    "nominal dependency target is not a declaration",
                )
                .for_node(*dependency)
                .with_related([*declaration]));
            }
            dependents
                .entry(*dependency)
                .or_default()
                .insert(*declaration);
        }
        pending.insert(*declaration, dependencies);
    }
    let mut ready = pending
        .iter()
        .filter_map(|(id, dependencies)| dependencies.is_empty().then_some(*id))
        .collect::<BTreeSet<_>>();
    let mut order = Vec::with_capacity(declarations.len());
    while let Some(next) = ready.pop_first() {
        if !pending.contains_key(&next) {
            continue;
        }
        pending.remove(&next);
        order.push(next);
        if let Some(owners) = dependents.get(&next) {
            for owner in owners {
                if let Some(dependencies) = pending.get_mut(owner) {
                    dependencies.remove(&next);
                    if dependencies.is_empty() {
                        ready.insert(*owner);
                    }
                }
            }
        }
    }
    Ok((order, pending.into_keys().collect()))
}

fn exact_cycle_participants(
    snapshot: &Snapshot,
    remaining: &BTreeSet<NodeId>,
) -> Result<Vec<NodeId>> {
    let mut dependencies = BTreeMap::<NodeId, BTreeSet<NodeId>>::new();
    let mut owners = remaining
        .iter()
        .copied()
        .map(|declaration| (declaration, BTreeSet::new()))
        .collect::<BTreeMap<_, _>>();
    for declaration in remaining {
        let declaration_dependencies = declaration_dependencies(snapshot, *declaration)?
            .into_iter()
            .filter(|dependency| remaining.contains(dependency))
            .collect::<BTreeSet<_>>();
        for dependency in &declaration_dependencies {
            owners.entry(*dependency).or_default().insert(*declaration);
        }
        dependencies.insert(*declaration, declaration_dependencies);
    }

    // Iterative Kosaraju traversal. Ordered maps and sets make both passes deterministic; explicit
    // finishing markers avoid consuming the native stack for user-controlled type depth.
    let mut visited = BTreeSet::new();
    let mut finishing_order = Vec::with_capacity(remaining.len());
    for root in remaining {
        if visited.contains(root) {
            continue;
        }
        let mut stack = vec![(*root, false)];
        while let Some((declaration, finishing)) = stack.pop() {
            if finishing {
                finishing_order.push(declaration);
                continue;
            }
            if !visited.insert(declaration) {
                continue;
            }
            stack.push((declaration, true));
            for dependency in dependencies[&declaration].iter().rev() {
                if !visited.contains(dependency) {
                    stack.push((*dependency, false));
                }
            }
        }
    }

    visited.clear();
    let mut selected = None::<Vec<NodeId>>;
    for root in finishing_order.into_iter().rev() {
        if !visited.insert(root) {
            continue;
        }
        let mut component = Vec::new();
        let mut stack = vec![root];
        while let Some(declaration) = stack.pop() {
            component.push(declaration);
            for owner in owners[&declaration].iter().rev() {
                if visited.insert(*owner) {
                    stack.push(*owner);
                }
            }
        }
        component.sort_unstable();
        let cyclic = component.len() > 1 || dependencies[&component[0]].contains(&component[0]);
        if cyclic
            && selected
                .as_ref()
                .is_none_or(|current| component[0] < current[0])
        {
            selected = Some(component);
        }
    }

    selected.ok_or_else(|| {
        LkError::new(
            ErrorCode::ByValueTypeCycle,
            "cyclic nominal declaration residual has no cycle component",
        )
    })
}

fn declaration_dependencies(snapshot: &Snapshot, declaration: NodeId) -> Result<BTreeSet<NodeId>> {
    let mut dependencies = BTreeSet::new();
    match snapshot.node(declaration)? {
        Node::ProductType { fields, .. } => {
            for field in fields {
                let Node::ProductField { ty, .. } = snapshot.node(*field)? else {
                    return Err(LkError::new(
                        ErrorCode::WrongKind,
                        "product member is not a field",
                    )
                    .for_node(*field));
                };
                if let SemanticType::Nominal(target) = ty {
                    dependencies.insert(*target);
                }
            }
        }
        Node::SumType { variants, .. } => {
            for variant in variants {
                let Node::SumVariant { payload, .. } = snapshot.node(*variant)? else {
                    return Err(
                        LkError::new(ErrorCode::WrongKind, "sum member is not a variant")
                            .for_node(*variant),
                    );
                };
                if let Some(SemanticType::Nominal(target)) = payload {
                    dependencies.insert(*target);
                }
            }
        }
        Node::SequenceType { .. } => {}
        node => {
            return Err(LkError::new(
                ErrorCode::WrongKind,
                "layout target is not a nominal declaration",
            )
            .for_node(declaration)
            .with_kinds(crate::schema::NodeKind::ProductType, node.kind()));
        }
    }
    Ok(dependencies)
}

fn derive_declaration(
    snapshot: &Snapshot,
    declaration: NodeId,
    layouts: &BTreeMap<NodeId, DerivedLayout>,
) -> Result<DerivedLayout> {
    match snapshot.node(declaration)? {
        Node::ProductType { fields, .. } => derive_product(snapshot, fields, layouts),
        Node::SumType { variants, .. } => derive_sum(snapshot, variants, layouts),
        Node::SequenceType { .. } => Ok(DerivedLayout::Representable(managed_handle_layout())),
        _ => Err(LkError::new(
            ErrorCode::WrongKind,
            "layout target is not a nominal declaration",
        )
        .for_node(declaration)),
    }
}

fn derive_product(
    snapshot: &Snapshot,
    fields: &[NodeId],
    layouts: &BTreeMap<NodeId, DerivedLayout>,
) -> Result<DerivedLayout> {
    let mut offset = 0_u64;
    let mut align = 1_u64;
    let mut cells = 0_u64;
    let mut derived_fields = Vec::with_capacity(fields.len());
    for field in fields {
        let Node::ProductField { ty, .. } = snapshot.node(*field)? else {
            return Err(
                LkError::new(ErrorCode::WrongKind, "product member is not a field")
                    .for_node(*field),
            );
        };
        let DerivedLayout::Representable(layout) = layout_of(snapshot, *ty, layouts)? else {
            return Ok(DerivedLayout::Unrepresentable(
                LayoutFailure::InvalidDependency,
            ));
        };
        align = align.max(layout.align);
        offset = match align_up(offset, layout.align) {
            Some(value) => value,
            None => {
                return Ok(DerivedLayout::Unrepresentable(
                    LayoutFailure::ByteSizeOverflow,
                ));
            }
        };
        let field_offset = offset;
        offset = match offset.checked_add(layout.size) {
            Some(value) => value,
            None => {
                return Ok(DerivedLayout::Unrepresentable(
                    LayoutFailure::ByteSizeOverflow,
                ));
            }
        };
        cells = match cells.checked_add(layout.cells) {
            Some(value) => value,
            None => {
                return Ok(DerivedLayout::Unrepresentable(
                    LayoutFailure::CellCountOverflow,
                ));
            }
        };
        derived_fields.push(FieldLayout {
            field: *field,
            offset: field_offset,
            cells: layout.cells,
        });
    }
    let Some(size) = align_up(offset, align) else {
        return Ok(DerivedLayout::Unrepresentable(
            LayoutFailure::ByteSizeOverflow,
        ));
    };
    Ok(DerivedLayout::Representable(ValueLayout {
        size,
        align,
        cells,
        shape: LayoutShape::Product {
            fields: derived_fields,
        },
    }))
}

fn derive_sum(
    snapshot: &Snapshot,
    variants: &[NodeId],
    layouts: &BTreeMap<NodeId, DerivedLayout>,
) -> Result<DerivedLayout> {
    let discriminant_bytes = discriminant_width(variants.len());
    let tag_size = u64::from(discriminant_bytes);
    let mut payload_size = 0_u64;
    let mut payload_align = 1_u64;
    let mut payload_cells = 0_u64;
    let mut derived_variants = Vec::with_capacity(variants.len());
    for (ordinal, variant) in variants.iter().enumerate() {
        let Node::SumVariant { payload, .. } = snapshot.node(*variant)? else {
            return Err(
                LkError::new(ErrorCode::WrongKind, "sum member is not a variant")
                    .for_node(*variant),
            );
        };
        let layout = if let Some(payload) = payload {
            match layout_of(snapshot, *payload, layouts)? {
                DerivedLayout::Representable(layout) => layout,
                DerivedLayout::Unrepresentable(_) => {
                    return Ok(DerivedLayout::Unrepresentable(
                        LayoutFailure::InvalidDependency,
                    ));
                }
            }
        } else {
            ValueLayout {
                size: 0,
                align: 1,
                cells: 0,
                shape: LayoutShape::Primitive,
            }
        };
        payload_size = payload_size.max(layout.size);
        payload_align = payload_align.max(layout.align);
        payload_cells = payload_cells.max(layout.cells);
        derived_variants.push(VariantLayout {
            variant: *variant,
            discriminant: u64::try_from(ordinal).map_err(|_| {
                LkError::new(
                    ErrorCode::TypeLayoutUnrepresentable,
                    "variant discriminant exceeds u64",
                )
                .for_node(*variant)
            })?,
            payload_size: layout.size,
            payload_align: layout.align,
            payload_cells: layout.cells,
        });
    }
    let align = tag_size.max(payload_align);
    let Some(payload_offset) = align_up(tag_size, payload_align) else {
        return Ok(DerivedLayout::Unrepresentable(
            LayoutFailure::ByteSizeOverflow,
        ));
    };
    let Some(end) = payload_offset.checked_add(payload_size) else {
        return Ok(DerivedLayout::Unrepresentable(
            LayoutFailure::ByteSizeOverflow,
        ));
    };
    let Some(size) = align_up(end, align) else {
        return Ok(DerivedLayout::Unrepresentable(
            LayoutFailure::ByteSizeOverflow,
        ));
    };
    let Some(cells) = 1_u64.checked_add(payload_cells) else {
        return Ok(DerivedLayout::Unrepresentable(
            LayoutFailure::CellCountOverflow,
        ));
    };
    Ok(DerivedLayout::Representable(ValueLayout {
        size,
        align,
        cells,
        shape: LayoutShape::Sum {
            discriminant_bytes,
            payload_offset,
            variants: derived_variants,
        },
    }))
}

fn discriminant_width(variant_count: usize) -> u8 {
    let maximum = variant_count.saturating_sub(1);
    if maximum <= u8::MAX as usize {
        1
    } else if maximum <= u16::MAX as usize {
        2
    } else if u32::try_from(maximum).is_ok() {
        4
    } else {
        8
    }
}

fn align_up(value: u64, align: u64) -> Option<u64> {
    debug_assert!(align > 0 && align.is_power_of_two());
    let mask = align.checked_sub(1)?;
    value.checked_add(mask).map(|value| value & !mask)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::{Revision, SnapshotHash, WorkspaceId};

    #[derive(Clone, Copy)]
    enum TestDeclarationKind {
        Product,
        Sum,
    }

    fn dependency_snapshot(specifications: &[(u64, TestDeclarationKind, Option<u64>)]) -> Snapshot {
        let workspace = WorkspaceId::from_bytes([0x79; 16]);
        let id = |serial| NodeId::new(workspace, serial).expect("node");
        let declarations = specifications
            .iter()
            .map(|(serial, _, _)| id(*serial))
            .collect::<Vec<_>>();
        let mut nodes = BTreeMap::from([
            (
                id(1),
                Node::WorkspaceRoot {
                    packages: vec![id(2)],
                    targets: Vec::new(),
                },
            ),
            (
                id(2),
                Node::Package {
                    owner: id(1),
                    name: "p".into(),
                    modules: vec![id(3)],
                    entry: None,
                },
            ),
            (
                id(3),
                Node::Module {
                    owner: id(2),
                    name: "m".into(),
                    types: declarations,
                    functions: Vec::new(),
                },
            ),
        ]);
        let mut next_member = specifications
            .iter()
            .map(|(serial, _, _)| *serial)
            .max()
            .unwrap_or(3)
            + 1;
        for (serial, kind, dependency) in specifications {
            let declaration = id(*serial);
            let member = id(next_member);
            next_member += 1;
            match kind {
                TestDeclarationKind::Product => {
                    nodes.insert(
                        declaration,
                        Node::ProductType {
                            owner: id(3),
                            name: format!("P{serial}"),
                            fields: vec![member],
                        },
                    );
                    nodes.insert(
                        member,
                        Node::ProductField {
                            owner: declaration,
                            ordinal: 0,
                            name: "value".into(),
                            ty: dependency
                                .map(|target| SemanticType::Nominal(id(target)))
                                .unwrap_or(SemanticType::I64),
                        },
                    );
                }
                TestDeclarationKind::Sum => {
                    nodes.insert(
                        declaration,
                        Node::SumType {
                            owner: id(3),
                            name: format!("S{serial}"),
                            variants: vec![member],
                        },
                    );
                    nodes.insert(
                        member,
                        Node::SumVariant {
                            owner: declaration,
                            ordinal: 0,
                            name: "value".into(),
                            payload: dependency.map(|target| SemanticType::Nominal(id(target))),
                        },
                    );
                }
            }
        }
        Snapshot {
            workspace,
            revision: Revision::INITIAL,
            root: id(1),
            next_serial: next_member,
            tombstones: BTreeSet::new(),
            nodes,
            hash: SnapshotHash::from_bytes([0; SnapshotHash::BYTE_LEN]),
        }
    }

    fn assert_cycle_diagnostics(snapshot: &Snapshot, expected_serials: &[u64]) {
        let expected = expected_serials
            .iter()
            .map(|serial| NodeId::new(snapshot.workspace, *serial).expect("node"))
            .collect::<Vec<_>>();
        let validation_error = validate_acyclic(snapshot).expect_err("validation cycle");
        let layout_error = derive_layouts(snapshot).expect_err("layout cycle");
        for error in [&validation_error, &layout_error] {
            assert_eq!(error.code, ErrorCode::ByValueTypeCycle);
            assert_eq!(error.target, expected.first().copied());
            assert_eq!(error.related.as_ref(), expected.as_slice());
        }
    }

    #[test]
    fn primitive_layout_contract_is_exact() {
        assert_eq!(primitive_layout(SemanticType::Unit).expect("unit").size, 0);
        assert_eq!(primitive_layout(SemanticType::Bool).expect("bool").align, 1);
        assert_eq!(primitive_layout(SemanticType::I64).expect("i64").size, 8);
        assert_eq!(primitive_layout(SemanticType::I64).expect("i64").cells, 1);
    }

    #[test]
    fn alignment_is_checked_and_exact() {
        assert_eq!(align_up(1, 8), Some(8));
        assert_eq!(align_up(8, 8), Some(8));
        assert_eq!(align_up(u64::MAX, 8), None);
    }

    #[test]
    fn acyclic_dependent_is_excluded_from_mixed_cycle_diagnostics() {
        let snapshot = dependency_snapshot(&[
            (4, TestDeclarationKind::Product, Some(6)),
            (6, TestDeclarationKind::Sum, Some(8)),
            (8, TestDeclarationKind::Product, Some(6)),
        ]);

        assert_cycle_diagnostics(&snapshot, &[6, 8]);
    }

    #[test]
    fn lower_id_dependents_cannot_crowd_cycle_participants_out_of_diagnostics() {
        let mut specifications = (4..=68)
            .map(|serial| (serial, TestDeclarationKind::Product, Some(100)))
            .collect::<Vec<_>>();
        specifications.extend([
            (100, TestDeclarationKind::Product, Some(101)),
            (101, TestDeclarationKind::Sum, Some(100)),
        ]);
        let snapshot = dependency_snapshot(&specifications);

        assert_cycle_diagnostics(&snapshot, &[100, 101]);
    }

    #[test]
    fn self_cycle_reports_its_only_participant() {
        let snapshot = dependency_snapshot(&[(4, TestDeclarationKind::Sum, Some(4))]);

        assert_cycle_diagnostics(&snapshot, &[4]);
    }

    #[test]
    fn multiple_cycles_select_the_component_with_the_lowest_participant() {
        let snapshot = dependency_snapshot(&[
            (10, TestDeclarationKind::Sum, Some(11)),
            (11, TestDeclarationKind::Product, Some(10)),
            (4, TestDeclarationKind::Product, Some(5)),
            (5, TestDeclarationKind::Product, Some(4)),
        ]);

        assert_cycle_diagnostics(&snapshot, &[4, 5]);
    }

    #[test]
    fn deep_nominal_dependency_traversal_is_iterative_and_deterministic() {
        let workspace = WorkspaceId::from_bytes([0x77; 16]);
        let id = |serial| NodeId::new(workspace, serial).expect("node");
        let depth = 2_000_u64;
        let mut nodes = BTreeMap::new();
        let declarations = (0..depth)
            .map(|index| id(4 + index * 2))
            .collect::<Vec<_>>();
        nodes.insert(
            id(1),
            Node::WorkspaceRoot {
                packages: vec![id(2)],
                targets: Vec::new(),
            },
        );
        nodes.insert(
            id(2),
            Node::Package {
                owner: id(1),
                name: "p".into(),
                modules: vec![id(3)],
                entry: None,
            },
        );
        nodes.insert(
            id(3),
            Node::Module {
                owner: id(2),
                name: "m".into(),
                types: declarations.clone(),
                functions: Vec::new(),
            },
        );
        for index in 0..depth {
            let declaration = id(4 + index * 2);
            let field = id(5 + index * 2);
            nodes.insert(
                declaration,
                Node::ProductType {
                    owner: id(3),
                    name: format!("T{index}"),
                    fields: vec![field],
                },
            );
            let ty = if index + 1 == depth {
                SemanticType::I64
            } else {
                SemanticType::Nominal(id(4 + (index + 1) * 2))
            };
            nodes.insert(
                field,
                Node::ProductField {
                    owner: declaration,
                    ordinal: 0,
                    name: "next".into(),
                    ty,
                },
            );
        }
        let snapshot = Snapshot {
            workspace,
            revision: Revision::INITIAL,
            root: id(1),
            next_serial: 4 + depth * 2,
            tombstones: BTreeSet::new(),
            nodes,
            hash: SnapshotHash::from_bytes([0; SnapshotHash::BYTE_LEN]),
        };
        validate_acyclic(&snapshot).expect("deep acyclic chain");
        let first = derive_layouts(&snapshot).expect("deep layouts");
        assert_eq!(
            first,
            derive_layouts(&snapshot).expect("deterministic layouts")
        );
        assert_eq!(first.len(), usize::try_from(depth).expect("depth"));

        let mut cyclic = snapshot.clone();
        let last_field = id(5 + (depth - 1) * 2);
        let Node::ProductField { ty, .. } = cyclic.nodes.get_mut(&last_field).expect("last field")
        else {
            panic!("field")
        };
        *ty = SemanticType::Nominal(id(4));
        let error = validate_acyclic(&cyclic).expect_err("deep cycle");
        assert_eq!(error.code, ErrorCode::ByValueTypeCycle);
        assert_eq!(error.target, Some(id(4)));
    }

    #[test]
    fn layout_overflow_is_derived_and_does_not_invalidate_graph_authority() {
        let workspace = WorkspaceId::from_bytes([0x78; 16]);
        let id = |serial| NodeId::new(workspace, serial).expect("node");
        let depth = 70_u64;
        let mut next = 4_u64;
        let mut declarations = Vec::new();
        let mut records = Vec::new();
        for level in 0..depth {
            let declaration = id(next);
            let left = id(next + 1);
            let right = id(next + 2);
            next += 3;
            declarations.push(declaration);
            records.push((declaration, left, right, level));
        }
        let mut nodes = BTreeMap::from([
            (
                id(1),
                Node::WorkspaceRoot {
                    packages: vec![id(2)],
                    targets: Vec::new(),
                },
            ),
            (
                id(2),
                Node::Package {
                    owner: id(1),
                    name: "p".into(),
                    modules: vec![id(3)],
                    entry: None,
                },
            ),
            (
                id(3),
                Node::Module {
                    owner: id(2),
                    name: "m".into(),
                    types: declarations.clone(),
                    functions: Vec::new(),
                },
            ),
        ]);
        for (index, (declaration, left, right, level)) in records.iter().enumerate() {
            let ty = declarations
                .get(index + 1)
                .copied()
                .map(SemanticType::Nominal)
                .unwrap_or(SemanticType::I64);
            nodes.insert(
                *declaration,
                Node::ProductType {
                    owner: id(3),
                    name: format!("Wide{level}"),
                    fields: vec![*left, *right],
                },
            );
            nodes.insert(
                *left,
                Node::ProductField {
                    owner: *declaration,
                    ordinal: 0,
                    name: "left".into(),
                    ty,
                },
            );
            nodes.insert(
                *right,
                Node::ProductField {
                    owner: *declaration,
                    ordinal: 1,
                    name: "right".into(),
                    ty,
                },
            );
        }
        let snapshot = Snapshot::from_parts(
            workspace,
            Revision::INITIAL,
            id(1),
            next,
            BTreeSet::new(),
            nodes,
        )
        .expect("structurally valid overflowing layout graph");
        let layouts = derive_layouts(&snapshot).expect("derived layouts");
        assert!(
            layouts
                .values()
                .any(|layout| matches!(layout, DerivedLayout::Unrepresentable(_)))
        );
    }

    #[test]
    fn discriminant_width_crosses_bootstrap_thresholds() {
        assert_eq!(discriminant_width(1), 1);
        assert_eq!(discriminant_width(256), 1);
        assert_eq!(discriminant_width(257), 2);
        assert_eq!(discriminant_width(65_537), 4);
    }
}
