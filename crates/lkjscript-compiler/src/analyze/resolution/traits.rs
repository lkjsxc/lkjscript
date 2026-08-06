use crate::analyze::*;
use std::collections::btree_map::Entry;
use std::collections::{BTreeMap, VecDeque};

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum CanonicalTypeNode {
    Atom(u8),
    Named(u8, String),
    Enum([u8; 32], Vec<usize>),
    List(usize),
    Function(Vec<usize>, usize),
    Forall(Vec<String>, usize),
}

#[derive(Default)]
struct CanonicalTypes {
    nodes: BTreeMap<CanonicalTypeNode, usize>,
    completed: HashMap<*const Type, usize>,
}

impl CanonicalTypes {
    fn intern(&mut self, root: &Type) -> Result<usize> {
        enum Work<'a> {
            Visit(&'a Type),
            Finish(&'a Type),
        }

        if let Some(id) = self.completed.get(&std::ptr::from_ref(root)) {
            return Ok(*id);
        }
        let mut work = vec![Work::Visit(root)];
        while let Some(item) = work.pop() {
            match item {
                Work::Visit(ty) => {
                    let pointer = std::ptr::from_ref(ty);
                    if self.completed.contains_key(&pointer) {
                        continue;
                    }
                    work.try_reserve(1)
                        .map_err(|_| Error::msg("trait canonicalization work allocation failed"))?;
                    work.push(Work::Finish(ty));
                    match ty {
                        Type::Enum { arguments, .. } => {
                            work.try_reserve(arguments.len()).map_err(|_| {
                                Error::msg("trait canonicalization work allocation failed")
                            })?;
                            work.extend(arguments.iter().rev().map(Work::Visit));
                        }
                        Type::List(inner) => work.push(Work::Visit(inner)),
                        Type::Fn { params, ret } => {
                            let additional = params.len().checked_add(1).ok_or_else(|| {
                                Error::msg("trait canonicalization work size overflow")
                            })?;
                            work.try_reserve(additional).map_err(|_| {
                                Error::msg("trait canonicalization work allocation failed")
                            })?;
                            work.push(Work::Visit(ret));
                            work.extend(params.iter().rev().map(Work::Visit));
                        }
                        Type::Forall { body, .. } => work.push(Work::Visit(body)),
                        _ => {}
                    }
                }
                Work::Finish(ty) => {
                    let child = |value: &Type| {
                        self.completed
                            .get(&std::ptr::from_ref(value))
                            .copied()
                            .ok_or_else(|| Error::msg("trait canonicalization lost a child type"))
                    };
                    let node = match ty {
                        Type::Never => CanonicalTypeNode::Atom(0),
                        Type::Unit => CanonicalTypeNode::Atom(1),
                        Type::Bool => CanonicalTypeNode::Atom(2),
                        Type::I64 => CanonicalTypeNode::Atom(3),
                        Type::F64 => CanonicalTypeNode::Atom(4),
                        Type::Str => CanonicalTypeNode::Atom(5),
                        Type::Bytes => CanonicalTypeNode::Atom(6),
                        Type::ByteVector => CanonicalTypeNode::Atom(7),
                        Type::ByteSlice => CanonicalTypeNode::Atom(8),
                        Type::ByteSliceMut => CanonicalTypeNode::Atom(9),
                        Type::Path => CanonicalTypeNode::Atom(10),
                        Type::Capability(kind) => {
                            CanonicalTypeNode::Named(11, kind.as_str().to_owned())
                        }
                        Type::Symbol => CanonicalTypeNode::Atom(12),
                        Type::Resource(kind) => {
                            CanonicalTypeNode::Named(13, kind.as_str().to_owned())
                        }
                        Type::Product(name) => CanonicalTypeNode::Named(14, name.clone()),
                        Type::Enum { id, arguments, .. } => CanonicalTypeNode::Enum(
                            id.bytes(),
                            arguments.iter().map(child).collect::<Result<Vec<_>>>()?,
                        ),
                        Type::Param(name) => CanonicalTypeNode::Named(16, name.clone()),
                        Type::List(inner) => CanonicalTypeNode::List(child(inner)?),
                        Type::Fn { params, ret } => CanonicalTypeNode::Function(
                            params.iter().map(child).collect::<Result<Vec<_>>>()?,
                            child(ret)?,
                        ),
                        Type::Forall { vars, body } => {
                            CanonicalTypeNode::Forall(vars.clone(), child(body)?)
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
            .ok_or_else(|| Error::msg("trait canonicalization omitted its root"))
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

impl Resolver<'_> {
    pub(in crate::analyze) fn solve_trait_bound(
        &self,
        function: &str,
        trait_id: TraitId,
        ty: &Type,
    ) -> Result<TraitWitness> {
        let definition = self
            .analyzer
            .traits
            .get(trait_id.index().unwrap_or(usize::MAX))
            .filter(|definition| definition.id == trait_id)
            .ok_or_else(|| {
                self.error(format!(
                    "{function}: bound references unknown TraitId {}",
                    trait_id.raw()
                ))
            })?;
        let kind = if let Some(core_trait) = definition.core.filter(|role| role.is_auto()) {
            match self.auto_trait_holds(core_trait, ty)? {
                true => TraitWitnessKind::AutoTrait,
                false => {
                    return Err(self.error(format!(
                        "{function}: type {ty} does not satisfy trait {}",
                        definition.name
                    )))
                }
            }
        } else {
            let Type::Product(name) = ty else {
                return Err(self.error(format!(
                    "{function}: type {ty} has no exact implementation of trait {}",
                    definition.name
                )));
            };
            let product = self
                .analyzer
                .product_names
                .get(name)
                .copied()
                .ok_or_else(|| self.error(format!("{function}: unknown product type {name}")))?;
            let implementation = self
                .analyzer
                .implementation_index
                .get(&(trait_id, product))
                .copied()
                .ok_or_else(|| {
                    self.error(format!(
                        "{function}: product {name} does not implement trait {}",
                        definition.name
                    ))
                })?;
            TraitWitnessKind::Explicit(implementation)
        };
        Ok(TraitWitness {
            trait_id,
            ty: ty.clone(),
            kind,
        })
    }

    pub(in crate::analyze) fn auto_trait_holds(
        &self,
        core_trait: CoreTrait,
        ty: &Type,
    ) -> Result<bool> {
        let role = core_trait_key(core_trait);
        let mut canonical = CanonicalTypes::default();
        let root = (role, canonical.intern(ty)?);
        let mut memo = BTreeMap::<(u8, usize), MemoState>::new();
        memo.insert(root, MemoState::Visiting);
        let mut pending = vec![(root, ty)];

        while let Some((key, subject)) = pending.pop() {
            let (intrinsic, children) = self.auto_trait_dependencies(core_trait, subject)?;
            let mut dependencies = Vec::new();
            dependencies
                .try_reserve(children.len())
                .map_err(|_| self.error("trait solver dependency allocation failed"))?;
            for child in children {
                let child_key = (role, canonical.intern(child)?);
                dependencies.push(child_key);
                if let Entry::Vacant(entry) = memo.entry(child_key) {
                    entry.insert(MemoState::Visiting);
                    pending
                        .try_reserve(1)
                        .map_err(|_| self.error("trait solver work allocation failed"))?;
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
                return Err(self.error("trait solver left a visiting obligation"));
            };
            values[index] = *intrinsic;
            if !intrinsic {
                false_queue.push_back(index);
            }
            for dependency in dependencies {
                let dependency_index = indexes
                    .get(dependency)
                    .copied()
                    .ok_or_else(|| self.error("trait solver lost a canonical dependency"))?;
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
            .ok_or_else(|| self.error("trait solver omitted its root obligation"))?;
        Ok(values[root_index])
    }

    fn auto_trait_dependencies<'a>(
        &'a self,
        core_trait: CoreTrait,
        ty: &'a Type,
    ) -> Result<(bool, Vec<&'a Type>)> {
        let result = match core_trait {
            CoreTrait::Copy => match ty {
                Type::Unit
                | Type::Bool
                | Type::I64
                | Type::F64
                | Type::Capability(_)
                | Type::Str
                | Type::Path
                | Type::Symbol
                | Type::ByteSlice => (true, Vec::new()),
                Type::Never
                | Type::Bytes
                | Type::ByteVector
                | Type::ByteSliceMut
                | Type::Resource(_)
                | Type::Fn { .. }
                | Type::Forall { .. }
                | Type::Param(_) => (false, Vec::new()),
                Type::List(inner) => (true, vec![inner.as_ref()]),
                Type::Enum { id, arguments, .. }
                    if matches!(
                        id.bytes(),
                        lkjscript_core::OPTION_ID | lkjscript_core::RESULT_ID
                    ) =>
                {
                    (true, arguments.iter().collect())
                }
                Type::Enum { .. } => (false, Vec::new()),
                Type::Product(name) => {
                    let product = self.analyzer.product_by_name(name)?;
                    (true, product.fields.iter().map(|field| &field.ty).collect())
                }
            },
            CoreTrait::Send | CoreTrait::Sync => (
                matches!(ty, Type::Unit | Type::Bool | Type::I64 | Type::F64),
                Vec::new(),
            ),
            CoreTrait::Clone | CoreTrait::Drop => (false, Vec::new()),
        };
        Ok(result)
    }
}

const fn core_trait_key(core_trait: CoreTrait) -> u8 {
    match core_trait {
        CoreTrait::Copy => 0,
        CoreTrait::Clone => 1,
        CoreTrait::Drop => 2,
        CoreTrait::Send => 3,
        CoreTrait::Sync => 4,
    }
}
