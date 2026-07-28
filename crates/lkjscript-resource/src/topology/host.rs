use super::{FactCertainty, FactSource, HardwareTopology};
use crate::{CpuSet, ResourceError, ResourceResult};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ObservedFact<T> {
    pub value: T,
    pub source: FactSource,
    pub certainty: FactCertainty,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HostSchedulerRecord {
    pub online: ObservedFact<CpuSet>,
    pub allowed: ObservedFact<CpuSet>,
    pub quota_workers: ObservedFact<Option<usize>>,
    pub topology: ObservedFact<HardwareTopology>,
}

impl HostSchedulerRecord {
    pub fn validate(&self) -> ResourceResult<()> {
        self.topology.value.validate()?;
        if self
            .allowed
            .value
            .as_slice()
            .iter()
            .any(|cpu| !self.online.value.contains(*cpu))
        {
            return Err(ResourceError::new(
                "host-offline",
                "allowed CPU is not online",
            ));
        }
        Ok(())
    }
}
