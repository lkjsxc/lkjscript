use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::hir::Operation;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ClosedBuiltinOperation(pub(crate) Operation);

impl Serialize for ClosedBuiltinOperation {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.0.name())
    }
}

impl<'de> Deserialize<'de> for ClosedBuiltinOperation {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let name = String::deserialize(deserializer)?;
        Operation::from_name(&name)
            .filter(|operation| operation.record().semantic_source_builtin_call)
            .map(Self)
            .ok_or_else(|| serde::de::Error::custom(format!("unknown built-in operation {name:?}")))
    }
}
