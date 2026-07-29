mod lifecycle;

use std::collections::BTreeMap;

use lkjscript_core::{DomainKey, RootKey};

use crate::{
    DataOwnerId, GenerationTable, NoLiveLoanProof, OwnerHomeTable, OwnerMetrics, RemoteRelease,
    ResourceError, ResourceResult, TaskId, WorkerId,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StructuralNoLiveLoanProof {
    domain: DomainKey,
    owner: DataOwnerId,
    proof: NoLiveLoanProof,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StructuralRemoteRelease {
    pub domain: DomainKey,
    pub from_task: TaskId,
    authority: RemoteRelease,
}

#[derive(Clone, Debug)]
pub struct StructuralOwnerHomeTable {
    owners: GenerationTable<DataOwnerId>,
    homes: OwnerHomeTable,
    by_domain: BTreeMap<DomainKey, DataOwnerId>,
    by_owner: BTreeMap<DataOwnerId, DomainKey>,
    owner_limit: usize,
}

impl StructuralOwnerHomeTable {
    pub fn new(owner_limit: usize, release_limit: usize) -> Self {
        Self {
            owners: GenerationTable::new(owner_limit),
            homes: OwnerHomeTable::new(owner_limit, release_limit),
            by_domain: BTreeMap::new(),
            by_owner: BTreeMap::new(),
            owner_limit,
        }
    }

    pub fn register(&mut self, domain: DomainKey, home: WorkerId) -> ResourceResult<DataOwnerId> {
        if self.by_domain.len() >= self.owner_limit || self.by_domain.contains_key(&domain) {
            return Err(ResourceError::new(
                "structural-owner-capacity",
                "structural owner table full or duplicate",
            ));
        }
        let owner = self.owners.allocate()?;
        if let Err(error) = self.homes.insert(owner, home) {
            self.owners.release(owner)?;
            return Err(error);
        }
        self.by_domain.insert(domain, owner);
        self.by_owner.insert(owner, domain);
        Ok(owner)
    }

    pub fn register_root(&mut self, root: RootKey, home: WorkerId) -> ResourceResult<DataOwnerId> {
        self.register(root.domain(), home)
    }

    pub fn owner(&self, domain: DomainKey) -> ResourceResult<DataOwnerId> {
        self.by_domain
            .get(&domain)
            .copied()
            .ok_or_else(|| ResourceError::new("structural-owner-stale", "domain is not registered"))
    }

    pub fn home(&self, domain: DomainKey) -> ResourceResult<WorkerId> {
        self.homes.home(self.owner(domain)?)
    }

    pub fn begin_loan(&mut self, domain: DomainKey) -> ResourceResult<()> {
        self.homes.begin_loan(self.owner(domain)?)
    }

    pub fn end_loan(&mut self, domain: DomainKey) -> ResourceResult<()> {
        self.homes.end_loan(self.owner(domain)?)
    }

    pub fn prove_no_live_loan(
        &self,
        domain: DomainKey,
    ) -> ResourceResult<StructuralNoLiveLoanProof> {
        let owner = self.owner(domain)?;
        Ok(StructuralNoLiveLoanProof {
            domain,
            owner,
            proof: self.homes.prove_no_live_loan(owner)?,
        })
    }

    pub fn move_home(
        &mut self,
        domain: DomainKey,
        home: WorkerId,
        proof: StructuralNoLiveLoanProof,
    ) -> ResourceResult<()> {
        self.check_proof(domain, proof)?;
        self.homes.move_owner(proof.owner, home, proof.proof)
    }

    pub fn remote_release(&mut self, domain: DomainKey, from_task: TaskId) -> ResourceResult<()> {
        let owner = self.owner(domain)?;
        let home = self.homes.home(owner)?;
        let proof = self.homes.prove_no_live_loan(owner)?;
        self.homes
            .remote_release(home, RemoteRelease::new(owner, from_task), proof)
    }

    pub fn drain_releases(
        &mut self,
        home: WorkerId,
        limit: usize,
    ) -> ResourceResult<Vec<StructuralRemoteRelease>> {
        self.homes
            .drain_releases(home, limit)?
            .into_iter()
            .map(|release| {
                let domain = self.by_owner.get(&release.owner).copied().ok_or_else(|| {
                    ResourceError::new("structural-owner-stale", "release owner is not registered")
                })?;
                Ok(StructuralRemoteRelease {
                    domain,
                    from_task: release.from_task,
                    authority: release,
                })
            })
            .collect()
    }

    pub fn metrics(&self) -> OwnerMetrics {
        self.homes.metrics()
    }

    pub fn validate_empty(&self) -> ResourceResult<()> {
        if self.by_domain.is_empty() && self.by_owner.is_empty() {
            Ok(())
        } else {
            Err(ResourceError::new(
                "structural-owner-leak",
                "structural owners remain registered",
            ))
        }
    }

    fn check_proof(
        &self,
        domain: DomainKey,
        proof: StructuralNoLiveLoanProof,
    ) -> ResourceResult<()> {
        if proof.domain != domain || self.owner(domain)? != proof.owner {
            return Err(ResourceError::new(
                "structural-owner-proof",
                "proof belongs to another structural owner",
            ));
        }
        Ok(())
    }
}
