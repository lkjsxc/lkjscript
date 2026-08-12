use std::fmt;

/// Stable name of one independently checked lkjscript contract.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct ContractName(String);

impl ContractName {
    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub(crate) fn registered(value: &str) -> Self {
        Self(value.to_owned())
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ContractItemKind {
    Type,
    Field,
    Operation,
    Rule,
    Capability,
}

impl ContractItemKind {
    pub(crate) const fn tag(self) -> u8 {
        match self {
            Self::Type => 1,
            Self::Field => 2,
            Self::Operation => 4,
            Self::Rule => 7,
            Self::Capability => 8,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FactOrdering {
    StableIdentity,
    Semantic,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NameIdentity {
    Included,
    Metadata,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContractFact {
    pub stable_id: String,
    pub name: String,
    pub value: String,
    pub required: bool,
    pub closed: bool,
    pub name_identity: NameIdentity,
}

impl ContractFact {
    pub fn required(
        stable_id: impl Into<String>,
        name: impl Into<String>,
        value: impl Into<String>,
    ) -> Self {
        Self {
            stable_id: stable_id.into(),
            name: name.into(),
            value: value.into(),
            required: true,
            closed: true,
            name_identity: NameIdentity::Included,
        }
    }

    pub fn presentation_name(mut self) -> Self {
        self.name_identity = NameIdentity::Metadata;
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContractItem {
    pub stable_id: String,
    pub kind: ContractItemKind,
    pub facts: Vec<ContractFact>,
    pub fact_ordering: FactOrdering,
}

impl ContractItem {
    pub fn new(stable_id: impl Into<String>, kind: ContractItemKind) -> Self {
        Self {
            stable_id: stable_id.into(),
            kind,
            facts: Vec::new(),
            fact_ordering: FactOrdering::StableIdentity,
        }
    }

    pub fn fact(mut self, fact: ContractFact) -> Self {
        self.facts.push(fact);
        self
    }

    pub fn semantic_order(mut self) -> Self {
        self.fact_ordering = FactOrdering::Semantic;
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContractDependency {
    pub name: ContractName,
    pub digest: [u8; 32],
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContractDescriptor {
    pub name: ContractName,
    pub items: Vec<ContractItem>,
    pub dependencies: Vec<ContractDependency>,
}

impl ContractDescriptor {
    pub fn item(mut self, item: ContractItem) -> Self {
        self.items.push(item);
        self
    }

    pub fn dependency(mut self, name: ContractName, digest: [u8; 32]) -> Self {
        self.dependencies.push(ContractDependency { name, digest });
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ContractError {
    DuplicateDependency(String),
    DuplicateFact(String),
    DuplicateItem(String),
    InvalidStableId(String),
    LengthOverflow,
}

impl fmt::Display for ContractError {
    fn fmt(&self, output: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(output, "{self:?}")
    }
}

impl std::error::Error for ContractError {}
