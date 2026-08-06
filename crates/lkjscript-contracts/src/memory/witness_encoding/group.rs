use super::{ExecutableMemoryWitnessDependency, ExecutableMemoryWitnessFacts};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExecutableMemoryWitnessGroupMember {
    pub id: [u8; 32],
    pub ordinal: u16,
    pub semantic_identity: [u8; 32],
    pub facts: ExecutableMemoryWitnessFacts,
    pub dependencies: Vec<ExecutableMemoryWitnessDependency>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExecutableMemoryWitnessGroup {
    pub id: [u8; 32],
    pub recursive: bool,
    pub members: Vec<ExecutableMemoryWitnessGroupMember>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExecutableMemoryWitnessGroupError(pub &'static str);

impl std::fmt::Display for ExecutableMemoryWitnessGroupError {
    fn fmt(&self, output: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        output.write_str(self.0)
    }
}

impl std::error::Error for ExecutableMemoryWitnessGroupError {}
