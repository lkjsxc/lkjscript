//! Independent observations for the evaluator-hosted structural type reader.
//! Canonical encoders make fixture bytes; these matches and the DFS do not call
//! the compiler's child walker or dependency-count queue.
use super::*;
use crate::platform::compiler::selfhost_source_input::{
    self as source_input, RawRevisionProvenance, RawSourceGraph, RawType,
};
use crate::platform::semantic_id::{
    AssociatedTypeMemberId, BindingId, DeclarationId, EffectParameterId, ImplementationParameterId,
    MethodId, ReadScopeParameterId, RequirementId, RequirementParameterId, TypeParameterId,
};

pub(super) type Graph = Vec<Vec<usize>>;

pub(super) fn alias(index: usize) -> TypeObjectDigest {
    let mut bytes = [0x5a; 32];
    bytes[..8].copy_from_slice(&(index as u64).to_le_bytes());
    TypeObjectDigest::from_bytes(bytes)
}

pub(super) fn image(form: kernel::TypeForm) -> Vec<u8> {
    kernel::encode_type_object(&kernel::TypeObject::new(form).unwrap())
        .unwrap()
        .1
}

pub(super) fn carrier(rows: &[(TypeObjectDigest, Vec<u8>)]) -> Vec<u8> {
    // Aliases deliberately need not hash their images: this probes a structural
    // catalogue, whose content/authority admission belongs to its caller. This
    // also makes missing endpoints and literal cycles representable fixtures.
    let types: Vec<_> = rows
        .iter()
        .map(|(digest, bytes)| RawType {
            digest: digest.bytes(),
            bytes,
        })
        .collect();
    source_input::carrier(&RawSourceGraph {
        package: [1; 16],
        package_name: b"typed-fixture",
        semantic_state: [0; 32],
        provenance: RawRevisionProvenance {
            claimed_revision: [0; 32],
            canonical_core: &[],
        },
        owners: &[],
        types: &types,
        retirements: &[],
    })
    .unwrap()
}

pub(super) fn rows(graph: &Graph) -> Vec<(TypeObjectDigest, Vec<u8>)> {
    graph
        .iter()
        .enumerate()
        .map(|(index, children)| {
            let form = match children.as_slice() {
                [] => kernel::TypeForm::Unit,
                [child] => kernel::TypeForm::List {
                    item: alias(*child),
                },
                _ => kernel::TypeForm::StructuralRecord {
                    fields: children
                        .iter()
                        .enumerate()
                        .map(|(position, child)| kernel::StructuralTypeField {
                            name: kernel::Name::new(format!("f{position:016}")).unwrap(),
                            ty: alias(*child),
                        })
                        .collect(),
                },
            };
            (alias(index), image(form))
        })
        .collect()
}

pub(super) fn shape(form: &kernel::TypeForm) -> (&'static str, Vec<TypeObjectDigest>) {
    use kernel::TypeForm::*;
    match form {
        Unit => ("unit", vec![]),
        Bool => ("bool", vec![]),
        I64 => ("i64", vec![]),
        Bytes => ("bytes", vec![]),
        Text => ("text", vec![]),
        StaticText => ("static-text", vec![]),
        Secret => ("secret", vec![]),
        TypeParameter { .. } => ("type-parameter", vec![]),
        Named { .. } => ("named", vec![]),
        CapabilityResource { .. } => ("capability-resource", vec![]),
        StructuralRecord { fields } => ("structural-record", fields.iter().map(|f| f.ty).collect()),
        List { item } => ("list", vec![*item]),
        Map { key, value } => ("map", vec![*key, *value]),
        Option { item } => ("option", vec![*item]),
        Result { ok, error } => ("result", vec![*ok, *error]),
        Stream { item } => ("stream", vec![*item]),
        Function { parameters, result } => (
            "function",
            parameters.iter().copied().chain([*result]).collect(),
        ),
        Applied { arguments, .. } => ("applied", arguments.clone()),
        TaskFunction {
            parameters, result, ..
        } => (
            "task-function",
            parameters.iter().copied().chain([*result]).collect(),
        ),
        F64 => ("f64", vec![]),
        ByteBuffer => ("byte-buffer", vec![]),
        OwnedI64Cell => ("owned-i64-cell", vec![]),
        OwnedProduct { fields } => ("owned-product", fields.iter().map(|f| f.ty).collect()),
        OwnedChoice { cases } => ("owned-choice", cases.iter().map(|f| f.ty).collect()),
        OwnedSequence { item } => ("owned-sequence", vec![*item]),
        AssociatedContractMember { .. } => ("associated-contract-member", vec![]),
        AssociatedWitnessProjection { .. } => ("associated-witness-projection", vec![]),
        ReadViewApplied { arguments, .. } => ("read-view-applied", arguments.clone()),
        AssociatedReadViewMember { .. } => ("associated-read-view-member", vec![]),
        AssociatedReadViewProjection { .. } => ("associated-read-view-projection", vec![]),
        RegionBrand { .. } => ("region-brand", vec![]),
        Region { region } => ("region", vec![*region]),
        RelPtr { region, item } => ("rel-ptr", vec![*region, *item]),
        RegionSlice { region, item } => ("region-slice", vec![*region, *item]),
        RegionCap { region, .. } => ("region-cap", vec![*region]),
    }
}

