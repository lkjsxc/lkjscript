use super::super::{dag_children, SemanticDagSnapshot, SemanticDagType};
use super::model::{SealedSemanticDagError, SealedSemanticDagFailure, SealedSemanticDagOwner};
use super::{SealedDagCell, SealedSemanticDagRuntime};
use crate::structural::{SealedBuilder, SealedRef, SealedRegionStore, StructuralRuntime};

impl SealedSemanticDagRuntime {
    pub fn rehydrate(
        &mut self,
        snapshot: SemanticDagSnapshot,
        expected_root: SemanticDagType,
        type_closure: &[SemanticDagType],
    ) -> Result<SealedSemanticDagOwner, SealedSemanticDagFailure> {
        let plan = match self.plan(&snapshot, expected_root, type_closure) {
            Ok(plan) => plan,
            Err(error) => return Err(failure(error, snapshot)),
        };
        let store_index = match self.ensure_store(expected_root) {
            Ok(index) => index,
            Err(error) => return Err(failure(error, snapshot)),
        };
        let typed = &mut self.stores[store_index as usize];
        let builder = match typed.store.begin(&mut self.runtime) {
            Ok(builder) => builder,
            Err(error) => return Err(failure(error.into(), snapshot)),
        };
        let mut references = Vec::new();
        if references.try_reserve_exact(plan.cells.len()).is_err() {
            return Err(abort_failure(
                &mut typed.store,
                &mut self.runtime,
                builder,
                SealedSemanticDagError::AllocationFailed,
                snapshot,
            ));
        }
        for &cell in &plan.cells {
            match typed.store.allocate(&builder, cell) {
                Ok(reference) => references.push(reference),
                Err(error) => {
                    return Err(abort_failure(
                        &mut typed.store,
                        &mut self.runtime,
                        builder,
                        error.into(),
                        snapshot,
                    ));
                }
            }
        }
        let mut edge_error = None;
        'nodes: for (parent, node) in snapshot.nodes().iter().enumerate() {
            for child in dag_children(&node.payload) {
                if let Err(error) =
                    add_edge(&mut typed.store, &builder, &references, parent, child.get())
                {
                    edge_error = Some(error);
                    break 'nodes;
                }
            }
        }
        if let Some(error) = edge_error {
            return Err(abort_failure(
                &mut typed.store,
                &mut self.runtime,
                builder,
                error,
                snapshot,
            ));
        }
        let owner = match typed.store.seal_batch(&mut self.runtime, vec![builder]) {
            Ok(mut owners) => match owners.pop() {
                Some(owner) if owners.is_empty() => owner,
                _ => return Err(failure(SealedSemanticDagError::CorruptRegion, snapshot)),
            },
            Err(seal) => {
                let error = seal.error.into();
                for builder in seal.builders {
                    typed
                        .store
                        .rollback_dropless_builder(&mut self.runtime, builder);
                }
                return Err(failure(error, snapshot));
            }
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

fn abort_failure(
    store: &mut SealedRegionStore<SealedDagCell, ()>,
    runtime: &mut StructuralRuntime,
    builder: SealedBuilder<SealedDagCell, ()>,
    error: SealedSemanticDagError,
    snapshot: SemanticDagSnapshot,
) -> SealedSemanticDagFailure {
    store.rollback_dropless_builder(runtime, builder);
    failure(error, snapshot)
}

fn failure(
    error: SealedSemanticDagError,
    snapshot: SemanticDagSnapshot,
) -> SealedSemanticDagFailure {
    SealedSemanticDagFailure {
        error,
        snapshot: Box::new(snapshot),
    }
}
