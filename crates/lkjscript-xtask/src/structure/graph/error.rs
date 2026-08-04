use serde::ser::{Serialize, SerializeStruct, Serializer};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GraphBuildError {
    pub dimension: String,
    pub subject: String,
    pub limit: u64,
    pub used: u64,
    pub attempted: u64,
}

impl GraphBuildError {
    pub(super) fn exhausted(dimension: &str, limit: u64, used: u64, attempted: u64) -> Self {
        Self {
            dimension: dimension.into(),
            subject: String::new(),
            limit,
            used,
            attempted,
        }
    }

    pub(super) fn conflicting_node(subject: &str) -> Self {
        let mut error = Self::exhausted("conflicting-node-identity", 0, 0, 1);
        error.subject = subject.into();
        error
    }
}

impl Serialize for GraphBuildError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("GraphBuildError", 9)?;
        state.serialize_field("schema", "lkjscript.repository-graph-error")?;
        state.serialize_field(
            "contract",
            &lkjscript_contracts::REPOSITORY_GRAPH_DIGEST.to_hex(),
        )?;
        state.serialize_field("operation", "complete-repository-graph-construction")?;
        state.serialize_field("dimension", &self.dimension)?;
        state.serialize_field("subject", &self.subject)?;
        state.serialize_field("limit", &self.limit)?;
        state.serialize_field("used", &self.used)?;
        state.serialize_field("attempted", &self.attempted)?;
        state.serialize_field("atomicity", "no graph or identity is published")?;
        state.end()
    }
}

impl std::fmt::Display for GraphBuildError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "repository graph {} exhausted for {}: used {}, attempted {}, limit {}",
            self.dimension, self.subject, self.used, self.attempted, self.limit
        )
    }
}

impl std::error::Error for GraphBuildError {}
