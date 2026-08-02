use std::convert::Infallible;

use super::model::{
    SealedSemanticDagBorrow, SealedSemanticDagBorrowFailure, SealedSemanticDagError,
    SealedSemanticDagOwner, SealedSemanticDagReleaseFailure, SealedSemanticDagReleaseReport,
};
use super::SealedSemanticDagRuntime;

impl SealedSemanticDagRuntime {
    pub fn retain(
        &mut self,
        owner: &SealedSemanticDagOwner,
    ) -> Result<SealedSemanticDagOwner, SealedSemanticDagError> {
        let retained = self
            .store_mut(owner.store, owner.value_type)?
            .store
            .retain(&owner.owner)?;
        Ok(SealedSemanticDagOwner {
            store: owner.store,
            root: owner.root,
            nodes: owner.nodes,
            cells: owner.cells,
            value_type: owner.value_type,
            owner: retained,
        })
    }

    pub fn begin_borrow(
        &mut self,
        owner: &SealedSemanticDagOwner,
    ) -> Result<SealedSemanticDagBorrow, SealedSemanticDagError> {
        let typed = self.store_mut(owner.store, owner.value_type)?;
        let root = typed.store.root(&owner.owner, owner.root)?;
        let borrow = typed.store.begin_borrow(root)?;
        Ok(SealedSemanticDagBorrow {
            store: owner.store,
            root: owner.root,
            nodes: owner.nodes,
            cells: owner.cells,
            value_type: owner.value_type,
            borrow,
        })
    }

    pub fn end_borrow(
        &mut self,
        borrow: SealedSemanticDagBorrow,
    ) -> Result<(), SealedSemanticDagBorrowFailure> {
        let index = borrow.store as usize;
        if self
            .stores
            .get(index)
            .is_none_or(|typed| typed.value_type != borrow.value_type)
        {
            return Err(borrow_failure(
                SealedSemanticDagError::CorruptRegion,
                borrow,
            ));
        }
        let SealedSemanticDagBorrow {
            store,
            root,
            nodes,
            cells,
            value_type,
            borrow: inner,
        } = borrow;
        match self.stores[index].store.end_borrow(inner) {
            Ok(()) => Ok(()),
            Err((error, inner)) => Err(borrow_failure(
                error.into(),
                SealedSemanticDagBorrow {
                    store,
                    root,
                    nodes,
                    cells,
                    value_type,
                    borrow: inner,
                },
            )),
        }
    }

    pub fn release(
        &mut self,
        owner: SealedSemanticDagOwner,
    ) -> Result<SealedSemanticDagReleaseReport, SealedSemanticDagReleaseFailure> {
        let index = owner.store as usize;
        if self
            .stores
            .get(index)
            .is_none_or(|typed| typed.value_type != owner.value_type)
        {
            return Err(release_failure(
                SealedSemanticDagError::CorruptRegion,
                owner,
            ));
        }
        let SealedSemanticDagOwner {
            store,
            root,
            nodes,
            cells,
            value_type,
            owner: inner,
        } = owner;
        let typed = &mut self.stores[index];
        match typed.store.release(&mut self.runtime, inner, |_| {
            Result::<(), Infallible>::Ok(())
        }) {
            Ok(report) => {
                drop(report.drop_failures);
                Ok(SealedSemanticDagReleaseReport {
                    regions_released: report.regions_released,
                    cells_released: report.objects_released,
                    dependency_releases: report.dependency_releases,
                })
            }
            Err((error, inner)) => Err(release_failure(
                error.into(),
                SealedSemanticDagOwner {
                    store,
                    root,
                    nodes,
                    cells,
                    value_type,
                    owner: inner,
                },
            )),
        }
    }
}

fn borrow_failure(
    error: SealedSemanticDagError,
    borrow: SealedSemanticDagBorrow,
) -> SealedSemanticDagBorrowFailure {
    SealedSemanticDagBorrowFailure {
        error,
        borrow: Box::new(borrow),
    }
}

fn release_failure(
    error: SealedSemanticDagError,
    owner: SealedSemanticDagOwner,
) -> SealedSemanticDagReleaseFailure {
    SealedSemanticDagReleaseFailure { error, owner }
}
