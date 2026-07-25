use super::ResourceCategory;

impl ResourceCategory {
    pub const fn unit(self) -> &'static str {
        match self {
            Self::SourceBytes
            | Self::ProtocolRequestBytes
            | Self::ProtocolResponseBytes
            | Self::ExhaustivenessWitnessBytes
            | Self::SemanticSessionInputBytes
            | Self::SemanticSessionOutputBytes
            | Self::SemanticSessionRetainedBytes
            | Self::StagedPublicationBytes => "bytes",
            Self::ParserWork
            | Self::ValidationWork
            | Self::PathWork
            | Self::TypeWork
            | Self::TraitWork
            | Self::EnumRecursionWork
            | Self::UsefulnessSpecializationWork
            | Self::HoleSearchWork
            | Self::SemanticSessionLifetimeFuel => "work-units",
            Self::LogicalAggregateConstructions => "semantic-events",
            _ => "records",
        }
    }
}
