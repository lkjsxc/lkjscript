use super::super::{dag_children, SemanticDagSnapshot, SemanticDagType};
use super::model::{SealedSemanticDagError, SealedSemanticDagFailure, SealedSemanticDagOwner};
use super::planning::RehydrationPlan;
use super::{SealedDagCell, SealedSemanticDagRuntime, TypedSealedDagStore};
use crate::structural::{
    SealedBuilder, SealedOwner, SealedRef, SealedRegionStore, StructuralRuntime,
};

impl SealedSemanticDagRuntime {
    pub(super) fn rehydrate(
        &mut self,
        snapshot: SemanticDagSnapshot,
        expected_root: SemanticDagType,
        type_closure: &[SemanticDagType],
    ) -> Result<SealedSemanticDagOwner, SealedSemanticDagFailure> {
        let plan = match self.plan(&snapshot, expected_root, type_closure) {
            Ok(plan) => plan,
            Err(error) => return Err(failure(error, snapshot)),
        };
        let existing = self
            .stores
            .iter()
            .position(|typed| typed.value_type == expected_root);
        let (store_index, owner) = if let Some(index) = existing {
            let store_index = match u32::try_from(index) {
                Ok(index) => index,
                Err(_) => {
                    return Err(failure(
                        SealedSemanticDagError::ArithmeticOverflow,
                        snapshot,
                    ));
                }
            };
            let owner = match build(
                &mut self.stores[index].store,
                &mut self.runtime,
                &snapshot,
                &plan,
            ) {
                Ok(owner) => owner,
                Err(error) => return Err(failure(error, snapshot)),
            };
            (store_index, owner)
        } else {
            let store_index = match u32::try_from(self.stores.len()) {
                Ok(index) => index,
                Err(_) => {
                    return Err(failure(
                        SealedSemanticDagError::ArithmeticOverflow,
                        snapshot,
                    ));
                }
            };
            let mut typed = match self.fresh_store(expected_root) {
                Ok(store) => store,
                Err(error) => return Err(failure(error, snapshot)),
            };
            let owner = match build(&mut typed.store, &mut self.runtime, &snapshot, &plan) {
                Ok(owner) => owner,
                Err(error) => return Err(failure(error, snapshot)),
            };
            self.stores.push(typed);
            (store_index, owner)
        };
        Ok(SealedSemanticDagOwner {
            store: store_index,
            root: plan.root,
            nodes: plan.nodes,
            cells: plan.cell_count,
            value_type: expected_root,
            owner,
        })
    }

    fn fresh_store(
        &mut self,
        value_type: SemanticDagType,
    ) -> Result<TypedSealedDagStore, SealedSemanticDagError> {
        if self.stores.len() >= self.limits.max_domains as usize {
            return Err(
                crate::StructuralError::LimitExceeded(crate::StructuralLimit::Domains).into(),
            );
        }
        self.stores
            .try_reserve(1)
            .map_err(|_| SealedSemanticDagError::AllocationFailed)?;
        let store = SealedRegionStore::new(
            self.runtime.identity(),
            value_type.layout,
            value_type.semantic_type,
            self.limits,
        )?;
        Ok(TypedSealedDagStore { value_type, store })
    }
}

fn build(
    store: &mut SealedRegionStore<SealedDagCell, ()>,
    runtime: &mut StructuralRuntime,
    snapshot: &SemanticDagSnapshot,
    plan: &RehydrationPlan,
) -> Result<SealedOwner<SealedDagCell, ()>, SealedSemanticDagError> {
    let builder = store.begin(runtime)?;
    let mut references = Vec::new();
    if references.try_reserve_exact(plan.cells.len()).is_err() {
        return Err(abort_error(
            store,
            runtime,
            builder,
            SealedSemanticDagError::AllocationFailed,
        ));
    }
    for &cell in &plan.cells {
        match store.allocate(&builder, cell) {
            Ok(reference) => references.push(reference),
            Err(error) => return Err(abort_error(store, runtime, builder, error.into())),
        }
    }
    for (parent, node) in snapshot.nodes().iter().enumerate() {
        for child in dag_children(&node.payload) {
            if let Err(error) = add_edge(store, &builder, &references, parent, child.get()) {
                return Err(abort_error(store, runtime, builder, error));
            }
        }
    }
    match store.seal_batch(runtime, vec![builder]) {
        Ok(mut owners) => owners
            .pop()
            .filter(|_| owners.is_empty())
            .ok_or(SealedSemanticDagError::CorruptRegion),
        Err(seal) => {
            let error = seal.error.into();
            for builder in seal.builders {
                store.rollback_dropless_builder(runtime, builder);
            }
            Err(error)
        }
    }
}

fn add_edge(
    store: &mut SealedRegionStore<SealedDagCell, ()>,
    builder: &SealedBuilder<SealedDagCell, ()>,
    references: &[SealedRef<SealedDagCell>],
    parent: usize,
    child: u32,
) -> Result<(), SealedSemanticDagError> {
    let from = references
        .get(parent)
        .copied()
        .ok_or(SealedSemanticDagError::CorruptRegion)?;
    let to = references
        .get(child as usize)
        .copied()
        .ok_or(SealedSemanticDagError::CorruptRegion)?;
    store.add_internal_edge(builder, from, to)?;
    Ok(())
}

fn abort_error(
    store: &mut SealedRegionStore<SealedDagCell, ()>,
    runtime: &mut StructuralRuntime,
    builder: SealedBuilder<SealedDagCell, ()>,
    error: SealedSemanticDagError,
) -> SealedSemanticDagError {
    store.rollback_dropless_builder(runtime, builder);
    error
}

fn failure(
    error: SealedSemanticDagError,
    snapshot: SemanticDagSnapshot,
) -> SealedSemanticDagFailure {
    SealedSemanticDagFailure::new(error, snapshot)
}
