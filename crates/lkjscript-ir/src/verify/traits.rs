use std::collections::btree_map::Entry;
use std::collections::{BTreeMap, HashMap, VecDeque};

use crate::verify::*;
use crate::{Program, SsaType, TraitRole, TraitWitnessKind};

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum CanonicalSsaTypeNode {
    Atom(u8),
    Named(u8, String),
    Id(u8, u64),
    Enum([u8; 32], Vec<usize>),
    List(usize),
    Function {
        type_parameters: Vec<String>,
        bounds: Vec<(String, u64)>,
        witnesses: Vec<(String, Vec<u8>)>,
        parameters: Vec<usize>,
        result: usize,
    },
}

#[derive(Default)]
struct CanonicalSsaTypes {
    nodes: BTreeMap<CanonicalSsaTypeNode, usize>,
    completed: HashMap<*const SsaType, usize>,
}

impl CanonicalSsaTypes {
    fn intern(&mut self, root: &SsaType) -> crate::Result<usize> {
        enum Work<'a> {
            Visit(&'a SsaType),
            Finish(&'a SsaType),
        }
        if let Some(id) = self.completed.get(&std::ptr::from_ref(root)) {
            return Ok(*id);
        }
        let mut pending = vec![Work::Visit(root)];
        while let Some(item) = pending.pop() {
            match item {
                Work::Visit(ty) => {
                    let pointer = std::ptr::from_ref(ty);
                    if self.completed.contains_key(&pointer) {
                        continue;
                    }
                    pending.try_reserve(1).map_err(|_| {
                        crate::IrError::new("SSA trait canonicalization allocation failed")
                    })?;
                    pending.push(Work::Finish(ty));
                    match ty {
                        SsaType::Enum { arguments, .. } => {
                            pending.try_reserve(arguments.len()).map_err(|_| {
                                crate::IrError::new("SSA trait canonicalization allocation failed")
                            })?;
                            pending.extend(arguments.iter().rev().map(Work::Visit));
                        }
                        SsaType::List(inner) => pending.push(Work::Visit(inner)),
                        SsaType::Function(signature) => {
                            let additional =
                                signature.parameters.len().checked_add(1).ok_or_else(|| {
                                    crate::IrError::new("SSA trait canonicalization size overflow")
                                })?;
                            pending.try_reserve(additional).map_err(|_| {
                                crate::IrError::new("SSA trait canonicalization allocation failed")
                            })?;
                            pending.push(Work::Visit(&signature.result));
                            pending.extend(signature.parameters.iter().rev().map(Work::Visit));
                        }
                        _ => {}
                    }
                }
                Work::Finish(ty) => {
                    let child = |value: &SsaType| {
                        self.completed
                            .get(&std::ptr::from_ref(value))
                            .copied()
                            .ok_or_else(|| {
                                crate::IrError::new("SSA trait canonicalization lost a child")
                            })
                    };
                    let node = match ty {
                        SsaType::Unit => CanonicalSsaTypeNode::Atom(0),
                        SsaType::Bool => CanonicalSsaTypeNode::Atom(1),
                        SsaType::I64 => CanonicalSsaTypeNode::Atom(2),
                        SsaType::F64 => CanonicalSsaTypeNode::Atom(3),
                        SsaType::Str => CanonicalSsaTypeNode::Atom(4),
                        SsaType::Symbol => CanonicalSsaTypeNode::Atom(5),
                        SsaType::Bytes => CanonicalSsaTypeNode::Atom(6),
                        SsaType::ByteVector => CanonicalSsaTypeNode::Atom(7),
                        SsaType::ByteSlice => CanonicalSsaTypeNode::Atom(8),
                        SsaType::ByteSliceMut => CanonicalSsaTypeNode::Atom(9),
                        SsaType::Path => CanonicalSsaTypeNode::Atom(10),
                        SsaType::Capability(kind) => {
                            CanonicalSsaTypeNode::Named(11, format!("{kind:?}"))
                        }
                        SsaType::Resource(kind) => {
                            CanonicalSsaTypeNode::Named(12, format!("{kind:?}"))
                        }
                        SsaType::StructuralDestination(id) => {
                            CanonicalSsaTypeNode::Id(13, id.raw())
                        }
                        SsaType::Product(id) => CanonicalSsaTypeNode::Id(14, id.raw()),
                        SsaType::Enum { id, arguments } => CanonicalSsaTypeNode::Enum(
                            id.bytes(),
                            arguments
                                .iter()
                                .map(child)
                                .collect::<crate::Result<Vec<_>>>()?,
                        ),
                        SsaType::List(inner) => CanonicalSsaTypeNode::List(child(inner)?),
                        SsaType::Function(signature) => CanonicalSsaTypeNode::Function {
                            type_parameters: signature.type_parameters.clone(),
                            bounds: signature
                                .bounds
                                .iter()
                                .map(|bound| (bound.parameter.clone(), bound.trait_id.raw()))
                                .collect(),
                            witnesses: signature
                                .memory_witness_parameters
                                .iter()
                                .map(|witness| {
                                    (
                                        witness.parameter.clone(),
                                        witness
                                            .operations
                                            .iter()
                                            .flat_map(|operation| {
                                                format!("{operation:?}").into_bytes()
                                            })
                                            .collect(),
                                    )
                                })
                                .collect(),
                            parameters: signature
                                .parameters
                                .iter()
                                .map(child)
                                .collect::<crate::Result<Vec<_>>>()?,
                            result: child(&signature.result)?,
                        },
                        SsaType::TypeParameter(name) => {
                            CanonicalSsaTypeNode::Named(18, name.clone())
                        }
                    };
                    let next = self.nodes.len();
                    let id = *self.nodes.entry(node).or_insert(next);
                    self.completed.insert(std::ptr::from_ref(ty), id);
                }
            }
        }
        self.completed
            .get(&std::ptr::from_ref(root))
            .copied()
            .ok_or_else(|| crate::IrError::new("SSA trait canonicalization omitted its root"))
    }
}

#[derive(Clone, Debug)]
enum MemoState {
    Visiting,
    Resolved {
        intrinsic: bool,
        dependencies: Vec<(u8, usize)>,
        value: bool,
    },
}

pub(crate) fn verify_witness(
    program: &Program,
    witness: &crate::TraitWitness,
) -> crate::Result<()> {
    let trait_metadata = trait_by_id(program, witness.trait_id)?;
    match witness.kind {
        TraitWitnessKind::AutoTrait => {
            if !trait_metadata.role.is_auto() {
                return fail("SSA auto-trait witness references a non-auto trait");
            }
            if !auto_trait_holds(program, trait_metadata.role, &witness.ty)? {
                return fail("SSA auto-trait witness asserts an unsupported type fact");
            }
        }
        TraitWitnessKind::Explicit(implementation_id) => {
            let implementation = impl_by_id(program, implementation_id)?;
            let SsaType::Product(product) = witness.ty else {
                return fail("SSA explicit marker witness does not target a product");
            };
            if implementation.trait_id != witness.trait_id || implementation.product != product {
                return fail(
                    "SSA explicit marker witness identity does not match trait and product",
                );
            }
        }
    }
    Ok(())
}

pub(crate) fn auto_trait_holds(
    program: &Program,
    role: TraitRole,
    ty: &SsaType,
) -> crate::Result<bool> {
    let role_key = trait_role_key(role);
    let mut canonical = CanonicalSsaTypes::default();
    let root = (role_key, canonical.intern(ty)?);
    let mut memo = BTreeMap::<(u8, usize), MemoState>::new();
    memo.insert(root, MemoState::Visiting);
    let mut pending = vec![(root, ty)];
    while let Some((key, subject)) = pending.pop() {
        let (intrinsic, children) = auto_trait_dependencies(program, role, subject)?;
        let mut dependencies = Vec::new();
        dependencies
            .try_reserve(children.len())
            .map_err(|_| crate::IrError::new("SSA trait dependency allocation failed"))?;
        for child in children {
            let child_key = (role_key, canonical.intern(child)?);
            dependencies.push(child_key);
            if let Entry::Vacant(entry) = memo.entry(child_key) {
                entry.insert(MemoState::Visiting);
                pending
                    .try_reserve(1)
                    .map_err(|_| crate::IrError::new("SSA trait work allocation failed"))?;
                pending.push((child_key, child));
            }
        }
        memo.insert(
            key,
            MemoState::Resolved {
                intrinsic,
                dependencies,
                value: intrinsic,
            },
        );
    }

    let keys: Vec<_> = memo.keys().copied().collect();
    let indexes: BTreeMap<_, _> = keys
        .iter()
        .copied()
        .enumerate()
        .map(|(index, key)| (key, index))
        .collect();
    let mut dependents = vec![Vec::new(); keys.len()];
    let mut values = vec![true; keys.len()];
    let mut false_queue = VecDeque::new();
    for (index, key) in keys.iter().enumerate() {
        let Some(MemoState::Resolved {
            intrinsic,
            dependencies,
            ..
        }) = memo.get(key)
        else {
            return fail("SSA trait solver left a visiting obligation");
        };
        values[index] = *intrinsic;
        if !intrinsic {
            false_queue.push_back(index);
        }
        for dependency in dependencies {
            let dependency_index = indexes
                .get(dependency)
                .copied()
                .ok_or_else(|| crate::IrError::new("SSA trait solver lost a dependency"))?;
            dependents[dependency_index].push(index);
        }
    }
    while let Some(failed) = false_queue.pop_front() {
        for dependent in &dependents[failed] {
            if values[*dependent] {
                values[*dependent] = false;
                false_queue.push_back(*dependent);
            }
        }
    }
    for (index, key) in keys.iter().enumerate() {
        if let Some(MemoState::Resolved { value, .. }) = memo.get_mut(key) {
            *value = values[index];
        }
    }
    let root_index = indexes
        .get(&root)
        .copied()
        .ok_or_else(|| crate::IrError::new("SSA trait solver omitted its root"))?;
    Ok(values[root_index])
}

fn auto_trait_dependencies<'a>(
    program: &'a Program,
    role: TraitRole,
    ty: &'a SsaType,
) -> crate::Result<(bool, Vec<&'a SsaType>)> {
    let result = match role {
        TraitRole::Copy => match ty {
            SsaType::Unit
            | SsaType::Bool
            | SsaType::I64
            | SsaType::F64
            | SsaType::Capability(_)
            | SsaType::Str
            | SsaType::Path
            | SsaType::Symbol
            | SsaType::ByteSlice => (true, Vec::new()),
            SsaType::Bytes
            | SsaType::ByteVector
            | SsaType::ByteSliceMut
            | SsaType::Resource(_)
            | SsaType::StructuralDestination(_)
            | SsaType::Function(_)
            | SsaType::TypeParameter(_) => (false, Vec::new()),
            SsaType::List(inner) => (true, vec![inner.as_ref()]),
            SsaType::Enum { id, arguments }
                if matches!(
                    id.bytes(),
                    crate::prelude_contract::OPTION_ID | crate::prelude_contract::RESULT_ID
                ) =>
            {
                (true, arguments.iter().collect())
            }
            SsaType::Enum { .. } => (false, Vec::new()),
            SsaType::Product(product) => {
                let metadata = product_by_id(program, *product)?;
                (
                    true,
                    metadata.fields.iter().map(|field| &field.ty).collect(),
                )
            }
        },
        TraitRole::Send | TraitRole::Sync => (
            matches!(
                ty,
                SsaType::Unit | SsaType::Bool | SsaType::I64 | SsaType::F64
            ),
            Vec::new(),
        ),
        TraitRole::Clone | TraitRole::Drop | TraitRole::User => (false, Vec::new()),
    };
    Ok(result)
}

const fn trait_role_key(role: TraitRole) -> u8 {
    match role {
        TraitRole::Copy => 0,
        TraitRole::Clone => 1,
        TraitRole::Drop => 2,
        TraitRole::Send => 3,
        TraitRole::Sync => 4,
        TraitRole::User => 5,
    }
}