pub(super) fn constructors() -> Vec<kernel::TypeForm> {
    use kernel::{ReadScopeArgument, ReadScopeBinder, ReadScopeReference, RegionMode, TypeForm::*};
    let seed = b"typed compiler input constructor corpus";
    let package = kernel::PackageId::from_bytes([1; 16]).unwrap();
    let declaration = DeclarationReference {
        package,
        declaration: DeclarationId::allocate(seed, 0),
    };
    let member = AssociatedTypeMemberId::allocate(seed, 0);
    let parameter = ImplementationParameterId::allocate(seed, 0);
    let binding = BindingId::allocate(seed, 0);
    let lexical = ReadScopeArgument::Lexical {
        callable: declaration,
        binding,
    };
    let [a, b, c] = [alias(7), alias(3), alias(11)];
    let fields: Vec<_> = [("a", a), ("b", b), ("c", a)]
        .into_iter()
        .map(|(name, ty)| kernel::StructuralTypeField {
            name: kernel::Name::new(name).unwrap(),
            ty,
        })
        .collect();
    let mut effect = kernel::EffectRow {
        requirements: vec![
            kernel::RequirementOperand::Concrete(kernel::RequirementReference {
                package,
                requirement: RequirementId::allocate(seed, 0),
            }),
            kernel::RequirementOperand::Parameter(kernel::RequirementParameterReference {
                package,
                parameter: RequirementParameterId::allocate(seed, 0),
            }),
        ],
        parameters: vec![kernel::EffectParameterReference {
            package,
            parameter: EffectParameterId::allocate(seed, 0),
        }],
    };
    effect.normalize().unwrap();
    let mut forms = vec![
        Unit,
        Bool,
        I64,
        Bytes,
        Text,
        StaticText,
        Secret,
        TypeParameter {
            parameter: TypeParameterId::allocate(seed, 0),
        },
        Named { declaration },
        CapabilityResource {
            interface: declaration,
        },
        StructuralRecord {
            fields: fields.clone(),
        },
        List { item: a },
        Map { key: b, value: a },
        Option { item: b },
        Result { ok: b, error: a },
        Stream { item: c },
        Function {
            parameters: vec![a, b, a],
            result: c,
        },
        Applied {
            declaration,
            arguments: vec![a, b, a],
        },
        TaskFunction {
            parameters: vec![c, b, c],
            result: a,
            effect,
        },
        F64,
        ByteBuffer,
        OwnedI64Cell,
        OwnedProduct {
            fields: fields.clone(),
        },
        OwnedChoice { cases: fields },
        OwnedSequence { item: c },
        AssociatedContractMember {
            contract: declaration,
            member,
        },
        AssociatedWitnessProjection {
            scope: declaration,
            parameter,
            contract: declaration,
            member,
        },
        ReadViewApplied {
            declaration,
            arguments: vec![b, a, b],
            read_scope: lexical,
        },
        AssociatedReadViewMember {
            contract: declaration,
            member,
            read_scope: lexical,
        },
        AssociatedReadViewProjection {
            scope: declaration,
            parameter,
            contract: declaration,
            member,
            read_scope: lexical,
        },
        RegionBrand {
            callable: declaration,
            binding,
        },
        Region { region: c },
        RelPtr { region: c, item: b },
        RegionSlice { region: b, item: a },
        RegionCap {
            region: c,
            mode: RegionMode::Unique,
        },
        Function {
            parameters: vec![],
            result: a,
        },
        TaskFunction {
            parameters: vec![],
            result: a,
            effect: kernel::EffectRow::default(),
        },
        ReadViewApplied {
            declaration,
            arguments: vec![],
            read_scope: lexical,
        },
        RegionCap {
            region: c,
            mode: RegionMode::Frozen,
        },
        RegionCap {
            region: c,
            mode: RegionMode::Pinned,
        },
    ];
    for binder in [
        ReadScopeBinder::Declaration(declaration),
        ReadScopeBinder::ContractMethod {
            contract: declaration,
            method: MethodId::allocate(seed, 0),
        },
        ReadScopeBinder::AssociatedMember {
            contract: declaration,
            member,
        },
        ReadScopeBinder::AssociatedBinding {
            implementation: declaration,
            member,
        },
    ] {
        let read_scope = ReadScopeArgument::Parameter(ReadScopeReference {
            binder,
            parameter: ReadScopeParameterId::allocate(seed, 0),
        });
        forms.push(ReadViewApplied {
            declaration,
            arguments: vec![a, b],
            read_scope,
        });
        forms.push(RegionCap {
            region: c,
            mode: RegionMode::Shared { scope: read_scope },
        });
    }
    forms.push(RegionCap {
        region: c,
        mode: RegionMode::Shared { scope: lexical },
    });
    forms
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Class {
    Ready,
    Missing,
    Cycle,
}

pub(super) fn class(graph: &Graph) -> Class {
    if graph.iter().flatten().any(|child| *child >= graph.len()) {
        Class::Missing
    } else if unresolved(graph).is_empty() {
        Class::Ready
    } else {
        Class::Cycle
    }
}

pub(super) fn unresolved(graph: &Graph) -> Vec<usize> {
    // A failed DFS path remains marked: every ancestor depends on that cycle.
    // Completed finite subgraphs are marked separately and shared by all roots.
    fn visit(graph: &Graph, node: usize, colors: &mut [u8]) -> bool {
        match colors[node] {
            1 => return false,
            2 => return true,
            _ => {}
        }
        colors[node] = 1;
        for child in &graph[node] {
            if !visit(graph, *child, colors) {
                return false;
            }
        }
        colors[node] = 2;
        true
    }
    let mut colors = vec![0; graph.len()];
    (0..graph.len())
        .filter(|node| !visit(graph, *node, &mut colors))
        .collect()
}

pub(super) fn parents(graph: &Graph) -> Graph {
    let mut parents = vec![vec![]; graph.len()];
    for (parent, children) in graph.iter().enumerate() {
        for child in children {
            parents[*child].push(parent);
        }
    }
    parents
}

/// Greedy reduction preserves the oracle outcome and the caller's disagreement.
/// Remove individual occurrences first, then vertices and their incident edges.
/// Renumber surviving references, retaining an absent endpoint as absent.
pub(super) fn reduce(mut graph: Graph, mut disagrees: impl FnMut(&Graph) -> bool) -> Graph {
    let outcome = class(&graph);
    loop {
        let mut smaller = None;
        'edges: for parent in 0..graph.len() {
            for position in 0..graph[parent].len() {
                let mut candidate = graph.clone();
                candidate[parent].remove(position);
                if class(&candidate) == outcome && disagrees(&candidate) {
                    smaller = Some(candidate);
                    break 'edges;
                }
            }
        }
        if smaller.is_none() {
            for removed in 0..graph.len() {
                let new_len = graph.len() - 1;
                let candidate: Graph = graph
                    .iter()
                    .enumerate()
                    .filter(|(row, _)| *row != removed)
                    .map(|(_, children)| {
                        children
                            .iter()
                            .copied()
                            .filter(|child| *child != removed)
                            .map(|child| {
                                if child >= graph.len() {
                                    new_len
                                } else {
                                    child - usize::from(child > removed)
                                }
                            })
                            .collect()
                    })
                    .collect();
                if class(&candidate) == outcome && disagrees(&candidate) {
                    smaller = Some(candidate);
                    break;
                }
            }
        }
        match smaller {
            Some(candidate) => graph = candidate,
            None => return graph,
        }
    }
}

pub(super) fn chain(size: usize) -> Graph {
    (0..size)
        .map(|row| if row == 0 { vec![] } else { vec![row - 1] })
        .collect()
}

pub(super) fn wide(size: usize) -> Graph {
    let mut graph = vec![vec![]; size];
    if size > 1 {
        graph[size - 1] = (0..size - 1).chain([0, 0]).collect();
    }
    graph
}
