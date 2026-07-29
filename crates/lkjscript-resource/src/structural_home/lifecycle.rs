use super::{StructuralNoLiveLoanProof, StructuralOwnerHomeTable, StructuralRemoteRelease};
use crate::{ResourceError, ResourceResult};
use lkjscript_core::DomainKey;

impl StructuralOwnerHomeTable {
    pub fn complete_release<F>(
        &mut self,
        release: StructuralRemoteRelease,
        teardown: F,
    ) -> ResourceResult<()>
    where
        F: FnOnce() -> ResourceResult<()>,
    {
        let owner = self.owner(release.domain)?;
        if owner != release.authority.owner || !self.owners.contains(owner) {
            return Err(ResourceError::new(
                "structural-release-authority",
                "remote structural release is stale",
            ));
        }
        self.homes.complete_release(release.authority, teardown)?;
        self.owners.release(owner)?;
        self.by_domain.remove(&release.domain);
        self.by_owner.remove(&owner);
        Ok(())
    }

    pub fn unregister(
        &mut self,
        domain: DomainKey,
        proof: StructuralNoLiveLoanProof,
    ) -> ResourceResult<()> {
        self.check_proof(domain, proof)?;
        if !self.owners.contains(proof.owner) {
            return Err(ResourceError::new(
                "structural-owner-stale",
                "owner generation is not live",
            ));
        }
        self.homes.remove(proof.owner, proof.proof)?;
        self.owners.release(proof.owner)?;
        self.by_domain.remove(&domain);
        self.by_owner.remove(&proof.owner);
        Ok(())
    }
}
