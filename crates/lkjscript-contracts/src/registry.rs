use std::collections::BTreeMap;
use std::fmt;

use crate::{
    canonical_bytes, sha256, ContractDescriptor, ContractDigest, ContractError, ContractName,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RegisteredContract {
    descriptor: ContractDescriptor,
    canonical_bytes: Vec<u8>,
    digest: ContractDigest,
}

impl RegisteredContract {
    pub fn descriptor(&self) -> &ContractDescriptor {
        &self.descriptor
    }

    pub fn canonical_bytes(&self) -> &[u8] {
        &self.canonical_bytes
    }

    pub const fn digest(&self) -> ContractDigest {
        self.digest
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ContractSet {
    records: BTreeMap<ContractName, RegisteredContract>,
}

impl ContractSet {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(
        &mut self,
        descriptor: ContractDescriptor,
    ) -> Result<ContractDigest, ContractError> {
        if self.records.contains_key(&descriptor.name) {
            return Err(ContractError::DuplicateContract(
                descriptor.name.as_str().to_owned(),
            ));
        }
        for dependency in &descriptor.dependencies {
            let record = self.records.get(&dependency.name).ok_or_else(|| {
                ContractError::MissingDependency(dependency.name.as_str().to_owned())
            })?;
            if record.digest.as_bytes() != dependency.digest {
                return Err(ContractError::DependencyMismatch(
                    dependency.name.as_str().to_owned(),
                ));
            }
        }
        let bytes = canonical_bytes(&descriptor)?;
        let digest = ContractDigest::from_bytes(sha256(&bytes));
        self.records.insert(
            descriptor.name.clone(),
            RegisteredContract {
                descriptor,
                canonical_bytes: bytes,
                digest,
            },
        );
        Ok(digest)
    }

    pub fn get(&self, name: &str) -> Option<&RegisteredContract> {
        self.records
            .iter()
            .find_map(|(key, value)| (key.as_str() == name).then_some(value))
    }

    pub fn iter(&self) -> impl Iterator<Item = &RegisteredContract> {
        self.records.values()
    }

    pub fn len(&self) -> usize {
        self.records.len()
    }

    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContractMismatch {
    pub name: ContractName,
    pub expected: ContractDigest,
    pub actual: ContractDigest,
    pub producer: String,
    pub consumer: String,
}

impl fmt::Display for ContractMismatch {
    fn fmt(&self, output: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            output,
            concat!(
                "contract mismatch for {}: expected {}, actual {}; ",
                "producer={}, consumer={}; update the producer or rebuild the artifact"
            ),
            self.name.as_str(),
            self.expected,
            self.actual,
            self.producer,
            self.consumer
        )
    }
}

impl std::error::Error for ContractMismatch {}

pub fn require_exact(
    name: ContractName,
    expected: ContractDigest,
    actual: ContractDigest,
    producer: impl Into<String>,
    consumer: impl Into<String>,
) -> Result<(), Box<ContractMismatch>> {
    if expected == actual {
        Ok(())
    } else {
        Err(Box::new(ContractMismatch {
            name,
            expected,
            actual,
            producer: producer.into(),
            consumer: consumer.into(),
        }))
    }
}
